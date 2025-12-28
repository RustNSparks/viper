# Viper Internal JavaScript Modules

This directory contains internal JavaScript modules for Viper's Node.js compatibility layer.

## Structure

```
src/js/
├── internal/           # Internal utilities (not exposed to users)
│   ├── errors.js       # Node.js error codes and error classes
│   └── validators.js   # Argument validation utilities
└── README.md
```

## Approach

Viper follows Bun's approach to Node.js compatibility:

1. **Implement in JavaScript/TypeScript** where possible for maintainability
2. **Reference Node.js source code** but adapt for Viper's runtime
3. **Use Rust for performance-critical paths** (buffers, crypto, networking)
4. **Keep MIT license** attribution from Node.js

## Internal Modules

### `internal/errors.js`

Provides Node.js-compatible error codes and error classes:

- **Error Classes**: `NodeError`, `NodeTypeError`, `NodeRangeError`, `SystemError`, `AbortError`
- **Error Codes**: All major Node.js error codes (`ERR_INVALID_ARG_TYPE`, `ERR_MODULE_NOT_FOUND`, etc.)
- **Utilities**: `hideStackFrames()`, `genericNodeError()`

Usage in Viper modules:
```javascript
const { codes: { ERR_INVALID_ARG_TYPE } } = require('internal/errors');

function myFunction(name) {
  if (typeof name !== 'string') {
    throw new ERR_INVALID_ARG_TYPE('name', 'string', name);
  }
}
```

### `internal/validators.js`

Provides validation utilities for Node.js API arguments:

- **Type Validators**: `validateString()`, `validateNumber()`, `validateFunction()`, etc.
- **Range Validators**: `validateInteger()`, `validatePort()`, `validateInt32()`
- **Special Validators**: `validateAbortSignal()`, `validateURL()`, `validateEncoding()`

Usage in Viper modules:
```javascript
const { validateString, validateInteger } = require('internal/validators');

function readFile(path, options) {
  validateString(path, 'path');
  if (options?.fd !== undefined) {
    validateInteger(options.fd, 'options.fd', 0);
  }
  // ... rest of implementation
}
```

## Benefits

1. ✅ **Better Error Messages** - Match Node.js error messages exactly
2. ✅ **Consistent Validation** - Reusable validators across all modules
3. ✅ **Type Safety** - Clear error codes for programmatic error handling
4. ✅ **Node.js Compatibility** - Users can catch specific error codes
5. ✅ **Maintainability** - JavaScript is easier to update than Rust for high-level logic

## Error Code Coverage

Currently implemented Node.js error codes:

### Argument Errors
- `ERR_INVALID_ARG_TYPE` - Wrong argument type
- `ERR_INVALID_ARG_VALUE` - Invalid argument value
- `ERR_OUT_OF_RANGE` - Value out of range
- `ERR_MISSING_ARGS` - Missing required arguments

### Module Errors
- `ERR_MODULE_NOT_FOUND` - Module not found
- `ERR_INVALID_MODULE_SPECIFIER` - Invalid import specifier
- `ERR_PACKAGE_PATH_NOT_EXPORTED` - Package subpath not exported
- `ERR_REQUIRE_ESM` - Cannot require() ES module

### Network Errors
- `ERR_SOCKET_BAD_PORT` - Invalid port number
- `ERR_SOCKET_CLOSED` - Socket already closed
- `ERR_SERVER_ALREADY_LISTEN` - Server already listening
- `ERR_INVALID_IP_ADDRESS` - Invalid IP address

### Stream Errors
- `ERR_STREAM_WRITE_AFTER_END` - Write after end
- `ERR_STREAM_DESTROYED` - Stream destroyed
- `ERR_STREAM_PUSH_AFTER_EOF` - Push after EOF

### HTTP Errors
- `ERR_HTTP_HEADERS_SENT` - Headers already sent
- `ERR_HTTP_INVALID_STATUS_CODE` - Invalid status code
- `ERR_HTTP_INVALID_HEADER_VALUE` - Invalid header value

### File System Errors
- `ERR_FS_FILE_TOO_LARGE` - File too large (>2GB)
- `ERR_FS_EISDIR` - Path is a directory

### Crypto Errors
- `ERR_CRYPTO_HASH_FINALIZED` - Hash already finalized
- `ERR_CRYPTO_INVALID_DIGEST` - Invalid digest algorithm

### Buffer Errors
- `ERR_BUFFER_OUT_OF_BOUNDS` - Buffer out of bounds
- `ERR_BUFFER_TOO_LARGE` - Buffer too large

### Generic Errors
- `ERR_INVALID_STATE` - Invalid state
- `ERR_OPERATION_FAILED` - Operation failed
- `ERR_METHOD_NOT_IMPLEMENTED` - Method not implemented
- `ABORT_ERR` - Operation aborted (Web standard)

More error codes can be added as needed.

## Integration with Rust

Viper's Rust runtime can also use these error codes:

```rust
// In Rust
use boa_engine::{JsResult, Context};

pub fn throw_err_invalid_arg_type(
    context: &mut Context,
    name: &str,
    expected: &str,
    actual: &str,
) -> JsResult<()> {
    let error_code = r#"
        const { codes: { ERR_INVALID_ARG_TYPE } } = require('internal/errors');
        throw new ERR_INVALID_ARG_TYPE(name, expected, actual);
    "#;
    context.eval(error_code)?;
    Ok(())
}
```

## Future Work

### Additional Error Codes to Implement
- Worker thread errors (`ERR_WORKER_*`)
- TLS/SSL errors (`ERR_TLS_*`)
- HTTP/2 errors (`ERR_HTTP2_*`)
- VM errors (`ERR_VM_*`)
- WASI errors (`ERR_WASI_*`)

### Additional Internal Modules
- `internal/util.js` - Shared utilities
- `internal/streams.js` - Stream utilities
- `internal/url.js` - URL parsing utilities
- `internal/buffer.js` - Buffer utilities
- `internal/process.js` - Process utilities

### Testing
Each internal module should have comprehensive tests to ensure Node.js compatibility.

## References

- [Node.js errors.js](https://github.com/nodejs/node/blob/main/lib/internal/errors.js)
- [Node.js validators.js](https://github.com/nodejs/node/blob/main/lib/internal/validators.js)
- [Bun internal modules](https://github.com/oven-sh/bun/tree/main/src/js/internal)
- [Node.js Error Codes Documentation](https://nodejs.org/api/errors.html#nodejs-error-codes)

## License

These modules are based on Node.js source code and maintain the Node.js MIT license.

Copyright Joyent, Inc. and other Node contributors.

See individual file headers for full license text.