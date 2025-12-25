//! High-performance file system runtime integration for JavaScript
//!
//! Provides Bun-style fs API integrated with Boa's promise system

use boa_engine::{
    js_string, Context, JsArgs, JsError, JsNativeError, JsObject, JsResult, JsString, JsValue,
    NativeFunction, Source,
};
use std::path::Path;

/// Register the complete file system API in the runtime
pub fn register_file_system(context: &mut Context) -> JsResult<()> {
    // Inject the file system API as a JavaScript polyfill
    // This gives us better integration with promises and async/await
    let fs_api = r#"
        // High-performance file system API for Viper

        // File reference class (lazy-loaded)
        class ViperFile {
            constructor(path, options = {}) {
                this.path = path;
                this.type = options.type || this._guessMimeType(path);
                this._size = null;
            }

            _guessMimeType(path) {
                const ext = path.split('.').pop().toLowerCase();
                const mimeTypes = {
                    'txt': 'text/plain;charset=utf-8',
                    'json': 'application/json;charset=utf-8',
                    'html': 'text/html;charset=utf-8',
                    'js': 'text/javascript;charset=utf-8',
                    'mjs': 'text/javascript;charset=utf-8',
                    'ts': 'text/typescript;charset=utf-8',
                    'tsx': 'text/typescript;charset=utf-8',
                    'css': 'text/css;charset=utf-8',
                    'png': 'image/png',
                    'jpg': 'image/jpeg',
                    'jpeg': 'image/jpeg',
                    'svg': 'image/svg+xml',
                    'pdf': 'application/pdf',
                    'wasm': 'application/wasm',
                };
                return mimeTypes[ext] || 'text/plain;charset=utf-8';
            }

            async text() {
                return await globalThis.__viper_fs_read_text(this.path);
            }

            async bytes() {
                return await globalThis.__viper_fs_read_bytes(this.path);
            }

            async json() {
                const text = await this.text();
                return JSON.parse(text);
            }

            async exists() {
                return await globalThis.__viper_fs_exists(this.path);
            }

            async size() {
                if (this._size === null) {
                    this._size = await globalThis.__viper_fs_size(this.path);
                }
                return this._size;
            }

            async delete() {
                return await globalThis.__viper_fs_delete(this.path);
            }

            writer(options = {}) {
                return new ViperFileSink(this.path, options);
            }

            stream() {
                // TODO: Implement ReadableStream
                throw new Error('stream() not yet implemented');
            }

            async arrayBuffer() {
                const bytes = await this.bytes();
                return bytes.buffer;
            }
        }

        // Incremental file writer
        class ViperFileSink {
            constructor(path, options = {}) {
                this.path = path;
                this.highWaterMark = options.highWaterMark || 16384; // 16KB
                this.buffer = [];
                this.bufferSize = 0;
                this.closed = false;
                this.bytesWritten = 0;
            }

            write(chunk) {
                if (this.closed) {
                    throw new Error('FileSink is closed');
                }

                let data;
                if (typeof chunk === 'string') {
                    data = new TextEncoder().encode(chunk);
                } else if (chunk instanceof ArrayBuffer) {
                    data = new Uint8Array(chunk);
                } else if (ArrayBuffer.isView(chunk)) {
                    data = new Uint8Array(chunk.buffer, chunk.byteOffset, chunk.byteLength);
                } else {
                    throw new TypeError('Chunk must be string, ArrayBuffer, or TypedArray');
                }

                this.buffer.push(data);
                this.bufferSize += data.length;

                // Auto-flush if we exceed high water mark
                if (this.bufferSize >= this.highWaterMark) {
                    return this.flush();
                }

                return data.length;
            }

            async flush() {
                if (this.buffer.length === 0) {
                    return 0;
                }

                // Combine all buffered chunks
                const totalSize = this.bufferSize;
                const combined = new Uint8Array(totalSize);
                let offset = 0;
                for (const chunk of this.buffer) {
                    combined.set(chunk, offset);
                    offset += chunk.length;
                }

                // Write to disk
                await globalThis.__viper_fs_append(this.path, combined);

                this.bytesWritten += totalSize;
                this.buffer = [];
                this.bufferSize = 0;

                return totalSize;
            }

            async end(error) {
                if (this.closed) {
                    return this.bytesWritten;
                }

                if (error) {
                    this.buffer = [];
                    this.bufferSize = 0;
                    this.closed = true;
                    throw error;
                }

                await this.flush();
                this.closed = true;
                return this.bytesWritten;
            }

            ref() {
                // TODO: Implement keep-alive reference
            }

            unref() {
                // TODO: Implement unreference
            }
        }

        // Main API
        globalThis.file = function(path, options) {
            return new ViperFile(path, options);
        };

        globalThis.write = async function(destination, data) {
            let path;

            // Handle different destination types
            if (typeof destination === 'string') {
                path = destination;
            } else if (destination instanceof ViperFile) {
                path = destination.path;
            } else if (destination && destination.path) {
                path = destination.path;
            } else {
                throw new TypeError('Invalid destination');
            }

            // Handle different data types
            let bytes;
            if (typeof data === 'string') {
                bytes = new TextEncoder().encode(data);
            } else if (data instanceof ViperFile) {
                // Copy file
                return await globalThis.__viper_fs_copy(data.path, path);
            } else if (data instanceof ArrayBuffer) {
                bytes = new Uint8Array(data);
            } else if (ArrayBuffer.isView(data)) {
                bytes = new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
            } else if (data && typeof data.text === 'function') {
                // Response or Blob
                const text = await data.text();
                bytes = new TextEncoder().encode(text);
            } else {
                throw new TypeError('Invalid data type');
            }

            return await globalThis.__viper_fs_write(path, bytes);
        };

        // Convenience: stdin, stdout, stderr
        globalThis.stdin = new ViperFile('/dev/stdin', { type: 'text/plain' });
        globalThis.stdout = new ViperFile('/dev/stdout', { type: 'text/plain' });
        globalThis.stderr = new ViperFile('/dev/stderr', { type: 'text/plain' });
    "#;

    let source = Source::from_bytes(fs_api.as_bytes());
    context.eval(source).map_err(|e| {
        JsError::from_native(JsNativeError::error().with_message(format!("Failed to load fs API: {}", e)))
    })?;

    // Register native implementations
    register_native_fs_functions(context)?;

    Ok(())
}

/// Register native file system functions that are called from JavaScript
fn register_native_fs_functions(context: &mut Context) -> JsResult<()> {
    // __viper_fs_read_text
    let read_text_fn = NativeFunction::from_async_fn(|_this, args, context| {
        Box::pin(async move {
            let path_arg = args.get_or_undefined(0);
            let path = path_arg.to_string(context)?.to_std_string_escaped();

            match tokio::fs::read_to_string(&path).await {
                Ok(content) => Ok(JsValue::from(js_string!(content))),
                Err(e) => Err(JsError::from_native(
                    JsNativeError::error().with_message(format!("Failed to read file: {}", e))
                )),
            }
        })
    });
    context.global_object().set(
        js_string!("__viper_fs_read_text"),
        read_text_fn,
        false,
        context,
    )?;

    // __viper_fs_read_bytes
    let read_bytes_fn = NativeFunction::from_async_fn(|_this, args, context| {
        Box::pin(async move {
            let path_arg = args.get_or_undefined(0);
            let path = path_arg.to_string(context)?.to_std_string_escaped();

            match tokio::fs::read(&path).await {
                Ok(bytes) => {
                    // Create Uint8Array
                    let uint8_array = context.intrinsics().constructors().uint8_array();
                    let array_buffer = boa_engine::builtins::array_buffer::ArrayBuffer::from_byte_block(
                        bytes,
                        context,
                    )?;
                    let uint8 = uint8_array.construct(
                        &[JsValue::from(array_buffer)],
                        Some(&uint8_array.clone().into()),
                        context,
                    )?;
                    Ok(uint8.into())
                }
                Err(e) => Err(JsError::from_native(
                    JsNativeError::error().with_message(format!("Failed to read file: {}", e))
                )),
            }
        })
    });
    context.global_object().set(
        js_string!("__viper_fs_read_bytes"),
        read_bytes_fn,
        false,
        context,
    )?;

    // __viper_fs_exists
    let exists_fn = NativeFunction::from_async_fn(|_this, args, context| {
        Box::pin(async move {
            let path_arg = args.get_or_undefined(0);
            let path = path_arg.to_string(context)?.to_std_string_escaped();

            let exists = tokio::fs::metadata(&path).await.is_ok();
            Ok(JsValue::from(exists))
        })
    });
    context.global_object().set(
        js_string!("__viper_fs_exists"),
        exists_fn,
        false,
        context,
    )?;

    // __viper_fs_size
    let size_fn = NativeFunction::from_async_fn(|_this, args, context| {
        Box::pin(async move {
            let path_arg = args.get_or_undefined(0);
            let path = path_arg.to_string(context)?.to_std_string_escaped();

            match tokio::fs::metadata(&path).await {
                Ok(metadata) => Ok(JsValue::from(metadata.len() as f64)),
                Err(e) => Err(JsError::from_native(
                    JsNativeError::error().with_message(format!("Failed to get file size: {}", e))
                )),
            }
        })
    });
    context.global_object().set(
        js_string!("__viper_fs_size"),
        size_fn,
        false,
        context,
    )?;

    // __viper_fs_delete
    let delete_fn = NativeFunction::from_async_fn(|_this, args, context| {
        Box::pin(async move {
            let path_arg = args.get_or_undefined(0);
            let path = path_arg.to_std_string_escaped();

            match tokio::fs::remove_file(&path).await {
                Ok(_) => Ok(JsValue::undefined()),
                Err(e) => Err(JsError::from_native(
                    JsNativeError::error().with_message(format!("Failed to delete file: {}", e))
                )),
            }
        })
    });
    context.global_object().set(
        js_string!("__viper_fs_delete"),
        delete_fn,
        false,
        context,
    )?;

    // __viper_fs_write
    let write_fn = NativeFunction::from_async_fn(|_this, args, context| {
        Box::pin(async move {
            let path_arg = args.get_or_undefined(0);
            let path = path_arg.to_string(context)?.to_std_string_escaped();

            let data_arg = args.get_or_undefined(1);

            // Extract bytes from Uint8Array
            let bytes = if let Some(obj) = data_arg.as_object() {
                if let Some(typed_array) = obj.downcast_ref::<boa_engine::builtins::typed_array::Uint8Array>() {
                    typed_array.iter(context).collect::<Result<Vec<_>, _>>()?
                } else {
                    return Err(JsError::from_native(
                        JsNativeError::typ().with_message("Expected Uint8Array")
                    ));
                }
            } else {
                return Err(JsError::from_native(
                    JsNativeError::typ().with_message("Expected Uint8Array")
                ));
            };

            match tokio::fs::write(&path, &bytes).await {
                Ok(_) => Ok(JsValue::from(bytes.len() as f64)),
                Err(e) => Err(JsError::from_native(
                    JsNativeError::error().with_message(format!("Failed to write file: {}", e))
                )),
            }
        })
    });
    context.global_object().set(
        js_string!("__viper_fs_write"),
        write_fn,
        false,
        context,
    )?;

    // __viper_fs_append
    let append_fn = NativeFunction::from_async_fn(|_this, args, context| {
        Box::pin(async move {
            let path_arg = args.get_or_undefined(0);
            let path = path_arg.to_string(context)?.to_std_string_escaped();

            let data_arg = args.get_or_undefined(1);

            // Extract bytes
            let bytes = if let Some(obj) = data_arg.as_object() {
                if let Some(typed_array) = obj.downcast_ref::<boa_engine::builtins::typed_array::Uint8Array>() {
                    typed_array.iter(context).collect::<Result<Vec<_>, _>>()?
                } else {
                    return Err(JsError::from_native(
                        JsNativeError::typ().with_message("Expected Uint8Array")
                    ));
                }
            } else {
                return Err(JsError::from_native(
                    JsNativeError::typ().with_message("Expected Uint8Array")
                ));
            };

            // Append to file
            use tokio::io::AsyncWriteExt;
            let mut file = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .await
                .map_err(|e| JsError::from_native(
                    JsNativeError::error().with_message(format!("Failed to open file: {}", e))
                ))?;

            file.write_all(&bytes).await.map_err(|e| JsError::from_native(
                JsNativeError::error().with_message(format!("Failed to append to file: {}", e))
            ))?;

            Ok(JsValue::undefined())
        })
    });
    context.global_object().set(
        js_string!("__viper_fs_append"),
        append_fn,
        false,
        context,
    )?;

    // __viper_fs_copy
    let copy_fn = NativeFunction::from_async_fn(|_this, args, context| {
        Box::pin(async move {
            let src_arg = args.get_or_undefined(0);
            let src = src_arg.to_string(context)?.to_std_string_escaped();

            let dest_arg = args.get_or_undefined(1);
            let dest = dest_arg.to_string(context)?.to_std_string_escaped();

            match tokio::fs::copy(&src, &dest).await {
                Ok(bytes) => Ok(JsValue::from(bytes as f64)),
                Err(e) => Err(JsError::from_native(
                    JsNativeError::error().with_message(format!("Failed to copy file: {}", e))
                )),
            }
        })
    });
    context.global_object().set(
        js_string!("__viper_fs_copy"),
        copy_fn,
        false,
        context,
    )?;

    Ok(())
}
