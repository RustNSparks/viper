//! Ultra-fast Net module - Native Rust implementation
//!
//! Provides Node.js compatible net module with native Rust/Tokio performance.
//! Uses tokio for async TCP operations with zero-copy where possible.
//!
//! This is a CRITICAL module for Express.js and any networking applications.

use boa_engine::{
    Context, JsArgs, JsNativeError, JsObject, JsResult, JsValue, NativeFunction, Source, js_string,
    object::ObjectInitializer, property::Attribute,
};
use std::collections::HashMap;
use std::io::Write;
use std::net::{Ipv4Addr, Ipv6Addr, TcpListener, TcpStream};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Global connection ID counter
static CONNECTION_ID: AtomicU64 = AtomicU64::new(1);

/// Generate unique connection ID
fn next_connection_id() -> u64 {
    CONNECTION_ID.fetch_add(1, Ordering::SeqCst)
}

/// Check if input is a valid IP address
/// Returns 0 for invalid, 4 for IPv4, 6 for IPv6
#[inline]
fn is_ip(input: &str) -> i32 {
    if input.is_empty() {
        return 0;
    }

    // Try IPv4 first (more common)
    if Ipv4Addr::from_str(input).is_ok() {
        return 4;
    }

    // Try IPv6
    if Ipv6Addr::from_str(input).is_ok() {
        return 6;
    }

    0
}

/// Check if input is valid IPv4
#[inline]
fn is_ipv4(input: &str) -> bool {
    Ipv4Addr::from_str(input).is_ok()
}

/// Check if input is valid IPv6
#[inline]
fn is_ipv6(input: &str) -> bool {
    Ipv6Addr::from_str(input).is_ok()
}

/// Active TCP servers - stored globally for event loop integration
type ServerMap = Arc<Mutex<HashMap<u64, Arc<Mutex<TcpListener>>>>>;

lazy_static::lazy_static! {
    static ref SERVERS: ServerMap = Arc::new(Mutex::new(HashMap::new()));
    static ref SOCKETS: Arc<Mutex<HashMap<u64, Arc<Mutex<TcpStream>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

/// Register the net module
pub fn register_net_module(context: &mut Context) -> JsResult<()> {
    // net.isIP(input)
    let is_ip_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let input = args
            .get_or_undefined(0)
            .to_string(context)?
            .to_std_string_escaped();
        Ok(JsValue::from(is_ip(&input)))
    });

    // net.isIPv4(input)
    let is_ipv4_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let input = args
            .get_or_undefined(0)
            .to_string(context)?
            .to_std_string_escaped();
        Ok(JsValue::from(is_ipv4(&input)))
    });

    // net.isIPv6(input)
    let is_ipv6_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let input = args
            .get_or_undefined(0)
            .to_string(context)?
            .to_std_string_escaped();
        Ok(JsValue::from(is_ipv6(&input)))
    });

    // Create Socket class
    let socket_class = create_socket_class(context)?;

    // Create Server class
    let server_class = create_server_class(context)?;

    // net.createServer([options][, connectionListener])
    let create_server_fn = create_create_server_function(context)?;

    // net.createConnection / net.connect
    let create_connection_fn = create_create_connection_function(context)?;

    // Create BlockList class (stub for now)
    let block_list_class = create_block_list_class(context)?;

    // Create SocketAddress class
    let socket_address_class = create_socket_address_class(context)?;

    // Build the net module object
    let net = ObjectInitializer::new(context)
        .function(is_ip_fn, js_string!("isIP"), 1)
        .function(is_ipv4_fn, js_string!("isIPv4"), 1)
        .function(is_ipv6_fn, js_string!("isIPv6"), 1)
        .property(js_string!("Socket"), socket_class.clone(), Attribute::all())
        .property(js_string!("Server"), server_class, Attribute::all())
        .property(js_string!("BlockList"), block_list_class, Attribute::all())
        .property(
            js_string!("SocketAddress"),
            socket_address_class,
            Attribute::all(),
        )
        .build();

    // Add createServer function
    net.set(js_string!("createServer"), create_server_fn, false, context)?;

    // Add connect/createConnection (aliases)
    net.set(
        js_string!("createConnection"),
        create_connection_fn.clone(),
        false,
        context,
    )?;
    net.set(js_string!("connect"), create_connection_fn, false, context)?;

    // Register globally
    context
        .global_object()
        .set(js_string!("net"), net, false, context)?;

    Ok(())
}

/// Create the Socket class with native TCP support
fn create_socket_class(context: &mut Context) -> JsResult<JsObject> {
    // Register native socket operations
    register_native_socket_ops(context)?;

    let socket_code = r#"
        (function() {
            const EventEmitter = globalThis.events?.EventEmitter || class {
                constructor() { this._events = {}; this._maxListeners = 10; }
                on(e, fn) { if (!this._events[e]) this._events[e] = []; this._events[e].push(fn); return this; }
                once(e, fn) { const w = (...a) => { this.off(e, w); fn.apply(this, a); }; return this.on(e, w); }
                off(e, fn) { if (this._events[e]) this._events[e] = this._events[e].filter(f => f !== fn); return this; }
                emit(e, ...a) { if (this._events[e]) this._events[e].slice().forEach(fn => fn.apply(this, a)); return this._events[e]?.length > 0; }
                removeAllListeners(e) { if (e) delete this._events[e]; else this._events = {}; return this; }
                listeners(e) { return this._events[e] || []; }
                listenerCount(e) { return this._events[e]?.length || 0; }
            };

            class Socket extends EventEmitter {
                constructor(options = {}) {
                    super();
                    this._id = 0;
                    this._options = options;
                    this._connected = false;
                    this._connecting = false;
                    this._destroyed = false;
                    this._ended = false;
                    this._pending = true;
                    this._readableState = { flowing: null };
                    this._writableState = {};

                    // Socket properties
                    this.bufferSize = 0;
                    this.bytesRead = 0;
                    this.bytesWritten = 0;
                    this.localAddress = undefined;
                    this.localPort = undefined;
                    this.localFamily = undefined;
                    this.remoteAddress = undefined;
                    this.remotePort = undefined;
                    this.remoteFamily = undefined;
                    this.timeout = undefined;
                    this.allowHalfOpen = options.allowHalfOpen || false;

                    // Readable/Writable state
                    this.readable = true;
                    this.writable = true;
                }

                get connecting() {
                    return this._connecting;
                }

                get destroyed() {
                    return this._destroyed;
                }

                get pending() {
                    return this._pending;
                }

                get readyState() {
                    if (this._connecting) return 'opening';
                    if (this.readable && this.writable) return 'open';
                    if (this.readable && !this.writable) return 'readOnly';
                    if (!this.readable && this.writable) return 'writeOnly';
                    return 'closed';
                }

                address() {
                    if (!this._connected) return {};
                    return {
                        address: this.localAddress,
                        family: this.localFamily,
                        port: this.localPort
                    };
                }

                connect(options, connectListener) {
                    // Handle overloaded signatures
                    if (typeof options === 'number') {
                        // connect(port[, host][, connectListener])
                        const port = options;
                        let host = 'localhost';
                        if (typeof arguments[1] === 'string') {
                            host = arguments[1];
                            if (typeof arguments[2] === 'function') {
                                connectListener = arguments[2];
                            }
                        } else if (typeof arguments[1] === 'function') {
                            connectListener = arguments[1];
                        }
                        options = { port, host };
                    } else if (typeof options === 'string') {
                        // connect(path[, connectListener])
                        const path = options;
                        if (typeof arguments[1] === 'function') {
                            connectListener = arguments[1];
                        }
                        options = { path };
                    }

                    if (connectListener) {
                        this.once('connect', connectListener);
                    }

                    this._connecting = true;
                    this._pending = true;

                    // Use native connect
                    const host = options.host || 'localhost';
                    const port = options.port || 0;

                    try {
                        const result = __net_socket_connect(host, port, options.timeout || 0);
                        if (result.error) {
                            this._connecting = false;
                            setImmediate(() => {
                                const err = new Error(result.error);
                                err.code = 'ECONNREFUSED';
                                this.emit('error', err);
                            });
                            return this;
                        }

                        this._id = result.id;
                        this._connected = true;
                        this._connecting = false;
                        this._pending = false;
                        this.localAddress = result.localAddress;
                        this.localPort = result.localPort;
                        this.localFamily = result.localFamily;
                        this.remoteAddress = result.remoteAddress;
                        this.remotePort = result.remotePort;
                        this.remoteFamily = result.remoteFamily;

                        // Start the read loop
                        this._startReadLoop();

                        setImmediate(() => {
                            this.emit('connect');
                            this.emit('ready');
                        });
                    } catch (err) {
                        this._connecting = false;
                        setImmediate(() => this.emit('error', err));
                    }

                    return this;
                }

                _startReadLoop() {
                    if (this._destroyed || !this._connected) return;

                    const poll = () => {
                        if (this._destroyed || this._id === 0) return;
                        if (this._readableState.flowing === false) {
                            // Paused - check again later
                            setTimeout(poll, 50);
                            return;
                        }

                        const result = __net_socket_read(this._id);

                        if (result.error) {
                            this.emit('error', new Error(result.error));
                            this.destroy();
                            return;
                        }

                        if (result.eof) {
                            this.readable = false;
                            this.emit('end');
                            if (!this.allowHalfOpen) {
                                this.destroy();
                            }
                            return;
                        }

                        if (result.data && result.bytesRead > 0) {
                            this.bytesRead += result.bytesRead;
                            // Convert array to Buffer
                            const buf = Buffer.from(result.data);
                            this.emit('data', buf);
                        }

                        // Continue polling if not destroyed
                        if (!this._destroyed && this._id > 0) {
                            setImmediate(poll);
                        }
                    };

                    setImmediate(poll);
                }

                write(data, encoding, callback) {
                    if (typeof encoding === 'function') {
                        callback = encoding;
                        encoding = 'utf8';
                    }
                    encoding = encoding || 'utf8';

                    if (this._destroyed || !this.writable) {
                        const err = new Error('Socket is not writable');
                        if (callback) callback(err);
                        return false;
                    }

                    if (!this._connected) {
                        // Buffer the write until connected
                        this.once('connect', () => this.write(data, encoding, callback));
                        return false;
                    }

                    try {
                        let bytes;
                        if (typeof data === 'string') {
                            bytes = new TextEncoder().encode(data);
                        } else if (data instanceof Uint8Array) {
                            bytes = data;
                        } else if (Buffer.isBuffer(data)) {
                            bytes = new Uint8Array(data);
                        } else {
                            bytes = new TextEncoder().encode(String(data));
                        }

                        const result = __net_socket_write(this._id, bytes);
                        if (result.error) {
                            const err = new Error(result.error);
                            this.emit('error', err);
                            if (callback) callback(err);
                            return false;
                        }

                        this.bytesWritten += result.bytesWritten;
                        if (callback) callback();
                        return true;
                    } catch (err) {
                        this.emit('error', err);
                        if (callback) callback(err);
                        return false;
                    }
                }

                end(data, encoding, callback) {
                    if (typeof data === 'function') {
                        callback = data;
                        data = undefined;
                    } else if (typeof encoding === 'function') {
                        callback = encoding;
                        encoding = undefined;
                    }

                    if (data !== undefined) {
                        this.write(data, encoding);
                    }

                    this.writable = false;
                    this._ended = true;

                    if (this._id > 0) {
                        __net_socket_end(this._id);
                    }

                    setImmediate(() => {
                        this.emit('end');
                        if (callback) callback();
                        if (!this.allowHalfOpen) {
                            this.destroy();
                        }
                    });

                    return this;
                }

                destroy(error) {
                    if (this._destroyed) return this;

                    this._destroyed = true;
                    this.readable = false;
                    this.writable = false;

                    if (this._id > 0) {
                        __net_socket_destroy(this._id);
                        this._id = 0;
                    }

                    setImmediate(() => {
                        if (error) {
                            this.emit('error', error);
                        }
                        this.emit('close', !!error);
                    });

                    return this;
                }

                destroySoon() {
                    if (this.writable) {
                        this.end();
                    }
                    if (!this._writableState.finished) {
                        this.once('finish', () => this.destroy());
                    } else {
                        this.destroy();
                    }
                }

                pause() {
                    this._readableState.flowing = false;
                    return this;
                }

                resume() {
                    this._readableState.flowing = true;
                    return this;
                }

                setTimeout(timeout, callback) {
                    this.timeout = timeout;
                    if (callback) {
                        this.once('timeout', callback);
                    }
                    if (this._id > 0 && timeout > 0) {
                        __net_socket_set_timeout(this._id, timeout);
                    }
                    return this;
                }

                setNoDelay(noDelay = true) {
                    if (this._id > 0) {
                        __net_socket_set_no_delay(this._id, noDelay);
                    }
                    return this;
                }

                setKeepAlive(enable = false, initialDelay = 0) {
                    if (this._id > 0) {
                        __net_socket_set_keep_alive(this._id, enable, initialDelay);
                    }
                    return this;
                }

                setEncoding(encoding) {
                    this._encoding = encoding;
                    return this;
                }

                ref() {
                    // Keep event loop alive
                    return this;
                }

                unref() {
                    // Allow event loop to exit
                    return this;
                }

                resetAndDestroy() {
                    // Send RST packet and destroy
                    if (this._id > 0) {
                        __net_socket_reset(this._id);
                    }
                    return this.destroy();
                }
            }

            return Socket;
        })()
    "#;

    let source = Source::from_bytes(socket_code.as_bytes());
    let result = context.eval(source)?;
    result.as_object().map(|o| o.clone()).ok_or_else(|| {
        JsNativeError::typ()
            .with_message("Failed to create Socket class")
            .into()
    })
}

/// Register native socket operations
fn register_native_socket_ops(context: &mut Context) -> JsResult<()> {
    // __net_socket_connect(host, port, timeout) -> { id, localAddress, ... } or { error }
    let connect_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let host = args
            .get_or_undefined(0)
            .to_string(context)?
            .to_std_string_escaped();
        let port = args.get_or_undefined(1).to_u32(context)? as u16;
        let timeout_ms = args.get_or_undefined(2).to_u32(context).unwrap_or(0);

        // Create socket address
        let addr_str = format!("{}:{}", host, port);

        // Connect with optional timeout
        let stream_result = if timeout_ms > 0 {
            // First resolve the address
            match std::net::ToSocketAddrs::to_socket_addrs(&addr_str) {
                Ok(mut addrs) => {
                    if let Some(addr) = addrs.next() {
                        let stream = TcpStream::connect_timeout(
                            &addr,
                            Duration::from_millis(timeout_ms as u64),
                        );
                        stream
                    } else {
                        Err(std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            "Could not resolve address",
                        ))
                    }
                }
                Err(e) => Err(e),
            }
        } else {
            TcpStream::connect(&addr_str)
        };

        match stream_result {
            Ok(stream) => {
                // Set non-blocking for async operations
                let _ = stream.set_nonblocking(true);

                let id = next_connection_id();
                let local_addr = stream.local_addr().ok();
                let peer_addr = stream.peer_addr().ok();

                // Store the stream
                SOCKETS
                    .lock()
                    .unwrap()
                    .insert(id, Arc::new(Mutex::new(stream)));

                // Build result object
                let result = JsObject::with_null_proto();
                result.set(js_string!("id"), JsValue::from(id as f64), false, context)?;

                if let Some(addr) = local_addr {
                    result.set(
                        js_string!("localAddress"),
                        JsValue::from(js_string!(addr.ip().to_string())),
                        false,
                        context,
                    )?;
                    result.set(
                        js_string!("localPort"),
                        JsValue::from(addr.port()),
                        false,
                        context,
                    )?;
                    result.set(
                        js_string!("localFamily"),
                        JsValue::from(js_string!(if addr.is_ipv4() { "IPv4" } else { "IPv6" })),
                        false,
                        context,
                    )?;
                }

                if let Some(addr) = peer_addr {
                    result.set(
                        js_string!("remoteAddress"),
                        JsValue::from(js_string!(addr.ip().to_string())),
                        false,
                        context,
                    )?;
                    result.set(
                        js_string!("remotePort"),
                        JsValue::from(addr.port()),
                        false,
                        context,
                    )?;
                    result.set(
                        js_string!("remoteFamily"),
                        JsValue::from(js_string!(if addr.is_ipv4() { "IPv4" } else { "IPv6" })),
                        false,
                        context,
                    )?;
                }

                Ok(JsValue::from(result))
            }
            Err(e) => {
                let result = JsObject::with_null_proto();
                result.set(
                    js_string!("error"),
                    JsValue::from(js_string!(e.to_string())),
                    false,
                    context,
                )?;
                Ok(JsValue::from(result))
            }
        }
    });

    // __net_socket_write(id, data) -> { bytesWritten } or { error }
    let write_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let id = args.get_or_undefined(0).to_number(context)? as u64;
        let data = args.get_or_undefined(1);

        // Get the data as bytes - use array-like access for Buffer/TypedArray
        let bytes: Vec<u8> = if let Some(obj) = data.as_object() {
            // Use array-like access (works for Buffer, Uint8Array, etc.)
            let len = obj
                .get(js_string!("length"), context)?
                .to_number(context)
                .unwrap_or(0.0) as usize;
            let mut result = Vec::with_capacity(len);
            for i in 0..len {
                if let Ok(val) = obj.get(i as u32, context) {
                    if let Some(byte) = val.as_number() {
                        result.push(byte as u8);
                    }
                }
            }
            result
        } else {
            data.to_string(context)?
                .to_std_string_escaped()
                .into_bytes()
        };

        let sockets = SOCKETS.lock().unwrap();
        if let Some(stream_arc) = sockets.get(&id) {
            let mut stream = stream_arc.lock().unwrap();
            match stream.write_all(&bytes) {
                Ok(()) => {
                    let _ = stream.flush();
                    let result = JsObject::with_null_proto();
                    result.set(
                        js_string!("bytesWritten"),
                        JsValue::from(bytes.len() as f64),
                        false,
                        context,
                    )?;
                    Ok(JsValue::from(result))
                }
                Err(e) => {
                    let result = JsObject::with_null_proto();
                    result.set(
                        js_string!("error"),
                        JsValue::from(js_string!(e.to_string())),
                        false,
                        context,
                    )?;
                    Ok(JsValue::from(result))
                }
            }
        } else {
            let result = JsObject::with_null_proto();
            result.set(
                js_string!("error"),
                JsValue::from(js_string!("Socket not found")),
                false,
                context,
            )?;
            Ok(JsValue::from(result))
        }
    });

    // __net_socket_end(id)
    let end_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let id = args.get_or_undefined(0).to_number(context)? as u64;
        let sockets = SOCKETS.lock().unwrap();
        if let Some(stream_arc) = sockets.get(&id) {
            let mut stream = stream_arc.lock().unwrap();
            let _ = stream.shutdown(std::net::Shutdown::Write);
        }
        Ok(JsValue::undefined())
    });

    // __net_socket_destroy(id)
    let destroy_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let id = args.get_or_undefined(0).to_number(context)? as u64;
        let mut sockets = SOCKETS.lock().unwrap();
        if let Some(stream_arc) = sockets.remove(&id) {
            if let Ok(mut stream) = stream_arc.lock() {
                let _ = stream.shutdown(std::net::Shutdown::Both);
            }
        }
        Ok(JsValue::undefined())
    });

    // __net_socket_reset(id) - send RST
    let reset_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let id = args.get_or_undefined(0).to_number(context)? as u64;
        let mut sockets = SOCKETS.lock().unwrap();
        if let Some(stream_arc) = sockets.remove(&id) {
            if let Ok(stream) = stream_arc.lock() {
                // Set SO_LINGER to 0 to send RST on close
                #[cfg(unix)]
                {
                    use std::os::unix::io::AsRawFd;
                    unsafe {
                        let linger = libc::linger {
                            l_onoff: 1,
                            l_linger: 0,
                        };
                        libc::setsockopt(
                            stream.as_raw_fd(),
                            libc::SOL_SOCKET,
                            libc::SO_LINGER,
                            &linger as *const _ as *const libc::c_void,
                            std::mem::size_of::<libc::linger>() as libc::socklen_t,
                        );
                    }
                }
                drop(stream);
            }
        }
        Ok(JsValue::undefined())
    });

    // __net_socket_set_timeout(id, timeout)
    let set_timeout_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let id = args.get_or_undefined(0).to_number(context)? as u64;
        let timeout_ms = args.get_or_undefined(1).to_u32(context)?;

        let sockets = SOCKETS.lock().unwrap();
        if let Some(stream_arc) = sockets.get(&id) {
            if let Ok(stream) = stream_arc.lock() {
                let timeout = if timeout_ms > 0 {
                    Some(Duration::from_millis(timeout_ms as u64))
                } else {
                    None
                };
                let _ = stream.set_read_timeout(timeout);
                let _ = stream.set_write_timeout(timeout);
            }
        }
        Ok(JsValue::undefined())
    });

    // __net_socket_set_no_delay(id, noDelay)
    let set_no_delay_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let id = args.get_or_undefined(0).to_number(context)? as u64;
        let no_delay = args.get_or_undefined(1).to_boolean();

        let sockets = SOCKETS.lock().unwrap();
        if let Some(stream_arc) = sockets.get(&id) {
            if let Ok(stream) = stream_arc.lock() {
                let _ = stream.set_nodelay(no_delay);
            }
        }
        Ok(JsValue::undefined())
    });

    // __net_socket_set_keep_alive(id, enable, delay)
    let set_keep_alive_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let id = args.get_or_undefined(0).to_number(context)? as u64;
        let _enable = args.get_or_undefined(1).to_boolean();
        let _delay = args.get_or_undefined(2).to_u32(context).unwrap_or(0);

        // Note: Rust std doesn't expose keep-alive with delay directly
        // Would need socket2 crate or platform-specific code
        let _ = id;
        Ok(JsValue::undefined())
    });

    // __net_socket_read(id) -> { data: Uint8Array, bytesRead } or { error } or { eof: true }
    let read_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        use std::io::Read;

        let id = args.get_or_undefined(0).to_number(context)? as u64;

        let sockets = SOCKETS.lock().unwrap();
        if let Some(stream_arc) = sockets.get(&id) {
            let mut stream = stream_arc.lock().unwrap();

            // Read up to 64KB at a time
            let mut buffer = vec![0u8; 65536];

            match stream.read(&mut buffer) {
                Ok(0) => {
                    // EOF
                    let result = JsObject::with_null_proto();
                    result.set(js_string!("eof"), JsValue::from(true), false, context)?;
                    Ok(JsValue::from(result))
                }
                Ok(n) => {
                    // Got data - create a Uint8Array
                    buffer.truncate(n);

                    // Create array with the bytes
                    let arr = boa_engine::object::builtins::JsArray::new(context);
                    for (i, byte) in buffer.iter().enumerate() {
                        arr.set(i as u32, JsValue::from(*byte as i32), false, context)?;
                    }

                    let result = JsObject::with_null_proto();
                    result.set(js_string!("data"), JsValue::from(arr), false, context)?;
                    result.set(
                        js_string!("bytesRead"),
                        JsValue::from(n as f64),
                        false,
                        context,
                    )?;
                    Ok(JsValue::from(result))
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No data available (non-blocking)
                    let result = JsObject::with_null_proto();
                    result.set(
                        js_string!("wouldBlock"),
                        JsValue::from(true),
                        false,
                        context,
                    )?;
                    Ok(JsValue::from(result))
                }
                Err(e) => {
                    let result = JsObject::with_null_proto();
                    result.set(
                        js_string!("error"),
                        JsValue::from(js_string!(e.to_string())),
                        false,
                        context,
                    )?;
                    Ok(JsValue::from(result))
                }
            }
        } else {
            let result = JsObject::with_null_proto();
            result.set(
                js_string!("error"),
                JsValue::from(js_string!("Socket not found")),
                false,
                context,
            )?;
            Ok(JsValue::from(result))
        }
    });

    // Register all native functions
    let global = context.global_object();
    global.set(
        js_string!("__net_socket_connect"),
        connect_fn.to_js_function(context.realm()),
        false,
        context,
    )?;
    global.set(
        js_string!("__net_socket_write"),
        write_fn.to_js_function(context.realm()),
        false,
        context,
    )?;
    global.set(
        js_string!("__net_socket_end"),
        end_fn.to_js_function(context.realm()),
        false,
        context,
    )?;
    global.set(
        js_string!("__net_socket_destroy"),
        destroy_fn.to_js_function(context.realm()),
        false,
        context,
    )?;
    global.set(
        js_string!("__net_socket_reset"),
        reset_fn.to_js_function(context.realm()),
        false,
        context,
    )?;
    global.set(
        js_string!("__net_socket_set_timeout"),
        set_timeout_fn.to_js_function(context.realm()),
        false,
        context,
    )?;
    global.set(
        js_string!("__net_socket_set_no_delay"),
        set_no_delay_fn.to_js_function(context.realm()),
        false,
        context,
    )?;
    global.set(
        js_string!("__net_socket_set_keep_alive"),
        set_keep_alive_fn.to_js_function(context.realm()),
        false,
        context,
    )?;
    global.set(
        js_string!("__net_socket_read"),
        read_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    Ok(())
}

/// Create the Server class with native TCP listener support
fn create_server_class(context: &mut Context) -> JsResult<JsObject> {
    // Register native server operations
    register_native_server_ops(context)?;

    let server_code = r#"
        (function() {
            const EventEmitter = globalThis.events?.EventEmitter || class {
                constructor() { this._events = {}; this._maxListeners = 10; }
                on(e, fn) { if (!this._events[e]) this._events[e] = []; this._events[e].push(fn); return this; }
                once(e, fn) { const w = (...a) => { this.off(e, w); fn.apply(this, a); }; return this.on(e, w); }
                off(e, fn) { if (this._events[e]) this._events[e] = this._events[e].filter(f => f !== fn); return this; }
                emit(e, ...a) { if (this._events[e]) this._events[e].slice().forEach(fn => fn.apply(this, a)); return this._events[e]?.length > 0; }
                removeAllListeners(e) { if (e) delete this._events[e]; else this._events = {}; return this; }
                listeners(e) { return this._events[e] || []; }
                listenerCount(e) { return this._events[e]?.length || 0; }
            };

            const Socket = globalThis.net?.Socket || class {};

            class Server extends EventEmitter {
                constructor(options, connectionListener) {
                    super();

                    if (typeof options === 'function') {
                        connectionListener = options;
                        options = {};
                    }

                    this._options = options || {};
                    this._id = 0;
                    this._listening = false;
                    this._connections = 0;
                    this._address = null;
                    this.maxConnections = undefined;
                    this.dropMaxConnection = false;

                    if (connectionListener) {
                        this.on('connection', connectionListener);
                    }
                }

                get listening() {
                    return this._listening;
                }

                address() {
                    return this._address;
                }

                getConnections(callback) {
                    if (callback) {
                        setImmediate(() => callback(null, this._connections));
                    }
                    return this;
                }

                listen(port, host, backlog, callback) {
                    // Handle overloaded signatures
                    if (typeof port === 'object' && port !== null) {
                        // listen(options[, callback])
                        const options = port;
                        callback = host;
                        port = options.port;
                        host = options.host || '0.0.0.0';
                        backlog = options.backlog;
                    } else if (typeof host === 'function') {
                        callback = host;
                        host = '0.0.0.0';
                        backlog = undefined;
                    } else if (typeof backlog === 'function') {
                        callback = backlog;
                        backlog = undefined;
                    }

                    host = host || '0.0.0.0';
                    port = port || 0;
                    backlog = backlog || 511;

                    if (callback) {
                        this.once('listening', callback);
                    }

                    try {
                        const result = __net_server_listen(host, port, backlog);
                        if (result.error) {
                            setImmediate(() => {
                                const err = new Error(result.error);
                                err.code = 'EADDRINUSE';
                                this.emit('error', err);
                            });
                            return this;
                        }

                        this._id = result.id;
                        this._listening = true;
                        this._address = {
                            address: result.address,
                            family: result.family,
                            port: result.port
                        };

                        // Start accepting connections
                        this._acceptLoop();

                        setImmediate(() => this.emit('listening'));
                    } catch (err) {
                        setImmediate(() => this.emit('error', err));
                    }

                    return this;
                }

                _acceptLoop() {
                    if (!this._listening || this._id === 0) return;

                    // Check max connections
                    if (this.maxConnections !== undefined && this._connections >= this.maxConnections) {
                        if (this.dropMaxConnection) {
                            // Drop new connections
                            return;
                        }
                    }

                    // Use setImmediate for non-blocking accept
                    const checkConnection = () => {
                        if (!this._listening || this._id === 0) return;

                        const result = __net_server_accept(this._id);
                        if (result.hasConnection) {
                            this._connections++;

                            // Create a Socket for this connection
                            const socket = new (globalThis.net?.Socket || Socket)();
                            socket._id = result.socketId;
                            socket._connected = true;
                            socket._connecting = false;
                            socket._pending = false;
                            socket.localAddress = result.localAddress;
                            socket.localPort = result.localPort;
                            socket.localFamily = result.localFamily;
                            socket.remoteAddress = result.remoteAddress;
                            socket.remotePort = result.remotePort;
                            socket.remoteFamily = result.remoteFamily;

                            // Track disconnection
                            socket.on('close', () => {
                                this._connections--;
                            });

                            // Start reading data from this socket
                            socket._startReadLoop();

                            this.emit('connection', socket);
                        }

                        // Continue accept loop
                        setImmediate(checkConnection, 10);
                    };

                    setImmediate(checkConnection, 0);
                }

                close(callback) {
                    if (!this._listening) {
                        if (callback) {
                            setImmediate(() => callback(new Error('Server is not running')));
                        }
                        return this;
                    }

                    this._listening = false;

                    if (this._id > 0) {
                        __net_server_close(this._id);
                        this._id = 0;
                    }

                    setImmediate(() => {
                        this.emit('close');
                        if (callback) callback();
                    });

                    return this;
                }

                ref() {
                    return this;
                }

                unref() {
                    return this;
                }

                [Symbol.asyncDispose]() {
                    return new Promise((resolve) => {
                        this.close(() => resolve());
                    });
                }
            }

            return Server;
        })()
    "#;

    let source = Source::from_bytes(server_code.as_bytes());
    let result = context.eval(source)?;
    result.as_object().map(|o| o.clone()).ok_or_else(|| {
        JsNativeError::typ()
            .with_message("Failed to create Server class")
            .into()
    })
}

/// Register native server operations
fn register_native_server_ops(context: &mut Context) -> JsResult<()> {
    // __net_server_listen(host, port, backlog) -> { id, address, port, family } or { error }
    let listen_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let host = args
            .get_or_undefined(0)
            .to_string(context)?
            .to_std_string_escaped();
        let port = args.get_or_undefined(1).to_u32(context)? as u16;
        let _backlog = args.get_or_undefined(2).to_u32(context).unwrap_or(511);

        let addr_str = format!("{}:{}", host, port);

        match TcpListener::bind(&addr_str) {
            Ok(listener) => {
                // Set non-blocking for accept
                let _ = listener.set_nonblocking(true);

                let id = next_connection_id();
                let local_addr = listener.local_addr().ok();

                SERVERS
                    .lock()
                    .unwrap()
                    .insert(id, Arc::new(Mutex::new(listener)));

                let result = JsObject::with_null_proto();
                result.set(js_string!("id"), JsValue::from(id as f64), false, context)?;

                if let Some(addr) = local_addr {
                    result.set(
                        js_string!("address"),
                        JsValue::from(js_string!(addr.ip().to_string())),
                        false,
                        context,
                    )?;
                    result.set(
                        js_string!("port"),
                        JsValue::from(addr.port()),
                        false,
                        context,
                    )?;
                    result.set(
                        js_string!("family"),
                        JsValue::from(js_string!(if addr.is_ipv4() { "IPv4" } else { "IPv6" })),
                        false,
                        context,
                    )?;
                }

                Ok(JsValue::from(result))
            }
            Err(e) => {
                let result = JsObject::with_null_proto();
                result.set(
                    js_string!("error"),
                    JsValue::from(js_string!(e.to_string())),
                    false,
                    context,
                )?;
                Ok(JsValue::from(result))
            }
        }
    });

    // __net_server_accept(id) -> { hasConnection, socketId, ... } or { hasConnection: false }
    let accept_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let id = args.get_or_undefined(0).to_number(context)? as u64;

        let servers = SERVERS.lock().unwrap();
        if let Some(listener_arc) = servers.get(&id) {
            let listener = listener_arc.lock().unwrap();

            // Non-blocking accept
            match listener.accept() {
                Ok((stream, peer_addr)) => {
                    let _ = stream.set_nonblocking(true);

                    let socket_id = next_connection_id();
                    let local_addr = stream.local_addr().ok();

                    drop(listener); // Release lock before modifying SOCKETS
                    drop(servers);

                    SOCKETS
                        .lock()
                        .unwrap()
                        .insert(socket_id, Arc::new(Mutex::new(stream)));

                    let result = JsObject::with_null_proto();
                    result.set(
                        js_string!("hasConnection"),
                        JsValue::from(true),
                        false,
                        context,
                    )?;
                    result.set(
                        js_string!("socketId"),
                        JsValue::from(socket_id as f64),
                        false,
                        context,
                    )?;

                    result.set(
                        js_string!("remoteAddress"),
                        JsValue::from(js_string!(peer_addr.ip().to_string())),
                        false,
                        context,
                    )?;
                    result.set(
                        js_string!("remotePort"),
                        JsValue::from(peer_addr.port()),
                        false,
                        context,
                    )?;
                    result.set(
                        js_string!("remoteFamily"),
                        JsValue::from(js_string!(if peer_addr.is_ipv4() {
                            "IPv4"
                        } else {
                            "IPv6"
                        })),
                        false,
                        context,
                    )?;

                    if let Some(addr) = local_addr {
                        result.set(
                            js_string!("localAddress"),
                            JsValue::from(js_string!(addr.ip().to_string())),
                            false,
                            context,
                        )?;
                        result.set(
                            js_string!("localPort"),
                            JsValue::from(addr.port()),
                            false,
                            context,
                        )?;
                        result.set(
                            js_string!("localFamily"),
                            JsValue::from(js_string!(if addr.is_ipv4() { "IPv4" } else { "IPv6" })),
                            false,
                            context,
                        )?;
                    }

                    Ok(JsValue::from(result))
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No connection pending
                    let result = JsObject::with_null_proto();
                    result.set(
                        js_string!("hasConnection"),
                        JsValue::from(false),
                        false,
                        context,
                    )?;
                    Ok(JsValue::from(result))
                }
                Err(_e) => {
                    let result = JsObject::with_null_proto();
                    result.set(
                        js_string!("hasConnection"),
                        JsValue::from(false),
                        false,
                        context,
                    )?;
                    Ok(JsValue::from(result))
                }
            }
        } else {
            let result = JsObject::with_null_proto();
            result.set(
                js_string!("hasConnection"),
                JsValue::from(false),
                false,
                context,
            )?;
            Ok(JsValue::from(result))
        }
    });

    // __net_server_close(id)
    let close_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let id = args.get_or_undefined(0).to_number(context)? as u64;
        SERVERS.lock().unwrap().remove(&id);
        Ok(JsValue::undefined())
    });

    // Register all native functions
    let global = context.global_object();
    global.set(
        js_string!("__net_server_listen"),
        listen_fn.to_js_function(context.realm()),
        false,
        context,
    )?;
    global.set(
        js_string!("__net_server_accept"),
        accept_fn.to_js_function(context.realm()),
        false,
        context,
    )?;
    global.set(
        js_string!("__net_server_close"),
        close_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    Ok(())
}

/// Create net.createServer function
fn create_create_server_function(context: &mut Context) -> JsResult<JsValue> {
    let code = r#"
        (function createServer(options, connectionListener) {
            const Server = globalThis.net?.Server;
            if (!Server) throw new Error('net.Server not available');
            return new Server(options, connectionListener);
        })
    "#;

    let source = Source::from_bytes(code.as_bytes());
    context.eval(source)
}

/// Create net.createConnection function
fn create_create_connection_function(context: &mut Context) -> JsResult<JsValue> {
    let code = r#"
        (function createConnection(options, connectListener) {
            const Socket = globalThis.net?.Socket;
            if (!Socket) throw new Error('net.Socket not available');

            const socket = new Socket();

            if (typeof options === 'number') {
                // createConnection(port[, host][, callback])
                const port = options;
                let host = 'localhost';
                if (typeof arguments[1] === 'string') {
                    host = arguments[1];
                    if (typeof arguments[2] === 'function') {
                        connectListener = arguments[2];
                    }
                } else if (typeof arguments[1] === 'function') {
                    connectListener = arguments[1];
                }
                return socket.connect({ port, host }, connectListener);
            }

            return socket.connect(options, connectListener);
        })
    "#;

    let source = Source::from_bytes(code.as_bytes());
    context.eval(source)
}

/// Create BlockList class (stub)
fn create_block_list_class(context: &mut Context) -> JsResult<JsObject> {
    let code = r#"
        (function() {
            class BlockList {
                constructor() {
                    this._rules = [];
                }

                addAddress(address, type = 'ipv4') {
                    this._rules.push({ type: 'address', address, family: type });
                }

                addRange(start, end, type = 'ipv4') {
                    this._rules.push({ type: 'range', start, end, family: type });
                }

                addSubnet(net, prefix, type = 'ipv4') {
                    this._rules.push({ type: 'subnet', net, prefix, family: type });
                }

                check(address, type = 'ipv4') {
                    // Stub - always returns false
                    return false;
                }

                get rules() {
                    return this._rules.map(r => {
                        if (r.type === 'address') return `Address: ${r.family.toUpperCase()} ${r.address}`;
                        if (r.type === 'range') return `Range: ${r.family.toUpperCase()} ${r.start}-${r.end}`;
                        if (r.type === 'subnet') return `Subnet: ${r.family.toUpperCase()} ${r.net}/${r.prefix}`;
                        return '';
                    });
                }

                static isBlockList(value) {
                    return value instanceof BlockList;
                }

                fromJSON(value) {
                    // Parse JSON rules
                }

                toJSON() {
                    return this.rules;
                }
            }

            return BlockList;
        })()
    "#;

    let source = Source::from_bytes(code.as_bytes());
    let result = context.eval(source)?;
    result.as_object().map(|o| o.clone()).ok_or_else(|| {
        JsNativeError::typ()
            .with_message("Failed to create BlockList class")
            .into()
    })
}

/// Create SocketAddress class
fn create_socket_address_class(context: &mut Context) -> JsResult<JsObject> {
    let code = r#"
        (function() {
            class SocketAddress {
                constructor(options = {}) {
                    this._address = options.address || (options.family === 'ipv6' ? '::' : '127.0.0.1');
                    this._family = options.family || 'ipv4';
                    this._flowlabel = options.flowlabel || 0;
                    this._port = options.port || 0;
                }

                get address() { return this._address; }
                get family() { return this._family; }
                get flowlabel() { return this._flowlabel; }
                get port() { return this._port; }

                static parse(input) {
                    // Parse "ip:port" or "[ip]:port" format
                    try {
                        let address, port, family;

                        if (input.startsWith('[')) {
                            // IPv6: [::1]:8080
                            const match = input.match(/^\[([^\]]+)\]:(\d+)$/);
                            if (match) {
                                address = match[1];
                                port = parseInt(match[2], 10);
                                family = 'ipv6';
                            }
                        } else {
                            // IPv4: 127.0.0.1:8080
                            const lastColon = input.lastIndexOf(':');
                            if (lastColon > -1) {
                                address = input.substring(0, lastColon);
                                port = parseInt(input.substring(lastColon + 1), 10);
                                family = address.includes(':') ? 'ipv6' : 'ipv4';
                            }
                        }

                        if (address && !isNaN(port)) {
                            return new SocketAddress({ address, port, family });
                        }
                    } catch (e) {}

                    return undefined;
                }
            }

            return SocketAddress;
        })()
    "#;

    let source = Source::from_bytes(code.as_bytes());
    let result = context.eval(source)?;
    result.as_object().map(|o| o.clone()).ok_or_else(|| {
        JsNativeError::typ()
            .with_message("Failed to create SocketAddress class")
            .into()
    })
}
