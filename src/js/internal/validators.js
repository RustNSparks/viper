// Internal validators for Node.js API argument validation
// Reference: https://github.com/nodejs/node/blob/main/lib/internal/validators.js

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

const {
  codes: {
    ERR_INVALID_ARG_TYPE,
    ERR_INVALID_ARG_VALUE,
    ERR_OUT_OF_RANGE,
    ERR_MISSING_ARGS,
    ERR_INVALID_URL,
    ERR_INVALID_URL_SCHEME,
  }
} = require('./errors');

function isInt32(value) {
  return value === (value | 0);
}

function isUint32(value) {
  return value === (value >>> 0);
}

function validateString(value, name) {
  if (typeof value !== 'string') {
    throw new ERR_INVALID_ARG_TYPE(name, 'string', value);
  }
}

function validateNumber(value, name, min = undefined, max = undefined) {
  if (typeof value !== 'number') {
    throw new ERR_INVALID_ARG_TYPE(name, 'number', value);
  }

  if (min !== undefined && value < min ||
      max !== undefined && value > max ||
      Number.isNaN(value)) {
    throw new ERR_OUT_OF_RANGE(
      name,
      `${min !== undefined ? `>= ${min}` : ''}${min !== undefined && max !== undefined ? ' && ' : ''}${max !== undefined ? `<= ${max}` : ''}`,
      value
    );
  }
}

function validateInteger(value, name, min = Number.MIN_SAFE_INTEGER, max = Number.MAX_SAFE_INTEGER) {
  if (typeof value !== 'number') {
    throw new ERR_INVALID_ARG_TYPE(name, 'integer', value);
  }

  if (!Number.isInteger(value)) {
    throw new ERR_OUT_OF_RANGE(name, 'an integer', value);
  }

  if (value < min || value > max) {
    throw new ERR_OUT_OF_RANGE(name, `>= ${min} && <= ${max}`, value);
  }
}

function validateInt32(value, name, min = -2147483648, max = 2147483647) {
  if (typeof value !== 'number') {
    throw new ERR_INVALID_ARG_TYPE(name, 'number', value);
  }

  if (!Number.isInteger(value)) {
    throw new ERR_OUT_OF_RANGE(name, 'an integer', value);
  }

  if (value < min || value > max) {
    throw new ERR_OUT_OF_RANGE(name, `>= ${min} && <= ${max}`, value);
  }
}

function validateUint32(value, name, positive = false) {
  if (typeof value !== 'number') {
    throw new ERR_INVALID_ARG_TYPE(name, 'number', value);
  }

  if (!Number.isInteger(value)) {
    throw new ERR_OUT_OF_RANGE(name, 'an integer', value);
  }

  const min = positive ? 1 : 0;
  const max = 4294967295;

  if (value < min || value > max) {
    throw new ERR_OUT_OF_RANGE(name, `>= ${min} && <= ${max}`, value);
  }
}

function validateBoolean(value, name) {
  if (typeof value !== 'boolean') {
    throw new ERR_INVALID_ARG_TYPE(name, 'boolean', value);
  }
}

function validateObject(value, name, options = {}) {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    throw new ERR_INVALID_ARG_TYPE(name, 'object', value);
  }

  if (options.nullable && value === null) {
    return;
  }

  if (options.allowArray && Array.isArray(value)) {
    return;
  }

  if (options.allowFunction && typeof value === 'function') {
    return;
  }
}

function validateArray(value, name, options = {}) {
  if (!Array.isArray(value)) {
    throw new ERR_INVALID_ARG_TYPE(name, 'Array', value);
  }

  if (options.minLength !== undefined && value.length < options.minLength) {
    throw new ERR_INVALID_ARG_VALUE(
      name,
      value,
      `must have at least ${options.minLength} elements`
    );
  }
}

function validateFunction(value, name) {
  if (typeof value !== 'function') {
    throw new ERR_INVALID_ARG_TYPE(name, 'function', value);
  }
}

function validateBuffer(value, name) {
  if (!Buffer.isBuffer(value)) {
    throw new ERR_INVALID_ARG_TYPE(name, 'Buffer', value);
  }
}

function validateEncoding(value, name) {
  const normalizedEncoding = String(value).toLowerCase();
  const validEncodings = [
    'utf8', 'utf-8',
    'hex',
    'base64', 'base64url',
    'latin1', 'binary',
    'ascii',
    'ucs2', 'ucs-2', 'utf16le', 'utf-16le'
  ];

  if (!validEncodings.includes(normalizedEncoding)) {
    throw new ERR_INVALID_ARG_VALUE(name, value, 'must be a valid encoding');
  }
}

function validatePort(value, name = 'port', allowZero = true) {
  if (typeof value !== 'number' && typeof value !== 'string') {
    throw new ERR_INVALID_ARG_TYPE(name, ['number', 'string'], value);
  }

  const port = Number(value);

  if (!Number.isInteger(port)) {
    throw new ERR_INVALID_ARG_VALUE(name, value, 'must be a valid port number');
  }

  if (port < (allowZero ? 0 : 1) || port > 65535) {
    throw new ERR_OUT_OF_RANGE(
      name,
      allowZero ? '>= 0 && < 65536' : '>= 1 && < 65536',
      value
    );
  }

  return port;
}

function validateAbortSignal(signal, name) {
  if (signal !== undefined &&
      (signal === null ||
       typeof signal !== 'object' ||
       !('aborted' in signal))) {
    throw new ERR_INVALID_ARG_TYPE(name, 'AbortSignal', signal);
  }
}

function validateSignalName(signal, name = 'signal') {
  validateString(signal, name);

  const signals = [
    'SIGABRT', 'SIGALRM', 'SIGBUS', 'SIGCHLD', 'SIGCONT', 'SIGFPE',
    'SIGHUP', 'SIGILL', 'SIGINT', 'SIGIO', 'SIGIOT', 'SIGKILL',
    'SIGPIPE', 'SIGPOLL', 'SIGPROF', 'SIGPWR', 'SIGQUIT', 'SIGSEGV',
    'SIGSTKFLT', 'SIGSTOP', 'SIGSYS', 'SIGTERM', 'SIGTRAP', 'SIGTSTP',
    'SIGTTIN', 'SIGTTOU', 'SIGUNUSED', 'SIGURG', 'SIGUSR1', 'SIGUSR2',
    'SIGVTALRM', 'SIGWINCH', 'SIGXCPU', 'SIGXFSZ'
  ];

  if (!signals.includes(signal)) {
    throw new ERR_INVALID_ARG_VALUE(name, signal, 'must be a valid signal name');
  }
}

function validateOneOf(value, name, oneOf) {
  if (!Array.isArray(oneOf)) {
    throw new ERR_INVALID_ARG_TYPE('oneOf', 'Array', oneOf);
  }

  if (!oneOf.includes(value)) {
    const allowed = oneOf
      .map((v) => (typeof v === 'string' ? `'${v}'` : String(v)))
      .join(', ');
    throw new ERR_INVALID_ARG_VALUE(
      name,
      value,
      `must be one of: ${allowed}`
    );
  }
}

function validatePlainFunction(value, name) {
  if (typeof value !== 'function' || value.constructor.name !== 'Function') {
    throw new ERR_INVALID_ARG_TYPE(name, 'Function', value);
  }
}

function validateUndefined(value, name) {
  if (value !== undefined) {
    throw new ERR_INVALID_ARG_TYPE(name, 'undefined', value);
  }
}

function validateUnion(value, name, union) {
  if (!Array.isArray(union)) {
    throw new ERR_INVALID_ARG_TYPE('union', 'Array', union);
  }

  const types = union.map((type) => {
    switch (type) {
      case 'string':
      case 'number':
      case 'boolean':
      case 'bigint':
      case 'symbol':
      case 'undefined':
        return typeof value === type;
      case 'object':
        return value !== null && typeof value === 'object';
      case 'null':
        return value === null;
      case 'array':
        return Array.isArray(value);
      case 'function':
        return typeof value === 'function';
      default:
        return false;
    }
  });

  if (!types.some(Boolean)) {
    const allowed = union.join(' or ');
    throw new ERR_INVALID_ARG_TYPE(name, allowed, value);
  }
}

function validateLinkHeaderFormat(hints) {
  if (typeof hints !== 'string' || hints.length === 0) {
    throw new ERR_INVALID_ARG_VALUE(
      'hints',
      hints,
      'must be a non-empty string'
    );
  }
}

// URL validation
function isURL(value) {
  return typeof value === 'object' && value !== null &&
    value.href !== undefined &&
    value.origin !== undefined &&
    value.protocol !== undefined &&
    value.pathname !== undefined;
}

function validateURL(value, name) {
  if (!isURL(value)) {
    throw new ERR_INVALID_URL();
  }
}

function validateURLScheme(url, name, schemes) {
  if (!isURL(url)) {
    throw new ERR_INVALID_URL();
  }

  const protocol = url.protocol.slice(0, -1); // Remove trailing ':'

  if (!schemes.includes(protocol)) {
    throw new ERR_INVALID_URL_SCHEME(schemes);
  }
}

module.exports = {
  isInt32,
  isUint32,
  isURL,
  validateString,
  validateNumber,
  validateInteger,
  validateInt32,
  validateUint32,
  validateBoolean,
  validateObject,
  validateArray,
  validateFunction,
  validateBuffer,
  validateEncoding,
  validatePort,
  validateAbortSignal,
  validateSignalName,
  validateOneOf,
  validatePlainFunction,
  validateUndefined,
  validateUnion,
  validateLinkHeaderFormat,
  validateURL,
  validateURLScheme,
};
