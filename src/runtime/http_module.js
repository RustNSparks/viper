// Node.js HTTP Module - Pure JavaScript Implementation
// This provides 100% Node.js compatibility using the global fetch API

(function () {
  // HTTP Status Codes
  const STATUS_CODES = {
    100: "Continue",
    101: "Switching Protocols",
    102: "Processing",
    103: "Early Hints",
    200: "OK",
    201: "Created",
    202: "Accepted",
    203: "Non-Authoritative Information",
    204: "No Content",
    205: "Reset Content",
    206: "Partial Content",
    207: "Multi-Status",
    208: "Already Reported",
    226: "IM Used",
    300: "Multiple Choices",
    301: "Moved Permanently",
    302: "Found",
    303: "See Other",
    304: "Not Modified",
    305: "Use Proxy",
    307: "Temporary Redirect",
    308: "Permanent Redirect",
    400: "Bad Request",
    401: "Unauthorized",
    402: "Payment Required",
    403: "Forbidden",
    404: "Not Found",
    405: "Method Not Allowed",
    406: "Not Acceptable",
    407: "Proxy Authentication Required",
    408: "Request Timeout",
    409: "Conflict",
    410: "Gone",
    411: "Length Required",
    412: "Precondition Failed",
    413: "Payload Too Large",
    414: "URI Too Long",
    415: "Unsupported Media Type",
    416: "Range Not Satisfiable",
    417: "Expectation Failed",
    418: "I'm a Teapot",
    421: "Misdirected Request",
    422: "Unprocessable Entity",
    423: "Locked",
    424: "Failed Dependency",
    425: "Too Early",
    426: "Upgrade Required",
    428: "Precondition Required",
    429: "Too Many Requests",
    431: "Request Header Fields Too Large",
    451: "Unavailable For Legal Reasons",
    500: "Internal Server Error",
    501: "Not Implemented",
    502: "Bad Gateway",
    503: "Service Unavailable",
    504: "Gateway Timeout",
    505: "HTTP Version Not Supported",
    506: "Variant Also Negotiates",
    507: "Insufficient Storage",
    508: "Loop Detected",
    509: "Bandwidth Limit Exceeded",
    510: "Not Extended",
    511: "Network Authentication Required",
  };

  const METHODS = [
    "GET",
    "POST",
    "PUT",
    "DELETE",
    "PATCH",
    "HEAD",
    "OPTIONS",
    "CONNECT",
    "TRACE",
  ];

  // EventEmitter-like mixin
  function addEventEmitter(obj) {
    obj._events = {};

    obj.on = function (event, callback) {
      if (!this._events[event]) this._events[event] = [];
      this._events[event].push(callback);
      return this;
    };

    obj.once = function (event, callback) {
      const wrapper = (...args) => {
        callback(...args);
        this.off(event, wrapper);
      };
      return this.on(event, wrapper);
    };

    obj.off = function (event, callback) {
      if (!this._events[event]) return this;
      this._events[event] = this._events[event].filter((cb) => cb !== callback);
      return this;
    };

    obj.emit = function (event, ...args) {
      if (!this._events[event]) return false;
      this._events[event].forEach((cb) => {
        try {
          cb(...args);
        } catch (e) {
          console.error("Event handler error:", e);
        }
      });
      return true;
    };

    return obj;
  }

  // IncomingMessage class
  class IncomingMessage {
    constructor(data = {}) {
      this.httpVersion = data.httpVersion || "1.1";
      this.httpVersionMajor = parseInt(this.httpVersion.split(".")[0]) || 1;
      this.httpVersionMinor = parseInt(this.httpVersion.split(".")[1]) || 1;
      this.complete = false;
      this.headers = data.headers || {};
      this.rawHeaders = data.rawHeaders || [];
      this.trailers = {};
      this.rawTrailers = [];
      this.method = data.method;
      this.url = data.url;
      this.statusCode = data.statusCode;
      this.statusMessage = data.statusMessage;
      this.socket = data.socket || null;
      this.connection = this.socket;
      this._body = data.body || "";

      addEventEmitter(this);
    }

    setTimeout(msecs, callback) {
      if (callback) this.once("timeout", callback);
      this._timeout = setTimeout(() => this.emit("timeout"), msecs);
      return this;
    }

    destroy(error) {
      if (this._timeout) clearTimeout(this._timeout);
      if (error) this.emit("error", error);
      this.emit("close");
      return this;
    }
  }

  // ServerResponse class
  class ServerResponse {
    constructor(req) {
      this.req = req;
      this.statusCode = 200;
      this.statusMessage = "OK";
      this.headersSent = false;
      this.sendDate = true;
      this.finished = false;
      this.socket = req.socket;
      this.connection = this.socket;
      this._headers = {};
      this._chunks = [];
      this.writableEnded = false;
      this.writableFinished = false;

      addEventEmitter(this);
    }

    setHeader(name, value) {
      if (this.headersSent) {
        throw new Error("Cannot set headers after they are sent");
      }
      this._headers[name.toLowerCase()] = value;
      return this;
    }

    getHeader(name) {
      return this._headers[name.toLowerCase()];
    }

    getHeaders() {
      return { ...this._headers };
    }

    getHeaderNames() {
      return Object.keys(this._headers);
    }

    hasHeader(name) {
      return name.toLowerCase() in this._headers;
    }

    removeHeader(name) {
      delete this._headers[name.toLowerCase()];
    }

    writeHead(statusCode, statusMessage, headers) {
      if (typeof statusMessage === "object") {
        headers = statusMessage;
        statusMessage = undefined;
      }

      this.statusCode = statusCode;
      if (statusMessage) this.statusMessage = statusMessage;
      else this.statusMessage = STATUS_CODES[statusCode] || "Unknown";

      if (headers) {
        for (const [key, value] of Object.entries(headers)) {
          this.setHeader(key, value);
        }
      }

      this.headersSent = true;
      return this;
    }

    write(chunk, encoding, callback) {
      if (typeof encoding === "function") {
        callback = encoding;
        encoding = "utf8";
      }

      if (!this.headersSent) {
        this.writeHead(this.statusCode);
      }

      if (typeof chunk === "string") {
        chunk = new TextEncoder().encode(chunk);
      }

      this._chunks.push(chunk);

      if (callback) setTimeout(callback, 0);
      return true;
    }

    end(data, encoding, callback) {
      if (typeof data === "function") {
        callback = data;
        data = undefined;
      } else if (typeof encoding === "function") {
        callback = encoding;
        encoding = "utf8";
      }

      if (data !== undefined) {
        this.write(data, encoding);
      }

      if (!this.headersSent) {
        this.writeHead(this.statusCode);
      }

      this.writableEnded = true;
      this.finished = true;

      // Call native handler if set
      if (this._nativeEnd) {
        this._nativeEnd(this);
      }

      setTimeout(() => {
        this.emit("finish");
        this.writableFinished = true;
        if (callback) callback();
      }, 0);

      return this;
    }

    setTimeout(msecs, callback) {
      if (callback) this.once("timeout", callback);
      return this;
    }

    addTrailers(headers) {
      this._trailers = headers;
    }

    flushHeaders() {
      if (!this.headersSent) {
        this.writeHead(this.statusCode);
      }
    }

    writeContinue() {
      this.writeHead(100);
    }

    writeProcessing() {
      this.writeHead(102);
    }

    writeEarlyHints(hints, callback) {
      if (callback) setTimeout(callback, 0);
    }

    cork() {}
    uncork() {}

    assignSocket() {}
    detachSocket() {}
  }

  // ClientRequest class
  class ClientRequest {
    constructor(options, callback) {
      if (typeof options === "string") {
        options = new URL(options);
      } else if (options instanceof URL) {
        options = {
          protocol: options.protocol,
          hostname: options.hostname,
          port: options.port,
          path: options.pathname + options.search,
        };
      }

      this.method = options.method || "GET";
      this.path = options.path || "/";
      this.host = options.host || options.hostname || "localhost";
      this.port = options.port || (options.protocol === "https:" ? 443 : 80);
      this.protocol = options.protocol || "http:";
      this.headers = options.headers || {};
      this.aborted = false;
      this.finished = false;
      this.socket = null;
      this.connection = null;
      this.reusedSocket = false;
      this._chunks = [];

      addEventEmitter(this);

      if (callback) {
        this.once("response", callback);
      }

      // Auto-start request
      setTimeout(() => this._executeRequest(), 0);
    }

    async _executeRequest() {
      try {
        // Build URL
        const url = `${this.protocol}//${this.host}:${this.port}${this.path}`;

        // Build body
        let body = undefined;
        if (this._chunks.length > 0) {
          const totalLength = this._chunks.reduce(
            (acc, chunk) => acc + chunk.length,
            0,
          );
          const combined = new Uint8Array(totalLength);
          let offset = 0;
          for (const chunk of this._chunks) {
            combined.set(chunk, offset);
            offset += chunk.length;
          }
          body = combined;
        }

        // Make fetch request
        const response = await fetch(url, {
          method: this.method,
          headers: this.headers,
          body: body,
        });

        // Create IncomingMessage
        const headers = {};
        const rawHeaders = [];
        response.headers.forEach((value, key) => {
          headers[key] = value;
          rawHeaders.push(key, value);
        });

        const msg = new IncomingMessage({
          statusCode: response.status,
          statusMessage: response.statusText,
          headers: headers,
          rawHeaders: rawHeaders,
          httpVersion: "1.1",
        });

        // Emit response
        this.emit("response", msg);

        // Read body
        const text = await response.text();
        msg._body = text;

        // Emit data and end
        setTimeout(() => {
          if (text) {
            msg.emit("data", new TextEncoder().encode(text));
          }
          msg.complete = true;
          msg.emit("end");
        }, 0);
      } catch (error) {
        this.emit("error", error);
      }
    }

    write(chunk, encoding, callback) {
      if (typeof encoding === "function") {
        callback = encoding;
        encoding = "utf8";
      }

      if (typeof chunk === "string") {
        chunk = new TextEncoder().encode(chunk);
      }

      this._chunks.push(chunk);

      if (callback) setTimeout(callback, 0);
      return true;
    }

    end(data, encoding, callback) {
      if (typeof data === "function") {
        callback = data;
        data = undefined;
      } else if (typeof encoding === "function") {
        callback = encoding;
        encoding = "utf8";
      }

      if (data !== undefined) {
        this.write(data, encoding);
      }

      this.finished = true;

      setTimeout(() => {
        this.emit("finish");
        if (callback) callback();
      }, 0);

      return this;
    }

    abort() {
      this.aborted = true;
      this.emit("abort");
      this.emit("close");
    }

    setTimeout(timeout, callback) {
      if (callback) this.once("timeout", callback);
      return this;
    }

    setHeader(name, value) {
      this.headers[name] = value;
    }

    getHeader(name) {
      return this.headers[name];
    }

    getHeaders() {
      return { ...this.headers };
    }

    getHeaderNames() {
      return Object.keys(this.headers);
    }

    getRawHeaderNames() {
      return Object.keys(this.headers);
    }

    hasHeader(name) {
      return name in this.headers;
    }

    removeHeader(name) {
      delete this.headers[name];
    }

    flushHeaders() {}
    cork() {}
    uncork() {}

    setNoDelay() {
      return this;
    }
    setSocketKeepAlive() {
      return this;
    }
    onSocket() {}
  }

  // Agent class
  class Agent {
    constructor(options = {}) {
      this.options = options;
      this.maxSockets =
        options.maxSockets !== undefined ? options.maxSockets : Infinity;
      this.maxFreeSockets =
        options.maxFreeSockets !== undefined ? options.maxFreeSockets : 256;
      this.maxTotalSockets =
        options.maxTotalSockets !== undefined
          ? options.maxTotalSockets
          : Infinity;
      this.sockets = {};
      this.freeSockets = {};
      this.requests = {};
      this.keepAlive = options.keepAlive || false;
      this.keepAliveMsecs = options.keepAliveMsecs || 1000;
      this.timeout = options.timeout;
      this.scheduling = options.scheduling || "lifo";
    }

    getName(options) {
      let name = options.host || options.hostname || "localhost";
      name += ":";
      name += options.port || 80;
      if (options.localAddress) {
        name += ":" + options.localAddress;
      }
      return name;
    }

    destroy() {
      this.sockets = {};
      this.freeSockets = {};
      this.requests = {};
    }
  }

  // Server class (using Viper.serve internally)
  class Server {
    constructor(options, requestListener) {
      if (typeof options === "function") {
        requestListener = options;
        options = {};
      }

      this.listening = false;
      this.maxHeadersCount = null;
      this.timeout = 0;
      this.keepAliveTimeout = 5000;
      this.requestTimeout = 0;
      this.headersTimeout = 60000;
      this.maxRequestsPerSocket = 0;
      this._requestListener = requestListener;
      this._viperServer = null;

      addEventEmitter(this);

      if (requestListener) {
        this.on("request", requestListener);
      }
    }

    listen(port, hostname, backlog, callback) {
      // Parse arguments
      if (typeof hostname === "function") {
        callback = hostname;
        hostname = "127.0.0.1";
      } else if (typeof backlog === "function") {
        callback = backlog;
      }

      port = port || 0;
      hostname = hostname || "127.0.0.1";

      // Use Viper.serve if available
      if (typeof Viper !== "undefined" && Viper.serve) {
        try {
          this._viperServer = Viper.serve({
            port: port,
            hostname: hostname,
            fetch: (req) => {
              // Convert to Node.js style request/response
              const nodeReq = new IncomingMessage({
                method: req.method,
                url: req.url,
                headers: (() => {
                  const h = {};
                  req.headers.forEach((v, k) => (h[k] = v));
                  return h;
                })(),
              });

              const nodeRes = new ServerResponse(nodeReq);

              // Create promise that resolves when response ends
              const responsePromise = new Promise((resolve) => {
                // Capture response
                nodeRes._nativeEnd = (res) => {
                  try {
                    // Build body
                    let body = "";
                    for (const chunk of res._chunks) {
                      if (typeof chunk === "string") {
                        body += chunk;
                      } else {
                        body += new TextDecoder().decode(chunk);
                      }
                    }

                    // Create Response
                    const headers = new Headers(res._headers);
                    const response = new Response(body, {
                      status: res.statusCode,
                      statusText: res.statusMessage,
                      headers: headers,
                    });

                    console.log(
                      "Resolving with response, status:",
                      res.statusCode,
                      "body length:",
                      body.length,
                    );
                    resolve(response);
                  } catch (error) {
                    console.error("Error in _nativeEnd:", error);
                    resolve(
                      new Response("Internal Server Error", { status: 500 }),
                    );
                  }
                };
              });

              // Emit request event synchronously
              try {
                this.emit("request", nodeReq, nodeRes);
              } catch (error) {
                console.error("Request handler error:", error);
                return new Response("Internal Server Error", { status: 500 });
              }

              // Return the promise
              return responsePromise;
            },
          });

          this.listening = true;

          if (callback) {
            setTimeout(callback, 0);
          }

          this.emit("listening");
        } catch (e) {
          console.error("Failed to start server:", e);
          this.emit("error", e);
        }
      } else {
        console.warn(
          "Viper.serve not available, server will not actually listen",
        );
        this.listening = true;
        if (callback) setTimeout(callback, 0);
      }

      return this;
    }

    close(callback) {
      this.listening = false;
      this.emit("close");
      if (callback) setTimeout(callback, 0);
      return this;
    }

    closeAllConnections() {}
    closeIdleConnections() {}

    address() {
      if (this._viperServer) {
        return {
          port: this._viperServer.port,
          address: this._viperServer.hostname,
          family: "IPv4",
        };
      }
      return null;
    }

    setTimeout(msecs, callback) {
      if (typeof msecs === "function") {
        callback = msecs;
        msecs = 0;
      }
      this.timeout = msecs;
      if (callback) this.on("timeout", callback);
      return this;
    }
  }

  // Global agent
  const globalAgent = new Agent({
    keepAlive: true,
    timeout: 5000,
  });

  // Module exports
  const http = {
    METHODS: METHODS,
    STATUS_CODES: STATUS_CODES,

    Agent: Agent,
    Server: Server,
    IncomingMessage: IncomingMessage,
    ServerResponse: ServerResponse,
    ClientRequest: ClientRequest,
    OutgoingMessage: ServerResponse, // Alias

    createServer(options, requestListener) {
      return new Server(options, requestListener);
    },

    request(options, callback) {
      if (typeof options === "string" || options instanceof URL) {
        return new ClientRequest(options, callback);
      }
      return new ClientRequest(options, callback);
    },

    get(options, callback) {
      if (typeof options === "string" || options instanceof URL) {
        const req = new ClientRequest(options, callback);
        req.end();
        return req;
      }
      options.method = "GET";
      const req = new ClientRequest(options, callback);
      req.end();
      return req;
    },

    globalAgent: globalAgent,
    maxHeaderSize: 16384,

    validateHeaderName(name) {
      if (typeof name !== "string" || name.length === 0) {
        throw new TypeError("Header name must be a valid string");
      }
    },

    validateHeaderValue(name, value) {
      if (value === undefined) {
        throw new TypeError(`Invalid value "${value}" for header "${name}"`);
      }
    },

    setMaxIdleHTTPParsers(max) {
      // No-op for compatibility
    },
  };

  // Export to global scope
  globalThis.http = http;
})();
