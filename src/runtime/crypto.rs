//! Crypto API - Web Crypto compatible APIs
//!
//! Provides:
//! - crypto.randomUUID() - Generate a random UUID v4
//! - crypto.getRandomValues(array) - Fill typed array with random values
//! - crypto.randomBytes(size) - Generate random bytes (Node.js compat)

use boa_engine::{
    js_string, object::builtins::JsUint8Array,
    Context, JsNativeError, JsResult, JsValue, NativeFunction, Source,
};
use rand::Rng;
use uuid::Uuid;

/// Register the crypto object
pub fn register_crypto(context: &mut Context) -> JsResult<()> {
    // crypto.randomUUID()
    let random_uuid_fn = NativeFunction::from_fn_ptr(|_this, _args, _context| {
        let uuid = Uuid::new_v4().to_string();
        Ok(JsValue::from(js_string!(uuid)))
    });
    context.global_object().set(
        js_string!("__viper_random_uuid"),
        random_uuid_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // crypto.getRandomValues(typedArray)
    let get_random_values_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let array = args.get(0).ok_or_else(|| {
            JsNativeError::typ().with_message("getRandomValues requires a typed array argument")
        })?;

        let obj = array.as_object().ok_or_else(|| {
            JsNativeError::typ().with_message("Argument must be a typed array")
        })?;

        // Get the underlying buffer and fill with random bytes
        if let Ok(uint8) = JsUint8Array::from_object(obj.clone()) {
            let len = uint8.length(context)?;
            let mut rng = rand::rng();

            for i in 0..len {
                let random_byte: u8 = rng.random();
                uint8.set(i, JsValue::from(random_byte as i32), true, context)?;
            }

            return Ok(array.clone());
        }

        // For other typed arrays, try to get length and fill
        if let Ok(length_val) = obj.get(js_string!("length"), context) {
            if let Some(len) = length_val.as_number() {
                let len = len as usize;
                let mut rng = rand::rng();

                for i in 0..len {
                    // For Uint8Array-like, use bytes
                    // For larger types, this is an approximation
                    let random_val: u32 = rng.random();
                    obj.set(i, JsValue::from(random_val as i32), true, context)?;
                }

                return Ok(array.clone());
            }
        }

        Err(JsNativeError::typ()
            .with_message("Argument must be a typed array")
            .into())
    });
    context.global_object().set(
        js_string!("__viper_get_random_values"),
        get_random_values_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // crypto.randomBytes(size) - Node.js compatible
    let random_bytes_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let size = args
            .get(0)
            .map(|v| v.to_u32(context))
            .transpose()?
            .unwrap_or(0) as usize;

        if size > 65536 {
            return Err(JsNativeError::range()
                .with_message("Size must be less than 65536")
                .into());
        }

        let mut bytes = vec![0u8; size];
        rand::rng().fill(&mut bytes[..]);

        // Create a Uint8Array from the bytes
        let uint8_array = JsUint8Array::from_iter(bytes, context)?;

        Ok(uint8_array.into())
    });
    context.global_object().set(
        js_string!("__viper_random_bytes"),
        random_bytes_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // Create the crypto object in JavaScript
    let crypto_code = r#"
        globalThis.crypto = {
            // Generate a random UUID v4
            randomUUID: () => __viper_random_uuid(),

            // Fill typed array with cryptographically random values
            getRandomValues: (array) => __viper_get_random_values(array),

            // Node.js compatible randomBytes
            randomBytes: (size) => __viper_random_bytes(size),

            // Subtle crypto (placeholder - complex to implement fully)
            subtle: {
                digest: async (algorithm, data) => {
                    throw new Error('crypto.subtle.digest() is not yet implemented');
                },
                encrypt: async (algorithm, key, data) => {
                    throw new Error('crypto.subtle.encrypt() is not yet implemented');
                },
                decrypt: async (algorithm, key, data) => {
                    throw new Error('crypto.subtle.decrypt() is not yet implemented');
                },
                sign: async (algorithm, key, data) => {
                    throw new Error('crypto.subtle.sign() is not yet implemented');
                },
                verify: async (algorithm, key, signature, data) => {
                    throw new Error('crypto.subtle.verify() is not yet implemented');
                },
                generateKey: async (algorithm, extractable, keyUsages) => {
                    throw new Error('crypto.subtle.generateKey() is not yet implemented');
                },
                importKey: async (format, keyData, algorithm, extractable, keyUsages) => {
                    throw new Error('crypto.subtle.importKey() is not yet implemented');
                },
                exportKey: async (format, key) => {
                    throw new Error('crypto.subtle.exportKey() is not yet implemented');
                },
            },
        };
    "#;

    let source = Source::from_bytes(crypto_code.as_bytes());
    context.eval(source)?;

    Ok(())
}
