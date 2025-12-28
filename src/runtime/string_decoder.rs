//! String Decoder Module - Node.js compatible string decoding
//!
//! Provides an API for decoding Buffer objects into strings in a manner that
//! preserves encoded multi-byte UTF-8 and UTF-16 characters.
//!
//! This is a high-performance native Rust implementation.

use boa_engine::{
    Context, JsData, JsNativeError, JsResult, JsValue, NativeFunction, Source, js_string,
    object::{JsObject, ObjectInitializer, builtins::JsUint8Array},
};
use boa_gc::{Finalize, Trace};
use std::sync::{Arc, Mutex};

/// Supported encodings
#[derive(Clone, Debug, PartialEq)]
enum Encoding {
    Utf8,
    Utf16Le,
    Utf16Be,
    Base64,
    Latin1,
    Ascii,
    Hex,
}

impl Encoding {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "utf8" | "utf-8" => Some(Encoding::Utf8),
            "utf16le" | "utf-16le" | "ucs2" | "ucs-2" => Some(Encoding::Utf16Le),
            "utf16be" | "utf-16be" => Some(Encoding::Utf16Be),
            "base64" => Some(Encoding::Base64),
            "latin1" | "binary" => Some(Encoding::Latin1),
            "ascii" => Some(Encoding::Ascii),
            "hex" => Some(Encoding::Hex),
            _ => None,
        }
    }
}

/// Internal state for StringDecoder
#[derive(Clone)]
struct DecoderState {
    encoding: Encoding,
    /// Buffer for incomplete multi-byte sequences
    pending: Vec<u8>,
    /// For UTF-16: whether we have a pending first byte
    utf16_pending: Option<u8>,
}

impl DecoderState {
    fn new(encoding: Encoding) -> Self {
        Self {
            encoding,
            pending: Vec::new(),
            utf16_pending: None,
        }
    }

    /// Write bytes and return decoded string, keeping incomplete sequences buffered
    fn write(&mut self, bytes: &[u8]) -> String {
        match self.encoding {
            Encoding::Utf8 => self.write_utf8(bytes),
            Encoding::Utf16Le => self.write_utf16le(bytes),
            Encoding::Utf16Be => self.write_utf16be(bytes),
            Encoding::Base64 => self.write_base64(bytes),
            Encoding::Latin1 => self.write_latin1(bytes),
            Encoding::Ascii => self.write_ascii(bytes),
            Encoding::Hex => self.write_hex(bytes),
        }
    }

    /// End decoding, return any remaining bytes as string with replacement chars
    fn end(&mut self, bytes: Option<&[u8]>) -> String {
        let mut result = String::new();

        if let Some(b) = bytes {
            result = self.write(b);
        }

        // Handle any remaining pending bytes
        if !self.pending.is_empty() {
            match self.encoding {
                Encoding::Utf8 => {
                    // Replace incomplete UTF-8 with replacement character
                    result.push('\u{FFFD}');
                }
                Encoding::Utf16Le | Encoding::Utf16Be => {
                    // Replace incomplete UTF-16 with replacement character
                    if self.pending.len() == 1 || self.utf16_pending.is_some() {
                        result.push('\u{FFFD}');
                    }
                }
                _ => {
                    // For other encodings, just output what we have
                    for &b in &self.pending {
                        result.push(b as char);
                    }
                }
            }
            self.pending.clear();
            self.utf16_pending = None;
        }

        result
    }

    fn write_utf8(&mut self, bytes: &[u8]) -> String {
        let mut result = String::new();

        // Prepend any pending bytes
        let mut all_bytes: Vec<u8> = self.pending.drain(..).collect();
        all_bytes.extend_from_slice(bytes);

        let mut i = 0;
        while i < all_bytes.len() {
            let b = all_bytes[i];

            // Determine the expected length of this UTF-8 sequence
            let seq_len = if b < 0x80 {
                1
            } else if b < 0xC0 {
                // Invalid continuation byte at start - skip it
                result.push('\u{FFFD}');
                i += 1;
                continue;
            } else if b < 0xE0 {
                2
            } else if b < 0xF0 {
                3
            } else if b < 0xF8 {
                4
            } else {
                // Invalid UTF-8 start byte
                result.push('\u{FFFD}');
                i += 1;
                continue;
            };

            // Check if we have enough bytes
            if i + seq_len > all_bytes.len() {
                // Not enough bytes - buffer the rest for next call
                self.pending = all_bytes[i..].to_vec();
                break;
            }

            // Try to decode the sequence
            let seq = &all_bytes[i..i + seq_len];
            match std::str::from_utf8(seq) {
                Ok(s) => result.push_str(s),
                Err(_) => result.push('\u{FFFD}'),
            }

            i += seq_len;
        }

        result
    }

    fn write_utf16le(&mut self, bytes: &[u8]) -> String {
        let mut result = String::new();

        // Prepend any pending bytes
        let mut all_bytes: Vec<u8> = Vec::new();
        if let Some(b) = self.utf16_pending.take() {
            all_bytes.push(b);
        }
        all_bytes.extend_from_slice(bytes);

        let mut i = 0;
        while i + 1 < all_bytes.len() {
            let lo = all_bytes[i] as u16;
            let hi = all_bytes[i + 1] as u16;
            let code_unit = lo | (hi << 8);

            // Check for surrogate pairs
            if (0xD800..=0xDBFF).contains(&code_unit) {
                // High surrogate - need low surrogate
                if i + 3 < all_bytes.len() {
                    let lo2 = all_bytes[i + 2] as u16;
                    let hi2 = all_bytes[i + 3] as u16;
                    let code_unit2 = lo2 | (hi2 << 8);

                    if (0xDC00..=0xDFFF).contains(&code_unit2) {
                        // Valid surrogate pair
                        let code_point = 0x10000
                            + ((code_unit as u32 - 0xD800) << 10)
                            + (code_unit2 as u32 - 0xDC00);
                        if let Some(c) = char::from_u32(code_point) {
                            result.push(c);
                        } else {
                            result.push('\u{FFFD}');
                        }
                        i += 4;
                        continue;
                    }
                } else {
                    // Not enough bytes for surrogate pair - buffer
                    self.pending = all_bytes[i..].to_vec();
                    return result;
                }
            }

            // Regular BMP character
            if let Some(c) = char::from_u32(code_unit as u32) {
                result.push(c);
            } else {
                result.push('\u{FFFD}');
            }
            i += 2;
        }

        // Buffer any remaining odd byte
        if i < all_bytes.len() {
            self.utf16_pending = Some(all_bytes[i]);
        }

        result
    }

    fn write_utf16be(&mut self, bytes: &[u8]) -> String {
        let mut result = String::new();

        // Prepend any pending bytes
        let mut all_bytes: Vec<u8> = Vec::new();
        if let Some(b) = self.utf16_pending.take() {
            all_bytes.push(b);
        }
        all_bytes.extend_from_slice(bytes);

        let mut i = 0;
        while i + 1 < all_bytes.len() {
            let hi = all_bytes[i] as u16;
            let lo = all_bytes[i + 1] as u16;
            let code_unit = (hi << 8) | lo;

            // Check for surrogate pairs
            if (0xD800..=0xDBFF).contains(&code_unit) {
                // High surrogate - need low surrogate
                if i + 3 < all_bytes.len() {
                    let hi2 = all_bytes[i + 2] as u16;
                    let lo2 = all_bytes[i + 3] as u16;
                    let code_unit2 = (hi2 << 8) | lo2;

                    if (0xDC00..=0xDFFF).contains(&code_unit2) {
                        // Valid surrogate pair
                        let code_point = 0x10000
                            + ((code_unit as u32 - 0xD800) << 10)
                            + (code_unit2 as u32 - 0xDC00);
                        if let Some(c) = char::from_u32(code_point) {
                            result.push(c);
                        } else {
                            result.push('\u{FFFD}');
                        }
                        i += 4;
                        continue;
                    }
                } else {
                    // Not enough bytes for surrogate pair - buffer
                    self.pending = all_bytes[i..].to_vec();
                    return result;
                }
            }

            // Regular BMP character
            if let Some(c) = char::from_u32(code_unit as u32) {
                result.push(c);
            } else {
                result.push('\u{FFFD}');
            }
            i += 2;
        }

        // Buffer any remaining odd byte
        if i < all_bytes.len() {
            self.utf16_pending = Some(all_bytes[i]);
        }

        result
    }

    fn write_base64(&mut self, bytes: &[u8]) -> String {
        use base64::{Engine, engine::general_purpose};

        // Prepend any pending bytes
        let mut all_bytes: Vec<u8> = self.pending.drain(..).collect();
        all_bytes.extend_from_slice(bytes);

        // Base64 works in groups of 3 bytes -> 4 chars
        let complete_len = (all_bytes.len() / 3) * 3;

        if complete_len > 0 {
            let result = general_purpose::STANDARD.encode(&all_bytes[..complete_len]);
            self.pending = all_bytes[complete_len..].to_vec();
            result
        } else {
            self.pending = all_bytes;
            String::new()
        }
    }

    fn write_latin1(&mut self, bytes: &[u8]) -> String {
        bytes.iter().map(|&b| b as char).collect()
    }

    fn write_ascii(&mut self, bytes: &[u8]) -> String {
        bytes.iter().map(|&b| (b & 0x7F) as char).collect()
    }

    fn write_hex(&mut self, bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

/// StringDecoder data stored in JS object
#[derive(Clone, Trace, Finalize, JsData)]
struct StringDecoderData {
    #[unsafe_ignore_trace]
    state: Arc<Mutex<DecoderState>>,
}

/// Register the string_decoder module
pub fn register_string_decoder_module(context: &mut Context) -> JsResult<()> {
    // Create the prototype with write and end methods
    let prototype = ObjectInitializer::new(context)
        .function(
            NativeFunction::from_fn_ptr(string_decoder_write),
            js_string!("write"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(string_decoder_end),
            js_string!("end"),
            1,
        )
        .build();

    // Store prototype globally
    context.global_object().set(
        js_string!("__StringDecoder_prototype__"),
        prototype,
        false,
        context,
    )?;

    // Create the constructor factory
    let constructor = NativeFunction::from_fn_ptr(|_this, args, context| {
        let encoding_str = args
            .get(0)
            .and_then(|v| v.as_string())
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_else(|| "utf8".to_string());

        let encoding = Encoding::from_str(&encoding_str).ok_or_else(|| {
            JsNativeError::typ().with_message(format!("Unknown encoding: {}", encoding_str))
        })?;

        let data = StringDecoderData {
            state: Arc::new(Mutex::new(DecoderState::new(encoding.clone()))),
        };

        // Get the prototype
        let proto = context
            .global_object()
            .get(js_string!("__StringDecoder_prototype__"), context)?;
        let proto_obj = proto.as_object().map(|o| o.clone());

        // Create the object with data
        let obj = JsObject::from_proto_and_data(proto_obj, data);

        // Add encoding property
        let encoding_name = match encoding {
            Encoding::Utf8 => "utf8",
            Encoding::Utf16Le => "utf16le",
            Encoding::Utf16Be => "utf16be",
            Encoding::Base64 => "base64",
            Encoding::Latin1 => "latin1",
            Encoding::Ascii => "ascii",
            Encoding::Hex => "hex",
        };
        obj.set(
            js_string!("encoding"),
            js_string!(encoding_name),
            false,
            context,
        )?;

        Ok(JsValue::from(obj))
    });

    // Register factory function
    context.global_object().set(
        js_string!("__StringDecoder_create__"),
        constructor.to_js_function(context.realm()),
        false,
        context,
    )?;

    // Create proper constructor using JavaScript wrapper
    let js_code = r#"
        (function() {
            function StringDecoder(encoding) {
                return globalThis.__StringDecoder_create__(encoding);
            }
            StringDecoder.prototype = globalThis.__StringDecoder_prototype__;
            return StringDecoder;
        })()
    "#;

    let sd_constructor = context.eval(Source::from_bytes(js_code.as_bytes()))?;

    // Create the module object
    let module = ObjectInitializer::new(context).build();
    module.set(
        js_string!("StringDecoder"),
        sd_constructor.clone(),
        false,
        context,
    )?;

    // Register globally
    context
        .global_object()
        .set(js_string!("string_decoder"), module, false, context)?;

    // Also register StringDecoder directly on global for convenience
    context
        .global_object()
        .set(js_string!("StringDecoder"), sd_constructor, false, context)?;

    Ok(())
}

/// Helper to get decoder state from this
fn get_decoder_state(this: &JsValue) -> JsResult<Arc<Mutex<DecoderState>>> {
    let obj = this
        .as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("this is not a StringDecoder"))?;
    let data = obj
        .downcast_ref::<StringDecoderData>()
        .ok_or_else(|| JsNativeError::typ().with_message("this is not a StringDecoder"))?;
    Ok(data.state.clone())
}

/// Get bytes from a value (Buffer, TypedArray, DataView, or string)
fn get_bytes(value: &JsValue, context: &mut Context) -> JsResult<Vec<u8>> {
    if let Some(obj) = value.as_object() {
        // Check for TypedArray (Uint8Array)
        if let Ok(arr) = JsUint8Array::from_object(obj.clone()) {
            let len = arr.length(context)?;
            let mut bytes = Vec::with_capacity(len as usize);
            for i in 0..len {
                let val = arr.get(i, context)?;
                bytes.push(val.to_number(context)? as u8);
            }
            return Ok(bytes);
        }

        // Check for ArrayBuffer via 'buffer' property (DataView, other TypedArrays)
        if obj.has_property(js_string!("buffer"), context)? {
            // Try to get byte data
            if let Ok(byte_length) = obj.get(js_string!("byteLength"), context) {
                if byte_length.is_number() {
                    let len = byte_length.to_number(context)? as usize;
                    let mut bytes = Vec::with_capacity(len);
                    for i in 0..len {
                        if let Ok(val) = obj.get(i as u32, context) {
                            if val.is_number() {
                                bytes.push(val.to_number(context)? as u8);
                            }
                        }
                    }
                    if bytes.len() == len {
                        return Ok(bytes);
                    }
                }
            }
        }

        // Check for Buffer-like object with 'data' property or array-like
        if obj.has_property(js_string!("length"), context)? {
            let len = obj.get(js_string!("length"), context)?.to_number(context)? as usize;
            let mut bytes = Vec::with_capacity(len);
            for i in 0..len {
                let val = obj.get(i as u32, context)?;
                bytes.push(val.to_number(context)? as u8);
            }
            return Ok(bytes);
        }
    }

    // Try as string - convert to UTF-8 bytes
    if let Some(s) = value.as_string() {
        return Ok(s.to_std_string_escaped().into_bytes());
    }

    Err(JsNativeError::typ()
        .with_message("argument must be a Buffer, TypedArray, DataView, or string")
        .into())
}

/// StringDecoder.prototype.write(buffer)
fn string_decoder_write(
    this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let state = get_decoder_state(this)?;

    let bytes = if let Some(arg) = args.get(0) {
        if arg.is_undefined() || arg.is_null() {
            Vec::new()
        } else {
            get_bytes(arg, context)?
        }
    } else {
        Vec::new()
    };

    let result = state.lock().unwrap().write(&bytes);
    Ok(JsValue::from(js_string!(result)))
}

/// StringDecoder.prototype.end([buffer])
fn string_decoder_end(
    this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let state = get_decoder_state(this)?;

    let bytes = if let Some(arg) = args.get(0) {
        if arg.is_undefined() || arg.is_null() {
            None
        } else {
            Some(get_bytes(arg, context)?)
        }
    } else {
        None
    };

    let result = state.lock().unwrap().end(bytes.as_deref());
    Ok(JsValue::from(js_string!(result)))
}
