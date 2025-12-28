/**
 * Node.js compatible events module
 * Full implementation of EventEmitter and related APIs
 */
(function () {
  "use strict";

  // Symbols for internal state
  const kCapture = Symbol("kCapture");
  const kErrorMonitor = Symbol.for("nodejs.rejection");
  const kRejection = Symbol.for("nodejs.rejection");

  // Default max listeners
  let defaultMaxListeners = 10;

  /**
   * EventEmitter class - the core of Node.js event system
   */
  class EventEmitter {
    constructor(options = {}) {
      this._events = Object.create(null);
      this._eventsCount = 0;
      this._maxListeners = undefined;
      this[kCapture] = options.captureRejections === true;
    }

    /**
     * Adds a listener to the end of the listeners array for the specified event
     */
    addListener(eventName, listener) {
      return this.on(eventName, listener);
    }

    /**
     * Adds a listener to the end of the listeners array for the specified event
     */
    on(eventName, listener) {
      if (typeof listener !== "function") {
        throw new TypeError('The "listener" argument must be of type Function');
      }

      // Initialize _events if not present (for mixin support)
      if (this._events === undefined) {
        this._events = Object.create(null);
        this._eventsCount = 0;
      }
      if (this._maxListeners === undefined) {
        this._maxListeners = EventEmitter.defaultMaxListeners;
      }

      // Emit newListener before adding
      if (this._events.newListener !== undefined) {
        this.emit("newListener", eventName, listener);
      }

      let existing = this._events[eventName];
      if (existing === undefined) {
        this._events[eventName] = listener;
        this._eventsCount++;
      } else if (typeof existing === "function") {
        this._events[eventName] = [existing, listener];
      } else {
        existing.push(listener);
      }

      // Check for listener leak
      const maxListeners = this.getMaxListeners();
      if (maxListeners > 0) {
        const count = this.listenerCount(eventName);
        if (count > maxListeners && !existing?.warned) {
          const warning = new Error(
            `MaxListenersExceededWarning: Possible EventEmitter memory leak detected. ` +
              `${count} ${String(eventName)} listeners added. ` +
              `Use emitter.setMaxListeners() to increase limit`,
          );
          warning.name = "MaxListenersExceededWarning";
          warning.emitter = this;
          warning.type = eventName;
          warning.count = count;
          console.warn(warning.message);
          if (existing) existing.warned = true;
        }
      }

      return this;
    }

    /**
     * Adds a one-time listener for the specified event
     */
    once(eventName, listener) {
      if (typeof listener !== "function") {
        throw new TypeError('The "listener" argument must be of type Function');
      }

      const wrapper = (...args) => {
        this.removeListener(eventName, wrapper);
        listener.apply(this, args);
      };
      wrapper.listener = listener;

      return this.on(eventName, wrapper);
    }

    /**
     * Adds a listener to the beginning of the listeners array
     */
    prependListener(eventName, listener) {
      if (typeof listener !== "function") {
        throw new TypeError('The "listener" argument must be of type Function');
      }

      // Emit newListener before adding
      if (this._events.newListener !== undefined) {
        this.emit("newListener", eventName, listener);
      }

      let existing = this._events[eventName];
      if (existing === undefined) {
        this._events[eventName] = listener;
        this._eventsCount++;
      } else if (typeof existing === "function") {
        this._events[eventName] = [listener, existing];
      } else {
        existing.unshift(listener);
      }

      return this;
    }

    /**
     * Adds a one-time listener to the beginning of the listeners array
     */
    prependOnceListener(eventName, listener) {
      if (typeof listener !== "function") {
        throw new TypeError('The "listener" argument must be of type Function');
      }

      const wrapper = (...args) => {
        this.removeListener(eventName, wrapper);
        listener.apply(this, args);
      };
      wrapper.listener = listener;

      return this.prependListener(eventName, wrapper);
    }

    /**
     * Removes a listener from the listener array for the specified event
     */
    removeListener(eventName, listener) {
      if (typeof listener !== "function") {
        throw new TypeError('The "listener" argument must be of type Function');
      }

      const events = this._events[eventName];
      if (events === undefined) {
        return this;
      }

      if (events === listener || events.listener === listener) {
        if (--this._eventsCount === 0) {
          this._events = Object.create(null);
        } else {
          delete this._events[eventName];
        }
        if (this._events.removeListener) {
          this.emit("removeListener", eventName, listener);
        }
      } else if (typeof events !== "function") {
        let position = -1;
        for (let i = events.length - 1; i >= 0; i--) {
          if (events[i] === listener || events[i].listener === listener) {
            position = i;
            break;
          }
        }

        if (position < 0) {
          return this;
        }

        if (position === 0) {
          events.shift();
        } else {
          events.splice(position, 1);
        }

        if (events.length === 1) {
          this._events[eventName] = events[0];
        }

        if (this._events.removeListener) {
          this.emit("removeListener", eventName, listener);
        }
      }

      return this;
    }

    /**
     * Alias for removeListener
     */
    off(eventName, listener) {
      return this.removeListener(eventName, listener);
    }

    /**
     * Removes all listeners, or those of the specified event
     */
    removeAllListeners(eventName) {
      if (eventName === undefined) {
        // Remove all listeners for all events
        const keys = Object.keys(this._events);
        for (const key of keys) {
          if (key !== "removeListener") {
            this.removeAllListeners(key);
          }
        }
        this.removeAllListeners("removeListener");
        this._events = Object.create(null);
        this._eventsCount = 0;
        return this;
      }

      const listeners = this._events[eventName];
      if (listeners === undefined) {
        return this;
      }

      if (typeof listeners === "function") {
        this.removeListener(eventName, listeners);
      } else {
        // Iterate backwards to avoid issues with splice
        for (let i = listeners.length - 1; i >= 0; i--) {
          this.removeListener(eventName, listeners[i]);
        }
      }

      return this;
    }

    /**
     * Synchronously calls each listener registered for the event
     */
    emit(eventName, ...args) {
      let doError = eventName === "error";

      const events = this._events;
      if (events !== undefined) {
        if (doError && events[kErrorMonitor] !== undefined) {
          this.emit(kErrorMonitor, ...args);
        }
        doError = doError && events.error === undefined;
      } else if (!doError) {
        return false;
      }

      // If no error listeners and error event, throw
      if (doError) {
        let er = args[0];
        if (er instanceof Error) {
          throw er;
        }
        const err = new Error("Unhandled error." + (er ? ` (${er})` : ""));
        err.context = er;
        throw err;
      }

      const handler = events[eventName];
      if (handler === undefined) {
        return false;
      }

      if (typeof handler === "function") {
        try {
          const result = handler.apply(this, args);
          // Handle async functions with captureRejections
          if (
            this[kCapture] &&
            result !== undefined &&
            typeof result.then === "function"
          ) {
            result.catch((err) => this._handleRejection(err, eventName, args));
          }
        } catch (err) {
          if (eventName !== "error") {
            this.emit("error", err);
          } else {
            throw err;
          }
        }
      } else {
        const listeners = handler.slice();
        for (let i = 0; i < listeners.length; i++) {
          try {
            const result = listeners[i].apply(this, args);
            // Handle async functions with captureRejections
            if (
              this[kCapture] &&
              result !== undefined &&
              typeof result.then === "function"
            ) {
              result.catch((err) =>
                this._handleRejection(err, eventName, args),
              );
            }
          } catch (err) {
            if (eventName !== "error") {
              this.emit("error", err);
            } else {
              throw err;
            }
          }
        }
      }

      return true;
    }

    /**
     * Handle promise rejections when captureRejections is enabled
     */
    _handleRejection(err, eventName, args) {
      if (typeof this[kRejection] === "function") {
        this[kRejection](err, eventName, ...args);
      } else {
        // Temporarily disable captureRejections to avoid infinite loop
        const capture = this[kCapture];
        this[kCapture] = false;
        this.emit("error", err);
        this[kCapture] = capture;
      }
    }

    /**
     * Returns an array listing the events for which the emitter has registered listeners
     */
    eventNames() {
      return Object.keys(this._events).concat(
        Object.getOwnPropertySymbols(this._events),
      );
    }

    /**
     * Returns the current max listener value for the EventEmitter
     */
    getMaxListeners() {
      return this._maxListeners === undefined
        ? defaultMaxListeners
        : this._maxListeners;
    }

    /**
     * Sets the max listeners for this emitter
     */
    setMaxListeners(n) {
      if (typeof n !== "number" || n < 0 || Number.isNaN(n)) {
        throw new RangeError(
          'The value of "n" is out of range. It must be a non-negative number.',
        );
      }
      this._maxListeners = n;
      return this;
    }

    /**
     * Returns the number of listeners for the event
     */
    listenerCount(eventName, listener) {
      const events = this._events[eventName];
      if (events === undefined) {
        return 0;
      }

      if (typeof events === "function") {
        if (listener !== undefined) {
          return events === listener || events.listener === listener ? 1 : 0;
        }
        return 1;
      }

      if (listener !== undefined) {
        let count = 0;
        for (const ev of events) {
          if (ev === listener || ev.listener === listener) {
            count++;
          }
        }
        return count;
      }

      return events.length;
    }

    /**
     * Returns a copy of the array of listeners for the event
     */
    listeners(eventName) {
      const events = this._events[eventName];
      if (events === undefined) {
        return [];
      }

      if (typeof events === "function") {
        return [events.listener || events];
      }

      return events.map((ev) => ev.listener || ev);
    }

    /**
     * Returns a copy of the array of listeners including wrappers
     */
    rawListeners(eventName) {
      const events = this._events[eventName];
      if (events === undefined) {
        return [];
      }

      if (typeof events === "function") {
        return [events];
      }

      return events.slice();
    }

    /**
     * Static method to get listener count (deprecated)
     */
    static listenerCount(emitter, eventName) {
      return emitter.listenerCount(eventName);
    }
  }

  // Static property for default max listeners
  Object.defineProperty(EventEmitter, "defaultMaxListeners", {
    get() {
      return defaultMaxListeners;
    },
    set(value) {
      if (typeof value !== "number" || value < 0 || Number.isNaN(value)) {
        throw new RangeError(
          'The value of "defaultMaxListeners" is out of range.',
        );
      }
      defaultMaxListeners = value;
    },
    enumerable: true,
    configurable: true,
  });

  /**
   * Creates a Promise that is fulfilled when the EventEmitter emits the given event
   */
  function once(emitter, eventName, options = {}) {
    return new Promise((resolve, reject) => {
      const signal = options.signal;

      if (signal !== undefined && signal.aborted) {
        reject(new Error("AbortError"));
        return;
      }

      const eventHandler = (...args) => {
        if (signal !== undefined) {
          signal.removeEventListener("abort", abortHandler);
        }
        resolve(args);
      };

      const errorHandler = (err) => {
        if (signal !== undefined) {
          signal.removeEventListener("abort", abortHandler);
        }
        emitter.removeListener(eventName, eventHandler);
        reject(err);
      };

      const abortHandler = () => {
        emitter.removeListener(eventName, eventHandler);
        emitter.removeListener("error", errorHandler);
        reject(new Error("AbortError"));
      };

      if (signal !== undefined) {
        signal.addEventListener("abort", abortHandler, { once: true });
      }

      emitter.once(eventName, eventHandler);

      if (eventName !== "error") {
        emitter.once("error", errorHandler);
      }
    });
  }

  /**
   * Returns an AsyncIterator that iterates eventName events
   */
  function on(emitter, eventName, options = {}) {
    const signal = options.signal;
    const closeEvents = options.close || [];
    const highWaterMark = options.highWaterMark || Number.MAX_SAFE_INTEGER;
    const lowWaterMark = options.lowWaterMark || 1;

    const unconsumedEvents = [];
    const unconsumedPromises = [];
    let error = null;
    let finished = false;
    let paused = false;

    const eventHandler = (...args) => {
      const promise = unconsumedPromises.shift();
      if (promise) {
        promise.resolve({ value: args, done: false });
      } else {
        unconsumedEvents.push(args);
        if (unconsumedEvents.length >= highWaterMark && !paused) {
          paused = true;
          if (typeof emitter.pause === "function") {
            emitter.pause();
          }
        }
      }
    };

    const errorHandler = (err) => {
      error = err;
      const promise = unconsumedPromises.shift();
      if (promise) {
        promise.reject(err);
      }
    };

    const closeHandler = () => {
      finished = true;
      const promise = unconsumedPromises.shift();
      if (promise) {
        promise.resolve({ value: undefined, done: true });
      }
    };

    const abortHandler = () => {
      errorHandler(new Error("AbortError"));
      closeHandler();
    };

    emitter.on(eventName, eventHandler);
    emitter.on("error", errorHandler);
    for (const event of closeEvents) {
      emitter.on(event, closeHandler);
    }
    if (signal) {
      signal.addEventListener("abort", abortHandler);
    }

    const iterator = {
      async next() {
        const event = unconsumedEvents.shift();
        if (event) {
          if (unconsumedEvents.length < lowWaterMark && paused) {
            paused = false;
            if (typeof emitter.resume === "function") {
              emitter.resume();
            }
          }
          return { value: event, done: false };
        }

        if (error) {
          const err = error;
          error = null;
          throw err;
        }

        if (finished) {
          return { value: undefined, done: true };
        }

        return new Promise((resolve, reject) => {
          unconsumedPromises.push({ resolve, reject });
        });
      },

      async return() {
        emitter.removeListener(eventName, eventHandler);
        emitter.removeListener("error", errorHandler);
        for (const event of closeEvents) {
          emitter.removeListener(event, closeHandler);
        }
        if (signal) {
          signal.removeEventListener("abort", abortHandler);
        }
        finished = true;
        for (const promise of unconsumedPromises) {
          promise.resolve({ value: undefined, done: true });
        }
        return { value: undefined, done: true };
      },

      async throw(err) {
        error = err;
        return this.return();
      },

      [Symbol.asyncIterator]() {
        return this;
      },
    };

    return iterator;
  }

  /**
   * Returns a copy of the array of listeners for the event
   */
  function getEventListeners(emitterOrTarget, eventName) {
    if (typeof emitterOrTarget.listeners === "function") {
      return emitterOrTarget.listeners(eventName);
    }
    // EventTarget - not fully supported
    return [];
  }

  /**
   * Returns the current max amount of listeners
   */
  function getMaxListeners(emitterOrTarget) {
    if (typeof emitterOrTarget.getMaxListeners === "function") {
      return emitterOrTarget.getMaxListeners();
    }
    return defaultMaxListeners;
  }

  /**
   * Sets the max listeners for the given emitters
   */
  function setMaxListeners(n, ...eventTargets) {
    if (typeof n !== "number" || n < 0 || Number.isNaN(n)) {
      throw new RangeError('The value of "n" is out of range.');
    }

    if (eventTargets.length === 0) {
      defaultMaxListeners = n;
      return;
    }

    for (const target of eventTargets) {
      if (typeof target.setMaxListeners === "function") {
        target.setMaxListeners(n);
      }
    }
  }

  /**
   * Static listenerCount (deprecated)
   */
  function listenerCount(emitter, eventName) {
    return emitter.listenerCount(eventName);
  }

  /**
   * Adds an abort listener that handles stopImmediatePropagation
   */
  function addAbortListener(signal, listener) {
    if (signal.aborted) {
      queueMicrotask(() => listener());
      return {
        [Symbol.dispose]() {},
      };
    }

    const handler = () => listener();
    signal.addEventListener("abort", handler);

    return {
      [Symbol.dispose]() {
        signal.removeEventListener("abort", handler);
      },
    };
  }

  // Build the events module object
  const events = {
    EventEmitter,
    once,
    on,
    getEventListeners,
    getMaxListeners,
    setMaxListeners,
    listenerCount,
    addAbortListener,
    errorMonitor: kErrorMonitor,
    captureRejectionSymbol: kRejection,
    captureRejections: false,
  };

  // Make defaultMaxListeners a getter/setter on the module
  Object.defineProperty(events, "defaultMaxListeners", {
    get() {
      return defaultMaxListeners;
    },
    set(value) {
      EventEmitter.defaultMaxListeners = value;
    },
    enumerable: true,
    configurable: true,
  });

  // Set default export
  events.default = EventEmitter;

  // Export to global
  globalThis.events = events;
  globalThis.EventEmitter = EventEmitter;
})();
