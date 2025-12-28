//! Zlib Module - Node.js compatible compression/decompression
//!
//! Uses flate2 with zlib-rs backend for maximum performance.
//! Provides:
//! - zlib.gzipSync(buffer) / zlib.gzip(buffer, callback)
//! - zlib.gunzipSync(buffer) / zlib.gunzip(buffer, callback)
//! - zlib.deflateSync(buffer) / zlib.deflate(buffer, callback)
//! - zlib.inflateSync(buffer) / zlib.inflate(buffer, callback)
//! - zlib.deflateRawSync(buffer) / zlib.deflateRaw(buffer, callback)
//! - zlib.inflateRawSync(buffer) / zlib.inflateRaw(buffer, callback)
//! - zlib.unzipSync(buffer) / zlib.unzip(buffer, callback)
//! - zlib.crc32(data[, value])
//! - zlib.constants

use boa_engine::{
    Context, JsNativeError, JsResult, JsValue, NativeFunction, Source, js_string,
    object::builtins::JsUint8Array,
};
use flate2::{
    Compression,
    read::{DeflateDecoder, DeflateEncoder, GzDecoder, GzEncoder, ZlibDecoder, ZlibEncoder},
};
use std::io::Read;

/// Register the zlib module
pub fn register_zlib_module(context: &mut Context) -> JsResult<()> {
    register_native_zlib_functions(context)?;

    let zlib_code = r#"
        (function() {
            const zlib = {
                // Sync methods
                gzipSync: (buffer, options) => __viper_zlib_gzip_sync(buffer, options),
                gunzipSync: (buffer, options) => __viper_zlib_gunzip_sync(buffer, options),
                deflateSync: (buffer, options) => __viper_zlib_deflate_sync(buffer, options),
                inflateSync: (buffer, options) => __viper_zlib_inflate_sync(buffer, options),
                deflateRawSync: (buffer, options) => __viper_zlib_deflate_raw_sync(buffer, options),
                inflateRawSync: (buffer, options) => __viper_zlib_inflate_raw_sync(buffer, options),
                unzipSync: (buffer, options) => __viper_zlib_unzip_sync(buffer, options),

                // Async methods (callback-based)
                gzip: (buffer, optionsOrCallback, callback) => {
                    const opts = typeof optionsOrCallback === 'function' ? {} : optionsOrCallback;
                    const cb = typeof optionsOrCallback === 'function' ? optionsOrCallback : callback;
                    try {
                        const result = __viper_zlib_gzip_sync(buffer, opts);
                        if (cb) cb(null, result);
                    } catch (err) {
                        if (cb) cb(err);
                    }
                },
                gunzip: (buffer, optionsOrCallback, callback) => {
                    const opts = typeof optionsOrCallback === 'function' ? {} : optionsOrCallback;
                    const cb = typeof optionsOrCallback === 'function' ? optionsOrCallback : callback;
                    try {
                        const result = __viper_zlib_gunzip_sync(buffer, opts);
                        if (cb) cb(null, result);
                    } catch (err) {
                        if (cb) cb(err);
                    }
                },
                deflate: (buffer, optionsOrCallback, callback) => {
                    const opts = typeof optionsOrCallback === 'function' ? {} : optionsOrCallback;
                    const cb = typeof optionsOrCallback === 'function' ? optionsOrCallback : callback;
                    try {
                        const result = __viper_zlib_deflate_sync(buffer, opts);
                        if (cb) cb(null, result);
                    } catch (err) {
                        if (cb) cb(err);
                    }
                },
                inflate: (buffer, optionsOrCallback, callback) => {
                    const opts = typeof optionsOrCallback === 'function' ? {} : optionsOrCallback;
                    const cb = typeof optionsOrCallback === 'function' ? optionsOrCallback : callback;
                    try {
                        const result = __viper_zlib_inflate_sync(buffer, opts);
                        if (cb) cb(null, result);
                    } catch (err) {
                        if (cb) cb(err);
                    }
                },
                deflateRaw: (buffer, optionsOrCallback, callback) => {
                    const opts = typeof optionsOrCallback === 'function' ? {} : optionsOrCallback;
                    const cb = typeof optionsOrCallback === 'function' ? optionsOrCallback : callback;
                    try {
                        const result = __viper_zlib_deflate_raw_sync(buffer, opts);
                        if (cb) cb(null, result);
                    } catch (err) {
                        if (cb) cb(err);
                    }
                },
                inflateRaw: (buffer, optionsOrCallback, callback) => {
                    const opts = typeof optionsOrCallback === 'function' ? {} : optionsOrCallback;
                    const cb = typeof optionsOrCallback === 'function' ? optionsOrCallback : callback;
                    try {
                        const result = __viper_zlib_inflate_raw_sync(buffer, opts);
                        if (cb) cb(null, result);
                    } catch (err) {
                        if (cb) cb(err);
                    }
                },
                unzip: (buffer, optionsOrCallback, callback) => {
                    const opts = typeof optionsOrCallback === 'function' ? {} : optionsOrCallback;
                    const cb = typeof optionsOrCallback === 'function' ? optionsOrCallback : callback;
                    try {
                        const result = __viper_zlib_unzip_sync(buffer, opts);
                        if (cb) cb(null, result);
                    } catch (err) {
                        if (cb) cb(err);
                    }
                },

                // CRC32
                crc32: (data, value) => __viper_zlib_crc32(data, value),

                // Constants
                constants: {
                    // Flush values
                    Z_NO_FLUSH: 0,
                    Z_PARTIAL_FLUSH: 1,
                    Z_SYNC_FLUSH: 2,
                    Z_FULL_FLUSH: 3,
                    Z_FINISH: 4,
                    Z_BLOCK: 5,
                    Z_TREES: 6,

                    // Return codes
                    Z_OK: 0,
                    Z_STREAM_END: 1,
                    Z_NEED_DICT: 2,
                    Z_ERRNO: -1,
                    Z_STREAM_ERROR: -2,
                    Z_DATA_ERROR: -3,
                    Z_MEM_ERROR: -4,
                    Z_BUF_ERROR: -5,
                    Z_VERSION_ERROR: -6,

                    // Compression levels
                    Z_NO_COMPRESSION: 0,
                    Z_BEST_SPEED: 1,
                    Z_BEST_COMPRESSION: 9,
                    Z_DEFAULT_COMPRESSION: -1,

                    // Compression strategies
                    Z_FILTERED: 1,
                    Z_HUFFMAN_ONLY: 2,
                    Z_RLE: 3,
                    Z_FIXED: 4,
                    Z_DEFAULT_STRATEGY: 0,

                    // Data types
                    Z_BINARY: 0,
                    Z_TEXT: 1,
                    Z_ASCII: 1,
                    Z_UNKNOWN: 2,

                    // Deflate compression method
                    Z_DEFLATED: 8,

                    // Window bits
                    Z_MIN_WINDOWBITS: 8,
                    Z_MAX_WINDOWBITS: 15,
                    Z_DEFAULT_WINDOWBITS: 15,

                    // Memory level
                    Z_MIN_MEMLEVEL: 1,
                    Z_MAX_MEMLEVEL: 9,
                    Z_DEFAULT_MEMLEVEL: 8,

                    // Min/max chunk
                    Z_MIN_CHUNK: 64,
                    Z_MAX_CHUNK: Infinity,
                    Z_DEFAULT_CHUNK: 16384,

                    // Min/max level
                    Z_MIN_LEVEL: -1,
                    Z_MAX_LEVEL: 9,
                    Z_DEFAULT_LEVEL: -1,

                    // Brotli constants (for compatibility, not fully implemented)
                    BROTLI_OPERATION_PROCESS: 0,
                    BROTLI_OPERATION_FLUSH: 1,
                    BROTLI_OPERATION_FINISH: 2,
                    BROTLI_OPERATION_EMIT_METADATA: 3,
                    BROTLI_PARAM_MODE: 0,
                    BROTLI_MODE_GENERIC: 0,
                    BROTLI_MODE_TEXT: 1,
                    BROTLI_MODE_FONT: 2,
                    BROTLI_PARAM_QUALITY: 1,
                    BROTLI_MIN_QUALITY: 0,
                    BROTLI_MAX_QUALITY: 11,
                    BROTLI_DEFAULT_QUALITY: 11,
                    BROTLI_PARAM_LGWIN: 2,
                    BROTLI_MIN_WINDOW_BITS: 10,
                    BROTLI_MAX_WINDOW_BITS: 24,
                    BROTLI_LARGE_MAX_WINDOW_BITS: 30,
                    BROTLI_DEFAULT_WINDOW: 22,
                    BROTLI_PARAM_LGBLOCK: 3,
                    BROTLI_MIN_INPUT_BLOCK_BITS: 16,
                    BROTLI_MAX_INPUT_BLOCK_BITS: 24,
                    BROTLI_PARAM_DISABLE_LITERAL_CONTEXT_MODELING: 4,
                    BROTLI_PARAM_SIZE_HINT: 5,
                    BROTLI_PARAM_LARGE_WINDOW: 6,
                    BROTLI_PARAM_NPOSTFIX: 7,
                    BROTLI_PARAM_NDIRECT: 8,
                    BROTLI_DECODER_PARAM_DISABLE_RING_BUFFER_REALLOCATION: 0,
                    BROTLI_DECODER_PARAM_LARGE_WINDOW: 1,
                },
            };

            // Also expose constants at top level for compatibility
            Object.assign(zlib, zlib.constants);

            globalThis.zlib = zlib;
            return zlib;
        })();
    "#;

    let source = Source::from_bytes(zlib_code.as_bytes());
    context.eval(source)?;

    Ok(())
}

/// Register native zlib functions
fn register_native_zlib_functions(context: &mut Context) -> JsResult<()> {
    let global = context.global_object();

    // gzipSync
    let gzip_sync_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let input = get_buffer_data(args.get(0), context)?;
        let level = get_compression_level(args.get(1), context);

        let mut encoder = GzEncoder::new(&input[..], Compression::new(level));
        let mut output = Vec::new();

        encoder
            .read_to_end(&mut output)
            .map_err(|e| JsNativeError::error().with_message(format!("gzip error: {}", e)))?;

        create_buffer_from_vec(output, context)
    });
    global.set(
        js_string!("__viper_zlib_gzip_sync"),
        gzip_sync_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // gunzipSync
    let gunzip_sync_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let input = get_buffer_data(args.get(0), context)?;

        let mut decoder = GzDecoder::new(&input[..]);
        let mut output = Vec::new();

        decoder
            .read_to_end(&mut output)
            .map_err(|e| JsNativeError::error().with_message(format!("gunzip error: {}", e)))?;

        create_buffer_from_vec(output, context)
    });
    global.set(
        js_string!("__viper_zlib_gunzip_sync"),
        gunzip_sync_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // deflateSync (zlib format with header)
    let deflate_sync_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let input = get_buffer_data(args.get(0), context)?;
        let level = get_compression_level(args.get(1), context);

        let mut encoder = ZlibEncoder::new(&input[..], Compression::new(level));
        let mut output = Vec::new();

        encoder
            .read_to_end(&mut output)
            .map_err(|e| JsNativeError::error().with_message(format!("deflate error: {}", e)))?;

        create_buffer_from_vec(output, context)
    });
    global.set(
        js_string!("__viper_zlib_deflate_sync"),
        deflate_sync_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // inflateSync (zlib format with header)
    let inflate_sync_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let input = get_buffer_data(args.get(0), context)?;

        let mut decoder = ZlibDecoder::new(&input[..]);
        let mut output = Vec::new();

        decoder
            .read_to_end(&mut output)
            .map_err(|e| JsNativeError::error().with_message(format!("inflate error: {}", e)))?;

        create_buffer_from_vec(output, context)
    });
    global.set(
        js_string!("__viper_zlib_inflate_sync"),
        inflate_sync_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // deflateRawSync (raw deflate without header)
    let deflate_raw_sync_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let input = get_buffer_data(args.get(0), context)?;
        let level = get_compression_level(args.get(1), context);

        let mut encoder = DeflateEncoder::new(&input[..], Compression::new(level));
        let mut output = Vec::new();

        encoder
            .read_to_end(&mut output)
            .map_err(|e| JsNativeError::error().with_message(format!("deflateRaw error: {}", e)))?;

        create_buffer_from_vec(output, context)
    });
    global.set(
        js_string!("__viper_zlib_deflate_raw_sync"),
        deflate_raw_sync_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // inflateRawSync (raw deflate without header)
    let inflate_raw_sync_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let input = get_buffer_data(args.get(0), context)?;

        let mut decoder = DeflateDecoder::new(&input[..]);
        let mut output = Vec::new();

        decoder
            .read_to_end(&mut output)
            .map_err(|e| JsNativeError::error().with_message(format!("inflateRaw error: {}", e)))?;

        create_buffer_from_vec(output, context)
    });
    global.set(
        js_string!("__viper_zlib_inflate_raw_sync"),
        inflate_raw_sync_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // unzipSync (auto-detect gzip or deflate)
    let unzip_sync_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let input = get_buffer_data(args.get(0), context)?;

        // Try to detect format based on magic bytes
        if input.len() >= 2 && input[0] == 0x1f && input[1] == 0x8b {
            // Gzip format
            let mut decoder = GzDecoder::new(&input[..]);
            let mut output = Vec::new();
            decoder
                .read_to_end(&mut output)
                .map_err(|e| JsNativeError::error().with_message(format!("unzip error: {}", e)))?;
            create_buffer_from_vec(output, context)
        } else if input.len() >= 2 {
            // Try zlib format (deflate with header)
            let mut decoder = ZlibDecoder::new(&input[..]);
            let mut output = Vec::new();
            match decoder.read_to_end(&mut output) {
                Ok(_) => create_buffer_from_vec(output, context),
                Err(_) => {
                    // Try raw deflate as fallback
                    let mut decoder = DeflateDecoder::new(&input[..]);
                    let mut output = Vec::new();
                    decoder.read_to_end(&mut output).map_err(|e| {
                        JsNativeError::error().with_message(format!("unzip error: {}", e))
                    })?;
                    create_buffer_from_vec(output, context)
                }
            }
        } else {
            Err(JsNativeError::error()
                .with_message("unzip error: input too short")
                .into())
        }
    });
    global.set(
        js_string!("__viper_zlib_unzip_sync"),
        unzip_sync_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // crc32
    let crc32_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let input = get_buffer_data(args.get(0), context)?;
        let initial = args
            .get(1)
            .and_then(|v| v.as_number())
            .map(|n| n as u32)
            .unwrap_or(0);

        let crc = crc32_compute(&input, initial);
        Ok(JsValue::from(crc))
    });
    global.set(
        js_string!("__viper_zlib_crc32"),
        crc32_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    Ok(())
}

/// Get buffer data from a JsValue (supports Buffer, Uint8Array, ArrayBuffer, string)
fn get_buffer_data(value: Option<&JsValue>, context: &mut Context) -> JsResult<Vec<u8>> {
    let value = value.ok_or_else(|| JsNativeError::typ().with_message("buffer required"))?;

    // Handle string
    if let Some(s) = value.as_string() {
        return Ok(s.to_std_string_escaped().into_bytes());
    }

    // Handle object (Buffer, Uint8Array, ArrayBuffer)
    if let Some(obj) = value.as_object() {
        // Try as Uint8Array first (Buffer is a Uint8Array)
        if let Ok(typed_array) = JsUint8Array::from_object(obj.clone()) {
            let len = typed_array.length(context)?;
            let mut data = Vec::with_capacity(len);
            for i in 0..len {
                if let Some(byte) = typed_array.get(i, context)?.as_number() {
                    data.push(byte as u8);
                }
            }
            return Ok(data);
        }

        // Check for Buffer-like object with data property
        if let Ok(data_val) = obj.get(js_string!("data"), context) {
            if let Some(data_obj) = data_val.as_object() {
                if let Ok(typed_array) = JsUint8Array::from_object(data_obj.clone()) {
                    let len = typed_array.length(context)?;
                    let mut data = Vec::with_capacity(len);
                    for i in 0..len {
                        if let Some(byte) = typed_array.get(i, context)?.as_number() {
                            data.push(byte as u8);
                        }
                    }
                    return Ok(data);
                }
            }
        }
    }

    Err(JsNativeError::typ()
        .with_message("expected Buffer, Uint8Array, ArrayBuffer, or string")
        .into())
}

/// Get compression level from options
fn get_compression_level(options: Option<&JsValue>, context: &mut Context) -> u32 {
    if let Some(opts) = options {
        if let Some(obj) = opts.as_object() {
            if let Ok(level_val) = obj.get(js_string!("level"), context) {
                if let Some(level) = level_val.as_number() {
                    let level = level as i32;
                    if level == -1 {
                        return 6; // Default compression
                    }
                    return level.clamp(0, 9) as u32;
                }
            }
        }
        // If options is a number, use it directly as level
        if let Some(level) = opts.as_number() {
            let level = level as i32;
            if level == -1 {
                return 6;
            }
            return level.clamp(0, 9) as u32;
        }
    }
    6 // Default compression level
}

/// Create a Buffer from a Vec<u8>
fn create_buffer_from_vec(data: Vec<u8>, context: &mut Context) -> JsResult<JsValue> {
    // Create a Uint8Array and populate it with the data
    let uint8_array = JsUint8Array::from_iter(data.into_iter(), context)?;
    Ok(uint8_array.into())
}

/// Compute CRC32 checksum
fn crc32_compute(data: &[u8], initial: u32) -> u32 {
    // CRC32 lookup table (IEEE polynomial)
    const CRC32_TABLE: [u32; 256] = {
        let mut table = [0u32; 256];
        let mut i = 0;
        while i < 256 {
            let mut crc = i as u32;
            let mut j = 0;
            while j < 8 {
                if crc & 1 != 0 {
                    crc = 0xEDB88320 ^ (crc >> 1);
                } else {
                    crc >>= 1;
                }
                j += 1;
            }
            table[i] = crc;
            i += 1;
        }
        table
    };

    let mut crc = !initial;
    for &byte in data {
        crc = CRC32_TABLE[((crc ^ byte as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    !crc
}
