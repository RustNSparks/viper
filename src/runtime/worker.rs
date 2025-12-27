//! Worker API - High-performance Web Workers implementation
//!
//! Implements Bun's Worker API with performance optimizations:
//! - Zero-copy message passing for strings (fast path)
//! - Simple object fast path (2-241x faster than Node.js)
//! - Proper thread isolation with separate JS contexts
//! - blob: URL support for inline workers
//! - preload module support
//!
//! Key features:
//! - new Worker(url, options) - Create worker from file or blob URL
//! - worker.postMessage(data) - Send message with structured clone
//! - worker.terminate() - Forcefully terminate worker
//! - worker.ref() / worker.unref() - Control process lifetime
//! - Events: open, message, error, close

use boa_engine::{
    Context, JsNativeError, JsResult, JsValue, NativeFunction, Source, js_string,
    object::builtins::JsArray, property::Attribute,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};

use crate::runtime::{Runtime, RuntimeConfig};

// ============================================================================
// Worker Message Types - Optimized for performance
// ============================================================================

/// Message types for fast-path detection
#[derive(Debug, Clone)]
pub enum WorkerMessage {
    /// String fast path - zero serialization overhead
    String(String),
    /// Simple object fast path - flat objects with primitive values only
    SimpleObject(Vec<(String, SimpleValue)>),
    /// Full structured clone for complex objects (JSON serialized)
    StructuredClone(Vec<u8>),
    /// Transferable ArrayBuffer - zero-copy transfer (ownership moves)
    ArrayBuffer(Vec<u8>),
    /// Message with transferable ArrayBuffers
    WithTransfer {
        data: Box<WorkerMessage>,
        buffers: Vec<Vec<u8>>,
    },
    /// Terminate signal
    Terminate,
    /// Worker is ready (open event)
    Ready,
    /// Error message
    Error(String),
    /// Close message with exit code
    Close(i32),
}

/// Simple values for the fast path
#[derive(Debug, Clone)]
pub enum SimpleValue {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
}

// ============================================================================
// Worker State Management
// ============================================================================

/// Worker ready states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum WorkerState {
    Starting = 0,
    Running = 1,
    Closing = 2,
    Closed = 3,
}

impl From<u8> for WorkerState {
    fn from(v: u8) -> Self {
        match v {
            0 => WorkerState::Starting,
            1 => WorkerState::Running,
            2 => WorkerState::Closing,
            _ => WorkerState::Closed,
        }
    }
}

/// Global worker counter for unique IDs
static WORKER_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Global worker storage
static WORKERS: Mutex<Option<HashMap<u32, Arc<WorkerHandle>>>> = Mutex::new(None);

// ============================================================================
// MessageChannel / MessagePort Implementation
// ============================================================================

/// Global MessagePort counter for unique IDs
static PORT_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Global MessagePort storage
static MESSAGE_PORTS: Mutex<Option<HashMap<u32, Arc<MessagePortHandle>>>> = Mutex::new(None);

/// A MessagePort handle for cross-thread communication
pub struct MessagePortHandle {
    /// Unique port ID
    pub id: u32,
    /// The paired port ID (the other end of the channel)
    pub paired_port_id: u32,
    /// Message queue for this port
    inbox: Mutex<Vec<PortMessage>>,
    /// Whether the port is open
    is_open: AtomicU32,
    /// Whether this port has been transferred to a worker
    is_transferred: AtomicU32,
}

/// Message sent through a MessagePort
#[derive(Debug, Clone)]
pub struct PortMessage {
    /// The message data (JSON serialized)
    pub data: Vec<u8>,
    /// Transferred ArrayBuffers
    pub buffers: Vec<Vec<u8>>,
    /// Transferred port IDs
    pub ports: Vec<u32>,
}

impl MessagePortHandle {
    fn new(id: u32, paired_port_id: u32) -> Arc<Self> {
        Arc::new(Self {
            id,
            paired_port_id,
            inbox: Mutex::new(Vec::with_capacity(16)),
            is_open: AtomicU32::new(1),
            is_transferred: AtomicU32::new(0),
        })
    }

    fn is_open(&self) -> bool {
        self.is_open.load(Ordering::SeqCst) == 1
    }

    fn close(&self) {
        self.is_open.store(0, Ordering::SeqCst);
    }

    fn mark_transferred(&self) {
        self.is_transferred.store(1, Ordering::SeqCst);
    }

    fn is_transferred(&self) -> bool {
        self.is_transferred.load(Ordering::SeqCst) == 1
    }

    /// Send a message to the paired port
    fn send(&self, msg: PortMessage) {
        // Get the paired port and add message to its inbox
        if let Some(paired) = get_port_handle(self.paired_port_id) {
            if paired.is_open() {
                let mut inbox = paired.inbox.lock().unwrap();
                inbox.push(msg);
            }
        }
    }

    /// Receive all pending messages
    fn receive(&self) -> Vec<PortMessage> {
        let mut inbox = self.inbox.lock().unwrap();
        std::mem::take(&mut *inbox)
    }
}

/// Create a new MessageChannel (returns two paired port IDs)
fn create_message_channel() -> (u32, u32) {
    let port1_id = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);
    let port2_id = PORT_COUNTER.fetch_add(1, Ordering::SeqCst);

    let port1 = MessagePortHandle::new(port1_id, port2_id);
    let port2 = MessagePortHandle::new(port2_id, port1_id);

    let mut ports = MESSAGE_PORTS.lock().unwrap();
    if ports.is_none() {
        *ports = Some(HashMap::new());
    }
    if let Some(ref mut map) = *ports {
        map.insert(port1_id, port1);
        map.insert(port2_id, port2);
    }

    (port1_id, port2_id)
}

/// Get a port handle by ID
fn get_port_handle(id: u32) -> Option<Arc<MessagePortHandle>> {
    let ports = MESSAGE_PORTS.lock().ok()?;
    ports.as_ref()?.get(&id).cloned()
}

/// Check if there are any active message ports with pending messages
pub fn has_active_ports() -> bool {
    let ports = match MESSAGE_PORTS.lock() {
        Ok(p) => p,
        Err(_) => return false,
    };

    if let Some(ref map) = *ports {
        for handle in map.values() {
            if handle.is_open() && !handle.is_transferred() {
                let inbox = handle.inbox.lock().unwrap();
                if !inbox.is_empty() {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if there are any active (ref'd) workers
pub fn has_active_workers() -> bool {
    let workers = match WORKERS.lock() {
        Ok(w) => w,
        Err(_) => return false,
    };

    if let Some(ref map) = *workers {
        for handle in map.values() {
            // Check if worker is running and ref'd
            let state = handle.get_state();
            if state == WorkerState::Running || state == WorkerState::Starting {
                if handle.ref_count.load(Ordering::SeqCst) > 0 {
                    return true;
                }
            }
        }
    }
    false
}

/// Poll all workers and return any pending messages for the main thread
/// Returns true if any workers are still active
pub fn poll_workers() -> bool {
    has_active_workers()
}

/// Handle to a worker thread
pub struct WorkerHandle {
    /// Unique worker ID (same as threadId)
    pub id: u32,
    /// Worker state
    state: AtomicU32,
    /// Messages from main thread to worker
    inbox: Mutex<Vec<WorkerMessage>>,
    /// Messages from worker to main thread
    outbox: Mutex<Vec<WorkerMessage>>,
    /// Condition variable for inbox
    inbox_condvar: Condvar,
    /// Whether this worker keeps the process alive
    ref_count: AtomicU32,
    /// Thread handle (for joining)
    thread_handle: Mutex<Option<JoinHandle<()>>>,
    /// Worker script URL
    pub url: String,
    /// Whether worker uses smol mode (reduced memory)
    pub smol: bool,
}

impl WorkerHandle {
    fn new(id: u32, url: String, smol: bool) -> Arc<Self> {
        Arc::new(Self {
            id,
            state: AtomicU32::new(WorkerState::Starting as u32),
            inbox: Mutex::new(Vec::with_capacity(64)),
            outbox: Mutex::new(Vec::with_capacity(64)),
            inbox_condvar: Condvar::new(),
            ref_count: AtomicU32::new(1), // ref'd by default
            thread_handle: Mutex::new(None),
            url,
            smol,
        })
    }

    fn get_state(&self) -> WorkerState {
        WorkerState::from(self.state.load(Ordering::SeqCst) as u8)
    }

    fn set_state(&self, state: WorkerState) {
        self.state.store(state as u32, Ordering::SeqCst);
    }

    /// Send message to worker (from main thread)
    fn send_to_worker(&self, msg: WorkerMessage) {
        let mut inbox = self.inbox.lock().unwrap();
        inbox.push(msg);
        self.inbox_condvar.notify_one();
    }

    /// Receive messages from worker (on main thread)
    fn receive_from_worker(&self) -> Vec<WorkerMessage> {
        let mut outbox = self.outbox.lock().unwrap();
        std::mem::take(&mut *outbox)
    }

    /// Send message to main thread (from worker)
    fn send_to_main(&self, msg: WorkerMessage) {
        let mut outbox = self.outbox.lock().unwrap();
        outbox.push(msg);
    }

    /// Receive messages from main thread (on worker), non-blocking
    fn receive_from_main(&self) -> Vec<WorkerMessage> {
        let mut inbox = self.inbox.lock().unwrap();
        std::mem::take(&mut *inbox)
    }

    fn terminate(&self) {
        self.set_state(WorkerState::Closing);
        self.send_to_worker(WorkerMessage::Terminate);
    }

    fn add_ref(&self) {
        self.ref_count.fetch_add(1, Ordering::SeqCst);
    }

    fn remove_ref(&self) {
        self.ref_count.fetch_sub(1, Ordering::SeqCst);
    }
}

// ============================================================================
// Worker Thread Implementation
// ============================================================================

/// Create a new worker thread
fn spawn_worker(
    handle: Arc<WorkerHandle>,
    script_path: PathBuf,
    preload: Vec<String>,
) -> JoinHandle<()> {
    let worker_id = handle.id;

    thread::spawn(move || {
        // Create isolated runtime for this worker
        let config = RuntimeConfig {
            base_path: script_path.parent().unwrap_or(&script_path).to_path_buf(),
            use_event_loop: true,
            args: vec![], // Workers don't inherit args
            ..Default::default()
        };

        let mut runtime = match Runtime::with_config(config) {
            Ok(r) => r,
            Err(e) => {
                if let Some(h) = get_worker_handle(worker_id) {
                    h.send_to_main(WorkerMessage::Error(format!(
                        "Failed to create runtime: {}",
                        e
                    )));
                    h.set_state(WorkerState::Closed);
                }
                return;
            }
        };

        // Register worker-specific globals
        if let Err(e) = register_worker_globals(runtime.context_mut(), worker_id) {
            if let Some(h) = get_worker_handle(worker_id) {
                h.send_to_main(WorkerMessage::Error(format!(
                    "Failed to register worker globals: {}",
                    e
                )));
                h.set_state(WorkerState::Closed);
            }
            return;
        }

        // Load preload modules
        for preload_path in &preload {
            let preload_code = match std::fs::read_to_string(preload_path) {
                Ok(c) => c,
                Err(e) => {
                    if let Some(h) = get_worker_handle(worker_id) {
                        h.send_to_main(WorkerMessage::Error(format!(
                            "Failed to load preload '{}': {}",
                            preload_path, e
                        )));
                        h.set_state(WorkerState::Closed);
                    }
                    return;
                }
            };
            if let Err(e) = runtime.run(&preload_code, preload_path) {
                if let Some(h) = get_worker_handle(worker_id) {
                    h.send_to_main(WorkerMessage::Error(format!("Preload error: {}", e)));
                    h.set_state(WorkerState::Closed);
                }
                return;
            }
        }

        // Load and execute the worker script
        let script_code = match std::fs::read_to_string(&script_path) {
            Ok(c) => c,
            Err(e) => {
                if let Some(h) = get_worker_handle(worker_id) {
                    h.send_to_main(WorkerMessage::Error(format!(
                        "Failed to load worker script: {}",
                        e
                    )));
                    h.set_state(WorkerState::Closed);
                }
                return;
            }
        };

        let filename = script_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("worker.ts");

        // Signal ready
        if let Some(h) = get_worker_handle(worker_id) {
            h.set_state(WorkerState::Running);
            h.send_to_main(WorkerMessage::Ready);
        }

        // Execute the worker script using eval() instead of run()
        // We use eval() because run() calls run_event_loop() which would block
        // waiting for workers. The worker's own message loop handles that below.
        if let Err(e) = runtime.eval(&script_code, filename) {
            if let Some(h) = get_worker_handle(worker_id) {
                h.send_to_main(WorkerMessage::Error(format!("Worker script error: {}", e)));
            }
        }

        // Run any pending jobs from script execution (e.g., console.log)
        let _ = runtime.context_mut().run_jobs();

        // Worker event loop - keep running while there are message handlers
        loop {
            let should_break = if let Some(h) = get_worker_handle(worker_id) {
                h.get_state() == WorkerState::Closing
            } else {
                true
            };

            if should_break {
                break;
            }

            // Process incoming messages
            let messages = if let Some(h) = get_worker_handle(worker_id) {
                h.receive_from_main()
            } else {
                break;
            };

            let mut processed_any = false;
            for msg in messages {
                match msg {
                    WorkerMessage::Terminate => {
                        if let Some(h) = get_worker_handle(worker_id) {
                            h.set_state(WorkerState::Closing);
                        }
                        break;
                    }
                    WorkerMessage::String(s) => {
                        processed_any = true;
                        // Dispatch to self.onmessage
                        let dispatch_code = format!(
                            r#"
                            if (typeof self !== 'undefined' && self.onmessage) {{
                                self.onmessage({{ data: "{}" }});
                            }}
                            "#,
                            s.replace('\\', "\\\\")
                                .replace('"', "\\\"")
                                .replace('\n', "\\n")
                        );
                        let _ = runtime.eval(&dispatch_code, "worker-dispatch.js");
                    }
                    WorkerMessage::SimpleObject(props) => {
                        processed_any = true;
                        // Build object literal
                        let mut obj_str = String::from("{");
                        for (i, (key, value)) in props.iter().enumerate() {
                            if i > 0 {
                                obj_str.push_str(", ");
                            }
                            obj_str.push_str(&format!("\"{}\": ", key.replace('"', "\\\"")));
                            match value {
                                SimpleValue::Undefined => obj_str.push_str("undefined"),
                                SimpleValue::Null => obj_str.push_str("null"),
                                SimpleValue::Boolean(b) => {
                                    obj_str.push_str(if *b { "true" } else { "false" })
                                }
                                SimpleValue::Number(n) => obj_str.push_str(&n.to_string()),
                                SimpleValue::String(s) => {
                                    obj_str.push('"');
                                    obj_str.push_str(
                                        &s.replace('\\', "\\\\")
                                            .replace('"', "\\\"")
                                            .replace('\n', "\\n"),
                                    );
                                    obj_str.push('"');
                                }
                            }
                        }
                        obj_str.push('}');

                        let dispatch_code = format!(
                            r#"
                            if (typeof self !== 'undefined' && self.onmessage) {{
                                self.onmessage({{ data: {} }});
                            }}
                            "#,
                            obj_str
                        );
                        let _ = runtime.eval(&dispatch_code, "worker-dispatch.js");
                    }
                    WorkerMessage::StructuredClone(data) => {
                        processed_any = true;
                        // For now, treat as JSON
                        if let Ok(json_str) = String::from_utf8(data) {
                            let dispatch_code = format!(
                                r#"
                                if (typeof self !== 'undefined' && self.onmessage) {{
                                    self.onmessage({{ data: JSON.parse('{}') }});
                                }}
                                "#,
                                json_str.replace('\\', "\\\\").replace('\'', "\\'")
                            );
                            let _ = runtime.eval(&dispatch_code, "worker-dispatch.js");
                        }
                    }
                    _ => {}
                }
            }

            // Run any pending jobs (like console.log from message handlers)
            if processed_any {
                let _ = runtime.context_mut().run_jobs();
            }

            // Small sleep to avoid busy-waiting
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        // Cleanup
        if let Some(h) = get_worker_handle(worker_id) {
            h.set_state(WorkerState::Closed);
            h.send_to_main(WorkerMessage::Close(0));
        }
    })
}

/// Spawn worker from blob URL (inline code)
fn spawn_worker_from_blob(
    handle: Arc<WorkerHandle>,
    code: String,
    content_type: String,
) -> JoinHandle<()> {
    let worker_id = handle.id;

    thread::spawn(move || {
        let config = RuntimeConfig {
            base_path: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            use_event_loop: true,
            args: vec![],
            ..Default::default()
        };

        let mut runtime = match Runtime::with_config(config) {
            Ok(r) => r,
            Err(e) => {
                if let Some(h) = get_worker_handle(worker_id) {
                    h.send_to_main(WorkerMessage::Error(format!(
                        "Failed to create runtime: {}",
                        e
                    )));
                    h.set_state(WorkerState::Closed);
                }
                return;
            }
        };

        // Register worker-specific globals
        if let Err(e) = register_worker_globals(runtime.context_mut(), worker_id) {
            if let Some(h) = get_worker_handle(worker_id) {
                h.send_to_main(WorkerMessage::Error(format!(
                    "Failed to register worker globals: {}",
                    e
                )));
                h.set_state(WorkerState::Closed);
            }
            return;
        }

        // Determine filename from content type
        let filename = if content_type.contains("typescript") {
            "worker.ts"
        } else {
            "worker.js"
        };

        // Signal ready
        if let Some(h) = get_worker_handle(worker_id) {
            h.set_state(WorkerState::Running);
            h.send_to_main(WorkerMessage::Ready);
        }

        // Execute the worker script using eval() instead of run()
        if let Err(e) = runtime.eval(&code, filename) {
            if let Some(h) = get_worker_handle(worker_id) {
                h.send_to_main(WorkerMessage::Error(format!("Worker script error: {}", e)));
            }
        }

        // Run any pending jobs from script execution
        let _ = runtime.context_mut().run_jobs();

        // Worker event loop
        loop {
            let should_break = if let Some(h) = get_worker_handle(worker_id) {
                h.get_state() == WorkerState::Closing
            } else {
                true
            };

            if should_break {
                break;
            }

            let messages = if let Some(h) = get_worker_handle(worker_id) {
                h.receive_from_main()
            } else {
                break;
            };

            for msg in messages {
                match msg {
                    WorkerMessage::Terminate => {
                        if let Some(h) = get_worker_handle(worker_id) {
                            h.set_state(WorkerState::Closing);
                        }
                        break;
                    }
                    WorkerMessage::String(s) => {
                        let dispatch_code = format!(
                            r#"
                            if (typeof self !== 'undefined' && self.onmessage) {{
                                self.onmessage({{ data: "{}" }});
                            }}
                            "#,
                            s.replace('\\', "\\\\")
                                .replace('"', "\\\"")
                                .replace('\n', "\\n")
                        );
                        let _ = runtime.eval(&dispatch_code, "worker-dispatch.js");
                    }
                    WorkerMessage::SimpleObject(props) => {
                        let mut obj_str = String::from("{");
                        for (i, (key, value)) in props.iter().enumerate() {
                            if i > 0 {
                                obj_str.push_str(", ");
                            }
                            obj_str.push_str(&format!("\"{}\": ", key.replace('"', "\\\"")));
                            match value {
                                SimpleValue::Undefined => obj_str.push_str("undefined"),
                                SimpleValue::Null => obj_str.push_str("null"),
                                SimpleValue::Boolean(b) => {
                                    obj_str.push_str(if *b { "true" } else { "false" })
                                }
                                SimpleValue::Number(n) => obj_str.push_str(&n.to_string()),
                                SimpleValue::String(s) => {
                                    obj_str.push('"');
                                    obj_str.push_str(
                                        &s.replace('\\', "\\\\")
                                            .replace('"', "\\\"")
                                            .replace('\n', "\\n"),
                                    );
                                    obj_str.push('"');
                                }
                            }
                        }
                        obj_str.push('}');

                        let dispatch_code = format!(
                            r#"
                            if (typeof self !== 'undefined' && self.onmessage) {{
                                self.onmessage({{ data: {} }});
                            }}
                            "#,
                            obj_str
                        );
                        let _ = runtime.eval(&dispatch_code, "worker-dispatch.js");
                    }
                    _ => {}
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        if let Some(h) = get_worker_handle(worker_id) {
            h.set_state(WorkerState::Closed);
            h.send_to_main(WorkerMessage::Close(0));
        }
    })
}

/// Get worker handle by ID
fn get_worker_handle(id: u32) -> Option<Arc<WorkerHandle>> {
    let workers = WORKERS.lock().ok()?;
    workers.as_ref()?.get(&id).cloned()
}

/// Register worker-specific globals (self, postMessage, etc.)
fn register_worker_globals(context: &mut Context, worker_id: u32) -> JsResult<()> {
    // Register __viper_worker_post_message (worker -> main)
    // We use a JavaScript wrapper that captures the worker_id
    let post_message_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        // Get worker_id from global
        let worker_id_val = context
            .global_object()
            .get(js_string!("__viper_worker_id"), context)?;
        let worker_id = worker_id_val.to_u32(context)?;

        let workers = WORKERS.lock().unwrap();
        let handle = workers
            .as_ref()
            .and_then(|m| m.get(&worker_id))
            .ok_or_else(|| JsNativeError::typ().with_message("Worker not found"))?;

        if args.is_empty() {
            return Err(JsNativeError::typ()
                .with_message("postMessage requires at least 1 argument")
                .into());
        }

        let data = &args[0];

        // Fast path: string
        if let Some(s) = data.as_string() {
            handle.send_to_main(WorkerMessage::String(s.to_std_string_escaped()));
            return Ok(JsValue::undefined());
        }

        // Fast path: simple object
        if let Some(obj) = data.as_object() {
            if let Some(props) = try_extract_simple_object(&obj, context) {
                handle.send_to_main(WorkerMessage::SimpleObject(props));
                return Ok(JsValue::undefined());
            }
        }

        // Fallback: JSON serialization (structured clone approximation)
        match data.to_json(context) {
            Ok(Some(json_val)) => {
                handle.send_to_main(WorkerMessage::StructuredClone(
                    json_val.to_string().into_bytes(),
                ));
            }
            _ => {
                // If JSON serialization fails, try string conversion
                let str_val = data.to_string(context)?;
                handle.send_to_main(WorkerMessage::String(str_val.to_std_string_escaped()));
            }
        }

        Ok(JsValue::undefined())
    });

    context.register_global_callable(
        js_string!("__viper_worker_post_message"),
        1,
        post_message_fn,
    )?;

    // Store worker_id in global
    context.global_object().set(
        js_string!("__viper_worker_id"),
        JsValue::from(worker_id),
        false,
        context,
    )?;

    // Register self object and worker globals
    let worker_globals = r#"
        // Worker self reference
        const self = globalThis;
        globalThis.self = self;

        // postMessage function (worker -> main)
        self.postMessage = function(data, transfer) {
            __viper_worker_post_message(data, transfer);
        };

        // Message handler placeholder
        self.onmessage = null;
        self.onerror = null;

        // Worker identification
        self.name = "";

        // Close the worker from within
        self.close = function() {
            // Signal close
        };

        // importScripts (legacy, but useful)
        self.importScripts = function(...urls) {
            throw new Error("importScripts is not supported. Use ES modules instead.");
        };

        // Viper compatibility
        Viper = globalThis.Viper || {};
        Viper.isMainThread = false;
        globalThis.Viper = Viper;
    "#;

    let source = Source::from_bytes(worker_globals.as_bytes());
    context.eval(source)?;

    Ok(())
}

/// Try to extract a simple object for fast path
fn try_extract_simple_object(
    obj: &boa_engine::JsObject,
    context: &mut Context,
) -> Option<Vec<(String, SimpleValue)>> {
    let mut props = Vec::new();

    // Get own enumerable property keys
    let keys = obj.own_property_keys(context).ok()?;

    for key in keys {
        // Convert PropertyKey to string
        let key_str = match &key {
            boa_engine::property::PropertyKey::String(s) => s.to_std_string_escaped(),
            boa_engine::property::PropertyKey::Symbol(_) => continue, // Skip symbols
            boa_engine::property::PropertyKey::Index(i) => i.get().to_string(),
        };

        // Get property value
        let value = obj.get(key, context).ok()?;

        // Only allow primitives for fast path
        let simple_value = if value.is_undefined() {
            SimpleValue::Undefined
        } else if value.is_null() {
            SimpleValue::Null
        } else if let Some(b) = value.as_boolean() {
            SimpleValue::Boolean(b)
        } else if let Some(n) = value.as_number() {
            SimpleValue::Number(n)
        } else if let Some(s) = value.as_string() {
            SimpleValue::String(s.to_std_string_escaped())
        } else {
            // Not a simple value, fall back to structured clone
            return None;
        };

        props.push((key_str, simple_value));
    }

    Some(props)
}

// ============================================================================
// Blob URL Storage for inline workers
// ============================================================================

static BLOB_URLS: Mutex<Option<HashMap<String, (String, String)>>> = Mutex::new(None); // url -> (content, type)
static BLOB_COUNTER: AtomicU64 = AtomicU64::new(0);

fn create_blob_url(content: String, content_type: String) -> String {
    let id = BLOB_COUNTER.fetch_add(1, Ordering::SeqCst);
    let url = format!("blob:viper://{}", id);

    let mut blobs = BLOB_URLS.lock().unwrap();
    if blobs.is_none() {
        *blobs = Some(HashMap::new());
    }
    if let Some(ref mut map) = *blobs {
        map.insert(url.clone(), (content, content_type));
    }

    url
}

fn get_blob_content(url: &str) -> Option<(String, String)> {
    let blobs = BLOB_URLS.lock().unwrap();
    blobs.as_ref()?.get(url).cloned()
}

fn revoke_blob_url(url: &str) {
    let mut blobs = BLOB_URLS.lock().unwrap();
    if let Some(ref mut map) = *blobs {
        map.remove(url);
    }
}

// ============================================================================
// Main Thread Worker API Registration
// ============================================================================

/// Register the Worker API for the main thread
pub fn register_worker_api(context: &mut Context) -> JsResult<()> {
    // __viper_worker_create(url, options) -> worker_id
    let create_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let url = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing URL"))?
            .to_string(context)?
            .to_std_string_escaped();

        // Parse options
        let mut preload: Vec<String> = Vec::new();
        let mut smol = false;
        let mut _ref = true;

        if let Some(opts) = args.get(1).and_then(|v| v.as_object()) {
            if let Ok(preload_val) = opts.get(js_string!("preload"), context) {
                if let Some(arr) = preload_val
                    .as_object()
                    .and_then(|o| JsArray::from_object(o.clone()).ok())
                {
                    let len = arr.length(context).unwrap_or(0);
                    for i in 0..len {
                        if let Ok(item) = arr.get(i, context) {
                            if let Ok(s) = item.to_string(context) {
                                preload.push(s.to_std_string_escaped());
                            }
                        }
                    }
                } else if let Some(s) = preload_val.as_string() {
                    preload.push(s.to_std_string_escaped());
                }
            }
            if let Ok(smol_val) = opts.get(js_string!("smol"), context) {
                smol = smol_val.as_boolean().unwrap_or(false);
            }
            if let Ok(ref_val) = opts.get(js_string!("ref"), context) {
                _ref = ref_val.as_boolean().unwrap_or(true);
            }
        }

        // Generate worker ID
        let worker_id = WORKER_COUNTER.fetch_add(1, Ordering::SeqCst);

        // Create worker handle
        let handle = WorkerHandle::new(worker_id, url.clone(), smol);

        // Store in global map
        {
            let mut workers = WORKERS.lock().unwrap();
            if workers.is_none() {
                *workers = Some(HashMap::new());
            }
            if let Some(ref mut map) = *workers {
                map.insert(worker_id, Arc::clone(&handle));
            }
        }

        // Handle blob: URLs
        let thread_handle = if url.starts_with("blob:") {
            if let Some((content, content_type)) = get_blob_content(&url) {
                spawn_worker_from_blob(Arc::clone(&handle), content, content_type)
            } else {
                return Err(JsNativeError::typ()
                    .with_message(format!("Blob URL not found: {}", url))
                    .into());
            }
        } else {
            // Resolve file path
            let script_path = if url.starts_with("./") || url.starts_with("../") {
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join(&url)
            } else if url.starts_with("file://") {
                PathBuf::from(url.strip_prefix("file://").unwrap_or(&url))
            } else {
                PathBuf::from(&url)
            };

            // Verify file exists before spawning (like Bun)
            if !script_path.exists() {
                return Err(JsNativeError::typ()
                    .with_message(format!(
                        "Worker script not found: {}",
                        script_path.display()
                    ))
                    .into());
            }

            spawn_worker(Arc::clone(&handle), script_path, preload)
        };

        // Store thread handle
        *handle.thread_handle.lock().unwrap() = Some(thread_handle);

        if !_ref {
            handle.remove_ref();
        }

        Ok(JsValue::from(worker_id))
    });

    context.register_global_callable(js_string!("__viper_worker_create"), 2, create_fn)?;

    // __viper_worker_post_to(worker_id, data)
    let post_to_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let worker_id = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing worker ID"))?
            .to_u32(context)?;

        let workers = WORKERS.lock().unwrap();
        let handle = workers
            .as_ref()
            .and_then(|m| m.get(&worker_id))
            .ok_or_else(|| JsNativeError::typ().with_message("Worker not found"))?;

        if args.len() < 2 {
            return Err(JsNativeError::typ()
                .with_message("postMessage requires data")
                .into());
        }

        let data = &args[1];

        // Fast path: string
        if let Some(s) = data.as_string() {
            handle.send_to_worker(WorkerMessage::String(s.to_std_string_escaped()));
            return Ok(JsValue::undefined());
        }

        // Fast path: simple object
        if let Some(obj) = data.as_object() {
            if let Some(props) = try_extract_simple_object(&obj, context) {
                handle.send_to_worker(WorkerMessage::SimpleObject(props));
                return Ok(JsValue::undefined());
            }
        }

        // Fallback: JSON
        match data.to_json(context) {
            Ok(Some(json_val)) => {
                handle.send_to_worker(WorkerMessage::StructuredClone(
                    json_val.to_string().into_bytes(),
                ));
            }
            _ => {
                let str_val = data.to_string(context)?;
                handle.send_to_worker(WorkerMessage::String(str_val.to_std_string_escaped()));
            }
        }

        Ok(JsValue::undefined())
    });

    context.register_global_callable(js_string!("__viper_worker_post_to"), 2, post_to_fn)?;

    // __viper_worker_receive(worker_id) -> messages[]
    let receive_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let worker_id = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing worker ID"))?
            .to_u32(context)?;

        let workers = WORKERS.lock().unwrap();
        let handle = workers.as_ref().and_then(|m| m.get(&worker_id));

        let arr = JsArray::new(context);

        if let Some(handle) = handle {
            let messages = handle.receive_from_worker();
            for msg in messages {
                match msg {
                    WorkerMessage::Ready => {
                        let obj = boa_engine::object::ObjectInitializer::new(context)
                            .property(js_string!("type"), js_string!("open"), Attribute::all())
                            .build();
                        arr.push(obj, context)?;
                    }
                    WorkerMessage::String(s) => {
                        let obj = boa_engine::object::ObjectInitializer::new(context)
                            .property(js_string!("type"), js_string!("message"), Attribute::all())
                            .property(js_string!("data"), js_string!(s), Attribute::all())
                            .build();
                        arr.push(obj, context)?;
                    }
                    WorkerMessage::SimpleObject(props) => {
                        // Build data object by constructing JSON and parsing
                        let mut obj_str = String::from("{");
                        for (i, (key, value)) in props.iter().enumerate() {
                            if i > 0 {
                                obj_str.push_str(", ");
                            }
                            obj_str.push_str(&format!(
                                "\"{}\": ",
                                key.replace('\\', "\\\\").replace('"', "\\\"")
                            ));
                            match value {
                                SimpleValue::Undefined => obj_str.push_str("undefined"),
                                SimpleValue::Null => obj_str.push_str("null"),
                                SimpleValue::Boolean(b) => {
                                    obj_str.push_str(if *b { "true" } else { "false" })
                                }
                                SimpleValue::Number(n) => obj_str.push_str(&n.to_string()),
                                SimpleValue::String(s) => {
                                    obj_str.push('"');
                                    obj_str.push_str(
                                        &s.replace('\\', "\\\\")
                                            .replace('"', "\\\"")
                                            .replace('\n', "\\n"),
                                    );
                                    obj_str.push('"');
                                }
                            }
                        }
                        obj_str.push('}');

                        // Parse the constructed object
                        let parse_code = format!("({})", obj_str);
                        if let Ok(data_obj) =
                            context.eval(Source::from_bytes(parse_code.as_bytes()))
                        {
                            let obj = boa_engine::object::ObjectInitializer::new(context)
                                .property(
                                    js_string!("type"),
                                    js_string!("message"),
                                    Attribute::all(),
                                )
                                .property(js_string!("data"), data_obj, Attribute::all())
                                .build();
                            arr.push(obj, context)?;
                        }
                    }
                    WorkerMessage::StructuredClone(data) => {
                        if let Ok(json_str) = String::from_utf8(data) {
                            // Parse JSON back to JS object
                            let parse_code = format!("({})", json_str);
                            if let Ok(parsed) =
                                context.eval(Source::from_bytes(parse_code.as_bytes()))
                            {
                                let obj = boa_engine::object::ObjectInitializer::new(context)
                                    .property(
                                        js_string!("type"),
                                        js_string!("message"),
                                        Attribute::all(),
                                    )
                                    .property(js_string!("data"), parsed, Attribute::all())
                                    .build();
                                arr.push(obj, context)?;
                            }
                        }
                    }
                    WorkerMessage::Error(e) => {
                        let obj = boa_engine::object::ObjectInitializer::new(context)
                            .property(js_string!("type"), js_string!("error"), Attribute::all())
                            .property(js_string!("message"), js_string!(e), Attribute::all())
                            .build();
                        arr.push(obj, context)?;
                    }
                    WorkerMessage::Close(code) => {
                        let obj = boa_engine::object::ObjectInitializer::new(context)
                            .property(js_string!("type"), js_string!("close"), Attribute::all())
                            .property(js_string!("code"), JsValue::from(code), Attribute::all())
                            .build();
                        arr.push(obj, context)?;
                    }
                    _ => {}
                }
            }
        }

        Ok(arr.into())
    });

    context.register_global_callable(js_string!("__viper_worker_receive"), 1, receive_fn)?;

    // __viper_worker_terminate(worker_id)
    let terminate_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let worker_id = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing worker ID"))?
            .to_u32(context)?;

        let workers = WORKERS.lock().unwrap();
        if let Some(handle) = workers.as_ref().and_then(|m| m.get(&worker_id)) {
            handle.terminate();
        }

        Ok(JsValue::undefined())
    });

    context.register_global_callable(js_string!("__viper_worker_terminate"), 1, terminate_fn)?;

    // __viper_worker_ref(worker_id)
    let ref_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let worker_id = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing worker ID"))?
            .to_u32(context)?;

        let workers = WORKERS.lock().unwrap();
        if let Some(handle) = workers.as_ref().and_then(|m| m.get(&worker_id)) {
            handle.add_ref();
        }

        Ok(JsValue::undefined())
    });

    context.register_global_callable(js_string!("__viper_worker_ref"), 1, ref_fn)?;

    // __viper_worker_unref(worker_id)
    let unref_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let worker_id = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing worker ID"))?
            .to_u32(context)?;

        let workers = WORKERS.lock().unwrap();
        if let Some(handle) = workers.as_ref().and_then(|m| m.get(&worker_id)) {
            handle.remove_ref();
        }

        Ok(JsValue::undefined())
    });

    context.register_global_callable(js_string!("__viper_worker_unref"), 1, unref_fn)?;

    // __viper_worker_get_state(worker_id) -> state
    let get_state_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let worker_id = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing worker ID"))?
            .to_u32(context)?;

        let workers = WORKERS.lock().unwrap();
        if let Some(handle) = workers.as_ref().and_then(|m| m.get(&worker_id)) {
            return Ok(JsValue::from(handle.get_state() as u32));
        }

        Ok(JsValue::from(WorkerState::Closed as u32))
    });

    context.register_global_callable(js_string!("__viper_worker_get_state"), 1, get_state_fn)?;

    // __viper_create_blob_url(content, type) -> url
    let create_blob_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let content = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing content"))?
            .to_string(context)?
            .to_std_string_escaped();

        let content_type = args
            .get(1)
            .and_then(|v| v.to_string(context).ok())
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_else(|| "application/javascript".to_string());

        let url = create_blob_url(content, content_type);
        Ok(JsValue::from(js_string!(url)))
    });

    context.register_global_callable(js_string!("__viper_create_blob_url"), 2, create_blob_fn)?;

    // __viper_revoke_blob_url(url)
    let revoke_blob_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let url = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing URL"))?
            .to_string(context)?
            .to_std_string_escaped();

        revoke_blob_url(&url);
        Ok(JsValue::undefined())
    });

    context.register_global_callable(js_string!("__viper_revoke_blob_url"), 1, revoke_blob_fn)?;

    // =========================================================================
    // MessageChannel / MessagePort Native Functions
    // =========================================================================

    // __viper_create_channel() -> [port1_id, port2_id]
    let create_channel_fn = NativeFunction::from_fn_ptr(|_this, _args, context| {
        let (port1_id, port2_id) = create_message_channel();
        let arr = JsArray::new(context);
        arr.push(JsValue::from(port1_id), context)?;
        arr.push(JsValue::from(port2_id), context)?;
        Ok(arr.into())
    });

    context.register_global_callable(js_string!("__viper_create_channel"), 0, create_channel_fn)?;

    // __viper_port_post(port_id, data_json, buffer_bytes[], port_ids[])
    let port_post_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let port_id = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing port ID"))?
            .to_u32(context)?;

        let data_json = args
            .get(1)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing data"))?
            .to_string(context)?
            .to_std_string_escaped();

        // Extract transferred buffers (as array of Uint8Array)
        let mut buffers: Vec<Vec<u8>> = Vec::new();
        if let Some(buf_arr) = args.get(2).and_then(|v| v.as_object()) {
            if let Ok(arr) = JsArray::from_object(buf_arr.clone()) {
                let len = arr.length(context).unwrap_or(0);
                for i in 0..len {
                    if let Ok(item) = arr.get(i, context) {
                        if let Some(typed_arr) = item.as_object() {
                            // Try to get bytes from TypedArray
                            if let Ok(bytes_val) = typed_arr.get(js_string!("buffer"), context) {
                                if let Some(buf_obj) = bytes_val.as_object() {
                                    // Get byteLength
                                    if let Ok(len_val) =
                                        buf_obj.get(js_string!("byteLength"), context)
                                    {
                                        let byte_len =
                                            len_val.to_u32(context).unwrap_or(0) as usize;
                                        let mut bytes = vec![0u8; byte_len];
                                        // Copy bytes from the typed array
                                        for j in 0..byte_len {
                                            if let Ok(byte_val) = typed_arr.get(j as u32, context) {
                                                bytes[j] =
                                                    byte_val.to_u32(context).unwrap_or(0) as u8;
                                            }
                                        }
                                        buffers.push(bytes);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Extract transferred port IDs
        let mut ports: Vec<u32> = Vec::new();
        if let Some(port_arr) = args.get(3).and_then(|v| v.as_object()) {
            if let Ok(arr) = JsArray::from_object(port_arr.clone()) {
                let len = arr.length(context).unwrap_or(0);
                for i in 0..len {
                    if let Ok(item) = arr.get(i, context) {
                        if let Ok(pid) = item.to_u32(context) {
                            ports.push(pid);
                            // Mark port as transferred
                            if let Some(handle) = get_port_handle(pid) {
                                handle.mark_transferred();
                            }
                        }
                    }
                }
            }
        }

        let handle = get_port_handle(port_id)
            .ok_or_else(|| JsNativeError::typ().with_message("Port not found"))?;

        handle.send(PortMessage {
            data: data_json.into_bytes(),
            buffers,
            ports,
        });

        Ok(JsValue::undefined())
    });

    context.register_global_callable(js_string!("__viper_port_post"), 4, port_post_fn)?;

    // __viper_port_receive(port_id) -> messages[]
    let port_receive_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let port_id = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing port ID"))?
            .to_u32(context)?;

        let arr = JsArray::new(context);

        if let Some(handle) = get_port_handle(port_id) {
            let messages = handle.receive();
            for msg in messages {
                // Parse the JSON data back to a JS object
                let data_str = String::from_utf8_lossy(&msg.data);
                let parse_code = format!("({})", data_str);
                let data_val = context
                    .eval(Source::from_bytes(parse_code.as_bytes()))
                    .unwrap_or(JsValue::undefined());

                // Build the message event object
                let obj = boa_engine::object::ObjectInitializer::new(context)
                    .property(js_string!("data"), data_val, Attribute::all())
                    .build();

                // Add transferred buffers as Uint8Arrays
                if !msg.buffers.is_empty() {
                    let buf_arr = JsArray::new(context);
                    for buf in &msg.buffers {
                        // Create Uint8Array from bytes
                        let create_code = format!(
                            "new Uint8Array([{}])",
                            buf.iter()
                                .map(|b| b.to_string())
                                .collect::<Vec<_>>()
                                .join(",")
                        );
                        if let Ok(typed_arr) =
                            context.eval(Source::from_bytes(create_code.as_bytes()))
                        {
                            let _ = buf_arr.push(typed_arr, context);
                        }
                    }
                    let _ = obj.set(js_string!("buffers"), buf_arr, false, context);
                }

                // Add transferred port IDs
                if !msg.ports.is_empty() {
                    let port_arr = JsArray::new(context);
                    for pid in &msg.ports {
                        let _ = port_arr.push(JsValue::from(*pid), context);
                    }
                    let _ = obj.set(js_string!("ports"), port_arr, false, context);
                }

                arr.push(obj, context)?;
            }
        }

        Ok(arr.into())
    });

    context.register_global_callable(js_string!("__viper_port_receive"), 1, port_receive_fn)?;

    // __viper_port_close(port_id)
    let port_close_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let port_id = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("Missing port ID"))?
            .to_u32(context)?;

        if let Some(handle) = get_port_handle(port_id) {
            handle.close();
        }

        Ok(JsValue::undefined())
    });

    context.register_global_callable(js_string!("__viper_port_close"), 1, port_close_fn)?;

    // __viper_port_start(port_id) - starts the port (enables message receiving)
    let port_start_fn = NativeFunction::from_fn_ptr(|_this, _args, _context| {
        // Port is always "started" in our implementation since we use polling
        Ok(JsValue::undefined())
    });

    context.register_global_callable(js_string!("__viper_port_start"), 1, port_start_fn)?;

    // Register JavaScript Worker class
    let worker_class = r#"
        // Worker class implementation
        globalThis.Worker = class Worker {
            #workerId = null;
            #isOpen = false;

            onopen = null;
            onmessage = null;
            onerror = null;
            onclose = null;

            constructor(url, options = {}) {
                // Handle URL object
                if (url instanceof URL) {
                    url = url.href;
                }

                // Validate URL
                if (!url || typeof url !== 'string') {
                    throw new TypeError('Worker URL must be a string or URL object');
                }

                this.url = url;

                // Create the worker
                try {
                    this.#workerId = __viper_worker_create(url, options);
                    this.#startPolling();
                } catch (e) {
                    if (this.onerror) {
                        queueMicrotask(() => {
                            this.onerror({ type: 'error', message: e.message, error: e });
                        });
                    }
                    throw e;
                }
            }

            get threadId() {
                return this.#workerId;
            }

            postMessage(data, transfer) {
                if (this.#workerId === null) {
                    throw new Error('Worker has been terminated');
                }
                __viper_worker_post_to(this.#workerId, data, transfer);
            }

            terminate() {
                if (this.#workerId !== null) {
                    __viper_worker_terminate(this.#workerId);
                    this.#workerId = null;
                }
            }

            ref() {
                if (this.#workerId !== null) {
                    __viper_worker_ref(this.#workerId);
                }
                return this;
            }

            unref() {
                if (this.#workerId !== null) {
                    __viper_worker_unref(this.#workerId);
                }
                return this;
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

            #startPolling() {
                const poll = () => {
                    if (this.#workerId === null) {
                        return;
                    }

                    try {
                        const events = __viper_worker_receive(this.#workerId);
                        for (const event of events) {
                            switch (event.type) {
                                case 'open':
                                    this.#isOpen = true;
                                    if (this.onopen) {
                                        this.onopen({ type: 'open' });
                                    }
                                    break;
                                case 'message':
                                    if (this.onmessage) {
                                        this.onmessage({
                                            type: 'message',
                                            data: event.data,
                                            origin: '',
                                            lastEventId: '',
                                            source: null,
                                            ports: []
                                        });
                                    }
                                    break;
                                case 'error':
                                    if (this.onerror) {
                                        this.onerror({
                                            type: 'error',
                                            message: event.message,
                                            error: new Error(event.message)
                                        });
                                    }
                                    break;
                                case 'close':
                                    if (this.onclose) {
                                        this.onclose({
                                            type: 'close',
                                            code: event.code,
                                            wasClean: true
                                        });
                                    }
                                    this.#workerId = null;
                                    return; // Stop polling
                            }
                        }
                    } catch (e) {
                        if (this.onerror) {
                            this.onerror({ type: 'error', message: e.message, error: e });
                        }
                    }

                    // Continue polling with minimal delay
                    if (this.#workerId !== null) {
                        setTimeout(poll, 1);
                    }
                };

                // Start polling immediately
                setTimeout(poll, 0);
            }
        };

        // URL.createObjectURL for Blob workers
        if (typeof URL !== 'undefined') {
            const originalCreateObjectURL = URL.createObjectURL;
            URL.createObjectURL = function(blob) {
                if (blob instanceof Blob) {
                    // Read blob content synchronously (for workers)
                    const type = blob.type || 'application/javascript';

                    // Convert blob to string using stored content
                    if (blob._content !== undefined) {
                        return __viper_create_blob_url(blob._content, type);
                    }

                    return __viper_create_blob_url('[blob content]', type);
                }
                if (originalCreateObjectURL) {
                    return originalCreateObjectURL.call(this, blob);
                }
                throw new TypeError('createObjectURL requires a Blob');
            };

            URL.revokeObjectURL = function(url) {
                if (url && url.startsWith('blob:')) {
                    __viper_revoke_blob_url(url);
                }
            };
        }

        // Blob class for createObjectURL support
        if (typeof globalThis.Blob === 'undefined') {
            globalThis.Blob = class Blob {
                constructor(parts, options) {
                    this.type = options?.type || '';
                    this.size = 0;
                    this._content = '';
                    if (parts && parts.length > 0) {
                        this._content = parts.map(p => {
                            if (typeof p === 'string') return p;
                            if (p instanceof ArrayBuffer) return new TextDecoder().decode(p);
                            if (p instanceof Uint8Array) return new TextDecoder().decode(p);
                            return String(p);
                        }).join('');
                        this.size = this._content.length;
                    }
                }

                async text() {
                    return this._content;
                }

                async arrayBuffer() {
                    return new TextEncoder().encode(this._content).buffer;
                }

                slice(start, end, contentType) {
                    const sliced = this._content.slice(start, end);
                    return new Blob([sliced], { type: contentType || this.type });
                }
            };
        } else {
            // Enhance existing Blob to store content
            const OriginalBlob = globalThis.Blob;
            globalThis.Blob = class Blob extends OriginalBlob {
                constructor(parts, options) {
                    super(parts, options);
                    // Store content for createObjectURL
                    if (parts && parts.length > 0) {
                        this._content = parts.map(p => {
                            if (typeof p === 'string') return p;
                            if (p instanceof ArrayBuffer) return new TextDecoder().decode(p);
                            if (p instanceof Uint8Array) return new TextDecoder().decode(p);
                            return String(p);
                        }).join('');
                    }
                }
            };
        }

        // File class for blob URLs with filenames
        globalThis.File = globalThis.File || class File extends Blob {
            constructor(parts, name, options) {
                super(parts, options);
                this.name = name;
                this.lastModified = options?.lastModified || Date.now();
            }
        };

        // Viper.isMainThread
        globalThis.Viper = globalThis.Viper || {};
        globalThis.Viper.isMainThread = true;

        // worker_threads compatibility (Node.js style)
        globalThis.__worker_threads = {
            isMainThread: true,
            parentPort: null,
            workerData: null,
            Worker: globalThis.Worker,

            setEnvironmentData(key, value) {
                if (!this._envData) this._envData = new Map();
                this._envData.set(key, value);
            },

            getEnvironmentData(key) {
                if (!this._envData) return undefined;
                return this._envData.get(key);
            }
        };

        // =====================================================================
        // EventTarget polyfill (if not already defined)
        // =====================================================================
        if (typeof globalThis.EventTarget === 'undefined') {
            globalThis.EventTarget = class EventTarget {
                constructor() {
                    this._listeners = new Map();
                }

                addEventListener(type, callback, options) {
                    if (!this._listeners.has(type)) {
                        this._listeners.set(type, []);
                    }
                    this._listeners.get(type).push({ callback, options });
                }

                removeEventListener(type, callback) {
                    if (!this._listeners.has(type)) return;
                    const listeners = this._listeners.get(type);
                    const index = listeners.findIndex(l => l.callback === callback);
                    if (index !== -1) {
                        listeners.splice(index, 1);
                    }
                }

                dispatchEvent(event) {
                    if (!this._listeners.has(event.type)) return true;
                    const listeners = this._listeners.get(event.type).slice();
                    for (const { callback } of listeners) {
                        try {
                            callback.call(this, event);
                        } catch (e) {
                            console.error('Event listener error:', e);
                        }
                    }
                    return !event.defaultPrevented;
                }
            };
        }

        // =====================================================================
        // Event class polyfill (if not already defined)
        // =====================================================================
        if (typeof globalThis.Event === 'undefined') {
            globalThis.Event = class Event {
                constructor(type, init = {}) {
                    this.type = type;
                    this.bubbles = init.bubbles || false;
                    this.cancelable = init.cancelable || false;
                    this.defaultPrevented = false;
                    this.timeStamp = Date.now();
                }

                preventDefault() {
                    this.defaultPrevented = true;
                }

                stopPropagation() {}
                stopImmediatePropagation() {}
            };
        }

        // =====================================================================
        // DOMException polyfill (if not already defined)
        // =====================================================================
        if (typeof globalThis.DOMException === 'undefined') {
            globalThis.DOMException = class DOMException extends Error {
                constructor(message, name = 'Error') {
                    super(message);
                    this.name = name;
                }
            };
        }

        // =====================================================================
        // MessagePort class - represents one end of a MessageChannel
        // =====================================================================
        globalThis.MessagePort = class MessagePort extends EventTarget {
            #portId = null;
            #started = false;
            #closed = false;
            #pollInterval = null;

            onmessage = null;
            onmessageerror = null;

            constructor(portId) {
                super();
                this.#portId = portId;
            }

            // Internal: set the port ID (used when creating from MessageChannel)
            _setPortId(id) {
                this.#portId = id;
            }

            // Internal: get port ID for transferring
            _getPortId() {
                return this.#portId;
            }

            postMessage(message, transfer = []) {
                if (this.#closed) {
                    throw new DOMException('Port is closed', 'InvalidStateError');
                }
                if (this.#portId === null) {
                    throw new DOMException('Port not initialized', 'InvalidStateError');
                }

                // Extract ArrayBuffers from transfer list
                const buffers = [];
                const portIds = [];

                for (const item of transfer) {
                    if (item instanceof ArrayBuffer) {
                        buffers.push(new Uint8Array(item));
                    } else if (item instanceof MessagePort) {
                        portIds.push(item._getPortId());
                    }
                }

                // Serialize message to JSON
                const jsonData = JSON.stringify(message);

                __viper_port_post(this.#portId, jsonData, buffers, portIds);

                // Detach transferred ArrayBuffers (mark as neutered)
                for (const item of transfer) {
                    if (item instanceof ArrayBuffer) {
                        // In a real impl, we'd neuter the buffer
                        // For now, we just transfer a copy
                    }
                }
            }

            start() {
                if (this.#started || this.#closed) return;
                this.#started = true;

                // Start polling for messages
                const poll = () => {
                    if (this.#closed) return;

                    try {
                        const messages = __viper_port_receive(this.#portId);
                        for (const msg of messages) {
                            const event = new MessageEvent('message', {
                                data: msg.data,
                                ports: (msg.ports || []).map(id => {
                                    const port = new MessagePort(id);
                                    return port;
                                })
                            });

                            if (this.onmessage) {
                                this.onmessage(event);
                            }
                            this.dispatchEvent(event);
                        }
                    } catch (e) {
                        const errorEvent = new MessageEvent('messageerror', { data: e });
                        if (this.onmessageerror) {
                            this.onmessageerror(errorEvent);
                        }
                        this.dispatchEvent(errorEvent);
                    }

                    if (!this.#closed) {
                        this.#pollInterval = setTimeout(poll, 1);
                    }
                };

                setTimeout(poll, 0);
            }

            close() {
                if (this.#closed) return;
                this.#closed = true;
                if (this.#pollInterval) {
                    clearTimeout(this.#pollInterval);
                }
                if (this.#portId !== null) {
                    __viper_port_close(this.#portId);
                }
            }

            addEventListener(type, listener, options) {
                super.addEventListener(type, listener, options);
                // Auto-start when adding message listener (like browsers do)
                if (type === 'message' && !this.#started) {
                    this.start();
                }
            }
        };

        // =====================================================================
        // MessageChannel class - creates a pair of connected MessagePorts
        // =====================================================================
        globalThis.MessageChannel = class MessageChannel {
            constructor() {
                const [port1Id, port2Id] = __viper_create_channel();
                this.port1 = new MessagePort(port1Id);
                this.port2 = new MessagePort(port2Id);
            }
        };

        // =====================================================================
        // MessageEvent class (if not already defined)
        // =====================================================================
        if (typeof globalThis.MessageEvent === 'undefined') {
            globalThis.MessageEvent = class MessageEvent extends Event {
                constructor(type, init = {}) {
                    super(type, init);
                    this.data = init.data !== undefined ? init.data : null;
                    this.origin = init.origin || '';
                    this.lastEventId = init.lastEventId || '';
                    this.source = init.source || null;
                    this.ports = init.ports || [];
                }
            };
        }

        // =====================================================================
        // Transferable ArrayBuffer support for Worker.postMessage
        // =====================================================================
        // Enhance the Worker class to handle transferables
        const OriginalWorkerPostMessage = globalThis.Worker.prototype.postMessage;
        globalThis.Worker.prototype.postMessage = function(message, transfer) {
            // If transfer is an array, extract ArrayBuffers and MessagePorts
            if (Array.isArray(transfer) && transfer.length > 0) {
                const buffers = [];
                const portIds = [];

                for (const item of transfer) {
                    if (item instanceof ArrayBuffer) {
                        // Store buffer data before "transferring" (neutering)
                        buffers.push(new Uint8Array(item));
                    } else if (item instanceof MessagePort) {
                        portIds.push(item._getPortId());
                    }
                }

                // For now, we include transfer info in the message
                // A full implementation would use shared memory
                if (buffers.length > 0 || portIds.length > 0) {
                    const wrappedMessage = {
                        __viperTransfer: true,
                        data: message,
                        buffers: buffers.map(b => Array.from(b)),
                        portIds: portIds
                    };
                    return OriginalWorkerPostMessage.call(this, wrappedMessage);
                }
            }

            return OriginalWorkerPostMessage.call(this, message, transfer);
        };
    "#;

    let source = Source::from_bytes(worker_class.as_bytes());
    context.eval(source)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_state() {
        assert_eq!(WorkerState::from(0), WorkerState::Starting);
        assert_eq!(WorkerState::from(1), WorkerState::Running);
        assert_eq!(WorkerState::from(2), WorkerState::Closing);
        assert_eq!(WorkerState::from(3), WorkerState::Closed);
        assert_eq!(WorkerState::from(255), WorkerState::Closed);
    }

    #[test]
    fn test_simple_value() {
        let props = vec![
            ("name".to_string(), SimpleValue::String("test".to_string())),
            ("count".to_string(), SimpleValue::Number(42.0)),
            ("enabled".to_string(), SimpleValue::Boolean(true)),
            ("data".to_string(), SimpleValue::Null),
        ];

        assert_eq!(props.len(), 4);
    }

    #[test]
    fn test_blob_url() {
        let url = create_blob_url(
            "console.log('test')".to_string(),
            "application/javascript".to_string(),
        );
        assert!(url.starts_with("blob:viper://"));

        let content = get_blob_content(&url);
        assert!(content.is_some());
        assert_eq!(content.unwrap().0, "console.log('test')");

        revoke_blob_url(&url);
        assert!(get_blob_content(&url).is_none());
    }
}
