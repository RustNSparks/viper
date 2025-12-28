//! Ultra-fast file system module - Native Rust implementation
//!
//! High-performance Node.js-compatible fs module using direct syscalls.
//! No tokio runtime overhead - pure synchronous operations for maximum speed.
//!
//! Bun-style performance optimizations:
//! - Direct std::fs calls (no async overhead for sync operations)
//! - Zero-copy where possible
//! - Efficient buffer handling
//! - Native Rust string/path handling

use boa_engine::{
    Context, JsArgs, JsNativeError, JsObject, JsResult, JsValue, NativeFunction, Source, js_string,
    object::builtins::JsArrayBuffer, object::builtins::JsUint8Array,
};
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

/// Register the ultra-fast fs module
pub fn register_fs_module(context: &mut Context) -> JsResult<()> {
    let fs_obj = JsObject::with_null_proto();

    // ============================================================================
    // Synchronous methods (fastest - direct syscalls)
    // ============================================================================

    // readFileSync
    let read_file_sync = NativeFunction::from_fn_ptr(fs_read_file_sync);
    fs_obj.set(
        js_string!("readFileSync"),
        read_file_sync.to_js_function(context.realm()),
        false,
        context,
    )?;

    // writeFileSync
    let write_file_sync = NativeFunction::from_fn_ptr(fs_write_file_sync);
    fs_obj.set(
        js_string!("writeFileSync"),
        write_file_sync.to_js_function(context.realm()),
        false,
        context,
    )?;

    // appendFileSync
    let append_file_sync = NativeFunction::from_fn_ptr(fs_append_file_sync);
    fs_obj.set(
        js_string!("appendFileSync"),
        append_file_sync.to_js_function(context.realm()),
        false,
        context,
    )?;

    // existsSync
    let exists_sync = NativeFunction::from_fn_ptr(fs_exists_sync);
    fs_obj.set(
        js_string!("existsSync"),
        exists_sync.to_js_function(context.realm()),
        false,
        context,
    )?;

    // statSync
    let stat_sync = NativeFunction::from_fn_ptr(fs_stat_sync);
    fs_obj.set(
        js_string!("statSync"),
        stat_sync.to_js_function(context.realm()),
        false,
        context,
    )?;

    // lstatSync (reuse stat_sync by creating a new one)
    let lstat_sync = NativeFunction::from_fn_ptr(fs_stat_sync);
    fs_obj.set(
        js_string!("lstatSync"),
        lstat_sync.to_js_function(context.realm()),
        false,
        context,
    )?;

    // readdirSync
    let readdir_sync = NativeFunction::from_fn_ptr(fs_readdir_sync);
    fs_obj.set(
        js_string!("readdirSync"),
        readdir_sync.to_js_function(context.realm()),
        false,
        context,
    )?;

    // mkdirSync
    let mkdir_sync = NativeFunction::from_fn_ptr(fs_mkdir_sync);
    fs_obj.set(
        js_string!("mkdirSync"),
        mkdir_sync.to_js_function(context.realm()),
        false,
        context,
    )?;

    // rmdirSync
    let rmdir_sync = NativeFunction::from_fn_ptr(fs_rmdir_sync);
    fs_obj.set(
        js_string!("rmdirSync"),
        rmdir_sync.to_js_function(context.realm()),
        false,
        context,
    )?;

    // rmSync
    let rm_sync = NativeFunction::from_fn_ptr(fs_rm_sync);
    fs_obj.set(
        js_string!("rmSync"),
        rm_sync.to_js_function(context.realm()),
        false,
        context,
    )?;

    // unlinkSync
    let unlink_sync = NativeFunction::from_fn_ptr(fs_unlink_sync);
    fs_obj.set(
        js_string!("unlinkSync"),
        unlink_sync.to_js_function(context.realm()),
        false,
        context,
    )?;

    // renameSync
    let rename_sync = NativeFunction::from_fn_ptr(fs_rename_sync);
    fs_obj.set(
        js_string!("renameSync"),
        rename_sync.to_js_function(context.realm()),
        false,
        context,
    )?;

    // copyFileSync
    let copy_file_sync = NativeFunction::from_fn_ptr(fs_copy_file_sync);
    fs_obj.set(
        js_string!("copyFileSync"),
        copy_file_sync.to_js_function(context.realm()),
        false,
        context,
    )?;

    // chmodSync
    let chmod_sync = NativeFunction::from_fn_ptr(fs_chmod_sync);
    fs_obj.set(
        js_string!("chmodSync"),
        chmod_sync.to_js_function(context.realm()),
        false,
        context,
    )?;

    // realpathSync
    let realpath_sync = NativeFunction::from_fn_ptr(fs_realpath_sync);
    fs_obj.set(
        js_string!("realpathSync"),
        realpath_sync.to_js_function(context.realm()),
        false,
        context,
    )?;

    // accessSync
    let access_sync = NativeFunction::from_fn_ptr(fs_access_sync);
    fs_obj.set(
        js_string!("accessSync"),
        access_sync.to_js_function(context.realm()),
        false,
        context,
    )?;

    // truncateSync
    let truncate_sync = NativeFunction::from_fn_ptr(fs_truncate_sync);
    fs_obj.set(
        js_string!("truncateSync"),
        truncate_sync.to_js_function(context.realm()),
        false,
        context,
    )?;

    // openSync
    let open_sync = NativeFunction::from_fn_ptr(fs_open_sync);
    fs_obj.set(
        js_string!("openSync"),
        open_sync.to_js_function(context.realm()),
        false,
        context,
    )?;

    // closeSync
    let close_sync = NativeFunction::from_fn_ptr(fs_close_sync);
    fs_obj.set(
        js_string!("closeSync"),
        close_sync.to_js_function(context.realm()),
        false,
        context,
    )?;

    // readSync
    let read_sync = NativeFunction::from_fn_ptr(fs_read_sync);
    fs_obj.set(
        js_string!("readSync"),
        read_sync.to_js_function(context.realm()),
        false,
        context,
    )?;

    // writeSync
    let write_sync = NativeFunction::from_fn_ptr(fs_write_sync);
    fs_obj.set(
        js_string!("writeSync"),
        write_sync.to_js_function(context.realm()),
        false,
        context,
    )?;

    // Constants
    let constants = JsObject::with_null_proto();
    constants.set(js_string!("F_OK"), JsValue::from(0), false, context)?;
    constants.set(js_string!("R_OK"), JsValue::from(4), false, context)?;
    constants.set(js_string!("W_OK"), JsValue::from(2), false, context)?;
    constants.set(js_string!("X_OK"), JsValue::from(1), false, context)?;
    constants.set(
        js_string!("COPYFILE_EXCL"),
        JsValue::from(1),
        false,
        context,
    )?;
    constants.set(
        js_string!("COPYFILE_FICLONE"),
        JsValue::from(2),
        false,
        context,
    )?;
    constants.set(
        js_string!("COPYFILE_FICLONE_FORCE"),
        JsValue::from(4),
        false,
        context,
    )?;
    constants.set(js_string!("O_RDONLY"), JsValue::from(0), false, context)?;
    constants.set(js_string!("O_WRONLY"), JsValue::from(1), false, context)?;
    constants.set(js_string!("O_RDWR"), JsValue::from(2), false, context)?;
    constants.set(js_string!("O_CREAT"), JsValue::from(0o100), false, context)?;
    constants.set(js_string!("O_EXCL"), JsValue::from(0o200), false, context)?;
    constants.set(js_string!("O_TRUNC"), JsValue::from(0o1000), false, context)?;
    constants.set(
        js_string!("O_APPEND"),
        JsValue::from(0o2000),
        false,
        context,
    )?;
    constants.set(
        js_string!("S_IFMT"),
        JsValue::from(0o170000),
        false,
        context,
    )?;
    constants.set(
        js_string!("S_IFREG"),
        JsValue::from(0o100000),
        false,
        context,
    )?;
    constants.set(
        js_string!("S_IFDIR"),
        JsValue::from(0o40000),
        false,
        context,
    )?;
    constants.set(
        js_string!("S_IFLNK"),
        JsValue::from(0o120000),
        false,
        context,
    )?;
    fs_obj.set(js_string!("constants"), constants, false, context)?;

    // Set fs on global
    context
        .global_object()
        .set(js_string!("fs"), fs_obj.clone(), false, context)?;

    // Register async wrappers and promises API
    register_async_wrappers(context)?;

    Ok(())
}

/// Register async wrappers that return Promises
fn register_async_wrappers(context: &mut Context) -> JsResult<()> {
    let async_code = r#"
    (function() {
        const _fs = globalThis.fs;

        // Promisify a sync function
        function promisify(syncFn) {
            return function(...args) {
                return new Promise((resolve, reject) => {
                    try {
                        const result = syncFn(...args);
                        resolve(result);
                    } catch (e) {
                        reject(e);
                    }
                });
            };
        }

        // Async versions (callback style for Node.js compat)
        _fs.readFile = function(path, options, callback) {
            if (typeof options === 'function') {
                callback = options;
                options = {};
            }
            try {
                const result = _fs.readFileSync(path, options);
                if (callback) queueMicrotask(() => callback(null, result));
                else return Promise.resolve(result);
            } catch (e) {
                if (callback) queueMicrotask(() => callback(e));
                else return Promise.reject(e);
            }
        };

        _fs.writeFile = function(path, data, options, callback) {
            if (typeof options === 'function') {
                callback = options;
                options = {};
            }
            try {
                _fs.writeFileSync(path, data, options);
                if (callback) queueMicrotask(() => callback(null));
                else return Promise.resolve();
            } catch (e) {
                if (callback) queueMicrotask(() => callback(e));
                else return Promise.reject(e);
            }
        };

        _fs.appendFile = function(path, data, options, callback) {
            if (typeof options === 'function') {
                callback = options;
                options = {};
            }
            try {
                _fs.appendFileSync(path, data, options);
                if (callback) queueMicrotask(() => callback(null));
                else return Promise.resolve();
            } catch (e) {
                if (callback) queueMicrotask(() => callback(e));
                else return Promise.reject(e);
            }
        };

        _fs.exists = function(path, callback) {
            const exists = _fs.existsSync(path);
            if (callback) queueMicrotask(() => callback(exists));
            else return Promise.resolve(exists);
        };

        _fs.stat = function(path, options, callback) {
            if (typeof options === 'function') {
                callback = options;
                options = {};
            }
            try {
                const result = _fs.statSync(path, options);
                if (callback) queueMicrotask(() => callback(null, result));
                else return Promise.resolve(result);
            } catch (e) {
                if (callback) queueMicrotask(() => callback(e));
                else return Promise.reject(e);
            }
        };

        _fs.lstat = _fs.stat;

        _fs.readdir = function(path, options, callback) {
            if (typeof options === 'function') {
                callback = options;
                options = {};
            }
            try {
                const result = _fs.readdirSync(path, options);
                if (callback) queueMicrotask(() => callback(null, result));
                else return Promise.resolve(result);
            } catch (e) {
                if (callback) queueMicrotask(() => callback(e));
                else return Promise.reject(e);
            }
        };

        _fs.mkdir = function(path, options, callback) {
            if (typeof options === 'function') {
                callback = options;
                options = {};
            }
            try {
                _fs.mkdirSync(path, options);
                if (callback) queueMicrotask(() => callback(null));
                else return Promise.resolve();
            } catch (e) {
                if (callback) queueMicrotask(() => callback(e));
                else return Promise.reject(e);
            }
        };

        _fs.rmdir = function(path, options, callback) {
            if (typeof options === 'function') {
                callback = options;
                options = {};
            }
            try {
                _fs.rmdirSync(path, options);
                if (callback) queueMicrotask(() => callback(null));
                else return Promise.resolve();
            } catch (e) {
                if (callback) queueMicrotask(() => callback(e));
                else return Promise.reject(e);
            }
        };

        _fs.rm = function(path, options, callback) {
            if (typeof options === 'function') {
                callback = options;
                options = {};
            }
            try {
                _fs.rmSync(path, options);
                if (callback) queueMicrotask(() => callback(null));
                else return Promise.resolve();
            } catch (e) {
                if (callback) queueMicrotask(() => callback(e));
                else return Promise.reject(e);
            }
        };

        _fs.unlink = function(path, callback) {
            try {
                _fs.unlinkSync(path);
                if (callback) queueMicrotask(() => callback(null));
                else return Promise.resolve();
            } catch (e) {
                if (callback) queueMicrotask(() => callback(e));
                else return Promise.reject(e);
            }
        };

        _fs.rename = function(oldPath, newPath, callback) {
            try {
                _fs.renameSync(oldPath, newPath);
                if (callback) queueMicrotask(() => callback(null));
                else return Promise.resolve();
            } catch (e) {
                if (callback) queueMicrotask(() => callback(e));
                else return Promise.reject(e);
            }
        };

        _fs.copyFile = function(src, dest, mode, callback) {
            if (typeof mode === 'function') {
                callback = mode;
                mode = 0;
            }
            try {
                _fs.copyFileSync(src, dest, mode);
                if (callback) queueMicrotask(() => callback(null));
                else return Promise.resolve();
            } catch (e) {
                if (callback) queueMicrotask(() => callback(e));
                else return Promise.reject(e);
            }
        };

        _fs.access = function(path, mode, callback) {
            if (typeof mode === 'function') {
                callback = mode;
                mode = 0;
            }
            try {
                _fs.accessSync(path, mode);
                if (callback) queueMicrotask(() => callback(null));
                else return Promise.resolve();
            } catch (e) {
                if (callback) queueMicrotask(() => callback(e));
                else return Promise.reject(e);
            }
        };

        _fs.realpath = function(path, options, callback) {
            if (typeof options === 'function') {
                callback = options;
                options = {};
            }
            try {
                const result = _fs.realpathSync(path, options);
                if (callback) queueMicrotask(() => callback(null, result));
                else return Promise.resolve(result);
            } catch (e) {
                if (callback) queueMicrotask(() => callback(e));
                else return Promise.reject(e);
            }
        };

        _fs.chmod = function(path, mode, callback) {
            try {
                _fs.chmodSync(path, mode);
                if (callback) queueMicrotask(() => callback(null));
                else return Promise.resolve();
            } catch (e) {
                if (callback) queueMicrotask(() => callback(e));
                else return Promise.reject(e);
            }
        };

        _fs.truncate = function(path, len, callback) {
            if (typeof len === 'function') {
                callback = len;
                len = 0;
            }
            try {
                _fs.truncateSync(path, len);
                if (callback) queueMicrotask(() => callback(null));
                else return Promise.resolve();
            } catch (e) {
                if (callback) queueMicrotask(() => callback(e));
                else return Promise.reject(e);
            }
        };

        // fs.promises API
        _fs.promises = {
            readFile: promisify(_fs.readFileSync),
            writeFile: promisify(_fs.writeFileSync),
            appendFile: promisify(_fs.appendFileSync),
            stat: promisify(_fs.statSync),
            lstat: promisify(_fs.statSync),
            readdir: promisify(_fs.readdirSync),
            mkdir: promisify(_fs.mkdirSync),
            rmdir: promisify(_fs.rmdirSync),
            rm: promisify(_fs.rmSync),
            unlink: promisify(_fs.unlinkSync),
            rename: promisify(_fs.renameSync),
            copyFile: promisify(_fs.copyFileSync),
            access: promisify(_fs.accessSync),
            realpath: promisify(_fs.realpathSync),
            chmod: promisify(_fs.chmodSync),
            truncate: promisify(_fs.truncateSync),
        };

        // Dirent class for readdir with withFileTypes
        class Dirent {
            constructor(name, isDir, isFile, isSymlink) {
                this.name = name;
                this._isDir = isDir;
                this._isFile = isFile;
                this._isSymlink = isSymlink;
            }
            isDirectory() { return this._isDir; }
            isFile() { return this._isFile; }
            isSymbolicLink() { return this._isSymlink; }
            isBlockDevice() { return false; }
            isCharacterDevice() { return false; }
            isFIFO() { return false; }
            isSocket() { return false; }
        }
        _fs.Dirent = Dirent;

        // Stats class
        class Stats {
            constructor(data) {
                this.dev = data.dev || 0;
                this.ino = data.ino || 0;
                this.mode = data.mode || 0;
                this.nlink = data.nlink || 1;
                this.uid = data.uid || 0;
                this.gid = data.gid || 0;
                this.rdev = data.rdev || 0;
                this.size = data.size || 0;
                this.blksize = data.blksize || 4096;
                this.blocks = data.blocks || 0;
                this.atimeMs = data.atimeMs || 0;
                this.mtimeMs = data.mtimeMs || 0;
                this.ctimeMs = data.ctimeMs || 0;
                this.birthtimeMs = data.birthtimeMs || 0;
                this.atime = new Date(this.atimeMs);
                this.mtime = new Date(this.mtimeMs);
                this.ctime = new Date(this.ctimeMs);
                this.birthtime = new Date(this.birthtimeMs);
                this._isFile = data.isFile || false;
                this._isDir = data.isDirectory || false;
                this._isSymlink = data.isSymbolicLink || false;
            }
            isFile() { return this._isFile; }
            isDirectory() { return this._isDir; }
            isSymbolicLink() { return this._isSymlink; }
            isBlockDevice() { return false; }
            isCharacterDevice() { return false; }
            isFIFO() { return false; }
            isSocket() { return false; }
        }
        _fs.Stats = Stats;
    })();
    "#;

    let source = Source::from_bytes(async_code.as_bytes());
    context.eval(source)?;
    Ok(())
}

// ============================================================================
// Native sync implementations
// ============================================================================

/// fs.readFileSync(path[, options])
fn fs_read_file_sync(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = args
        .get_or_undefined(0)
        .to_string(context)?
        .to_std_string_escaped();
    let options = args.get(1);

    // Check encoding option
    let encoding = get_encoding_option(options, context)?;

    let bytes = fs::read(&path).map_err(|e| {
        JsNativeError::error().with_message(format!(
            "ENOENT: no such file or directory, open '{}'",
            path
        ))
    })?;

    if let Some(enc) = encoding {
        // Return string
        let content = match enc.as_str() {
            "utf8" | "utf-8" => String::from_utf8_lossy(&bytes).to_string(),
            "ascii" | "latin1" => bytes.iter().map(|&b| b as char).collect(),
            "base64" => {
                use base64::{Engine, engine::general_purpose::STANDARD};
                STANDARD.encode(&bytes)
            }
            "hex" => bytes.iter().map(|b| format!("{:02x}", b)).collect(),
            _ => String::from_utf8_lossy(&bytes).to_string(),
        };
        Ok(JsValue::from(js_string!(content)))
    } else {
        // Return Buffer
        create_buffer_from_bytes(&bytes, context)
    }
}

/// fs.writeFileSync(path, data[, options])
fn fs_write_file_sync(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = args
        .get_or_undefined(0)
        .to_string(context)?
        .to_std_string_escaped();
    let data = args.get_or_undefined(1);
    let options = args.get(2);

    let encoding = get_encoding_option(options, context)?.unwrap_or_else(|| "utf8".to_string());

    let bytes = extract_bytes(data, &encoding, context)?;

    fs::write(&path, &bytes).map_err(|e| {
        JsNativeError::error().with_message(format!(
            "ENOENT: no such file or directory, open '{}'",
            path
        ))
    })?;

    Ok(JsValue::undefined())
}

/// fs.appendFileSync(path, data[, options])
fn fs_append_file_sync(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = args
        .get_or_undefined(0)
        .to_string(context)?
        .to_std_string_escaped();
    let data = args.get_or_undefined(1);
    let options = args.get(2);

    let encoding = get_encoding_option(options, context)?.unwrap_or_else(|| "utf8".to_string());
    let bytes = extract_bytes(data, &encoding, context)?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| JsNativeError::error().with_message(format!("Failed to open file: {}", e)))?;

    file.write_all(&bytes)
        .map_err(|e| JsNativeError::error().with_message(format!("Failed to write: {}", e)))?;

    Ok(JsValue::undefined())
}

/// fs.existsSync(path)
fn fs_exists_sync(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let path = args
        .get_or_undefined(0)
        .to_string(context)?
        .to_std_string_escaped();
    Ok(JsValue::from(Path::new(&path).exists()))
}

/// fs.statSync(path[, options])
fn fs_stat_sync(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let path = args
        .get_or_undefined(0)
        .to_string(context)?
        .to_std_string_escaped();

    let metadata = fs::metadata(&path).map_err(|e| {
        JsNativeError::error().with_message(format!(
            "ENOENT: no such file or directory, stat '{}'",
            path
        ))
    })?;

    create_stats_object(&metadata, context)
}

/// fs.readdirSync(path[, options])
fn fs_readdir_sync(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let path = args
        .get_or_undefined(0)
        .to_string(context)?
        .to_std_string_escaped();
    let options = args.get(1);

    let with_file_types = if let Some(opts) = options {
        if let Some(obj) = opts.as_object() {
            obj.get(js_string!("withFileTypes"), context)?.to_boolean()
        } else {
            false
        }
    } else {
        false
    };

    let entries = fs::read_dir(&path).map_err(|e| {
        JsNativeError::error().with_message(format!(
            "ENOENT: no such file or directory, scandir '{}'",
            path
        ))
    })?;

    let arr = boa_engine::object::builtins::JsArray::new(context);

    for (i, entry) in entries.enumerate() {
        let entry = entry.map_err(|e| {
            JsNativeError::error().with_message(format!("Failed to read entry: {}", e))
        })?;

        let name = entry.file_name().to_string_lossy().to_string();

        if with_file_types {
            let file_type = entry.file_type().map_err(|e| {
                JsNativeError::error().with_message(format!("Failed to get file type: {}", e))
            })?;

            // Create Dirent object
            let dirent = JsObject::with_null_proto();
            dirent.set(js_string!("name"), js_string!(name), false, context)?;
            dirent.set(js_string!("_isDir"), file_type.is_dir(), false, context)?;
            dirent.set(js_string!("_isFile"), file_type.is_file(), false, context)?;
            dirent.set(
                js_string!("_isSymlink"),
                file_type.is_symlink(),
                false,
                context,
            )?;

            // Add methods
            let is_dir_code = "function() { return this._isDir; }";
            let is_dir_fn = context.eval(Source::from_bytes(is_dir_code.as_bytes()))?;
            dirent.set(js_string!("isDirectory"), is_dir_fn, false, context)?;

            let is_file_code = "function() { return this._isFile; }";
            let is_file_fn = context.eval(Source::from_bytes(is_file_code.as_bytes()))?;
            dirent.set(js_string!("isFile"), is_file_fn, false, context)?;

            let is_symlink_code = "function() { return this._isSymlink; }";
            let is_symlink_fn = context.eval(Source::from_bytes(is_symlink_code.as_bytes()))?;
            dirent.set(js_string!("isSymbolicLink"), is_symlink_fn, false, context)?;

            arr.set(i as u32, dirent, false, context)?;
        } else {
            arr.set(i as u32, js_string!(name), false, context)?;
        }
    }

    Ok(arr.into())
}

/// fs.mkdirSync(path[, options])
fn fs_mkdir_sync(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let path = args
        .get_or_undefined(0)
        .to_string(context)?
        .to_std_string_escaped();
    let options = args.get(1);

    let recursive = if let Some(opts) = options {
        if let Some(obj) = opts.as_object() {
            obj.get(js_string!("recursive"), context)?.to_boolean()
        } else if opts.is_boolean() {
            opts.to_boolean()
        } else {
            false
        }
    } else {
        false
    };

    let result = if recursive {
        fs::create_dir_all(&path)
    } else {
        fs::create_dir(&path)
    };

    result.map_err(|e| {
        JsNativeError::error()
            .with_message(format!("EEXIST: file already exists, mkdir '{}'", path))
    })?;

    Ok(JsValue::undefined())
}

/// fs.rmdirSync(path[, options])
fn fs_rmdir_sync(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let path = args
        .get_or_undefined(0)
        .to_string(context)?
        .to_std_string_escaped();
    let options = args.get(1);

    let recursive = if let Some(opts) = options {
        if let Some(obj) = opts.as_object() {
            obj.get(js_string!("recursive"), context)?.to_boolean()
        } else {
            false
        }
    } else {
        false
    };

    let result = if recursive {
        fs::remove_dir_all(&path)
    } else {
        fs::remove_dir(&path)
    };

    result.map_err(|e| {
        JsNativeError::error().with_message(format!(
            "ENOENT: no such file or directory, rmdir '{}'",
            path
        ))
    })?;

    Ok(JsValue::undefined())
}

/// fs.rmSync(path[, options])
fn fs_rm_sync(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let path = args
        .get_or_undefined(0)
        .to_string(context)?
        .to_std_string_escaped();
    let options = args.get(1);

    let (recursive, force) = if let Some(opts) = options {
        if let Some(obj) = opts.as_object() {
            let r = obj.get(js_string!("recursive"), context)?.to_boolean();
            let f = obj.get(js_string!("force"), context)?.to_boolean();
            (r, f)
        } else {
            (false, false)
        }
    } else {
        (false, false)
    };

    let p = Path::new(&path);
    if !p.exists() {
        if force {
            return Ok(JsValue::undefined());
        }
        return Err(JsNativeError::error()
            .with_message(format!("ENOENT: no such file or directory, rm '{}'", path))
            .into());
    }

    if p.is_dir() {
        if recursive {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_dir(&path)
        }
    } else {
        fs::remove_file(&path)
    }
    .map_err(|e| JsNativeError::error().with_message(format!("Failed to remove: {}", e)))?;

    Ok(JsValue::undefined())
}

/// fs.unlinkSync(path)
fn fs_unlink_sync(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let path = args
        .get_or_undefined(0)
        .to_string(context)?
        .to_std_string_escaped();

    fs::remove_file(&path).map_err(|e| {
        JsNativeError::error().with_message(format!(
            "ENOENT: no such file or directory, unlink '{}'",
            path
        ))
    })?;

    Ok(JsValue::undefined())
}

/// fs.renameSync(oldPath, newPath)
fn fs_rename_sync(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let old_path = args
        .get_or_undefined(0)
        .to_string(context)?
        .to_std_string_escaped();
    let new_path = args
        .get_or_undefined(1)
        .to_string(context)?
        .to_std_string_escaped();

    fs::rename(&old_path, &new_path).map_err(|e| {
        JsNativeError::error().with_message(format!(
            "ENOENT: no such file or directory, rename '{}' -> '{}'",
            old_path, new_path
        ))
    })?;

    Ok(JsValue::undefined())
}

/// fs.copyFileSync(src, dest[, mode])
fn fs_copy_file_sync(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let src = args
        .get_or_undefined(0)
        .to_string(context)?
        .to_std_string_escaped();
    let dest = args
        .get_or_undefined(1)
        .to_string(context)?
        .to_std_string_escaped();
    let mode = args
        .get(2)
        .map(|v| v.to_u32(context))
        .transpose()?
        .unwrap_or(0);

    // COPYFILE_EXCL = 1: fail if dest exists
    if mode & 1 != 0 && Path::new(&dest).exists() {
        return Err(JsNativeError::error()
            .with_message(format!(
                "EEXIST: file already exists, copyfile '{}' -> '{}'",
                src, dest
            ))
            .into());
    }

    fs::copy(&src, &dest).map_err(|e| {
        JsNativeError::error().with_message(format!(
            "ENOENT: no such file or directory, copyfile '{}' -> '{}'",
            src, dest
        ))
    })?;

    Ok(JsValue::undefined())
}

/// fs.chmodSync(path, mode)
fn fs_chmod_sync(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let path = args
        .get_or_undefined(0)
        .to_string(context)?
        .to_std_string_escaped();
    let _mode = args.get_or_undefined(1).to_u32(context)?;

    // On Windows, chmod is limited. On Unix, we'd use std::os::unix::fs::PermissionsExt
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = fs::Permissions::from_mode(_mode);
        fs::set_permissions(&path, permissions)
            .map_err(|e| JsNativeError::error().with_message(format!("Failed to chmod: {}", e)))?;
    }

    #[cfg(not(unix))]
    {
        // On Windows, just check the file exists
        if !Path::new(&path).exists() {
            return Err(JsNativeError::error()
                .with_message(format!(
                    "ENOENT: no such file or directory, chmod '{}'",
                    path
                ))
                .into());
        }
    }

    Ok(JsValue::undefined())
}

/// fs.realpathSync(path[, options])
fn fs_realpath_sync(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let path = args
        .get_or_undefined(0)
        .to_string(context)?
        .to_std_string_escaped();

    let real = fs::canonicalize(&path).map_err(|e| {
        JsNativeError::error().with_message(format!(
            "ENOENT: no such file or directory, realpath '{}'",
            path
        ))
    })?;

    Ok(JsValue::from(js_string!(
        real.to_string_lossy().to_string()
    )))
}

/// fs.accessSync(path[, mode])
fn fs_access_sync(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let path = args
        .get_or_undefined(0)
        .to_string(context)?
        .to_std_string_escaped();
    let _mode = args
        .get(1)
        .map(|v| v.to_u32(context))
        .transpose()?
        .unwrap_or(0);

    // Basic existence check (full access check would require platform-specific code)
    if !Path::new(&path).exists() {
        return Err(JsNativeError::error()
            .with_message(format!(
                "ENOENT: no such file or directory, access '{}'",
                path
            ))
            .into());
    }

    Ok(JsValue::undefined())
}

/// fs.truncateSync(path[, len])
fn fs_truncate_sync(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let path = args
        .get_or_undefined(0)
        .to_string(context)?
        .to_std_string_escaped();
    let len = args
        .get(1)
        .map(|v| v.to_u32(context))
        .transpose()?
        .unwrap_or(0) as u64;

    let file = OpenOptions::new()
        .write(true)
        .open(&path)
        .map_err(|e| JsNativeError::error().with_message(format!("Failed to open: {}", e)))?;

    file.set_len(len)
        .map_err(|e| JsNativeError::error().with_message(format!("Failed to truncate: {}", e)))?;

    Ok(JsValue::undefined())
}

// File descriptor operations (simplified - using path-based approach internally)
use std::collections::HashMap;
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref OPEN_FILES: Mutex<HashMap<i32, std::fs::File>> = Mutex::new(HashMap::new());
    static ref NEXT_FD: Mutex<i32> = Mutex::new(3); // Start after stdin/stdout/stderr
}

/// fs.openSync(path, flags[, mode])
fn fs_open_sync(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let path = args
        .get_or_undefined(0)
        .to_string(context)?
        .to_std_string_escaped();
    let flags = args.get_or_undefined(1);

    let flags_str = if flags.is_string() {
        flags.to_string(context)?.to_std_string_escaped()
    } else {
        "r".to_string()
    };

    let mut opts = OpenOptions::new();
    match flags_str.as_str() {
        "r" => {
            opts.read(true);
        }
        "r+" => {
            opts.read(true).write(true);
        }
        "w" => {
            opts.write(true).create(true).truncate(true);
        }
        "w+" => {
            opts.read(true).write(true).create(true).truncate(true);
        }
        "a" => {
            opts.append(true).create(true);
        }
        "a+" => {
            opts.read(true).append(true).create(true);
        }
        "wx" | "xw" => {
            opts.write(true).create_new(true);
        }
        _ => {
            opts.read(true);
        }
    }

    let file = opts.open(&path).map_err(|e| {
        JsNativeError::error().with_message(format!(
            "ENOENT: no such file or directory, open '{}'",
            path
        ))
    })?;

    let mut fd_counter = NEXT_FD.lock().unwrap();
    let fd = *fd_counter;
    *fd_counter += 1;

    OPEN_FILES.lock().unwrap().insert(fd, file);

    Ok(JsValue::from(fd))
}

/// fs.closeSync(fd)
fn fs_close_sync(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let fd = args.get_or_undefined(0).to_i32(context)?;

    OPEN_FILES.lock().unwrap().remove(&fd);

    Ok(JsValue::undefined())
}

/// fs.readSync(fd, buffer, offset, length, position)
fn fs_read_sync(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let fd = args.get_or_undefined(0).to_i32(context)?;
    let buffer = args.get_or_undefined(1);
    let offset = args
        .get(2)
        .map(|v| v.to_u32(context))
        .transpose()?
        .unwrap_or(0) as usize;
    let length = args.get(3).map(|v| v.to_u32(context)).transpose()?;
    let position = args.get(4);

    let mut files = OPEN_FILES.lock().unwrap();
    let file = files.get_mut(&fd).ok_or_else(|| {
        JsNativeError::error().with_message(format!("EBADF: bad file descriptor"))
    })?;

    // Handle position
    if let Some(pos) = position {
        if !pos.is_null() && !pos.is_undefined() {
            let pos_val = pos.to_u32(context)? as u64;
            file.seek(SeekFrom::Start(pos_val)).map_err(|e| {
                JsNativeError::error().with_message(format!("Failed to seek: {}", e))
            })?;
        }
    }

    // Get buffer object
    let buf_obj = buffer.as_object().ok_or_else(|| {
        JsNativeError::typ().with_message("buffer must be a Buffer or Uint8Array")
    })?;

    let buf_len = buf_obj
        .get(js_string!("length"), context)?
        .to_u32(context)? as usize;
    let read_len = length.map(|l| l as usize).unwrap_or(buf_len - offset);

    let mut temp_buf = vec![0u8; read_len];
    let bytes_read = file
        .read(&mut temp_buf)
        .map_err(|e| JsNativeError::error().with_message(format!("Failed to read: {}", e)))?;

    // Copy to buffer
    for i in 0..bytes_read {
        buf_obj.set(
            offset + i,
            JsValue::from(temp_buf[i] as u32),
            false,
            context,
        )?;
    }

    Ok(JsValue::from(bytes_read as u32))
}

/// fs.writeSync(fd, buffer[, offset[, length[, position]]])
fn fs_write_sync(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let fd = args.get_or_undefined(0).to_i32(context)?;
    let buffer = args.get_or_undefined(1);
    let offset = args
        .get(2)
        .map(|v| v.to_u32(context))
        .transpose()?
        .unwrap_or(0) as usize;
    let length = args.get(3).map(|v| v.to_u32(context)).transpose()?;
    let position = args.get(4);

    let mut files = OPEN_FILES.lock().unwrap();
    let file = files
        .get_mut(&fd)
        .ok_or_else(|| JsNativeError::error().with_message("EBADF: bad file descriptor"))?;

    // Handle position
    if let Some(pos) = position {
        if !pos.is_null() && !pos.is_undefined() {
            let pos_val = pos.to_u32(context)? as u64;
            file.seek(SeekFrom::Start(pos_val)).map_err(|e| {
                JsNativeError::error().with_message(format!("Failed to seek: {}", e))
            })?;
        }
    }

    // Handle string input
    if buffer.is_string() {
        let s = buffer.to_string(context)?.to_std_string_escaped();
        let bytes = s.as_bytes();
        let written = file
            .write(bytes)
            .map_err(|e| JsNativeError::error().with_message(format!("Failed to write: {}", e)))?;
        return Ok(JsValue::from(written as u32));
    }

    // Handle buffer input
    let buf_obj = buffer.as_object().ok_or_else(|| {
        JsNativeError::typ().with_message("buffer must be a string, Buffer, or Uint8Array")
    })?;

    let buf_len = buf_obj
        .get(js_string!("length"), context)?
        .to_u32(context)? as usize;
    let write_len = length.map(|l| l as usize).unwrap_or(buf_len - offset);

    let mut bytes = Vec::with_capacity(write_len);
    for i in 0..write_len {
        let byte = buf_obj.get(offset + i, context)?.to_u32(context)? as u8;
        bytes.push(byte);
    }

    let written = file
        .write(&bytes)
        .map_err(|e| JsNativeError::error().with_message(format!("Failed to write: {}", e)))?;

    Ok(JsValue::from(written as u32))
}

// ============================================================================
// Helper functions
// ============================================================================

/// Extract encoding option from options argument
fn get_encoding_option(
    options: Option<&JsValue>,
    context: &mut Context,
) -> JsResult<Option<String>> {
    if let Some(opts) = options {
        if opts.is_string() {
            return Ok(Some(opts.to_string(context)?.to_std_string_escaped()));
        }
        if let Some(obj) = opts.as_object() {
            let enc = obj.get(js_string!("encoding"), context)?;
            if !enc.is_undefined() && !enc.is_null() {
                return Ok(Some(enc.to_string(context)?.to_std_string_escaped()));
            }
        }
    }
    Ok(None)
}

/// Extract bytes from data argument
fn extract_bytes(data: &JsValue, encoding: &str, context: &mut Context) -> JsResult<Vec<u8>> {
    if data.is_string() {
        let s = data.to_string(context)?.to_std_string_escaped();
        return Ok(s.into_bytes());
    }

    if let Some(obj) = data.as_object() {
        // Try as Uint8Array/Buffer
        if let Ok(arr) = JsUint8Array::from_object(obj.clone()) {
            let len = arr.length(context)?;
            let mut bytes = Vec::with_capacity(len);
            for i in 0..len {
                let byte = arr.get(i, context)?.to_u32(context)? as u8;
                bytes.push(byte);
            }
            return Ok(bytes);
        }

        // Try as array-like
        if let Ok(len_val) = obj.get(js_string!("length"), context) {
            if let Ok(len) = len_val.to_u32(context) {
                let mut bytes = Vec::with_capacity(len as usize);
                for i in 0..len {
                    let byte = obj.get(i, context)?.to_u32(context)? as u8;
                    bytes.push(byte);
                }
                return Ok(bytes);
            }
        }
    }

    Err(JsNativeError::typ()
        .with_message("data must be a string, Buffer, or Uint8Array")
        .into())
}

/// Create a Buffer from bytes
fn create_buffer_from_bytes(bytes: &[u8], context: &mut Context) -> JsResult<JsValue> {
    // Use the global __createBuffer function if available
    let create_fn = context
        .global_object()
        .get(js_string!("__createBuffer"), context)?;

    let array_buffer = JsArrayBuffer::new(bytes.len(), context)?;
    let uint8_array = JsUint8Array::from_array_buffer(array_buffer, context)?;

    for (i, byte) in bytes.iter().enumerate() {
        uint8_array.set(i, JsValue::from(*byte as u32), false, context)?;
    }

    if let Some(fn_obj) = create_fn.as_object() {
        if fn_obj.is_callable() {
            return fn_obj.call(&JsValue::undefined(), &[uint8_array.into()], context);
        }
    }

    Ok(uint8_array.into())
}

/// Create a Stats object from metadata
fn create_stats_object(metadata: &fs::Metadata, context: &mut Context) -> JsResult<JsValue> {
    let stats = JsObject::with_null_proto();

    let size = metadata.len();
    let is_file = metadata.is_file();
    let is_dir = metadata.is_dir();
    let is_symlink = metadata.file_type().is_symlink();

    // Time values
    let mtime_ms = metadata
        .modified()
        .map(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as f64
        })
        .unwrap_or(0.0);
    let atime_ms = metadata
        .accessed()
        .map(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as f64
        })
        .unwrap_or(0.0);
    let ctime_ms = metadata
        .created()
        .map(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as f64
        })
        .unwrap_or(mtime_ms);

    stats.set(
        js_string!("size"),
        JsValue::from(size as f64),
        false,
        context,
    )?;
    stats.set(
        js_string!("_isFile"),
        JsValue::from(is_file),
        false,
        context,
    )?;
    stats.set(
        js_string!("_isDirectory"),
        JsValue::from(is_dir),
        false,
        context,
    )?;
    stats.set(
        js_string!("_isSymbolicLink"),
        JsValue::from(is_symlink),
        false,
        context,
    )?;
    stats.set(
        js_string!("mtimeMs"),
        JsValue::from(mtime_ms),
        false,
        context,
    )?;
    stats.set(
        js_string!("atimeMs"),
        JsValue::from(atime_ms),
        false,
        context,
    )?;
    stats.set(
        js_string!("ctimeMs"),
        JsValue::from(ctime_ms),
        false,
        context,
    )?;
    stats.set(
        js_string!("birthtimeMs"),
        JsValue::from(ctime_ms),
        false,
        context,
    )?;
    stats.set(js_string!("dev"), JsValue::from(0), false, context)?;
    stats.set(js_string!("ino"), JsValue::from(0), false, context)?;
    stats.set(js_string!("mode"), JsValue::from(0o644), false, context)?;
    stats.set(js_string!("nlink"), JsValue::from(1), false, context)?;
    stats.set(js_string!("uid"), JsValue::from(0), false, context)?;
    stats.set(js_string!("gid"), JsValue::from(0), false, context)?;
    stats.set(js_string!("rdev"), JsValue::from(0), false, context)?;
    stats.set(js_string!("blksize"), JsValue::from(4096), false, context)?;
    stats.set(
        js_string!("blocks"),
        JsValue::from((size / 512) as i32),
        false,
        context,
    )?;

    // Add methods via JavaScript evaluation
    let methods_code = r#"
        (function(obj) {
            obj.isFile = function() { return obj._isFile; };
            obj.isDirectory = function() { return obj._isDirectory; };
            obj.isSymbolicLink = function() { return obj._isSymbolicLink; };
            obj.isBlockDevice = function() { return false; };
            obj.isCharacterDevice = function() { return false; };
            obj.isFIFO = function() { return false; };
            obj.isSocket = function() { return false; };
            obj.mtime = new Date(obj.mtimeMs);
            obj.atime = new Date(obj.atimeMs);
            obj.ctime = new Date(obj.ctimeMs);
            obj.birthtime = new Date(obj.birthtimeMs);
            return obj;
        })
    "#;

    let wrapper_fn = context.eval(Source::from_bytes(methods_code.as_bytes()))?;
    if let Some(fn_obj) = wrapper_fn.as_object() {
        return fn_obj.call(&JsValue::undefined(), &[stats.into()], context);
    }

    Ok(stats.into())
}
