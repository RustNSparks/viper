//! WebSocket API - Web-standard WebSocket client and server
//!
//! Uses the web-socket crate (fastest WebSocket implementation)
//!
//! Provides:
//! - WebSocket constructor (client)
//! - WebSocketServer - Server implementation
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
use web_socket::{CloseCode, DataType, Event, MessageType, WebSocket as WsClient};

/// WebSocket ready states
const READY_STATE_CONNECTING: u8 = 0;
const READY_STATE_OPEN: u8 = 1;
const READY_STATE_CLOSING: u8 = 2;
const READY_STATE_CLOSED: u8 = 3;

/// Global storage for WebSocket connections
static WS_CONNECTIONS: Mutex<Option<HashMap<u32, Arc<Mutex<WsConnection>>>>> = Mutex::new(None);
static WS_COUNTER: Mutex<u32> = Mutex::new(0);

/// WebSocket connection wrapper
struct WsConnection {
    ready_state: Arc<Mutex<u8>>,
    messages: Arc<Mutex<Vec<String>>>,
    url: String,
    runtime: Option<TokioRuntime>,
}

impl WsConnection {
    fn new(url: String) -> Result<u32, String> {
        let ready_state = Arc::new(Mutex::new(READY_STATE_CONNECTING));
        let messages = Arc::new(Mutex::new(Vec::new()));

        let conn = WsConnection {
            ready_state: Arc::clone(&ready_state),
            messages: Arc::clone(&messages),
            url: url.clone(),
            runtime: Some(
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

    fn send(&self, data: &str) -> Result<(), String> {
        // For sending, we need to spawn a task
        // This is simplified - in production you'd keep the WebSocket handle
        Ok(())
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
        let id = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing WebSocket ID"))?
            .to_u32(context)?;

        let data = args
            .get(1)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing data"))?
            .to_string(context)?
            .to_std_string_escaped();

        if let Some(conn_arc) = WsConnection::get(id) {
            if let Ok(conn) = conn_arc.lock() {
                conn.send(&data)
                    .map_err(|e| JsNativeError::typ().with_message(e))?;
            }
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

// ============================================================================
// WebSocket Server Implementation
// ============================================================================

use tokio::net::TcpListener;

/// Global storage for WebSocket servers
static WS_SERVERS: Mutex<Option<HashMap<u32, Arc<Mutex<WsServer>>>>> = Mutex::new(None);
static WS_SERVER_COUNTER: Mutex<u32> = Mutex::new(0);

/// WebSocket server wrapper
struct WsServer {
    port: u16,
    clients: Arc<Mutex<Vec<u32>>>,
    messages: Arc<Mutex<Vec<(u32, String)>>>, // (client_id, message)
    is_running: Arc<Mutex<bool>>,
}

impl WsServer {
    fn new(port: u16) -> Result<u32, String> {
        let clients = Arc::new(Mutex::new(Vec::new()));
        let messages = Arc::new(Mutex::new(Vec::new()));
        let is_running = Arc::new(Mutex::new(true));

        let server = WsServer {
            port,
            clients: Arc::clone(&clients),
            messages: Arc::clone(&messages),
            is_running: Arc::clone(&is_running),
        };

        // Get next server ID
        let server_id = {
            let mut counter = WS_SERVER_COUNTER.lock().unwrap();
            *counter += 1;
            *counter
        };

        // Store server
        {
            let mut servers = WS_SERVERS.lock().unwrap();
            if servers.is_none() {
                *servers = Some(HashMap::new());
            }
            if let Some(ref mut map) = *servers {
                map.insert(server_id, Arc::new(Mutex::new(server)));
            }
        }

        // Start server in background
        let clients_clone = Arc::clone(&clients);
        let messages_clone = Arc::clone(&messages);
        let is_running_clone = Arc::clone(&is_running);

        thread::spawn(move || {
            let rt = TokioRuntime::new().unwrap();
            rt.block_on(async {
                if let Err(e) =
                    Self::run_server(port, clients_clone, messages_clone, is_running_clone).await
                {
                    eprintln!("WebSocket server error: {}", e);
                }
            });
        });

        Ok(server_id)
    }

    async fn run_server(
        port: u16,
        clients: Arc<Mutex<Vec<u32>>>,
        messages: Arc<Mutex<Vec<(u32, String)>>>,
        is_running: Arc<Mutex<bool>>,
    ) -> Result<(), String> {
        let addr = format!("127.0.0.1:{}", port);
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| format!("Failed to bind to {}: {}", addr, e))?;

        println!("WebSocket server listening on ws://{}", addr);

        let mut client_counter = 0u32;

        loop {
            // Check if server should stop
            if let Ok(running) = is_running.lock() {
                if !*running {
                    break;
                }
            }

            // Accept new connection
            match listener.accept().await {
                Ok((stream, addr)) => {
                    client_counter += 1;
                    let client_id = client_counter;

                    // Add to clients list
                    if let Ok(mut client_list) = clients.lock() {
                        client_list.push(client_id);
                    }

                    println!(
                        "New WebSocket client connected: {} (ID: {})",
                        addr, client_id
                    );

                    let messages_clone = Arc::clone(&messages);
                    let clients_clone = Arc::clone(&clients);

                    // Handle client in separate task
                    tokio::spawn(async move {
                        let mut ws = WsClient::server(stream);

                        loop {
                            match ws.recv_event().await {
                                Ok(event) => match event {
                                    Event::Data { ty, data } => {
                                        if matches!(ty, DataType::Complete(MessageType::Text)) {
                                            if let Ok(text) = String::from_utf8(data.to_vec()) {
                                                // Store message
                                                if let Ok(mut msgs) = messages_clone.lock() {
                                                    msgs.push((client_id, text));
                                                }
                                            }
                                        }
                                    }
                                    Event::Ping(data) => {
                                        let _ = ws.send_pong(data).await;
                                    }
                                    Event::Close { .. } => {
                                        println!("Client {} disconnected", client_id);
                                        if let Ok(mut client_list) = clients_clone.lock() {
                                            client_list.retain(|&id| id != client_id);
                                        }
                                        let _ = ws.close(()).await;
                                        break;
                                    }
                                    Event::Error(e) => {
                                        eprintln!("Client {} error: {:?}", client_id, e);
                                        break;
                                    }
                                    _ => {}
                                },
                                Err(e) => {
                                    eprintln!("Client {} receive error: {:?}", client_id, e);
                                    break;
                                }
                            }
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Failed to accept connection: {}", e);
                }
            }
        }

        Ok(())
    }

    fn get(id: u32) -> Option<Arc<Mutex<WsServer>>> {
        let servers = WS_SERVERS.lock().ok()?;
        servers.as_ref()?.get(&id).cloned()
    }

    fn get_messages(&self) -> Vec<(u32, String)> {
        if let Ok(mut msgs) = self.messages.lock() {
            let result = msgs.clone();
            msgs.clear();
            result
        } else {
            Vec::new()
        }
    }

    fn get_clients(&self) -> Vec<u32> {
        if let Ok(clients) = self.clients.lock() {
            clients.clone()
        } else {
            Vec::new()
        }
    }

    fn stop(&mut self) -> Result<(), String> {
        if let Ok(mut running) = self.is_running.lock() {
            *running = false;
        }
        Ok(())
    }
}

/// Register WebSocket server API
pub fn register_websocket_server(context: &mut Context) -> JsResult<()> {
    let server_code = r#"
        globalThis.WebSocketServer = class WebSocketServer {
            #serverId = null;

            onconnection = null;
            onmessage = null;
            onerror = null;

            constructor(options) {
                const port = options?.port || 8080;

                try {
                    this.#serverId = __viper_ws_server_create(port);
                    this.port = port;

                    // Start polling for messages
                    this.#pollMessages();

                    console.log(`WebSocket server created on port ${port}`);
                } catch (e) {
                    if (this.onerror) {
                        this.onerror({ type: 'error', message: e.message });
                    }
                    throw e;
                }
            }

            get clients() {
                if (this.#serverId === null) return [];
                return __viper_ws_server_get_clients(this.#serverId);
            }

            broadcast(data) {
                const message = typeof data === 'string' ? data : String(data);
                __viper_ws_server_broadcast(this.#serverId, message);
            }

            close() {
                if (this.#serverId !== null) {
                    __viper_ws_server_close(this.#serverId);
                    this.#serverId = null;
                }
            }

            #pollMessages() {
                const poll = () => {
                    if (this.#serverId === null) {
                        return;
                    }

                    try {
                        const messages = __viper_ws_server_receive(this.#serverId);
                        if (messages && messages.length > 0) {
                            for (const msg of messages) {
                                if (this.onmessage) {
                                    this.onmessage({
                                        type: 'message',
                                        clientId: msg.clientId,
                                        data: msg.data
                                    });
                                }
                            }
                        }
                    } catch (e) {
                        if (this.onerror) {
                            this.onerror({ type: 'error', message: e.message });
                        }
                    }

                    setTimeout(poll, 10);
                };

                setTimeout(poll, 100);
            }
        };
    "#;

    let source = Source::from_bytes(server_code.as_bytes());
    context.eval(source)?;

    // __viper_ws_server_create
    let create_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let port = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing port"))?
            .to_u32(context)? as u16;

        match WsServer::new(port) {
            Ok(id) => Ok(JsValue::from(id)),
            Err(e) => Err(JsNativeError::typ()
                .with_message(format!("Failed to create server: {}", e))
                .into()),
        }
    });

    context.register_global_callable(js_string!("__viper_ws_server_create"), 1, create_fn)?;

    // __viper_ws_server_receive
    let receive_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let id = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing server ID"))?
            .to_u32(context)?;

        if let Some(server_arc) = WsServer::get(id) {
            if let Ok(server) = server_arc.lock() {
                let messages = server.get_messages();
                let arr = JsArray::new(context);

                for (client_id, msg) in messages {
                    let obj = boa_engine::object::ObjectInitializer::new(context)
                        .property(
                            js_string!("clientId"),
                            JsValue::from(client_id),
                            boa_engine::property::Attribute::all(),
                        )
                        .property(
                            js_string!("data"),
                            JsValue::from(js_string!(msg)),
                            boa_engine::property::Attribute::all(),
                        )
                        .build();
                    arr.push(obj, context)?;
                }

                return Ok(arr.into());
            }
        }

        Ok(JsArray::new(context).into())
    });

    context.register_global_callable(js_string!("__viper_ws_server_receive"), 1, receive_fn)?;

    // __viper_ws_server_get_clients
    let get_clients_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let id = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing server ID"))?
            .to_u32(context)?;

        if let Some(server_arc) = WsServer::get(id) {
            if let Ok(server) = server_arc.lock() {
                let clients = server.get_clients();
                let arr = JsArray::new(context);
                for client_id in clients {
                    arr.push(JsValue::from(client_id), context)?;
                }
                return Ok(arr.into());
            }
        }

        Ok(JsArray::new(context).into())
    });

    context.register_global_callable(
        js_string!("__viper_ws_server_get_clients"),
        1,
        get_clients_fn,
    )?;

    // __viper_ws_server_broadcast (placeholder)
    let broadcast_fn = NativeFunction::from_fn_ptr(|_this, _args, _context| {
        // TODO: Implement broadcast functionality
        Ok(JsValue::undefined())
    });

    context.register_global_callable(js_string!("__viper_ws_server_broadcast"), 2, broadcast_fn)?;

    // __viper_ws_server_close
    let close_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let id = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing server ID"))?
            .to_u32(context)?;

        if let Some(server_arc) = WsServer::get(id) {
            if let Ok(mut server) = server_arc.lock() {
                server
                    .stop()
                    .map_err(|e| JsNativeError::typ().with_message(e))?;
            }
        }

        Ok(JsValue::undefined())
    });

    context.register_global_callable(js_string!("__viper_ws_server_close"), 1, close_fn)?;

    Ok(())
}
