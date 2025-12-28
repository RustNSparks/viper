//! Process API - Node.js compatible process object
//!
//! Provides comprehensive Node.js process compatibility with native Rust performance:
//! - process.argv, process.argv0, process.execArgv, process.execPath
//! - process.exit(code), process.abort()
//! - process.cwd(), process.chdir(dir)
//! - process.env - Environment variables
//! - process.pid, process.ppid
//! - process.platform, process.arch
//! - process.version, process.versions
//! - process.hrtime(), process.hrtime.bigint()
//! - process.memoryUsage(), process.memoryUsage.rss()
//! - process.cpuUsage(), process.resourceUsage()
//! - process.uptime()
//! - process.nextTick(callback)
//! - process.kill(pid, signal)
//! - process.stdout, process.stderr, process.stdin
//! - Event emitter: on, once, off, emit for exit, beforeExit, uncaughtException, etc.

use boa_engine::{
    Context, JsNativeError, JsResult, JsValue, NativeFunction, Source, js_string,
    object::ObjectInitializer, object::builtins::JsArray,
};
use std::sync::OnceLock;
use std::time::Instant;

/// Global start time for process.uptime() - thread-safe
static PROCESS_START_TIME: OnceLock<Instant> = OnceLock::new();

/// Initialize process start time
fn init_start_time() {
    PROCESS_START_TIME.get_or_init(Instant::now);
}

/// Get process uptime in seconds
fn get_uptime() -> f64 {
    PROCESS_START_TIME
        .get()
        .map(|start| start.elapsed().as_secs_f64())
        .unwrap_or(0.0)
}

/// Register the process object
pub fn register_process(context: &mut Context, args: &[String]) -> JsResult<()> {
    init_start_time();

    // Platform detection
    let platform = if cfg!(target_os = "windows") {
        "win32"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "freebsd") {
        "freebsd"
    } else if cfg!(target_os = "openbsd") {
        "openbsd"
    } else if cfg!(target_os = "android") {
        "android"
    } else {
        "unknown"
    };

    // Architecture detection
    let arch = if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else if cfg!(target_arch = "x86") {
        "ia32"
    } else if cfg!(target_arch = "arm") {
        "arm"
    } else if cfg!(target_arch = "mips") {
        "mips"
    } else if cfg!(target_arch = "mips64") {
        "mips64"
    } else if cfg!(target_arch = "powerpc64") {
        "ppc64"
    } else if cfg!(target_arch = "s390x") {
        "s390x"
    } else {
        "unknown"
    };

    // Register platform and arch
    context.global_object().set(
        js_string!("__VIPER_PLATFORM__"),
        JsValue::from(js_string!(platform)),
        false,
        context,
    )?;

    context.global_object().set(
        js_string!("__VIPER_ARCH__"),
        JsValue::from(js_string!(arch)),
        false,
        context,
    )?;

    // Store argv
    let argv_array = JsArray::new(context);
    for arg in args {
        argv_array.push(JsValue::from(js_string!(arg.clone())), context)?;
    }
    context
        .global_object()
        .set(js_string!("__viper_argv"), argv_array, false, context)?;

    // argv0 - original first argument
    let argv0 = args.first().map(|s| s.as_str()).unwrap_or("viper");
    context.global_object().set(
        js_string!("__viper_argv0"),
        JsValue::from(js_string!(argv0)),
        false,
        context,
    )?;

    // Register native functions
    register_native_process_functions(context)?;

    // Create the process object in JavaScript
    let process_code = r#"
        (function() {
            // EventEmitter base for process
            const _listeners = {};
            const _maxListeners = 10;

            // Event emitter methods
            function on(event, listener) {
                if (!_listeners[event]) _listeners[event] = [];
                _listeners[event].push(listener);
                return process;
            }

            function once(event, listener) {
                const wrapped = (...args) => {
                    off(event, wrapped);
                    listener.apply(process, args);
                };
                wrapped.listener = listener;
                return on(event, wrapped);
            }

            function off(event, listener) {
                if (_listeners[event]) {
                    _listeners[event] = _listeners[event].filter(l =>
                        l !== listener && l.listener !== listener
                    );
                }
                return process;
            }

            function emit(event, ...args) {
                if (_listeners[event]) {
                    const listeners = _listeners[event].slice();
                    for (const listener of listeners) {
                        try {
                            listener.apply(process, args);
                        } catch (e) {
                            if (event !== 'uncaughtException') {
                                emit('uncaughtException', e);
                            }
                        }
                    }
                    return true;
                }
                return false;
            }

            function removeAllListeners(event) {
                if (event) {
                    delete _listeners[event];
                } else {
                    for (const key in _listeners) {
                        delete _listeners[key];
                    }
                }
                return process;
            }

            function listeners(event) {
                return _listeners[event] ? _listeners[event].slice() : [];
            }

            function listenerCount(event) {
                return _listeners[event]?.length || 0;
            }

            // Create env proxy with proper enumeration
            const envProxy = new Proxy({}, {
                get: (target, prop) => {
                    if (typeof prop === 'symbol') return undefined;
                    return __viper_env_get(String(prop));
                },
                set: (target, prop, value) => {
                    __viper_env_set(String(prop), String(value));
                    return true;
                },
                has: (target, prop) => {
                    return __viper_env_get(String(prop)) !== undefined;
                },
                deleteProperty: (target, prop) => {
                    __viper_env_delete(String(prop));
                    return true;
                },
                ownKeys: () => {
                    return Object.keys(__viper_env_all());
                },
                getOwnPropertyDescriptor: (target, prop) => {
                    const value = __viper_env_get(String(prop));
                    if (value !== undefined) {
                        return { value, writable: true, enumerable: true, configurable: true };
                    }
                    return undefined;
                }
            });

            // hrtime function with bigint method
            function hrtime(prev) {
                return __viper_hrtime(prev);
            }
            hrtime.bigint = () => __viper_hrtime_bigint();

            // memoryUsage function with rss method
            function memoryUsage() {
                return __viper_memory_usage();
            }
            memoryUsage.rss = () => __viper_memory_usage().rss;

            // Create process object
            const process = {
                // Command line
                argv: __viper_argv || [],
                argv0: __viper_argv0 || 'viper',
                execArgv: [],
                execPath: __viper_argv?.[0] || 'viper',

                // Exit functions
                exit: (code = 0) => {
                    process.exitCode = code;
                    emit('beforeExit', code);
                    emit('exit', code);
                    __viper_exit(code);
                },
                abort: () => __viper_abort(),
                exitCode: undefined,

                // Working directory
                cwd: () => __viper_cwd(),
                chdir: (dir) => __viper_chdir(dir),

                // Environment
                env: envProxy,

                // Process info
                pid: __viper_pid || 0,
                ppid: __viper_ppid || 0,
                platform: __VIPER_PLATFORM__ || 'unknown',
                arch: __VIPER_ARCH__ || 'unknown',

                // Version info
                version: 'v' + (__VIPER_VERSION__ || '0.1.0'),
                versions: {
                    node: '20.0.0',  // Compatibility version
                    viper: __VIPER_VERSION__ || '0.1.0',
                    boa: '0.21.0',
                    oxc: '0.105.0',
                    v8: '0.0.0',
                    uv: '0.0.0',
                    zlib: '0.0.0',
                    modules: '0',
                },

                // Title
                title: 'viper',

                // Timing
                hrtime: hrtime,
                uptime: () => __viper_uptime(),

                // Resource usage
                memoryUsage: memoryUsage,
                cpuUsage: (prev) => __viper_cpu_usage(prev),
                resourceUsage: () => __viper_resource_usage(),

                // Process control
                kill: (pid, signal) => __viper_kill(pid, signal),

                // nextTick - highest priority microtask
                nextTick: (callback, ...args) => {
                    if (typeof callback !== 'function') {
                        throw new TypeError('callback must be a function');
                    }
                    queueMicrotask(() => callback.apply(null, args));
                },

                // Standard streams
                stdout: {
                    write: (data) => {
                        __viper_stdout_write(String(data));
                        return true;
                    },
                    fd: 1,
                    isTTY: __viper_isatty(1),
                    columns: 80,
                    rows: 24,
                },
                stderr: {
                    write: (data) => {
                        __viper_stderr_write(String(data));
                        return true;
                    },
                    fd: 2,
                    isTTY: __viper_isatty(2),
                    columns: 80,
                    rows: 24,
                },
                stdin: {
                    fd: 0,
                    isTTY: __viper_isatty(0),
                    resume: () => process.stdin,
                    pause: () => process.stdin,
                    setEncoding: () => process.stdin,
                },

                // Debug port
                debugPort: 9229,

                // Features
                features: {
                    inspector: false,
                    debug: false,
                    uv: true,
                    ipv6: true,
                    tls_alpn: false,
                    tls_sni: false,
                    tls_ocsp: false,
                    tls: false,
                    cached_builtins: true,
                },

                // Config (minimal)
                config: {
                    target_defaults: {},
                    variables: {},
                },

                // Release info
                release: {
                    name: 'viper',
                    sourceUrl: '',
                    headersUrl: '',
                },

                // Connected (for IPC)
                connected: false,

                // Deprecation flags
                noDeprecation: false,
                throwDeprecation: false,
                traceDeprecation: false,

                // Warning emission
                emitWarning: (warning, type, code, ctor) => {
                    if (typeof warning === 'string') {
                        const err = new Error(warning);
                        err.name = type || 'Warning';
                        if (code) err.code = code;
                        warning = err;
                    }
                    emit('warning', warning);
                },

                // Report (stub)
                report: {
                    compact: false,
                    directory: '',
                    filename: '',
                    getReport: () => ({}),
                    reportOnFatalError: false,
                    reportOnSignal: false,
                    reportOnUncaughtException: false,
                    signal: 'SIGUSR2',
                    writeReport: () => '',
                },

                // Allowed Node environment flags (stub)
                allowedNodeEnvironmentFlags: new Set(),

                // Event emitter methods
                on: on,
                once: once,
                off: off,
                addListener: on,
                removeListener: off,
                removeAllListeners: removeAllListeners,
                emit: emit,
                listeners: listeners,
                listenerCount: listenerCount,
                prependListener: (event, listener) => {
                    if (!_listeners[event]) _listeners[event] = [];
                    _listeners[event].unshift(listener);
                    return process;
                },
                prependOnceListener: (event, listener) => {
                    const wrapped = (...args) => {
                        off(event, wrapped);
                        listener.apply(process, args);
                    };
                    wrapped.listener = listener;
                    if (!_listeners[event]) _listeners[event] = [];
                    _listeners[event].unshift(wrapped);
                    return process;
                },
                setMaxListeners: (n) => { return process; },
                getMaxListeners: () => _maxListeners,
                rawListeners: listeners,
                eventNames: () => Object.keys(_listeners),

                // Uncaught exception handling
                setUncaughtExceptionCaptureCallback: (fn) => {
                    process._uncaughtExceptionCallback = fn;
                },
                hasUncaughtExceptionCaptureCallback: () => {
                    return typeof process._uncaughtExceptionCallback === 'function';
                },

                // Source maps (stub)
                setSourceMapsEnabled: (val) => {},
                sourceMapsEnabled: false,

                // Main module (deprecated)
                mainModule: undefined,

                // Channel (for IPC, stub)
                channel: undefined,
                disconnect: () => {},
                send: () => false,

                // Domain (deprecated, stub)
                domain: null,

                // Active handles/requests info
                getActiveResourcesInfo: () => [],

                // Get builtin module
                getBuiltinModule: (id) => {
                    const builtins = {
                        'fs': globalThis.fs,
                        'path': globalThis.path,
                        'events': globalThis.events,
                        'buffer': globalThis.buffer,
                        'stream': globalThis.stream,
                        'util': globalThis.util,
                        'crypto': globalThis.crypto,
                        'http': globalThis.http,
                        'net': globalThis.net,
                        'tty': globalThis.tty,
                    };
                    const key = id.replace(/^node:/, '');
                    return builtins[key];
                },

                // Constrained memory (stub)
                constrainedMemory: () => 0,
                availableMemory: () => __viper_available_memory(),
            };

            // Make process global
            globalThis.process = process;

            return process;
        })();
    "#;

    let source = Source::from_bytes(process_code.as_bytes());
    context.eval(source)?;

    Ok(())
}

/// Register all native process functions
fn register_native_process_functions(context: &mut Context) -> JsResult<()> {
    let global = context.global_object();

    // process.pid
    global.set(
        js_string!("__viper_pid"),
        JsValue::from(std::process::id() as i32),
        false,
        context,
    )?;

    // process.ppid (parent process ID)
    #[cfg(unix)]
    let ppid = unsafe { libc::getppid() as i32 };
    #[cfg(windows)]
    let ppid = {
        use std::mem::MaybeUninit;
        use windows_sys::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, PROCESSENTRY32, Process32First, Process32Next,
            TH32CS_SNAPPROCESS,
        };
        use windows_sys::Win32::System::Threading::GetCurrentProcessId;

        let current_pid = unsafe { GetCurrentProcessId() };
        let mut parent_pid = 0u32;

        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if !snapshot.is_null() && snapshot != -1isize as *mut _ {
                let mut entry = MaybeUninit::<PROCESSENTRY32>::uninit();
                let ptr = entry.as_mut_ptr();
                (*ptr).dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;

                if Process32First(snapshot, ptr) != 0 {
                    loop {
                        let e = entry.assume_init_ref();
                        if e.th32ProcessID == current_pid {
                            parent_pid = e.th32ParentProcessID;
                            break;
                        }
                        if Process32Next(snapshot, ptr) == 0 {
                            break;
                        }
                    }
                }
                windows_sys::Win32::Foundation::CloseHandle(snapshot);
            }
        }
        parent_pid as i32
    };
    #[cfg(not(any(unix, windows)))]
    let ppid = 0i32;

    global.set(
        js_string!("__viper_ppid"),
        JsValue::from(ppid),
        false,
        context,
    )?;

    // process.exit(code)
    let exit_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let code = args
            .get(0)
            .map(|v| v.to_i32(context))
            .transpose()?
            .unwrap_or(0);
        std::process::exit(code);
    });
    global.set(
        js_string!("__viper_exit"),
        exit_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // process.abort()
    let abort_fn = NativeFunction::from_fn_ptr(|_this, _args, _context| {
        std::process::abort();
    });
    global.set(
        js_string!("__viper_abort"),
        abort_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // process.cwd()
    let cwd_fn =
        NativeFunction::from_fn_ptr(|_this, _args, _context| match std::env::current_dir() {
            Ok(path) => Ok(JsValue::from(js_string!(
                path.to_string_lossy().to_string()
            ))),
            Err(_) => Ok(JsValue::from(js_string!("."))),
        });
    global.set(
        js_string!("__viper_cwd"),
        cwd_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // process.chdir(dir)
    let chdir_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let dir = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("directory argument required"))?
            .to_string(context)?
            .to_std_string_escaped();

        std::env::set_current_dir(&dir).map_err(|e| {
            JsNativeError::error().with_message(format!(
                "ENOENT: no such file or directory, chdir '{}'",
                dir
            ))
        })?;

        Ok(JsValue::undefined())
    });
    global.set(
        js_string!("__viper_chdir"),
        chdir_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // Environment variable functions
    let env_get_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let key = args
            .get(0)
            .map(|v| v.to_string(context))
            .transpose()?
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default();

        match std::env::var(&key) {
            Ok(val) => Ok(JsValue::from(js_string!(val))),
            Err(_) => Ok(JsValue::undefined()),
        }
    });
    global.set(
        js_string!("__viper_env_get"),
        env_get_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    let env_set_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let key = args
            .get(0)
            .map(|v| v.to_string(context))
            .transpose()?
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default();
        let value = args
            .get(1)
            .map(|v| v.to_string(context))
            .transpose()?
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default();

        unsafe {
            std::env::set_var(&key, &value);
        }
        Ok(JsValue::undefined())
    });
    global.set(
        js_string!("__viper_env_set"),
        env_set_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    let env_delete_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let key = args
            .get(0)
            .map(|v| v.to_string(context))
            .transpose()?
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default();

        unsafe {
            std::env::remove_var(&key);
        }
        Ok(JsValue::undefined())
    });
    global.set(
        js_string!("__viper_env_delete"),
        env_delete_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    let env_all_fn = NativeFunction::from_fn_ptr(|_this, _args, context| {
        let obj = ObjectInitializer::new(context).build();
        for (key, value) in std::env::vars() {
            obj.set(
                js_string!(key),
                JsValue::from(js_string!(value)),
                false,
                context,
            )?;
        }
        Ok(JsValue::from(obj))
    });
    global.set(
        js_string!("__viper_env_all"),
        env_all_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // process.hrtime([time]) - high-resolution time
    let hrtime_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();

        let secs = now.as_secs();
        let nanos = now.subsec_nanos();

        // If previous time provided, calculate difference
        if let Some(prev) = args.get(0) {
            if !prev.is_undefined() && !prev.is_null() {
                if let Some(prev_arr) = prev.as_object() {
                    let prev_secs = prev_arr.get(0, context)?.to_number(context)? as u64;
                    let prev_nanos = prev_arr.get(1, context)?.to_number(context)? as u32;

                    let total_nanos = secs * 1_000_000_000 + nanos as u64;
                    let prev_total_nanos = prev_secs * 1_000_000_000 + prev_nanos as u64;
                    let diff_nanos = total_nanos.saturating_sub(prev_total_nanos);

                    let diff_secs = diff_nanos / 1_000_000_000;
                    let diff_remaining_nanos = (diff_nanos % 1_000_000_000) as u32;

                    let result = JsArray::new(context);
                    result.push(JsValue::from(diff_secs as f64), context)?;
                    result.push(JsValue::from(diff_remaining_nanos as f64), context)?;
                    return Ok(result.into());
                }
            }
        }

        let result = JsArray::new(context);
        result.push(JsValue::from(secs as f64), context)?;
        result.push(JsValue::from(nanos as f64), context)?;
        Ok(result.into())
    });
    global.set(
        js_string!("__viper_hrtime"),
        hrtime_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // process.hrtime.bigint() - returns nanoseconds as BigInt
    let hrtime_bigint_fn = NativeFunction::from_fn_ptr(|_this, _args, context| {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();

        let nanos = now.as_nanos();

        // Return as BigInt
        let bigint = boa_engine::JsBigInt::from(nanos as i64);
        Ok(JsValue::from(bigint))
    });
    global.set(
        js_string!("__viper_hrtime_bigint"),
        hrtime_bigint_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // process.uptime() - seconds since process started
    let uptime_fn =
        NativeFunction::from_fn_ptr(|_this, _args, _context| Ok(JsValue::from(get_uptime())));
    global.set(
        js_string!("__viper_uptime"),
        uptime_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // process.memoryUsage() - memory statistics
    let memory_usage_fn = NativeFunction::from_fn_ptr(|_this, _args, context| {
        let obj = ObjectInitializer::new(context).build();

        // Get RSS from system if possible
        #[cfg(unix)]
        let rss = {
            use std::fs;
            fs::read_to_string("/proc/self/statm")
                .ok()
                .and_then(|s| s.split_whitespace().nth(1)?.parse::<u64>().ok())
                .map(|pages| pages * 4096) // page size typically 4KB
                .unwrap_or(0)
        };

        #[cfg(windows)]
        let rss = {
            use std::mem::MaybeUninit;
            use windows_sys::Win32::System::ProcessStatus::{
                GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
            };
            use windows_sys::Win32::System::Threading::GetCurrentProcess;

            let mut pmc = MaybeUninit::<PROCESS_MEMORY_COUNTERS>::uninit();
            unsafe {
                let handle = GetCurrentProcess();
                let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
                if GetProcessMemoryInfo(handle, pmc.as_mut_ptr(), size) != 0 {
                    let pmc = pmc.assume_init();
                    pmc.WorkingSetSize as u64
                } else {
                    0u64
                }
            }
        };

        #[cfg(not(any(unix, windows)))]
        let rss = 0u64;

        obj.set(js_string!("rss"), JsValue::from(rss as f64), false, context)?;
        obj.set(js_string!("heapTotal"), JsValue::from(0), false, context)?;
        obj.set(js_string!("heapUsed"), JsValue::from(0), false, context)?;
        obj.set(js_string!("external"), JsValue::from(0), false, context)?;
        obj.set(js_string!("arrayBuffers"), JsValue::from(0), false, context)?;

        Ok(JsValue::from(obj))
    });
    global.set(
        js_string!("__viper_memory_usage"),
        memory_usage_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // process.cpuUsage([previousValue])
    let cpu_usage_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        #[cfg(unix)]
        let (user, system) = {
            use std::mem::MaybeUninit;
            let mut usage = MaybeUninit::<libc::rusage>::uninit();
            unsafe {
                if libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) == 0 {
                    let usage = usage.assume_init();
                    let user =
                        usage.ru_utime.tv_sec as u64 * 1_000_000 + usage.ru_utime.tv_usec as u64;
                    let system =
                        usage.ru_stime.tv_sec as u64 * 1_000_000 + usage.ru_stime.tv_usec as u64;
                    (user, system)
                } else {
                    (0u64, 0u64)
                }
            }
        };

        #[cfg(windows)]
        let (user, system) = {
            use std::mem::MaybeUninit;
            use windows_sys::Win32::Foundation::FILETIME;
            use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

            let mut creation = MaybeUninit::<FILETIME>::uninit();
            let mut exit = MaybeUninit::<FILETIME>::uninit();
            let mut kernel = MaybeUninit::<FILETIME>::uninit();
            let mut user_time = MaybeUninit::<FILETIME>::uninit();

            unsafe {
                let handle = GetCurrentProcess();
                if GetProcessTimes(
                    handle,
                    creation.as_mut_ptr(),
                    exit.as_mut_ptr(),
                    kernel.as_mut_ptr(),
                    user_time.as_mut_ptr(),
                ) != 0
                {
                    let kernel = kernel.assume_init();
                    let user_time = user_time.assume_init();

                    // FILETIME is in 100-nanosecond intervals, convert to microseconds
                    let kernel_us =
                        ((kernel.dwHighDateTime as u64) << 32 | kernel.dwLowDateTime as u64) / 10;
                    let user_us = ((user_time.dwHighDateTime as u64) << 32
                        | user_time.dwLowDateTime as u64)
                        / 10;

                    (user_us, kernel_us)
                } else {
                    (0u64, 0u64)
                }
            }
        };

        #[cfg(not(any(unix, windows)))]
        let (user, system) = (0u64, 0u64);

        // If previous value provided, calculate diff
        let (user, system) = if let Some(prev) = args.get(0) {
            if let Some(prev_obj) = prev.as_object() {
                let prev_user = prev_obj
                    .get(js_string!("user"), context)?
                    .to_number(context)? as u64;
                let prev_system = prev_obj
                    .get(js_string!("system"), context)?
                    .to_number(context)? as u64;
                (
                    user.saturating_sub(prev_user),
                    system.saturating_sub(prev_system),
                )
            } else {
                (user, system)
            }
        } else {
            (user, system)
        };

        let obj = ObjectInitializer::new(context).build();
        obj.set(
            js_string!("user"),
            JsValue::from(user as f64),
            false,
            context,
        )?;
        obj.set(
            js_string!("system"),
            JsValue::from(system as f64),
            false,
            context,
        )?;

        Ok(JsValue::from(obj))
    });
    global.set(
        js_string!("__viper_cpu_usage"),
        cpu_usage_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // process.resourceUsage()
    let resource_usage_fn = NativeFunction::from_fn_ptr(|_this, _args, context| {
        let obj = ObjectInitializer::new(context).build();

        #[cfg(unix)]
        {
            use std::mem::MaybeUninit;
            let mut usage = MaybeUninit::<libc::rusage>::uninit();
            unsafe {
                if libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) == 0 {
                    let usage = usage.assume_init();
                    obj.set(
                        js_string!("userCPUTime"),
                        JsValue::from(
                            (usage.ru_utime.tv_sec * 1_000_000 + usage.ru_utime.tv_usec) as f64,
                        ),
                        false,
                        context,
                    )?;
                    obj.set(
                        js_string!("systemCPUTime"),
                        JsValue::from(
                            (usage.ru_stime.tv_sec * 1_000_000 + usage.ru_stime.tv_usec) as f64,
                        ),
                        false,
                        context,
                    )?;
                    obj.set(
                        js_string!("maxRSS"),
                        JsValue::from(usage.ru_maxrss as f64),
                        false,
                        context,
                    )?;
                    obj.set(
                        js_string!("sharedMemorySize"),
                        JsValue::from(usage.ru_ixrss as f64),
                        false,
                        context,
                    )?;
                    obj.set(
                        js_string!("unsharedDataSize"),
                        JsValue::from(usage.ru_idrss as f64),
                        false,
                        context,
                    )?;
                    obj.set(
                        js_string!("unsharedStackSize"),
                        JsValue::from(usage.ru_isrss as f64),
                        false,
                        context,
                    )?;
                    obj.set(
                        js_string!("minorPageFault"),
                        JsValue::from(usage.ru_minflt as f64),
                        false,
                        context,
                    )?;
                    obj.set(
                        js_string!("majorPageFault"),
                        JsValue::from(usage.ru_majflt as f64),
                        false,
                        context,
                    )?;
                    obj.set(
                        js_string!("swappedOut"),
                        JsValue::from(usage.ru_nswap as f64),
                        false,
                        context,
                    )?;
                    obj.set(
                        js_string!("fsRead"),
                        JsValue::from(usage.ru_inblock as f64),
                        false,
                        context,
                    )?;
                    obj.set(
                        js_string!("fsWrite"),
                        JsValue::from(usage.ru_oublock as f64),
                        false,
                        context,
                    )?;
                    obj.set(
                        js_string!("ipcSent"),
                        JsValue::from(usage.ru_msgsnd as f64),
                        false,
                        context,
                    )?;
                    obj.set(
                        js_string!("ipcReceived"),
                        JsValue::from(usage.ru_msgrcv as f64),
                        false,
                        context,
                    )?;
                    obj.set(
                        js_string!("signalsCount"),
                        JsValue::from(usage.ru_nsignals as f64),
                        false,
                        context,
                    )?;
                    obj.set(
                        js_string!("voluntaryContextSwitches"),
                        JsValue::from(usage.ru_nvcsw as f64),
                        false,
                        context,
                    )?;
                    obj.set(
                        js_string!("involuntaryContextSwitches"),
                        JsValue::from(usage.ru_nivcsw as f64),
                        false,
                        context,
                    )?;
                }
            }
        }

        #[cfg(not(unix))]
        {
            // Windows: set all to 0
            obj.set(js_string!("userCPUTime"), JsValue::from(0), false, context)?;
            obj.set(
                js_string!("systemCPUTime"),
                JsValue::from(0),
                false,
                context,
            )?;
            obj.set(js_string!("maxRSS"), JsValue::from(0), false, context)?;
            obj.set(
                js_string!("sharedMemorySize"),
                JsValue::from(0),
                false,
                context,
            )?;
            obj.set(
                js_string!("unsharedDataSize"),
                JsValue::from(0),
                false,
                context,
            )?;
            obj.set(
                js_string!("unsharedStackSize"),
                JsValue::from(0),
                false,
                context,
            )?;
            obj.set(
                js_string!("minorPageFault"),
                JsValue::from(0),
                false,
                context,
            )?;
            obj.set(
                js_string!("majorPageFault"),
                JsValue::from(0),
                false,
                context,
            )?;
            obj.set(js_string!("swappedOut"), JsValue::from(0), false, context)?;
            obj.set(js_string!("fsRead"), JsValue::from(0), false, context)?;
            obj.set(js_string!("fsWrite"), JsValue::from(0), false, context)?;
            obj.set(js_string!("ipcSent"), JsValue::from(0), false, context)?;
            obj.set(js_string!("ipcReceived"), JsValue::from(0), false, context)?;
            obj.set(js_string!("signalsCount"), JsValue::from(0), false, context)?;
            obj.set(
                js_string!("voluntaryContextSwitches"),
                JsValue::from(0),
                false,
                context,
            )?;
            obj.set(
                js_string!("involuntaryContextSwitches"),
                JsValue::from(0),
                false,
                context,
            )?;
        }

        Ok(JsValue::from(obj))
    });
    global.set(
        js_string!("__viper_resource_usage"),
        resource_usage_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // process.kill(pid, signal)
    let kill_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let pid = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("pid argument required"))?
            .to_i32(context)?;

        let signal: i32 = if let Some(v) = args.get(1) {
            if v.is_number() {
                v.to_i32(context)?
            } else if v.is_string() {
                let sig_name = v.to_string(context)?.to_std_string_escaped();
                // Convert signal name to number
                match sig_name.as_str() {
                    "SIGTERM" | "15" => 15,
                    "SIGKILL" | "9" => 9,
                    "SIGINT" | "2" => 2,
                    "SIGHUP" | "1" => 1,
                    "SIGUSR1" | "10" => 10,
                    "SIGUSR2" | "12" => 12,
                    _ => 15, // Default to SIGTERM
                }
            } else {
                15 // SIGTERM
            }
        } else {
            15 // SIGTERM
        };

        #[cfg(unix)]
        {
            let result = unsafe { libc::kill(pid, signal) };
            if result != 0 {
                return Err(JsNativeError::error()
                    .with_message(format!("kill({}, {}) failed", pid, signal))
                    .into());
            }
        }

        #[cfg(windows)]
        {
            // Windows: use TerminateProcess
            use windows_sys::Win32::Foundation::CloseHandle;
            use windows_sys::Win32::System::Threading::{
                OpenProcess, PROCESS_TERMINATE, TerminateProcess,
            };

            unsafe {
                let handle = OpenProcess(PROCESS_TERMINATE, 0, pid as u32);
                if !handle.is_null() {
                    TerminateProcess(handle, 1);
                    CloseHandle(handle);
                }
            }
        }

        Ok(JsValue::from(true))
    });
    global.set(
        js_string!("__viper_kill"),
        kill_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // stdout.write
    let stdout_write_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let data = args
            .get(0)
            .map(|v| v.to_string(context))
            .transpose()?
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default();

        use std::io::Write;
        let _ = std::io::stdout().write_all(data.as_bytes());
        let _ = std::io::stdout().flush();

        Ok(JsValue::from(true))
    });
    global.set(
        js_string!("__viper_stdout_write"),
        stdout_write_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // stderr.write
    let stderr_write_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let data = args
            .get(0)
            .map(|v| v.to_string(context))
            .transpose()?
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default();

        use std::io::Write;
        let _ = std::io::stderr().write_all(data.as_bytes());
        let _ = std::io::stderr().flush();

        Ok(JsValue::from(true))
    });
    global.set(
        js_string!("__viper_stderr_write"),
        stderr_write_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // isatty check
    let isatty_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let fd = args
            .get(0)
            .map(|v| v.to_i32(context))
            .transpose()?
            .unwrap_or(-1);

        #[cfg(unix)]
        let result = unsafe { libc::isatty(fd) != 0 };

        #[cfg(windows)]
        let result = {
            use windows_sys::Win32::System::Console::{
                GetConsoleMode, GetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
            };

            let handle = match fd {
                0 => unsafe { GetStdHandle(STD_INPUT_HANDLE) },
                1 => unsafe { GetStdHandle(STD_OUTPUT_HANDLE) },
                2 => unsafe { GetStdHandle(STD_ERROR_HANDLE) },
                _ => return Ok(JsValue::from(false)),
            };

            let mut mode = 0u32;
            unsafe { GetConsoleMode(handle, &mut mode) != 0 }
        };

        #[cfg(not(any(unix, windows)))]
        let result = false;

        Ok(JsValue::from(result))
    });
    global.set(
        js_string!("__viper_isatty"),
        isatty_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // Available memory
    let available_memory_fn = NativeFunction::from_fn_ptr(|_this, _args, _context| {
        #[cfg(unix)]
        let available = {
            use std::fs;
            fs::read_to_string("/proc/meminfo")
                .ok()
                .and_then(|s| {
                    for line in s.lines() {
                        if line.starts_with("MemAvailable:") {
                            return line
                                .split_whitespace()
                                .nth(1)?
                                .parse::<u64>()
                                .ok()
                                .map(|kb| kb * 1024);
                        }
                    }
                    None
                })
                .unwrap_or(0)
        };

        #[cfg(windows)]
        let available = {
            use std::mem::MaybeUninit;
            use windows_sys::Win32::System::SystemInformation::{
                GlobalMemoryStatusEx, MEMORYSTATUSEX,
            };

            let mut mem_info = MaybeUninit::<MEMORYSTATUSEX>::uninit();
            unsafe {
                let ptr = mem_info.as_mut_ptr();
                (*ptr).dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
                if GlobalMemoryStatusEx(ptr) != 0 {
                    (*ptr).ullAvailPhys
                } else {
                    0u64
                }
            }
        };

        #[cfg(not(any(unix, windows)))]
        let available = 0u64;

        Ok(JsValue::from(available as f64))
    });
    global.set(
        js_string!("__viper_available_memory"),
        available_memory_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    Ok(())
}
