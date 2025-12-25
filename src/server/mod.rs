//! High-Performance HTTP Server for Viper Runtime
//!
//! This module provides a fast, async HTTP server similar to Bun.serve() and Deno.serve()
//! Built directly on Hyper for maximum performance.
//!
//! Architecture:
//! - Uses a channel-based design for JS callback integration
//! - Server runs in a separate thread with its own Tokio runtime
//! - Requests are serialized and sent to the JS thread via channels
//! - JS handler processes requests and sends responses back
//!
//! Features:
//! - Ultra-fast HTTP/1.1 with keep-alive
//! - Zero-copy request/response where possible
//! - Streaming support for large bodies
//! - Built-in static file serving
//! - Router with fast path matching

pub mod hyper_server;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

pub use axum::{
    body::Body,
    extract::Request,
    response::{IntoResponse, Response},
    Router,
};

/// Errors that can occur with the HTTP server
#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Server failed to start: {0}")]
    StartError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    #[error("Handler error: {0}")]
    HandlerError(String),

    #[error("Channel closed")]
    ChannelClosed,
}

/// Result type for server operations
pub type ServerResult<T> = Result<T, ServerError>;

/// HTTP server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Hostname to bind to (default: "127.0.0.1")
    pub hostname: String,

    /// Port to bind to (default: 3000)
    pub port: u16,

    /// Enable development mode with auto-reload
    pub development: bool,

    /// Static file serving directory
    pub static_dir: Option<String>,

    /// Request timeout in milliseconds
    pub request_timeout_ms: u64,

    /// Maximum request body size in bytes (default: 10MB)
    pub max_body_size: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            hostname: "127.0.0.1".to_string(),
            port: 3000,
            development: false,
            static_dir: None,
            request_timeout_ms: 30000,
            max_body_size: 10 * 1024 * 1024, // 10MB
        }
    }
}

/// Information about an incoming HTTP request
/// This is sent to the JS handler for processing
#[derive(Debug, Clone)]
pub struct RequestInfo {
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// Full URL path with query string
    pub url: String,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Request body (if any)
    pub body: Option<Vec<u8>>,
    /// Client IP address
    pub remote_addr: Option<String>,
}

impl RequestInfo {
    /// Get a header value by name (case-insensitive)
    pub fn header(&self, name: &str) -> Option<&str> {
        let name_lower = name.to_lowercase();
        self.headers.iter()
            .find(|(k, _)| k.to_lowercase() == name_lower)
            .map(|(_, v)| v.as_str())
    }

    /// Get the request body as a string
    pub fn text(&self) -> Option<String> {
        self.body.as_ref().map(|b| String::from_utf8_lossy(b).to_string())
    }

    /// Parse the request body as JSON
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Option<T> {
        self.body.as_ref()
            .and_then(|b| serde_json::from_slice(b).ok())
    }
}

/// Builder for HTTP responses
#[derive(Debug, Clone)]
pub struct ResponseBuilder {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl ResponseBuilder {
    pub fn new() -> Self {
        Self {
            status: 200,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    pub fn status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }

    pub fn json(mut self, json: impl AsRef<str>) -> Self {
        self.headers.insert("content-type".to_string(), "application/json".to_string());
        self.body = json.as_ref().as_bytes().to_vec();
        self
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.headers.insert("content-type".to_string(), "text/plain; charset=utf-8".to_string());
        self.body = text.into().into_bytes();
        self
    }

    pub fn html(mut self, html: impl Into<String>) -> Self {
        self.headers.insert("content-type".to_string(), "text/html; charset=utf-8".to_string());
        self.body = html.into().into_bytes();
        self
    }
}

impl Default for ResponseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoResponse for ResponseBuilder {
    fn into_response(self) -> Response {
        let mut response = Response::builder().status(self.status);

        for (name, value) in self.headers {
            response = response.header(name, value);
        }

        response
            .body(Body::from(self.body))
            .unwrap_or_else(|_| {
                Response::builder()
                    .status(500)
                    .body(Body::from("Internal Server Error"))
                    .unwrap()
            })
    }
}

/// A pending request waiting for a response from JS
pub struct PendingRequest {
    pub request: RequestInfo,
    pub response_tx: oneshot::Sender<ResponseBuilder>,
}

/// Handle for sending requests to the JS handler
#[derive(Clone)]
pub struct RequestSender {
    tx: mpsc::Sender<PendingRequest>,
}

impl RequestSender {
    /// Send a request and wait for a response
    pub async fn send(&self, request: RequestInfo) -> ServerResult<ResponseBuilder> {
        let (response_tx, response_rx) = oneshot::channel();

        self.tx.send(PendingRequest { request, response_tx })
            .await
            .map_err(|_| ServerError::ChannelClosed)?;

        response_rx.await.map_err(|_| ServerError::ChannelClosed)
    }
}

/// Receiver for getting requests in the JS thread
pub struct RequestReceiver {
    rx: mpsc::Receiver<PendingRequest>,
}

impl RequestReceiver {
    /// Try to receive a pending request (non-blocking)
    pub fn try_recv(&mut self) -> Option<PendingRequest> {
        self.rx.try_recv().ok()
    }

    /// Check if there are any pending requests
    pub fn is_empty(&self) -> bool {
        self.rx.is_empty()
    }
}

/// Create a request channel pair
pub fn request_channel(buffer_size: usize) -> (RequestSender, RequestReceiver) {
    let (tx, rx) = mpsc::channel(buffer_size);
    (RequestSender { tx }, RequestReceiver { rx })
}

/// Static handler type for simple use cases
pub type StaticHandler = Arc<dyn Fn(RequestInfo) -> ResponseBuilder + Send + Sync>;

/// HTTP Server with channel-based JS integration
pub struct Server {
    config: ServerConfig,
    handler: Option<StaticHandler>,
    request_sender: Option<RequestSender>,
}

impl Server {
    /// Create a server with a static Rust handler (no JS integration)
    pub fn with_static_handler(config: ServerConfig, handler: StaticHandler) -> Self {
        Self {
            config,
            handler: Some(handler),
            request_sender: None,
        }
    }

    /// Create a server with a channel-based handler for JS integration
    pub fn with_channel(config: ServerConfig, sender: RequestSender) -> Self {
        Self {
            config,
            handler: None,
            request_sender: Some(sender),
        }
    }

    /// Start the server (blocking)
    pub fn start(self) -> ServerResult<()> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(num_cpus::get().max(2))
            .enable_all()
            .build()
            .map_err(|e| ServerError::StartError(format!("Failed to create tokio runtime: {}", e)))?;

        rt.block_on(self.run_async())
    }

    /// Start the server asynchronously
    pub async fn run_async(self) -> ServerResult<()> {
        use axum::routing::any;

        let addr = format!("{}:{}", self.config.hostname, self.config.port)
            .parse::<SocketAddr>()
            .map_err(|e| ServerError::InvalidAddress(e.to_string()))?;

        let max_body_size = self.config.max_body_size;
        let timeout = Duration::from_millis(self.config.request_timeout_ms);

        let app = if let Some(handler) = self.handler {
            // Static handler mode
            let handler_clone = handler.clone();
            Router::new()
                .route("/", any(move |req| handle_static_request(req, handler_clone.clone(), max_body_size)))
                .route("/*path", any(move |req| handle_static_request(req, handler.clone(), max_body_size)))
        } else if let Some(sender) = self.request_sender {
            // Channel mode for JS integration
            let sender_clone = sender.clone();
            Router::new()
                .route("/", any(move |req| handle_channel_request(req, sender_clone.clone(), max_body_size, timeout)))
                .route("/*path", any(move |req| handle_channel_request(req, sender.clone(), max_body_size, timeout)))
        } else {
            return Err(ServerError::StartError("No handler configured".to_string()));
        };

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| ServerError::StartError(format!("Failed to bind to {}: {}", addr, e)))?;

        println!("🚀 Viper server listening on http://{}", addr);

        axum::serve(listener, app)
            .await
            .map_err(|e| ServerError::StartError(e.to_string()))
    }
}

/// Handle a request with a static handler
async fn handle_static_request(
    req: Request,
    handler: StaticHandler,
    max_body_size: usize,
) -> impl IntoResponse {
    let request_info = extract_request_info(req, max_body_size).await;
    handler(request_info)
}

/// Handle a request through the channel (for JS integration)
async fn handle_channel_request(
    req: Request,
    sender: RequestSender,
    max_body_size: usize,
    timeout: Duration,
) -> impl IntoResponse {
    let request_info = extract_request_info(req, max_body_size).await;

    match tokio::time::timeout(timeout, sender.send(request_info)).await {
        Ok(Ok(response)) => response,
        Ok(Err(_)) => ResponseBuilder::new()
            .status(503)
            .text("Service Unavailable: Handler not responding"),
        Err(_) => ResponseBuilder::new()
            .status(504)
            .text("Gateway Timeout: Request processing took too long"),
    }
}

/// Extract request information from an Axum request
async fn extract_request_info(req: Request, max_body_size: usize) -> RequestInfo {
    let method = req.method().to_string();
    let uri = req.uri().to_string();

    let headers: HashMap<String, String> = req
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let body_bytes = axum::body::to_bytes(req.into_body(), max_body_size)
        .await
        .ok()
        .map(|b| b.to_vec())
        .filter(|b| !b.is_empty());

    RequestInfo {
        method,
        url: uri,
        headers,
        body: body_bytes,
        remote_addr: None,
    }
}

/// Simple pattern-based router for fast path matching
#[derive(Default)]
pub struct PathRouter {
    routes: Vec<(String, bool)>, // (pattern, is_prefix)
}

impl PathRouter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an exact path match
    pub fn exact(&mut self, path: &str) -> &mut Self {
        self.routes.push((path.to_string(), false));
        self
    }

    /// Add a prefix match (e.g., "/api/*")
    pub fn prefix(&mut self, path: &str) -> &mut Self {
        let path = path.trim_end_matches('*').to_string();
        self.routes.push((path, true));
        self
    }

    /// Match a path and return the matched route index
    pub fn match_path(&self, path: &str) -> Option<usize> {
        for (i, (pattern, is_prefix)) in self.routes.iter().enumerate() {
            if *is_prefix {
                if path.starts_with(pattern) {
                    return Some(i);
                }
            } else if path == pattern {
                return Some(i);
            }
        }
        None
    }
}

/// Create a simple example server for testing
pub fn example_server() -> Server {
    let handler: StaticHandler = Arc::new(|req: RequestInfo| {
        match req.url.as_str() {
            "/" => ResponseBuilder::new().html(format!(
                r#"<!DOCTYPE html>
<html>
<head><title>Viper Server</title></head>
<body>
    <h1>🐍 Viper HTTP Server</h1>
    <p>Request: {} {}</p>
    <p>Ultra-fast, powered by Hyper + Axum</p>
</body>
</html>"#,
                req.method, req.url
            )),
            "/json" => ResponseBuilder::new()
                .json(r#"{"message":"Hello from Viper!","fast":true}"#),
            "/health" => ResponseBuilder::new()
                .json(r#"{"status":"ok"}"#),
            _ => ResponseBuilder::new()
                .status(404)
                .html("<h1>404 Not Found</h1>"),
        }
    });

    Server::with_static_handler(ServerConfig::default(), handler)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_builder() {
        let response = ResponseBuilder::new()
            .status(200)
            .text("Hello, World!");

        assert_eq!(response.status, 200);
        assert_eq!(response.body, b"Hello, World!");
        assert_eq!(response.headers.get("content-type").unwrap(), "text/plain; charset=utf-8");
    }

    #[test]
    fn test_json_response() {
        let response = ResponseBuilder::new()
            .json(r#"{"test": true}"#);

        assert_eq!(response.status, 200);
        assert_eq!(response.headers.get("content-type").unwrap(), "application/json");
    }

    #[test]
    fn test_server_config() {
        let config = ServerConfig::default();
        assert_eq!(config.hostname, "127.0.0.1");
        assert_eq!(config.port, 3000);
        assert_eq!(config.max_body_size, 10 * 1024 * 1024);
    }

    #[test]
    fn test_path_router() {
        let mut router = PathRouter::new();
        router.exact("/");
        router.exact("/about");
        router.prefix("/api/");

        assert_eq!(router.match_path("/"), Some(0));
        assert_eq!(router.match_path("/about"), Some(1));
        assert_eq!(router.match_path("/api/users"), Some(2));
        assert_eq!(router.match_path("/api/posts/1"), Some(2));
        assert_eq!(router.match_path("/other"), None);
    }

    #[test]
    fn test_request_info() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let req = RequestInfo {
            method: "POST".to_string(),
            url: "/api/test".to_string(),
            headers,
            body: Some(b"test body".to_vec()),
            remote_addr: None,
        };

        assert_eq!(req.header("content-type"), Some("application/json"));
        assert_eq!(req.text(), Some("test body".to_string()));
    }
}
