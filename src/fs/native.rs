//! Native Rust implementation of high-performance file system API
//!
//! Implements Bun-style fs API using Boa's promise system and tokio for async I/O

use boa_engine::{
    job::NativeJob,
    js_string, Context, JsArgs, JsError, JsNativeError, JsObject, JsResult, JsValue,
    NativeFunction, Source,
};
use boa_gc::{Finalize, Trace};
use std::path::PathBuf;
use std::sync::Arc;

/// Register the file system API in the JavaScript context
pub fn register_file_system(context: &mut Context) -> JsResult<()> {
    // Register the JavaScript polyfill that creates a clean API
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
                    css: 'text/css;charset=utf-8',
                };
                return types[ext] || 'text/plain;charset=utf-8';
            }

            text() {
                return __viper_read_text(this.path);
            }

            bytes() {
                return __viper_read_bytes(this.path);
            }

            async json() {
                const text = await this.text();
                return JSON.parse(text);
            }

            exists() {
                return __viper_exists(this.path);
            }

            async size() {
                const stats = await __viper_stat(this.path);
                return stats.size;
            }

            delete() {
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

    // Register native functions
    register_native_functions(context)?;

    Ok(())
}

/// Register native file system functions
fn register_native_functions(context: &mut Context) -> JsResult<()> {
    // __viper_read_text - Read file as text
    let read_text = NativeFunction::from_fn_ptr(|_this, args, context| {
        let path = args.get_or_undefined(0).to_string(context)?.to_std_string_escaped();

        // Create a promise
        let (promise, resolvers) = JsObject::promise(context);

        // Spawn async task
        let resolve = resolvers.resolve.clone();
        let reject = resolvers.reject.clone();

        let job = NativeJob::new(move |context| {
            // Execute async operation
            let runtime = tokio::runtime::Runtime::new().unwrap();
            match runtime.block_on(tokio::fs::read_to_string(&path)) {
                Ok(content) => {
                    let js_string = JsValue::from(js_string!(content));
                    resolve.call(&JsValue::undefined(), &[js_string], context)?;
                }
                Err(e) => {
                    let error = JsError::from_native(
                        JsNativeError::error().with_message(format!("Failed to read file: {}", e))
                    );
                    reject.call(&JsValue::undefined(), &[error.to_opaque(context)], context)?;
                }
            }
            Ok(())
        });

        context.job_queue().enqueue_promise_job(job, context);

        Ok(promise.into())
    });

    context.global_object().set(
        js_string!("__viper_read_text"),
        read_text,
        false,
        context,
    )?;

    // __viper_read_bytes - Read file as bytes
    let read_bytes = NativeFunction::from_fn_ptr(|_this, args, context| {
        let path = args.get_or_undefined(0).to_string(context)?.to_std_string_escaped();

        let (promise, resolvers) = JsObject::promise(context);
        let resolve = resolvers.resolve.clone();
        let reject = resolvers.reject.clone();

        let job = NativeJob::new(move |context| {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            match runtime.block_on(tokio::fs::read(&path)) {
                Ok(bytes) => {
                    // Create Uint8Array from bytes
                    let array = context.intrinsics().constructors().uint8_array().construct(
                        &[JsValue::from(bytes.len())],
                        None,
                        context,
                    )?;

                    // Copy bytes into the array
                    if let Some(obj) = array.as_object() {
                        for (i, &byte) in bytes.iter().enumerate() {
                            obj.set(i, JsValue::from(byte), true, context)?;
                        }
                    }

                    resolve.call(&JsValue::undefined(), &[array.into()], context)?;
                }
                Err(e) => {
                    let error = JsError::from_native(
                        JsNativeError::error().with_message(format!("Failed to read file: {}", e))
                    );
                    reject.call(&JsValue::undefined(), &[error.to_opaque(context)], context)?;
                }
            }
            Ok(())
        });

        context.job_queue().enqueue_promise_job(job, context);
        Ok(promise.into())
    });

    context.global_object().set(
        js_string!("__viper_read_bytes"),
        read_bytes,
        false,
        context,
    )?;

    // __viper_exists - Check if file exists
    let exists = NativeFunction::from_fn_ptr(|_this, args, context| {
        let path = args.get_or_undefined(0).to_string(context)?.to_std_string_escaped();

        let (promise, resolvers) = JsObject::promise(context);
        let resolve = resolvers.resolve.clone();

        let job = NativeJob::new(move |context| {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            let exists = runtime.block_on(tokio::fs::metadata(&path)).is_ok();
            resolve.call(&JsValue::undefined(), &[JsValue::from(exists)], context)?;
            Ok(())
        });

        context.job_queue().enqueue_promise_job(job, context);
        Ok(promise.into())
    });

    context.global_object().set(
        js_string!("__viper_exists"),
        exists,
        false,
        context,
    )?;

    // __viper_stat - Get file stats
    let stat = NativeFunction::from_fn_ptr(|_this, args, context| {
        let path = args.get_or_undefined(0).to_string(context)?.to_std_string_escaped();

        let (promise, resolvers) = JsObject::promise(context);
        let resolve = resolvers.resolve.clone();
        let reject = resolvers.reject.clone();

        let job = NativeJob::new(move |context| {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            match runtime.block_on(tokio::fs::metadata(&path)) {
                Ok(metadata) => {
                    let stats = JsObject::with_object_proto(context.intrinsics());
                    stats.set(js_string!("size"), JsValue::from(metadata.len() as f64), false, context)?;
                    stats.set(js_string!("isFile"), JsValue::from(metadata.is_file()), false, context)?;
                    stats.set(js_string!("isDirectory"), JsValue::from(metadata.is_dir()), false, context)?;

                    resolve.call(&JsValue::undefined(), &[stats.into()], context)?;
                }
                Err(e) => {
                    let error = JsError::from_native(
                        JsNativeError::error().with_message(format!("Failed to stat file: {}", e))
                    );
                    reject.call(&JsValue::undefined(), &[error.to_opaque(context)], context)?;
                }
            }
            Ok(())
        });

        context.job_queue().enqueue_promise_job(job, context);
        Ok(promise.into())
    });

    context.global_object().set(
        js_string!("__viper_stat"),
        stat,
        false,
        context,
    )?;

    // __viper_write - Write bytes to file
    let write = NativeFunction::from_fn_ptr(|_this, args, context| {
        let path = args.get_or_undefined(0).to_string(context)?.to_std_string_escaped();
        let data_arg = args.get_or_undefined(1);

        // Extract bytes from Uint8Array
        let mut bytes = Vec::new();
        if let Some(obj) = data_arg.as_object() {
            // Try to get the length property
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

        let (promise, resolvers) = JsObject::promise(context);
        let resolve = resolvers.resolve.clone();
        let reject = resolvers.reject.clone();

        let job = NativeJob::new(move |context| {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            match runtime.block_on(tokio::fs::write(&path, &bytes)) {
                Ok(_) => {
                    resolve.call(&JsValue::undefined(), &[JsValue::from(bytes.len() as f64)], context)?;
                }
                Err(e) => {
                    let error = JsError::from_native(
                        JsNativeError::error().with_message(format!("Failed to write file: {}", e))
                    );
                    reject.call(&JsValue::undefined(), &[error.to_opaque(context)], context)?;
                }
            }
            Ok(())
        });

        context.job_queue().enqueue_promise_job(job, context);
        Ok(promise.into())
    });

    context.global_object().set(
        js_string!("__viper_write"),
        write,
        false,
        context,
    )?;

    // __viper_append - Append bytes to file
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

        let (promise, resolvers) = JsObject::promise(context);
        let resolve = resolvers.resolve.clone();
        let reject = resolvers.reject.clone();

        let job = NativeJob::new(move |context| {
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
                Ok(_) => {
                    resolve.call(&JsValue::undefined(), &[JsValue::undefined()], context)?;
                }
                Err(e) => {
                    let error = JsError::from_native(
                        JsNativeError::error().with_message(format!("Failed to append to file: {}", e))
                    );
                    reject.call(&JsValue::undefined(), &[error.to_opaque(context)], context)?;
                }
            }
            Ok(())
        });

        context.job_queue().enqueue_promise_job(job, context);
        Ok(promise.into())
    });

    context.global_object().set(
        js_string!("__viper_append"),
        append,
        false,
        context,
    )?;

    // __viper_delete - Delete file
    let delete = NativeFunction::from_fn_ptr(|_this, args, context| {
        let path = args.get_or_undefined(0).to_string(context)?.to_std_string_escaped();

        let (promise, resolvers) = JsObject::promise(context);
        let resolve = resolvers.resolve.clone();
        let reject = resolvers.reject.clone();

        let job = NativeJob::new(move |context| {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            match runtime.block_on(tokio::fs::remove_file(&path)) {
                Ok(_) => {
                    resolve.call(&JsValue::undefined(), &[JsValue::undefined()], context)?;
                }
                Err(e) => {
                    let error = JsError::from_native(
                        JsNativeError::error().with_message(format!("Failed to delete file: {}", e))
                    );
                    reject.call(&JsValue::undefined(), &[error.to_opaque(context)], context)?;
                }
            }
            Ok(())
        });

        context.job_queue().enqueue_promise_job(job, context);
        Ok(promise.into())
    });

    context.global_object().set(
        js_string!("__viper_delete"),
        delete,
        false,
        context,
    )?;

    // __viper_copy - Copy file
    let copy = NativeFunction::from_fn_ptr(|_this, args, context| {
        let src = args.get_or_undefined(0).to_string(context)?.to_std_string_escaped();
        let dest = args.get_or_undefined(1).to_string(context)?.to_std_string_escaped();

        let (promise, resolvers) = JsObject::promise(context);
        let resolve = resolvers.resolve.clone();
        let reject = resolvers.reject.clone();

        let job = NativeJob::new(move |context| {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            match runtime.block_on(tokio::fs::copy(&src, &dest)) {
                Ok(bytes) => {
                    resolve.call(&JsValue::undefined(), &[JsValue::from(bytes as f64)], context)?;
                }
                Err(e) => {
                    let error = JsError::from_native(
                        JsNativeError::error().with_message(format!("Failed to copy file: {}", e))
                    );
                    reject.call(&JsValue::undefined(), &[error.to_opaque(context)], context)?;
                }
            }
            Ok(())
        });

        context.job_queue().enqueue_promise_job(job, context);
        Ok(promise.into())
    });

    context.global_object().set(
        js_string!("__viper_copy"),
        copy,
        false,
        context,
    )?;

    Ok(())
}
