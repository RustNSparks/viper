//! Query String Module - Node.js compatible URL query string parsing/formatting
//!
//! Provides utilities for parsing and formatting URL query strings:
//! - querystring.parse(str[, sep[, eq[, options]]])
//! - querystring.stringify(obj[, sep[, eq[, options]]])
//! - querystring.escape(str)
//! - querystring.unescape(str)
//! - querystring.decode() - alias for parse
//! - querystring.encode() - alias for stringify

use boa_engine::{
    Context, JsNativeError, JsResult, JsValue, NativeFunction, Source, js_string,
    object::ObjectInitializer, object::builtins::JsArray,
};

/// Register the querystring module
pub fn register_querystring_module(context: &mut Context) -> JsResult<()> {
    register_native_querystring_functions(context)?;

    let querystring_code = r#"
        (function() {
            const querystring = {
                // Parse a query string into an object
                parse: function(str, sep, eq, options) {
                    return __viper_querystring_parse(str, sep, eq, options);
                },

                // Stringify an object into a query string
                stringify: function(obj, sep, eq, options) {
                    return __viper_querystring_stringify(obj, sep, eq, options);
                },

                // URL percent-encode a string
                escape: function(str) {
                    return __viper_querystring_escape(str);
                },

                // Decode URL percent-encoded string
                unescape: function(str) {
                    return __viper_querystring_unescape(str);
                },

                // Aliases
                decode: function(str, sep, eq, options) {
                    return this.parse(str, sep, eq, options);
                },

                encode: function(obj, sep, eq, options) {
                    return this.stringify(obj, sep, eq, options);
                }
            };

            globalThis.querystring = querystring;
            return querystring;
        })();
    "#;

    let source = Source::from_bytes(querystring_code.as_bytes());
    context.eval(source)?;

    Ok(())
}

/// Register native querystring functions
fn register_native_querystring_functions(context: &mut Context) -> JsResult<()> {
    let global = context.global_object();

    // querystring.parse(str, sep, eq, options)
    let parse_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let str = args
            .get(0)
            .and_then(|v| v.as_string())
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default();

        let sep = args
            .get(1)
            .and_then(|v| {
                if v.is_null_or_undefined() {
                    None
                } else {
                    v.as_string().map(|s| s.to_std_string_escaped())
                }
            })
            .unwrap_or_else(|| "&".to_string());

        let eq = args
            .get(2)
            .and_then(|v| {
                if v.is_null_or_undefined() {
                    None
                } else {
                    v.as_string().map(|s| s.to_std_string_escaped())
                }
            })
            .unwrap_or_else(|| "=".to_string());

        let max_keys = args
            .get(3)
            .and_then(|v| v.as_object())
            .and_then(|obj| obj.get(js_string!("maxKeys"), context).ok())
            .and_then(|v| v.as_number())
            .map(|n| n as usize)
            .unwrap_or(1000);

        parse_query_string(&str, &sep, &eq, max_keys, context)
    });
    global.set(
        js_string!("__viper_querystring_parse"),
        parse_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // querystring.stringify(obj, sep, eq, options)
    let stringify_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let obj = args.get(0).cloned().unwrap_or(JsValue::undefined());

        let sep = args
            .get(1)
            .and_then(|v| {
                if v.is_null_or_undefined() {
                    None
                } else {
                    v.as_string().map(|s| s.to_std_string_escaped())
                }
            })
            .unwrap_or_else(|| "&".to_string());

        let eq = args
            .get(2)
            .and_then(|v| {
                if v.is_null_or_undefined() {
                    None
                } else {
                    v.as_string().map(|s| s.to_std_string_escaped())
                }
            })
            .unwrap_or_else(|| "=".to_string());

        stringify_object(&obj, &sep, &eq, context)
    });
    global.set(
        js_string!("__viper_querystring_stringify"),
        stringify_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // querystring.escape(str)
    let escape_fn = NativeFunction::from_fn_ptr(|_this, args, _context| {
        let str = args
            .get(0)
            .and_then(|v| v.as_string())
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default();

        let escaped = percent_encode(&str);
        Ok(JsValue::from(js_string!(escaped)))
    });
    global.set(
        js_string!("__viper_querystring_escape"),
        escape_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // querystring.unescape(str)
    let unescape_fn = NativeFunction::from_fn_ptr(|_this, args, _context| {
        let str = args
            .get(0)
            .and_then(|v| v.as_string())
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default();

        let unescaped = percent_decode(&str);
        Ok(JsValue::from(js_string!(unescaped)))
    });
    global.set(
        js_string!("__viper_querystring_unescape"),
        unescape_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    Ok(())
}

/// Parse a query string into an object
fn parse_query_string(
    str: &str,
    sep: &str,
    eq: &str,
    max_keys: usize,
    context: &mut Context,
) -> JsResult<JsValue> {
    let result = ObjectInitializer::new(context).build();

    if str.is_empty() {
        return Ok(result.into());
    }

    let pairs: Vec<&str> = str.split(sep).collect();
    let limit = if max_keys == 0 {
        pairs.len()
    } else {
        max_keys.min(pairs.len())
    };

    for pair in pairs.into_iter().take(limit) {
        if pair.is_empty() {
            continue;
        }

        let (key, value) = if let Some(idx) = pair.find(eq) {
            let (k, v) = pair.split_at(idx);
            (k, &v[eq.len()..])
        } else {
            (pair, "")
        };

        let decoded_key = percent_decode(key);
        let decoded_value = percent_decode(value);

        let js_key = js_string!(decoded_key);
        let js_value = JsValue::from(js_string!(decoded_value));

        // Check if key already exists
        if let Ok(existing) = result.get(js_key.clone(), context) {
            if existing.is_undefined() {
                // Key doesn't exist, set it
                result.set(js_key, js_value, false, context)?;
            } else if let Some(arr_obj) = existing.as_object() {
                if arr_obj.is_array() {
                    // Already an array, push to it
                    let arr = JsArray::from_object(arr_obj.clone())?;
                    arr.push(js_value, context)?;
                } else {
                    // Not an array, convert to array
                    let arr = JsArray::new(context);
                    arr.push(existing, context)?;
                    arr.push(js_value, context)?;
                    result.set(js_key, arr, false, context)?;
                }
            } else {
                // Existing value is not an object, convert to array
                let arr = JsArray::new(context);
                arr.push(existing, context)?;
                arr.push(js_value, context)?;
                result.set(js_key, arr, false, context)?;
            }
        } else {
            result.set(js_key, js_value, false, context)?;
        }
    }

    Ok(result.into())
}

/// Stringify an object into a query string
fn stringify_object(
    obj: &JsValue,
    sep: &str,
    eq: &str,
    context: &mut Context,
) -> JsResult<JsValue> {
    if obj.is_null_or_undefined() {
        return Ok(JsValue::from(js_string!("")));
    }

    let obj = match obj.as_object() {
        Some(o) => o,
        None => return Ok(JsValue::from(js_string!(""))),
    };

    let keys = obj.own_property_keys(context)?;
    let mut pairs = Vec::new();

    for key in keys {
        let key_str = key.to_string();
        let value = obj.get(key.clone(), context)?;

        if value.is_undefined() {
            continue;
        }

        let encoded_key = percent_encode(&key_str);

        if let Some(arr_obj) = value.as_object() {
            if arr_obj.is_array() {
                let arr = JsArray::from_object(arr_obj.clone())?;
                let len = arr.length(context)?;
                for i in 0..len {
                    let item = arr.get(i, context)?;
                    let item_str = value_to_string(&item, context)?;
                    let encoded_value = percent_encode(&item_str);
                    pairs.push(format!("{}{}{}", encoded_key, eq, encoded_value));
                }
                continue;
            }
        }

        let value_str = value_to_string(&value, context)?;
        let encoded_value = percent_encode(&value_str);
        pairs.push(format!("{}{}{}", encoded_key, eq, encoded_value));
    }

    Ok(JsValue::from(js_string!(pairs.join(sep))))
}

/// Convert a JsValue to a string for query string serialization
fn value_to_string(value: &JsValue, context: &mut Context) -> JsResult<String> {
    if value.is_undefined() || value.is_null() {
        return Ok(String::new());
    }

    if let Some(s) = value.as_string() {
        return Ok(s.to_std_string_escaped());
    }

    if let Some(n) = value.as_number() {
        if n.is_finite() {
            return Ok(format!("{}", n));
        }
        return Ok(String::new());
    }

    if let Some(b) = value.as_boolean() {
        return Ok(if b {
            "true".to_string()
        } else {
            "false".to_string()
        });
    }

    // For BigInt, try to convert to string
    if value.is_bigint() {
        let s = value.to_string(context)?;
        return Ok(s.to_std_string_escaped());
    }

    // For objects, return empty string (except arrays which are handled separately)
    Ok(String::new())
}

/// Percent-encode a string for URL query strings
fn percent_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);

    for byte in s.bytes() {
        match byte {
            // Unreserved characters (RFC 3986)
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            // Space becomes +
            b' ' => {
                result.push('+');
            }
            // Everything else is percent-encoded
            _ => {
                result.push('%');
                result.push_str(&format!("{:02X}", byte));
            }
        }
    }

    result
}

/// Percent-decode a URL-encoded string
fn percent_decode(s: &str) -> String {
    let mut result = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                // Try to decode hex
                let hex = &s[i + 1..i + 3];
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    result.push(byte);
                    i += 3;
                } else {
                    // Invalid hex, keep as-is
                    result.push(b'%');
                    i += 1;
                }
            }
            b'+' => {
                result.push(b' ');
                i += 1;
            }
            byte => {
                result.push(byte);
                i += 1;
            }
        }
    }

    String::from_utf8_lossy(&result).into_owned()
}
