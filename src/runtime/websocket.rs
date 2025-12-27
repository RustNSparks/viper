//! WebSocket API - Web-standard WebSocket client
//!
//! Uses tokio-tungstenite for reliable WebSocket client functionality with TLS support.
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
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::runtime::Runtime as TokioRuntime;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// WebSocket ready states
const READY_STATE_CONNECTING: u8 = 0;
const READY_STATE_OPEN: u8 = 1;
const READY_STATE_CLOSING: u8 = 2;
const READY_STATE_CLOSED: u8 = 3;

/// Global storage for WebSocket connections
static WS_CONNECTIONS: Mutex<Option<HashMap<u32, Arc<WsConnectionHandle>>>> = Mutex::new(None);
static WS_COUNTER: Mutex<u32> = Mutex::new(0);

/// WebSocket connection handle for communication between JS and async runtime
struct WsConnectionHandle {
    ready_state: Arc<Mutex<u8>>,
    received_messages: Arc<Mutex<Vec<String>>>,
    send_tx: Mutex<Option<mpsc::UnboundedSender<String>>>,
    error_message: Arc<Mutex<Option<String>>>,
}

impl WsConnectionHandle {
    fn new(url: String) -> Result<u32, String> {
        let ready_state = Arc::new(Mutex::new(READY_STATE_CONNECTING));
        let received_messages = Arc::new(Mutex::new(Vec::new()));
        let error_message = Arc::new(Mutex::new(None));
        let (send_tx, send_rx) = mpsc::unbounded_channel::<String>();

        let handle = Arc::new(WsConnectionHandle {
            ready_state: Arc::clone(&ready_state),
            received_messages: Arc::clone(&received_messages),
            send_tx: Mutex::new(Some(send_tx)),
            error_message: Arc::clone(&error_message),
        });

        // Get next ID
        let id = {
            let mut counter = WS_COUNTER.lock().unwrap();
            *counter += 1;
            *counter
        };

        // Store connection handle
        {
            let mut connections = WS_CONNECTIONS.lock().unwrap();
            if connections.is_none() {
                *connections = Some(HashMap::new());
            }
            if let Some(ref mut map) = *connections {
                map.insert(id, Arc::clone(&handle));
            }
        }

        // Start connection in background thread
        let ready_state_clone = Arc::clone(&ready_state);
        let messages_clone = Arc::clone(&received_messages);
        let error_clone = Arc::clone(&error_message);

        thread::spawn(move || {
            let rt = TokioRuntime::new().unwrap();
            rt.block_on(async {
                if let Err(e) = Self::run_connection(
                    &url,
                    ready_state_clone,
                    messages_clone,
                    error_clone,
                    send_rx,
                )
                .await
                {
                    eprintln!("WebSocket error: {}", e);
                }
            });
        });

        Ok(id)
    }

    async fn run_connection(
        url: &str,
        ready_state: Arc<Mutex<u8>>,
        messages: Arc<Mutex<Vec<String>>>,
        error_message: Arc<Mutex<Option<String>>>,
        mut send_rx: mpsc::UnboundedReceiver<String>,
    ) -> Result<(), String> {
        // Connect to WebSocket server
        let connect_result = connect_async(url).await;

        let (ws_stream, _response) = match connect_result {
            Ok(result) => result,
            Err(e) => {
                let err_msg = format!("Connection failed: {}", e);
                if let Ok(mut err) = error_message.lock() {
                    *err = Some(err_msg.clone());
                }
                if let Ok(mut state) = ready_state.lock() {
                    *state = READY_STATE_CLOSED;
                }
                return Err(err_msg);
            }
        };

        // Update state to OPEN
        if let Ok(mut state) = ready_state.lock() {
            *state = READY_STATE_OPEN;
        }

        let (mut write, mut read) = ws_stream.split();

        // Spawn task to handle sending messages
        let ready_state_send = Arc::clone(&ready_state);
        let send_task = tokio::spawn(async move {
            while let Some(msg) = send_rx.recv().await {
                if let Ok(state) = ready_state_send.lock() {
                    if *state != READY_STATE_OPEN {
                        break;
                    }
                }
                if let Err(e) = write.send(Message::Text(msg.into())).await {
                    eprintln!("Send error: {}", e);
                    break;
                }
            }
            // Try to close gracefully
            let _ = write.close().await;
        });

        // Handle receiving messages
        while let Some(msg_result) = read.next().await {
            match msg_result {
                Ok(msg) => {
                    match msg {
                        Message::Text(text) => {
                            if let Ok(mut msgs) = messages.lock() {
                                msgs.push(text.to_string());
                            }
                        }
                        Message::Binary(data) => {
                            // Convert binary to string
                            if let Ok(text) = String::from_utf8(data.to_vec()) {
                                if let Ok(mut msgs) = messages.lock() {
                                    msgs.push(text);
                                }
                            }
                        }
                        Message::Close(_) => {
                            if let Ok(mut state) = ready_state.lock() {
                                *state = READY_STATE_CLOSED;
                            }
                            break;
                        }
                        Message::Ping(_) | Message::Pong(_) => {
                            // Handled automatically by tungstenite
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    if let Ok(mut err) = error_message.lock() {
                        *err = Some(format!("Receive error: {}", e));
                    }
                    if let Ok(mut state) = ready_state.lock() {
                        *state = READY_STATE_CLOSED;
                    }
                    break;
                }
            }
        }

        // Update state to closed
        if let Ok(mut state) = ready_state.lock() {
            *state = READY_STATE_CLOSED;
        }

        send_task.abort();
        Ok(())
    }

    fn get(id: u32) -> Option<Arc<WsConnectionHandle>> {
        let connections = WS_CONNECTIONS.lock().ok()?;
        connections.as_ref()?.get(&id).cloned()
    }

    fn send(&self, message: String) -> Result<(), String> {
        if let Ok(tx_guard) = self.send_tx.lock() {
            if let Some(ref tx) = *tx_guard {
                tx.send(message)
                    .map_err(|e| format!("Send failed: {}", e))?;
                return Ok(());
            }
        }
        Err("WebSocket is not connected".to_string())
    }

    fn close(&self) {
        if let Ok(mut state) = self.ready_state.lock() {
            *state = READY_STATE_CLOSING;
        }
        // Drop the sender to signal closure
        if let Ok(mut tx_guard) = self.send_tx.lock() {
            *tx_guard = None;
        }
    }

    fn receive(&self) -> Vec<String> {
        if let Ok(mut msgs) = self.received_messages.lock() {
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

    fn get_error(&self) -> Option<String> {
        self.error_message.lock().ok().and_then(|e| e.clone())
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
            #hasError = false;

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
                    this.#hasError = true;
                    if (this.onerror) {
                        queueMicrotask(() => {
                            this.onerror({ type: 'error', message: e.message });
                        });
                    }
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

                    // Check for errors
                    if (!this.#hasError) {
                        const error = __viper_ws_get_error(this.#wsId);
                        if (error) {
                            this.#hasError = true;
                            if (this.onerror) {
                                this.onerror({ type: 'error', message: error });
                            }
                        }
                    }

                    // Check for connection open
                    if (!this.#isConnected && state === WebSocket.OPEN) {
                        this.#isConnected = true;
                        if (this.onopen) {
                            this.onopen({ type: 'open' });
                        }
                    }

                    // Check for closed
                    if (state === WebSocket.CLOSED) {
                        if (this.#isConnected && this.onclose) {
                            this.onclose({ type: 'close', code: 1000, reason: '', wasClean: true });
                        }
                        return;
                    }

                    // Poll for messages
                    if (state === WebSocket.OPEN) {
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
                    }

                    // Continue polling
                    if (state !== WebSocket.CLOSED) {
                        setTimeout(poll, 16);
                    }
                };

                setTimeout(poll, 50);
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

        match WsConnectionHandle::new(url) {
            Ok(id) => Ok(JsValue::from(id)),
            Err(e) => Err(JsNativeError::typ()
                .with_message(format!("WebSocket connection failed: {}", e))
                .into()),
        }
    });

    context.register_global_callable(js_string!("__viper_ws_connect"), 2, connect_fn)?;

    // __viper_ws_send
    let send_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let id = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing WebSocket ID"))?
            .to_u32(context)?;

        let data = args
            .get(1)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing data"))?
            .to_string(context)?
            .to_std_string_escaped();

        if let Some(handle) = WsConnectionHandle::get(id) {
            handle
                .send(data)
                .map_err(|e| JsNativeError::typ().with_message(e))?;
        }

        Ok(JsValue::undefined())
    });

    context.register_global_callable(js_string!("__viper_ws_send"), 2, send_fn)?;

    // __viper_ws_receive
    let receive_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let id = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing WebSocket ID"))?
            .to_u32(context)?;

        if let Some(handle) = WsConnectionHandle::get(id) {
            let messages = handle.receive();
            let arr = JsArray::new(context);
            for msg in messages {
                arr.push(JsValue::from(js_string!(msg)), context)?;
            }
            return Ok(arr.into());
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

        if let Some(handle) = WsConnectionHandle::get(id) {
            handle.close();
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

        if let Some(handle) = WsConnectionHandle::get(id) {
            return Ok(JsValue::from(handle.get_ready_state()));
        }

        Ok(JsValue::from(READY_STATE_CLOSED))
    });

    context.register_global_callable(js_string!("__viper_ws_get_state"), 1, get_state_fn)?;

    // __viper_ws_get_error
    let get_error_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let id = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing WebSocket ID"))?
            .to_u32(context)?;

        if let Some(handle) = WsConnectionHandle::get(id) {
            if let Some(error) = handle.get_error() {
                return Ok(JsValue::from(js_string!(error)));
            }
        }

        Ok(JsValue::null())
    });

    context.register_global_callable(js_string!("__viper_ws_get_error"), 1, get_error_fn)?;

    Ok(())
}
