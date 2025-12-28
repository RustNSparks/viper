// Reimplementation of Node.js internal errors
// Reference: https://github.com/nodejs/node/blob/main/lib/internal/errors.js

// Copyright Joyent, Inc. and other Node contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the
// "Software"), to deal in the Software without restriction, including
// without limitation the rights to use, copy, modify, merge, publish,
// distribute, sublicense, and/or sell copies of the Software, and to permit
// persons to whom the Software is furnished to do so, subject to the
// following conditions:
//
// The above copyright notice and this permission notice shall be included
// in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
// OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN
// NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
// DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
// OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE
// USE OR OTHER DEALINGS IN THE SOFTWARE.

'use strict';

// Error codes and messages
const codes = {};

class NodeError extends Error {
  constructor(code, message) {
    super(message);
    this.name = 'Error';
    this.code = code;
    Error.captureStackTrace(this, NodeError);
  }
}

class NodeTypeError extends TypeError {
  constructor(code, message) {
    super(message);
    this.name = 'TypeError';
    this.code = code;
    Error.captureStackTrace(this, NodeTypeError);
  }
}

class NodeRangeError extends RangeError {
  constructor(code, message) {
    super(message);
    this.name = 'RangeError';
    this.code = code;
    Error.captureStackTrace(this, NodeRangeError);
  }
}

class NodeURIError extends URIError {
  constructor(code, message) {
    super(message);
    this.name = 'URIError';
    this.code = code;
    Error.captureStackTrace(this, NodeURIError);
  }
}

class NodeSyntaxError extends SyntaxError {
  constructor(code, message) {
    super(message);
    this.name = 'SyntaxError';
    this.code = code;
    Error.captureStackTrace(this, NodeSyntaxError);
  }
}

class AbortError extends Error {
  constructor(message = 'The operation was aborted', options) {
    super(message, options);
    this.code = 'ABORT_ERR';
    this.name = 'AbortError';
  }
}

// System error class
class SystemError extends Error {
  constructor(code, message, context = {}) {
    super(message);
    this.code = code;
    this.name = 'SystemError';

    if (context.syscall) this.syscall = context.syscall;
    if (context.path) this.path = context.path;
    if (context.dest) this.dest = context.dest;
    if (context.errno !== undefined) this.errno = context.errno;
    if (context.port !== undefined) this.port = context.port;
    if (context.address) this.address = context.address;

    Error.captureStackTrace(this, SystemError);
  }
}

// Helper to create error constructor
function E(code, message, Base = NodeError) {
  codes[code] = class extends Base {
    constructor(...args) {
      const msg = typeof message === 'function' ? message(...args) : message;
      super(code, msg);
    }
  };
  return codes[code];
}

// Define error codes
// Argument errors
E('ERR_INVALID_ARG_TYPE', function(name, expected, actual) {
  let msg = `The "${name}" argument must be of type ${expected}.`;
  if (actual !== undefined) {
    msg += ` Received type ${typeof actual}`;
  }
  return msg;
}, NodeTypeError);

E('ERR_INVALID_ARG_VALUE', function(name, value, reason = 'is invalid') {
  return `The argument '${name}' ${reason}. Received ${value}`;
}, NodeTypeError);

E('ERR_OUT_OF_RANGE', function(name, range, received) {
  return `The value of "${name}" is out of range. It must be ${range}. Received ${received}`;
}, NodeRangeError);

E('ERR_MISSING_ARGS', function(...args) {
  const names = args.join('", "');
  return `The "${names}" argument${args.length > 1 ? 's' : ''} must be specified`;
}, NodeTypeError);

// Assertion errors
E('ERR_ASSERTION', 'Assertion failed');

E('ERR_AMBIGUOUS_ARGUMENT', function(name, usage) {
  return `The "${name}" argument is ambiguous. ${usage}`;
});

// Buffer errors
E('ERR_BUFFER_OUT_OF_BOUNDS', 'Attempt to access memory outside buffer bounds');
E('ERR_BUFFER_TOO_LARGE', 'Cannot create a Buffer larger than maximum size');
E('ERR_BUFFER_CONTEXT_NOT_AVAILABLE', 'Buffer is not available for the current context');

E('ERR_INVALID_BUFFER_SIZE', 'Buffer size must be a multiple of the specified size');

// Encoding errors
E('ERR_ENCODING_NOT_SUPPORTED', function(encoding) {
  return `The "${encoding}" encoding is not supported`;
}, NodeRangeError);

E('ERR_ENCODING_INVALID_ENCODED_DATA', function(encoding) {
  return `The encoded data was not valid for encoding ${encoding}`;
});

E('ERR_UNKNOWN_ENCODING', function(encoding) {
  return `Unknown encoding: ${encoding}`;
}, NodeTypeError);

// File system errors
E('ERR_FS_FILE_TOO_LARGE', 'File size is greater than 2 GiB');
E('ERR_FS_INVALID_SYMLINK_TYPE', 'Invalid symlink type');
E('ERR_FS_EISDIR', 'Path is a directory');

// HTTP errors
E('ERR_HTTP_HEADERS_SENT', 'Cannot set headers after they are sent to the client');
E('ERR_HTTP_INVALID_HEADER_VALUE', 'Invalid HTTP header value');
E('ERR_HTTP_INVALID_STATUS_CODE', 'Invalid status code');
E('ERR_HTTP_TRAILER_INVALID', 'Trailers are invalid with this transfer encoding');
E('ERR_HTTP_SOCKET_ENCODING', 'Cannot change socket encoding');

// Module errors
E('ERR_MODULE_NOT_FOUND', function(path, base) {
  let msg = `Cannot find module '${path}'`;
  if (base) msg += ` from '${base}'`;
  return msg;
});

E('ERR_INVALID_MODULE_SPECIFIER', function(request, reason) {
  return `Invalid module specifier "${request}"${reason ? ': ' + reason : ''}`;
});

E('ERR_INVALID_PACKAGE_CONFIG', function(path, message) {
  return `Invalid package config ${path}${message ? ': ' + message : ''}`;
});

E('ERR_PACKAGE_PATH_NOT_EXPORTED', function(subpath, pkgPath) {
  return `Package subpath '${subpath}' is not defined by "exports" in ${pkgPath}`;
});

E('ERR_REQUIRE_ESM', function(filename) {
  return `require() of ES Module ${filename} is not supported`;
});

// URL errors
E('ERR_INVALID_URL', 'Invalid URL', NodeTypeError);
E('ERR_INVALID_URL_SCHEME', function(expected) {
  return `The URL must be of scheme ${expected}`;
}, NodeTypeError);

E('ERR_INVALID_FILE_URL_PATH', 'File URL path must be absolute');
E('ERR_INVALID_FILE_URL_HOST', 'File URL host must be "localhost" or empty');

// Crypto errors
E('ERR_CRYPTO_HASH_FINALIZED', 'Digest already called');
E('ERR_CRYPTO_HASH_UPDATE_FAILED', 'Hash update failed');
E('ERR_CRYPTO_INVALID_DIGEST', 'Invalid digest algorithm');
E('ERR_CRYPTO_INVALID_STATE', 'Invalid crypto state');
E('ERR_CRYPTO_SIGN_KEY_REQUIRED', 'Sign key is required');

// Stream errors
E('ERR_STREAM_PUSH_AFTER_EOF', 'Cannot push after EOF');
E('ERR_STREAM_UNSHIFT_AFTER_END_EVENT', 'Cannot unshift after end event');
E('ERR_STREAM_WRITE_AFTER_END', 'Cannot write after end');
E('ERR_STREAM_DESTROYED', 'Cannot call write after a stream was destroyed');
E('ERR_STREAM_ALREADY_FINISHED', 'Stream already finished');
E('ERR_STREAM_CANNOT_PIPE', 'Cannot pipe to non-writable stream');
E('ERR_STREAM_NULL_VALUES', 'May not write null values to stream');
E('ERR_STREAM_PREMATURE_CLOSE', 'Premature close');

// Network errors
E('ERR_SOCKET_BAD_PORT', 'Port should be >= 0 and < 65536');
E('ERR_SOCKET_CLOSED', 'Socket is closed');
E('ERR_SOCKET_DGRAM_NOT_RUNNING', 'Socket is not running');
E('ERR_SOCKET_BAD_TYPE', 'Bad socket type');
E('ERR_SERVER_ALREADY_LISTEN', 'Server is already listening');
E('ERR_SERVER_NOT_RUNNING', 'Server is not running');

E('ERR_INVALID_IP_ADDRESS', 'Invalid IP address');
E('ERR_INVALID_ADDRESS_FAMILY', function(family) {
  return `Invalid address family: ${family}`;
});

// Child process errors
E('ERR_CHILD_PROCESS_IPC_REQUIRED', 'IPC channel is required for forked processes');
E('ERR_CHILD_PROCESS_STDIO_MAXBUFFER', 'stdout/stderr maxBuffer length exceeded');
E('ERR_INVALID_SYNC_FORK_INPUT', 'Asynchronous forks do not support Buffer, TypedArray, DataView or string input');

// Event errors
E('ERR_UNHANDLED_ERROR', function(err) {
  const msg = 'Unhandled error.';
  if (err === undefined) return msg;
  return `${msg} (${err})`;
});

E('ERR_EVENT_RECURSION', function(event) {
  return `The event "${event}" is already being dispatched`;
});

// Worker errors
E('ERR_WORKER_NOT_RUNNING', 'Worker is not running');
E('ERR_WORKER_PATH', 'Worker path must be absolute or relative');
E('ERR_WORKER_UNSERIALIZABLE_ERROR', 'Unserializable error during worker communication');
E('ERR_WORKER_UNSUPPORTED_OPERATION', 'Operation not supported in workers');

// Method errors
E('ERR_METHOD_NOT_IMPLEMENTED', function(method) {
  return `The ${method} method is not implemented`;
});

E('ERR_ILLEGAL_CONSTRUCTOR', 'Illegal constructor');
E('ERR_CONSTRUCT_CALL_REQUIRED', 'Class constructor must be called with new');

// Async errors
E('ERR_INVALID_ASYNC_ID', 'Invalid async ID');
E('ERR_ASYNC_CALLBACK', 'Callback must be a function');
E('ERR_ASYNC_TYPE', 'Invalid async resource type');

// State errors
E('ERR_INVALID_STATE', 'Invalid state');
E('ERR_CLOSED_MESSAGE_PORT', 'Cannot send data on closed MessagePort');

// Parse errors
E('ERR_PARSE_ARGS_INVALID_OPTION_VALUE', 'Invalid option value');
E('ERR_PARSE_ARGS_UNKNOWN_OPTION', function(option) {
  return `Unknown option '${option}'`;
});

E('ERR_PARSE_ARGS_UNEXPECTED_POSITIONAL', 'Unexpected positional argument');

// Generic errors
E('ERR_INVALID_THIS', 'Value of "this" is invalid');
E('ERR_INVALID_RETURN_VALUE', 'Invalid return value');
E('ERR_INVALID_RETURN_PROPERTY', 'Invalid return property');
E('ERR_INVALID_RETURN_PROPERTY_VALUE', 'Invalid return property value');
E('ERR_FALSY_VALUE_REJECTION', 'Promise was rejected with falsy value');
E('ERR_INVALID_PROTOCOL', 'Invalid protocol');
E('ERR_MULTIPLE_CALLBACK', 'Callback was already called');
E('ERR_INCOMPATIBLE_OPTION_PAIR', 'Incompatible option pair');
E('ERR_MISSING_OPTION', function(option) {
  return `Missing required option '${option}'`;
});

E('ERR_OPERATION_FAILED', 'Operation failed');
E('ERR_FEATURE_UNAVAILABLE_ON_PLATFORM', 'Feature is unavailable on this platform');
E('ERR_UNKNOWN_SIGNAL', function(signal) {
  return `Unknown signal: ${signal}`;
});

E('ERR_INTERNAL_ASSERTION', 'Internal assertion failed');
E('ERR_NOT_SUPPORTED_IN_SNAPSHOT', 'Operation is not supported during snapshot creation');

// HTTP2 errors
E('ERR_HTTP2_INVALID_SESSION', 'The session has been destroyed');
E('ERR_HTTP2_INVALID_STREAM', 'Invalid HTTP/2 stream');
E('ERR_HTTP2_HEADERS_SENT', 'Response has already been initiated');
E('ERR_HTTP2_INVALID_HEADER_VALUE', 'Invalid HTTP/2 header value');
E('ERR_HTTP2_INVALID_SETTING_VALUE', 'Invalid HTTP/2 setting value');
E('ERR_HTTP2_STREAM_CANCEL', 'Stream cancelled');

// TLS errors
E('ERR_TLS_CERT_ALTNAME_INVALID', 'Certificate altname does not match hostname');
E('ERR_TLS_HANDSHAKE_TIMEOUT', 'TLS handshake timeout');
E('ERR_TLS_INVALID_PROTOCOL_VERSION', 'Invalid TLS protocol version');
E('ERR_TLS_INVALID_STATE', 'TLS socket is not connected');
E('ERR_TLS_REQUIRED_SERVER_NAME', 'Server name is required');

// Zlib errors
E('ERR_ZLIB_INITIALIZATION_FAILED', 'Initialization failed');

// Console errors
E('ERR_CONSOLE_WRITABLE_STREAM', 'Console expects a writable stream instance');

// Inspector errors
E('ERR_INSPECTOR_NOT_AVAILABLE', 'Inspector is not available');
E('ERR_INSPECTOR_NOT_CONNECTED', 'Inspector is not connected');
E('ERR_INSPECTOR_ALREADY_CONNECTED', 'Inspector is already connected');
E('ERR_INSPECTOR_CLOSED', 'Inspector session is closed');

// Abort error (special case)
codes.ABORT_ERR = AbortError;

// Generic node error
codes.ERR_GENERIC_NODE_ERROR = NodeError;

// Helper function to hide implementation details
function hideStackFrames(fn) {
  const hidden = function(...args) {
    try {
      return fn.apply(this, args);
    } catch (err) {
      Error.captureStackTrace(err, hidden);
      throw err;
    }
  };
  return hidden;
}

// Exception helper
function genericNodeError(message, options = {}) {
  const err = new Error(message);
  err.code = options.code || 'ERR_GENERIC_NODE_ERROR';

  if (options.name) err.name = options.name;
  if (options.cause) err.cause = options.cause;

  Object.assign(err, options);
  return err;
}

module.exports = {
  codes,
  NodeError,
  NodeTypeError,
  NodeRangeError,
  NodeURIError,
  NodeSyntaxError,
  AbortError,
  SystemError,
  hideStackFrames,
  genericNodeError,
};
