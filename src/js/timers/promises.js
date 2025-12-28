// Reimplementation of Node.js timers/promises module
// Reference: https://github.com/nodejs/node/blob/main/lib/timers/promises.js

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
  codes: { ERR_INVALID_ARG_TYPE }
} = require('../internal/errors');

const {
  validateAbortSignal,
  validateBoolean,
  validateInteger,
  validateObject,
} = require('../internal/validators');

const {
  setTimeout: setTimeoutCallback,
  setImmediate: setImmediateCallback,
  setInterval: setIntervalCallback,
  clearTimeout,
  clearInterval,
  clearImmediate,
} = require('../internal/timers');

// Normalize delay
function normalizeDelay(delay) {
  delay = Number(delay);
  if (!(delay >= 1 && delay <= 2147483647)) {
    delay = 1;
  }
  return Math.trunc(delay);
}

// Promise-based setTimeout
function setTimeout(delay, value, options = {}) {
  if (delay === undefined) {
    delay = 1;
  } else {
    delay = normalizeDelay(delay);
  }

  if (options !== null && typeof options === 'object') {
    validateObject(options, 'options');

    if (options.signal !== undefined) {
      validateAbortSignal(options.signal, 'options.signal');
    }

    if (options.ref !== undefined) {
      validateBoolean(options.ref, 'options.ref');
    }
  }

  return new Promise((resolve, reject) => {
    const signal = options?.signal;

    if (signal?.aborted) {
      const err = new Error('The operation was aborted');
      err.code = 'ABORT_ERR';
      err.name = 'AbortError';
      err.cause = signal.reason;
      reject(err);
      return;
    }

    const timeout = setTimeoutCallback(() => {
      if (signal) {
        signal.removeEventListener('abort', onAbort);
      }
      resolve(value);
    }, delay);

    if (options?.ref === false) {
      timeout.unref();
    }

    function onAbort() {
      clearTimeout(timeout);
      const err = new Error('The operation was aborted');
      err.code = 'ABORT_ERR';
      err.name = 'AbortError';
      err.cause = signal.reason;
      reject(err);
    }

    if (signal) {
      signal.addEventListener('abort', onAbort, { once: true });
    }
  });
}

// Promise-based setImmediate
function setImmediate(value, options = {}) {
  if (options !== null && typeof options === 'object') {
    validateObject(options, 'options');

    if (options.signal !== undefined) {
      validateAbortSignal(options.signal, 'options.signal');
    }

    if (options.ref !== undefined) {
      validateBoolean(options.ref, 'options.ref');
    }
  }

  return new Promise((resolve, reject) => {
    const signal = options?.signal;

    if (signal?.aborted) {
      const err = new Error('The operation was aborted');
      err.code = 'ABORT_ERR';
      err.name = 'AbortError';
      err.cause = signal.reason;
      reject(err);
      return;
    }

    const immediate = setImmediateCallback(() => {
      if (signal) {
        signal.removeEventListener('abort', onAbort);
      }
      resolve(value);
    });

    if (options?.ref === false) {
      immediate.unref();
    }

    function onAbort() {
      clearImmediate(immediate);
      const err = new Error('The operation was aborted');
      err.code = 'ABORT_ERR';
      err.name = 'AbortError';
      err.cause = signal.reason;
      reject(err);
    }

    if (signal) {
      signal.addEventListener('abort', onAbort, { once: true });
    }
  });
}

// Async iterator for setInterval
async function* setInterval(delay, value, options = {}) {
  if (delay === undefined) {
    delay = 1;
  } else {
    delay = normalizeDelay(delay);
  }

  if (options !== null && typeof options === 'object') {
    validateObject(options, 'options');

    if (options.signal !== undefined) {
      validateAbortSignal(options.signal, 'options.signal');
    }

    if (options.ref !== undefined) {
      validateBoolean(options.ref, 'options.ref');
    }
  }

  const signal = options?.signal;

  if (signal?.aborted) {
    const err = new Error('The operation was aborted');
    err.code = 'ABORT_ERR';
    err.name = 'AbortError';
    err.cause = signal.reason;
    throw err;
  }

  let interval = null;
  let onAbortFn = null;

  try {
    const queue = [];
    let resolveNext = null;
    let done = false;

    interval = setIntervalCallback(() => {
      if (done) return;

      if (resolveNext) {
        const resolve = resolveNext;
        resolveNext = null;
        resolve({ value, done: false });
      } else {
        queue.push(value);
      }
    }, delay);

    if (options?.ref === false) {
      interval.unref();
    }

    onAbortFn = () => {
      done = true;
      if (interval) {
        clearInterval(interval);
        interval = null;
      }
      if (resolveNext) {
        const err = new Error('The operation was aborted');
        err.code = 'ABORT_ERR';
        err.name = 'AbortError';
        err.cause = signal.reason;
        resolveNext({ value: undefined, done: true });
      }
    };

    if (signal) {
      signal.addEventListener('abort', onAbortFn, { once: true });
    }

    while (!done) {
      if (queue.length > 0) {
        yield queue.shift();
      } else {
        await new Promise((resolve) => {
          resolveNext = resolve;
        });

        if (done) break;

        if (resolveNext === null) {
          // Value was already resolved
          continue;
        }
      }

      if (signal?.aborted) {
        done = true;
        break;
      }
    }
  } finally {
    if (interval) {
      clearInterval(interval);
    }
    if (signal && onAbortFn) {
      signal.removeEventListener('abort', onAbortFn);
    }
  }
}

// Scheduler API (experimental)
const scheduler = {
  wait(delay, options) {
    if (delay === undefined) {
      delay = 1;
    } else {
      delay = normalizeDelay(delay);
    }

    if (options !== null && typeof options === 'object') {
      validateObject(options, 'options');

      if (options.signal !== undefined) {
        validateAbortSignal(options.signal, 'options.signal');
      }

      if (options.ref !== undefined) {
        validateBoolean(options.ref, 'options.ref');
      }
    }

    return setTimeout(delay, undefined, options);
  },

  yield() {
    return setImmediate(undefined, {});
  }
};

module.exports = {
  setTimeout,
  setImmediate,
  setInterval,
  scheduler,
};
