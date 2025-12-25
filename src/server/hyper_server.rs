//! Ultra-fast single-threaded HTTP server using Hyper directly
//!
//! This implementation runs everything on a single thread with no synchronization
//! overhead. The JS fetch handler is called directly for each request.
//!
//! Architecture:
//! - Single-threaded Tokio runtime (current_thread)
//! - Hyper for raw HTTP performance
//! - WebSocket upgrade support using web-socket crate
//! - Direct JS callback invocation (no channels, no locks)
//! - Zero-copy where possible

use base64::Engine;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode, body::Incoming};
use hyper_util::rt::TokioIo;
use sha1::{Digest, Sha1};
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use web_socket::{DataType, Event, MessageType, WebSocket};

/// Server configuration
#[derive(Debug, Clone)]
pub struct HyperServerConfig {
    pub hostname: String,
    pub port: u16,
    pub max_body_size: usize,
}

impl Default for HyperServerConfig {
    fn default() -> Self {
        Self {
            hostname: "127.0.0.1".to_string(),
            port: 3000,
            max_body_size: 10 * 1024 * 1024,
        }
    }
}

/// Request data passed to the JS handler
#[derive(Debug, Clone)]
pub struct JsRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Bytes>,
    pub is_websocket_upgrade: bool,
}

/// Response data from the JS handler
#[derive(Debug, Clone)]
pub struct JsResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Bytes,
    pub upgrade_websocket: bool,
}

impl Default for JsResponse {
    fn default() -> Self {
        Self {
            status: 200,
            headers: HashMap::new(),
            body: Bytes::new(),
            upgrade_websocket: false,
        }
    }
}

impl JsResponse {
    pub fn text(status: u16, text: impl Into<String>) -> Self {
        let body = text.into();
        let mut headers = HashMap::new();
        headers.insert(
            "content-type".to_string(),
            "text/plain; charset=utf-8".to_string(),
        );
        Self {
            status,
            headers,
            body: Bytes::from(body),
            upgrade_websocket: false,
        }
    }

    pub fn html(status: u16, html: impl Into<String>) -> Self {
        let body = html.into();
        let mut headers = HashMap::new();
        headers.insert(
            "content-type".to_string(),
            "text/html; charset=utf-8".to_string(),
        );
        Self {
            status,
            headers,
            body: Bytes::from(body),
            upgrade_websocket: false,
        }
    }

    pub fn json(status: u16, json: impl Into<String>) -> Self {
        let body = json.into();
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        Self {
            status,
            headers,
            body: Bytes::from(body),
            upgrade_websocket: false,
        }
    }

    pub fn not_found() -> Self {
        Self::text(404, "Not Found")
    }

    pub fn internal_error(msg: impl Into<String>) -> Self {
        Self::text(500, msg)
    }

    pub fn websocket_upgrade() -> Self {
        Self {
            status: 101,
            headers: HashMap::new(),
            body: Bytes::new(),
            upgrade_websocket: true,
        }
    }
}

/// WebSocket message for JS callback
#[derive(Debug, Clone)]
pub struct WsMessage {
    pub client_id: u32,
    pub data: String,
}

/// WebSocket handler callbacks
pub struct WsHandlers {
    pub on_open: Option<Box<dyn Fn(u32) + Send + Sync>>,
    pub on_message: Option<Box<dyn Fn(u32, String) + Send + Sync>>,
    pub on_close: Option<Box<dyn Fn(u32) + Send + Sync>>,
}

/// Handler function type - called for each request
pub type RequestHandler = Rc<RefCell<dyn FnMut(JsRequest) -> JsResponse>>;

/// WebSocket event handler type
pub type WsEventHandler = Arc<Mutex<dyn FnMut(WsMessage) + Send>>;

/// Global WebSocket state
static WS_MESSAGES: Mutex<Option<Vec<WsMessage>>> = Mutex::new(None);
static WS_CLIENT_COUNTER: Mutex<u32> = Mutex::new(0);

/// Run the HTTP server on the current thread (blocking)
/// This uses a single-threaded Tokio runtime for maximum performance
pub fn run_server(
    config: HyperServerConfig,
    handler: RequestHandler,
) -> Result<(), Box<dyn std::error::Error>> {
    run_server_with_websocket(config, handler, None)
}

/// Run the HTTP server with WebSocket support
pub fn run_server_with_websocket(
    config: HyperServerConfig,
    handler: RequestHandler,
    ws_handler: Option<WsEventHandler>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize WebSocket message queue
    {
        let mut msgs = WS_MESSAGES.lock().unwrap();
        *msgs = Some(Vec::new());
    }

    // Create single-threaded runtime - no thread sync overhead
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async move {
        let addr: SocketAddr = format!("{}:{}", config.hostname, config.port).parse()?;
        let listener = TcpListener::bind(addr).await?;

        println!("🚀 Viper server listening on http://{}", addr);
        println!("📡 WebSocket available on ws://{}", addr);

        loop {
            let (stream, remote_addr) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let handler = handler.clone();
            let ws_handler = ws_handler.clone();
            let max_body_size = config.max_body_size;

            // Process connection
            let service = service_fn(move |req: Request<Incoming>| {
                let handler = handler.clone();
                let ws_handler = ws_handler.clone();
                let remote = remote_addr;
                async move { handle_request(req, handler, ws_handler, max_body_size, remote).await }
            });

            // Serve the connection with upgrades enabled
            let conn = http1::Builder::new()
                .serve_connection(io, service)
                .with_upgrades();

            if let Err(err) = conn.await {
                eprintln!("Error serving connection: {:?}", err);
            }
        }

        #[allow(unreachable_code)]
        Ok::<(), Box<dyn std::error::Error>>(())
    })
}

/// Generate WebSocket accept key from client key
fn generate_websocket_accept(key: &str) -> String {
    const WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let mut hasher = Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update(WS_GUID.as_bytes());
    let result = hasher.finalize();
    base64::engine::general_purpose::STANDARD.encode(result)
}

/// Check if request is a WebSocket upgrade
fn is_websocket_upgrade(req: &Request<Incoming>) -> bool {
    let dominated = req
        .headers()
        .get("upgrade")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("websocket"))
        .unwrap_or(false);

    let has_connection_upgrade = req
        .headers()
        .get("connection")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_lowercase().contains("upgrade"))
        .unwrap_or(false);

    dominated && has_connection_upgrade
}

/// Handle a single HTTP request
async fn handle_request(
    req: Request<Incoming>,
    handler: RequestHandler,
    ws_handler: Option<WsEventHandler>,
    max_body_size: usize,
    remote_addr: SocketAddr,
) -> Result<Response<Full<Bytes>>, Infallible> {
    // Check for WebSocket upgrade
    if is_websocket_upgrade(&req) {
        return handle_websocket_upgrade(req, handler, ws_handler, remote_addr).await;
    }

    // Regular HTTP request
    let method = req.method().to_string();
    let uri = req.uri().to_string();

    let headers: HashMap<String, String> = req
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    // Read body
    let body = match req.collect().await {
        Ok(collected) => {
            let bytes = collected.to_bytes();
            if bytes.len() > max_body_size {
                let response = JsResponse::text(413, "Request body too large");
                return Ok(build_response(response));
            }
            if bytes.is_empty() { None } else { Some(bytes) }
        }
        Err(_) => None,
    };

    let js_request = JsRequest {
        method,
        url: uri,
        headers,
        body,
        is_websocket_upgrade: false,
    };

    // Call the handler directly
    let js_response = handler.borrow_mut()(js_request);

    Ok(build_response(js_response))
}

/// Handle WebSocket upgrade request
async fn handle_websocket_upgrade(
    req: Request<Incoming>,
    handler: RequestHandler,
    ws_handler: Option<WsEventHandler>,
    remote_addr: SocketAddr,
) -> Result<Response<Full<Bytes>>, Infallible> {
    // Get the WebSocket key
    let ws_key = match req.headers().get("sec-websocket-key") {
        Some(key) => key.to_str().unwrap_or("").to_string(),
        None => {
            return Ok(build_response(JsResponse::text(
                400,
                "Missing WebSocket key",
            )));
        }
    };

    // Check if the JS handler accepts this WebSocket connection
    let uri = req.uri().to_string();
    let headers: HashMap<String, String> = req
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let js_request = JsRequest {
        method: "GET".to_string(),
        url: uri.clone(),
        headers,
        body: None,
        is_websocket_upgrade: true,
    };

    let js_response = handler.borrow_mut()(js_request);

    // If handler doesn't want to upgrade, return that response
    if !js_response.upgrade_websocket && js_response.status != 101 {
        return Ok(build_response(js_response));
    }

    // Generate accept key
    let accept_key = generate_websocket_accept(&ws_key);

    // Build the 101 Switching Protocols response
    let response = Response::builder()
        .status(StatusCode::SWITCHING_PROTOCOLS)
        .header("upgrade", "websocket")
        .header("connection", "Upgrade")
        .header("sec-websocket-accept", accept_key)
        .body(Full::new(Bytes::new()))
        .unwrap();

    // Spawn WebSocket handler task
    let client_id = {
        let mut counter = WS_CLIENT_COUNTER.lock().unwrap();
        *counter += 1;
        *counter
    };

    println!(
        "🔌 WebSocket client connected: {} (ID: {}) on {}",
        remote_addr, client_id, uri
    );

    // The actual WebSocket communication happens after the upgrade
    // This is handled by hyper's upgrade mechanism
    tokio::spawn(async move {
        // Get the upgraded connection
        match hyper::upgrade::on(req).await {
            Ok(upgraded) => {
                let io = TokioIo::new(upgraded);
                let mut ws = WebSocket::server(io);

                // Notify open
                if let Some(ref handler) = ws_handler {
                    if let Ok(mut h) = handler.lock() {
                        h(WsMessage {
                            client_id,
                            data: "__open__".to_string(),
                        });
                    }
                }

                // Message loop
                loop {
                    match ws.recv_event().await {
                        Ok(event) => match event {
                            Event::Data { ty, data } => {
                                if matches!(ty, DataType::Complete(MessageType::Text)) {
                                    if let Ok(text) = String::from_utf8(data.to_vec()) {
                                        // Store message for JS polling
                                        if let Ok(mut msgs) = WS_MESSAGES.lock() {
                                            if let Some(ref mut queue) = *msgs {
                                                queue.push(WsMessage {
                                                    client_id,
                                                    data: text.clone(),
                                                });
                                            }
                                        }

                                        // Call handler
                                        if let Some(ref handler) = ws_handler {
                                            if let Ok(mut h) = handler.lock() {
                                                h(WsMessage {
                                                    client_id,
                                                    data: text,
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                            Event::Ping(data) => {
                                let _ = ws.send_pong(data).await;
                            }
                            Event::Close { .. } => {
                                println!("👋 WebSocket client disconnected: {}", client_id);
                                if let Some(ref handler) = ws_handler {
                                    if let Ok(mut h) = handler.lock() {
                                        h(WsMessage {
                                            client_id,
                                            data: "__close__".to_string(),
                                        });
                                    }
                                }
                                let _ = ws.close(()).await;
                                break;
                            }
                            Event::Error(e) => {
                                eprintln!("WebSocket error for client {}: {:?}", client_id, e);
                                break;
                            }
                            _ => {}
                        },
                        Err(e) => {
                            eprintln!("WebSocket receive error: {:?}", e);
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("WebSocket upgrade failed: {:?}", e);
            }
        }
    });

    Ok(response)
}

/// Build a hyper Response from JsResponse
fn build_response(js_response: JsResponse) -> Response<Full<Bytes>> {
    let mut builder = Response::builder()
        .status(StatusCode::from_u16(js_response.status).unwrap_or(StatusCode::OK));

    for (key, value) in js_response.headers {
        builder = builder.header(key, value);
    }

    builder
        .body(Full::new(js_response.body))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from("Internal Server Error")))
                .unwrap()
        })
}

/// Get pending WebSocket messages (for JS polling)
pub fn get_ws_messages() -> Vec<WsMessage> {
    if let Ok(mut msgs) = WS_MESSAGES.lock() {
        if let Some(ref mut queue) = *msgs {
            let result = queue.clone();
            queue.clear();
            return result;
        }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_js_response_text() {
        let resp = JsResponse::text(200, "Hello");
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body, Bytes::from("Hello"));
    }

    #[test]
    fn test_js_response_json() {
        let resp = JsResponse::json(200, r#"{"ok":true}"#);
        assert_eq!(resp.status, 200);
        assert_eq!(
            resp.headers.get("content-type").unwrap(),
            "application/json"
        );
    }

    #[test]
    fn test_websocket_accept_key() {
        // Test with known key from RFC 6455
        let key = "dGhlIHNhbXBsZSBub25jZQ==";
        let accept = generate_websocket_accept(key);
        assert_eq!(accept, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }
}
