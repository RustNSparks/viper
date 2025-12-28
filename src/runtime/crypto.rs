//! Crypto API - Full Node.js crypto module implementation
//!
//! Provides:
//! - crypto.createHash(algorithm) - Create hash objects
//! - crypto.createHmac(algorithm, key) - Create HMAC objects
//! - crypto.createCipheriv(algorithm, key, iv) - Create cipher objects
//! - crypto.createDecipheriv(algorithm, key, iv) - Create decipher objects
//! - crypto.pbkdf2(password, salt, iterations, keylen, digest, callback)
//! - crypto.pbkdf2Sync(password, salt, iterations, keylen, digest)
//! - crypto.scrypt(password, salt, keylen, options, callback)
//! - crypto.scryptSync(password, salt, keylen, options)
//! - crypto.hkdf(digest, ikm, salt, info, keylen, callback)
//! - crypto.hkdfSync(digest, ikm, salt, info, keylen)
//! - crypto.randomUUID() - Generate a random UUID v4
//! - crypto.randomBytes(size) - Generate random bytes
//! - crypto.randomFillSync(buffer) - Fill buffer with random bytes
//! - crypto.randomFill(buffer, callback) - Async fill buffer with random bytes
//! - crypto.randomInt(min, max) - Generate random integer
//! - crypto.getRandomValues(array) - Fill typed array with random values
//! - crypto.timingSafeEqual(a, b) - Timing-safe comparison
//! - crypto.getCiphers() - List available ciphers
//! - crypto.getHashes() - List available hashes

use base64::{Engine as _, engine::general_purpose};
use boa_engine::{
    Context, JsData, JsNativeError, JsObject, JsResult, JsValue, NativeFunction, Source, js_string,
    object::builtins::JsUint8Array,
};
use boa_gc::{Finalize, Trace};
use hmac::{Hmac, Mac};
use md5::Md5;
use rand::Rng;
use sha1::Sha1;
use sha2::{Digest, Sha224, Sha256, Sha384, Sha512};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// Cipher imports
use aes::Aes128;
use aes::Aes192;
use aes::Aes256;
use cbc::{Decryptor as CbcDecryptor, Encryptor as CbcEncryptor};
use cipher::StreamCipher;
use cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit, block_padding::Pkcs7};
use ctr::Ctr128BE;

// Type aliases for CBC ciphers
type Aes128CbcEnc = CbcEncryptor<Aes128>;
type Aes192CbcEnc = CbcEncryptor<Aes192>;
type Aes256CbcEnc = CbcEncryptor<Aes256>;
type Aes128CbcDec = CbcDecryptor<Aes128>;
type Aes192CbcDec = CbcDecryptor<Aes192>;
type Aes256CbcDec = CbcDecryptor<Aes256>;

// HKDF
use hkdf::Hkdf;

// Scrypt
use scrypt::{Params as ScryptParams, scrypt};

/// Hash object wrapper
#[derive(Clone, Trace, Finalize, JsData)]
struct HashObject {
    #[unsafe_ignore_trace]
    hasher: Arc<Mutex<Box<dyn HashTrait>>>,
}

/// HMAC object wrapper
#[derive(Clone, Trace, Finalize, JsData)]
struct HmacObject {
    #[unsafe_ignore_trace]
    hmac: Arc<Mutex<Box<dyn HmacTrait>>>,
}

/// Cipher object wrapper
#[derive(Clone, Trace, Finalize, JsData)]
struct CipherObject {
    #[unsafe_ignore_trace]
    cipher: Arc<Mutex<CipherState>>,
}

/// Decipher object wrapper
#[derive(Clone, Trace, Finalize, JsData)]
struct DecipherObject {
    #[unsafe_ignore_trace]
    decipher: Arc<Mutex<DecipherState>>,
}

/// Cipher state for streaming encryption
struct CipherState {
    algorithm: String,
    key: Vec<u8>,
    iv: Vec<u8>,
    data: Vec<u8>,
    auto_padding: bool,
}

/// Decipher state for streaming decryption
struct DecipherState {
    algorithm: String,
    key: Vec<u8>,
    iv: Vec<u8>,
    data: Vec<u8>,
    auto_padding: bool,
}

/// Trait for unified hash interface
trait HashTrait: Send {
    fn update_hash(&mut self, data: &[u8]);
    fn finalize_hash(&mut self) -> Vec<u8>;
    fn box_clone(&self) -> Box<dyn HashTrait>;
}

impl Clone for Box<dyn HashTrait> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

/// Trait for unified HMAC interface
trait HmacTrait: Send {
    fn update_hmac(&mut self, data: &[u8]);
    fn finalize_hmac(&mut self) -> Vec<u8>;
    fn box_clone(&self) -> Box<dyn HmacTrait>;
}

impl Clone for Box<dyn HmacTrait> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

// Implement HashTrait for each digest algorithm
macro_rules! impl_hash_trait {
    ($name:ty) => {
        impl HashTrait for $name {
            fn update_hash(&mut self, data: &[u8]) {
                Digest::update(self, data);
            }

            fn finalize_hash(&mut self) -> Vec<u8> {
                let result = Digest::finalize_reset(self);
                result.to_vec()
            }

            fn box_clone(&self) -> Box<dyn HashTrait> {
                Box::new(self.clone())
            }
        }
    };
}

impl_hash_trait!(Sha256);
impl_hash_trait!(Sha512);
impl_hash_trait!(Sha384);
impl_hash_trait!(Sha224);
impl_hash_trait!(Sha1);
impl_hash_trait!(Md5);

// Implement HmacTrait for each HMAC algorithm
macro_rules! impl_hmac_trait {
    ($name:ty) => {
        impl HmacTrait for $name {
            fn update_hmac(&mut self, data: &[u8]) {
                Mac::update(self, data);
            }

            fn finalize_hmac(&mut self) -> Vec<u8> {
                let cloned = self.clone();
                let result = Mac::finalize(cloned);
                result.into_bytes().to_vec()
            }

            fn box_clone(&self) -> Box<dyn HmacTrait> {
                Box::new(self.clone())
            }
        }
    };
}

impl_hmac_trait!(Hmac<Sha256>);
impl_hmac_trait!(Hmac<Sha512>);
impl_hmac_trait!(Hmac<Sha384>);
impl_hmac_trait!(Hmac<Sha224>);
impl_hmac_trait!(Hmac<Sha1>);
impl_hmac_trait!(Hmac<Md5>);

/// Create a hash object based on algorithm name
fn create_hasher(algorithm: &str) -> Result<Box<dyn HashTrait>, String> {
    match algorithm.to_lowercase().as_str() {
        "sha256" | "sha-256" => Ok(Box::new(Sha256::new())),
        "sha512" | "sha-512" => Ok(Box::new(Sha512::new())),
        "sha384" | "sha-384" => Ok(Box::new(Sha384::new())),
        "sha224" | "sha-224" => Ok(Box::new(Sha224::new())),
        "sha1" | "sha-1" => Ok(Box::new(Sha1::new())),
        "md5" => Ok(Box::new(Md5::new())),
        _ => Err(format!("Unsupported hash algorithm: {}", algorithm)),
    }
}

/// Create an HMAC object based on algorithm name
fn create_hmac_obj(algorithm: &str, key: &[u8]) -> Result<Box<dyn HmacTrait>, String> {
    match algorithm.to_lowercase().as_str() {
        "sha256" | "sha-256" => {
            let hmac = Hmac::<Sha256>::new_from_slice(key)
                .map_err(|e| format!("Invalid key length: {}", e))?;
            Ok(Box::new(hmac))
        }
        "sha512" | "sha-512" => {
            let hmac = Hmac::<Sha512>::new_from_slice(key)
                .map_err(|e| format!("Invalid key length: {}", e))?;
            Ok(Box::new(hmac))
        }
        "sha384" | "sha-384" => {
            let hmac = Hmac::<Sha384>::new_from_slice(key)
                .map_err(|e| format!("Invalid key length: {}", e))?;
            Ok(Box::new(hmac))
        }
        "sha224" | "sha-224" => {
            let hmac = Hmac::<Sha224>::new_from_slice(key)
                .map_err(|e| format!("Invalid key length: {}", e))?;
            Ok(Box::new(hmac))
        }
        "sha1" | "sha-1" => {
            let hmac = Hmac::<Sha1>::new_from_slice(key)
                .map_err(|e| format!("Invalid key length: {}", e))?;
            Ok(Box::new(hmac))
        }
        "md5" => {
            let hmac = Hmac::<Md5>::new_from_slice(key)
                .map_err(|e| format!("Invalid key length: {}", e))?;
            Ok(Box::new(hmac))
        }
        _ => Err(format!("Unsupported HMAC algorithm: {}", algorithm)),
    }
}

/// Encrypt data using AES-CBC with PKCS7 padding
fn encrypt_aes_cbc(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
    use cbc::cipher::BlockEncryptMut as _;

    // AES block size is always 16
    let block_size = 16usize;
    let padding_len = block_size - (data.len() % block_size);
    let padded_len = data.len() + padding_len;

    // Create buffer with data and PKCS7 padding
    let mut buffer = vec![0u8; padded_len];
    buffer[..data.len()].copy_from_slice(data);

    // Add PKCS7 padding
    for byte in buffer.iter_mut().skip(data.len()) {
        *byte = padding_len as u8;
    }

    // Encrypt based on key length
    match key.len() {
        16 => {
            let cipher = Aes128CbcEnc::new_from_slices(key, iv)
                .map_err(|e| format!("Failed to create cipher: {}", e))?;
            cipher
                .encrypt_padded_mut::<Pkcs7>(&mut buffer, data.len())
                .map_err(|_| "Encryption failed".to_string())?;
        }
        24 => {
            let cipher = Aes192CbcEnc::new_from_slices(key, iv)
                .map_err(|e| format!("Failed to create cipher: {}", e))?;
            cipher
                .encrypt_padded_mut::<Pkcs7>(&mut buffer, data.len())
                .map_err(|_| "Encryption failed".to_string())?;
        }
        32 => {
            let cipher = Aes256CbcEnc::new_from_slices(key, iv)
                .map_err(|e| format!("Failed to create cipher: {}", e))?;
            cipher
                .encrypt_padded_mut::<Pkcs7>(&mut buffer, data.len())
                .map_err(|_| "Encryption failed".to_string())?;
        }
        _ => return Err(format!("Invalid key length: {}", key.len())),
    }

    Ok(buffer)
}

/// Decrypt data using AES-CBC with PKCS7 padding
fn decrypt_aes_cbc(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
    use cbc::cipher::BlockDecryptMut as _;

    let mut buffer = data.to_vec();

    // Decrypt based on key length
    let decrypted = match key.len() {
        16 => {
            let cipher = Aes128CbcDec::new_from_slices(key, iv)
                .map_err(|e| format!("Failed to create cipher: {}", e))?;
            cipher
                .decrypt_padded_mut::<Pkcs7>(&mut buffer)
                .map_err(|_| "Decryption failed: invalid padding".to_string())?
        }
        24 => {
            let cipher = Aes192CbcDec::new_from_slices(key, iv)
                .map_err(|e| format!("Failed to create cipher: {}", e))?;
            cipher
                .decrypt_padded_mut::<Pkcs7>(&mut buffer)
                .map_err(|_| "Decryption failed: invalid padding".to_string())?
        }
        32 => {
            let cipher = Aes256CbcDec::new_from_slices(key, iv)
                .map_err(|e| format!("Failed to create cipher: {}", e))?;
            cipher
                .decrypt_padded_mut::<Pkcs7>(&mut buffer)
                .map_err(|_| "Decryption failed: invalid padding".to_string())?
        }
        _ => return Err(format!("Invalid key length: {}", key.len())),
    };

    Ok(decrypted.to_vec())
}

/// Encrypt data using specified algorithm
fn encrypt_data(
    algorithm: &str,
    key: &[u8],
    iv: &[u8],
    data: &[u8],
    _auto_padding: bool,
) -> Result<Vec<u8>, String> {
    let algo = algorithm.to_lowercase();

    match algo.as_str() {
        "aes-128-cbc" => {
            if key.len() != 16 {
                return Err(format!(
                    "Invalid key length {} for aes-128-cbc, expected 16",
                    key.len()
                ));
            }
            if iv.len() != 16 {
                return Err(format!(
                    "Invalid IV length {} for aes-128-cbc, expected 16",
                    iv.len()
                ));
            }
            encrypt_aes_cbc(key, iv, data)
        }
        "aes-192-cbc" => {
            if key.len() != 24 {
                return Err(format!(
                    "Invalid key length {} for aes-192-cbc, expected 24",
                    key.len()
                ));
            }
            if iv.len() != 16 {
                return Err(format!(
                    "Invalid IV length {} for aes-192-cbc, expected 16",
                    iv.len()
                ));
            }
            encrypt_aes_cbc(key, iv, data)
        }
        "aes-256-cbc" => {
            if key.len() != 32 {
                return Err(format!(
                    "Invalid key length {} for aes-256-cbc, expected 32",
                    key.len()
                ));
            }
            if iv.len() != 16 {
                return Err(format!(
                    "Invalid IV length {} for aes-256-cbc, expected 16",
                    iv.len()
                ));
            }
            encrypt_aes_cbc(key, iv, data)
        }
        "aes-128-ctr" => {
            if key.len() != 16 {
                return Err(format!(
                    "Invalid key length {} for aes-128-ctr, expected 16",
                    key.len()
                ));
            }
            if iv.len() != 16 {
                return Err(format!(
                    "Invalid IV length {} for aes-128-ctr, expected 16",
                    iv.len()
                ));
            }
            let mut cipher = Ctr128BE::<Aes128>::new_from_slices(key, iv)
                .map_err(|e| format!("Failed to create cipher: {}", e))?;
            let mut buffer = data.to_vec();
            cipher.apply_keystream(&mut buffer);
            Ok(buffer)
        }
        "aes-192-ctr" => {
            if key.len() != 24 {
                return Err(format!(
                    "Invalid key length {} for aes-192-ctr, expected 24",
                    key.len()
                ));
            }
            if iv.len() != 16 {
                return Err(format!(
                    "Invalid IV length {} for aes-192-ctr, expected 16",
                    iv.len()
                ));
            }
            let mut cipher = Ctr128BE::<Aes192>::new_from_slices(key, iv)
                .map_err(|e| format!("Failed to create cipher: {}", e))?;
            let mut buffer = data.to_vec();
            cipher.apply_keystream(&mut buffer);
            Ok(buffer)
        }
        "aes-256-ctr" => {
            if key.len() != 32 {
                return Err(format!(
                    "Invalid key length {} for aes-256-ctr, expected 32",
                    key.len()
                ));
            }
            if iv.len() != 16 {
                return Err(format!(
                    "Invalid IV length {} for aes-256-ctr, expected 16",
                    iv.len()
                ));
            }
            let mut cipher = Ctr128BE::<Aes256>::new_from_slices(key, iv)
                .map_err(|e| format!("Failed to create cipher: {}", e))?;
            let mut buffer = data.to_vec();
            cipher.apply_keystream(&mut buffer);
            Ok(buffer)
        }
        _ => Err(format!("Unsupported cipher algorithm: {}", algorithm)),
    }
}

/// Decrypt data using specified algorithm
fn decrypt_data(
    algorithm: &str,
    key: &[u8],
    iv: &[u8],
    data: &[u8],
    _auto_padding: bool,
) -> Result<Vec<u8>, String> {
    let algo = algorithm.to_lowercase();

    match algo.as_str() {
        "aes-128-cbc" => {
            if key.len() != 16 {
                return Err(format!(
                    "Invalid key length {} for aes-128-cbc, expected 16",
                    key.len()
                ));
            }
            if iv.len() != 16 {
                return Err(format!(
                    "Invalid IV length {} for aes-128-cbc, expected 16",
                    iv.len()
                ));
            }
            decrypt_aes_cbc(key, iv, data)
        }
        "aes-192-cbc" => {
            if key.len() != 24 {
                return Err(format!(
                    "Invalid key length {} for aes-192-cbc, expected 24",
                    key.len()
                ));
            }
            if iv.len() != 16 {
                return Err(format!(
                    "Invalid IV length {} for aes-192-cbc, expected 16",
                    iv.len()
                ));
            }
            decrypt_aes_cbc(key, iv, data)
        }
        "aes-256-cbc" => {
            if key.len() != 32 {
                return Err(format!(
                    "Invalid key length {} for aes-256-cbc, expected 32",
                    key.len()
                ));
            }
            if iv.len() != 16 {
                return Err(format!(
                    "Invalid IV length {} for aes-256-cbc, expected 16",
                    iv.len()
                ));
            }
            decrypt_aes_cbc(key, iv, data)
        }
        "aes-128-ctr" | "aes-192-ctr" | "aes-256-ctr" => {
            // CTR mode decryption is the same as encryption
            encrypt_data(algorithm, key, iv, data, true)
        }
        _ => Err(format!("Unsupported cipher algorithm: {}", algorithm)),
    }
}

/// HKDF key derivation
fn hkdf_derive(
    digest: &str,
    ikm: &[u8],
    salt: &[u8],
    info: &[u8],
    keylen: usize,
) -> Result<Vec<u8>, String> {
    let mut okm = vec![0u8; keylen];

    match digest.to_lowercase().as_str() {
        "sha256" | "sha-256" => {
            let hk = Hkdf::<Sha256>::new(Some(salt), ikm);
            hk.expand(info, &mut okm)
                .map_err(|e| format!("HKDF expand failed: {}", e))?;
        }
        "sha512" | "sha-512" => {
            let hk = Hkdf::<Sha512>::new(Some(salt), ikm);
            hk.expand(info, &mut okm)
                .map_err(|e| format!("HKDF expand failed: {}", e))?;
        }
        "sha384" | "sha-384" => {
            let hk = Hkdf::<Sha384>::new(Some(salt), ikm);
            hk.expand(info, &mut okm)
                .map_err(|e| format!("HKDF expand failed: {}", e))?;
        }
        "sha1" | "sha-1" => {
            let hk = Hkdf::<Sha1>::new(Some(salt), ikm);
            hk.expand(info, &mut okm)
                .map_err(|e| format!("HKDF expand failed: {}", e))?;
        }
        _ => return Err(format!("Unsupported digest for HKDF: {}", digest)),
    }

    Ok(okm)
}

/// Scrypt key derivation
fn scrypt_derive(
    password: &[u8],
    salt: &[u8],
    keylen: usize,
    n: u32,
    r: u32,
    p: u32,
) -> Result<Vec<u8>, String> {
    let log_n = (n as f64).log2() as u8;
    let params = ScryptParams::new(log_n, r, p, keylen)
        .map_err(|e| format!("Invalid scrypt parameters: {}", e))?;

    let mut output = vec![0u8; keylen];
    scrypt(password, salt, &params, &mut output).map_err(|e| format!("Scrypt failed: {}", e))?;

    Ok(output)
}

/// Helper to extract bytes from JsValue
fn js_value_to_bytes(value: &JsValue, context: &mut Context) -> JsResult<Vec<u8>> {
    if let Some(s) = value.as_string() {
        Ok(s.to_std_string_escaped().into_bytes())
    } else if let Some(obj) = value.as_object() {
        if let Ok(uint8) = JsUint8Array::from_object(obj.clone()) {
            let len = uint8.length(context)?;
            let mut bytes = Vec::with_capacity(len);
            for i in 0..len {
                let val = uint8.get(i, context)?;
                if let Some(num) = val.as_number() {
                    bytes.push(num as u8);
                }
            }
            Ok(bytes)
        } else {
            Err(JsNativeError::typ()
                .with_message("Value must be a string or Uint8Array")
                .into())
        }
    } else {
        Err(JsNativeError::typ()
            .with_message("Value must be a string or Uint8Array")
            .into())
    }
}

/// Register the crypto module
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

        let obj = array
            .as_object()
            .ok_or_else(|| JsNativeError::typ().with_message("Argument must be a typed array"))?;

        if let Ok(uint8) = JsUint8Array::from_object(obj.clone()) {
            let len = uint8.length(context)?;
            let mut rng = rand::rng();

            for i in 0..len {
                let random_byte: u8 = rng.random();
                uint8.set(i, JsValue::from(random_byte as i32), true, context)?;
            }

            return Ok(array.clone());
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

    // crypto.randomBytes(size)
    let random_bytes_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let size = args
            .get(0)
            .and_then(|v| v.as_number())
            .ok_or_else(|| JsNativeError::typ().with_message("size must be a number"))?
            as usize;

        if size > 2147483647 {
            return Err(JsNativeError::range()
                .with_message("Size must be less than 2147483647")
                .into());
        }

        let mut bytes = vec![0u8; size];
        rand::rng().fill(&mut bytes[..]);

        let uint8_array = JsUint8Array::from_iter(bytes, context)?;
        Ok(uint8_array.into())
    });
    context.global_object().set(
        js_string!("__viper_random_bytes"),
        random_bytes_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // crypto.randomInt(min, max)
    let random_int_fn =
        NativeFunction::from_fn_ptr(|_this, args, _context| {
            let (min, max) =
                if args.len() == 1 {
                    let max =
                        args.get(0).and_then(|v| v.as_number()).ok_or_else(|| {
                            JsNativeError::typ().with_message("max must be a number")
                        })? as i64;
                    (0i64, max)
                } else {
                    let min =
                        args.get(0).and_then(|v| v.as_number()).ok_or_else(|| {
                            JsNativeError::typ().with_message("min must be a number")
                        })? as i64;
                    let max =
                        args.get(1).and_then(|v| v.as_number()).ok_or_else(|| {
                            JsNativeError::typ().with_message("max must be a number")
                        })? as i64;
                    (min, max)
                };

            if min >= max {
                return Err(JsNativeError::range()
                    .with_message("min must be less than max")
                    .into());
            }

            let mut rng = rand::rng();
            let value = rng.random_range(min..max);
            Ok(JsValue::from(value as f64))
        });
    context.global_object().set(
        js_string!("__viper_random_int"),
        random_int_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // crypto.timingSafeEqual(a, b)
    let timing_safe_equal_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let a = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("First argument is required"))?;
        let b = args
            .get(1)
            .ok_or_else(|| JsNativeError::typ().with_message("Second argument is required"))?;

        let bytes_a = js_value_to_bytes(a, context)?;
        let bytes_b = js_value_to_bytes(b, context)?;

        if bytes_a.len() != bytes_b.len() {
            return Err(JsNativeError::range()
                .with_message("Input buffers must have the same byte length")
                .into());
        }

        // Timing-safe comparison
        let result = subtle::ConstantTimeEq::ct_eq(&bytes_a[..], &bytes_b[..]);
        Ok(JsValue::from(bool::from(result)))
    });
    context.global_object().set(
        js_string!("__viper_timing_safe_equal"),
        timing_safe_equal_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // crypto.createHash(algorithm)
    let create_hash_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let algorithm = args
            .get(0)
            .and_then(|v| v.as_string())
            .ok_or_else(|| JsNativeError::typ().with_message("algorithm must be a string"))?;

        let algorithm_str = algorithm.to_std_string_escaped();

        let hasher =
            create_hasher(&algorithm_str).map_err(|e| JsNativeError::error().with_message(e))?;

        let hash_obj = HashObject {
            hasher: Arc::new(Mutex::new(hasher)),
        };

        let js_obj = JsObject::from_proto_and_data(None, hash_obj);

        // hash.update(data, encoding)
        let update_fn = NativeFunction::from_fn_ptr(|this, args, context| {
            let obj = this
                .as_object()
                .ok_or_else(|| JsNativeError::typ().with_message("this is not a Hash object"))?;
            let hash_obj = obj
                .downcast_ref::<HashObject>()
                .ok_or_else(|| JsNativeError::typ().with_message("this is not a Hash object"))?;

            let data = args
                .get(0)
                .ok_or_else(|| JsNativeError::typ().with_message("data argument is required"))?;

            let bytes = js_value_to_bytes(data, context)?;

            let mut hasher = hash_obj.hasher.lock().unwrap();
            hasher.update_hash(&bytes);

            Ok(this.clone())
        });

        js_obj.set(
            js_string!("update"),
            update_fn.to_js_function(context.realm()),
            false,
            context,
        )?;

        // hash.digest(encoding)
        let digest_fn = NativeFunction::from_fn_ptr(|this, args, context| {
            let obj = this
                .as_object()
                .ok_or_else(|| JsNativeError::typ().with_message("this is not a Hash object"))?;
            let hash_obj = obj
                .downcast_ref::<HashObject>()
                .ok_or_else(|| JsNativeError::typ().with_message("this is not a Hash object"))?;

            let encoding = args
                .get(0)
                .and_then(|v| v.as_string())
                .map(|s| s.to_std_string_escaped());

            let mut hasher = hash_obj.hasher.lock().unwrap();
            let result = hasher.finalize_hash();

            match encoding.as_deref() {
                Some("hex") => Ok(JsValue::from(js_string!(hex::encode(&result)))),
                Some("base64") => Ok(JsValue::from(js_string!(
                    general_purpose::STANDARD.encode(&result)
                ))),
                Some("base64url") => Ok(JsValue::from(js_string!(
                    general_purpose::URL_SAFE_NO_PAD.encode(&result)
                ))),
                Some("buffer") | None => {
                    let uint8_array = JsUint8Array::from_iter(result, context)?;
                    Ok(uint8_array.into())
                }
                Some(enc) => Err(JsNativeError::typ()
                    .with_message(format!("Unsupported encoding: {}", enc))
                    .into()),
            }
        });

        js_obj.set(
            js_string!("digest"),
            digest_fn.to_js_function(context.realm()),
            false,
            context,
        )?;

        // hash.copy()
        let copy_fn = NativeFunction::from_fn_ptr(|this, _args, context| {
            let obj = this
                .as_object()
                .ok_or_else(|| JsNativeError::typ().with_message("this is not a Hash object"))?;
            let hash_obj = obj
                .downcast_ref::<HashObject>()
                .ok_or_else(|| JsNativeError::typ().with_message("this is not a Hash object"))?;

            let hasher = hash_obj.hasher.lock().unwrap();
            let new_hash_obj = HashObject {
                hasher: Arc::new(Mutex::new(hasher.box_clone())),
            };

            let new_js_obj = JsObject::from_proto_and_data(None, new_hash_obj);
            Ok(new_js_obj.into())
        });

        js_obj.set(
            js_string!("copy"),
            copy_fn.to_js_function(context.realm()),
            false,
            context,
        )?;

        Ok(js_obj.into())
    });
    context.global_object().set(
        js_string!("__viper_create_hash"),
        create_hash_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // crypto.createHmac(algorithm, key)
    let create_hmac_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let algorithm = args
            .get(0)
            .and_then(|v| v.as_string())
            .ok_or_else(|| JsNativeError::typ().with_message("algorithm must be a string"))?;

        let key_arg = args
            .get(1)
            .ok_or_else(|| JsNativeError::typ().with_message("key argument is required"))?;

        let key_bytes = js_value_to_bytes(key_arg, context)?;
        let algorithm_str = algorithm.to_std_string_escaped();

        let hmac = create_hmac_obj(&algorithm_str, &key_bytes)
            .map_err(|e| JsNativeError::error().with_message(e))?;

        let hmac_obj = HmacObject {
            hmac: Arc::new(Mutex::new(hmac)),
        };

        let js_obj = JsObject::from_proto_and_data(None, hmac_obj);

        // hmac.update(data, encoding)
        let update_fn = NativeFunction::from_fn_ptr(|this, args, context| {
            let obj = this
                .as_object()
                .ok_or_else(|| JsNativeError::typ().with_message("this is not an Hmac object"))?;
            let hmac_obj = obj
                .downcast_ref::<HmacObject>()
                .ok_or_else(|| JsNativeError::typ().with_message("this is not an Hmac object"))?;

            let data = args
                .get(0)
                .ok_or_else(|| JsNativeError::typ().with_message("data argument is required"))?;

            let bytes = js_value_to_bytes(data, context)?;

            let mut hmac = hmac_obj.hmac.lock().unwrap();
            hmac.update_hmac(&bytes);

            Ok(this.clone())
        });

        js_obj.set(
            js_string!("update"),
            update_fn.to_js_function(context.realm()),
            false,
            context,
        )?;

        // hmac.digest(encoding)
        let digest_fn = NativeFunction::from_fn_ptr(|this, args, context| {
            let obj = this
                .as_object()
                .ok_or_else(|| JsNativeError::typ().with_message("this is not an Hmac object"))?;
            let hmac_obj = obj
                .downcast_ref::<HmacObject>()
                .ok_or_else(|| JsNativeError::typ().with_message("this is not an Hmac object"))?;

            let encoding = args
                .get(0)
                .and_then(|v| v.as_string())
                .map(|s| s.to_std_string_escaped());

            let mut hmac = hmac_obj.hmac.lock().unwrap();
            let result = hmac.finalize_hmac();

            match encoding.as_deref() {
                Some("hex") => Ok(JsValue::from(js_string!(hex::encode(&result)))),
                Some("base64") => Ok(JsValue::from(js_string!(
                    general_purpose::STANDARD.encode(&result)
                ))),
                Some("base64url") => Ok(JsValue::from(js_string!(
                    general_purpose::URL_SAFE_NO_PAD.encode(&result)
                ))),
                Some("buffer") | None => {
                    let uint8_array = JsUint8Array::from_iter(result, context)?;
                    Ok(uint8_array.into())
                }
                Some(enc) => Err(JsNativeError::typ()
                    .with_message(format!("Unsupported encoding: {}", enc))
                    .into()),
            }
        });

        js_obj.set(
            js_string!("digest"),
            digest_fn.to_js_function(context.realm()),
            false,
            context,
        )?;

        Ok(js_obj.into())
    });
    context.global_object().set(
        js_string!("__viper_create_hmac"),
        create_hmac_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // crypto.createCipheriv(algorithm, key, iv)
    let create_cipheriv_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let algorithm = args
            .get(0)
            .and_then(|v| v.as_string())
            .ok_or_else(|| JsNativeError::typ().with_message("algorithm must be a string"))?
            .to_std_string_escaped();

        let key_arg = args
            .get(1)
            .ok_or_else(|| JsNativeError::typ().with_message("key argument is required"))?;
        let iv_arg = args
            .get(2)
            .ok_or_else(|| JsNativeError::typ().with_message("iv argument is required"))?;

        let key_bytes = js_value_to_bytes(key_arg, context)?;
        let iv_bytes = js_value_to_bytes(iv_arg, context)?;

        let cipher_state = CipherState {
            algorithm,
            key: key_bytes,
            iv: iv_bytes,
            data: Vec::new(),
            auto_padding: true,
        };

        let cipher_obj = CipherObject {
            cipher: Arc::new(Mutex::new(cipher_state)),
        };

        let js_obj = JsObject::from_proto_and_data(None, cipher_obj);

        // cipher.update(data, inputEncoding, outputEncoding)
        let update_fn = NativeFunction::from_fn_ptr(|this, args, context| {
            let obj = this
                .as_object()
                .ok_or_else(|| JsNativeError::typ().with_message("this is not a Cipher object"))?;
            let cipher_obj = obj
                .downcast_ref::<CipherObject>()
                .ok_or_else(|| JsNativeError::typ().with_message("this is not a Cipher object"))?;

            let data = args
                .get(0)
                .ok_or_else(|| JsNativeError::typ().with_message("data argument is required"))?;

            let bytes = js_value_to_bytes(data, context)?;

            let mut cipher = cipher_obj.cipher.lock().unwrap();
            cipher.data.extend_from_slice(&bytes);

            // Return empty buffer for streaming compatibility
            let empty = JsUint8Array::from_iter(Vec::<u8>::new(), context)?;
            Ok(empty.into())
        });

        js_obj.set(
            js_string!("update"),
            update_fn.to_js_function(context.realm()),
            false,
            context,
        )?;

        // cipher.final(outputEncoding)
        let final_fn = NativeFunction::from_fn_ptr(|this, args, context| {
            let obj = this
                .as_object()
                .ok_or_else(|| JsNativeError::typ().with_message("this is not a Cipher object"))?;
            let cipher_obj = obj
                .downcast_ref::<CipherObject>()
                .ok_or_else(|| JsNativeError::typ().with_message("this is not a Cipher object"))?;

            let cipher = cipher_obj.cipher.lock().unwrap();
            let encrypted = encrypt_data(
                &cipher.algorithm,
                &cipher.key,
                &cipher.iv,
                &cipher.data,
                cipher.auto_padding,
            )
            .map_err(|e| JsNativeError::error().with_message(e))?;

            let encoding = args
                .get(0)
                .and_then(|v| v.as_string())
                .map(|s| s.to_std_string_escaped());

            match encoding.as_deref() {
                Some("hex") => Ok(JsValue::from(js_string!(hex::encode(&encrypted)))),
                Some("base64") => Ok(JsValue::from(js_string!(
                    general_purpose::STANDARD.encode(&encrypted)
                ))),
                Some("buffer") | None => {
                    let uint8_array = JsUint8Array::from_iter(encrypted, context)?;
                    Ok(uint8_array.into())
                }
                Some(enc) => Err(JsNativeError::typ()
                    .with_message(format!("Unsupported encoding: {}", enc))
                    .into()),
            }
        });

        js_obj.set(
            js_string!("final"),
            final_fn.to_js_function(context.realm()),
            false,
            context,
        )?;

        // cipher.setAutoPadding(autoPadding)
        let set_auto_padding_fn = NativeFunction::from_fn_ptr(|this, args, _context| {
            let obj = this
                .as_object()
                .ok_or_else(|| JsNativeError::typ().with_message("this is not a Cipher object"))?;
            let cipher_obj = obj
                .downcast_ref::<CipherObject>()
                .ok_or_else(|| JsNativeError::typ().with_message("this is not a Cipher object"))?;

            let auto_padding = args.get(0).map(|v| v.to_boolean()).unwrap_or(true);

            let mut cipher = cipher_obj.cipher.lock().unwrap();
            cipher.auto_padding = auto_padding;

            Ok(this.clone())
        });

        js_obj.set(
            js_string!("setAutoPadding"),
            set_auto_padding_fn.to_js_function(context.realm()),
            false,
            context,
        )?;

        Ok(js_obj.into())
    });
    context.global_object().set(
        js_string!("__viper_create_cipheriv"),
        create_cipheriv_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // crypto.createDecipheriv(algorithm, key, iv)
    let create_decipheriv_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let algorithm = args
            .get(0)
            .and_then(|v| v.as_string())
            .ok_or_else(|| JsNativeError::typ().with_message("algorithm must be a string"))?
            .to_std_string_escaped();

        let key_arg = args
            .get(1)
            .ok_or_else(|| JsNativeError::typ().with_message("key argument is required"))?;
        let iv_arg = args
            .get(2)
            .ok_or_else(|| JsNativeError::typ().with_message("iv argument is required"))?;

        let key_bytes = js_value_to_bytes(key_arg, context)?;
        let iv_bytes = js_value_to_bytes(iv_arg, context)?;

        let decipher_state = DecipherState {
            algorithm,
            key: key_bytes,
            iv: iv_bytes,
            data: Vec::new(),
            auto_padding: true,
        };

        let decipher_obj = DecipherObject {
            decipher: Arc::new(Mutex::new(decipher_state)),
        };

        let js_obj = JsObject::from_proto_and_data(None, decipher_obj);

        // decipher.update(data, inputEncoding, outputEncoding)
        let update_fn = NativeFunction::from_fn_ptr(|this, args, context| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("this is not a Decipher object")
            })?;
            let decipher_obj = obj.downcast_ref::<DecipherObject>().ok_or_else(|| {
                JsNativeError::typ().with_message("this is not a Decipher object")
            })?;

            let data = args
                .get(0)
                .ok_or_else(|| JsNativeError::typ().with_message("data argument is required"))?;

            let bytes = js_value_to_bytes(data, context)?;

            let mut decipher = decipher_obj.decipher.lock().unwrap();
            decipher.data.extend_from_slice(&bytes);

            let empty = JsUint8Array::from_iter(Vec::<u8>::new(), context)?;
            Ok(empty.into())
        });

        js_obj.set(
            js_string!("update"),
            update_fn.to_js_function(context.realm()),
            false,
            context,
        )?;

        // decipher.final(outputEncoding)
        let final_fn = NativeFunction::from_fn_ptr(|this, args, context| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("this is not a Decipher object")
            })?;
            let decipher_obj = obj.downcast_ref::<DecipherObject>().ok_or_else(|| {
                JsNativeError::typ().with_message("this is not a Decipher object")
            })?;

            let decipher = decipher_obj.decipher.lock().unwrap();
            let decrypted = decrypt_data(
                &decipher.algorithm,
                &decipher.key,
                &decipher.iv,
                &decipher.data,
                decipher.auto_padding,
            )
            .map_err(|e| JsNativeError::error().with_message(e))?;

            let encoding = args
                .get(0)
                .and_then(|v| v.as_string())
                .map(|s| s.to_std_string_escaped());

            match encoding.as_deref() {
                Some("utf8") | Some("utf-8") => {
                    let s = String::from_utf8(decrypted).map_err(|e| {
                        JsNativeError::error().with_message(format!("Invalid UTF-8: {}", e))
                    })?;
                    Ok(JsValue::from(js_string!(s)))
                }
                Some("hex") => Ok(JsValue::from(js_string!(hex::encode(&decrypted)))),
                Some("base64") => Ok(JsValue::from(js_string!(
                    general_purpose::STANDARD.encode(&decrypted)
                ))),
                Some("buffer") | None => {
                    let uint8_array = JsUint8Array::from_iter(decrypted, context)?;
                    Ok(uint8_array.into())
                }
                Some(enc) => Err(JsNativeError::typ()
                    .with_message(format!("Unsupported encoding: {}", enc))
                    .into()),
            }
        });

        js_obj.set(
            js_string!("final"),
            final_fn.to_js_function(context.realm()),
            false,
            context,
        )?;

        // decipher.setAutoPadding(autoPadding)
        let set_auto_padding_fn = NativeFunction::from_fn_ptr(|this, args, _context| {
            let obj = this.as_object().ok_or_else(|| {
                JsNativeError::typ().with_message("this is not a Decipher object")
            })?;
            let decipher_obj = obj.downcast_ref::<DecipherObject>().ok_or_else(|| {
                JsNativeError::typ().with_message("this is not a Decipher object")
            })?;

            let auto_padding = args.get(0).map(|v| v.to_boolean()).unwrap_or(true);

            let mut decipher = decipher_obj.decipher.lock().unwrap();
            decipher.auto_padding = auto_padding;

            Ok(this.clone())
        });

        js_obj.set(
            js_string!("setAutoPadding"),
            set_auto_padding_fn.to_js_function(context.realm()),
            false,
            context,
        )?;

        Ok(js_obj.into())
    });
    context.global_object().set(
        js_string!("__viper_create_decipheriv"),
        create_decipheriv_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // crypto.pbkdf2Sync(password, salt, iterations, keylen, digest)
    let pbkdf2_sync_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let password_arg = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("password is required"))?;
        let salt_arg = args
            .get(1)
            .ok_or_else(|| JsNativeError::typ().with_message("salt is required"))?;
        let iterations = args
            .get(2)
            .and_then(|v| v.as_number())
            .ok_or_else(|| JsNativeError::typ().with_message("iterations must be a number"))?
            as u32;
        let keylen = args
            .get(3)
            .and_then(|v| v.as_number())
            .ok_or_else(|| JsNativeError::typ().with_message("keylen must be a number"))?
            as usize;
        let digest = args
            .get(4)
            .and_then(|v| v.as_string())
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_else(|| "sha256".to_string());

        let password_bytes = js_value_to_bytes(password_arg, context)?;
        let salt_bytes = js_value_to_bytes(salt_arg, context)?;

        let mut output = vec![0u8; keylen];

        match digest.to_lowercase().as_str() {
            "sha256" | "sha-256" => {
                pbkdf2::pbkdf2_hmac::<Sha256>(
                    &password_bytes,
                    &salt_bytes,
                    iterations,
                    &mut output,
                );
            }
            "sha512" | "sha-512" => {
                pbkdf2::pbkdf2_hmac::<Sha512>(
                    &password_bytes,
                    &salt_bytes,
                    iterations,
                    &mut output,
                );
            }
            "sha384" | "sha-384" => {
                pbkdf2::pbkdf2_hmac::<Sha384>(
                    &password_bytes,
                    &salt_bytes,
                    iterations,
                    &mut output,
                );
            }
            "sha1" | "sha-1" => {
                pbkdf2::pbkdf2_hmac::<Sha1>(&password_bytes, &salt_bytes, iterations, &mut output);
            }
            "md5" => {
                pbkdf2::pbkdf2_hmac::<Md5>(&password_bytes, &salt_bytes, iterations, &mut output);
            }
            _ => {
                return Err(JsNativeError::typ()
                    .with_message(format!("Unsupported digest: {}", digest))
                    .into());
            }
        }

        let uint8_array = JsUint8Array::from_iter(output, context)?;
        Ok(uint8_array.into())
    });
    context.global_object().set(
        js_string!("__viper_pbkdf2_sync"),
        pbkdf2_sync_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // crypto.scryptSync(password, salt, keylen, options)
    let scrypt_sync_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let password_arg = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("password is required"))?;
        let salt_arg = args
            .get(1)
            .ok_or_else(|| JsNativeError::typ().with_message("salt is required"))?;
        let keylen = args
            .get(2)
            .and_then(|v| v.as_number())
            .ok_or_else(|| JsNativeError::typ().with_message("keylen must be a number"))?
            as usize;

        // Default scrypt parameters
        let mut n: u32 = 16384;
        let mut r: u32 = 8;
        let mut p: u32 = 1;

        // Parse options if provided
        if let Some(opts) = args.get(3).and_then(|v| v.as_object()) {
            if let Ok(n_val) = opts.get(js_string!("N"), context) {
                if let Some(num) = n_val.as_number() {
                    n = num as u32;
                }
            }
            if let Ok(r_val) = opts.get(js_string!("r"), context) {
                if let Some(num) = r_val.as_number() {
                    r = num as u32;
                }
            }
            if let Ok(p_val) = opts.get(js_string!("p"), context) {
                if let Some(num) = p_val.as_number() {
                    p = num as u32;
                }
            }
        }

        let password_bytes = js_value_to_bytes(password_arg, context)?;
        let salt_bytes = js_value_to_bytes(salt_arg, context)?;

        let result = scrypt_derive(&password_bytes, &salt_bytes, keylen, n, r, p)
            .map_err(|e| JsNativeError::error().with_message(e))?;

        let uint8_array = JsUint8Array::from_iter(result, context)?;
        Ok(uint8_array.into())
    });
    context.global_object().set(
        js_string!("__viper_scrypt_sync"),
        scrypt_sync_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // crypto.hkdfSync(digest, ikm, salt, info, keylen)
    let hkdf_sync_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let digest = args
            .get(0)
            .and_then(|v| v.as_string())
            .ok_or_else(|| JsNativeError::typ().with_message("digest must be a string"))?
            .to_std_string_escaped();
        let ikm_arg = args
            .get(1)
            .ok_or_else(|| JsNativeError::typ().with_message("ikm is required"))?;
        let salt_arg = args
            .get(2)
            .ok_or_else(|| JsNativeError::typ().with_message("salt is required"))?;
        let info_arg = args
            .get(3)
            .ok_or_else(|| JsNativeError::typ().with_message("info is required"))?;
        let keylen = args
            .get(4)
            .and_then(|v| v.as_number())
            .ok_or_else(|| JsNativeError::typ().with_message("keylen must be a number"))?
            as usize;

        let ikm_bytes = js_value_to_bytes(ikm_arg, context)?;
        let salt_bytes = js_value_to_bytes(salt_arg, context)?;
        let info_bytes = js_value_to_bytes(info_arg, context)?;

        let result = hkdf_derive(&digest, &ikm_bytes, &salt_bytes, &info_bytes, keylen)
            .map_err(|e| JsNativeError::error().with_message(e))?;

        let uint8_array = JsUint8Array::from_iter(result, context)?;
        Ok(uint8_array.into())
    });
    context.global_object().set(
        js_string!("__viper_hkdf_sync"),
        hkdf_sync_fn.to_js_function(context.realm()),
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
            randomBytes: (size, callback) => {
                if (typeof callback === 'function') {
                    setTimeout(() => {
                        try {
                            const result = __viper_random_bytes(size);
                            callback(null, result);
                        } catch (err) {
                            callback(err);
                        }
                    }, 0);
                    return;
                }
                return __viper_random_bytes(size);
            },

            // Random integer in range
            randomInt: (min, max, callback) => {
                if (typeof max === 'function') {
                    callback = max;
                    max = min;
                    min = 0;
                }
                if (typeof callback === 'function') {
                    setTimeout(() => {
                        try {
                            const result = __viper_random_int(min, max);
                            callback(null, result);
                        } catch (err) {
                            callback(err);
                        }
                    }, 0);
                    return;
                }
                return __viper_random_int(min, max);
            },

            // Fill buffer with random bytes synchronously
            randomFillSync: (buffer, offset, size) => {
                offset = offset || 0;
                size = size || buffer.length - offset;
                const random = __viper_random_bytes(size);
                for (let i = 0; i < size; i++) {
                    buffer[offset + i] = random[i];
                }
                return buffer;
            },

            // Fill buffer with random bytes asynchronously
            randomFill: (buffer, offset, size, callback) => {
                if (typeof offset === 'function') {
                    callback = offset;
                    offset = 0;
                    size = buffer.length;
                } else if (typeof size === 'function') {
                    callback = size;
                    size = buffer.length - offset;
                }
                setTimeout(() => {
                    try {
                        crypto.randomFillSync(buffer, offset, size);
                        callback(null, buffer);
                    } catch (err) {
                        callback(err);
                    }
                }, 0);
            },

            // Timing-safe comparison
            timingSafeEqual: (a, b) => __viper_timing_safe_equal(a, b),

            // Create hash object
            createHash: (algorithm) => __viper_create_hash(algorithm),

            // Create HMAC object
            createHmac: (algorithm, key) => __viper_create_hmac(algorithm, key),

            // Create cipher object
            createCipheriv: (algorithm, key, iv, options) => __viper_create_cipheriv(algorithm, key, iv),

            // Create decipher object
            createDecipheriv: (algorithm, key, iv, options) => __viper_create_decipheriv(algorithm, key, iv),

            // PBKDF2 synchronous
            pbkdf2Sync: (password, salt, iterations, keylen, digest) => {
                return __viper_pbkdf2_sync(password, salt, iterations, keylen, digest || 'sha256');
            },

            // PBKDF2 asynchronous
            pbkdf2: (password, salt, iterations, keylen, digest, callback) => {
                let actualDigest = digest;
                let actualCallback = callback;

                if (typeof digest === 'function') {
                    actualCallback = digest;
                    actualDigest = 'sha256';
                }

                if (typeof actualCallback !== 'function') {
                    throw new TypeError('callback must be a function');
                }

                setTimeout(() => {
                    try {
                        const result = __viper_pbkdf2_sync(password, salt, iterations, keylen, actualDigest || 'sha256');
                        actualCallback(null, result);
                    } catch (err) {
                        actualCallback(err);
                    }
                }, 0);
            },

            // Scrypt synchronous
            scryptSync: (password, salt, keylen, options) => {
                return __viper_scrypt_sync(password, salt, keylen, options);
            },

            // Scrypt asynchronous
            scrypt: (password, salt, keylen, options, callback) => {
                if (typeof options === 'function') {
                    callback = options;
                    options = {};
                }

                if (typeof callback !== 'function') {
                    throw new TypeError('callback must be a function');
                }

                setTimeout(() => {
                    try {
                        const result = __viper_scrypt_sync(password, salt, keylen, options);
                        callback(null, result);
                    } catch (err) {
                        callback(err);
                    }
                }, 0);
            },

            // HKDF synchronous
            hkdfSync: (digest, ikm, salt, info, keylen) => {
                return __viper_hkdf_sync(digest, ikm, salt, info, keylen);
            },

            // HKDF asynchronous
            hkdf: (digest, ikm, salt, info, keylen, callback) => {
                if (typeof callback !== 'function') {
                    throw new TypeError('callback must be a function');
                }

                setTimeout(() => {
                    try {
                        const result = __viper_hkdf_sync(digest, ikm, salt, info, keylen);
                        callback(null, result);
                    } catch (err) {
                        callback(err);
                    }
                }, 0);
            },

            // Get available ciphers
            getCiphers: () => [
                'aes-128-cbc', 'aes-192-cbc', 'aes-256-cbc',
                'aes-128-ctr', 'aes-192-ctr', 'aes-256-ctr'
            ],

            // Get available hashes
            getHashes: () => [
                'md5', 'sha1', 'sha224', 'sha256', 'sha384', 'sha512'
            ],

            // Subtle crypto (Web Crypto API)
            subtle: {
                digest: async (algorithm, data) => {
                    const algoName = typeof algorithm === 'string' ? algorithm : algorithm.name;
                    const hash = crypto.createHash(algoName.toLowerCase().replace('-', ''));
                    hash.update(new Uint8Array(data));
                    return hash.digest().buffer;
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
                deriveBits: async (algorithm, baseKey, length) => {
                    throw new Error('crypto.subtle.deriveBits() is not yet implemented');
                },
                deriveKey: async (algorithm, baseKey, derivedKeyAlgorithm, extractable, keyUsages) => {
                    throw new Error('crypto.subtle.deriveKey() is not yet implemented');
                },
                wrapKey: async (format, key, wrappingKey, wrapAlgorithm) => {
                    throw new Error('crypto.subtle.wrapKey() is not yet implemented');
                },
                unwrapKey: async (format, wrappedKey, unwrappingKey, unwrapAlgorithm, unwrappedKeyAlgorithm, extractable, keyUsages) => {
                    throw new Error('crypto.subtle.unwrapKey() is not yet implemented');
                },
            },

            // Constants
            constants: {
                OPENSSL_VERSION_NUMBER: 0,
                SSL_OP_ALL: 0,
                SSL_OP_ALLOW_NO_DHE_KEX: 0,
                SSL_OP_ALLOW_UNSAFE_LEGACY_RENEGOTIATION: 0,
                SSL_OP_CIPHER_SERVER_PREFERENCE: 0,
                SSL_OP_CISCO_ANYCONNECT: 0,
                SSL_OP_COOKIE_EXCHANGE: 0,
                SSL_OP_CRYPTOPRO_TLSEXT_BUG: 0,
                SSL_OP_DONT_INSERT_EMPTY_FRAGMENTS: 0,
                SSL_OP_EPHEMERAL_RSA: 0,
                SSL_OP_LEGACY_SERVER_CONNECT: 0,
                SSL_OP_MICROSOFT_BIG_SSLV3_BUFFER: 0,
                SSL_OP_MICROSOFT_SESS_ID_BUG: 0,
                SSL_OP_MSIE_SSLV2_RSA_PADDING: 0,
                SSL_OP_NETSCAPE_CA_DN_BUG: 0,
                SSL_OP_NETSCAPE_CHALLENGE_BUG: 0,
                SSL_OP_NETSCAPE_DEMO_CIPHER_CHANGE_BUG: 0,
                SSL_OP_NETSCAPE_REUSE_CIPHER_CHANGE_BUG: 0,
                SSL_OP_NO_COMPRESSION: 0,
                SSL_OP_NO_ENCRYPT_THEN_MAC: 0,
                SSL_OP_NO_QUERY_MTU: 0,
                SSL_OP_NO_RENEGOTIATION: 0,
                SSL_OP_NO_SESSION_RESUMPTION_ON_RENEGOTIATION: 0,
                SSL_OP_NO_SSLv2: 0,
                SSL_OP_NO_SSLv3: 0,
                SSL_OP_NO_TICKET: 0,
                SSL_OP_NO_TLSv1: 0,
                SSL_OP_NO_TLSv1_1: 0,
                SSL_OP_NO_TLSv1_2: 0,
                SSL_OP_NO_TLSv1_3: 0,
                SSL_OP_PKCS1_CHECK_1: 0,
                SSL_OP_PKCS1_CHECK_2: 0,
                SSL_OP_PRIORITIZE_CHACHA: 0,
                SSL_OP_SINGLE_DH_USE: 0,
                SSL_OP_SINGLE_ECDH_USE: 0,
                SSL_OP_SSLEAY_080_CLIENT_DH_BUG: 0,
                SSL_OP_SSLREF2_REUSE_CERT_TYPE_BUG: 0,
                SSL_OP_TLS_BLOCK_PADDING_BUG: 0,
                SSL_OP_TLS_D5_BUG: 0,
                SSL_OP_TLS_ROLLBACK_BUG: 0,
            },
        };
    "#;

    let source = Source::from_bytes(crypto_code.as_bytes());
    context.eval(source)?;

    Ok(())
}
