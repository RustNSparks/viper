//! Ultra-fast single-threaded HTTP server using Hyper directly
//!
//! This implementation runs everything on a single thread with no synchronization
//! overhead. The JS fetch handler is called directly for each request.
//!
//! Architecture:
//! - Single-threaded Tokio runtime (current_thread)
//! - Hyper for raw HTTP performance
//! - Direct JS callback invocation (no channels, no locks)
//! - Zero-copy where possible

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode, body::Incoming};
use hyper_util::rt::TokioIo;
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::rc::Rc;
use tokio::net::TcpListener;

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
}

/// Response data from the JS handler
#[derive(Debug, Clone)]
pub struct JsResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Bytes,
}

impl Default for JsResponse {
    fn default() -> Self {
        Self {
            status: 200,
            headers: HashMap::new(),
            body: Bytes::new(),
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
        }
    }

    pub fn not_found() -> Self {
        Self::text(404, "Not Found")
    }

    pub fn internal_error(msg: impl Into<String>) -> Self {
        Self::text(500, msg)
    }
}

/// Handler function type - called for each request
pub type RequestHandler = Rc<RefCell<dyn FnMut(JsRequest) -> JsResponse>>;

/// Run the HTTP server on the current thread (blocking)
/// This uses a single-threaded Tokio runtime for maximum performance
pub fn run_server(
    config: HyperServerConfig,
    handler: RequestHandler,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create single-threaded runtime - no thread sync overhead
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async move {
        let local_set = tokio::task::LocalSet::new();

        local_set
            .run_until(async move {
                let addr: SocketAddr = format!("{}:{}", config.hostname, config.port).parse()?;
                let listener = TcpListener::bind(addr).await?;

                println!("🚀 Viper server listening on http://{}", addr);

                loop {
                    let (stream, _remote_addr) = listener.accept().await?;
                    let io = TokioIo::new(stream);
                    let handler = handler.clone();
                    let max_body_size = config.max_body_size;

                    // Spawn each connection as a local task so we can handle multiple connections
                    tokio::task::spawn_local(async move {
                        // Process connection
                        let service = service_fn(move |req: Request<Incoming>| {
                            let handler = handler.clone();
                            async move { handle_request(req, handler, max_body_size).await }
                        });

                        // Serve the connection
                        let conn = http1::Builder::new().serve_connection(io, service);

                        if let Err(err) = conn.await {
                            eprintln!("Error serving connection: {:?}", err);
                        }
                    });
                }

                #[allow(unreachable_code)]
                Ok::<(), Box<dyn std::error::Error>>(())
            })
            .await
    })
}

/// Handle a single HTTP request
async fn handle_request(
    req: Request<Incoming>,
    handler: RequestHandler,
    max_body_size: usize,
) -> Result<Response<Full<Bytes>>, Infallible> {
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
    };

    // Call the handler directly
    let js_response = handler.borrow_mut()(js_request);

    Ok(build_response(js_response))
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
}
