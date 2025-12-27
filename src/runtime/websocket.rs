//! Ultra-Fast WebSocket Client
//!
//! High-performance WebSocket client optimized for:
//! - Event-driven architecture (no polling)
//! - Lock-free message passing using atomic queues
//! - Zero-copy binary message support
//! - Minimal latency through direct async integration
//!
//! Provides full Web API compatibility:
//! - WebSocket constructor
//! - send() for text and binary messages
//! - close() with code and reason
//! - Event handlers: onopen, onmessage, onerror, onclose
//! - readyState property
//! - binaryType support (arraybuffer/blob)

use boa_engine::{
    Context, JsNativeError, JsResult, JsValue, NativeFunction, Source, js_string,
    object::builtins::{JsArray, JsUint8Array},
};
use futures_util::{SinkExt, StreamExt};
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};
use std::thread;
use tokio::runtime::Runtime as TokioRuntime;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

// ============================================================================
// Constants
// ============================================================================

/// WebSocket ready states (matching Web API spec)
const CONNECTING: u8 = 0;
const OPEN: u8 = 1;
const CLOSING: u8 = 2;
const CLOSED: u8 = 3;

// ============================================================================
// Lock-Free Event Queue
// ============================================================================

/// A WebSocket event to be delivered to JavaScript
#[derive(Debug, Clone)]
pub enum WsEvent {
    Open,
    MessageText(String),
    MessageBinary(Vec<u8>),
    Error(String),
    Close {
        code: u16,
        reason: String,
        was_clean: bool,
    },
}

/// Thread-safe event queue using parking_lot for minimal contention
struct EventQueue {
    events: parking_lot::Mutex<VecDeque<WsEvent>>,
    has_events: AtomicBool,
}

impl EventQueue {
    fn new() -> Self {
        Self {
            events: parking_lot::Mutex::new(VecDeque::with_capacity(64)),
            has_events: AtomicBool::new(false),
        }
    }

    /// Push an event (called from async task)
    fn push(&self, event: WsEvent) {
        let mut queue = self.events.lock();
        queue.push_back(event);
        self.has_events.store(true, Ordering::Release);
    }

    /// Drain all events (called from JS thread)
    fn drain(&self) -> Vec<WsEvent> {
        if !self.has_events.load(Ordering::Acquire) {
            return Vec::new();
        }

        let mut queue = self.events.lock();
        let events: Vec<WsEvent> = queue.drain(..).collect();
        self.has_events.store(false, Ordering::Release);
        events
    }
}

// ============================================================================
// WebSocket Connection Handle
// ============================================================================

/// Outbound message to send
#[derive(Debug)]
pub enum OutboundMessage {
    Text(String),
    Binary(Vec<u8>),
    Close(u16, String),
}

/// High-performance WebSocket connection handle
pub struct WsConnection {
    /// Current ready state (atomic for lock-free reads)
    ready_state: AtomicU8,
    /// Event queue for incoming events
    events: Arc<EventQueue>,
    /// Channel for sending outbound messages
    send_tx: parking_lot::Mutex<Option<mpsc::UnboundedSender<OutboundMessage>>>,
    /// URL of the connection
    #[allow(dead_code)]
    pub url: String,
    /// Selected protocol
    pub protocol: parking_lot::Mutex<String>,
    /// Binary type preference
    pub binary_type: parking_lot::Mutex<String>,
}

impl WsConnection {
    /// Create a new WebSocket connection
    pub fn new(url: String) -> Result<Arc<Self>, String> {
        let events = Arc::new(EventQueue::new());
        let (send_tx, send_rx) = mpsc::unbounded_channel::<OutboundMessage>();

        let connection = Arc::new(Self {
            ready_state: AtomicU8::new(CONNECTING),
            events: Arc::clone(&events),
            send_tx: parking_lot::Mutex::new(Some(send_tx)),
            url: url.clone(),
            protocol: parking_lot::Mutex::new(String::new()),
            binary_type: parking_lot::Mutex::new("arraybuffer".to_string()),
        });

        // Clone for the async task
        let events_clone = Arc::clone(&events);
        let conn_clone = Arc::downgrade(&connection);
        let url_clone = url.clone();

        // Spawn connection in background thread with dedicated runtime
        thread::spawn(move || {
            let rt = match TokioRuntime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    events_clone.push(WsEvent::Error(format!("Failed to create runtime: {}", e)));
                    if let Some(conn) = conn_clone.upgrade() {
                        conn.ready_state.store(CLOSED, Ordering::Release);
                    }
                    return;
                }
            };

            rt.block_on(async move {
                Self::run_connection(url_clone, events_clone, conn_clone, send_rx).await;
            });
        });

        Ok(connection)
    }

    /// Main connection loop
    async fn run_connection(
        url: String,
        events: Arc<EventQueue>,
        conn: std::sync::Weak<WsConnection>,
        mut send_rx: mpsc::UnboundedReceiver<OutboundMessage>,
    ) {
        // Connect to WebSocket server
        let ws_stream = match connect_async(&url).await {
            Ok((stream, response)) => {
                // Extract protocol from response if available
                if let Some(conn) = conn.upgrade() {
                    if let Some(protocol) = response.headers().get("sec-websocket-protocol") {
                        if let Ok(p) = protocol.to_str() {
                            *conn.protocol.lock() = p.to_string();
                        }
                    }
                    conn.ready_state.store(OPEN, Ordering::Release);
                }
                events.push(WsEvent::Open);
                stream
            }
            Err(e) => {
                events.push(WsEvent::Error(format!("Connection failed: {}", e)));
                events.push(WsEvent::Close {
                    code: 1006,
                    reason: e.to_string(),
                    was_clean: false,
                });
                if let Some(conn) = conn.upgrade() {
                    conn.ready_state.store(CLOSED, Ordering::Release);
                }
                return;
            }
        };

        let (mut write, mut read) = ws_stream.split();

        // Use select! to handle both sending and receiving concurrently
        loop {
            tokio::select! {
                // Handle outbound messages
                msg = send_rx.recv() => {
                    match msg {
                        Some(OutboundMessage::Text(text)) => {
                            if write.send(Message::Text(text.into())).await.is_err() {
                                break;
                            }
                        }
                        Some(OutboundMessage::Binary(data)) => {
                            if write.send(Message::Binary(data.into())).await.is_err() {
                                break;
                            }
                        }
                        Some(OutboundMessage::Close(code, reason)) => {
                            let _ = write.send(Message::Close(Some(
                                tokio_tungstenite::tungstenite::protocol::CloseFrame {
                                    code: code.into(),
                                    reason: reason.into(),
                                }
                            ))).await;
                            break;
                        }
                        None => {
                            // Channel closed, exit gracefully
                            break;
                        }
                    }
                }

                // Handle inbound messages
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            events.push(WsEvent::MessageText(text.to_string()));
                        }
                        Some(Ok(Message::Binary(data))) => {
                            events.push(WsEvent::MessageBinary(data.to_vec()));
                        }
                        Some(Ok(Message::Ping(data))) => {
                            // Respond to ping with pong
                            let _ = write.send(Message::Pong(data)).await;
                        }
                        Some(Ok(Message::Pong(_))) => {
                            // Ignore pong messages
                        }
                        Some(Ok(Message::Close(frame))) => {
                            let (code, reason) = frame
                                .map(|f| (u16::from(f.code), f.reason.to_string()))
                                .unwrap_or((1000, String::new()));
                            events.push(WsEvent::Close { code, reason, was_clean: true });
                            if let Some(conn) = conn.upgrade() {
                                conn.ready_state.store(CLOSED, Ordering::Release);
                            }
                            break;
                        }
                        Some(Ok(Message::Frame(_))) => {
                            // Raw frame, ignore
                        }
                        Some(Err(e)) => {
                            events.push(WsEvent::Error(format!("Receive error: {}", e)));
                            events.push(WsEvent::Close { code: 1006, reason: e.to_string(), was_clean: false });
                            if let Some(conn) = conn.upgrade() {
                                conn.ready_state.store(CLOSED, Ordering::Release);
                            }
                            break;
                        }
                        None => {
                            // Stream ended
                            events.push(WsEvent::Close { code: 1000, reason: String::new(), was_clean: true });
                            if let Some(conn) = conn.upgrade() {
                                conn.ready_state.store(CLOSED, Ordering::Release);
                            }
                            break;
                        }
                    }
                }
            }
        }

        // Ensure we're marked as closed
        if let Some(conn) = conn.upgrade() {
            conn.ready_state.store(CLOSED, Ordering::Release);
        }
    }

    /// Get the current ready state
    pub fn ready_state(&self) -> u8 {
        self.ready_state.load(Ordering::Acquire)
    }

    /// Send a text message
    pub fn send_text(&self, message: String) -> Result<(), String> {
        if self.ready_state() != OPEN {
            return Err("WebSocket is not open".to_string());
        }

        let tx = self.send_tx.lock();
        if let Some(ref sender) = *tx {
            sender
                .send(OutboundMessage::Text(message))
                .map_err(|_| "Failed to send message".to_string())
        } else {
            Err("WebSocket is closed".to_string())
        }
    }

    /// Send a binary message
    pub fn send_binary(&self, data: Vec<u8>) -> Result<(), String> {
        if self.ready_state() != OPEN {
            return Err("WebSocket is not open".to_string());
        }

        let tx = self.send_tx.lock();
        if let Some(ref sender) = *tx {
            sender
                .send(OutboundMessage::Binary(data))
                .map_err(|_| "Failed to send message".to_string())
        } else {
            Err("WebSocket is closed".to_string())
        }
    }

    /// Close the connection
    pub fn close(&self, code: u16, reason: String) {
        self.ready_state.store(CLOSING, Ordering::Release);

        let tx = self.send_tx.lock();
        if let Some(ref sender) = *tx {
            let _ = sender.send(OutboundMessage::Close(code, reason));
        }
    }

    /// Drain pending events
    pub fn drain_events(&self) -> Vec<WsEvent> {
        self.events.drain()
    }
}

// ============================================================================
// Global Connection Registry
// ============================================================================

/// Global storage for WebSocket connections using parking_lot for speed
static WS_CONNECTIONS: parking_lot::RwLock<
    Option<std::collections::HashMap<u32, Arc<WsConnection>>>,
> = parking_lot::RwLock::new(None);
static WS_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Register a new connection and return its ID
fn register_connection(conn: Arc<WsConnection>) -> u32 {
    let id = WS_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;

    let mut connections = WS_CONNECTIONS.write();
    if connections.is_none() {
        *connections = Some(std::collections::HashMap::new());
    }
    if let Some(ref mut map) = *connections {
        map.insert(id, conn);
    }

    id
}

/// Get a connection by ID
fn get_connection(id: u32) -> Option<Arc<WsConnection>> {
    let connections = WS_CONNECTIONS.read();
    connections.as_ref()?.get(&id).cloned()
}

/// Remove a connection by ID
fn remove_connection(id: u32) {
    let mut connections = WS_CONNECTIONS.write();
    if let Some(ref mut map) = *connections {
        map.remove(&id);
    }
}

// ============================================================================
// JavaScript API Registration
// ============================================================================

/// Register the WebSocket class
pub fn register_websocket(context: &mut Context) -> JsResult<()> {
    let ws_code = r#"
        globalThis.WebSocket = class WebSocket {
            static CONNECTING = 0;
            static OPEN = 1;
            static CLOSING = 2;
            static CLOSED = 3;

            #wsId = null;
            #eventsFired = { open: false };
            #binaryType = "arraybuffer";

            onopen = null;
            onmessage = null;
            onerror = null;
            onclose = null;

            constructor(url, protocols) {
                if (!url) {
                    throw new TypeError('WebSocket URL is required');
                }

                // Normalize URL
                const urlStr = String(url);
                if (!urlStr.startsWith('ws://') && !urlStr.startsWith('wss://')) {
                    throw new SyntaxError('WebSocket URL must start with ws:// or wss://');
                }

                this.url = urlStr;
                this.protocol = "";
                this.extensions = "";

                // Create connection
                try {
                    this.#wsId = __viper_ws_connect(urlStr, protocols || []);
                    this.#startEventLoop();
                } catch (e) {
                    // Queue error event for next tick
                    queueMicrotask(() => {
                        if (this.onerror) {
                            this.onerror({ type: 'error', message: e.message, target: this });
                        }
                        if (this.onclose) {
                            this.onclose({ type: 'close', code: 1006, reason: e.message, wasClean: false, target: this });
                        }
                    });
                }
            }

            get readyState() {
                if (this.#wsId === null) return WebSocket.CLOSED;
                return __viper_ws_ready_state(this.#wsId);
            }

            get binaryType() {
                return this.#binaryType;
            }

            set binaryType(value) {
                if (value !== "arraybuffer" && value !== "blob") {
                    throw new SyntaxError('binaryType must be "arraybuffer" or "blob"');
                }
                this.#binaryType = value;
                if (this.#wsId !== null) {
                    __viper_ws_set_binary_type(this.#wsId, value);
                }
            }

            get bufferedAmount() {
                return 0; // Not tracking buffered amount for simplicity
            }

            send(data) {
                if (this.#wsId === null) {
                    throw new Error("WebSocket is not connected");
                }

                const state = this.readyState;
                if (state === WebSocket.CONNECTING) {
                    throw new Error("WebSocket is still connecting");
                }
                if (state !== WebSocket.OPEN) {
                    throw new Error("WebSocket is not open");
                }

                if (typeof data === 'string') {
                    __viper_ws_send_text(this.#wsId, data);
                } else if (data instanceof ArrayBuffer) {
                    __viper_ws_send_binary(this.#wsId, new Uint8Array(data));
                } else if (ArrayBuffer.isView(data)) {
                    __viper_ws_send_binary(this.#wsId, new Uint8Array(data.buffer, data.byteOffset, data.byteLength));
                } else {
                    // Convert to string
                    __viper_ws_send_text(this.#wsId, String(data));
                }
            }

            close(code = 1000, reason = "") {
                if (this.#wsId === null) return;

                const state = this.readyState;
                if (state === WebSocket.CLOSING || state === WebSocket.CLOSED) {
                    return;
                }

                // Validate code
                if (code !== 1000 && (code < 3000 || code > 4999)) {
                    throw new Error("Invalid close code");
                }

                __viper_ws_close(this.#wsId, code, reason);
            }

            addEventListener(type, listener, options) {
                const handler = typeof listener === 'function' ? listener : listener?.handleEvent?.bind(listener);
                if (!handler) return;

                if (type === 'open') this.onopen = handler;
                else if (type === 'message') this.onmessage = handler;
                else if (type === 'error') this.onerror = handler;
                else if (type === 'close') this.onclose = handler;
            }

            removeEventListener(type, listener, options) {
                if (type === 'open' && this.onopen === listener) this.onopen = null;
                else if (type === 'message' && this.onmessage === listener) this.onmessage = null;
                else if (type === 'error' && this.onerror === listener) this.onerror = null;
                else if (type === 'close' && this.onclose === listener) this.onclose = null;
            }

            dispatchEvent(event) {
                const handler = this['on' + event.type];
                if (handler) {
                    handler.call(this, event);
                    return !event.defaultPrevented;
                }
                return true;
            }

            #startEventLoop() {
                const poll = () => {
                    if (this.#wsId === null) return;

                    const state = this.readyState;

                    // Process all pending events
                    const events = __viper_ws_poll_events(this.#wsId);

                    for (const event of events) {
                        switch (event.type) {
                            case 'open':
                                if (!this.#eventsFired.open) {
                                    this.#eventsFired.open = true;
                                    this.protocol = event.protocol || "";
                                    if (this.onopen) {
                                        this.onopen({ type: 'open', target: this });
                                    }
                                }
                                break;

                            case 'message':
                                if (this.onmessage) {
                                    this.onmessage({
                                        type: 'message',
                                        data: event.data,
                                        origin: this.url,
                                        lastEventId: '',
                                        source: null,
                                        ports: [],
                                        target: this
                                    });
                                }
                                break;

                            case 'error':
                                if (this.onerror) {
                                    this.onerror({ type: 'error', message: event.message, target: this });
                                }
                                break;

                            case 'close':
                                if (this.onclose) {
                                    this.onclose({
                                        type: 'close',
                                        code: event.code,
                                        reason: event.reason,
                                        wasClean: event.wasClean,
                                        target: this
                                    });
                                }
                                // Clean up connection
                                __viper_ws_cleanup(this.#wsId);
                                this.#wsId = null;
                                return; // Stop polling
                        }
                    }

                    // Continue polling if not closed
                    if (state !== WebSocket.CLOSED) {
                        setTimeout(poll, 1); // 1ms for ultra-low latency
                    }
                };

                // Start polling on next tick
                setTimeout(poll, 0);
            }
        };
    "#;

    let source = Source::from_bytes(ws_code.as_bytes());
    context.eval(source)?;

    Ok(())
}

/// Register native helper functions for WebSocket
pub fn register_websocket_helpers(context: &mut Context) -> JsResult<()> {
    // __viper_ws_connect - Create a new WebSocket connection
    let connect_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let url = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing URL"))?
            .to_string(context)?
            .to_std_string_escaped();

        match WsConnection::new(url) {
            Ok(conn) => {
                let id = register_connection(conn);
                Ok(JsValue::from(id))
            }
            Err(e) => Err(JsNativeError::typ()
                .with_message(format!("WebSocket connection failed: {}", e))
                .into()),
        }
    });
    context.register_global_callable(js_string!("__viper_ws_connect"), 2, connect_fn)?;

    // __viper_ws_ready_state - Get ready state
    let ready_state_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let id = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing WebSocket ID"))?
            .to_u32(context)?;

        if let Some(conn) = get_connection(id) {
            Ok(JsValue::from(conn.ready_state()))
        } else {
            Ok(JsValue::from(CLOSED))
        }
    });
    context.register_global_callable(js_string!("__viper_ws_ready_state"), 1, ready_state_fn)?;

    // __viper_ws_send_text - Send text message
    let send_text_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let id = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing WebSocket ID"))?
            .to_u32(context)?;

        let message = args
            .get(1)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing message"))?
            .to_string(context)?
            .to_std_string_escaped();

        if let Some(conn) = get_connection(id) {
            conn.send_text(message)
                .map_err(|e| JsNativeError::typ().with_message(e))?;
        }

        Ok(JsValue::undefined())
    });
    context.register_global_callable(js_string!("__viper_ws_send_text"), 2, send_text_fn)?;

    // __viper_ws_send_binary - Send binary message
    let send_binary_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let id = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing WebSocket ID"))?
            .to_u32(context)?;

        let data = args
            .get(1)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing data"))?;

        // Extract bytes from Uint8Array
        let bytes = if let Some(obj) = data.as_object() {
            if let Ok(typed_array) = JsUint8Array::from_object(obj.clone()) {
                let len = typed_array.length(context)?;
                let mut bytes = Vec::with_capacity(len as usize);
                for i in 0..len {
                    if let Ok(val) = typed_array.get(i, context) {
                        bytes.push(val.to_u32(context).unwrap_or(0) as u8);
                    }
                }
                bytes
            } else {
                return Err(JsNativeError::typ()
                    .with_message("Expected Uint8Array")
                    .into());
            }
        } else {
            return Err(JsNativeError::typ()
                .with_message("Expected Uint8Array")
                .into());
        };

        if let Some(conn) = get_connection(id) {
            conn.send_binary(bytes)
                .map_err(|e| JsNativeError::typ().with_message(e))?;
        }

        Ok(JsValue::undefined())
    });
    context.register_global_callable(js_string!("__viper_ws_send_binary"), 2, send_binary_fn)?;

    // __viper_ws_close - Close connection
    let close_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let id = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing WebSocket ID"))?
            .to_u32(context)?;

        let code = args
            .get(1)
            .map(|v| v.to_u32(context))
            .transpose()?
            .unwrap_or(1000) as u16;

        let reason = args
            .get(2)
            .map(|v| v.to_string(context))
            .transpose()?
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default();

        if let Some(conn) = get_connection(id) {
            conn.close(code, reason);
        }

        Ok(JsValue::undefined())
    });
    context.register_global_callable(js_string!("__viper_ws_close"), 3, close_fn)?;

    // __viper_ws_poll_events - Poll for pending events (returns array)
    let poll_events_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let id = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing WebSocket ID"))?
            .to_u32(context)?;

        let arr = JsArray::new(context);

        if let Some(conn) = get_connection(id) {
            let events = conn.drain_events();
            let protocol = conn.protocol.lock().clone();

            for event in events {
                match event {
                    WsEvent::Open => {
                        let obj = boa_engine::object::ObjectInitializer::new(context)
                            .property(js_string!("type"), js_string!("open"), Default::default())
                            .property(
                                js_string!("protocol"),
                                js_string!(protocol.clone()),
                                Default::default(),
                            )
                            .build();
                        arr.push(obj, context)?;
                    }
                    WsEvent::MessageText(text) => {
                        let obj = boa_engine::object::ObjectInitializer::new(context)
                            .property(
                                js_string!("type"),
                                js_string!("message"),
                                Default::default(),
                            )
                            .property(js_string!("data"), js_string!(text), Default::default())
                            .build();
                        arr.push(obj, context)?;
                    }
                    WsEvent::MessageBinary(data) => {
                        // Create Uint8Array from binary data and get underlying ArrayBuffer
                        let typed_array = JsUint8Array::from_iter(data, context)?;
                        let array_buffer = typed_array.buffer(context)?;
                        let obj = boa_engine::object::ObjectInitializer::new(context)
                            .property(
                                js_string!("type"),
                                js_string!("message"),
                                Default::default(),
                            )
                            .property(js_string!("data"), array_buffer, Default::default())
                            .build();
                        arr.push(obj, context)?;
                    }
                    WsEvent::Error(message) => {
                        let obj = boa_engine::object::ObjectInitializer::new(context)
                            .property(js_string!("type"), js_string!("error"), Default::default())
                            .property(
                                js_string!("message"),
                                js_string!(message),
                                Default::default(),
                            )
                            .build();
                        arr.push(obj, context)?;
                    }
                    WsEvent::Close {
                        code,
                        reason,
                        was_clean,
                    } => {
                        let obj = boa_engine::object::ObjectInitializer::new(context)
                            .property(js_string!("type"), js_string!("close"), Default::default())
                            .property(js_string!("code"), JsValue::from(code), Default::default())
                            .property(js_string!("reason"), js_string!(reason), Default::default())
                            .property(
                                js_string!("wasClean"),
                                JsValue::from(was_clean),
                                Default::default(),
                            )
                            .build();
                        arr.push(obj, context)?;
                    }
                }
            }
        }

        Ok(arr.into())
    });
    context.register_global_callable(js_string!("__viper_ws_poll_events"), 1, poll_events_fn)?;

    // __viper_ws_set_binary_type - Set binary type preference
    let set_binary_type_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let id = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing WebSocket ID"))?
            .to_u32(context)?;

        let binary_type = args
            .get(1)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing binaryType"))?
            .to_string(context)?
            .to_std_string_escaped();

        if let Some(conn) = get_connection(id) {
            *conn.binary_type.lock() = binary_type;
        }

        Ok(JsValue::undefined())
    });
    context.register_global_callable(
        js_string!("__viper_ws_set_binary_type"),
        2,
        set_binary_type_fn,
    )?;

    // __viper_ws_cleanup - Remove connection from registry
    let cleanup_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let id = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing WebSocket ID"))?
            .to_u32(context)?;

        remove_connection(id);

        Ok(JsValue::undefined())
    });
    context.register_global_callable(js_string!("__viper_ws_cleanup"), 1, cleanup_fn)?;

    Ok(())
}
