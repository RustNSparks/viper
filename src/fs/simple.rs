//! Simple high-performance file system API using blocking operations
//!
//! This uses blocking operations at the native layer (with tokio runtime)
//! and returns promises from JavaScript

use boa_engine::{
    js_string, Context, JsArgs, JsError, JsNativeError, JsResult, JsValue,
    NativeFunction, Source,
};

/// Register the file system API
pub fn register_file_system(context: &mut Context) -> JsResult<()> {
    // Create tokio runtime for async operations
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| JsError::from_native(
            JsNativeError::error().with_message(format!("Failed to create tokio runtime: {}", e))
        ))?;

    // Store runtime handle globally (this is safe since we only create one runtime)
    let handle = runtime.handle().clone();
    std::mem::forget(runtime); // Keep runtime alive for the program duration

    // Register JavaScript polyfill
    let fs_polyfill = r#"
        // High-performance file system API

        class ViperFile {
            constructor(path, options = {}) {
                this.path = path;
                this.type = options.type || this._guessMimeType(path);
            }

            _guessMimeType(path) {
                const ext = path.split('.').pop()?.toLowerCase() || '';
                const types = {
                    txt: 'text/plain;charset=utf-8',
                    json: 'application/json;charset=utf-8',
                    html: 'text/html;charset=utf-8',
                    js: 'text/javascript;charset=utf-8',
                    ts: 'text/typescript;charset=utf-8',
                };
                return types[ext] || 'text/plain;charset=utf-8';
            }

            async text() {
                return __viper_read_text(this.path);
            }

            async json() {
                const text = await this.text();
                return JSON.parse(text);
            }

            async exists() {
                return __viper_exists(this.path);
            }

            async size() {
                return __viper_size(this.path);
            }

            async delete() {
                return __viper_delete(this.path);
            }

            writer(options = {}) {
                return new ViperFileSink(this.path, options);
            }
        }

        class ViperFileSink {
            constructor(path, options = {}) {
                this.path = path;
                this.highWaterMark = options.highWaterMark || 16384;
                this.buffer = [];
                this.bufferSize = 0;
                this.closed = false;
                this.bytesWritten = 0;
            }

            write(chunk) {
                if (this.closed) throw new Error('FileSink is closed');

                let data;
                if (typeof chunk === 'string') {
                    data = new TextEncoder().encode(chunk);
                } else if (chunk instanceof ArrayBuffer) {
                    data = new Uint8Array(chunk);
                } else if (ArrayBuffer.isView(chunk)) {
                    data = new Uint8Array(chunk.buffer, chunk.byteOffset, chunk.byteLength);
                } else {
                    throw new TypeError('Invalid chunk type');
                }

                this.buffer.push(data);
                this.bufferSize += data.length;

                if (this.bufferSize >= this.highWaterMark) {
                    return this.flush();
                }

                return data.length;
            }

            async flush() {
                if (this.buffer.length === 0) return 0;

                const totalSize = this.bufferSize;
                const combined = new Uint8Array(totalSize);
                let offset = 0;
                for (const chunk of this.buffer) {
                    combined.set(chunk, offset);
                    offset += chunk.length;
                }

                await __viper_append(this.path, combined);

                this.bytesWritten += totalSize;
                this.buffer = [];
                this.bufferSize = 0;

                return totalSize;
            }

            async end(error) {
                if (this.closed) return this.bytesWritten;
                if (error) {
                    this.buffer = [];
                    this.closed = true;
                    throw error;
                }
                await this.flush();
                this.closed = true;
                return this.bytesWritten;
            }
        }

        globalThis.file = (path, options) => new ViperFile(path, options);

        globalThis.write = async (destination, data) => {
            let path = typeof destination === 'string' ? destination : destination.path;

            if (typeof data === 'string') {
                return __viper_write(path, new TextEncoder().encode(data));
            } else if (data instanceof ViperFile) {
                return __viper_copy(data.path, path);
            } else if (data instanceof ArrayBuffer) {
                return __viper_write(path, new Uint8Array(data));
            } else if (ArrayBuffer.isView(data)) {
                return __viper_write(path, new Uint8Array(data.buffer, data.byteOffset, data.byteLength));
            }

            throw new TypeError('Invalid data type');
        };
    "#;

    let source = Source::from_bytes(fs_polyfill.as_bytes());
    context.eval(source)?;

    // Register native functions that return promises via setTimeout trick
    register_native_functions(context, handle)?;

    Ok(())
}

/// Register native file system functions
fn register_native_functions(context: &mut Context, _handle: tokio::runtime::Handle) -> JsResult<()> {
    // Since Boa 0.21 doesn't expose easy promise creation from native code,
    // we'll make these synchronous but they'll be called from async JS functions

    // __viper_read_text
    let read_text = NativeFunction::from_fn_ptr(|_this, args, context| {
        let path = args.get_or_undefined(0).to_string(context)?.to_std_string_escaped();

        // Block on async operation
        let runtime = tokio::runtime::Runtime::new().unwrap();
        match runtime.block_on(tokio::fs::read_to_string(&path)) {
            Ok(content) => Ok(JsValue::from(js_string!(content))),
            Err(e) => Err(JsError::from_native(
                JsNativeError::error().with_message(format!("Failed to read file: {}", e))
            )),
        }
    });

    context.global_object().set(
        js_string!("__viper_read_text"),
        read_text.to_js_function(context.realm()),
        false,
        context,
    )?;

    // __viper_exists
    let exists = NativeFunction::from_fn_ptr(|_this, args, context| {
        let path = args.get_or_undefined(0).to_string(context)?.to_std_string_escaped();

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let exists = runtime.block_on(tokio::fs::metadata(&path)).is_ok();
        Ok(JsValue::from(exists))
    });

    context.global_object().set(
        js_string!("__viper_exists"),
        exists.to_js_function(context.realm()),
        false,
        context,
    )?;

    // __viper_size
    let size = NativeFunction::from_fn_ptr(|_this, args, context| {
        let path = args.get_or_undefined(0).to_string(context)?.to_std_string_escaped();

        let runtime = tokio::runtime::Runtime::new().unwrap();
        match runtime.block_on(tokio::fs::metadata(&path)) {
            Ok(metadata) => Ok(JsValue::from(metadata.len() as f64)),
            Err(e) => Err(JsError::from_native(
                JsNativeError::error().with_message(format!("Failed to get file size: {}", e))
            )),
        }
    });

    context.global_object().set(
        js_string!("__viper_size"),
        size.to_js_function(context.realm()),
        false,
        context,
    )?;

    // __viper_write
    let write = NativeFunction::from_fn_ptr(|_this, args, context| {
        let path = args.get_or_undefined(0).to_string(context)?.to_std_string_escaped();
        let data_arg = args.get_or_undefined(1);

        // Extract bytes from Uint8Array
        let mut bytes = Vec::new();
        if let Some(obj) = data_arg.as_object() {
            if let Ok(len_val) = obj.get(js_string!("length"), context) {
                if let Some(len) = len_val.as_number() {
                    for i in 0..(len as usize) {
                        if let Ok(byte_val) = obj.get(i, context) {
                            if let Some(byte) = byte_val.as_number() {
                                bytes.push(byte as u8);
                            }
                        }
                    }
                }
            }
        }

        let runtime = tokio::runtime::Runtime::new().unwrap();
        match runtime.block_on(tokio::fs::write(&path, &bytes)) {
            Ok(_) => Ok(JsValue::from(bytes.len() as f64)),
            Err(e) => Err(JsError::from_native(
                JsNativeError::error().with_message(format!("Failed to write file: {}", e))
            )),
        }
    });

    context.global_object().set(
        js_string!("__viper_write"),
        write.to_js_function(context.realm()),
        false,
        context,
    )?;

    // __viper_append
    let append = NativeFunction::from_fn_ptr(|_this, args, context| {
        let path = args.get_or_undefined(0).to_string(context)?.to_std_string_escaped();
        let data_arg = args.get_or_undefined(1);

        let mut bytes = Vec::new();
        if let Some(obj) = data_arg.as_object() {
            if let Ok(len_val) = obj.get(js_string!("length"), context) {
                if let Some(len) = len_val.as_number() {
                    for i in 0..(len as usize) {
                        if let Ok(byte_val) = obj.get(i, context) {
                            if let Some(byte) = byte_val.as_number() {
                                bytes.push(byte as u8);
                            }
                        }
                    }
                }
            }
        }

        let runtime = tokio::runtime::Runtime::new().unwrap();
        use tokio::io::AsyncWriteExt;
        let result = runtime.block_on(async {
            let mut file = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .await?;
            file.write_all(&bytes).await?;
            Ok::<(), std::io::Error>(())
        });

        match result {
            Ok(_) => Ok(JsValue::undefined()),
            Err(e) => Err(JsError::from_native(
                JsNativeError::error().with_message(format!("Failed to append to file: {}", e))
            )),
        }
    });

    context.global_object().set(
        js_string!("__viper_append"),
        append.to_js_function(context.realm()),
        false,
        context,
    )?;

    // __viper_delete
    let delete = NativeFunction::from_fn_ptr(|_this, args, context| {
        let path = args.get_or_undefined(0).to_string(context)?.to_std_string_escaped();

        let runtime = tokio::runtime::Runtime::new().unwrap();
        match runtime.block_on(tokio::fs::remove_file(&path)) {
            Ok(_) => Ok(JsValue::undefined()),
            Err(e) => Err(JsError::from_native(
                JsNativeError::error().with_message(format!("Failed to delete file: {}", e))
            )),
        }
    });

    context.global_object().set(
        js_string!("__viper_delete"),
        delete.to_js_function(context.realm()),
        false,
        context,
    )?;

    // __viper_copy
    let copy = NativeFunction::from_fn_ptr(|_this, args, context| {
        let src = args.get_or_undefined(0).to_string(context)?.to_std_string_escaped();
        let dest = args.get_or_undefined(1).to_string(context)?.to_std_string_escaped();

        let runtime = tokio::runtime::Runtime::new().unwrap();
        match runtime.block_on(tokio::fs::copy(&src, &dest)) {
            Ok(bytes) => Ok(JsValue::from(bytes as f64)),
            Err(e) => Err(JsError::from_native(
                JsNativeError::error().with_message(format!("Failed to copy file: {}", e))
            )),
        }
    });

    context.global_object().set(
        js_string!("__viper_copy"),
        copy.to_js_function(context.realm()),
        false,
        context,
    )?;

    Ok(())
}
