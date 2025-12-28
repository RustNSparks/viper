//! URL Module - Node.js compatible URL parsing and formatting utilities
//!
//! This module extends the WHATWG URL API (provided by boa_runtime) with Node.js
//! specific utilities like url.parse(), url.format(), url.resolve(),
//! domainToASCII(), domainToUnicode(), fileURLToPath(), pathToFileURL(), etc.
//!
//! Also provides a high-performance native Rust implementation of URLSearchParams.

use boa_engine::{
    Context, JsData, JsNativeError, JsResult, JsValue, NativeFunction, Source,
    class::{Class, ClassBuilder},
    js_string,
    object::{ObjectInitializer, builtins::JsArray},
    property::Attribute,
};
use boa_gc::{Finalize, Trace};
use std::collections::HashMap;

/// Register the url module with Node.js compatible utilities
pub fn register_url_module(context: &mut Context) -> JsResult<()> {
    // Create the url module object
    let url_module = ObjectInitializer::new(context)
        .function(
            NativeFunction::from_fn_ptr(url_parse),
            js_string!("parse"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(url_format),
            js_string!("format"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(url_resolve),
            js_string!("resolve"),
            2,
        )
        .function(
            NativeFunction::from_fn_ptr(domain_to_ascii),
            js_string!("domainToASCII"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(domain_to_unicode),
            js_string!("domainToUnicode"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(file_url_to_path),
            js_string!("fileURLToPath"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(path_to_file_url),
            js_string!("pathToFileURL"),
            1,
        )
        .function(
            NativeFunction::from_fn_ptr(url_to_http_options),
            js_string!("urlToHttpOptions"),
            1,
        )
        .build();

    // Register globally
    context
        .global_object()
        .set(js_string!("url"), url_module, false, context)?;

    // Also add URL and URLSearchParams references to the url module for convenience
    // (they're already global from boa_runtime's UrlExtension)
    let js_code = r#"
        (function() {
            // Add URL and URLSearchParams to the url module
            globalThis.url.URL = globalThis.URL;
            globalThis.url.URLSearchParams = globalThis.URLSearchParams;

            // Add Url class (legacy alias)
            globalThis.url.Url = class Url {
                constructor() {
                    this.protocol = null;
                    this.slashes = null;
                    this.auth = null;
                    this.host = null;
                    this.port = null;
                    this.hostname = null;
                    this.hash = null;
                    this.search = null;
                    this.query = null;
                    this.pathname = null;
                    this.path = null;
                    this.href = null;
                }
            };
        })();
    "#;

    context.eval(Source::from_bytes(js_code.as_bytes()))?;

    // Register URLSearchParams class
    register_url_search_params(context)?;

    Ok(())
}

// =============================================================================
// URLSearchParams Implementation - High-performance native Rust
// =============================================================================

/// Internal storage for URLSearchParams - maintains insertion order
#[derive(Clone, Trace, Finalize, JsData)]
struct UrlSearchParamsData {
    /// Params stored as Vec to maintain insertion order (important for spec compliance)
    #[unsafe_ignore_trace]
    params: std::sync::Arc<std::sync::Mutex<Vec<(String, String)>>>,
}

impl UrlSearchParamsData {
    fn new() -> Self {
        Self {
            params: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    fn from_string(init: &str) -> Self {
        let data = Self::new();
        let input = if init.starts_with('?') {
            &init[1..]
        } else {
            init
        };

        if !input.is_empty() {
            let mut params = data.params.lock().unwrap();
            for pair in input.split('&') {
                if pair.is_empty() {
                    continue;
                }
                let (key, value) = if let Some(eq_idx) = pair.find('=') {
                    (url_decode(&pair[..eq_idx]), url_decode(&pair[eq_idx + 1..]))
                } else {
                    (url_decode(pair), String::new())
                };
                params.push((key, value));
            }
        }
        data
    }
}

/// URL decode (percent-decode) a string
fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut bytes = s.bytes().peekable();

    while let Some(b) = bytes.next() {
        if b == b'%' {
            let hex: Vec<u8> = bytes.by_ref().take(2).collect();
            if hex.len() == 2 {
                let hex_str = String::from_utf8_lossy(&hex);
                if let Ok(byte) = u8::from_str_radix(&hex_str, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            result.push('%');
            for h in hex {
                result.push(h as char);
            }
        } else if b == b'+' {
            result.push(' ');
        } else {
            result.push(b as char);
        }
    }

    result
}

/// URL encode a string for use in query strings
fn url_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);

    for c in s.chars() {
        match c {
            'A'..='Z'
            | 'a'..='z'
            | '0'..='9'
            | '-'
            | '_'
            | '.'
            | '!'
            | '~'
            | '*'
            | '\''
            | '('
            | ')' => {
                result.push(c);
            }
            ' ' => {
                result.push('+');
            }
            _ => {
                for byte in c.to_string().as_bytes() {
                    result.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }

    result
}

/// Register the URLSearchParams class
fn register_url_search_params(context: &mut Context) -> JsResult<()> {
    use boa_engine::object::JsObject;

    // Create the prototype object with all methods
    let prototype = ObjectInitializer::new(context)
        .function(
            NativeFunction::from_fn_ptr(usp_append),
            js_string!("append"),
            2,
        )
        .function(
            NativeFunction::from_fn_ptr(usp_delete),
            js_string!("delete"),
            1,
        )
        .function(NativeFunction::from_fn_ptr(usp_get), js_string!("get"), 1)
        .function(
            NativeFunction::from_fn_ptr(usp_get_all),
            js_string!("getAll"),
            1,
        )
        .function(NativeFunction::from_fn_ptr(usp_has), js_string!("has"), 1)
        .function(NativeFunction::from_fn_ptr(usp_set), js_string!("set"), 2)
        .function(NativeFunction::from_fn_ptr(usp_sort), js_string!("sort"), 0)
        .function(
            NativeFunction::from_fn_ptr(usp_to_string),
            js_string!("toString"),
            0,
        )
        .function(
            NativeFunction::from_fn_ptr(usp_entries),
            js_string!("entries"),
            0,
        )
        .function(NativeFunction::from_fn_ptr(usp_keys), js_string!("keys"), 0)
        .function(
            NativeFunction::from_fn_ptr(usp_values),
            js_string!("values"),
            0,
        )
        .function(
            NativeFunction::from_fn_ptr(usp_for_each),
            js_string!("forEach"),
            1,
        )
        .build();

    // Store prototype globally for use in constructor
    context.global_object().set(
        js_string!("__URLSearchParams_prototype__"),
        prototype.clone(),
        false,
        context,
    )?;

    // Create the constructor function
    let constructor = NativeFunction::from_fn_ptr(|_this, args, context| {
        let data = if let Some(init) = args.get(0) {
            if init.is_undefined() || init.is_null() {
                UrlSearchParamsData::new()
            } else if let Some(s) = init.as_string() {
                UrlSearchParamsData::from_string(&s.to_std_string_escaped())
            } else if let Some(obj) = init.as_object() {
                // Check if it's another URLSearchParams
                if let Some(other_data) = obj.downcast_ref::<UrlSearchParamsData>() {
                    let params = other_data.params.lock().unwrap().clone();
                    let data = UrlSearchParamsData::new();
                    *data.params.lock().unwrap() = params;
                    data
                } else {
                    // Try to iterate as object or array
                    let data = UrlSearchParamsData::new();
                    let mut params = data.params.lock().unwrap();

                    // Check if it's iterable (has Symbol.iterator or is array-like)
                    if obj.has_property(js_string!("length"), context)? {
                        // Array-like: expect [[key, value], ...]
                        let len =
                            obj.get(js_string!("length"), context)?.to_number(context)? as u32;
                        for i in 0..len {
                            let item = obj.get(i, context)?;
                            if let Some(item_obj) = item.as_object() {
                                let key = item_obj
                                    .get(0, context)?
                                    .to_string(context)?
                                    .to_std_string_escaped();
                                let value = item_obj
                                    .get(1, context)?
                                    .to_string(context)?
                                    .to_std_string_escaped();
                                params.push((key, value));
                            }
                        }
                    } else {
                        // Plain object: iterate keys
                        let keys = obj.own_property_keys(context)?;
                        for key in keys {
                            let key_str = key.to_string();
                            let value = obj
                                .get(key, context)?
                                .to_string(context)?
                                .to_std_string_escaped();
                            params.push((key_str, value));
                        }
                    }
                    drop(params);
                    data
                }
            } else {
                // Convert to string
                let s = init.to_string(context)?.to_std_string_escaped();
                UrlSearchParamsData::from_string(&s)
            }
        } else {
            UrlSearchParamsData::new()
        };

        // Get the prototype
        let proto = context
            .global_object()
            .get(js_string!("__URLSearchParams_prototype__"), context)?;
        let proto_obj = proto.as_object().map(|o| o.clone());

        // Create the object with data
        let obj = JsObject::from_proto_and_data(proto_obj, data);

        // Add size getter
        let size_getter = NativeFunction::from_fn_ptr(|this, _args, _context| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("this is not a URLSearchParams")
            })?;
            let data = obj.downcast_ref::<UrlSearchParamsData>().ok_or_else(|| {
                JsNativeError::typ().with_message("this is not a URLSearchParams")
            })?;
            let params = data.params.lock().unwrap();
            Ok(JsValue::from(params.len() as i32))
        });

        obj.define_property_or_throw(
            js_string!("size"),
            boa_engine::property::PropertyDescriptor::builder()
                .get(size_getter.to_js_function(context.realm()))
                .enumerable(false)
                .configurable(true)
                .build(),
            context,
        )?;

        Ok(JsValue::from(obj))
    });

    // Register the factory function with a different name
    context.global_object().set(
        js_string!("__URLSearchParams_create__"),
        constructor.to_js_function(context.realm()),
        false,
        context,
    )?;

    // Create a proper constructor using JavaScript that wraps our factory
    let js_constructor = r#"
        (function() {
            function URLSearchParams(init) {
                // Call the native factory function
                return globalThis.__URLSearchParams_create__(init);
            }

            // Copy prototype methods
            URLSearchParams.prototype = globalThis.__URLSearchParams_prototype__;

            return URLSearchParams;
        })()
    "#;

    let usp_constructor = context.eval(Source::from_bytes(js_constructor.as_bytes()))?;

    // Register as global URLSearchParams
    context.global_object().set(
        js_string!("URLSearchParams"),
        usp_constructor.clone(),
        false,
        context,
    )?;

    // Also add to url module
    let url_module = context.global_object().get(js_string!("url"), context)?;
    if let Some(url_obj) = url_module.as_object() {
        url_obj.set(
            js_string!("URLSearchParams"),
            usp_constructor,
            false,
            context,
        )?;
    }

    Ok(())
}

/// Helper to get UrlSearchParamsData from this
fn get_usp_data(
    this: &JsValue,
) -> JsResult<std::sync::Arc<std::sync::Mutex<Vec<(String, String)>>>> {
    let obj = this
        .as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("this is not a URLSearchParams"))?;
    let data = obj
        .downcast_ref::<UrlSearchParamsData>()
        .ok_or_else(|| JsNativeError::typ().with_message("this is not a URLSearchParams"))?;
    Ok(data.params.clone())
}

/// URLSearchParams.prototype.append(name, value)
fn usp_append(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let params = get_usp_data(this)?;
    let name = args
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

    params.lock().unwrap().push((name, value));
    Ok(JsValue::undefined())
}

/// URLSearchParams.prototype.delete(name[, value])
fn usp_delete(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let params = get_usp_data(this)?;
    let name = args
        .get(0)
        .map(|v| v.to_string(context))
        .transpose()?
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let value_filter = args
        .get(1)
        .filter(|v| !v.is_undefined())
        .map(|v| v.to_string(context))
        .transpose()?
        .map(|s| s.to_std_string_escaped());

    let mut params = params.lock().unwrap();
    if let Some(value) = value_filter {
        params.retain(|(k, v)| !(k == &name && v == &value));
    } else {
        params.retain(|(k, _)| k != &name);
    }
    Ok(JsValue::undefined())
}

/// URLSearchParams.prototype.get(name)
fn usp_get(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let params = get_usp_data(this)?;
    let name = args
        .get(0)
        .map(|v| v.to_string(context))
        .transpose()?
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();

    let params = params.lock().unwrap();
    for (k, v) in params.iter() {
        if k == &name {
            return Ok(JsValue::from(js_string!(v.clone())));
        }
    }
    Ok(JsValue::null())
}

/// URLSearchParams.prototype.getAll(name)
fn usp_get_all(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let params = get_usp_data(this)?;
    let name = args
        .get(0)
        .map(|v| v.to_string(context))
        .transpose()?
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();

    let params = params.lock().unwrap();
    let arr = JsArray::new(context);
    for (k, v) in params.iter() {
        if k == &name {
            arr.push(JsValue::from(js_string!(v.clone())), context)?;
        }
    }
    Ok(arr.into())
}

/// URLSearchParams.prototype.has(name[, value])
fn usp_has(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let params = get_usp_data(this)?;
    let name = args
        .get(0)
        .map(|v| v.to_string(context))
        .transpose()?
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let value_filter = args
        .get(1)
        .filter(|v| !v.is_undefined())
        .map(|v| v.to_string(context))
        .transpose()?
        .map(|s| s.to_std_string_escaped());

    let params = params.lock().unwrap();
    let result = if let Some(value) = value_filter {
        params.iter().any(|(k, v)| k == &name && v == &value)
    } else {
        params.iter().any(|(k, _)| k == &name)
    };
    Ok(JsValue::from(result))
}

/// URLSearchParams.prototype.set(name, value)
fn usp_set(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let params = get_usp_data(this)?;
    let name = args
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

    let mut params = params.lock().unwrap();

    // Find first occurrence and update it, remove all others
    let mut found = false;
    let mut i = 0;
    while i < params.len() {
        if params[i].0 == name {
            if !found {
                params[i].1 = value.clone();
                found = true;
                i += 1;
            } else {
                params.remove(i);
            }
        } else {
            i += 1;
        }
    }

    // If not found, append
    if !found {
        params.push((name, value));
    }

    Ok(JsValue::undefined())
}

/// URLSearchParams.prototype.sort()
fn usp_sort(this: &JsValue, _args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
    let params = get_usp_data(this)?;
    let mut params = params.lock().unwrap();

    // Stable sort by key name (preserves relative order of same-key entries)
    params.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(JsValue::undefined())
}

/// URLSearchParams.prototype.toString()
fn usp_to_string(this: &JsValue, _args: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
    let params = get_usp_data(this)?;
    let params = params.lock().unwrap();

    let result: Vec<String> = params
        .iter()
        .map(|(k, v)| format!("{}={}", url_encode(k), url_encode(v)))
        .collect();

    Ok(JsValue::from(js_string!(result.join("&"))))
}

/// URLSearchParams.prototype.entries()
fn usp_entries(this: &JsValue, _args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let params = get_usp_data(this)?;
    let params = params.lock().unwrap();

    // Create an array of [key, value] pairs
    let arr = JsArray::new(context);
    for (k, v) in params.iter() {
        let pair = JsArray::new(context);
        pair.push(JsValue::from(js_string!(k.clone())), context)?;
        pair.push(JsValue::from(js_string!(v.clone())), context)?;
        arr.push(pair, context)?;
    }

    // Return an iterator-like object
    create_array_iterator(arr.into(), context)
}

/// URLSearchParams.prototype.keys()
fn usp_keys(this: &JsValue, _args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let params = get_usp_data(this)?;
    let params = params.lock().unwrap();

    let arr = JsArray::new(context);
    for (k, _) in params.iter() {
        arr.push(JsValue::from(js_string!(k.clone())), context)?;
    }

    create_array_iterator(arr.into(), context)
}

/// URLSearchParams.prototype.values()
fn usp_values(this: &JsValue, _args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let params = get_usp_data(this)?;
    let params = params.lock().unwrap();

    let arr = JsArray::new(context);
    for (_, v) in params.iter() {
        arr.push(JsValue::from(js_string!(v.clone())), context)?;
    }

    create_array_iterator(arr.into(), context)
}

/// URLSearchParams.prototype.forEach(callback[, thisArg])
fn usp_for_each(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let params = get_usp_data(this)?;
    let callback = args
        .get(0)
        .ok_or_else(|| JsNativeError::typ().with_message("forEach requires a callback function"))?;
    let this_arg = args.get(1).cloned().unwrap_or(JsValue::undefined());

    let callback_obj = callback
        .as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("callback must be a function"))?;

    let params = params.lock().unwrap().clone();
    for (k, v) in params.iter() {
        callback_obj.call(
            &this_arg,
            &[
                JsValue::from(js_string!(v.clone())),
                JsValue::from(js_string!(k.clone())),
                this.clone(),
            ],
            context,
        )?;
    }

    Ok(JsValue::undefined())
}

/// Create a simple array iterator
fn create_array_iterator(arr: JsValue, context: &mut Context) -> JsResult<JsValue> {
    // Create iterator object with next() method
    let iterator = ObjectInitializer::new(context).build();

    // Store array and index
    iterator.set(js_string!("__array__"), arr, false, context)?;
    iterator.set(js_string!("__index__"), JsValue::from(0), false, context)?;

    // Add next() method
    let next_fn = NativeFunction::from_fn_ptr(|this, _args, context| {
        let obj = this
            .as_object()
            .ok_or_else(|| JsNativeError::typ().with_message("this is not an iterator"))?;

        let arr = obj.get(js_string!("__array__"), context)?;
        let arr_obj = arr
            .as_object()
            .ok_or_else(|| JsNativeError::typ().with_message("invalid iterator state"))?;

        let index = obj
            .get(js_string!("__index__"), context)?
            .to_number(context)? as u32;

        let len = arr_obj
            .get(js_string!("length"), context)?
            .to_number(context)? as u32;

        let result = ObjectInitializer::new(context).build();

        if index < len {
            let value = arr_obj.get(index, context)?;
            result.set(js_string!("value"), value, false, context)?;
            result.set(js_string!("done"), JsValue::from(false), false, context)?;
            obj.set(
                js_string!("__index__"),
                JsValue::from(index + 1),
                false,
                context,
            )?;
        } else {
            result.set(js_string!("value"), JsValue::undefined(), false, context)?;
            result.set(js_string!("done"), JsValue::from(true), false, context)?;
        }

        Ok(JsValue::from(result))
    });

    iterator.set(
        js_string!("next"),
        next_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // Make it a proper iterator by adding Symbol.iterator
    let symbol_iterator = boa_engine::JsSymbol::iterator();
    let self_fn = NativeFunction::from_fn_ptr(|this, _args, _context| Ok(this.clone()));
    iterator.set(
        symbol_iterator,
        self_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    Ok(JsValue::from(iterator))
}

/// url.parse(urlString[, parseQueryString[, slashesDenoteHost]])
/// Parse a URL string into a URL object (legacy API)
fn url_parse(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let url_string = args
        .get(0)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();

    let parse_query_string = args.get(1).map(|v| v.to_boolean()).unwrap_or(false);

    let slashes_denote_host = args.get(2).map(|v| v.to_boolean()).unwrap_or(false);

    // Parse the URL
    let parsed = parse_url_string(&url_string, slashes_denote_host);

    // Create the result object
    let result = ObjectInitializer::new(context).build();

    // Set properties
    result.set(
        js_string!("protocol"),
        parsed
            .protocol
            .map(|s| JsValue::from(js_string!(s)))
            .unwrap_or(JsValue::null()),
        false,
        context,
    )?;

    result.set(
        js_string!("slashes"),
        JsValue::from(parsed.slashes),
        false,
        context,
    )?;

    result.set(
        js_string!("auth"),
        parsed
            .auth
            .map(|s| JsValue::from(js_string!(s)))
            .unwrap_or(JsValue::null()),
        false,
        context,
    )?;

    result.set(
        js_string!("host"),
        parsed
            .host
            .clone()
            .map(|s| JsValue::from(js_string!(s)))
            .unwrap_or(JsValue::null()),
        false,
        context,
    )?;

    result.set(
        js_string!("port"),
        parsed
            .port
            .clone()
            .map(|s| JsValue::from(js_string!(s)))
            .unwrap_or(JsValue::null()),
        false,
        context,
    )?;

    result.set(
        js_string!("hostname"),
        parsed
            .hostname
            .clone()
            .map(|s| JsValue::from(js_string!(s)))
            .unwrap_or(JsValue::null()),
        false,
        context,
    )?;

    result.set(
        js_string!("hash"),
        parsed
            .hash
            .clone()
            .map(|s| JsValue::from(js_string!(s)))
            .unwrap_or(JsValue::null()),
        false,
        context,
    )?;

    result.set(
        js_string!("search"),
        parsed
            .search
            .clone()
            .map(|s| JsValue::from(js_string!(s)))
            .unwrap_or(JsValue::null()),
        false,
        context,
    )?;

    // query can be string or object depending on parseQueryString
    if parse_query_string {
        // Parse query string into object
        let query_str = parsed
            .search
            .as_ref()
            .map(|s| {
                if s.starts_with('?') {
                    &s[1..]
                } else {
                    s.as_str()
                }
            })
            .unwrap_or("");

        let query_obj = parse_query_to_object(query_str, context)?;
        result.set(js_string!("query"), query_obj, false, context)?;
    } else {
        result.set(
            js_string!("query"),
            parsed
                .search
                .as_ref()
                .map(|s| {
                    if s.starts_with('?') {
                        JsValue::from(js_string!(s[1..].to_string()))
                    } else {
                        JsValue::from(js_string!(s.clone()))
                    }
                })
                .unwrap_or(JsValue::null()),
            false,
            context,
        )?;
    }

    result.set(
        js_string!("pathname"),
        parsed
            .pathname
            .clone()
            .map(|s| JsValue::from(js_string!(s)))
            .unwrap_or(JsValue::null()),
        false,
        context,
    )?;

    // path = pathname + search
    let path = match (&parsed.pathname, &parsed.search) {
        (Some(p), Some(s)) => Some(format!("{}{}", p, s)),
        (Some(p), None) => Some(p.clone()),
        (None, Some(s)) => Some(s.clone()),
        (None, None) => None,
    };
    result.set(
        js_string!("path"),
        path.map(|s| JsValue::from(js_string!(s)))
            .unwrap_or(JsValue::null()),
        false,
        context,
    )?;

    result.set(
        js_string!("href"),
        JsValue::from(js_string!(url_string)),
        false,
        context,
    )?;

    Ok(JsValue::from(result))
}

/// Parsed URL components
struct ParsedUrl {
    protocol: Option<String>,
    slashes: bool,
    auth: Option<String>,
    host: Option<String>,
    port: Option<String>,
    hostname: Option<String>,
    hash: Option<String>,
    search: Option<String>,
    pathname: Option<String>,
}

/// Parse a URL string into components (legacy Node.js style)
fn parse_url_string(url: &str, slashes_denote_host: bool) -> ParsedUrl {
    let mut result = ParsedUrl {
        protocol: None,
        slashes: false,
        auth: None,
        host: None,
        port: None,
        hostname: None,
        hash: None,
        search: None,
        pathname: None,
    };

    if url.is_empty() {
        return result;
    }

    let mut rest = url.to_string();

    // Extract hash
    if let Some(hash_idx) = rest.find('#') {
        result.hash = Some(rest[hash_idx..].to_string());
        rest = rest[..hash_idx].to_string();
    }

    // Extract search/query
    if let Some(query_idx) = rest.find('?') {
        result.search = Some(rest[query_idx..].to_string());
        rest = rest[..query_idx].to_string();
    }

    // Extract protocol
    if let Some(colon_idx) = rest.find(':') {
        let proto = &rest[..=colon_idx];
        // Check if it looks like a valid protocol
        if proto
            .chars()
            .take(colon_idx)
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
        {
            result.protocol = Some(proto.to_lowercase());
            rest = rest[colon_idx + 1..].to_string();
        }
    }

    // Check for slashes
    if rest.starts_with("//") {
        result.slashes = true;
        rest = rest[2..].to_string();

        // Find where host ends
        let host_end = rest.find('/').unwrap_or(rest.len());
        let host_part = rest[..host_end].to_string();
        rest = rest[host_end..].to_string();

        // Extract auth
        if let Some(at_idx) = host_part.find('@') {
            result.auth = Some(host_part[..at_idx].to_string());
            let host_only = &host_part[at_idx + 1..];
            parse_host(host_only, &mut result);
        } else {
            parse_host(&host_part, &mut result);
        }
    } else if slashes_denote_host && rest.starts_with('/') {
        // Handle slashesDenoteHost option
        rest = rest[1..].to_string();
        if let Some(slash_idx) = rest.find('/') {
            let host_part = rest[..slash_idx].to_string();
            rest = rest[slash_idx..].to_string();
            parse_host(&host_part, &mut result);
        }
    }

    // Remaining is pathname
    if !rest.is_empty() {
        result.pathname = Some(rest);
    } else if result.host.is_some() {
        // If we have a host, pathname defaults to "/"
        result.pathname = Some("/".to_string());
    }

    result
}

/// Parse host[:port] into hostname and port
fn parse_host(host: &str, result: &mut ParsedUrl) {
    if host.is_empty() {
        return;
    }

    // Handle IPv6 addresses
    if host.starts_with('[') {
        if let Some(bracket_end) = host.find(']') {
            result.hostname = Some(host[..=bracket_end].to_string());
            if host.len() > bracket_end + 1 && host.chars().nth(bracket_end + 1) == Some(':') {
                result.port = Some(host[bracket_end + 2..].to_string());
            }
            result.host = Some(host.to_string());
            return;
        }
    }

    // Regular host:port
    if let Some(colon_idx) = host.rfind(':') {
        let potential_port = &host[colon_idx + 1..];
        if potential_port.chars().all(|c| c.is_ascii_digit()) {
            result.hostname = Some(host[..colon_idx].to_lowercase());
            result.port = Some(potential_port.to_string());
            result.host = Some(host.to_lowercase());
            return;
        }
    }

    result.hostname = Some(host.to_lowercase());
    result.host = Some(host.to_lowercase());
}

/// Parse query string to object
fn parse_query_to_object(query: &str, context: &mut Context) -> JsResult<JsValue> {
    let result = ObjectInitializer::new(context).build();

    if query.is_empty() {
        return Ok(JsValue::from(result));
    }

    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }

        let (key, value) = if let Some(eq_idx) = pair.find('=') {
            let k = percent_decode(&pair[..eq_idx]);
            let v = percent_decode(&pair[eq_idx + 1..]);
            (k, v)
        } else {
            (percent_decode(pair), String::new())
        };

        // Check if key already exists
        let existing = result.get(js_string!(key.clone()), context)?;

        if existing.is_undefined() {
            result.set(js_string!(key), js_string!(value), false, context)?;
        } else if let Some(obj) = existing.as_object() {
            // Check if it's an array by looking for length property
            if obj.has_property(js_string!("length"), context)? {
                // Already an array, push to it
                let len = obj.get(js_string!("length"), context)?.to_number(context)? as u32;
                obj.set(len, js_string!(value), false, context)?;
            } else {
                // Not an array, convert to array
                let arr = boa_engine::object::builtins::JsArray::new(context);
                arr.push(existing, context)?;
                arr.push(JsValue::from(js_string!(value)), context)?;
                result.set(js_string!(key), arr, false, context)?;
            }
        } else {
            // Convert to array
            let arr = boa_engine::object::builtins::JsArray::new(context);
            arr.push(existing, context)?;
            arr.push(JsValue::from(js_string!(value)), context)?;
            result.set(js_string!(key), arr, false, context)?;
        }
    }

    Ok(JsValue::from(result))
}

/// Percent-decode a string
fn percent_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            result.push('%');
            result.push_str(&hex);
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }

    result
}

/// url.format(urlObject)
/// Format a URL object into a URL string
fn url_format(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let url_obj = args
        .get(0)
        .ok_or_else(|| JsNativeError::typ().with_message("url.format requires an argument"))?;

    // If it's a string, return as-is
    if let Some(s) = url_obj.as_string() {
        return Ok(JsValue::from(s.clone()));
    }

    // If it's a URL object, use href
    if let Some(obj) = url_obj.as_object() {
        // Check if it's a WHATWG URL object (has href getter)
        if let Ok(href) = obj.get(js_string!("href"), context) {
            if let Some(s) = href.as_string() {
                return Ok(JsValue::from(s.clone()));
            }
        }

        // Legacy URL object format
        let mut result = String::new();

        // Protocol
        if let Ok(proto) = obj.get(js_string!("protocol"), context) {
            if let Some(s) = proto.as_string() {
                let p = s.to_std_string_escaped();
                result.push_str(&p);
                if !p.ends_with(':') {
                    result.push(':');
                }
            }
        }

        // Slashes
        let has_slashes = obj
            .get(js_string!("slashes"), context)
            .map(|v| v.to_boolean())
            .unwrap_or(false);

        // Check for http/https/ftp/etc that need slashes
        let proto_needs_slashes = result.starts_with("http:")
            || result.starts_with("https:")
            || result.starts_with("ftp:")
            || result.starts_with("file:")
            || result.starts_with("ws:")
            || result.starts_with("wss:");

        if has_slashes || proto_needs_slashes {
            result.push_str("//");
        }

        // Auth
        if let Ok(auth) = obj.get(js_string!("auth"), context) {
            if let Some(s) = auth.as_string() {
                let a = s.to_std_string_escaped();
                if !a.is_empty() {
                    result.push_str(&a);
                    result.push('@');
                }
            }
        }

        // Host or hostname+port
        let host = obj.get(js_string!("host"), context).ok();
        let hostname = obj.get(js_string!("hostname"), context).ok();
        let port = obj.get(js_string!("port"), context).ok();

        if let Some(h) = host.and_then(|v| v.as_string().map(|s| s.to_std_string_escaped())) {
            if !h.is_empty() {
                result.push_str(&h);
            }
        } else if let Some(h) =
            hostname.and_then(|v| v.as_string().map(|s| s.to_std_string_escaped()))
        {
            if !h.is_empty() {
                result.push_str(&h);
                if let Some(p) = port.and_then(|v| {
                    if v.is_number() {
                        Some(v.to_number(context).ok()?.to_string())
                    } else {
                        v.as_string().map(|s| s.to_std_string_escaped())
                    }
                }) {
                    if !p.is_empty() {
                        result.push(':');
                        result.push_str(&p);
                    }
                }
            }
        }

        // Pathname
        if let Ok(pathname) = obj.get(js_string!("pathname"), context) {
            if let Some(s) = pathname.as_string() {
                let p = s.to_std_string_escaped();
                if !p.is_empty() {
                    if !p.starts_with('/') && !result.is_empty() {
                        result.push('/');
                    }
                    result.push_str(&p);
                }
            }
        }

        // Search/query
        let search = obj.get(js_string!("search"), context).ok();
        let query = obj.get(js_string!("query"), context).ok();

        if let Some(s) = search.and_then(|v| v.as_string().map(|s| s.to_std_string_escaped())) {
            if !s.is_empty() {
                if !s.starts_with('?') {
                    result.push('?');
                }
                result.push_str(&s);
            }
        } else if let Some(q) = query {
            if let Some(s) = q.as_string() {
                let qs = s.to_std_string_escaped();
                if !qs.is_empty() {
                    result.push('?');
                    result.push_str(&qs);
                }
            } else if q.is_object() {
                // Stringify query object
                let qs = stringify_query_object(&q, context)?;
                if !qs.is_empty() {
                    result.push('?');
                    result.push_str(&qs);
                }
            }
        }

        // Hash
        if let Ok(hash) = obj.get(js_string!("hash"), context) {
            if let Some(s) = hash.as_string() {
                let h = s.to_std_string_escaped();
                if !h.is_empty() {
                    if !h.starts_with('#') {
                        result.push('#');
                    }
                    result.push_str(&h);
                }
            }
        }

        return Ok(JsValue::from(js_string!(result)));
    }

    Ok(JsValue::from(js_string!("")))
}

/// Stringify a query object to query string
fn stringify_query_object(obj: &JsValue, context: &mut Context) -> JsResult<String> {
    let obj = match obj.as_object() {
        Some(o) => o,
        None => return Ok(String::new()),
    };

    let keys = obj.own_property_keys(context)?;
    let mut pairs = Vec::new();

    for key in keys {
        let key_str = key.to_string();
        let value = obj.get(key.clone(), context)?;

        // Check if value is an array-like object (has numeric length)
        let is_array_like = if let Some(val_obj) = value.as_object() {
            val_obj.has_property(js_string!("length"), context)?
                && val_obj.get(js_string!("length"), context)?.is_number()
        } else {
            false
        };

        if is_array_like {
            let arr = value.as_object().unwrap();
            let len = arr.get(js_string!("length"), context)?.to_number(context)? as u32;
            for i in 0..len {
                let v = arr.get(i, context)?;
                let v_str = v.to_string(context)?.to_std_string_escaped();
                pairs.push(format!(
                    "{}={}",
                    percent_encode(&key_str),
                    percent_encode(&v_str)
                ));
            }
        } else {
            let v_str = value.to_string(context)?.to_std_string_escaped();
            pairs.push(format!(
                "{}={}",
                percent_encode(&key_str),
                percent_encode(&v_str)
            ));
        }
    }

    Ok(pairs.join("&"))
}

/// Percent-encode for query strings
fn percent_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);

    for c in s.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => {
                result.push(c);
            }
            ' ' => {
                result.push('+');
            }
            _ => {
                for byte in c.to_string().as_bytes() {
                    result.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }

    result
}

/// url.resolve(from, to)
/// Resolve a target URL relative to a base URL
fn url_resolve(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let from = args
        .get(0)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();

    let to = args
        .get(1)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();

    // Use JavaScript's URL API to resolve
    let js_code = format!(
        r#"
        (function() {{
            try {{
                const resolved = new URL({:?}, new URL({:?}, 'resolve://'));
                if (resolved.protocol === 'resolve:') {{
                    return resolved.pathname + resolved.search + resolved.hash;
                }}
                return resolved.href;
            }} catch (e) {{
                return {:?};
            }}
        }})()
        "#,
        to, from, to
    );

    context.eval(Source::from_bytes(js_code.as_bytes()))
}

/// url.domainToASCII(domain)
/// Returns the Punycode ASCII serialization of the domain
fn domain_to_ascii(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let domain = args
        .get(0)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();

    // Use idna crate for proper Punycode encoding
    match idna::domain_to_ascii(&domain) {
        Ok(ascii) => Ok(JsValue::from(js_string!(ascii))),
        Err(_) => Ok(JsValue::from(js_string!(""))),
    }
}

/// url.domainToUnicode(domain)
/// Returns the Unicode serialization of the domain
fn domain_to_unicode(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let domain = args
        .get(0)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();

    // Use idna crate for proper Punycode decoding
    let (unicode, _result) = idna::domain_to_unicode(&domain);
    Ok(JsValue::from(js_string!(unicode)))
}

/// url.fileURLToPath(url)
/// Convert a file URL to a local file path
fn file_url_to_path(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let url_arg = args.get(0).ok_or_else(|| {
        JsNativeError::typ().with_message("fileURLToPath requires a URL argument")
    })?;

    // Get the URL string
    let url_str = if let Some(s) = url_arg.as_string() {
        s.to_std_string_escaped()
    } else if let Some(obj) = url_arg.as_object() {
        // URL object - get href
        obj.get(js_string!("href"), context)?
            .as_string()
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default()
    } else {
        return Err(JsNativeError::typ()
            .with_message("fileURLToPath requires a string or URL object")
            .into());
    };

    // Must be a file: URL
    if !url_str.starts_with("file:") {
        return Err(JsNativeError::typ()
            .with_message("fileURLToPath requires a file: URL")
            .into());
    }

    // Parse and convert
    let path = file_url_to_path_impl(&url_str)?;
    Ok(JsValue::from(js_string!(path)))
}

/// Implementation of file URL to path conversion
fn file_url_to_path_impl(url: &str) -> JsResult<String> {
    // Remove file:// prefix
    let path_part = if url.starts_with("file:///") {
        &url[8..] // file:/// (3 slashes)
    } else if url.starts_with("file://") {
        &url[7..] // file:// with host
    } else {
        &url[5..] // file: only
    };

    // Percent-decode the path
    let decoded = percent_decode(path_part);

    // Handle Windows paths
    #[cfg(windows)]
    {
        // Convert forward slashes to backslashes
        let mut path = decoded.replace('/', "\\");

        // Handle drive letter (e.g., /C:/path -> C:\path)
        if path.starts_with('\\') && path.len() >= 3 {
            if path.chars().nth(2) == Some(':') {
                path = path[1..].to_string();
            }
        }

        Ok(path)
    }

    #[cfg(not(windows))]
    {
        // Unix paths - ensure leading slash
        if decoded.starts_with('/') {
            Ok(decoded)
        } else {
            Ok(format!("/{}", decoded))
        }
    }
}

/// url.pathToFileURL(path)
/// Convert a local file path to a file URL
fn path_to_file_url(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let path = args
        .get(0)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .ok_or_else(|| JsNativeError::typ().with_message("pathToFileURL requires a path string"))?;

    let file_url = path_to_file_url_impl(&path);

    // Return a URL object
    let js_code = format!(r#"new URL({:?})"#, file_url);
    context.eval(Source::from_bytes(js_code.as_bytes()))
}

/// Implementation of path to file URL conversion
fn path_to_file_url_impl(path: &str) -> String {
    let mut url = String::from("file://");

    #[cfg(windows)]
    {
        // Make path absolute if not already
        let abs_path = if path.len() >= 2 && path.chars().nth(1) == Some(':') {
            // Already absolute (C:\...)
            path.to_string()
        } else if path.starts_with("\\\\") {
            // UNC path
            path.to_string()
        } else {
            // Relative path - would need to resolve, but for now just use as-is
            path.to_string()
        };

        // Handle UNC paths
        if abs_path.starts_with("\\\\") {
            // file://server/share/path
            url.push_str(&abs_path[2..].replace('\\', "/"));
        } else {
            // file:///C:/path
            url.push('/');
            // Percent-encode special characters
            for c in abs_path.chars() {
                match c {
                    '\\' => url.push('/'),
                    '#' => url.push_str("%23"),
                    '?' => url.push_str("%3F"),
                    '%' => url.push_str("%25"),
                    ' ' => url.push_str("%20"),
                    _ => url.push(c),
                }
            }
        }
    }

    #[cfg(not(windows))]
    {
        // Unix path
        url.push('/');
        for c in path.chars() {
            match c {
                '#' => url.push_str("%23"),
                '?' => url.push_str("%3F"),
                '%' => url.push_str("%25"),
                ' ' => url.push_str("%20"),
                _ => url.push(c),
            }
        }
    }

    url
}

/// url.urlToHttpOptions(url)
/// Convert a URL to http.request options
fn url_to_http_options(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let url_arg = args.get(0).ok_or_else(|| {
        JsNativeError::typ().with_message("urlToHttpOptions requires a URL argument")
    })?;

    // Handle URL object or string
    let (protocol, hostname, port, pathname, search, hash, href, username, password) =
        if let Some(obj) = url_arg.as_object() {
            let protocol = obj
                .get(js_string!("protocol"), context)?
                .as_string()
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let hostname = obj
                .get(js_string!("hostname"), context)?
                .as_string()
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let port = obj
                .get(js_string!("port"), context)?
                .as_string()
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let pathname = obj
                .get(js_string!("pathname"), context)?
                .as_string()
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let search = obj
                .get(js_string!("search"), context)?
                .as_string()
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let hash = obj
                .get(js_string!("hash"), context)?
                .as_string()
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let href = obj
                .get(js_string!("href"), context)?
                .as_string()
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let username = obj
                .get(js_string!("username"), context)?
                .as_string()
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let password = obj
                .get(js_string!("password"), context)?
                .as_string()
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();

            (
                protocol, hostname, port, pathname, search, hash, href, username, password,
            )
        } else {
            return Err(JsNativeError::typ()
                .with_message("urlToHttpOptions requires a URL object")
                .into());
        };

    // Build options object
    let options = ObjectInitializer::new(context).build();

    options.set(
        js_string!("protocol"),
        js_string!(protocol.clone()),
        false,
        context,
    )?;
    options.set(
        js_string!("hostname"),
        js_string!(hostname.clone()),
        false,
        context,
    )?;

    if !hash.is_empty() {
        options.set(js_string!("hash"), js_string!(hash), false, context)?;
    }

    if !search.is_empty() {
        options.set(
            js_string!("search"),
            js_string!(search.clone()),
            false,
            context,
        )?;
    }

    options.set(
        js_string!("pathname"),
        js_string!(pathname.clone()),
        false,
        context,
    )?;

    // path = pathname + search
    let path = format!("{}{}", pathname, search);
    options.set(js_string!("path"), js_string!(path), false, context)?;

    options.set(js_string!("href"), js_string!(href), false, context)?;

    // Port as number if present
    if !port.is_empty() {
        if let Ok(port_num) = port.parse::<u16>() {
            options.set(
                js_string!("port"),
                JsValue::from(port_num as i32),
                false,
                context,
            )?;
        }
    }

    // Auth (username:password)
    if !username.is_empty() || !password.is_empty() {
        let auth = if !password.is_empty() {
            format!("{}:{}", username, password)
        } else {
            username
        };
        options.set(js_string!("auth"), js_string!(auth), false, context)?;
    }

    Ok(JsValue::from(options))
}
