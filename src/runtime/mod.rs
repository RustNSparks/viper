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
    sync::atomic::{AtomicU32, Ordering},
};
use thiserror::Error;

/// Global counter for pending timers (setTimeout/setInterval)
/// This allows the event loop to know when to keep running
static PENDING_TIMER_COUNT: AtomicU32 = AtomicU32::new(0);

/// Increment pending timer count
pub fn increment_pending_timers() {
    PENDING_TIMER_COUNT.fetch_add(1, Ordering::SeqCst);
}

/// Decrement pending timer count
pub fn decrement_pending_timers() {
    PENDING_TIMER_COUNT.fetch_sub(1, Ordering::SeqCst);
}

/// Check if there are pending timers
pub fn has_pending_timers() -> bool {
    PENDING_TIMER_COUNT.load(Ordering::SeqCst) > 0
}

/// Reset pending timer count (for new runtime instances)
fn reset_pending_timers() {
    PENDING_TIMER_COUNT.store(0, Ordering::SeqCst);
}

mod assert;
mod buffer;
mod crypto;
mod event_loop;
mod events;
mod http;
mod net;
mod os;
mod path;
mod process;
mod querystring;
mod server_api;
mod spawn;
mod stream;
mod string_decoder;
mod tty;
mod url;
mod util;
mod websocket;
pub mod worker;
mod zlib;

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
            "http" | "node:http" => Some(
                r#"
                const h = globalThis.http;
                export default h;
                export const {
                    Server, Agent, ClientRequest, IncomingMessage, ServerResponse,
                    OutgoingMessage, METHODS, STATUS_CODES, createServer, request,
                    get, globalAgent, maxHeaderSize
                } = h;
                "#
                .to_string(),
            ),
            "events" | "node:events" => Some(
                r#"
                const e = globalThis.events;
                export default e.EventEmitter;
                export const EventEmitter = e.EventEmitter;
                export const once = e.once;
                export const on = e.on;
                export const getEventListeners = e.getEventListeners;
                export const getMaxListeners = e.getMaxListeners;
                export const setMaxListeners = e.setMaxListeners;
                export const listenerCount = e.listenerCount;
                export const addAbortListener = e.addAbortListener;
                export const errorMonitor = e.errorMonitor;
                export const captureRejectionSymbol = e.captureRejectionSymbol;
                export const captureRejections = e.captureRejections;
                export const defaultMaxListeners = e.defaultMaxListeners;
                "#
                .to_string(),
            ),
            "buffer" | "node:buffer" => Some(
                r#"
                const b = globalThis.buffer;
                export default b;
                export const Buffer = globalThis.Buffer;
                export const constants = b.constants;
                export const kMaxLength = b.kMaxLength;
                export const INSPECT_MAX_BYTES = b.INSPECT_MAX_BYTES;
                export const SlowBuffer = globalThis.Buffer;
                export const Blob = globalThis.Blob;
                export const File = globalThis.File;
                export const atob = globalThis.atob;
                export const btoa = globalThis.btoa;
                export const transcode = function(source, fromEnc, toEnc) {
                    return Buffer.from(source.toString(fromEnc), toEnc);
                };
                export const isUtf8 = function(input) {
                    try {
                        const str = input.toString('utf8');
                        return Buffer.from(str, 'utf8').equals(input);
                    } catch { return false; }
                };
                export const isAscii = function(input) {
                    for (let i = 0; i < input.length; i++) {
                        if (input[i] > 127) return false;
                    }
                    return true;
                };
                "#
                .to_string(),
            ),
            "stream" | "node:stream" => Some(
                r#"
                const s = globalThis.stream;
                export default s;
                export const Stream = s.Stream;
                export const Readable = s.Readable;
                export const Writable = s.Writable;
                export const Duplex = s.Duplex;
                export const Transform = s.Transform;
                export const PassThrough = s.PassThrough;
                export const pipeline = s.pipeline;
                export const finished = s.finished;
                export const addAbortSignal = s.addAbortSignal;
                export const promises = s.promises;
                "#
                .to_string(),
            ),
            "fs" | "node:fs" => Some(
                r#"
                const f = globalThis.fs;
                export default f;
                export const readFileSync = f.readFileSync;
                export const writeFileSync = f.writeFileSync;
                export const appendFileSync = f.appendFileSync;
                export const existsSync = f.existsSync;
                export const statSync = f.statSync;
                export const lstatSync = f.lstatSync;
                export const readdirSync = f.readdirSync;
                export const mkdirSync = f.mkdirSync;
                export const rmdirSync = f.rmdirSync;
                export const rmSync = f.rmSync;
                export const unlinkSync = f.unlinkSync;
                export const renameSync = f.renameSync;
                export const copyFileSync = f.copyFileSync;
                export const chmodSync = f.chmodSync;
                export const realpathSync = f.realpathSync;
                export const accessSync = f.accessSync;
                export const truncateSync = f.truncateSync;
                export const openSync = f.openSync;
                export const closeSync = f.closeSync;
                export const readSync = f.readSync;
                export const writeSync = f.writeSync;
                export const readFile = f.readFile;
                export const writeFile = f.writeFile;
                export const appendFile = f.appendFile;
                export const exists = f.exists;
                export const stat = f.stat;
                export const lstat = f.lstat;
                export const readdir = f.readdir;
                export const mkdir = f.mkdir;
                export const rmdir = f.rmdir;
                export const rm = f.rm;
                export const unlink = f.unlink;
                export const rename = f.rename;
                export const copyFile = f.copyFile;
                export const chmod = f.chmod;
                export const realpath = f.realpath;
                export const access = f.access;
                export const truncate = f.truncate;
                export const promises = f.promises;
                export const constants = f.constants;
                export const Dirent = f.Dirent;
                export const Stats = f.Stats;
                "#
                .to_string(),
            ),
            "fs/promises" | "node:fs/promises" => Some(
                r#"
                const p = globalThis.fs.promises;
                export default p;
                export const readFile = p.readFile;
                export const writeFile = p.writeFile;
                export const appendFile = p.appendFile;
                export const stat = p.stat;
                export const lstat = p.lstat;
                export const readdir = p.readdir;
                export const mkdir = p.mkdir;
                export const rmdir = p.rmdir;
                export const rm = p.rm;
                export const unlink = p.unlink;
                export const rename = p.rename;
                export const copyFile = p.copyFile;
                export const chmod = p.chmod;
                export const realpath = p.realpath;
                export const access = p.access;
                export const truncate = p.truncate;
                "#
                .to_string(),
            ),
            "util" | "node:util" => Some(
                r#"
                const u = globalThis.util;
                export default u;
                export const promisify = u.promisify;
                export const callbackify = u.callbackify;
                export const format = u.format;
                export const formatWithOptions = u.formatWithOptions;
                export const inspect = u.inspect;
                export const deprecate = u.deprecate;
                export const isDeepStrictEqual = u.isDeepStrictEqual;
                export const inherits = u.inherits;
                export const debuglog = u.debuglog;
                export const getSystemErrorName = u.getSystemErrorName;
                export const getSystemErrorMap = u.getSystemErrorMap;
                export const types = u.types;
                "#
                .to_string(),
            ),
            "net" | "node:net" => Some(
                r#"
                const n = globalThis.net;
                export default n;
                export const Socket = n.Socket;
                export const Server = n.Server;
                export const BlockList = n.BlockList;
                export const SocketAddress = n.SocketAddress;
                export const createServer = n.createServer;
                export const createConnection = n.createConnection;
                export const connect = n.connect;
                export const isIP = n.isIP;
                export const isIPv4 = n.isIPv4;
                export const isIPv6 = n.isIPv6;
                "#
                .to_string(),
            ),
            "tty" | "node:tty" => Some(
                r#"
                const t = globalThis.tty;
                export default t;
                export const isatty = t.isatty;
                export const ReadStream = t.ReadStream;
                export const WriteStream = t.WriteStream;
                "#
                .to_string(),
            ),
            "url" | "node:url" => Some(
                r#"
                const u = globalThis.url;
                export default u;
                export const URL = globalThis.URL;
                export const URLSearchParams = globalThis.URLSearchParams;
                export const parse = u.parse;
                export const format = u.format;
                export const resolve = u.resolve;
                export const domainToASCII = u.domainToASCII;
                export const domainToUnicode = u.domainToUnicode;
                export const fileURLToPath = u.fileURLToPath;
                export const pathToFileURL = u.pathToFileURL;
                export const urlToHttpOptions = u.urlToHttpOptions;
                export const Url = u.Url;
                "#
                .to_string(),
            ),
            "querystring" | "node:querystring" => Some(
                r#"
                const qs = globalThis.querystring;
                export default qs;
                export const parse = qs.parse;
                export const stringify = qs.stringify;
                export const escape = qs.escape;
                export const unescape = qs.unescape;
                export const decode = qs.decode;
                export const encode = qs.encode;
                "#
                .to_string(),
            ),
            "string_decoder" | "node:string_decoder" => Some(
                r#"
                const sd = globalThis.string_decoder;
                export default sd;
                export const StringDecoder = sd.StringDecoder;
                "#
                .to_string(),
            ),
            "assert" | "node:assert" => Some(
                r#"
                const a = globalThis.assert;
                export default a;
                export const AssertionError = a.AssertionError;
                export const ok = a.ok;
                export const equal = a.equal;
                export const notEqual = a.notEqual;
                export const strictEqual = a.strictEqual;
                export const notStrictEqual = a.notStrictEqual;
                export const deepEqual = a.deepEqual;
                export const notDeepEqual = a.notDeepEqual;
                export const deepStrictEqual = a.deepStrictEqual;
                export const notDeepStrictEqual = a.notDeepStrictEqual;
                export const fail = a.fail;
                export const throws = a.throws;
                export const doesNotThrow = a.doesNotThrow;
                export const rejects = a.rejects;
                export const doesNotReject = a.doesNotReject;
                export const match = a.match;
                export const doesNotMatch = a.doesNotMatch;
                export const ifError = a.ifError;
                export const strict = a.strict;
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
            // Handle JSON files specially - they don't need the require wrapper
            if id.ends_with(".json") {
                bundle.push_str(&format!(
                    r#"__cjs_modules__["{}"] = function(exports, module, __parent_require__, __filename) {{
module.exports = {};
}};

"#,
                    id, code
                ));
                continue;
            }

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
    // Check built-in modules first
    if (specifier === 'buffer' || specifier === 'node:buffer') {{
        // Create a callable Buffer constructor wrapper
        const _Buffer = globalThis.Buffer;
        function Buffer(arg, encodingOrOffset, length) {{
            if (!(this instanceof Buffer)) {{
                return Buffer.from(arg, encodingOrOffset, length);
            }}
            return Buffer.from(arg, encodingOrOffset, length);
        }}
        Buffer.prototype = Object.create(Uint8Array.prototype);
        Buffer.prototype.constructor = Buffer;
        Buffer.from = _Buffer.from;
        Buffer.alloc = _Buffer.alloc;
        Buffer.allocUnsafe = _Buffer.allocUnsafe || _Buffer.alloc;
        Buffer.allocUnsafeSlow = _Buffer.allocUnsafeSlow || _Buffer.alloc;
        Buffer.concat = _Buffer.concat;
        Buffer.byteLength = _Buffer.byteLength;
        Buffer.compare = _Buffer.compare;
        Buffer.isBuffer = _Buffer.isBuffer;
        Buffer.isEncoding = _Buffer.isEncoding;
        Buffer.poolSize = 8192;
        return {{ Buffer: Buffer, constants: globalThis.buffer?.constants || {{}}, kMaxLength: 2147483647, SlowBuffer: Buffer }};
    }}
    if (specifier === 'path' || specifier === 'node:path') {{
        return globalThis.path;
    }}
    if (specifier === 'events' || specifier === 'node:events') {{
        return globalThis.events;
    }}
    if (specifier === 'http' || specifier === 'node:http') {{
        return globalThis.http;
    }}
    if (specifier === 'fs' || specifier === 'node:fs') {{
        return globalThis.fs;
    }}
    if (specifier === 'fs/promises' || specifier === 'node:fs/promises') {{
        return globalThis.fs.promises;
    }}
    if (specifier === 'process' || specifier === 'node:process') {{
        return globalThis.process;
    }}
    if (specifier === 'util' || specifier === 'node:util') {{
        return globalThis.util;
    }}
    if (specifier === 'stream' || specifier === 'node:stream') {{
        return globalThis.stream;
    }}
    if (specifier === 'crypto' || specifier === 'node:crypto') {{
        return globalThis.crypto;
    }}
    if (specifier === 'tty' || specifier === 'node:tty') {{
        return {{
            isatty: (fd) => false,
            ReadStream: class ReadStream {{}},
            WriteStream: class WriteStream {{
                constructor() {{ this.isTTY = false; this.columns = 80; this.rows = 24; }}
                getColorDepth() {{ return 1; }}
                hasColors() {{ return false; }}
            }}
        }};
    }}
    if (specifier === 'os' || specifier === 'node:os') {{
        return {{
            platform: () => globalThis.process?.platform || 'unknown',
            arch: () => globalThis.process?.arch || 'unknown',
            cpus: () => [],
            totalmem: () => 0,
            freemem: () => 0,
            homedir: () => globalThis.process?.env?.HOME || globalThis.process?.env?.USERPROFILE || '',
            tmpdir: () => globalThis.process?.env?.TMPDIR || globalThis.process?.env?.TEMP || '/tmp',
            hostname: () => 'localhost',
            type: () => 'Unknown',
            release: () => '0.0.0',
            EOL: globalThis.process?.platform === 'win32' ? '\\r\\n' : '\\n'
        }};
    }}
    if (specifier === 'zlib' || specifier === 'node:zlib') {{
        return {{
            createGzip: () => new (globalThis.stream?.Transform || class{{}})(),
            createGunzip: () => new (globalThis.stream?.Transform || class{{}})(),
            createDeflate: () => new (globalThis.stream?.Transform || class{{}})(),
            createInflate: () => new (globalThis.stream?.Transform || class{{}})(),
            gzip: (buf, cb) => cb(null, buf),
            gunzip: (buf, cb) => cb(null, buf),
            deflate: (buf, cb) => cb(null, buf),
            inflate: (buf, cb) => cb(null, buf)
        }};
    }}
    if (specifier === 'string_decoder' || specifier === 'node:string_decoder') {{
        return {{
            StringDecoder: class StringDecoder {{
                constructor(encoding = 'utf8') {{ this.encoding = encoding; }}
                write(buffer) {{ return buffer.toString(this.encoding); }}
                end(buffer) {{ return buffer ? buffer.toString(this.encoding) : ''; }}
            }}
        }};
    }}
    if (specifier === 'net' || specifier === 'node:net') {{
        return globalThis.net;
    }}
    if (specifier === 'url' || specifier === 'node:url') {{
        return {{
            URL: globalThis.URL,
            URLSearchParams: globalThis.URLSearchParams,
            parse: (urlStr) => {{
                try {{
                    const u = new URL(urlStr);
                    return {{ protocol: u.protocol, hostname: u.hostname, host: u.host, port: u.port, pathname: u.pathname, search: u.search, hash: u.hash, href: u.href, path: u.pathname + u.search }};
                }} catch(e) {{ return null; }}
            }},
            format: (urlObj) => {{
                if (typeof urlObj === 'string') return urlObj;
                return urlObj.href || '';
            }},
            resolve: (from, to) => new URL(to, from).href
        }};
    }}
    if (specifier === 'querystring' || specifier === 'node:querystring') {{
        return {{
            parse: (str) => {{
                const result = {{}};
                if (!str || typeof str !== 'string') return result;
                str = str.replace(/^\\?/, '');
                const pairs = str.split('&');
                for (const pair of pairs) {{
                    if (!pair) continue;
                    const idx = pair.indexOf('=');
                    const key = idx >= 0 ? decodeURIComponent(pair.slice(0, idx)) : decodeURIComponent(pair);
                    const value = idx >= 0 ? decodeURIComponent(pair.slice(idx + 1)) : '';
                    if (result[key] !== undefined) {{
                        if (Array.isArray(result[key])) result[key].push(value);
                        else result[key] = [result[key], value];
                    }} else {{
                        result[key] = value;
                    }}
                }}
                return result;
            }},
            stringify: (obj) => {{
                const pairs = [];
                for (const key in obj) {{
                    const value = obj[key];
                    if (Array.isArray(value)) {{
                        for (const v of value) pairs.push(encodeURIComponent(key) + '=' + encodeURIComponent(v));
                    }} else {{
                        pairs.push(encodeURIComponent(key) + '=' + encodeURIComponent(value));
                    }}
                }}
                return pairs.join('&');
            }},
            escape: (str) => encodeURIComponent(str),
            unescape: (str) => decodeURIComponent(str)
        }};
    }}
    if (specifier === 'assert' || specifier === 'node:assert') {{
        const AssertionError = class extends Error {{ constructor(msg) {{ super(msg); this.name = 'AssertionError'; }} }};
        const ok = (value, message) => {{ if (!value) throw new AssertionError(message || 'Assertion failed'); }};
        ok.ok = ok;
        ok.strictEqual = (a, b, message) => {{ if (a !== b) throw new AssertionError(message || `Expected ${{a}} === ${{b}}`); }};
        ok.notStrictEqual = (a, b, message) => {{ if (a === b) throw new AssertionError(message || `Expected ${{a}} !== ${{b}}`); }};
        ok.deepStrictEqual = (a, b, message) => {{ if (JSON.stringify(a) !== JSON.stringify(b)) throw new AssertionError(message || 'Deep equality failed'); }};
        ok.throws = (fn, message) => {{ try {{ fn(); throw new AssertionError(message || 'Expected function to throw'); }} catch(e) {{}} }};
        ok.AssertionError = AssertionError;
        return ok;
    }}
    if (specifier === 'timers' || specifier === 'node:timers') {{
        return {{
            setTimeout: globalThis.setTimeout,
            clearTimeout: globalThis.clearTimeout,
            setInterval: globalThis.setInterval,
            clearInterval: globalThis.clearInterval,
            setImmediate: globalThis.setImmediate || ((fn) => setTimeout(fn, 0)),
            clearImmediate: globalThis.clearImmediate || globalThis.clearTimeout
        }};
    }}
    if (specifier === 'constants' || specifier === 'node:constants') {{
        return {{}};
    }}
    if (specifier === 'punycode' || specifier === 'node:punycode') {{
        return {{
            encode: (str) => str,
            decode: (str) => str,
            toASCII: (str) => str,
            toUnicode: (str) => str
        }};
    }}
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
            // Resolve the require path using CommonJS resolver (prefers "require" condition)
            if let Ok(resolved) = self.resolver.resolve_cjs(&req, file_path) {
                // Skip TypeScript declaration files (.d.ts) - they're not runtime code
                let resolved_str = resolved.to_string_lossy();
                if resolved_str.ends_with(".d.ts")
                    || resolved_str.ends_with(".d.mts")
                    || resolved_str.ends_with(".d.cts")
                {
                    continue;
                }

                let resolved_id = resolved_str.replace('\\', "/");
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
            let mut resolved_path = self
                .resolver
                .resolve(&specifier_str, &referrer_path)
                .map_err(|e| {
                    JsError::from_opaque(JsValue::from(js_string!(format!(
                        "Failed to resolve module '{}': {}",
                        specifier_str, e
                    ))))
                })?;

            // Skip TypeScript declaration files (.d.ts) - try to find the JS version
            let resolved_str = resolved_path.to_string_lossy();
            if resolved_str.ends_with(".d.ts")
                || resolved_str.ends_with(".d.mts")
                || resolved_str.ends_with(".d.cts")
            {
                // Try to find corresponding .js file
                let js_path = if resolved_str.ends_with(".d.ts") {
                    resolved_path.with_extension("js")
                } else if resolved_str.ends_with(".d.mts") {
                    resolved_path.with_extension("mjs")
                } else {
                    resolved_path.with_extension("cjs")
                };

                if js_path.exists() {
                    resolved_path = js_path;
                } else {
                    // Try index.js in same directory
                    let parent = resolved_path.parent().unwrap_or(&resolved_path);
                    let index_js = parent.join("index.js");
                    if index_js.exists() {
                        resolved_path = index_js;
                    }
                }
            }

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

        // Wrap setTimeout/setInterval to track pending timers
        Self::wrap_timer_functions(&mut context)?;

        // Register ultra-fast file system API (Node.js compatible)
        fs::fast::register_fs_module(&mut context)
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

        // Register WebSocket API (client only)
        websocket::register_websocket(&mut context)
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;
        websocket::register_websocket_helpers(&mut context)
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Register path module (Node.js compatible)
        path::register_path(&mut context).map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Register events module (Node.js compatible EventEmitter)
        events::register_events_module(&mut context)
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Register HTTP module (Node.js compatible)
        http::register_http_module(&mut context)
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Register Buffer module (Node.js compatible, high-performance native Rust)
        buffer::register_buffer_module(&mut context)
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Register util module (Node.js compatible utility functions)
        util::register_util_module(&mut context)
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Register stream module (Node.js compatible streams)
        stream::register_stream_module(&mut context)
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Register TTY module (Node.js compatible, native Rust performance)
        tty::register_tty_module(&mut context).map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Register net module (Node.js compatible TCP networking, native Rust performance)
        net::register_net_module(&mut context).map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Register os module (Node.js compatible, native Rust performance)
        os::register_os_module(&mut context).map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Register zlib module (Node.js compatible compression, using zlib-rs for max performance)
        zlib::register_zlib_module(&mut context)
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Register querystring module (Node.js compatible URL query string utilities)
        querystring::register_querystring_module(&mut context)
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Register url module (Node.js compatible URL utilities extending WHATWG URL API)
        url::register_url_module(&mut context).map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Register string_decoder module (Node.js compatible string decoding)
        string_decoder::register_string_decoder_module(&mut context)
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Register assert module (Node.js compatible assertions)
        assert::register_assert_module(&mut context)
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Register global require() function for CommonJS compatibility
        Self::register_require_function(&mut context)?;

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

    /// Wrap setTimeout/setInterval to track pending timers for the event loop
    fn wrap_timer_functions(context: &mut Context) -> RuntimeResult<()> {
        // Reset timer count for this runtime instance
        reset_pending_timers();

        let wrapper_code = r#"
            (function() {
                // Store original functions
                const _origSetTimeout = globalThis.setTimeout;
                const _origSetInterval = globalThis.setInterval;
                const _origClearTimeout = globalThis.clearTimeout;
                const _origClearInterval = globalThis.clearInterval;

                // Track active timers
                const activeTimers = new Set();
                const activeIntervals = new Set();

                // nextTick queue (highest priority - runs before I/O)
                const nextTickQueue = [];
                let processingNextTick = false;

                // setImmediate queue (runs after I/O, in check phase)
                const immediateQueue = [];
                let immediateId = 0;
                const immediateCallbacks = new Map();

                // Process nextTick queue
                function processNextTicks() {
                    if (processingNextTick) return;
                    processingNextTick = true;
                    while (nextTickQueue.length > 0) {
                        const { callback, args } = nextTickQueue.shift();
                        try {
                            callback.apply(null, args);
                        } catch (e) {
                            console.error('nextTick error:', e);
                        }
                    }
                    processingNextTick = false;
                }

                // process.nextTick - runs before any I/O
                if (!globalThis.process) globalThis.process = {};
                globalThis.process.nextTick = function(callback, ...args) {
                    nextTickQueue.push({ callback, args });
                    // Use queueMicrotask to process before next macrotask
                    queueMicrotask(processNextTicks);
                };

                // setImmediate - runs in check phase (after I/O)
                globalThis.setImmediate = function(callback, ...args) {
                    const id = ++immediateId;
                    immediateCallbacks.set(id, { callback, args });
                    __viper_timer_increment();
                    // Use setTimeout(0) to schedule in macrotask queue
                    _origSetTimeout(() => {
                        const entry = immediateCallbacks.get(id);
                        if (entry) {
                            immediateCallbacks.delete(id);
                            __viper_timer_decrement();
                            entry.callback.apply(null, entry.args);
                        }
                    }, 0);
                    return id;
                };

                globalThis.clearImmediate = function(id) {
                    if (immediateCallbacks.has(id)) {
                        immediateCallbacks.delete(id);
                        __viper_timer_decrement();
                    }
                };

                // Wrap setTimeout
                globalThis.setTimeout = function(callback, delay, ...args) {
                    __viper_timer_increment();
                    const id = _origSetTimeout(function() {
                        activeTimers.delete(id);
                        __viper_timer_decrement();
                        callback.apply(this, args);
                    }, delay);
                    activeTimers.add(id);
                    return id;
                };

                // Wrap setInterval
                globalThis.setInterval = function(callback, delay, ...args) {
                    __viper_timer_increment();
                    const id = _origSetInterval(function() {
                        callback.apply(this, args);
                    }, delay);
                    activeIntervals.add(id);
                    return id;
                };

                // Wrap clearTimeout
                globalThis.clearTimeout = function(id) {
                    if (activeTimers.has(id)) {
                        activeTimers.delete(id);
                        __viper_timer_decrement();
                    }
                    return _origClearTimeout(id);
                };

                // Wrap clearInterval
                globalThis.clearInterval = function(id) {
                    if (activeIntervals.has(id)) {
                        activeIntervals.delete(id);
                        __viper_timer_decrement();
                    }
                    return _origClearInterval(id);
                };
            })();
        "#;

        // Register native timer tracking functions
        let increment_fn = boa_engine::NativeFunction::from_fn_ptr(|_this, _args, _context| {
            increment_pending_timers();
            Ok(JsValue::undefined())
        });
        context
            .global_object()
            .set(
                js_string!("__viper_timer_increment"),
                increment_fn.to_js_function(context.realm()),
                false,
                context,
            )
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;

        let decrement_fn = boa_engine::NativeFunction::from_fn_ptr(|_this, _args, _context| {
            decrement_pending_timers();
            Ok(JsValue::undefined())
        });
        context
            .global_object()
            .set(
                js_string!("__viper_timer_decrement"),
                decrement_fn.to_js_function(context.realm()),
                false,
                context,
            )
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;

        // Execute wrapper code
        let source = Source::from_bytes(wrapper_code.as_bytes());
        context
            .eval(source)
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;

        Ok(())
    }

    /// Register global require() function for CommonJS compatibility
    fn register_require_function(context: &mut Context) -> RuntimeResult<()> {
        let require_code = r#"
            // V8-specific Error API polyfills (needed for depd, etc.)
            if (typeof Error.captureStackTrace !== 'function') {
                Error.captureStackTrace = function(targetObject, constructorOpt) {
                    // Create a mock stack trace array that mimics V8's CallSite objects
                    const mockCallSite = {
                        getFileName: () => 'unknown',
                        getLineNumber: () => 0,
                        getColumnNumber: () => 0,
                        getFunctionName: () => 'anonymous',
                        getTypeName: () => null,
                        getMethodName: () => null,
                        getEvalOrigin: () => null,
                        isTopLevel: () => true,
                        isEval: () => false,
                        isNative: () => false,
                        isConstructor: () => false,
                        toString: () => 'at anonymous (unknown:0:0)'
                    };

                    // Assign a stack array with mock call sites
                    targetObject.stack = [mockCallSite, mockCallSite, mockCallSite];
                };
            }

            if (typeof Error.stackTraceLimit === 'undefined') {
                Error.stackTraceLimit = 10;
            }

            if (typeof Error.prepareStackTrace === 'undefined') {
                Error.prepareStackTrace = undefined;
            }

            // Module cache
            globalThis.__moduleCache = {};

            // Create a proper Buffer constructor wrapper for Node.js compatibility
            // safer-buffer and other modules expect Buffer to be a constructor with a prototype
            const BufferConstructor = (function() {
                const _Buffer = globalThis.Buffer;

                // Create constructor function
                function Buffer(arg, encodingOrOffset, length) {
                    if (!(this instanceof Buffer)) {
                        return Buffer.from(arg, encodingOrOffset, length);
                    }
                    // When called as constructor, delegate to from
                    const buf = _Buffer.from(arg, encodingOrOffset, length);
                    Object.setPrototypeOf(buf, Buffer.prototype);
                    return buf;
                }

                // Create prototype
                Buffer.prototype = Object.create(Uint8Array.prototype);
                Buffer.prototype.constructor = Buffer;

                // Copy static methods from _Buffer
                Buffer.alloc = _Buffer.alloc;
                Buffer.allocUnsafe = _Buffer.allocUnsafe;
                Buffer.allocUnsafeSlow = _Buffer.allocUnsafeSlow;
                Buffer.from = function(value, encodingOrOffset, length) {
                    const buf = _Buffer.from(value, encodingOrOffset, length);
                    // Don't change prototype - it adds overhead
                    return buf;
                };
                Buffer.concat = _Buffer.concat;
                Buffer.byteLength = _Buffer.byteLength;
                Buffer.compare = _Buffer.compare;
                Buffer.isBuffer = _Buffer.isBuffer;
                Buffer.isEncoding = _Buffer.isEncoding;

                // Node.js Buffer constants
                Buffer.poolSize = 8192;
                Buffer.kMaxLength = 2147483647;

                return Buffer;
            })();

            // Built-in modules map
            const builtinModules = {
                'buffer': () => ({ Buffer: BufferConstructor, constants: globalThis.buffer?.constants, kMaxLength: globalThis.buffer?.kMaxLength, SlowBuffer: BufferConstructor }),
                'path': () => globalThis.path,
                'events': () => globalThis.events,
                'http': () => globalThis.http,
                'fs': () => globalThis.fs,
                'fs/promises': () => globalThis.fs?.promises,
                'process': () => globalThis.process,
                'crypto': () => globalThis.crypto,
                'util': () => globalThis.util,
                'stream': () => globalThis.stream,
                'url': () => ({
                    URL: globalThis.URL,
                    parse: (urlStr) => {
                        try {
                            const u = new URL(urlStr);
                            return { protocol: u.protocol, hostname: u.hostname, port: u.port, pathname: u.pathname, search: u.search, hash: u.hash, href: u.href };
                        } catch(e) { return null; }
                    },
                    format: (urlObj) => urlObj.href || '',
                }),
                'querystring': () => ({
                    parse: (str) => {
                        const result = {};
                        if (!str || typeof str !== 'string') return result;
                        str = str.replace(/^\?/, '');
                        const pairs = str.split('&');
                        for (const pair of pairs) {
                            if (!pair) continue;
                            const idx = pair.indexOf('=');
                            const key = idx >= 0 ? decodeURIComponent(pair.slice(0, idx)) : decodeURIComponent(pair);
                            const value = idx >= 0 ? decodeURIComponent(pair.slice(idx + 1)) : '';
                            if (result[key] !== undefined) {
                                if (Array.isArray(result[key])) result[key].push(value);
                                else result[key] = [result[key], value];
                            } else {
                                result[key] = value;
                            }
                        }
                        return result;
                    },
                    stringify: (obj) => {
                        const pairs = [];
                        for (const key in obj) {
                            const value = obj[key];
                            if (Array.isArray(value)) {
                                for (const v of value) pairs.push(encodeURIComponent(key) + '=' + encodeURIComponent(v));
                            } else {
                                pairs.push(encodeURIComponent(key) + '=' + encodeURIComponent(value));
                            }
                        }
                        return pairs.join('&');
                    },
                    encode: function(obj) { return this.stringify(obj); },
                    decode: function(str) { return this.parse(str); },
                    escape: (str) => encodeURIComponent(str),
                    unescape: (str) => decodeURIComponent(str),
                }),
                'string_decoder': () => ({
                    StringDecoder: class StringDecoder {
                        constructor(encoding = 'utf8') { this.encoding = encoding; }
                        write(buffer) { return buffer.toString(this.encoding); }
                        end(buffer) { return buffer ? buffer.toString(this.encoding) : ''; }
                    }
                }),
                'assert': () => ({
                    ok: (value, message) => { if (!value) throw new Error(message || 'Assertion failed'); },
                    strictEqual: (a, b, message) => { if (a !== b) throw new Error(message || `Expected ${a} === ${b}`); },
                    deepStrictEqual: (a, b, message) => { if (JSON.stringify(a) !== JSON.stringify(b)) throw new Error(message || 'Deep equality failed'); },
                    notStrictEqual: (a, b, message) => { if (a === b) throw new Error(message || `Expected ${a} !== ${b}`); },
                    throws: (fn, message) => { try { fn(); throw new Error(message || 'Expected function to throw'); } catch(e) {} },
                }),
                'os': () => ({
                    platform: () => globalThis.process?.platform || 'unknown',
                    arch: () => globalThis.process?.arch || 'unknown',
                    cpus: () => [],
                    totalmem: () => 0,
                    freemem: () => 0,
                    homedir: () => globalThis.process?.env?.HOME || globalThis.process?.env?.USERPROFILE || '',
                    tmpdir: () => globalThis.process?.env?.TMPDIR || globalThis.process?.env?.TEMP || '/tmp',
                    hostname: () => 'localhost',
                    type: () => 'Unknown',
                    release: () => '0.0.0',
                    uptime: () => 0,
                    loadavg: () => [0, 0, 0],
                    networkInterfaces: () => ({}),
                    EOL: globalThis.process?.platform === 'win32' ? '\r\n' : '\n',
                }),
                'timers': () => ({
                    setTimeout: globalThis.setTimeout,
                    clearTimeout: globalThis.clearTimeout,
                    setInterval: globalThis.setInterval,
                    clearInterval: globalThis.clearInterval,
                    setImmediate: globalThis.setImmediate || ((fn) => setTimeout(fn, 0)),
                    clearImmediate: globalThis.clearImmediate || clearTimeout,
                }),
                'tty': () => ({
                    isatty: (fd) => false,
                    ReadStream: class ReadStream {},
                    WriteStream: class WriteStream {
                        constructor() { this.isTTY = false; this.columns = 80; this.rows = 24; }
                        getColorDepth() { return 1; }
                        hasColors() { return false; }
                    },
                }),
                'net': () => ({
                    Socket: class Socket extends globalThis.events?.EventEmitter {
                        constructor() { super(); this.writable = true; this.readable = true; }
                        connect() { return this; }
                        write() { return true; }
                        end() {}
                        destroy() {}
                        setEncoding() {}
                        setNoDelay() {}
                        setKeepAlive() {}
                        setTimeout() {}
                    },
                    Server: class Server extends globalThis.events?.EventEmitter {
                        constructor() { super(); }
                        listen() { return this; }
                        close() {}
                        address() { return { port: 0, family: 'IPv4', address: '0.0.0.0' }; }
                    },
                    createServer: (options, connectionListener) => new (builtinModules['net']()).Server(),
                    createConnection: () => new (builtinModules['net']()).Socket(),
                    connect: () => new (builtinModules['net']()).Socket(),
                    isIP: (input) => { try { return input.includes(':') ? 6 : (input.match(/^\d+\.\d+\.\d+\.\d+$/) ? 4 : 0); } catch(e) { return 0; } },
                    isIPv4: (input) => builtinModules['net']().isIP(input) === 4,
                    isIPv6: (input) => builtinModules['net']().isIP(input) === 6,
                }),
                'zlib': () => ({
                    createGzip: () => new (globalThis.stream?.Transform || class{})(),
                    createGunzip: () => new (globalThis.stream?.Transform || class{})(),
                    createDeflate: () => new (globalThis.stream?.Transform || class{})(),
                    createInflate: () => new (globalThis.stream?.Transform || class{})(),
                    gzip: (buf, cb) => cb(null, buf),
                    gunzip: (buf, cb) => cb(null, buf),
                    deflate: (buf, cb) => cb(null, buf),
                    inflate: (buf, cb) => cb(null, buf),
                }),
            };

            // Add node: prefix versions
            for (const key of Object.keys(builtinModules)) {
                builtinModules['node:' + key] = builtinModules[key];
            }

            // Path utilities for require resolution
            const isWindows = globalThis.process?.platform === 'win32';

            function dirname(p) {
                const normalized = p.replace(/\\/g, '/');
                const lastSlash = normalized.lastIndexOf('/');
                if (lastSlash === -1) return '.';
                if (lastSlash === 0) return '/';
                // Handle Windows drive letters like C:/
                if (lastSlash === 2 && normalized[1] === ':') return normalized.slice(0, 3);
                return normalized.slice(0, lastSlash);
            }

            function join(...parts) {
                const joined = parts.map(p => p.replace(/\\/g, '/')).join('/');
                // Clean up multiple slashes but preserve Windows drive letters
                return joined.replace(/([^:])\/+/g, '$1/');
            }

            function resolve(from, to) {
                // Normalize slashes
                from = from.replace(/\\/g, '/');
                to = to.replace(/\\/g, '/');

                // If 'to' is absolute, return it
                if (to.startsWith('/')) return to;
                if (to.length >= 2 && to[1] === ':') return to; // Windows absolute path

                // Handle relative paths
                const fromParts = from.split('/').filter(p => p !== '');
                const toParts = to.split('/');

                for (const part of toParts) {
                    if (part === '..') {
                        // Don't pop the drive letter on Windows
                        if (fromParts.length > 1 || (fromParts.length === 1 && !fromParts[0].includes(':'))) {
                            fromParts.pop();
                        }
                    } else if (part !== '.' && part !== '') {
                        fromParts.push(part);
                    }
                }

                // Reconstruct path
                const result = fromParts.join('/');
                // If original from started with / and result doesn't have drive letter, add /
                if (from.startsWith('/') && !result.match(/^[A-Za-z]:/)) {
                    return '/' + result;
                }
                return result;
            }

            // Try to read a file
            function tryReadFile(filepath) {
                try {
                    return globalThis.fs.readFileSync(filepath, 'utf8');
                } catch (e) {
                    return null;
                }
            }

            // Check if file exists
            function fileExists(filepath) {
                try {
                    return globalThis.fs.existsSync(filepath);
                } catch (e) {
                    return false;
                }
            }

            // Find package.json main entry
            function getPackageMain(pkgDir) {
                const pkgJsonPath = join(pkgDir, 'package.json');
                const content = tryReadFile(pkgJsonPath);
                if (!content) return null;

                try {
                    const pkg = JSON.parse(content);
                    // Try various fields in order of priority
                    return pkg.main || pkg.module || 'index.js';
                } catch (e) {
                    return null;
                }
            }

            // Check if path is absolute (Unix or Windows)
            function isAbsolutePath(p) {
                if (p.startsWith('/')) return true;
                // Windows absolute path like C:/ or C:\
                if (p.length >= 2 && p[1] === ':') return true;
                return false;
            }

            // Resolve module path
            function resolveModule(specifier, fromDir) {
                // Relative or absolute paths
                if (specifier.startsWith('./') || specifier.startsWith('../') || isAbsolutePath(specifier)) {
                    const resolved = isAbsolutePath(specifier) ? specifier : resolve(fromDir, specifier);

                    // Try exact path
                    if (fileExists(resolved)) return resolved;

                    // Try with extensions
                    for (const ext of ['.js', '.mjs', '.cjs', '.json', '.ts']) {
                        if (fileExists(resolved + ext)) return resolved + ext;
                    }

                    // Try as directory with index
                    for (const ext of ['.js', '.mjs', '.cjs', '.ts']) {
                        if (fileExists(join(resolved, 'index' + ext))) return join(resolved, 'index' + ext);
                    }

                    // Try package.json main
                    const main = getPackageMain(resolved);
                    if (main) {
                        const mainPath = join(resolved, main);
                        if (fileExists(mainPath)) return mainPath;
                        for (const ext of ['.js', '.mjs', '.cjs']) {
                            if (fileExists(mainPath + ext)) return mainPath + ext;
                        }
                    }

                    return null;
                }

                // Node modules resolution - check if at root
                function isRootDir(dir) {
                    if (!dir || dir === '/' || dir === '.') return true;
                    // Windows root like C:/ or C:
                    if (dir.match(/^[A-Za-z]:[\\/]?$/)) return true;
                    return false;
                }

                let currentDir = fromDir;
                let prevDir = null;
                while (currentDir && !isRootDir(currentDir) && currentDir !== prevDir) {
                    const nodeModulesDir = join(currentDir, 'node_modules');
                    const pkgDir = join(nodeModulesDir, specifier);

                    // Check if package exists
                    if (fileExists(pkgDir)) {
                        // It's a directory - find main entry
                        const main = getPackageMain(pkgDir) || 'index.js';
                        const mainPath = join(pkgDir, main);

                        if (fileExists(mainPath)) return mainPath;

                        // Try with extensions
                        for (const ext of ['.js', '.mjs', '.cjs']) {
                            if (fileExists(mainPath + ext)) return mainPath + ext;
                        }

                        // Try index files
                        for (const ext of ['.js', '.mjs', '.cjs']) {
                            if (fileExists(join(pkgDir, 'index' + ext))) return join(pkgDir, 'index' + ext);
                        }
                    }

                    // Check for file directly in node_modules
                    for (const ext of ['', '.js', '.mjs', '.cjs', '.json']) {
                        const filePath = join(nodeModulesDir, specifier + ext);
                        if (fileExists(filePath)) return filePath;
                    }

                    // Go up one directory
                    prevDir = currentDir;
                    currentDir = dirname(currentDir);
                }

                return null;
            }

            // Create require function for a given directory
            function createRequire(fromDir) {
                function require(specifier) {
                    // Check built-in modules first
                    if (builtinModules[specifier]) {
                        return builtinModules[specifier]();
                    }

                    // Resolve module path
                    const resolvedPath = resolveModule(specifier, fromDir);
                    if (!resolvedPath) {
                        throw new Error(`Cannot find module '${specifier}' from '${fromDir}'`);
                    }

                    // Check cache
                    if (globalThis.__moduleCache[resolvedPath]) {
                        return globalThis.__moduleCache[resolvedPath].exports;
                    }

                    // Read file
                    const code = tryReadFile(resolvedPath);
                    if (code === null) {
                        throw new Error(`Cannot read module '${resolvedPath}'`);
                    }

                    // Handle JSON files
                    if (resolvedPath.endsWith('.json')) {
                        const exports = JSON.parse(code);
                        globalThis.__moduleCache[resolvedPath] = { exports };
                        return exports;
                    }

                    // Create module object
                    const module = { exports: {}, id: resolvedPath, filename: resolvedPath };
                    globalThis.__moduleCache[resolvedPath] = module;

                    // Create require for this module's directory
                    const moduleDir = dirname(resolvedPath);
                    const moduleRequire = createRequire(moduleDir);
                    moduleRequire.resolve = (id) => resolveModule(id, moduleDir);
                    moduleRequire.cache = globalThis.__moduleCache;

                    // Wrap and execute
                    const wrapper = `(function(exports, require, module, __filename, __dirname) { ${code} \n})`;
                    try {
                        const fn = eval(wrapper);
                        fn(module.exports, moduleRequire, module, resolvedPath, moduleDir);
                    } catch (e) {
                        delete globalThis.__moduleCache[resolvedPath];
                        throw e;
                    }

                    return module.exports;
                }

                require.resolve = (id) => resolveModule(id, fromDir);
                require.cache = globalThis.__moduleCache;

                return require;
            }

            // Set up global require from cwd
            const cwd = globalThis.process?.cwd?.() || '.';
            globalThis.require = createRequire(cwd);
            globalThis.require.resolve = (id) => resolveModule(id, cwd);
            globalThis.require.cache = globalThis.__moduleCache;

            // Also support module.exports pattern for simple scripts
            globalThis.module = { exports: {} };
            globalThis.exports = globalThis.module.exports;
        "#;

        let source = Source::from_bytes(require_code.as_bytes());
        context
            .eval(source)
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;

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

        // Wrap the main script in a CommonJS-like wrapper to provide __dirname, __filename, etc.
        // This allows top-level scripts to use require() and have access to module-like globals
        // Convert Windows paths to forward slashes for consistency
        let normalized_filename = filename.replace('\\', "/");
        let wrapped_code = format!(
            r#"(function() {{
                const __filename = '{}';
                const __dirname = globalThis.path ? globalThis.path.dirname(__filename) : '.';
                const exports = {{}};
                const module = {{ exports: exports }};
                {}
            }})();"#,
            normalized_filename.replace('\'', "\\'"),
            js_code
        );

        // Evaluate the JavaScript code
        let source = Source::from_bytes(wrapped_code.as_bytes());
        let result = self.context.eval(source);

        // Run the event loop to completion, including waiting for workers
        self.run_event_loop()?;

        result.map_err(|e| RuntimeError::JsError(e.to_string()))
    }

    /// Run the event loop until all work is complete (including workers, timers, promises)
    ///
    /// This implements a proper event loop that:
    /// 1. Runs all immediate jobs (promises, microtasks)
    /// 2. Waits for timers to fire and runs their callbacks
    /// 3. Keeps running while there are active workers
    fn run_event_loop(&mut self) -> RuntimeResult<()> {
        use std::time::{Duration, Instant};

        let start_time = Instant::now();
        let max_runtime = Duration::from_secs(300); // 5 minute max runtime safety limit

        loop {
            // Safety: don't run forever
            if start_time.elapsed() > max_runtime {
                break;
            }

            // Run pending jobs - this processes promises and ready timers
            self.context
                .run_jobs()
                .map_err(|e| RuntimeError::JsError(e.to_string()))?;

            // Check if we have active workers or pending timers
            let has_workers = worker::has_active_workers();
            let has_timers = has_pending_timers();

            // If we have workers or pending timers, keep running
            if has_workers || has_timers {
                std::thread::sleep(Duration::from_millis(1));
                continue;
            }

            // No workers and no pending timers - we're done
            break;
        }

        // Final cleanup run
        self.context
            .run_jobs()
            .map_err(|e| RuntimeError::JsError(e.to_string()))?;

        Ok(())
    }

    /// Run a TypeScript file with full event loop support
    pub fn run_file(&mut self, path: &Path) -> RuntimeResult<JsValue> {
        let source = std::fs::read_to_string(path)?;

        // Get the full absolute path for __filename
        let full_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let full_path_str = full_path.to_string_lossy().to_string();

        self.run(&source, &full_path_str)
    }

    /// Execute TypeScript code as a module (supports top-level await)
    #[allow(dead_code)]
    pub fn execute_module(&mut self, code: &str, filename: &str) -> RuntimeResult<JsValue> {
        use std::time::{Duration, Instant};

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

        // Run the event loop until the module promise resolves
        // This handles top-level await properly
        let start_time = Instant::now();
        let max_runtime = Duration::from_secs(300); // 5 minute max

        loop {
            // Safety check
            if start_time.elapsed() > max_runtime {
                return Err(RuntimeError::ModuleError(
                    "Module execution timed out".to_string(),
                ));
            }

            // Run pending jobs
            self.context
                .run_jobs()
                .map_err(|e| RuntimeError::JsError(e.to_string()))?;

            // Check promise state
            match promise.state() {
                PromiseState::Fulfilled(_) => {
                    // Module executed successfully, now run event loop for workers/timers
                    self.run_event_loop()?;
                    return Ok(JsValue::undefined());
                }
                PromiseState::Rejected(err) => {
                    let err_str: JsString = err
                        .to_string(&mut self.context)
                        .unwrap_or_else(|_| js_string!("Unknown error"));
                    return Err(RuntimeError::ModuleError(err_str.to_std_string_escaped()));
                }
                PromiseState::Pending => {
                    // Still pending - check if we have timers or workers keeping us alive
                    let has_workers = worker::has_active_workers();
                    let has_timers = has_pending_timers();

                    if has_workers || has_timers {
                        // Keep running, there's async work to do
                        std::thread::sleep(Duration::from_millis(1));
                        continue;
                    }

                    // No timers or workers, but promise is pending
                    // Give it a bit more time for microtasks to complete
                    std::thread::sleep(Duration::from_millis(1));

                    // Run jobs again
                    self.context
                        .run_jobs()
                        .map_err(|e| RuntimeError::JsError(e.to_string()))?;

                    // Check again
                    match promise.state() {
                        PromiseState::Fulfilled(_) => {
                            self.run_event_loop()?;
                            return Ok(JsValue::undefined());
                        }
                        PromiseState::Rejected(err) => {
                            let err_str: JsString = err
                                .to_string(&mut self.context)
                                .unwrap_or_else(|_| js_string!("Unknown error"));
                            return Err(RuntimeError::ModuleError(err_str.to_std_string_escaped()));
                        }
                        PromiseState::Pending => {
                            // If still pending with no work, it might be waiting for
                            // something that won't happen. Continue for a bit.
                            continue;
                        }
                    }
                }
            }
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
