//! JavaScript runtime powered by Boa engine
//!
//! This module provides the JavaScript execution environment using
//! the Boa JS engine with WebAPI support from boa_runtime.
//!
//! Features provided:
//! - Console API (console.log, console.error, etc.)
//! - Timers (setTimeout, setInterval, clearTimeout, clearInterval)
//! - URL API (URL, URLSearchParams)
//! - Text encoding (TextEncoder, TextDecoder)
//! - structuredClone
//! - queueMicrotask
//! - Promise/async-await support
//! - ES Modules with TypeScript transpilation

use boa_engine::{
    Context, JsError, JsResult, JsString, JsValue, Source,
    builtins::promise::PromiseState,
    context::ContextBuilder,
    js_string,
    module::{Module, ModuleLoader, Referrer},
};
use boa_gc::{Finalize, Trace};
use boa_runtime::{
    ConsoleState, Logger,
    extensions::{
        ConsoleExtension, EncodingExtension, FetchExtension, MicrotaskExtension,
        StructuredCloneExtension, TimeoutExtension, UrlExtension,
    },
    fetch::BlockingReqwestFetcher,
    register_extensions,
};
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};
use thiserror::Error;

mod crypto;
mod event_loop;
mod path;
mod process;
mod server_api;
mod spawn;
mod websocket;
pub mod worker;

use crate::fs;
use crate::resolver::ModuleResolver;
use crate::transpiler::{Transpiler, TranspilerConfig};
use event_loop::ViperEventLoop;

/// Errors that can occur during runtime execution
#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("JavaScript error: {0}")]
    JsError(String),

    #[error("Transpilation error: {0}")]
    TranspileError(#[from] crate::transpiler::TranspileError),

    #[allow(dead_code)]
    #[error("Module load error: {0}")]
    ModuleError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Result type for runtime operations
pub type RuntimeResult<T> = Result<T, RuntimeError>;

/// Custom logger that prints to stdout/stderr
#[derive(Debug, Clone, Default, Trace, Finalize)]
pub struct ViperLogger;

impl Logger for ViperLogger {
    fn log(&self, msg: String, _state: &ConsoleState, _context: &mut Context) -> JsResult<()> {
        println!("{}", msg);
        Ok(())
    }

    fn info(&self, msg: String, _state: &ConsoleState, _context: &mut Context) -> JsResult<()> {
        println!("[INFO] {}", msg);
        Ok(())
    }

    fn warn(&self, msg: String, _state: &ConsoleState, _context: &mut Context) -> JsResult<()> {
        eprintln!("[WARN] {}", msg);
        Ok(())
    }

    fn error(&self, msg: String, _state: &ConsoleState, _context: &mut Context) -> JsResult<()> {
        eprintln!("[ERROR] {}", msg);
        Ok(())
    }
}

/// TypeScript module loader that transpiles .ts files on-the-fly
/// Uses oxc_resolver for Node.js/Bun-compatible module resolution
pub struct TypeScriptModuleLoader {
    base_path: PathBuf,
    transpiler: Transpiler,
    resolver: ModuleResolver,
}

impl TypeScriptModuleLoader {
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        let base = base_path.as_ref().to_path_buf();
        Self {
            base_path: base.clone(),
            transpiler: Transpiler::new(),
            resolver: ModuleResolver::new(&base),
        }
    }

    /// Get built-in module code for Node.js compatible modules
    fn get_builtin_module(specifier: &str) -> Option<String> {
        match specifier {
            "path" | "node:path" => Some(
                r#"
                const p = globalThis.path;
                export default p;
                export const {
                    sep, delimiter, join, resolve, normalize, dirname,
                    basename, extname, isAbsolute, relative, parse,
                    format, toNamespacedPath, matchesGlob, posix, win32
                } = p;
                "#
                .to_string(),
            ),
            // Add more built-in modules here as they're implemented
            _ => None,
        }
    }
}

impl TypeScriptModuleLoader {
    /// Check if code is CommonJS (has module.exports or require())
    fn is_commonjs(code: &str) -> bool {
        // Simple heuristic: check for CommonJS patterns
        // Avoid false positives from comments
        for line in code.lines() {
            let trimmed = line.trim();
            // Skip comments
            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*") {
                continue;
            }
            // Check for CommonJS patterns
            if trimmed.contains("module.exports")
                || trimmed.contains("exports.")
                || (trimmed.contains("require(") && !trimmed.contains("import"))
            {
                return true;
            }
            // Check for ESM patterns (if found, it's not CJS)
            if trimmed.starts_with("import ") || trimmed.starts_with("export ") {
                return false;
            }
        }
        false
    }

    /// Recursively bundle a CommonJS module and all its dependencies
    fn bundle_commonjs(&self, entry_path: &Path) -> Result<String, String> {
        use std::collections::{HashMap, HashSet};

        // Maps: module_id -> (code, require_map)
        // require_map: original_specifier -> resolved_module_id
        let mut modules: HashMap<String, (String, HashMap<String, String>)> = HashMap::new();
        let mut visited: HashSet<String> = HashSet::new();

        // Recursively collect all required modules
        self.collect_cjs_modules(entry_path, &mut modules, &mut visited)?;

        let entry_id = entry_path.to_string_lossy().replace('\\', "/");

        // Build the bundle with a require runtime
        let mut bundle = String::from(
            r#"
// CommonJS Runtime
const __cjs_modules__ = {};
const __cjs_cache__ = {};

function __cjs_require__(id) {
    if (__cjs_cache__[id]) {
        return __cjs_cache__[id].exports;
    }

    const module = { exports: {} };
    __cjs_cache__[id] = module;

    const moduleFunc = __cjs_modules__[id];
    if (!moduleFunc) {
        throw new Error(`Cannot find module '${id}'`);
    }

    moduleFunc(module.exports, module, __cjs_require__, id);
    return module.exports;
}

"#,
        );

        // Add each module as a function
        for (id, (code, require_map)) in &modules {
            // Build a local require function that maps specifiers to resolved IDs
            let mut require_mappings = String::from("const __require_map__ = {\n");
            for (spec, resolved_id) in require_map {
                require_mappings.push_str(&format!(
                    "  \"{}\": \"{}\",\n",
                    spec.replace('\\', "\\\\").replace('"', "\\\""),
                    resolved_id
                ));
            }
            require_mappings.push_str("};\n");

            bundle.push_str(&format!(
                r#"__cjs_modules__["{}"] = function(exports, module, __parent_require__, __filename) {{
const __dirname = __filename.substring(0, __filename.lastIndexOf('/'));
{}
function require(specifier) {{
    const resolved = __require_map__[specifier];
    if (resolved) {{
        return __parent_require__(resolved);
    }}
    throw new Error(`Cannot find module '${{specifier}}'`);
}}
{}
}};

"#,
                id, require_mappings, code
            ));
        }

        // Execute entry and export
        bundle.push_str(&format!(
            r#"
// Execute entry module and export
const __entry_exports__ = __cjs_require__("{}");
export default __entry_exports__;
export {{ __entry_exports__ as module }};
"#,
            entry_id
        ));

        Ok(bundle)
    }

    /// Recursively collect all CommonJS modules with their require mappings
    fn collect_cjs_modules(
        &self,
        file_path: &Path,
        modules: &mut std::collections::HashMap<
            String,
            (String, std::collections::HashMap<String, String>),
        >,
        visited: &mut std::collections::HashSet<String>,
    ) -> Result<(), String> {
        let file_id = file_path.to_string_lossy().replace('\\', "/");

        if visited.contains(&file_id) {
            return Ok(());
        }
        visited.insert(file_id.clone());

        let code = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read {}: {}", file_path.display(), e))?;

        // Find all require() calls and resolve them
        let requires = Self::find_requires(&code);
        let mut require_map: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        for req in requires {
            // Resolve the require path
            if let Ok(resolved) = self.resolver.resolve(&req, file_path) {
                let resolved_id = resolved.to_string_lossy().replace('\\', "/");
                require_map.insert(req.clone(), resolved_id);
                self.collect_cjs_modules(&resolved, modules, visited)?;
            }
            // If resolution fails, it might be a built-in - we'll handle at runtime
        }

        modules.insert(file_id, (code, require_map));
        Ok(())
    }

    /// Find all require() calls in code (simple regex-like matching)
    fn find_requires(code: &str) -> Vec<String> {
        let mut requires = Vec::new();

        // Simple pattern matching for require('...') or require("...")
        let mut chars = code.chars().peekable();

        while let Some(c) = chars.next() {
            if c == 'r' {
                // Check for "require("
                let rest: String = std::iter::once(c).chain(chars.clone().take(7)).collect();
                if rest.starts_with("require(") {
                    // Skip "require("
                    for _ in 0..7 {
                        chars.next();
                    }

                    // Get the quote character
                    if let Some(quote) = chars.next() {
                        if quote == '\'' || quote == '"' {
                            // Collect until closing quote
                            let mut path = String::new();
                            for ch in chars.by_ref() {
                                if ch == quote {
                                    break;
                                }
                                path.push(ch);
                            }
                            if !path.is_empty() {
                                requires.push(path);
                            }
                        }
                    }
                }
            }
        }

        requires
    }
}

impl ModuleLoader for TypeScriptModuleLoader {
    fn load_imported_module(
        self: Rc<Self>,
        referrer: Referrer,
        specifier: boa_engine::JsString,
        context: &RefCell<&mut Context>,
    ) -> impl std::future::Future<Output = JsResult<Module>> {
        let specifier_str = specifier.to_std_string_escaped();

        async move {
            // Check for built-in modules first
            if let Some(builtin_code) = Self::get_builtin_module(&specifier_str) {
                let mut ctx = context.borrow_mut();
                let source = Source::from_bytes(builtin_code.as_bytes());
                return Module::parse(source, None, &mut *ctx);
            }

            // Get the referrer path using Boa's built-in path() method
            // This properly tracks where each module is loaded from
            let referrer_path = referrer
                .path()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| self.base_path.join("index.ts"));

            // Use oxc_resolver for Node.js/Bun-compatible module resolution
            let resolved_path = self
                .resolver
                .resolve(&specifier_str, &referrer_path)
                .map_err(|e| {
                    JsError::from_opaque(JsValue::from(js_string!(format!(
                        "Failed to resolve module '{}': {}",
                        specifier_str, e
                    ))))
                })?;

            // Read the resolved file
            let source_code = std::fs::read_to_string(&resolved_path).map_err(|e| {
                JsError::from_opaque(JsValue::from(js_string!(format!(
                    "Failed to read module '{}': {}",
                    resolved_path.display(),
                    e
                ))))
            })?;

            // Get the extension to determine if transpilation is needed
            let extension = resolved_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");

            // Transpile TypeScript/TSX files
            let js_code = if matches!(extension, "ts" | "tsx" | "mts") {
                let filename = resolved_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("module.ts");

                self.transpiler
                    .transpile(&source_code, filename)
                    .map_err(|e| JsError::from_opaque(JsValue::from(js_string!(e.to_string()))))?
            } else {
                // JavaScript files don't need transpilation
                source_code
            };

            // Check if this is a CommonJS module and bundle it for ESM compatibility
            let esm_code = if Self::is_commonjs(&js_code) {
                self.bundle_commonjs(&resolved_path)
                    .map_err(|e| JsError::from_opaque(JsValue::from(js_string!(e))))?
            } else {
                js_code
            };

            // Parse and load the module with its path for proper referrer tracking
            // Star exports (export * from "...") are now handled natively by Boa
            // with proper path tracking via Source::with_path()
            let source = Source::from_bytes(esm_code.as_bytes()).with_path(&resolved_path);
            let mut ctx = context.borrow_mut();
            Module::parse(source, None, &mut *ctx)
        }
    }
}

/// Configuration for the Viper runtime
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Base path for module resolution
    pub base_path: PathBuf,
    /// Transpiler configuration
    pub transpiler_config: TranspilerConfig,
    /// Whether to use the high-performance event loop
    pub use_event_loop: bool,
    /// Command-line arguments (for process.argv)
    pub args: Vec<String>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            base_path: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            transpiler_config: TranspilerConfig::default(),
            use_event_loop: true,
            args: std::env::args().collect(),
        }
    }
}

impl RuntimeConfig {
    /// Create a config with command-line arguments
    pub fn with_args(args: Vec<String>) -> Self {
        Self {
            args,
            ..Default::default()
        }
    }
}

/// The main Viper TypeScript runtime
pub struct Runtime {
    context: Context,
    transpiler: Transpiler,
    #[allow(dead_code)]
    config: RuntimeConfig,
    #[allow(dead_code)]
    event_loop: Option<Rc<ViperEventLoop>>,
}

impl Runtime {
    /// Create a new runtime with default configuration
    pub fn new() -> RuntimeResult<Self> {
        Self::with_config(RuntimeConfig::default())
    }

    /// Create a new runtime with custom configuration
    pub fn with_config(config: RuntimeConfig) -> RuntimeResult<Self> {
        // Create the high-performance event loop
        let event_loop = if config.use_event_loop {
            Some(Rc::new(ViperEventLoop::new()))
        } else {
            None
        };

        // Create module loader
        let module_loader = Rc::new(TypeScriptModuleLoader::new(&config.base_path));

        // Build the context with module loader
        // Note: We don't set the event loop as job_executor when using modules
        // because it causes RefCell borrow conflicts in async module loading
        let builder = ContextBuilder::default().module_loader(module_loader);

        // Only use event loop for non-module code
        // For module code, Boa handles its own job queue
        if let Some(ref el) = event_loop {
            // We'll manually use the event loop instead of setting it as executor
            let _ = el;
        }

        let mut context = builder
            .build()
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Increase runtime limits to match Node.js/V8 defaults
        // This handles large module graphs (e.g., date-fns has 245 re-exports)
        context.runtime_limits_mut().set_recursion_limit(16384);
        context
            .runtime_limits_mut()
            .set_stack_size_limit(1024 * 1024); // 1MB

        // Register all boa_runtime extensions using tuple syntax
        // This gives us: console, setTimeout/setInterval, URL, TextEncoder/TextDecoder,
        // structuredClone, queueMicrotask, and fetch
        register_extensions(
            (
                ConsoleExtension(ViperLogger),
                TimeoutExtension,
                UrlExtension,
                EncodingExtension,
                StructuredCloneExtension,
                MicrotaskExtension,
                FetchExtension(BlockingReqwestFetcher::default()),
            ),
            None,
            &mut context,
        )
        .map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Add global 'global' object (like Node.js)
        let global = context.global_object();
        context
            .global_object()
            .set(js_string!("global"), global, false, &mut context)
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Add Viper-specific globals
        Self::register_viper_globals(&mut context)?;

        // Register high-performance file system API
        fs::simple::register_file_system(&mut context)
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Register Viper.serve() API
        server_api::register_server_api(&mut context)
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Register process object (with command-line args)
        process::register_process(&mut context, &config.args)
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Register crypto API
        crypto::register_crypto(&mut context).map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Register spawn/exec APIs
        spawn::register_spawn(&mut context).map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Register WebSocket API (client and server)
        websocket::register_websocket(&mut context)
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;
        websocket::register_websocket_helpers(&mut context)
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Register path module (Node.js compatible)
        path::register_path(&mut context).map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Register Worker API (high-performance Web Workers)
        worker::register_worker_api(&mut context)
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;

        let transpiler = Transpiler::with_config(config.transpiler_config.clone());

        Ok(Self {
            context,
            transpiler,
            config,
            event_loop,
        })
    }

    /// Register Viper-specific global functions and objects
    fn register_viper_globals(context: &mut Context) -> RuntimeResult<()> {
        // Add version info
        context
            .global_object()
            .set(
                js_string!("__VIPER_VERSION__"),
                JsValue::from(js_string!(env!("CARGO_PKG_VERSION"))),
                false,
                context,
            )
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Add runtime name
        context
            .global_object()
            .set(
                js_string!("__VIPER_RUNTIME__"),
                JsValue::from(js_string!("Viper")),
                false,
                context,
            )
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Add JSX runtime
        Self::register_jsx_runtime(context)?;

        Ok(())
    }

    /// Register JSX runtime functions for classic JSX mode
    fn register_jsx_runtime(context: &mut Context) -> RuntimeResult<()> {
        // Simple JSX runtime that creates plain objects representing elements
        let jsx_runtime = r#"
            // JSX element creation function
            globalThis.__viper_jsx = function(type, props, ...children) {
                // Handle null/undefined props
                if (props === null || props === undefined) {
                    props = {};
                }

                // Flatten children array (handle nested arrays)
                const flatChildren = [];
                function flatten(arr) {
                    for (const item of arr) {
                        if (Array.isArray(item)) {
                            flatten(item);
                        } else if (item !== null && item !== undefined && item !== false) {
                            flatChildren.push(item);
                        }
                    }
                }
                flatten(children);

                // If children exist, add them to props
                if (flatChildren.length > 0) {
                    props = { ...props, children: flatChildren.length === 1 ? flatChildren[0] : flatChildren };
                }

                // If type is a function (component), call it
                if (typeof type === 'function') {
                    return type(props);
                }

                // Otherwise, create a plain object representing the element
                return {
                    type: type,
                    props: props,
                    $$typeof: Symbol.for('viper.element')
                };
            };

            // JSX fragment function
            globalThis.__viper_fragment = function(props, ...children) {
                return __viper_jsx(Symbol.for('viper.fragment'), props, ...children);
            };

            // Simple HTML renderer for JSX elements
            globalThis.renderToString = function(element) {
                if (element === null || element === undefined) {
                    return '';
                }

                // Handle text nodes
                if (typeof element === 'string' || typeof element === 'number') {
                    return String(element);
                }

                // Handle arrays
                if (Array.isArray(element)) {
                    return element.map(renderToString).join('');
                }

                // Handle JSX elements
                if (element.$$typeof === Symbol.for('viper.element')) {
                    const { type, props } = element;

                    // Handle fragments
                    if (type === Symbol.for('viper.fragment')) {
                        return renderToString(props.children);
                    }

                    // Void elements that don't need closing tags
                    const voidElements = ['area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input', 'link', 'meta', 'param', 'source', 'track', 'wbr'];

                    // Build opening tag
                    let html = '<' + type;

                    // Add attributes
                    for (const [key, value] of Object.entries(props)) {
                        if (key === 'children') continue;

                        // Handle className -> class
                        const attrName = key === 'className' ? 'class' : key;

                        // Skip functions and undefined/null
                        if (typeof value === 'function' || value === undefined || value === null) {
                            continue;
                        }

                        // Boolean attributes
                        if (typeof value === 'boolean') {
                            if (value) {
                                html += ' ' + attrName;
                            }
                        } else {
                            html += ' ' + attrName + '="' + String(value) + '"';
                        }
                    }

                    // Handle void elements
                    if (voidElements.includes(type)) {
                        html += ' />';
                        return html;
                    }

                    html += '>';

                    // Add children
                    if (props.children) {
                        html += renderToString(props.children);
                    }

                    // Closing tag
                    html += '</' + type + '>';

                    return html;
                }

                return '';
            };
        "#;

        let source = Source::from_bytes(jsx_runtime.as_bytes());
        context
            .eval(source)
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;

        Ok(())
    }

    /// Evaluate TypeScript code and return the result
    pub fn eval(&mut self, code: &str, filename: &str) -> RuntimeResult<JsValue> {
        // Determine if this is TypeScript based on filename
        let is_typescript = filename.ends_with(".ts") || filename.ends_with(".tsx");

        let js_code = if is_typescript {
            self.transpiler.transpile(code, filename)?
        } else {
            code.to_string()
        };

        // Evaluate the JavaScript code
        let source = Source::from_bytes(js_code.as_bytes());
        let result = self.context.eval(source);

        // Run any pending jobs using the event loop
        let _ = self.context.run_jobs();

        result.map_err(|e| RuntimeError::JsError(e.to_string()))
    }

    /// Execute a TypeScript file
    #[allow(dead_code)]
    pub fn execute_file(&mut self, path: &Path) -> RuntimeResult<JsValue> {
        let source = std::fs::read_to_string(path)?;
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("input.ts");

        self.eval(&source, filename)
    }

    /// Check if code contains ES module syntax (import/export statements or top-level await)
    fn has_module_syntax(code: &str) -> bool {
        let mut in_multiline_comment = false;
        let mut brace_depth = 0; // Track brace nesting for top-level detection

        for line in code.lines() {
            let trimmed = line.trim();

            // Handle multiline comments
            if trimmed.contains("/*") {
                in_multiline_comment = true;
            }
            if trimmed.contains("*/") {
                in_multiline_comment = false;
                continue;
            }
            if in_multiline_comment {
                continue;
            }

            // Skip single-line comments
            if trimmed.starts_with("//") {
                continue;
            }

            // Check for import or export at start of line
            if trimmed.starts_with("import ")
                || trimmed.starts_with("export ")
                || trimmed.starts_with("import{")
                || trimmed.starts_with("export{")
            {
                return true;
            }

            // Track brace depth (simple heuristic for top-level detection)
            // Function declarations/expressions increase depth
            if trimmed.contains("function ") || trimmed.contains("=>") {
                brace_depth += trimmed.matches('{').count() as i32;
            } else {
                brace_depth += trimmed.matches('{').count() as i32;
            }

            // Check for top-level await (await at depth 0)
            if brace_depth == 0 && trimmed.contains("await ") {
                // Make sure it's not in a comment
                if let Some(pos) = trimmed.find("await ") {
                    let before = &trimmed[..pos];
                    if !before.contains("//") {
                        return true;
                    }
                }
            }

            brace_depth -= trimmed.matches('}').count() as i32;
            if brace_depth < 0 {
                brace_depth = 0;
            }
        }
        false
    }

    /// Run a TypeScript file with full event loop support
    /// This will keep running until all timers and async operations complete
    pub fn run(&mut self, code: &str, filename: &str) -> RuntimeResult<JsValue> {
        // Auto-detect module mode based on:
        // 1. File extension (.tsx, .jsx, .mjs, .mts)
        // 2. Presence of import/export statements
        let use_module_mode = filename.ends_with(".tsx")
            || filename.ends_with(".jsx")
            || filename.ends_with(".mjs")
            || filename.ends_with(".mts")
            || Self::has_module_syntax(code);

        if use_module_mode {
            return self.execute_module(code, filename);
        }

        // Determine if this is TypeScript based on filename
        let is_typescript = filename.ends_with(".ts") || filename.ends_with(".tsx");

        let js_code = if is_typescript {
            self.transpiler.transpile(code, filename)?
        } else {
            code.to_string()
        };

        // Evaluate the JavaScript code
        let source = Source::from_bytes(js_code.as_bytes());
        let result = self.context.eval(source);

        // Run the event loop to completion, including waiting for workers
        self.run_event_loop()?;

        result.map_err(|e| RuntimeError::JsError(e.to_string()))
    }

    /// Run the event loop until all work is complete (including workers)
    fn run_event_loop(&mut self) -> RuntimeResult<()> {
        loop {
            // Run any pending jobs
            self.context
                .run_jobs()
                .map_err(|e| RuntimeError::JsError(e.to_string()))?;

            // Check if there are active workers that should keep us alive
            if !worker::has_active_workers() {
                break;
            }

            // Small sleep to avoid busy-waiting, then run jobs again
            // This allows timer callbacks and worker message polling to execute
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        Ok(())
    }

    /// Run a TypeScript file with full event loop support
    pub fn run_file(&mut self, path: &Path) -> RuntimeResult<JsValue> {
        let source = std::fs::read_to_string(path)?;
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("input.ts");

        self.run(&source, filename)
    }

    /// Execute TypeScript code as a module
    #[allow(dead_code)]
    pub fn execute_module(&mut self, code: &str, filename: &str) -> RuntimeResult<JsValue> {
        // Transpile if TypeScript
        let is_typescript = filename.ends_with(".ts") || filename.ends_with(".tsx");
        let js_code = if is_typescript {
            self.transpiler.transpile(code, filename)?
        } else {
            code.to_string()
        };

        // Parse as module
        let source = Source::from_bytes(js_code.as_bytes());
        let module = Module::parse(source, None, &mut self.context)
            .map_err(|e| RuntimeError::ModuleError(e.to_string()))?;

        // Load and evaluate the module
        let promise = module.load_link_evaluate(&mut self.context);

        // Run jobs to execute the module
        self.context
            .run_jobs()
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Check if the promise resolved successfully
        match promise.state() {
            PromiseState::Fulfilled(_) => {
                // Module executed, now run event loop for workers/timers
                self.run_event_loop()?;
                Ok(JsValue::undefined())
            }
            PromiseState::Rejected(err) => {
                let err_str: JsString = err
                    .to_string(&mut self.context)
                    .unwrap_or_else(|_| js_string!("Unknown error"));
                Err(RuntimeError::ModuleError(err_str.to_std_string_escaped()))
            }
            PromiseState::Pending => Err(RuntimeError::ModuleError(
                "Module evaluation is pending".to_string(),
            )),
        }
    }

    /// Get mutable reference to the underlying context
    #[allow(dead_code)]
    pub fn context_mut(&mut self) -> &mut Context {
        &mut self.context
    }

    /// Get reference to the underlying context
    #[allow(dead_code)]
    pub fn context(&self) -> &Context {
        &self.context
    }

    /// Convert a JsValue to a displayable string
    pub fn value_to_string(&mut self, value: &JsValue) -> String {
        value
            .to_string(&mut self.context)
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_else(|_| "[error converting value]".to_string())
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new().expect("Failed to create default runtime")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_creation() {
        let runtime = Runtime::new();
        assert!(runtime.is_ok());
    }

    #[test]
    fn test_basic_javascript() {
        let mut runtime = Runtime::new().unwrap();
        let result = runtime.eval("1 + 1", "test.js");
        assert!(result.is_ok());
    }

    #[test]
    fn test_typescript_execution() {
        let mut runtime = Runtime::new().unwrap();
        let ts_code = r#"
            const x: number = 10;
            const y: number = 20;
            x + y
        "#;
        let result = runtime.eval(ts_code, "test.ts");
        assert!(result.is_ok());
    }

    #[test]
    fn test_console_log() {
        let mut runtime = Runtime::new().unwrap();
        let result = runtime.eval("console.log('Hello from Viper!')", "test.js");
        assert!(result.is_ok());
    }

    #[test]
    fn test_promise() {
        let mut runtime = Runtime::new().unwrap();
        let code = r#"
            let result = 0;
            Promise.resolve(42).then(v => { result = v; });
            result
        "#;
        let _ = runtime.run(code, "test.js");
        // Promise should be resolved after run
    }

    #[test]
    fn test_url_api() {
        let mut runtime = Runtime::new().unwrap();
        let code = r#"
            const url = new URL('https://example.com/path?query=value');
            url.hostname
        "#;
        let result = runtime.eval(code, "test.js");
        assert!(result.is_ok());
    }

    #[test]
    fn test_text_encoder() {
        let mut runtime = Runtime::new().unwrap();
        let code = r#"
            const encoder = new TextEncoder();
            const encoded = encoder.encode('Hello');
            encoded.length
        "#;
        let result = runtime.eval(code, "test.js");
        assert!(result.is_ok());
    }
}
