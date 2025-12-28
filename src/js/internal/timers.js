// Reimplementation of Node.js timers module
// Reference: https://github.com/nodejs/node/blob/main/lib/timers.js

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
} = require('./errors');

const {
  validateFunction,
  validateNumber,
} = require('./validators');

// Store for tracking active timers
const activeTimers = new Map();
let timerId = 1;

// Normalize delay to valid range
function normalizeDelay(delay) {
  delay = Number(delay);
  if (!(delay >= 1 && delay <= 2147483647)) {
    delay = 1;
  }
  return Math.trunc(delay);
}

// Timeout class
class Timeout {
  constructor(callback, delay, args, repeat) {
    this._id = timerId++;
    this._callback = callback;
    this._delay = delay;
    this._args = args;
    this._repeat = repeat;
    this._destroyed = false;
    this._ref = true;
    this._startTime = Date.now();

    // Schedule the actual timer
    if (repeat) {
      this._handle = setInterval(() => this._onTimeout(), delay);
    } else {
      this._handle = setTimeout(() => this._onTimeout(), delay);
    }

    activeTimers.set(this._id, this);
  }

  _onTimeout() {
    if (this._destroyed) return;

    try {
      if (this._args && this._args.length > 0) {
        this._callback.apply(null, this._args);
      } else {
        this._callback();
      }
    } catch (err) {
      // In Node.js, timer errors are handled by the domain or uncaughtException
      if (typeof process !== 'undefined' && process.emit) {
        process.nextTick(() => {
          throw err;
        });
      } else {
        throw err;
      }
    }

    // Clean up if not repeating
    if (!this._repeat) {
      this.close();
    }
  }

  hasRef() {
    return this._ref;
  }

  ref() {
    this._ref = true;
    return this;
  }

  unref() {
    this._ref = false;
    return this;
  }

  refresh() {
    if (this._destroyed) return this;

    // Cancel current timer
    if (this._repeat) {
      clearInterval(this._handle);
    } else {
      clearTimeout(this._handle);
    }

    // Restart with same delay
    this._startTime = Date.now();
    if (this._repeat) {
      this._handle = setInterval(() => this._onTimeout(), this._delay);
    } else {
      this._handle = setTimeout(() => this._onTimeout(), this._delay);
    }

    return this;
  }

  close() {
    if (this._destroyed) return;

    this._destroyed = true;
    activeTimers.delete(this._id);

    if (this._repeat) {
      clearInterval(this._handle);
    } else {
      clearTimeout(this._handle);
    }

    this._handle = null;
  }

  [Symbol.toPrimitive]() {
    return this._id;
  }

  [Symbol.dispose]() {
    this.close();
  }
}

// Immediate class
class Immediate {
  constructor(callback, args) {
    this._id = timerId++;
    this._callback = callback;
    this._args = args;
    this._destroyed = false;
    this._ref = true;

    // Schedule immediate using setImmediate or setTimeout(0)
    if (typeof globalThis.setImmediate === 'function') {
      this._handle = globalThis.setImmediate(() => this._onImmediate());
    } else {
      this._handle = setTimeout(() => this._onImmediate(), 0);
    }

    activeTimers.set(this._id, this);
  }

  _onImmediate() {
    if (this._destroyed) return;

    try {
      if (this._args && this._args.length > 0) {
        this._callback.apply(null, this._args);
      } else {
        this._callback();
      }
    } catch (err) {
      if (typeof process !== 'undefined' && process.emit) {
        process.nextTick(() => {
          throw err;
        });
      } else {
        throw err;
      }
    }

    this.close();
  }

  hasRef() {
    return this._ref;
  }

  ref() {
    this._ref = true;
    return this;
  }

  unref() {
    this._ref = false;
    return this;
  }

  close() {
    if (this._destroyed) return;

    this._destroyed = true;
    activeTimers.delete(this._id);

    if (typeof globalThis.clearImmediate === 'function') {
      globalThis.clearImmediate(this._handle);
    } else {
      clearTimeout(this._handle);
    }

    this._handle = null;
  }

  [Symbol.dispose]() {
    this.close();
  }
}

// Public API - scheduling timers
function setTimeout(callback, delay, ...args) {
  validateFunction(callback, 'callback');

  delay = normalizeDelay(delay);

  return new Timeout(callback, delay, args, false);
}

function setInterval(callback, delay, ...args) {
  validateFunction(callback, 'callback');

  delay = normalizeDelay(delay);

  return new Timeout(callback, delay, args, true);
}

function setImmediate(callback, ...args) {
  validateFunction(callback, 'callback');

  return new Immediate(callback, args);
}

// Public API - cancelling timers
function clearTimeout(timeout) {
  if (timeout && typeof timeout === 'object' && timeout instanceof Timeout) {
    timeout.close();
  } else if (typeof timeout === 'number' || typeof timeout === 'string') {
    const timer = activeTimers.get(Number(timeout));
    if (timer) {
      timer.close();
    }
  }
}

function clearInterval(timeout) {
  clearTimeout(timeout);
}

function clearImmediate(immediate) {
  if (immediate && typeof immediate === 'object' && immediate instanceof Immediate) {
    immediate.close();
  } else if (typeof immediate === 'number' || typeof immediate === 'string') {
    const timer = activeTimers.get(Number(immediate));
    if (timer) {
      timer.close();
    }
  }
}

module.exports = {
  setTimeout,
  setInterval,
  setImmediate,
  clearTimeout,
  clearInterval,
  clearImmediate,
  Timeout,
  Immediate,
  // For internal use
  _activeTimers: activeTimers,
};
