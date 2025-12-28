//! Buffer API - Node.js compatible buffer module
//!
//! High-performance native Rust Buffer implementation.
//! Buffer extends Uint8Array with additional methods for binary data manipulation.
//!
//! Provides:
//! - Buffer.alloc(size[, fill[, encoding]]) - Allocate zero-filled buffer
//! - Buffer.allocUnsafe(size) - Allocate uninitialized buffer (fast)
//! - Buffer.from(array|string|buffer[, encoding]) - Create from data
//! - Buffer.concat(list[, totalLength]) - Concatenate buffers
//! - Buffer.byteLength(string[, encoding]) - Get byte length
//! - Buffer.compare(buf1, buf2) - Compare two buffers
//! - Buffer.isBuffer(obj) - Check if object is a Buffer
//! - Buffer.isEncoding(encoding) - Check if encoding is supported
//! - All read/write methods for integers, floats, etc.

use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
use boa_engine::{
    Context, JsArgs, JsNativeError, JsObject, JsResult, JsValue, NativeFunction, js_string,
    object::builtins::JsArrayBuffer, object::builtins::JsUint8Array,
};

/// Supported encodings
const ENCODINGS: &[&str] = &[
    "utf8",
    "utf-8",
    "hex",
    "base64",
    "base64url",
    "ascii",
    "latin1",
    "binary",
    "ucs2",
    "ucs-2",
    "utf16le",
    "utf-16le",
];

/// Register the buffer module
pub fn register_buffer_module(context: &mut Context) -> JsResult<()> {
    // Create Buffer constructor function
    let buffer_alloc = NativeFunction::from_fn_ptr(buffer_alloc);
    let buffer_alloc_unsafe = NativeFunction::from_fn_ptr(buffer_alloc_unsafe);
    let buffer_from = NativeFunction::from_fn_ptr(buffer_from);
    let buffer_concat = NativeFunction::from_fn_ptr(buffer_concat);
    let buffer_byte_length = NativeFunction::from_fn_ptr(buffer_byte_length);
    let buffer_compare = NativeFunction::from_fn_ptr(buffer_compare);
    let buffer_is_buffer = NativeFunction::from_fn_ptr(buffer_is_buffer);
    let buffer_is_encoding = NativeFunction::from_fn_ptr(buffer_is_encoding);

    // Create Buffer object with static methods
    let buffer_obj = JsObject::with_null_proto();

    buffer_obj.set(
        js_string!("alloc"),
        buffer_alloc.to_js_function(context.realm()),
        false,
        context,
    )?;

    buffer_obj.set(
        js_string!("allocUnsafe"),
        buffer_alloc_unsafe.clone().to_js_function(context.realm()),
        false,
        context,
    )?;

    buffer_obj.set(
        js_string!("allocUnsafeSlow"),
        buffer_alloc_unsafe.to_js_function(context.realm()),
        false,
        context,
    )?;

    buffer_obj.set(
        js_string!("from"),
        buffer_from.to_js_function(context.realm()),
        false,
        context,
    )?;

    buffer_obj.set(
        js_string!("concat"),
        buffer_concat.to_js_function(context.realm()),
        false,
        context,
    )?;

    buffer_obj.set(
        js_string!("byteLength"),
        buffer_byte_length.to_js_function(context.realm()),
        false,
        context,
    )?;

    buffer_obj.set(
        js_string!("compare"),
        buffer_compare.to_js_function(context.realm()),
        false,
        context,
    )?;

    buffer_obj.set(
        js_string!("isBuffer"),
        buffer_is_buffer.to_js_function(context.realm()),
        false,
        context,
    )?;

    buffer_obj.set(
        js_string!("isEncoding"),
        buffer_is_encoding.to_js_function(context.realm()),
        false,
        context,
    )?;

    // poolSize property
    buffer_obj.set(js_string!("poolSize"), JsValue::from(8192), false, context)?;

    // Set Buffer on global
    context
        .global_object()
        .set(js_string!("Buffer"), buffer_obj.clone(), false, context)?;

    // Register helper functions for Buffer instances
    register_buffer_prototype_helpers(context)?;

    // Create buffer module object
    let buffer_module = JsObject::with_null_proto();
    buffer_module.set(js_string!("Buffer"), buffer_obj.clone(), false, context)?;

    // Constants
    let constants = JsObject::with_null_proto();
    constants.set(
        js_string!("MAX_LENGTH"),
        JsValue::from(0x7FFFFFFF_i32), // ~2GB
        false,
        context,
    )?;
    constants.set(
        js_string!("MAX_STRING_LENGTH"),
        JsValue::from(0x1FFFFFE8_i32), // V8's limit
        false,
        context,
    )?;
    buffer_module.set(js_string!("constants"), constants, false, context)?;

    // kMaxLength alias
    buffer_module.set(
        js_string!("kMaxLength"),
        JsValue::from(0x7FFFFFFF_i32),
        false,
        context,
    )?;

    // INSPECT_MAX_BYTES
    buffer_module.set(
        js_string!("INSPECT_MAX_BYTES"),
        JsValue::from(50),
        false,
        context,
    )?;

    context
        .global_object()
        .set(js_string!("buffer"), buffer_module, false, context)?;

    Ok(())
}

/// Register prototype helper functions that will be attached to Buffer instances
fn register_buffer_prototype_helpers(context: &mut Context) -> JsResult<()> {
    // We'll create helper functions that work with Uint8Array instances
    let helpers_code = r#"
    (function() {
        // Store original Buffer reference
        const _Buffer = globalThis.Buffer;

        // Helper to create a Buffer-like Uint8Array with extra methods
        globalThis.__createBuffer = function(uint8array) {
            // Add Buffer methods to the Uint8Array instance
            Object.defineProperties(uint8array, {
                // toString with encoding support
                toString: {
                    value: function(encoding = 'utf8', start = 0, end = this.length) {
                        return __bufferToString(this, encoding, start, end);
                    },
                    writable: true,
                    configurable: true
                },
                // write method
                write: {
                    value: function(string, offset = 0, length, encoding = 'utf8') {
                        if (typeof offset === 'string') {
                            encoding = offset;
                            offset = 0;
                            length = this.length;
                        } else if (typeof length === 'string') {
                            encoding = length;
                            length = this.length - offset;
                        }
                        length = length === undefined ? this.length - offset : length;
                        return __bufferWrite(this, string, offset, length, encoding);
                    },
                    writable: true,
                    configurable: true
                },
                // toJSON
                toJSON: {
                    value: function() {
                        return { type: 'Buffer', data: Array.from(this) };
                    },
                    writable: true,
                    configurable: true
                },
                // equals
                equals: {
                    value: function(other) {
                        if (this.length !== other.length) return false;
                        for (let i = 0; i < this.length; i++) {
                            if (this[i] !== other[i]) return false;
                        }
                        return true;
                    },
                    writable: true,
                    configurable: true
                },
                // compare
                compare: {
                    value: function(target, targetStart = 0, targetEnd = target.length, sourceStart = 0, sourceEnd = this.length) {
                        for (let i = sourceStart, j = targetStart; i < sourceEnd && j < targetEnd; i++, j++) {
                            if (this[i] < target[j]) return -1;
                            if (this[i] > target[j]) return 1;
                        }
                        const sourceLen = sourceEnd - sourceStart;
                        const targetLen = targetEnd - targetStart;
                        if (sourceLen < targetLen) return -1;
                        if (sourceLen > targetLen) return 1;
                        return 0;
                    },
                    writable: true,
                    configurable: true
                },
                // copy
                copy: {
                    value: function(target, targetStart = 0, sourceStart = 0, sourceEnd = this.length) {
                        let copied = 0;
                        for (let i = sourceStart; i < sourceEnd && targetStart + copied < target.length; i++) {
                            target[targetStart + copied] = this[i];
                            copied++;
                        }
                        return copied;
                    },
                    writable: true,
                    configurable: true
                },
                // slice (returns a new Buffer view)
                slice: {
                    value: function(start = 0, end = this.length) {
                        return __createBuffer(this.subarray(start, end));
                    },
                    writable: true,
                    configurable: true
                },
                // fill
                fill: {
                    value: function(value, offset = 0, end = this.length, encoding = 'utf8') {
                        if (typeof value === 'string') {
                            if (typeof offset === 'string') {
                                encoding = offset;
                                offset = 0;
                                end = this.length;
                            }
                            const bytes = __stringToBytes(value, encoding);
                            for (let i = offset; i < end; i++) {
                                this[i] = bytes[(i - offset) % bytes.length];
                            }
                        } else if (typeof value === 'number') {
                            value = value & 0xFF;
                            for (let i = offset; i < end; i++) {
                                this[i] = value;
                            }
                        }
                        return this;
                    },
                    writable: true,
                    configurable: true
                },
                // includes
                includes: {
                    value: function(value, byteOffset = 0, encoding = 'utf8') {
                        return this.indexOf(value, byteOffset, encoding) !== -1;
                    },
                    writable: true,
                    configurable: true
                },
                // indexOf
                indexOf: {
                    value: function(value, byteOffset = 0, encoding = 'utf8') {
                        if (typeof value === 'string') {
                            const bytes = __stringToBytes(value, encoding);
                            outer: for (let i = byteOffset; i <= this.length - bytes.length; i++) {
                                for (let j = 0; j < bytes.length; j++) {
                                    if (this[i + j] !== bytes[j]) continue outer;
                                }
                                return i;
                            }
                            return -1;
                        } else if (typeof value === 'number') {
                            value = value & 0xFF;
                            for (let i = byteOffset; i < this.length; i++) {
                                if (this[i] === value) return i;
                            }
                            return -1;
                        }
                        return -1;
                    },
                    writable: true,
                    configurable: true
                },
                // lastIndexOf
                lastIndexOf: {
                    value: function(value, byteOffset = this.length - 1, encoding = 'utf8') {
                        if (typeof value === 'string') {
                            const bytes = __stringToBytes(value, encoding);
                            outer: for (let i = Math.min(byteOffset, this.length - bytes.length); i >= 0; i--) {
                                for (let j = 0; j < bytes.length; j++) {
                                    if (this[i + j] !== bytes[j]) continue outer;
                                }
                                return i;
                            }
                            return -1;
                        } else if (typeof value === 'number') {
                            value = value & 0xFF;
                            for (let i = Math.min(byteOffset, this.length - 1); i >= 0; i--) {
                                if (this[i] === value) return i;
                            }
                            return -1;
                        }
                        return -1;
                    },
                    writable: true,
                    configurable: true
                },
                // swap16
                swap16: {
                    value: function() {
                        if (this.length % 2 !== 0) throw new RangeError('Buffer size must be a multiple of 16-bits');
                        for (let i = 0; i < this.length; i += 2) {
                            const t = this[i];
                            this[i] = this[i + 1];
                            this[i + 1] = t;
                        }
                        return this;
                    },
                    writable: true,
                    configurable: true
                },
                // swap32
                swap32: {
                    value: function() {
                        if (this.length % 4 !== 0) throw new RangeError('Buffer size must be a multiple of 32-bits');
                        for (let i = 0; i < this.length; i += 4) {
                            let t = this[i];
                            this[i] = this[i + 3];
                            this[i + 3] = t;
                            t = this[i + 1];
                            this[i + 1] = this[i + 2];
                            this[i + 2] = t;
                        }
                        return this;
                    },
                    writable: true,
                    configurable: true
                },
                // swap64
                swap64: {
                    value: function() {
                        if (this.length % 8 !== 0) throw new RangeError('Buffer size must be a multiple of 64-bits');
                        for (let i = 0; i < this.length; i += 8) {
                            for (let j = 0; j < 4; j++) {
                                const t = this[i + j];
                                this[i + j] = this[i + 7 - j];
                                this[i + 7 - j] = t;
                            }
                        }
                        return this;
                    },
                    writable: true,
                    configurable: true
                },
                // Read methods
                readUInt8: {
                    value: function(offset = 0) {
                        return this[offset];
                    },
                    writable: true,
                    configurable: true
                },
                readUint8: { get: function() { return this.readUInt8; } },
                readInt8: {
                    value: function(offset = 0) {
                        const val = this[offset];
                        return val > 127 ? val - 256 : val;
                    },
                    writable: true,
                    configurable: true
                },
                readUInt16BE: {
                    value: function(offset = 0) {
                        return (this[offset] << 8) | this[offset + 1];
                    },
                    writable: true,
                    configurable: true
                },
                readUint16BE: { get: function() { return this.readUInt16BE; } },
                readUInt16LE: {
                    value: function(offset = 0) {
                        return this[offset] | (this[offset + 1] << 8);
                    },
                    writable: true,
                    configurable: true
                },
                readUint16LE: { get: function() { return this.readUInt16LE; } },
                readInt16BE: {
                    value: function(offset = 0) {
                        const val = (this[offset] << 8) | this[offset + 1];
                        return val > 32767 ? val - 65536 : val;
                    },
                    writable: true,
                    configurable: true
                },
                readInt16LE: {
                    value: function(offset = 0) {
                        const val = this[offset] | (this[offset + 1] << 8);
                        return val > 32767 ? val - 65536 : val;
                    },
                    writable: true,
                    configurable: true
                },
                readUInt32BE: {
                    value: function(offset = 0) {
                        return ((this[offset] << 24) | (this[offset + 1] << 16) | (this[offset + 2] << 8) | this[offset + 3]) >>> 0;
                    },
                    writable: true,
                    configurable: true
                },
                readUint32BE: { get: function() { return this.readUInt32BE; } },
                readUInt32LE: {
                    value: function(offset = 0) {
                        return (this[offset] | (this[offset + 1] << 8) | (this[offset + 2] << 16) | (this[offset + 3] << 24)) >>> 0;
                    },
                    writable: true,
                    configurable: true
                },
                readUint32LE: { get: function() { return this.readUInt32LE; } },
                readInt32BE: {
                    value: function(offset = 0) {
                        return (this[offset] << 24) | (this[offset + 1] << 16) | (this[offset + 2] << 8) | this[offset + 3];
                    },
                    writable: true,
                    configurable: true
                },
                readInt32LE: {
                    value: function(offset = 0) {
                        return this[offset] | (this[offset + 1] << 8) | (this[offset + 2] << 16) | (this[offset + 3] << 24);
                    },
                    writable: true,
                    configurable: true
                },
                readFloatBE: {
                    value: function(offset = 0) {
                        const view = new DataView(this.buffer, this.byteOffset + offset, 4);
                        return view.getFloat32(0, false);
                    },
                    writable: true,
                    configurable: true
                },
                readFloatLE: {
                    value: function(offset = 0) {
                        const view = new DataView(this.buffer, this.byteOffset + offset, 4);
                        return view.getFloat32(0, true);
                    },
                    writable: true,
                    configurable: true
                },
                readDoubleBE: {
                    value: function(offset = 0) {
                        const view = new DataView(this.buffer, this.byteOffset + offset, 8);
                        return view.getFloat64(0, false);
                    },
                    writable: true,
                    configurable: true
                },
                readDoubleLE: {
                    value: function(offset = 0) {
                        const view = new DataView(this.buffer, this.byteOffset + offset, 8);
                        return view.getFloat64(0, true);
                    },
                    writable: true,
                    configurable: true
                },
                readBigInt64BE: {
                    value: function(offset = 0) {
                        const view = new DataView(this.buffer, this.byteOffset + offset, 8);
                        return view.getBigInt64(0, false);
                    },
                    writable: true,
                    configurable: true
                },
                readBigInt64LE: {
                    value: function(offset = 0) {
                        const view = new DataView(this.buffer, this.byteOffset + offset, 8);
                        return view.getBigInt64(0, true);
                    },
                    writable: true,
                    configurable: true
                },
                readBigUInt64BE: {
                    value: function(offset = 0) {
                        const view = new DataView(this.buffer, this.byteOffset + offset, 8);
                        return view.getBigUint64(0, false);
                    },
                    writable: true,
                    configurable: true
                },
                readBigUint64BE: { get: function() { return this.readBigUInt64BE; } },
                readBigUInt64LE: {
                    value: function(offset = 0) {
                        const view = new DataView(this.buffer, this.byteOffset + offset, 8);
                        return view.getBigUint64(0, true);
                    },
                    writable: true,
                    configurable: true
                },
                readBigUint64LE: { get: function() { return this.readBigUInt64LE; } },
                // Write methods
                writeUInt8: {
                    value: function(value, offset = 0) {
                        this[offset] = value & 0xFF;
                        return offset + 1;
                    },
                    writable: true,
                    configurable: true
                },
                writeUint8: { get: function() { return this.writeUInt8; } },
                writeInt8: {
                    value: function(value, offset = 0) {
                        this[offset] = value < 0 ? value + 256 : value;
                        return offset + 1;
                    },
                    writable: true,
                    configurable: true
                },
                writeUInt16BE: {
                    value: function(value, offset = 0) {
                        this[offset] = (value >> 8) & 0xFF;
                        this[offset + 1] = value & 0xFF;
                        return offset + 2;
                    },
                    writable: true,
                    configurable: true
                },
                writeUint16BE: { get: function() { return this.writeUInt16BE; } },
                writeUInt16LE: {
                    value: function(value, offset = 0) {
                        this[offset] = value & 0xFF;
                        this[offset + 1] = (value >> 8) & 0xFF;
                        return offset + 2;
                    },
                    writable: true,
                    configurable: true
                },
                writeUint16LE: { get: function() { return this.writeUInt16LE; } },
                writeInt16BE: {
                    value: function(value, offset = 0) {
                        if (value < 0) value = 65536 + value;
                        this[offset] = (value >> 8) & 0xFF;
                        this[offset + 1] = value & 0xFF;
                        return offset + 2;
                    },
                    writable: true,
                    configurable: true
                },
                writeInt16LE: {
                    value: function(value, offset = 0) {
                        if (value < 0) value = 65536 + value;
                        this[offset] = value & 0xFF;
                        this[offset + 1] = (value >> 8) & 0xFF;
                        return offset + 2;
                    },
                    writable: true,
                    configurable: true
                },
                writeUInt32BE: {
                    value: function(value, offset = 0) {
                        this[offset] = (value >> 24) & 0xFF;
                        this[offset + 1] = (value >> 16) & 0xFF;
                        this[offset + 2] = (value >> 8) & 0xFF;
                        this[offset + 3] = value & 0xFF;
                        return offset + 4;
                    },
                    writable: true,
                    configurable: true
                },
                writeUint32BE: { get: function() { return this.writeUInt32BE; } },
                writeUInt32LE: {
                    value: function(value, offset = 0) {
                        this[offset] = value & 0xFF;
                        this[offset + 1] = (value >> 8) & 0xFF;
                        this[offset + 2] = (value >> 16) & 0xFF;
                        this[offset + 3] = (value >> 24) & 0xFF;
                        return offset + 4;
                    },
                    writable: true,
                    configurable: true
                },
                writeUint32LE: { get: function() { return this.writeUInt32LE; } },
                writeInt32BE: {
                    value: function(value, offset = 0) {
                        return this.writeUInt32BE(value >>> 0, offset);
                    },
                    writable: true,
                    configurable: true
                },
                writeInt32LE: {
                    value: function(value, offset = 0) {
                        return this.writeUInt32LE(value >>> 0, offset);
                    },
                    writable: true,
                    configurable: true
                },
                writeFloatBE: {
                    value: function(value, offset = 0) {
                        const view = new DataView(this.buffer, this.byteOffset + offset, 4);
                        view.setFloat32(0, value, false);
                        return offset + 4;
                    },
                    writable: true,
                    configurable: true
                },
                writeFloatLE: {
                    value: function(value, offset = 0) {
                        const view = new DataView(this.buffer, this.byteOffset + offset, 4);
                        view.setFloat32(0, value, true);
                        return offset + 4;
                    },
                    writable: true,
                    configurable: true
                },
                writeDoubleBE: {
                    value: function(value, offset = 0) {
                        const view = new DataView(this.buffer, this.byteOffset + offset, 8);
                        view.setFloat64(0, value, false);
                        return offset + 8;
                    },
                    writable: true,
                    configurable: true
                },
                writeDoubleLE: {
                    value: function(value, offset = 0) {
                        const view = new DataView(this.buffer, this.byteOffset + offset, 8);
                        view.setFloat64(0, value, true);
                        return offset + 8;
                    },
                    writable: true,
                    configurable: true
                },
                writeBigInt64BE: {
                    value: function(value, offset = 0) {
                        const view = new DataView(this.buffer, this.byteOffset + offset, 8);
                        view.setBigInt64(0, value, false);
                        return offset + 8;
                    },
                    writable: true,
                    configurable: true
                },
                writeBigInt64LE: {
                    value: function(value, offset = 0) {
                        const view = new DataView(this.buffer, this.byteOffset + offset, 8);
                        view.setBigInt64(0, value, true);
                        return offset + 8;
                    },
                    writable: true,
                    configurable: true
                },
                writeBigUInt64BE: {
                    value: function(value, offset = 0) {
                        const view = new DataView(this.buffer, this.byteOffset + offset, 8);
                        view.setBigUint64(0, value, false);
                        return offset + 8;
                    },
                    writable: true,
                    configurable: true
                },
                writeBigUint64BE: { get: function() { return this.writeBigUInt64BE; } },
                writeBigUInt64LE: {
                    value: function(value, offset = 0) {
                        const view = new DataView(this.buffer, this.byteOffset + offset, 8);
                        view.setBigUint64(0, value, true);
                        return offset + 8;
                    },
                    writable: true,
                    configurable: true
                },
                writeBigUint64LE: { get: function() { return this.writeBigUInt64LE; } },
                // readIntBE/LE and readUIntBE/LE for variable byte length
                readIntBE: {
                    value: function(offset, byteLength) {
                        let val = 0;
                        for (let i = 0; i < byteLength; i++) {
                            val = (val << 8) | this[offset + i];
                        }
                        // Sign extend
                        const highBit = 1 << (byteLength * 8 - 1);
                        if (val >= highBit) {
                            val -= (1 << (byteLength * 8));
                        }
                        return val;
                    },
                    writable: true,
                    configurable: true
                },
                readIntLE: {
                    value: function(offset, byteLength) {
                        let val = 0;
                        for (let i = byteLength - 1; i >= 0; i--) {
                            val = (val << 8) | this[offset + i];
                        }
                        const highBit = 1 << (byteLength * 8 - 1);
                        if (val >= highBit) {
                            val -= (1 << (byteLength * 8));
                        }
                        return val;
                    },
                    writable: true,
                    configurable: true
                },
                readUIntBE: {
                    value: function(offset, byteLength) {
                        let val = 0;
                        for (let i = 0; i < byteLength; i++) {
                            val = (val << 8) | this[offset + i];
                        }
                        return val;
                    },
                    writable: true,
                    configurable: true
                },
                readUintBE: { get: function() { return this.readUIntBE; } },
                readUIntLE: {
                    value: function(offset, byteLength) {
                        let val = 0;
                        for (let i = byteLength - 1; i >= 0; i--) {
                            val = (val << 8) | this[offset + i];
                        }
                        return val;
                    },
                    writable: true,
                    configurable: true
                },
                readUintLE: { get: function() { return this.readUIntLE; } },
                writeIntBE: {
                    value: function(value, offset, byteLength) {
                        for (let i = byteLength - 1; i >= 0; i--) {
                            this[offset + i] = value & 0xFF;
                            value >>= 8;
                        }
                        return offset + byteLength;
                    },
                    writable: true,
                    configurable: true
                },
                writeIntLE: {
                    value: function(value, offset, byteLength) {
                        for (let i = 0; i < byteLength; i++) {
                            this[offset + i] = value & 0xFF;
                            value >>= 8;
                        }
                        return offset + byteLength;
                    },
                    writable: true,
                    configurable: true
                },
                writeUIntBE: {
                    value: function(value, offset, byteLength) {
                        for (let i = byteLength - 1; i >= 0; i--) {
                            this[offset + i] = value & 0xFF;
                            value >>= 8;
                        }
                        return offset + byteLength;
                    },
                    writable: true,
                    configurable: true
                },
                writeUintBE: { get: function() { return this.writeUIntBE; } },
                writeUIntLE: {
                    value: function(value, offset, byteLength) {
                        for (let i = 0; i < byteLength; i++) {
                            this[offset + i] = value & 0xFF;
                            value >>= 8;
                        }
                        return offset + byteLength;
                    },
                    writable: true,
                    configurable: true
                },
                writeUintLE: { get: function() { return this.writeUIntLE; } },
            });

            return uint8array;
        };
    })();
    "#;

    let source = boa_engine::Source::from_bytes(helpers_code.as_bytes());
    context.eval(source)?;

    // Register native helper functions for encoding/decoding
    register_encoding_helpers(context)?;

    Ok(())
}

/// Register native encoding helper functions
fn register_encoding_helpers(context: &mut Context) -> JsResult<()> {
    // __bufferToString - convert buffer to string with encoding
    let to_string_fn = NativeFunction::from_fn_ptr(buffer_to_string_native);
    context.global_object().set(
        js_string!("__bufferToString"),
        to_string_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // __bufferWrite - write string to buffer with encoding
    let write_fn = NativeFunction::from_fn_ptr(buffer_write_native);
    context.global_object().set(
        js_string!("__bufferWrite"),
        write_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // __stringToBytes - convert string to bytes with encoding
    let string_to_bytes_fn = NativeFunction::from_fn_ptr(string_to_bytes_native);
    context.global_object().set(
        js_string!("__stringToBytes"),
        string_to_bytes_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    Ok(())
}

/// Buffer.alloc(size[, fill[, encoding]]) - Allocate zero-filled buffer
fn buffer_alloc(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let size = args.get_or_undefined(0).to_u32(context)? as usize;

    // Create ArrayBuffer
    let array_buffer = JsArrayBuffer::new(size, context)?;
    let uint8_array = JsUint8Array::from_array_buffer(array_buffer, context)?;

    // Handle fill if provided
    if let Some(fill_val) = args.get(1) {
        if !fill_val.is_undefined() {
            let encoding = args
                .get(2)
                .map(|v| v.to_string(context).map(|s| s.to_std_string_escaped()))
                .transpose()?
                .unwrap_or_else(|| "utf8".to_string());

            fill_buffer(&uint8_array, fill_val, &encoding, context)?;
        }
    }

    // Create Buffer by calling __createBuffer
    create_buffer_from_uint8array(uint8_array, context)
}

/// Buffer.allocUnsafe(size) - Allocate uninitialized buffer (fast)
fn buffer_alloc_unsafe(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let size = args.get_or_undefined(0).to_u32(context)? as usize;

    // Create ArrayBuffer (Boa always zero-fills, but this is the "unsafe" API)
    let array_buffer = JsArrayBuffer::new(size, context)?;
    let uint8_array = JsUint8Array::from_array_buffer(array_buffer, context)?;

    create_buffer_from_uint8array(uint8_array, context)
}

/// Buffer.from(data[, encoding]) - Create buffer from various sources
fn buffer_from(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let source = args.get_or_undefined(0);
    let encoding = args
        .get(1)
        .map(|v| v.to_string(context).map(|s| s.to_std_string_escaped()))
        .transpose()?
        .unwrap_or_else(|| "utf8".to_string());

    // Handle string
    if let Some(s) = source.as_string() {
        let string = s.to_std_string_escaped();
        let bytes = encode_string(&string, &encoding)?;
        return create_buffer_from_bytes(&bytes, context);
    }

    // Handle array-like (including arrays and other buffers)
    if let Some(obj) = source.as_object() {
        // Check if it's a Uint8Array or similar TypedArray
        if let Ok(typed_array) = JsUint8Array::from_object(obj.clone()) {
            let len = typed_array.length(context)?;
            let mut bytes = vec![0u8; len];
            for i in 0..len {
                if let Ok(val) = typed_array.get(i, context) {
                    bytes[i] = val.to_u32(context).unwrap_or(0) as u8;
                }
            }
            return create_buffer_from_bytes(&bytes, context);
        }

        // Check if it's an array
        if let Ok(length_val) = obj.get(js_string!("length"), context) {
            if let Ok(length) = length_val.to_u32(context) {
                let mut bytes = vec![0u8; length as usize];
                for i in 0..length {
                    if let Ok(val) = obj.get(i, context) {
                        bytes[i as usize] = val.to_u32(context).unwrap_or(0) as u8;
                    }
                }
                return create_buffer_from_bytes(&bytes, context);
            }
        }
    }

    Err(JsNativeError::typ()
        .with_message(
            "First argument must be a string, Buffer, ArrayBuffer, Array, or array-like object",
        )
        .into())
}

/// Buffer.concat(list[, totalLength]) - Concatenate buffers
fn buffer_concat(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let list = args.get_or_undefined(0);
    let total_length = args.get(1);

    let list_obj = list
        .as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("list must be an array"))?;

    let list_len = list_obj
        .get(js_string!("length"), context)?
        .to_u32(context)? as usize;

    // Collect all bytes
    let mut all_bytes: Vec<u8> = Vec::new();
    for i in 0..list_len {
        let item = list_obj.get(i as u32, context)?;
        if let Some(obj) = item.as_object() {
            if let Ok(typed_array) = JsUint8Array::from_object(obj.clone()) {
                let len = typed_array.length(context)?;
                for j in 0..len {
                    if let Ok(val) = typed_array.get(j, context) {
                        all_bytes.push(val.to_u32(context).unwrap_or(0) as u8);
                    }
                }
            }
        }
    }

    // Apply totalLength limit if specified
    if let Some(tl) = total_length {
        if !tl.is_undefined() {
            let limit = tl.to_u32(context)? as usize;
            if limit < all_bytes.len() {
                all_bytes.truncate(limit);
            } else if limit > all_bytes.len() {
                all_bytes.resize(limit, 0);
            }
        }
    }

    create_buffer_from_bytes(&all_bytes, context)
}

/// Buffer.byteLength(string[, encoding])
fn buffer_byte_length(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let source = args.get_or_undefined(0);
    let encoding = args
        .get(1)
        .map(|v| v.to_string(context).map(|s| s.to_std_string_escaped()))
        .transpose()?
        .unwrap_or_else(|| "utf8".to_string());

    // If it's a string, calculate byte length for encoding
    if let Some(s) = source.as_string() {
        let string = s.to_std_string_escaped();
        let bytes = encode_string(&string, &encoding)?;
        return Ok(JsValue::from(bytes.len() as u32));
    }

    // If it's a buffer/typed array, return its length
    if let Some(obj) = source.as_object() {
        if let Ok(typed_array) = JsUint8Array::from_object(obj.clone()) {
            return Ok(JsValue::from(typed_array.length(context)? as u32));
        }
    }

    Ok(JsValue::from(0))
}

/// Buffer.compare(buf1, buf2)
fn buffer_compare(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let buf1 = args.get_or_undefined(0);
    let buf2 = args.get_or_undefined(1);

    let obj1 = buf1
        .as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("buf1 must be a Buffer"))?;
    let obj2 = buf2
        .as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("buf2 must be a Buffer"))?;

    let arr1 = JsUint8Array::from_object(obj1.clone())
        .map_err(|_| JsNativeError::typ().with_message("buf1 must be a Buffer"))?;
    let arr2 = JsUint8Array::from_object(obj2.clone())
        .map_err(|_| JsNativeError::typ().with_message("buf2 must be a Buffer"))?;

    let len1 = arr1.length(context)?;
    let len2 = arr2.length(context)?;
    let min_len = len1.min(len2);

    for i in 0..min_len {
        let v1 = arr1.get(i, context)?.to_u32(context)? as u8;
        let v2 = arr2.get(i, context)?.to_u32(context)? as u8;
        if v1 < v2 {
            return Ok(JsValue::from(-1));
        }
        if v1 > v2 {
            return Ok(JsValue::from(1));
        }
    }

    if len1 < len2 {
        Ok(JsValue::from(-1))
    } else if len1 > len2 {
        Ok(JsValue::from(1))
    } else {
        Ok(JsValue::from(0))
    }
}

/// Buffer.isBuffer(obj)
fn buffer_is_buffer(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let obj = args.get_or_undefined(0);
    if let Some(o) = obj.as_object() {
        // Check if it's a Uint8Array with Buffer methods
        if JsUint8Array::from_object(o.clone()).is_ok() {
            // Check for Buffer-specific method
            if o.has_property(js_string!("readUInt8"), context)? {
                return Ok(JsValue::from(true));
            }
        }
    }
    Ok(JsValue::from(false))
}

/// Buffer.isEncoding(encoding)
fn buffer_is_encoding(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let encoding = args
        .get(0)
        .map(|v| {
            v.to_string(context)
                .map(|s| s.to_std_string_escaped().to_lowercase())
        })
        .transpose()?
        .unwrap_or_default();

    let is_valid = ENCODINGS.iter().any(|&e| e == encoding);
    Ok(JsValue::from(is_valid))
}

/// Native buffer toString implementation
fn buffer_to_string_native(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let buffer = args.get_or_undefined(0);
    let encoding = args
        .get(1)
        .map(|v| v.to_string(context).map(|s| s.to_std_string_escaped()))
        .transpose()?
        .unwrap_or_else(|| "utf8".to_string());
    let start = args
        .get(2)
        .map(|v| v.to_u32(context))
        .transpose()?
        .unwrap_or(0) as usize;
    let end = args.get(3);

    let obj = buffer
        .as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("Expected buffer"))?;
    let typed_array = JsUint8Array::from_object(obj.clone())
        .map_err(|_| JsNativeError::typ().with_message("Expected Uint8Array"))?;

    let len = typed_array.length(context)?;
    let end_idx = end
        .map(|v| v.to_u32(context))
        .transpose()?
        .unwrap_or(len as u32) as usize;

    let actual_end = end_idx.min(len);
    let actual_start = start.min(actual_end);

    let mut bytes = vec![0u8; actual_end - actual_start];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = typed_array
            .get(actual_start + i, context)?
            .to_u32(context)? as u8;
    }

    let result = decode_bytes(&bytes, &encoding)?;
    Ok(JsValue::from(js_string!(result)))
}

/// Native buffer write implementation
fn buffer_write_native(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let buffer = args.get_or_undefined(0);
    let string = args
        .get(1)
        .map(|v| v.to_string(context).map(|s| s.to_std_string_escaped()))
        .transpose()?
        .unwrap_or_default();
    let offset = args
        .get(2)
        .map(|v| v.to_u32(context))
        .transpose()?
        .unwrap_or(0) as usize;
    let length = args.get(3).map(|v| v.to_u32(context)).transpose()?;
    let encoding = args
        .get(4)
        .map(|v| v.to_string(context).map(|s| s.to_std_string_escaped()))
        .transpose()?
        .unwrap_or_else(|| "utf8".to_string());

    let obj = buffer
        .as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("Expected buffer"))?;
    let typed_array = JsUint8Array::from_object(obj.clone())
        .map_err(|_| JsNativeError::typ().with_message("Expected Uint8Array"))?;

    let buf_len = typed_array.length(context)?;
    let bytes = encode_string(&string, &encoding)?;

    let max_len = buf_len.saturating_sub(offset);
    let write_len = length
        .map(|l| l as usize)
        .unwrap_or(max_len)
        .min(max_len)
        .min(bytes.len());

    for i in 0..write_len {
        typed_array.set(offset + i, JsValue::from(bytes[i] as u32), false, context)?;
    }

    Ok(JsValue::from(write_len as u32))
}

/// Native string to bytes conversion
fn string_to_bytes_native(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let string = args
        .get(0)
        .map(|v| v.to_string(context).map(|s| s.to_std_string_escaped()))
        .transpose()?
        .unwrap_or_default();
    let encoding = args
        .get(1)
        .map(|v| v.to_string(context).map(|s| s.to_std_string_escaped()))
        .transpose()?
        .unwrap_or_else(|| "utf8".to_string());

    let bytes = encode_string(&string, &encoding)?;

    // Create a Uint8Array and return it
    let array_buffer = JsArrayBuffer::new(bytes.len(), context)?;
    let uint8_array = JsUint8Array::from_array_buffer(array_buffer, context)?;
    for (i, byte) in bytes.iter().enumerate() {
        uint8_array.set(i, JsValue::from(*byte as u32), false, context)?;
    }

    Ok(uint8_array.into())
}

// ============================================================================
// Helper functions
// ============================================================================

/// Create a Buffer from a Uint8Array by calling __createBuffer
fn create_buffer_from_uint8array(
    uint8_array: JsUint8Array,
    context: &mut Context,
) -> JsResult<JsValue> {
    let create_fn = context
        .global_object()
        .get(js_string!("__createBuffer"), context)?;

    if let Some(fn_obj) = create_fn.as_object() {
        if fn_obj.is_callable() {
            return fn_obj.call(&JsValue::undefined(), &[uint8_array.into()], context);
        }
    }

    // Fallback: return the Uint8Array directly
    Ok(uint8_array.into())
}

/// Create a Buffer from raw bytes
fn create_buffer_from_bytes(bytes: &[u8], context: &mut Context) -> JsResult<JsValue> {
    let array_buffer = JsArrayBuffer::new(bytes.len(), context)?;
    let uint8_array = JsUint8Array::from_array_buffer(array_buffer, context)?;

    for (i, byte) in bytes.iter().enumerate() {
        uint8_array.set(i, JsValue::from(*byte as u32), false, context)?;
    }

    create_buffer_from_uint8array(uint8_array, context)
}

/// Fill a buffer with a value
fn fill_buffer(
    buffer: &JsUint8Array,
    fill_val: &JsValue,
    encoding: &str,
    context: &mut Context,
) -> JsResult<()> {
    let len = buffer.length(context)?;

    if let Some(s) = fill_val.as_string() {
        let string = s.to_std_string_escaped();
        let bytes = encode_string(&string, encoding)?;
        if !bytes.is_empty() {
            for i in 0..len {
                buffer.set(
                    i,
                    JsValue::from(bytes[i % bytes.len()] as u32),
                    false,
                    context,
                )?;
            }
        }
    } else if fill_val.is_number() {
        let val = fill_val.to_u32(context)? as u8;
        for i in 0..len {
            buffer.set(i, JsValue::from(val as u32), false, context)?;
        }
    }

    Ok(())
}

/// Encode a string to bytes using the specified encoding
fn encode_string(string: &str, encoding: &str) -> JsResult<Vec<u8>> {
    let enc = encoding.to_lowercase();
    match enc.as_str() {
        "utf8" | "utf-8" => Ok(string.as_bytes().to_vec()),
        "ascii" | "latin1" | "binary" => {
            Ok(string.chars().map(|c| (c as u32 & 0xFF) as u8).collect())
        }
        "hex" => hex_decode(string),
        "base64" => BASE64_STANDARD.decode(string).map_err(|e| {
            JsNativeError::typ()
                .with_message(format!("Invalid base64: {}", e))
                .into()
        }),
        "base64url" => {
            let standard = string.replace('-', "+").replace('_', "/");
            BASE64_STANDARD.decode(&standard).map_err(|e| {
                JsNativeError::typ()
                    .with_message(format!("Invalid base64url: {}", e))
                    .into()
            })
        }
        "utf16le" | "utf-16le" | "ucs2" | "ucs-2" => {
            let mut bytes = Vec::with_capacity(string.len() * 2);
            for c in string.encode_utf16() {
                bytes.push((c & 0xFF) as u8);
                bytes.push((c >> 8) as u8);
            }
            Ok(bytes)
        }
        _ => Err(JsNativeError::typ()
            .with_message(format!("Unknown encoding: {}", encoding))
            .into()),
    }
}

/// Decode bytes to a string using the specified encoding
fn decode_bytes(bytes: &[u8], encoding: &str) -> JsResult<String> {
    let enc = encoding.to_lowercase();
    match enc.as_str() {
        "utf8" | "utf-8" => String::from_utf8(bytes.to_vec())
            .or_else(|_| Ok(String::from_utf8_lossy(bytes).to_string())),
        "ascii" | "latin1" | "binary" => Ok(bytes.iter().map(|&b| b as char).collect()),
        "hex" => Ok(hex_encode(bytes)),
        "base64" => Ok(BASE64_STANDARD.encode(bytes)),
        "base64url" => {
            let encoded = BASE64_STANDARD.encode(bytes);
            Ok(encoded.replace('+', "-").replace('/', "_").replace('=', ""))
        }
        "utf16le" | "utf-16le" | "ucs2" | "ucs-2" => {
            let mut chars = Vec::new();
            for i in (0..bytes.len()).step_by(2) {
                if i + 1 < bytes.len() {
                    let code = (bytes[i] as u16) | ((bytes[i + 1] as u16) << 8);
                    if let Some(c) = char::from_u32(code as u32) {
                        chars.push(c);
                    }
                }
            }
            Ok(chars.into_iter().collect())
        }
        _ => Err(JsNativeError::typ()
            .with_message(format!("Unknown encoding: {}", encoding))
            .into()),
    }
}

/// Decode hex string to bytes
fn hex_decode(s: &str) -> JsResult<Vec<u8>> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(Vec::new());
    }

    let mut bytes = Vec::with_capacity(s.len() / 2);
    let mut chars = s.chars().peekable();

    while let Some(c1) = chars.next() {
        let c2 = chars.next();
        let hex = match c2 {
            Some(c2) => format!("{}{}", c1, c2),
            None => format!("{}", c1),
        };
        match u8::from_str_radix(&hex, 16) {
            Ok(b) => bytes.push(b),
            Err(_) => break, // Stop at invalid hex
        }
    }

    Ok(bytes)
}

/// Encode bytes to hex string
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
