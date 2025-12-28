//! Node.js util module - High-performance implementation
//!
//! Provides utility functions for Node.js compatibility including:
//! - promisify/callbackify for async conversion
//! - format/formatWithOptions for string formatting
//! - inspect for object inspection
//! - deprecate for deprecation warnings
//! - types utilities for type checking

use boa_engine::{
    Context, JsArgs, JsNativeError, JsObject, JsResult, JsString, JsValue, NativeFunction, Source,
    js_string,
};
use std::fmt::Write as _;

/// Register the util module with the JavaScript context
pub fn register_util_module(context: &mut Context) -> JsResult<()> {
    let util = JsObject::with_object_proto(context.intrinsics());

    // Create JavaScript helper functions first to avoid Boa closure issues
    register_util_helpers(context)?;

    // util.promisify(original) - Convert callback-based function to Promise-based
    let promisify_fn = NativeFunction::from_fn_ptr(util_promisify);
    util.set(
        js_string!("promisify"),
        promisify_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // util.callbackify(original) - Convert Promise-based function to callback-based
    let callbackify_fn = NativeFunction::from_fn_ptr(util_callbackify);
    util.set(
        js_string!("callbackify"),
        callbackify_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // util.format(format, ...args) - Printf-like string formatting
    let format_fn = NativeFunction::from_fn_ptr(util_format);
    util.set(
        js_string!("format"),
        format_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // util.formatWithOptions(inspectOptions, format, ...args)
    let format_with_options_fn = NativeFunction::from_fn_ptr(util_format_with_options);
    util.set(
        js_string!("formatWithOptions"),
        format_with_options_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // util.inspect(object, options) - Object inspection
    let inspect_fn = NativeFunction::from_fn_ptr(util_inspect);
    util.set(
        js_string!("inspect"),
        inspect_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // util.deprecate(fn, msg, code) - Mark function as deprecated
    let deprecate_fn = NativeFunction::from_fn_ptr(util_deprecate);
    util.set(
        js_string!("deprecate"),
        deprecate_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // util.isDeepStrictEqual(val1, val2) - Deep equality check
    let is_deep_strict_equal_fn = NativeFunction::from_fn_ptr(util_is_deep_strict_equal);
    util.set(
        js_string!("isDeepStrictEqual"),
        is_deep_strict_equal_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // util.types - Type checking utilities (JavaScript-based for speed and compatibility)
    create_types_object(context, &util)?;

    // util.inherits(constructor, superConstructor) - Legacy prototype inheritance
    let inherits_fn = NativeFunction::from_fn_ptr(util_inherits);
    util.set(
        js_string!("inherits"),
        inherits_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // util.debuglog(section) - Conditional debug logging
    let debuglog_fn = NativeFunction::from_fn_ptr(util_debuglog);
    util.set(
        js_string!("debuglog"),
        debuglog_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // util.getSystemErrorName(err) - Get system error name from error code
    let get_system_error_name_fn = NativeFunction::from_fn_ptr(util_get_system_error_name);
    util.set(
        js_string!("getSystemErrorName"),
        get_system_error_name_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // util.getSystemErrorMap() - Get all system error codes
    let get_system_error_map_fn = NativeFunction::from_fn_ptr(util_get_system_error_map);
    util.set(
        js_string!("getSystemErrorMap"),
        get_system_error_map_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // Set util as global
    context
        .global_object()
        .set(js_string!("util"), util, false, context)?;

    Ok(())
}

/// Register JavaScript helper functions to avoid Boa closure issues
fn register_util_helpers(context: &mut Context) -> JsResult<()> {
    let code = r#"
        // Helper for promisify - stored globally to avoid closure issues
        globalThis.__viper_promisify_helper = function(original) {
            if (typeof original !== 'function') {
                throw new TypeError('The "original" argument must be of type function');
            }

            function promisified(...args) {
                return new Promise((resolve, reject) => {
                    function callback(err, value) {
                        if (err) {
                            reject(err);
                        } else {
                            resolve(value);
                        }
                    }

                    args.push(callback);

                    try {
                        original(...args);
                    } catch (err) {
                        reject(err);
                    }
                });
            }

            return promisified;
        };

        // Helper for callbackify - stored globally to avoid closure issues
        globalThis.__viper_callbackify_helper = function(original) {
            if (typeof original !== 'function') {
                throw new TypeError('The "original" argument must be of type function');
            }

            function callbackified(...args) {
                if (args.length === 0) {
                    throw new TypeError('The last argument must be a callback function');
                }

                const callback = args[args.length - 1];
                if (typeof callback !== 'function') {
                    throw new TypeError('The last argument must be a callback function');
                }

                const callArgs = args.slice(0, -1);

                let result;
                try {
                    result = original(...callArgs);
                } catch (err) {
                    queueMicrotask(() => callback(err));
                    return;
                }

                if (result && typeof result.then === 'function') {
                    result.then(
                        (value) => {
                            queueMicrotask(() => callback(null, value));
                        },
                        (err) => {
                            // Handle falsy rejection values as per Node.js spec
                            if (!err) {
                                const wrappedErr = new Error('Promise was rejected with a falsy value');
                                wrappedErr.reason = err;
                                queueMicrotask(() => callback(wrappedErr));
                            } else {
                                queueMicrotask(() => callback(err));
                            }
                        }
                    );
                } else {
                    queueMicrotask(() => callback(null, result));
                }
            }

            return callbackified;
        };

        // Helper for deprecate - stored globally to avoid closure issues
        globalThis.__viper_deprecate_helper = function(fn, msg, code) {
            if (typeof fn !== 'function') {
                throw new TypeError('The "fn" argument must be of type function');
            }

            const key = '__viper_warn_' + (code || msg);

            return function(...args) {
                if (!globalThis[key]) {
                    globalThis[key] = true;
                    console.warn('DeprecationWarning:', msg);
                }
                return fn(...args);
            };
        };
    "#;

    context.eval(Source::from_bytes(code.as_bytes()))?;
    Ok(())
}

/// util.promisify(original) - Convert callback-based function to Promise-based
fn util_promisify(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let original = args.get_or_undefined(0);

    // Get helper function from global
    let helper = context
        .global_object()
        .get(js_string!("__viper_promisify_helper"), context)?;

    if let Some(func) = helper.as_callable() {
        func.call(&JsValue::undefined(), &[original.clone()], context)
    } else {
        Err(JsNativeError::typ()
            .with_message("Promisify helper not found")
            .into())
    }
}

/// util.callbackify(original) - Convert Promise-based function to callback-based
fn util_callbackify(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let original = args.get_or_undefined(0);

    // Get helper function from global
    let helper = context
        .global_object()
        .get(js_string!("__viper_callbackify_helper"), context)?;

    if let Some(func) = helper.as_callable() {
        func.call(&JsValue::undefined(), &[original.clone()], context)
    } else {
        Err(JsNativeError::typ()
            .with_message("Callbackify helper not found")
            .into())
    }
}

/// util.format(format, ...args) - Printf-like string formatting
fn util_format(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    if args.is_empty() {
        return Ok(JsValue::from(js_string!("")));
    }

    let format_str = args[0].to_string(context)?;
    let format = format_str.to_std_string_escaped();

    let mut result = String::new();
    let mut arg_index = 1;
    let mut chars = format.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            if let Some(&next) = chars.peek() {
                match next {
                    's' => {
                        // String - converts almost anything except BigInt/Symbol
                        chars.next();
                        if arg_index < args.len() {
                            let val = &args[arg_index];
                            if val.is_symbol() {
                                result.push_str("[Symbol]");
                            } else if val.is_bigint() {
                                let s = val.to_string(context)?;
                                result.push_str(&s.to_std_string_escaped());
                                result.push('n');
                            } else {
                                let s = val.to_string(context)?;
                                result.push_str(&s.to_std_string_escaped());
                            }
                            arg_index += 1;
                        } else {
                            result.push_str("%s");
                        }
                    }
                    'd' | 'i' => {
                        // Number / Integer
                        chars.next();
                        if arg_index < args.len() {
                            let val = &args[arg_index];
                            if let Some(num) = val.as_number() {
                                if next == 'i' {
                                    write!(&mut result, "{}", num as i64).unwrap();
                                } else {
                                    write!(&mut result, "{}", num).unwrap();
                                }
                            } else if val.is_bigint() {
                                let s = val.to_string(context)?;
                                result.push_str(&s.to_std_string_escaped());
                            } else if val.is_symbol() {
                                result.push_str("NaN");
                            } else {
                                result.push_str("NaN");
                            }
                            arg_index += 1;
                        } else {
                            result.push('%');
                            result.push(next);
                        }
                    }
                    'f' => {
                        // Float
                        chars.next();
                        if arg_index < args.len() {
                            let val = &args[arg_index];
                            if let Some(num) = val.as_number() {
                                write!(&mut result, "{}", num).unwrap();
                            } else if val.is_symbol() {
                                result.push_str("NaN");
                            } else {
                                result.push_str("NaN");
                            }
                            arg_index += 1;
                        } else {
                            result.push_str("%f");
                        }
                    }
                    'j' => {
                        // JSON
                        chars.next();
                        if arg_index < args.len() {
                            let val = &args[arg_index];
                            match stringify_json(val, context) {
                                Ok(json_str) => result.push_str(&json_str),
                                Err(_) => result.push_str("[Circular]"),
                            }
                            arg_index += 1;
                        } else {
                            result.push_str("%j");
                        }
                    }
                    'o' | 'O' => {
                        // Object inspection
                        chars.next();
                        if arg_index < args.len() {
                            let val = &args[arg_index];
                            let inspect_str = inspect_value(val, context)?;
                            result.push_str(&inspect_str);
                            arg_index += 1;
                        } else {
                            result.push('%');
                            result.push(next);
                        }
                    }
                    'c' => {
                        // CSS - ignored in Node.js
                        chars.next();
                        if arg_index < args.len() {
                            arg_index += 1;
                        }
                    }
                    '%' => {
                        // Escaped percent
                        chars.next();
                        result.push('%');
                    }
                    _ => {
                        result.push(ch);
                    }
                }
            } else {
                result.push(ch);
            }
        } else {
            result.push(ch);
        }
    }

    // Append remaining arguments with spaces
    while arg_index < args.len() {
        result.push(' ');
        let val = &args[arg_index];
        let s = val.to_string(context)?;
        result.push_str(&s.to_std_string_escaped());
        arg_index += 1;
    }

    Ok(JsValue::from(js_string!(result)))
}

/// util.formatWithOptions(inspectOptions, format, ...args)
fn util_format_with_options(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    if args.len() < 2 {
        return util_format(_this, &[], context);
    }
    // TODO: Actually use inspectOptions for %o and %O formatting
    util_format(_this, &args[1..], context)
}

/// util.inspect(object, options) - Object inspection
fn util_inspect(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let obj = args.get_or_undefined(0);
    // TODO: Parse and use options (depth, colors, showHidden, etc.)
    let result = inspect_value(obj, context)?;
    Ok(JsValue::from(js_string!(result)))
}

/// Helper: Inspect a value and return string representation
fn inspect_value(value: &JsValue, context: &mut Context) -> JsResult<String> {
    if value.is_undefined() {
        return Ok("undefined".to_string());
    }
    if value.is_null() {
        return Ok("null".to_string());
    }
    if value.is_boolean() {
        return Ok(value.to_boolean().to_string());
    }
    if let Some(num) = value.as_number() {
        if num.is_nan() {
            return Ok("NaN".to_string());
        }
        if num.is_infinite() {
            return Ok(if num.is_sign_positive() {
                "Infinity"
            } else {
                "-Infinity"
            }
            .to_string());
        }
        return Ok(num.to_string());
    }
    if value.is_string() {
        let s = value.to_string(context)?;
        return Ok(format!("'{}'", s.to_std_string_escaped()));
    }
    if value.is_symbol() {
        return Ok("Symbol()".to_string());
    }
    if value.is_bigint() {
        let s = value.to_string(context)?;
        return Ok(format!("{}n", s.to_std_string_escaped()));
    }
    if value.is_object() {
        return stringify_json(value, context);
    }

    Ok("[Object]".to_string())
}

/// Helper: Stringify value as JSON (with circular reference protection)
fn stringify_json(value: &JsValue, context: &mut Context) -> JsResult<String> {
    let json_obj = context.global_object().get(js_string!("JSON"), context)?;
    let stringify_fn: Option<_> = json_obj
        .as_object()
        .and_then(|o| o.get(js_string!("stringify"), context).ok())
        .and_then(|v| v.as_callable().map(|c| c.clone()));

    if let Some(stringify) = stringify_fn {
        match stringify.call(&json_obj, &[value.clone()], context) {
            Ok(result) => {
                let s = result.to_string(context)?;
                Ok(s.to_std_string_escaped())
            }
            Err(_) => Ok("[Circular]".to_string()),
        }
    } else {
        Ok("[Object]".to_string())
    }
}

/// util.deprecate(fn, msg, code) - Mark function as deprecated
fn util_deprecate(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let func = args.get_or_undefined(0);
    let msg = args.get_or_undefined(1);
    let code = args.get(2).cloned().unwrap_or(JsValue::undefined());

    // Get helper function from global
    let helper = context
        .global_object()
        .get(js_string!("__viper_deprecate_helper"), context)?;

    if let Some(func_helper) = helper.as_callable() {
        func_helper.call(
            &JsValue::undefined(),
            &[func.clone(), msg.clone(), code],
            context,
        )
    } else {
        Err(JsNativeError::typ()
            .with_message("Deprecate helper not found")
            .into())
    }
}

/// util.isDeepStrictEqual(val1, val2) - Deep equality check
fn util_is_deep_strict_equal(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let val1 = args.get_or_undefined(0);
    let val2 = args.get_or_undefined(1);

    let equal = deep_strict_equal(val1, val2, context)?;
    Ok(JsValue::from(equal))
}

/// Helper: Deep strict equality comparison (handles edge cases)
fn deep_strict_equal(a: &JsValue, b: &JsValue, context: &mut Context) -> JsResult<bool> {
    // Same reference
    if std::ptr::eq(a, b) {
        return Ok(true);
    }

    // Type mismatch
    if a.get_type() != b.get_type() {
        return Ok(false);
    }

    // Primitives
    if a.is_undefined() && b.is_undefined() {
        return Ok(true);
    }
    if a.is_null() && b.is_null() {
        return Ok(true);
    }
    if a.is_boolean() && b.is_boolean() {
        return Ok(a.to_boolean() == b.to_boolean());
    }
    if let (Some(n1), Some(n2)) = (a.as_number(), b.as_number()) {
        // Handle NaN === NaN
        if n1.is_nan() && n2.is_nan() {
            return Ok(true);
        }
        // Handle +0 vs -0
        if n1 == 0.0 && n2 == 0.0 {
            return Ok(n1.is_sign_positive() == n2.is_sign_positive());
        }
        return Ok(n1 == n2);
    }
    if a.is_string() && b.is_string() {
        let s1 = a.to_string(context)?;
        let s2 = b.to_string(context)?;
        return Ok(s1 == s2);
    }
    if a.is_symbol() && b.is_symbol() {
        // Symbols are compared by reference
        return Ok(false); // Different symbol objects are never equal
    }
    if a.is_bigint() && b.is_bigint() {
        let s1 = a.to_string(context)?;
        let s2 = b.to_string(context)?;
        return Ok(s1 == s2);
    }

    // Object comparison - reference equality for now
    // TODO: Implement deep object comparison
    if a.is_object() && b.is_object() {
        return Ok(a.as_object().map(|o| o.as_ref() as *const _)
            == b.as_object().map(|o| o.as_ref() as *const _));
    }

    Ok(false)
}

/// util.inherits(constructor, superConstructor) - Legacy prototype inheritance
fn util_inherits(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let constructor = args.get_or_undefined(0);
    let super_constructor = args.get_or_undefined(1);

    if !constructor.is_callable() || !super_constructor.is_callable() {
        return Err(JsNativeError::typ()
            .with_message("Both arguments must be constructors")
            .into());
    }

    let code = r#"
        (function(ctor, superCtor) {
            if (typeof ctor !== 'function' || typeof superCtor !== 'function') {
                throw new TypeError('Both arguments must be constructors');
            }
            ctor.super_ = superCtor;
            Object.setPrototypeOf(ctor.prototype, superCtor.prototype);
        })
    "#;

    let wrapper = context.eval(Source::from_bytes(code.as_bytes()))?;
    wrapper
        .as_callable()
        .ok_or_else(|| JsNativeError::typ().with_message("Failed to create inherits wrapper"))?
        .call(
            &JsValue::undefined(),
            &[constructor.clone(), super_constructor.clone()],
            context,
        )?;

    Ok(JsValue::undefined())
}

/// util.debuglog(section) - Conditional debug logging
fn util_debuglog(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let section = args.get_or_undefined(0).to_string(context)?;
    let section_str = section.to_std_string_escaped();

    let node_debug = std::env::var("NODE_DEBUG").unwrap_or_default();
    let enabled = node_debug
        .split(',')
        .any(|s| s.trim() == section_str || s.trim() == "*");

    let code = if enabled {
        format!(
            r#"
            (function() {{
                const log = function(...args) {{
                    const pid = globalThis.process ? globalThis.process.pid : 0;
                    console.log(`{} ${{pid}}:`, ...args);
                }};
                log.enabled = true;
                return log;
            }})()
        "#,
            section_str.to_uppercase()
        )
    } else {
        r#"
            (function() {
                const noop = function() {};
                noop.enabled = false;
                return noop;
            })()
        "#
        .to_string()
    };

    context.eval(Source::from_bytes(code.as_bytes()))
}

/// util.getSystemErrorName(err) - Get system error name from error code
fn util_get_system_error_name(
    _this: &JsValue,
    args: &[JsValue],
    _context: &mut Context,
) -> JsResult<JsValue> {
    let err = args.get_or_undefined(0);
    let code = err
        .as_number()
        .ok_or_else(|| JsNativeError::typ().with_message("Argument must be a number"))?
        as i32;

    let name = get_error_name(code);
    Ok(JsValue::from(js_string!(name)))
}

/// util.getSystemErrorMap() - Get all system error codes
fn util_get_system_error_map(
    _this: &JsValue,
    _args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let map_ctor = context.global_object().get(js_string!("Map"), context)?;
    let map_obj = map_ctor
        .as_constructor()
        .ok_or_else(|| JsNativeError::typ().with_message("Map constructor not found"))?
        .construct(&[], None, context)?;

    let map_val = JsValue::from(map_obj.clone());

    for (code, name) in ERROR_CODES {
        let set_fn = map_obj.get(js_string!("set"), context)?;
        if let Some(set) = set_fn.as_callable() {
            set.call(
                &map_val,
                &[JsValue::from(*code), JsValue::from(js_string!(*name))],
                context,
            )?;
        }
    }

    Ok(map_val)
}

/// Helper: Get error name from code
fn get_error_name(code: i32) -> &'static str {
    for (err_code, name) in ERROR_CODES {
        if *err_code == code {
            return name;
        }
    }
    "UNKNOWN"
}

/// Common system error codes
const ERROR_CODES: &[(i32, &str)] = &[
    (-2, "ENOENT"),
    (-13, "EACCES"),
    (-17, "EEXIST"),
    (-20, "ENOTDIR"),
    (-21, "EISDIR"),
    (-22, "EINVAL"),
    (-27, "EFBIG"),
    (-28, "ENOSPC"),
    (-39, "ENOTEMPTY"),
    (-48, "EADDRINUSE"),
    (-61, "ECONNREFUSED"),
];

/// Create util.types object with JavaScript-based type checking (Bun-style fast checks)
fn create_types_object(context: &mut Context, util: &JsObject) -> JsResult<()> {
    // Use JavaScript for type checking - this is fast and avoids Boa internal API issues
    let types_code = r#"
        (function() {
            return {
                isAnyArrayBuffer: (v) => v instanceof ArrayBuffer || (typeof SharedArrayBuffer !== 'undefined' && v instanceof SharedArrayBuffer),
                isArrayBufferView: (v) => ArrayBuffer.isView(v),
                isArgumentsObject: (v) => Object.prototype.toString.call(v) === '[object Arguments]',
                isArrayBuffer: (v) => v instanceof ArrayBuffer,
                isAsyncFunction: (v) => Object.prototype.toString.call(v) === '[object AsyncFunction]',
                isBigInt64Array: (v) => Object.prototype.toString.call(v) === '[object BigInt64Array]',
                isBigIntObject: (v) => Object.prototype.toString.call(v) === '[object BigInt]' && typeof v === 'object',
                isBigUint64Array: (v) => Object.prototype.toString.call(v) === '[object BigUint64Array]',
                isBooleanObject: (v) => Object.prototype.toString.call(v) === '[object Boolean]' && typeof v === 'object',
                isBoxedPrimitive: (v) => {
                    const type = Object.prototype.toString.call(v);
                    return type === '[object Boolean]' || type === '[object Number]' ||
                           type === '[object String]' || type === '[object Symbol]' ||
                           type === '[object BigInt]';
                },
                isCryptoKey: (v) => false, // Not implemented yet
                isDataView: (v) => v instanceof DataView,
                isDate: (v) => v instanceof Date,
                isExternal: (v) => false, // Internal type
                isFloat32Array: (v) => v instanceof Float32Array,
                isFloat64Array: (v) => v instanceof Float64Array,
                isGeneratorFunction: (v) => Object.prototype.toString.call(v) === '[object GeneratorFunction]',
                isGeneratorObject: (v) => Object.prototype.toString.call(v) === '[object Generator]',
                isInt8Array: (v) => v instanceof Int8Array,
                isInt16Array: (v) => v instanceof Int16Array,
                isInt32Array: (v) => v instanceof Int32Array,
                isKeyObject: (v) => false, // Not implemented yet
                isMap: (v) => v instanceof Map,
                isMapIterator: (v) => Object.prototype.toString.call(v) === '[object Map Iterator]',
                isModuleNamespaceObject: (v) => Object.prototype.toString.call(v) === '[object Module]',
                isNativeError: (v) => v instanceof Error,
                isNumberObject: (v) => Object.prototype.toString.call(v) === '[object Number]' && typeof v === 'object',
                isPromise: (v) => v instanceof Promise,
                isProxy: (v) => false, // Cannot reliably detect proxies
                isRegExp: (v) => v instanceof RegExp,
                isSet: (v) => v instanceof Set,
                isSetIterator: (v) => Object.prototype.toString.call(v) === '[object Set Iterator]',
                isSharedArrayBuffer: (v) => typeof SharedArrayBuffer !== 'undefined' && v instanceof SharedArrayBuffer,
                isStringObject: (v) => Object.prototype.toString.call(v) === '[object String]' && typeof v === 'object',
                isSymbolObject: (v) => Object.prototype.toString.call(v) === '[object Symbol]' && typeof v === 'object',
                isTypedArray: (v) => ArrayBuffer.isView(v) && !(v instanceof DataView),
                isUint8Array: (v) => v instanceof Uint8Array,
                isUint8ClampedArray: (v) => v instanceof Uint8ClampedArray,
                isUint16Array: (v) => v instanceof Uint16Array,
                isUint32Array: (v) => v instanceof Uint32Array,
                isWeakMap: (v) => v instanceof WeakMap,
                isWeakSet: (v) => v instanceof WeakSet,
            };
        })()
    "#;

    let types = context.eval(Source::from_bytes(types_code.as_bytes()))?;
    util.set(js_string!("types"), types, false, context)?;

    Ok(())
}
