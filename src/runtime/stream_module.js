/**
 * Node.js Stream Module - Full Implementation
 * Provides Readable, Writable, Duplex, Transform, and PassThrough streams
 */
(function () {
  "use strict";

  // Use existing EventEmitter if available
  const EventEmitter =
    globalThis.EventEmitter ||
    (function () {
      class EE {
        constructor() {
          this._events = {};
        }
        on(event, fn) {
          if (!this._events[event]) this._events[event] = [];
          this._events[event].push(fn);
          // Auto-resume when 'data' listener is added (flowing mode)
          if (event === "data" && typeof this.resume === "function") {
            this.resume();
          }
          return this;
        }
        once(event, fn) {
          const wrapper = (...args) => {
            this.off(event, wrapper);
            fn.apply(this, args);
          };
          wrapper.listener = fn;
          return this.on(event, wrapper);
        }
        off(event, fn) {
          if (!this._events[event]) return this;
          this._events[event] = this._events[event].filter(
            (f) => f !== fn && f.listener !== fn,
          );
          return this;
        }
        removeListener(event, fn) {
          return this.off(event, fn);
        }
        addListener(event, fn) {
          return this.on(event, fn);
        }
        emit(event, ...args) {
          if (!this._events[event]) return false;
          for (const fn of this._events[event].slice()) {
            try {
              fn.apply(this, args);
            } catch (e) {
              console.error(e);
            }
          }
          return true;
        }
        removeAllListeners(event) {
          if (event) delete this._events[event];
          else this._events = {};
          return this;
        }
        listeners(event) {
          return (this._events[event] || []).map((f) => f.listener || f);
        }
        listenerCount(event) {
          return (this._events[event] || []).length;
        }
        prependListener(event, fn) {
          if (!this._events[event]) this._events[event] = [];
          this._events[event].unshift(fn);
          return this;
        }
        eventNames() {
          return Object.keys(this._events);
        }
      }
      return EE;
    })();

  // Stream states
  const kDestroyed = Symbol("destroyed");
  const kEnded = Symbol("ended");
  const kFinished = Symbol("finished");

  /**
   * Readable Stream
   */
  class Readable extends EventEmitter {
    constructor(options = {}) {
      super();
      this.readable = true;
      this.readableEncoding = options.encoding || null;

      // Override on() to auto-resume when 'data' listener is added
      const originalOn = this.on.bind(this);
      this.on = (event, fn) => {
        originalOn(event, fn);
        if (event === "data") {
          this.resume();
        }
        return this;
      };
      this.addListener = this.on;
      this.readableEnded = false;
      this.readableFlowing = null;
      this.readableHighWaterMark = options.highWaterMark || 16384;
      this.readableLength = 0;
      this.readableObjectMode = options.objectMode || false;
      this[kDestroyed] = false;
      this[kEnded] = false;

      this._buffer = [];
      this._readableState = {
        flowing: null,
        ended: false,
        endEmitted: false,
        reading: false,
        paused: true,
        pipes: [],
        decoder: null,
        encoding: options.encoding || null,
      };

      if (typeof options.read === "function") {
        this._read = options.read;
      }
      if (typeof options.destroy === "function") {
        this._destroy = options.destroy;
      }
    }

    _read(size) {
      // Override in subclass
    }

    read(size) {
      if (this[kDestroyed]) return null;

      if (this._buffer.length === 0) {
        if (this[kEnded]) {
          if (!this._readableState.endEmitted) {
            this._readableState.endEmitted = true;
            this.readableEnded = true;
            this.emit("end");
          }
          return null;
        }
        this._read(size);
        return null;
      }

      let chunk;
      if (size === undefined || size >= this.readableLength) {
        if (this.readableObjectMode) {
          chunk = this._buffer.shift();
          this.readableLength--;
        } else {
          chunk = Buffer.concat(this._buffer);
          this._buffer = [];
          this.readableLength = 0;
        }
      } else {
        if (this.readableObjectMode) {
          chunk = this._buffer.shift();
          this.readableLength--;
        } else {
          const first = this._buffer[0];
          if (size >= first.length) {
            chunk = this._buffer.shift();
            this.readableLength -= chunk.length;
          } else {
            chunk = first.slice(0, size);
            this._buffer[0] = first.slice(size);
            this.readableLength -= size;
          }
        }
      }

      return chunk;
    }

    push(chunk, encoding) {
      if (this[kDestroyed]) return false;

      if (chunk === null) {
        this[kEnded] = true;
        if (this._buffer.length === 0) {
          this._readableState.endEmitted = true;
          this.readableEnded = true;
          queueMicrotask(() => this.emit("end"));
        }
        return false;
      }

      if (typeof chunk === "string") {
        chunk = Buffer.from(chunk, encoding || this.readableEncoding || "utf8");
      }

      this._buffer.push(chunk);
      if (this.readableObjectMode) {
        this.readableLength++;
      } else {
        this.readableLength += chunk.length;
      }

      this.emit("readable");

      // Don't emit data here - _flow() will handle it when in flowing mode

      return this.readableLength < this.readableHighWaterMark;
    }

    unshift(chunk) {
      if (this[kDestroyed]) return;

      if (typeof chunk === "string") {
        chunk = Buffer.from(chunk, this.readableEncoding || "utf8");
      }

      this._buffer.unshift(chunk);
      if (this.readableObjectMode) {
        this.readableLength++;
      } else {
        this.readableLength += chunk.length;
      }
    }

    pause() {
      if (this._readableState.flowing !== false) {
        this._readableState.flowing = false;
        this.readableFlowing = false;
        this.emit("pause");
      }
      return this;
    }

    resume() {
      if (!this._readableState.flowing) {
        this._readableState.flowing = true;
        this.readableFlowing = true;
        this.emit("resume");
        // Use setTimeout to ensure it runs in event loop
        setTimeout(() => this._flow(), 0);
      }
      return this;
    }

    _flow() {
      if (!this._readableState.flowing) return;

      // If buffer is empty, call _read to get more data
      if (this._buffer.length === 0 && !this[kEnded]) {
        this._read(this.readableHighWaterMark);
      }

      // Emit buffered data
      while (this._readableState.flowing && this._buffer.length > 0) {
        const chunk = this._buffer.shift();
        if (this.readableObjectMode) {
          this.readableLength--;
        } else {
          this.readableLength -= chunk.length;
        }
        this.emit("data", chunk);
      }

      // Check if ended
      if (
        this[kEnded] &&
        this._buffer.length === 0 &&
        !this._readableState.endEmitted
      ) {
        this._readableState.endEmitted = true;
        this.readableEnded = true;
        this.emit("end");
      }
    }

    isPaused() {
      return this._readableState.flowing === false;
    }

    pipe(dest, options = {}) {
      const end = options.end !== false;

      this._readableState.pipes.push(dest);

      const ondata = (chunk) => {
        const ret = dest.write(chunk);
        if (ret === false) {
          this.pause();
        }
      };

      const ondrain = () => {
        this.resume();
      };

      const onend = () => {
        if (end) {
          dest.end();
        }
      };

      const onerror = (err) => {
        dest.destroy(err);
      };

      const onclose = () => {
        this.unpipe(dest);
      };

      this.on("data", ondata);
      dest.on("drain", ondrain);
      this.once("end", onend);
      this.once("error", onerror);
      dest.once("close", onclose);

      dest.emit("pipe", this);

      this.resume();

      return dest;
    }

    unpipe(dest) {
      const pipes = this._readableState.pipes;

      if (dest) {
        const index = pipes.indexOf(dest);
        if (index !== -1) {
          pipes.splice(index, 1);
        }
        dest.emit("unpipe", this);
      } else {
        for (const d of pipes) {
          d.emit("unpipe", this);
        }
        this._readableState.pipes = [];
      }

      return this;
    }

    setEncoding(encoding) {
      this.readableEncoding = encoding;
      this._readableState.encoding = encoding;
      return this;
    }

    destroy(err) {
      if (this[kDestroyed]) return this;
      this[kDestroyed] = true;

      if (this._destroy) {
        this._destroy(err, (e) => {
          if (e) this.emit("error", e);
          this.emit("close");
        });
      } else {
        if (err) this.emit("error", err);
        this.emit("close");
      }

      return this;
    }

    get destroyed() {
      return this[kDestroyed];
    }

    [Symbol.asyncIterator]() {
      const stream = this;
      const buffer = [];
      let resolve = null;
      let ended = false;
      let error = null;

      stream.on("data", (chunk) => {
        if (resolve) {
          const r = resolve;
          resolve = null;
          r({ value: chunk, done: false });
        } else {
          buffer.push(chunk);
        }
      });

      stream.once("end", () => {
        ended = true;
        if (resolve) {
          const r = resolve;
          resolve = null;
          r({ value: undefined, done: true });
        }
      });

      stream.once("error", (err) => {
        error = err;
        if (resolve) {
          const r = resolve;
          resolve = null;
          r(Promise.reject(err));
        }
      });

      return {
        next() {
          if (error) return Promise.reject(error);
          if (buffer.length > 0) {
            return Promise.resolve({ value: buffer.shift(), done: false });
          }
          if (ended) {
            return Promise.resolve({ value: undefined, done: true });
          }
          return new Promise((r) => {
            resolve = r;
          });
        },
        return() {
          stream.destroy();
          return Promise.resolve({ value: undefined, done: true });
        },
        throw(err) {
          stream.destroy(err);
          return Promise.reject(err);
        },
        [Symbol.asyncIterator]() {
          return this;
        },
      };
    }

    // Static methods
    static from(iterable, options = {}) {
      const readable = new Readable(options);

      (async () => {
        try {
          for await (const chunk of iterable) {
            if (!readable.push(chunk)) {
              await new Promise((r) => readable.once("drain", r));
            }
          }
          readable.push(null);
        } catch (err) {
          readable.destroy(err);
        }
      })();

      return readable;
    }
  }

  /**
   * Writable Stream
   */
  class Writable extends EventEmitter {
    constructor(options = {}) {
      super();
      this.writable = true;
      this.writableEnded = false;
      this.writableFinished = false;
      this.writableHighWaterMark = options.highWaterMark || 16384;
      this.writableLength = 0;
      this.writableObjectMode = options.objectMode || false;
      this.writableCorked = 0;
      this[kDestroyed] = false;
      this[kFinished] = false;

      this._buffer = [];
      this._writableState = {
        ended: false,
        finished: false,
        corked: 0,
        finalCalled: false,
        needDrain: false,
      };

      if (typeof options.write === "function") {
        this._write = options.write;
      }
      if (typeof options.writev === "function") {
        this._writev = options.writev;
      }
      if (typeof options.destroy === "function") {
        this._destroy = options.destroy;
      }
      if (typeof options.final === "function") {
        this._final = options.final;
      }
    }

    _write(chunk, encoding, callback) {
      callback();
    }

    _writev(chunks, callback) {
      callback();
    }

    _final(callback) {
      callback();
    }

    write(chunk, encoding, callback) {
      if (this[kDestroyed] || this._writableState.ended) {
        const err = new Error("write after end");
        if (typeof callback === "function") {
          queueMicrotask(() => callback(err));
        }
        this.emit("error", err);
        return false;
      }

      if (typeof encoding === "function") {
        callback = encoding;
        encoding = "utf8";
      }

      if (typeof chunk === "string") {
        chunk = Buffer.from(chunk, encoding || "utf8");
      }

      if (this.writableCorked > 0) {
        this._buffer.push({ chunk, encoding, callback });
        this.writableLength += chunk.length;
        return false;
      }

      this.writableLength += chunk.length;
      const ret = this.writableLength < this.writableHighWaterMark;

      this._write(chunk, encoding, (err) => {
        this.writableLength -= chunk.length;
        if (err) {
          if (callback) callback(err);
          this.emit("error", err);
        } else {
          if (callback) callback();
          if (this._writableState.needDrain && this.writableLength === 0) {
            this._writableState.needDrain = false;
            this.emit("drain");
          }
        }
      });

      if (!ret) {
        this._writableState.needDrain = true;
      }

      return ret;
    }

    end(chunk, encoding, callback) {
      if (typeof chunk === "function") {
        callback = chunk;
        chunk = null;
      } else if (typeof encoding === "function") {
        callback = encoding;
        encoding = null;
      }

      if (this._writableState.ended) {
        if (callback) queueMicrotask(callback);
        return this;
      }

      this._writableState.ended = true;
      this.writableEnded = true;

      if (chunk !== null && chunk !== undefined) {
        this.write(chunk, encoding);
      }

      if (callback) {
        this.once("finish", callback);
      }

      this._doFinish();

      return this;
    }

    _doFinish() {
      if (this[kFinished] || this._writableState.finalCalled) return;

      const finish = () => {
        this[kFinished] = true;
        this.writableFinished = true;
        this.emit("finish");
      };

      if (this._final && !this._writableState.finalCalled) {
        this._writableState.finalCalled = true;
        this._final((err) => {
          if (err) {
            this.emit("error", err);
          } else {
            finish();
          }
        });
      } else {
        finish();
      }
    }

    cork() {
      this.writableCorked++;
      this._writableState.corked++;
    }

    uncork() {
      if (this.writableCorked > 0) {
        this.writableCorked--;
        this._writableState.corked--;

        if (this.writableCorked === 0 && this._buffer.length > 0) {
          const buffer = this._buffer;
          this._buffer = [];

          if (this._writev) {
            this._writev(buffer, (err) => {
              if (err) this.emit("error", err);
            });
          } else {
            for (const { chunk, encoding, callback } of buffer) {
              this._write(chunk, encoding, callback || (() => {}));
            }
          }
        }
      }
    }

    setDefaultEncoding(encoding) {
      this._defaultEncoding = encoding;
      return this;
    }

    destroy(err) {
      if (this[kDestroyed]) return this;
      this[kDestroyed] = true;

      if (this._destroy) {
        this._destroy(err, (e) => {
          if (e) this.emit("error", e);
          this.emit("close");
        });
      } else {
        if (err) this.emit("error", err);
        this.emit("close");
      }

      return this;
    }

    get destroyed() {
      return this[kDestroyed];
    }
  }

  /**
   * Duplex Stream - Both Readable and Writable
   */
  class Duplex extends Readable {
    constructor(options = {}) {
      super(options);

      // Add Writable properties
      this.writable = true;
      this.writableEnded = false;
      this.writableFinished = false;
      this.writableHighWaterMark =
        options.writableHighWaterMark || options.highWaterMark || 16384;
      this.writableLength = 0;
      this.writableObjectMode =
        options.writableObjectMode || options.objectMode || false;
      this.writableCorked = 0;
      this[kFinished] = false;

      this._writeBuffer = [];
      this._writableState = {
        ended: false,
        finished: false,
        corked: 0,
        finalCalled: false,
        needDrain: false,
      };

      if (typeof options.write === "function") {
        this._write = options.write;
      }
      if (typeof options.writev === "function") {
        this._writev = options.writev;
      }
      if (typeof options.final === "function") {
        this._final = options.final;
      }
    }

    // Inherit Writable methods
    _write(chunk, encoding, callback) {
      callback();
    }

    _writev(chunks, callback) {
      callback();
    }

    _final(callback) {
      callback();
    }

    write(chunk, encoding, callback) {
      return Writable.prototype.write.call(this, chunk, encoding, callback);
    }

    end(chunk, encoding, callback) {
      return Writable.prototype.end.call(this, chunk, encoding, callback);
    }

    cork() {
      Writable.prototype.cork.call(this);
    }

    uncork() {
      Writable.prototype.uncork.call(this);
    }

    setDefaultEncoding(encoding) {
      return Writable.prototype.setDefaultEncoding.call(this, encoding);
    }

    _doFinish() {
      Writable.prototype._doFinish.call(this);
    }
  }

  // Copy Writable methods to Duplex prototype
  Duplex.prototype._write = Writable.prototype._write;
  Duplex.prototype._writev = Writable.prototype._writev;
  Duplex.prototype._final = Writable.prototype._final;

  /**
   * Transform Stream - Modify data passing through
   */
  class Transform extends Duplex {
    constructor(options = {}) {
      super(options);

      this._transformState = {
        transforming: false,
        writecb: null,
        writechunk: null,
      };

      if (typeof options.transform === "function") {
        this._transform = options.transform;
      }
      if (typeof options.flush === "function") {
        this._flush = options.flush;
      }
    }

    _transform(chunk, encoding, callback) {
      callback(null, chunk);
    }

    _flush(callback) {
      callback();
    }

    _write(chunk, encoding, callback) {
      this._transformState.transforming = true;
      this._transformState.writecb = callback;
      this._transformState.writechunk = chunk;

      this._transform(chunk, encoding, (err, data) => {
        this._transformState.transforming = false;

        if (err) {
          callback(err);
          return;
        }

        if (data !== null && data !== undefined) {
          this.push(data);
        }

        callback();
      });
    }

    _final(callback) {
      this._flush((err, data) => {
        if (err) {
          callback(err);
          return;
        }

        if (data !== null && data !== undefined) {
          this.push(data);
        }

        this.push(null);
        callback();
      });
    }
  }

  /**
   * PassThrough Stream - Pass data through unchanged
   */
  class PassThrough extends Transform {
    constructor(options) {
      super(options);
    }

    _transform(chunk, encoding, callback) {
      callback(null, chunk);
    }
  }

  /**
   * pipeline - Connect streams and handle errors
   */
  function pipeline(...streams) {
    let callback = streams[streams.length - 1];
    if (typeof callback !== "function") {
      callback = null;
    } else {
      streams = streams.slice(0, -1);
    }

    if (streams.length < 2) {
      throw new Error("pipeline requires at least 2 streams");
    }

    let error = null;
    const destroys = [];

    const destroyer = (stream, reading, writing) => {
      let closed = false;

      const cleanup = () => {
        if (closed) return;
        closed = true;
        destroys.forEach((d) => d());
      };

      const onclose = () => {
        cleanup();
      };

      const onerror = (err) => {
        if (!error) error = err;
        cleanup();
        if (callback) callback(error);
      };

      stream.on("close", onclose);
      stream.on("error", onerror);

      destroys.push(() => {
        stream.removeListener("close", onclose);
        stream.removeListener("error", onerror);
      });
    };

    // Connect streams
    for (let i = 0; i < streams.length - 1; i++) {
      const src = streams[i];
      const dest = streams[i + 1];

      destroyer(src, true, false);
      destroyer(dest, false, true);

      src.pipe(dest);
    }

    // Handle completion
    const last = streams[streams.length - 1];
    last.on("finish", () => {
      if (callback && !error) {
        callback(null);
      }
    });

    return last;
  }

  /**
   * finished - Get notified when stream is done
   */
  function finished(stream, options, callback) {
    if (typeof options === "function") {
      callback = options;
      options = {};
    }

    options = options || {};

    const readable = options.readable !== false && stream.readable;
    const writable = options.writable !== false && stream.writable;

    let readableEnded = !readable;
    let writableFinished = !writable;
    let closed = false;
    let errored = false;

    const cleanup = () => {
      stream.removeListener("end", onend);
      stream.removeListener("finish", onfinish);
      stream.removeListener("error", onerror);
      stream.removeListener("close", onclose);
    };

    const onend = () => {
      readableEnded = true;
      if (writableFinished) {
        cleanup();
        callback(null);
      }
    };

    const onfinish = () => {
      writableFinished = true;
      if (readableEnded) {
        cleanup();
        callback(null);
      }
    };

    const onerror = (err) => {
      if (errored) return;
      errored = true;
      cleanup();
      callback(err);
    };

    const onclose = () => {
      if (closed) return;
      closed = true;

      if (readable && !readableEnded) {
        cleanup();
        callback(new Error("Premature close"));
        return;
      }
      if (writable && !writableFinished) {
        cleanup();
        callback(new Error("Premature close"));
        return;
      }
    };

    if (readable) stream.on("end", onend);
    if (writable) stream.on("finish", onfinish);
    stream.on("error", onerror);
    stream.on("close", onclose);

    return cleanup;
  }

  /**
   * addAbortSignal - Add abort signal support to stream
   */
  function addAbortSignal(signal, stream) {
    if (signal.aborted) {
      stream.destroy(new Error("AbortError"));
    } else {
      signal.addEventListener(
        "abort",
        () => {
          stream.destroy(new Error("AbortError"));
        },
        { once: true },
      );
    }
    return stream;
  }

  // Build module object
  // Create a Stream base class that is the main export
  // In Node.js, require('stream') returns the Stream constructor
  class Stream extends EventEmitter {
    constructor(options) {
      super();
      // Stream is basically an EventEmitter with pipe support
    }

    pipe(dest, options) {
      // Basic pipe implementation - delegate to Readable if this is readable
      return dest;
    }
  }

  // Make Stream also work as a namespace with all stream classes
  Stream.Stream = Stream;
  Stream.Readable = Readable;
  Stream.Writable = Writable;
  Stream.Duplex = Duplex;
  Stream.Transform = Transform;
  Stream.PassThrough = PassThrough;
  Stream.pipeline = pipeline;
  Stream.finished = finished;
  Stream.addAbortSignal = addAbortSignal;

  // Promisified versions
  Stream.promises = {
    pipeline: (...streams) => {
      return new Promise((resolve, reject) => {
        pipeline(...streams, (err) => {
          if (err) reject(err);
          else resolve();
        });
      });
    },
    finished: (stream, options) => {
      return new Promise((resolve, reject) => {
        finished(stream, options, (err) => {
          if (err) reject(err);
          else resolve();
        });
      });
    },
  };

  // Also export classes directly for CommonJS compatibility
  Stream.default = Stream;

  // Export to global - Stream is the main export (a constructor function)
  globalThis.stream = Stream;
  globalThis.Stream = Stream;
})();
