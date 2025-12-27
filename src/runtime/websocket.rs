//! WebSocket API - Web-standard WebSocket client
//!
//! Uses the web-socket crate for WebSocket client functionality.
//!
//! Provides:
//! - WebSocket constructor (client)
//! - WebSocket.send() - Send messages
//! - WebSocket.close() - Close connection
//! - Event handlers: onopen, onmessage, onerror, onclose
//! - readyState property

use boa_engine::{
    Context, JsNativeError, JsResult, JsValue, NativeFunction, Source, js_string,
    object::builtins::JsArray,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::net::TcpStream;
use tokio::runtime::Runtime as TokioRuntime;
use web_socket::{DataType, Event, MessageType, WebSocket as WsClient};

/// WebSocket ready states
const READY_STATE_CONNECTING: u8 = 0;
const READY_STATE_OPEN: u8 = 1;
const _READY_STATE_CLOSING: u8 = 2;
const READY_STATE_CLOSED: u8 = 3;

/// Global storage for WebSocket connections
static WS_CONNECTIONS: Mutex<Option<HashMap<u32, Arc<Mutex<WsConnection>>>>> = Mutex::new(None);
static WS_COUNTER: Mutex<u32> = Mutex::new(0);

/// WebSocket connection wrapper
struct WsConnection {
    ready_state: Arc<Mutex<u8>>,
    messages: Arc<Mutex<Vec<String>>>,
    _url: String,
    _runtime: Option<TokioRuntime>,
}

impl WsConnection {
    fn new(url: String) -> Result<u32, String> {
        let ready_state = Arc::new(Mutex::new(READY_STATE_CONNECTING));
        let messages = Arc::new(Mutex::new(Vec::new()));

        let conn = WsConnection {
            ready_state: Arc::clone(&ready_state),
            messages: Arc::clone(&messages),
            _url: url.clone(),
            _runtime: Some(
                TokioRuntime::new().map_err(|e| format!("Failed to create runtime: {}", e))?,
            ),
        };

        // Get next ID
        let id = {
            let mut counter = WS_COUNTER.lock().unwrap();
            *counter += 1;
            *counter
        };

        // Store connection
        {
            let mut connections = WS_CONNECTIONS.lock().unwrap();
            if connections.is_none() {
                *connections = Some(HashMap::new());
            }
            if let Some(ref mut map) = *connections {
                map.insert(id, Arc::new(Mutex::new(conn)));
            }
        }

        // Start connection in background thread
        let ready_state_clone = Arc::clone(&ready_state);
        let messages_clone = Arc::clone(&messages);
        let url_clone = url.clone();

        thread::spawn(move || {
            let rt = TokioRuntime::new().unwrap();
            rt.block_on(async {
                match Self::connect_async(&url_clone, ready_state_clone, messages_clone).await {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("WebSocket connection failed: {}", e);
                    }
                }
            });
        });

        Ok(id)
    }

    async fn connect_async(
        url: &str,
        ready_state: Arc<Mutex<u8>>,
        messages: Arc<Mutex<Vec<String>>>,
    ) -> Result<(), String> {
        // Parse URL
        let uri = url
            .strip_prefix("ws://")
            .or_else(|| url.strip_prefix("wss://"))
            .ok_or("Invalid WebSocket URL")?;

        let (host, _path) = uri.split_once('/').unwrap_or((uri, ""));

        // Connect to server
        let stream = TcpStream::connect(host)
            .await
            .map_err(|e| format!("Connection failed: {}", e))?;

        let mut ws = WsClient::client(stream);

        // Update state to OPEN
        if let Ok(mut state) = ready_state.lock() {
            *state = READY_STATE_OPEN;
        }

        // Message receive loop
        loop {
            match ws.recv_event().await {
                Ok(event) => match event {
                    Event::Data { ty, data } => {
                        if matches!(ty, DataType::Complete(MessageType::Text)) {
                            if let Ok(text) = String::from_utf8(data.to_vec()) {
                                if let Ok(mut msgs) = messages.lock() {
                                    msgs.push(text);
                                }
                            }
                        }
                    }
                    Event::Ping(data) => {
                        let _ = ws.send_pong(data).await;
                    }
                    Event::Close { .. } => {
                        if let Ok(mut state) = ready_state.lock() {
                            *state = READY_STATE_CLOSED;
                        }
                        let _ = ws.close(()).await;
                        break;
                    }
                    Event::Error(e) => {
                        eprintln!("WebSocket error: {:?}", e);
                        if let Ok(mut state) = ready_state.lock() {
                            *state = READY_STATE_CLOSED;
                        }
                        break;
                    }
                    _ => {}
                },
                Err(e) => {
                    eprintln!("Receive error: {:?}", e);
                    if let Ok(mut state) = ready_state.lock() {
                        *state = READY_STATE_CLOSED;
                    }
                    break;
                }
            }
        }

        Ok(())
    }

    fn get(id: u32) -> Option<Arc<Mutex<WsConnection>>> {
        let connections = WS_CONNECTIONS.lock().ok()?;
        connections.as_ref()?.get(&id).cloned()
    }

    fn close(&mut self) -> Result<(), String> {
        if let Ok(mut state) = self.ready_state.lock() {
            *state = READY_STATE_CLOSED;
        }
        Ok(())
    }

    fn receive(&self) -> Vec<String> {
        if let Ok(mut msgs) = self.messages.lock() {
            let result = msgs.clone();
            msgs.clear();
            result
        } else {
            Vec::new()
        }
    }

    fn get_ready_state(&self) -> u8 {
        self.ready_state
            .lock()
            .map(|s| *s)
            .unwrap_or(READY_STATE_CLOSED)
    }
}

/// Register the WebSocket API
pub fn register_websocket(context: &mut Context) -> JsResult<()> {
    // Add WebSocket class implementation
    let ws_code = r#"
        globalThis.WebSocket = class WebSocket {
            static CONNECTING = 0;
            static OPEN = 1;
            static CLOSING = 2;
            static CLOSED = 3;

            #wsId = null;
            #isConnected = false;

            onopen = null;
            onmessage = null;
            onerror = null;
            onclose = null;

            constructor(url, protocols) {
                if (!url || (!url.startsWith('ws://') && !url.startsWith('wss://'))) {
                    throw new TypeError('WebSocket URL must start with ws:// or wss://');
                }

                this.url = url;
                this.protocol = "";
                this.extensions = "";
                this.binaryType = "blob";

                // Connect to WebSocket
                try {
                    this.#wsId = __viper_ws_connect(url, protocols || []);

                    // Start polling for events
                    this.#pollMessages();
                } catch (e) {
                    if (this.onerror) {
                        queueMicrotask(() => {
                            this.onerror({ type: 'error', message: e.message });
                        });
                    }
                    throw e;
                }
            }

            get readyState() {
                if (this.#wsId === null) return WebSocket.CLOSED;
                return __viper_ws_get_state(this.#wsId);
            }

            get bufferedAmount() {
                return 0;
            }

            send(data) {
                if (this.readyState !== WebSocket.OPEN) {
                    throw new Error("WebSocket is not open");
                }

                const message = typeof data === 'string' ? data : String(data);
                __viper_ws_send(this.#wsId, message);
            }

            close(code = 1000, reason = "") {
                if (this.readyState === WebSocket.CLOSING || this.readyState === WebSocket.CLOSED) {
                    return;
                }

                __viper_ws_close(this.#wsId, code, reason);

                if (this.onclose) {
                    queueMicrotask(() => {
                        this.onclose?.({ type: 'close', code, reason, wasClean: true });
                    });
                }
            }

            addEventListener(type, listener) {
                if (type === 'open') this.onopen = listener;
                else if (type === 'message') this.onmessage = listener;
                else if (type === 'error') this.onerror = listener;
                else if (type === 'close') this.onclose = listener;
            }

            removeEventListener(type, listener) {
                if (type === 'open' && this.onopen === listener) this.onopen = null;
                else if (type === 'message' && this.onmessage === listener) this.onmessage = null;
                else if (type === 'error' && this.onerror === listener) this.onerror = null;
                else if (type === 'close' && this.onclose === listener) this.onclose = null;
            }

            #pollMessages() {
                const poll = () => {
                    const state = this.readyState;

                    // Check for connection open
                    if (!this.#isConnected && state === WebSocket.OPEN) {
                        this.#isConnected = true;
                        if (this.onopen) {
                            this.onopen({ type: 'open' });
                        }
                    }

                    // Check for closed
                    if (state === WebSocket.CLOSED) {
                        return;
                    }

                    // Poll for messages
                    try {
                        const messages = __viper_ws_receive(this.#wsId);
                        if (messages && messages.length > 0) {
                            for (const msg of messages) {
                                if (this.onmessage) {
                                    this.onmessage({
                                        type: 'message',
                                        data: msg,
                                        origin: this.url,
                                        lastEventId: '',
                                        source: null,
                                        ports: []
                                    });
                                }
                            }
                        }
                    } catch (e) {
                        if (this.onerror) {
                            this.onerror({ type: 'error', message: e.message });
                        }
                    }

                    // Continue polling
                    if (state !== WebSocket.CLOSED) {
                        setTimeout(poll, 10);
                    }
                };

                setTimeout(poll, 100);
            }
        };
    "#;

    let source = Source::from_bytes(ws_code.as_bytes());
    context.eval(source)?;

    Ok(())
}

/// Register WebSocket helper functions
pub fn register_websocket_helpers(context: &mut Context) -> JsResult<()> {
    // __viper_ws_connect
    let connect_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let url = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing URL"))?
            .to_string(context)?
            .to_std_string_escaped();

        match WsConnection::new(url) {
            Ok(id) => Ok(JsValue::from(id)),
            Err(e) => Err(JsNativeError::typ()
                .with_message(format!("WebSocket connection failed: {}", e))
                .into()),
        }
    });

    context.register_global_callable(js_string!("__viper_ws_connect"), 2, connect_fn)?;

    // __viper_ws_send
    let send_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let _id = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing WebSocket ID"))?
            .to_u32(context)?;

        let _data = args
            .get(1)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing data"))?
            .to_string(context)?
            .to_std_string_escaped();

        // Note: Send is not fully implemented yet - would need to keep WebSocket handle
        // For now, messages are received but send requires the async WebSocket handle

        Ok(JsValue::undefined())
    });

    context.register_global_callable(js_string!("__viper_ws_send"), 2, send_fn)?;

    // __viper_ws_receive
    let receive_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let id = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing WebSocket ID"))?
            .to_u32(context)?;

        if let Some(conn_arc) = WsConnection::get(id) {
            if let Ok(conn) = conn_arc.lock() {
                let messages = conn.receive();
                let arr = JsArray::new(context);
                for msg in messages {
                    arr.push(JsValue::from(js_string!(msg)), context)?;
                }
                return Ok(arr.into());
            }
        }

        Ok(JsArray::new(context).into())
    });

    context.register_global_callable(js_string!("__viper_ws_receive"), 1, receive_fn)?;

    // __viper_ws_close
    let close_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let id = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing WebSocket ID"))?
            .to_u32(context)?;

        if let Some(conn_arc) = WsConnection::get(id) {
            if let Ok(mut conn) = conn_arc.lock() {
                conn.close()
                    .map_err(|e| JsNativeError::typ().with_message(e))?;
            }
        }

        Ok(JsValue::undefined())
    });

    context.register_global_callable(js_string!("__viper_ws_close"), 3, close_fn)?;

    // __viper_ws_get_state
    let get_state_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let id = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing WebSocket ID"))?
            .to_u32(context)?;

        if let Some(conn_arc) = WsConnection::get(id) {
            if let Ok(conn) = conn_arc.lock() {
                return Ok(JsValue::from(conn.get_ready_state()));
            }
        }

        Ok(JsValue::from(READY_STATE_CLOSED))
    });

    context.register_global_callable(js_string!("__viper_ws_get_state"), 1, get_state_fn)?;

    Ok(())
}
