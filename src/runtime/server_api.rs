//! Viper.serve() API - Ultra-fast single-threaded HTTP server
//!
//! This implementation runs everything on a single thread with direct JS callback
//! invocation for each request - no channels, no locks, maximum performance.
//!
//! Usage in TypeScript:
//! ```typescript
//! const router = new Viper.Router();
//! router.get("/", () => new Response("Home"));
//! router.get("/api/users/:id", (req) => Response.json({ id: req.params.id }));
//!
//! Viper.serve({ port: 3000, fetch: router.fetch });
//! ```

#[cfg(feature = "server")]
use crate::server::hyper_server::{self, HyperServerConfig, JsRequest, JsResponse};
use boa_engine::{
    Context, JsNativeError, JsResult, JsValue, NativeFunction, js_string,
    object::ObjectInitializer, object::builtins::JsArray,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Register the Viper namespace with serve(), Router, and file APIs
#[cfg(feature = "server")]
pub fn register_server_api(context: &mut Context) -> JsResult<()> {
    // Register Web API classes first
    register_web_apis(context)?;

    // Create the Router class
    register_router_class(context)?;

    // Register native functions BEFORE evaluating JS that uses them
    register_native_helpers(context)?;

    // Create Viper namespace with all APIs
    let viper_code = r#"
        globalThis.Viper = {
            // Router class
            Router: ViperRouter,

            // serve() function - will be replaced by native
            serve: null,

            // File APIs - use global file() and write() from fs/simple.rs
            file: globalThis.file,
            write: globalThis.write,

            readFile: async (path) => {
                return __viper_read_text(path);
            },

            readDir: async (path) => {
                return __viper_readdir(path);
            },

            mkdir: async (path, options = {}) => {
                return __viper_mkdir(path, options.recursive || false);
            },

            remove: async (path, options = {}) => {
                return __viper_remove(path, options.recursive || false);
            },

            exists: async (path) => {
                return __viper_exists(path);
            },

            stat: async (path) => {
                return __viper_stat(path);
            },

            // Environment
            env: {
                get: (key) => __viper_env_get(key),
                set: (key, value) => __viper_env_set(key, value),
                has: (key) => __viper_env_get(key) !== undefined,
                toObject: () => __viper_env_all(),
            },

            // Process info
            pid: __viper_pid || 0,
            cwd: () => __viper_cwd(),

            // Version info
            version: __VIPER_VERSION__ || "0.1.0",
        };

        // Note: ViperFile class is defined in fs/simple.rs
        // We just add Viper.file and Viper.write aliases here
    "#;

    let source = boa_engine::Source::from_bytes(viper_code.as_bytes());
    context.eval(source)?;

    // Register native serve function
    let serve_fn = NativeFunction::from_fn_ptr(serve_function);
    let viper_obj = context.global_object().get(js_string!("Viper"), context)?;
    if let Some(viper) = viper_obj.as_object() {
        viper.set(
            js_string!("serve"),
            serve_fn.to_js_function(context.realm()),
            false,
            context,
        )?;
    }

    Ok(())
}

/// Placeholder when server feature is disabled
#[cfg(not(feature = "server"))]
pub fn register_server_api(_context: &mut Context) -> JsResult<()> {
    Ok(())
}

/// Register Web API classes (Request, Response, Headers)
#[cfg(feature = "server")]
fn register_web_apis(context: &mut Context) -> JsResult<()> {
    let classes = r#"
        // Headers class
        if (typeof Headers === 'undefined') {
            globalThis.Headers = class Headers {
                constructor(init = {}) {
                    this._headers = {};
                    if (init) {
                        if (Array.isArray(init)) {
                            for (const [key, value] of init) {
                                this.set(key, value);
                            }
                        } else if (init instanceof Headers) {
                            init.forEach((value, key) => this.set(key, value));
                        } else if (typeof init === 'object') {
                            for (const [key, value] of Object.entries(init)) {
                                this.set(key, value);
                            }
                        }
                    }
                }
                get(name) { return this._headers[name.toLowerCase()] || null; }
                set(name, value) { this._headers[name.toLowerCase()] = String(value); }
                has(name) { return name.toLowerCase() in this._headers; }
                delete(name) { delete this._headers[name.toLowerCase()]; }
                append(name, value) {
                    const key = name.toLowerCase();
                    if (this._headers[key]) {
                        this._headers[key] += ', ' + String(value);
                    } else {
                        this._headers[key] = String(value);
                    }
                }
                entries() { return Object.entries(this._headers)[Symbol.iterator](); }
                keys() { return Object.keys(this._headers)[Symbol.iterator](); }
                values() { return Object.values(this._headers)[Symbol.iterator](); }
                forEach(callback) {
                    for (const [key, value] of Object.entries(this._headers)) {
                        callback(value, key, this);
                    }
                }
                [Symbol.iterator]() { return this.entries(); }
            };
        }

        // Request class
        globalThis.Request = class Request {
            constructor(input, options = {}) {
                if (input instanceof Request) {
                    this.url = input.url;
                    this.method = options.method || input.method;
                    this.headers = new Headers(options.headers || input.headers);
                    this._body = options.body !== undefined ? options.body : input._body;
                } else {
                    this.url = String(input);
                    this.method = options.method || 'GET';
                    this.headers = new Headers(options.headers || {});
                    this._body = options.body || null;
                }
                // Parse URL for params (set by router)
                this.params = options.params || {};
                this.query = this._parseQuery();
            }

            _parseQuery() {
                const query = {};
                const qIndex = this.url.indexOf('?');
                if (qIndex !== -1) {
                    const qs = this.url.slice(qIndex + 1);
                    for (const pair of qs.split('&')) {
                        const [key, value] = pair.split('=');
                        if (key) query[decodeURIComponent(key)] = decodeURIComponent(value || '');
                    }
                }
                return query;
            }

            get body() { return this._body; }

            async text() {
                if (this._body === null) return '';
                if (typeof this._body === 'string') return this._body;
                return String(this._body);
            }

            async json() {
                const text = await this.text();
                return JSON.parse(text);
            }

            async formData() {
                const text = await this.text();
                const data = new Map();
                for (const pair of text.split('&')) {
                    const [key, value] = pair.split('=');
                    if (key) data.set(decodeURIComponent(key), decodeURIComponent(value || ''));
                }
                return data;
            }

            clone() {
                return new Request(this.url, {
                    method: this.method,
                    headers: this.headers,
                    body: this._body,
                    params: { ...this.params }
                });
            }
        };

        // Response class
        globalThis.Response = class Response {
            constructor(body = null, options = {}) {
                this._body = body;
                this.status = options.status || 200;
                this.statusText = options.statusText || 'OK';
                this.headers = new Headers(options.headers || {});
                this.ok = this.status >= 200 && this.status < 300;

                // Auto-detect content type
                if (body !== null && !this.headers.has('content-type')) {
                    if (typeof body === 'object' && !(body instanceof ArrayBuffer) && !ArrayBuffer.isView(body)) {
                        this.headers.set('content-type', 'application/json');
                        this._body = JSON.stringify(body);
                    } else if (typeof body === 'string') {
                        if (body.trim().startsWith('<')) {
                            this.headers.set('content-type', 'text/html; charset=utf-8');
                        } else {
                            this.headers.set('content-type', 'text/plain; charset=utf-8');
                        }
                    }
                }
            }

            get body() { return this._body; }

            async text() {
                if (this._body === null) return '';
                if (typeof this._body === 'string') return this._body;
                return String(this._body);
            }

            async json() {
                const text = await this.text();
                return JSON.parse(text);
            }

            clone() {
                return new Response(this._body, {
                    status: this.status,
                    statusText: this.statusText,
                    headers: new Headers(this.headers)
                });
            }

            static json(data, options = {}) {
                const headers = new Headers(options.headers || {});
                headers.set('content-type', 'application/json');
                return new Response(JSON.stringify(data), { ...options, headers });
            }

            static redirect(url, status = 302) {
                const headers = new Headers();
                headers.set('location', url);
                return new Response(null, { status, headers });
            }

            static error() {
                return new Response(null, { status: 500, statusText: 'Internal Server Error' });
            }
        };
    "#;

    let source = boa_engine::Source::from_bytes(classes.as_bytes());
    context.eval(source)?;
    Ok(())
}

/// Register the Router class
#[cfg(feature = "server")]
fn register_router_class(context: &mut Context) -> JsResult<()> {
    let router_code = r#"
        class ViperRouter {
            constructor() {
                this.routes = [];
                this.middleware = [];

                // Bind fetch to this instance
                this.fetch = this.handle.bind(this);
            }

            // Add middleware
            use(handler) {
                this.middleware.push(handler);
                return this;
            }

            // Route registration methods
            get(path, handler) { return this._addRoute('GET', path, handler); }
            post(path, handler) { return this._addRoute('POST', path, handler); }
            put(path, handler) { return this._addRoute('PUT', path, handler); }
            delete(path, handler) { return this._addRoute('DELETE', path, handler); }
            patch(path, handler) { return this._addRoute('PATCH', path, handler); }
            head(path, handler) { return this._addRoute('HEAD', path, handler); }
            options(path, handler) { return this._addRoute('OPTIONS', path, handler); }
            all(path, handler) { return this._addRoute('*', path, handler); }

            // Add a route
            _addRoute(method, path, handler) {
                const pattern = this._compilePattern(path);
                this.routes.push({ method, path, pattern, handler });
                return this;
            }

            // Compile path pattern to regex
            _compilePattern(path) {
                const paramNames = [];
                let regexStr = path;

                // Handle wildcard first (before escaping)
                let hasWildcard = false;
                if (regexStr.endsWith('*')) {
                    hasWildcard = true;
                    regexStr = regexStr.slice(0, -1); // Remove the *
                    paramNames.push('*');
                }

                // Replace :param with capture group
                regexStr = regexStr.replace(/:([a-zA-Z_][a-zA-Z0-9_]*)/g, (_, name) => {
                    paramNames.splice(paramNames.length - (hasWildcard ? 1 : 0), 0, name);
                    return '([^/]+)';
                });

                // Escape forward slashes
                regexStr = regexStr.replace(/\//g, '\\/');

                // Add wildcard capture at the end
                if (hasWildcard) {
                    regexStr += '(.*)';
                }

                return {
                    regex: new RegExp('^' + regexStr + '$'),
                    paramNames
                };
            }

            // Match a path against pattern
            _matchRoute(method, path) {
                // Remove query string for matching
                const pathWithoutQuery = path.split('?')[0];

                for (const route of this.routes) {
                    if (route.method !== '*' && route.method !== method) continue;

                    const match = pathWithoutQuery.match(route.pattern.regex);
                    if (match) {
                        const params = {};
                        route.pattern.paramNames.forEach((name, i) => {
                            params[name] = decodeURIComponent(match[i + 1]);
                        });
                        return { route, params };
                    }
                }
                return null;
            }

            // Handle incoming request
            handle(request) {
                try {
                    const method = request.method;
                    const url = request.url;

                    // Run middleware
                    for (const mw of this.middleware) {
                        const result = mw(request);
                        if (result instanceof Response) {
                            return result;
                        }
                    }

                    // Find matching route
                    const match = this._matchRoute(method, url);

                    if (match) {
                        // Add params to request
                        request.params = match.params;
                        return match.route.handler(request);
                    }

                    // 404 Not Found
                    return new Response('Not Found', { status: 404 });
                } catch (error) {
                    console.error('Router error:', error);
                    return new Response('Internal Server Error: ' + error.message, { status: 500 });
                }
            }

            // Group routes with a prefix
            group(prefix, callback) {
                const subRouter = new ViperRouter();
                callback(subRouter);

                // Add sub-router routes with prefix
                for (const route of subRouter.routes) {
                    this._addRoute(route.method, prefix + route.path, route.handler);
                }

                return this;
            }
        }

        globalThis.ViperRouter = ViperRouter;
    "#;

    let source = boa_engine::Source::from_bytes(router_code.as_bytes());
    context.eval(source)?;
    Ok(())
}

/// Register native helper functions
#[cfg(feature = "server")]
fn register_native_helpers(context: &mut Context) -> JsResult<()> {
    // __viper_env_get
    let env_get = NativeFunction::from_fn_ptr(|_this, args, context| {
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
    context.global_object().set(
        js_string!("__viper_env_get"),
        env_get.to_js_function(context.realm()),
        false,
        context,
    )?;

    // __viper_env_set
    let env_set = NativeFunction::from_fn_ptr(|_this, args, context| {
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

        // SAFETY: We're single-threaded and this is the only place we modify env vars
        unsafe {
            std::env::set_var(&key, &value);
        }
        Ok(JsValue::undefined())
    });
    context.global_object().set(
        js_string!("__viper_env_set"),
        env_set.to_js_function(context.realm()),
        false,
        context,
    )?;

    // __viper_env_all
    let env_all = NativeFunction::from_fn_ptr(|_this, _args, context| {
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
    context.global_object().set(
        js_string!("__viper_env_all"),
        env_all.to_js_function(context.realm()),
        false,
        context,
    )?;

    // __viper_cwd
    let cwd = NativeFunction::from_fn_ptr(|_this, _args, _context| match std::env::current_dir() {
        Ok(path) => Ok(JsValue::from(js_string!(
            path.to_string_lossy().to_string()
        ))),
        Err(_) => Ok(JsValue::from(js_string!("."))),
    });
    context.global_object().set(
        js_string!("__viper_cwd"),
        cwd.to_js_function(context.realm()),
        false,
        context,
    )?;

    // __viper_pid
    context.global_object().set(
        js_string!("__viper_pid"),
        JsValue::from(std::process::id() as i32),
        false,
        context,
    )?;

    // __viper_mkdir
    let mkdir = NativeFunction::from_fn_ptr(|_this, args, context| {
        let path = args
            .get(0)
            .map(|v| v.to_string(context))
            .transpose()?
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default();
        let recursive = args.get(1).map(|v| v.to_boolean()).unwrap_or(false);

        let result = if recursive {
            std::fs::create_dir_all(&path)
        } else {
            std::fs::create_dir(&path)
        };

        match result {
            Ok(_) => Ok(JsValue::undefined()),
            Err(e) => Err(JsNativeError::error()
                .with_message(format!("Failed to create directory: {}", e))
                .into()),
        }
    });
    context.global_object().set(
        js_string!("__viper_mkdir"),
        mkdir.to_js_function(context.realm()),
        false,
        context,
    )?;

    // __viper_remove
    let remove = NativeFunction::from_fn_ptr(|_this, args, context| {
        let path = args
            .get(0)
            .map(|v| v.to_string(context))
            .transpose()?
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default();
        let recursive = args.get(1).map(|v| v.to_boolean()).unwrap_or(false);

        let metadata = std::fs::metadata(&path);
        let result = match metadata {
            Ok(m) if m.is_dir() => {
                if recursive {
                    std::fs::remove_dir_all(&path)
                } else {
                    std::fs::remove_dir(&path)
                }
            }
            Ok(_) => std::fs::remove_file(&path),
            Err(e) => Err(e),
        };

        match result {
            Ok(_) => Ok(JsValue::undefined()),
            Err(e) => Err(JsNativeError::error()
                .with_message(format!("Failed to remove: {}", e))
                .into()),
        }
    });
    context.global_object().set(
        js_string!("__viper_remove"),
        remove.to_js_function(context.realm()),
        false,
        context,
    )?;

    // __viper_readdir
    let readdir = NativeFunction::from_fn_ptr(|_this, args, context| {
        let path = args
            .get(0)
            .map(|v| v.to_string(context))
            .transpose()?
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_else(|| ".".to_string());

        match std::fs::read_dir(&path) {
            Ok(entries) => {
                let arr = JsArray::new(context);
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    arr.push(JsValue::from(js_string!(name)), context)?;
                }
                Ok(arr.into())
            }
            Err(e) => Err(JsNativeError::error()
                .with_message(format!("Failed to read directory: {}", e))
                .into()),
        }
    });
    context.global_object().set(
        js_string!("__viper_readdir"),
        readdir.to_js_function(context.realm()),
        false,
        context,
    )?;

    // __viper_stat
    let stat = NativeFunction::from_fn_ptr(|_this, args, context| {
        let path = args
            .get(0)
            .map(|v| v.to_string(context))
            .transpose()?
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default();

        match std::fs::metadata(&path) {
            Ok(m) => {
                let obj = ObjectInitializer::new(context)
                    .property(
                        js_string!("size"),
                        JsValue::from(m.len() as f64),
                        Default::default(),
                    )
                    .property(
                        js_string!("isFile"),
                        JsValue::from(m.is_file()),
                        Default::default(),
                    )
                    .property(
                        js_string!("isDirectory"),
                        JsValue::from(m.is_dir()),
                        Default::default(),
                    )
                    .property(
                        js_string!("isSymlink"),
                        JsValue::from(m.is_symlink()),
                        Default::default(),
                    )
                    .build();
                Ok(JsValue::from(obj))
            }
            Err(e) => Err(JsNativeError::error()
                .with_message(format!("Failed to stat: {}", e))
                .into()),
        }
    });
    context.global_object().set(
        js_string!("__viper_stat"),
        stat.to_js_function(context.realm()),
        false,
        context,
    )?;

    Ok(())
}

/// Viper.serve() - starts an ultra-fast HTTP server
#[cfg(feature = "server")]
fn serve_function(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let options = args.get(0).ok_or_else(|| {
        JsNativeError::typ().with_message("Viper.serve() requires an options object")
    })?;

    if !options.is_object() {
        return Err(JsNativeError::typ()
            .with_message("Viper.serve() options must be an object")
            .into());
    }

    let options_obj = options.as_object().unwrap();

    // Extract config
    let port = options_obj
        .get(js_string!("port"), context)?
        .to_u32(context)
        .unwrap_or(3000) as u16;

    let hostname_val = options_obj.get(js_string!("hostname"), context)?;
    let hostname = if hostname_val.is_undefined() || hostname_val.is_null() {
        "127.0.0.1".to_string()
    } else {
        hostname_val
            .to_string(context)
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_else(|_| "127.0.0.1".to_string())
    };

    // Get the fetch handler (can be a function or router.fetch)
    let fetch_handler = options_obj.get(js_string!("fetch"), context)?;
    if !fetch_handler.is_callable() {
        return Err(JsNativeError::typ()
            .with_message("Viper.serve() requires a 'fetch' handler function")
            .into());
    }

    let fetch_fn = fetch_handler.as_object().unwrap().clone();

    // Store fetch function globally
    context.global_object().set(
        js_string!("__viper_fetch_handler"),
        fetch_fn.clone(),
        false,
        context,
    )?;

    // Create server config
    let config = HyperServerConfig {
        hostname: hostname.clone(),
        port,
        max_body_size: 10 * 1024 * 1024,
    };

    // Create the request handler
    let ctx_ptr = context as *mut Context;

    let handler: hyper_server::RequestHandler = Rc::new(RefCell::new(move |req: JsRequest| {
        let ctx = unsafe { &mut *ctx_ptr };

        match call_js_handler(ctx, &req) {
            Ok(response) => response,
            Err(e) => {
                eprintln!("Handler error: {}", e);
                JsResponse::internal_error(format!("Handler error: {}", e))
            }
        }
    }));

    // Run the server (blocks forever)
    if let Err(e) = hyper_server::run_server(config, handler) {
        return Err(JsNativeError::error()
            .with_message(format!("Server error: {}", e))
            .into());
    }

    Ok(JsValue::undefined())
}

/// Call the JavaScript fetch handler
#[cfg(feature = "server")]
fn call_js_handler(context: &mut Context, req: &JsRequest) -> Result<JsResponse, String> {
    use boa_engine::builtins::promise::PromiseState;
    use boa_engine::object::builtins::JsPromise;

    let fetch_fn = context
        .global_object()
        .get(js_string!("__viper_fetch_handler"), context)
        .map_err(|e| e.to_string())?;

    let fetch_obj = fetch_fn
        .as_object()
        .ok_or_else(|| "Fetch handler not found".to_string())?;

    let request_obj = create_js_request(context, req)?;

    let result = fetch_obj
        .call(&JsValue::undefined(), &[request_obj], context)
        .map_err(|e| e.to_string())?;

    // Check if result is a Promise by trying to convert it
    if let Some(promise_obj) = result.as_object() {
        // Try to convert to JsPromise - if successful, it's a promise
        if let Ok(promise) = JsPromise::from_object(promise_obj.clone()) {
            // Check if promise is already fulfilled (fast path - most common case)
            if let PromiseState::Fulfilled(ref value) = promise.state() {
                return extract_js_response(value, context);
            }

            // Promise is pending - run the event loop until it settles
            // This follows the same pattern as deno_core's approach:
            // run jobs until the promise resolves or we exhaust pending jobs
            let max_iterations = 10;
            for _ in 0..max_iterations {
                // Run pending microtasks/jobs
                if context.run_jobs().is_err() {
                    break; // No more jobs to run
                }

                // Check state after running jobs
                match promise.state() {
                    PromiseState::Fulfilled(ref value) => {
                        return extract_js_response(value, context);
                    }
                    PromiseState::Rejected(ref err) => {
                        let err_str = err
                            .to_string(context)
                            .map(|s| s.to_std_string_escaped())
                            .unwrap_or_else(|_| "Unknown error".to_string());
                        return Err(format!("Promise rejected: {}", err_str));
                    }
                    PromiseState::Pending => {
                        // Continue loop
                    }
                }
            }

            // Promise still pending after exhausting jobs
            return Err("Promise did not settle - handler must call res.end()".to_string());
        }
    }

    // Not a promise, extract directly
    extract_js_response(&result, context)
}

/// Create a JS Request object
#[cfg(feature = "server")]
fn create_js_request(context: &mut Context, req: &JsRequest) -> Result<JsValue, String> {
    let request_ctor = context
        .global_object()
        .get(js_string!("Request"), context)
        .map_err(|e| e.to_string())?;

    let ctor_obj = request_ctor
        .as_object()
        .ok_or_else(|| "Request constructor not found".to_string())?;

    let options = ObjectInitializer::new(context)
        .property(
            js_string!("method"),
            JsValue::from(js_string!(req.method.clone())),
            Default::default(),
        )
        .build();

    if let Some(body) = &req.body {
        let body_str = String::from_utf8_lossy(body);
        options
            .set(
                js_string!("body"),
                JsValue::from(js_string!(body_str.to_string())),
                false,
                context,
            )
            .map_err(|e| e.to_string())?;
    }

    let headers_obj = ObjectInitializer::new(context).build();
    for (key, value) in &req.headers {
        headers_obj
            .set(
                js_string!(key.clone()),
                JsValue::from(js_string!(value.clone())),
                false,
                context,
            )
            .map_err(|e| e.to_string())?;
    }
    options
        .set(js_string!("headers"), headers_obj, false, context)
        .map_err(|e| e.to_string())?;

    let request = ctor_obj
        .construct(
            &[
                JsValue::from(js_string!(req.url.clone())),
                JsValue::from(options),
            ],
            Some(&ctor_obj),
            context,
        )
        .map_err(|e| e.to_string())?;

    Ok(JsValue::from(request))
}

/// Extract JsResponse from JS Response
#[cfg(feature = "server")]
fn extract_js_response(value: &JsValue, context: &mut Context) -> Result<JsResponse, String> {
    if value.is_null_or_undefined() {
        return Ok(JsResponse::default());
    }

    if let Some(s) = value.as_string() {
        return Ok(JsResponse::text(200, s.to_std_string_escaped()));
    }

    let obj = value
        .as_object()
        .ok_or_else(|| "Expected Response object".to_string())?;

    let status = obj
        .get(js_string!("status"), context)
        .map_err(|e| e.to_string())?
        .to_u32(context)
        .unwrap_or(200) as u16;

    let mut headers = std::collections::HashMap::new();
    if let Ok(headers_val) = obj.get(js_string!("headers"), context) {
        if let Some(headers_obj) = headers_val.as_object() {
            if let Ok(get_fn) = headers_obj.get(js_string!("get"), context) {
                if let Some(get_fn_obj) = get_fn.as_object() {
                    if get_fn_obj.is_callable() {
                        for header_name in
                            ["content-type", "location", "set-cookie", "cache-control"]
                        {
                            if let Ok(val) = get_fn_obj.call(
                                &headers_val,
                                &[JsValue::from(js_string!(header_name))],
                                context,
                            ) {
                                if let Some(val_str) = val.as_string() {
                                    headers.insert(
                                        header_name.to_string(),
                                        val_str.to_std_string_escaped(),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let body = if let Ok(body_val) = obj.get(js_string!("_body"), context) {
        if let Some(s) = body_val.as_string() {
            bytes::Bytes::from(s.to_std_string_escaped())
        } else if !body_val.is_null_or_undefined() {
            let s = body_val
                .to_string(context)
                .map_err(|e| e.to_string())?
                .to_std_string_escaped();
            bytes::Bytes::from(s)
        } else {
            bytes::Bytes::new()
        }
    } else {
        bytes::Bytes::new()
    };

    Ok(JsResponse {
        status,
        headers,
        body,
    })
}
