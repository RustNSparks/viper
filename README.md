# Viper

A fast TypeScript runtime written in Rust, powered by [Boa](https://github.com/boa-dev/boa) (JS engine) and [OXC](https://github.com/oxc-project/oxc) (TypeScript transpiler).

> **Note:** This is an experimental project built with extensive AI assistance (Claude) to explore what's possible when combining Rust's performance with modern JavaScript tooling.

## Features

### Core Runtime

- **TypeScript/TSX Execution** - Run `.ts` and `.tsx` files directly without a compilation step
- **Fast Transpilation** - OXC-powered TypeScript to JavaScript conversion (50-100x faster than tsc)
- **ES Modules** - Full ESM support with `import`/`export`
- **CommonJS Interop** - Automatic CJS to ESM wrapping for npm packages
- **JSX/TSX Support** - Built-in JSX runtime with `renderToString()`
- **Async/Await** - Full Promise support with event loop

### Web APIs

- **Fetch API** - `fetch()`, `Request`, `Response`, `Headers`
- **URL API** - `URL`, `URLSearchParams`
- **Encoding** - `TextEncoder`, `TextDecoder`
- **Timers** - `setTimeout`, `setInterval`, `clearTimeout`, `clearInterval`, `queueMicrotask`
- **Console** - Full `console` API (`log`, `error`, `warn`, `info`, `debug`, `table`, `time`, etc.)
- **Crypto** - `crypto.randomUUID()`, `crypto.getRandomValues()`, `crypto.subtle` (hashing)
- **Structured Clone** - `structuredClone()` with transferable support
- **Events** - `EventTarget`, `Event`, `AbortController`, `AbortSignal`

### Node.js Built-in Modules

Viper now includes comprehensive support for Node.js built-in modules:

- **assert** - Assertion testing (`assert`, `assert.strictEqual`, `assert.deepStrictEqual`, etc.)
- **buffer** - Binary data handling (`Buffer.from()`, `Buffer.alloc()`, `Buffer.concat()`, etc.)
- **events** - Event emitter pattern (`EventEmitter`, `on()`, `emit()`, `once()`, etc.)
- **http** - HTTP client and server (`http.request()`, `http.get()`, `http.createServer()`)
- **net** - TCP networking (`net.createServer()`, `net.connect()`, Socket API)
- **os** - Operating system utilities (`os.platform()`, `os.cpus()`, `os.homedir()`, etc.)
- **path** - File path operations (`path.join()`, `path.resolve()`, `path.dirname()`, etc.)
- **querystring** - URL query string parsing (`querystring.parse()`, `querystring.stringify()`)
- **stream** - Stream API (`Readable`, `Writable`, `Transform`, `pipeline()`)
- **string_decoder** - String decoding (`StringDecoder`)
- **url** - URL parsing and formatting (`url.parse()`, `url.format()`, `URL` class)
- **util** - Utility functions (`util.promisify()`, `util.inherits()`, `util.inspect()`, etc.)
- **zlib** - Compression (`zlib.gzip()`, `zlib.gunzip()`, `zlib.deflate()`, etc.)

### Networking

- **HTTP Client** - Full Fetch API support + Node.js `http` module
- **HTTP Server** - `Viper.serve()` and `http.createServer()` for creating HTTP servers (requires `--features server`)
- **TCP Sockets** - `net` module for TCP client/server communication
- **WebSocket Client** - Ultra-fast WebSocket client with event-driven architecture and binary message support

### Workers

- **Web Workers** - Multi-threaded execution with `new Worker()`
- **Message Passing** - `postMessage()` / `onmessage` with structured clone
- **MessageChannel** - `MessageChannel` and `MessagePort` for bidirectional communication
- **Transferables** - `ArrayBuffer` transfer support

### File System

- **Bun-style API** - `file()`, `write()`, `readFile()`, `exists()`, `mkdir()`, `readDir()`, `stat()`
- **Node.js fs/promises** - Compatible with Node.js `fs.promises` API

### Process & System

- **Process API** - `process.argv`, `process.env`, `process.cwd()`, `process.exit()`, `process.platform`, `process.arch`, `process.memoryUsage()`
- **Spawn/Exec** - `Viper.spawn()`, `Viper.exec()` for running external commands
- **OS Module** - System information (`os.platform()`, `os.arch()`, `os.cpus()`, `os.totalmem()`, `os.freemem()`, etc.)

### Package Manager (Optional)

Built-in package manager powered by [Orogene](https://github.com/orogene/orogene):

```bash
# Build with package manager support
cargo build --release --features pm

# Install dependencies
viper install

# Add packages
viper add lodash date-fns
viper add -D typescript  # dev dependency

# Remove packages
viper remove lodash
```

### Bundler

Basic bundler that transpiles and concatenates TypeScript/JavaScript files:

```bash
# Bundle files
viper build src/index.ts -o dist --format esm

# With minification
viper build src/index.ts -o dist --minify

# Different formats (esm, cjs, iife)
viper build src/index.ts -o dist --format iife
```

> **Note:** The `rolldown` feature is currently disabled due to compatibility issues with `oxc_resolver` v11.16. Viper uses a simple transpile-and-concatenate bundler. For production bundling, consider using external tools like esbuild or Rollup.

## Installation

### From Source

```bash
# Clone the repo
git clone https://github.com/user/viper
cd viper

# Build release binary
cargo build --release

# With HTTP server support
cargo build --release --features server

# With package manager
cargo build --release --features pm

# With all features
cargo build --release --features "server pm"

# Binary is at ./target/release/viper
```

## Usage

### Run TypeScript Files

```bash
# Run a TypeScript file
viper run hello.ts

# Run with file argument
viper hello.ts

# Evaluate inline code
viper -e "console.log('Hello, Viper!')"
```

### REPL

```bash
viper repl
# or just
viper
```

### Transpile Only

```bash
# Output to stdout
viper transpile input.ts

# Output to file
viper transpile input.ts -o output.js

# Minify
viper transpile input.ts --minify
```

### Bundle

```bash
viper bundle src/index.ts -o dist --format esm
```

## Examples

### Hello World

```typescript
// hello.ts
const message: string = "Hello from Viper!";
console.log(message);
console.log(`Platform: ${process.platform}, Arch: ${process.arch}`);
```

### Async/Await & Fetch

```typescript
// fetch.ts
const response = await fetch("https://httpbin.org/get");
const data = await response.json();
console.log(`Origin: ${data.origin}`);

// POST request
const post = await fetch("https://httpbin.org/post", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ message: "Hello!" }),
});
console.log(await post.json());
```

### File System

```typescript
// fs.ts
// Read file
const content = await file("./data.txt").text();
console.log(content);

// Write file
await write("./output.txt", "Hello, World!");

// Check existence
if (await exists("./config.json")) {
  const config = JSON.parse(await readFile("./config.json"));
  console.log(config);
}

// Create directory
await mkdir("./logs", { recursive: true });

// List directory
const files = await readDir("./src");
console.log(files);
```

### HTTP Server

```typescript
// server.ts (run with: viper --features server server.ts)
const server = Viper.serve({
  port: 3000,
  hostname: "127.0.0.1",
  fetch(request: Request): Response {
    const url = new URL(request.url);
    
    if (url.pathname === "/api/hello") {
      return Response.json({ message: "Hello from Viper!" });
    }
    
    return new Response("Not Found", { status: 404 });
  },
});

console.log(`Server running at http://${server.hostname}:${server.port}`);
```

### Web Workers

```typescript
// main.ts
const workerCode = `
  self.onmessage = (event) => {
    const result = event.data * 2;
    self.postMessage(result);
  };
`;

await write("./worker.ts", workerCode);

const worker = new Worker("./worker.ts");

worker.onmessage = (event) => {
  console.log(`Result: ${event.data}`);
  worker.terminate();
};

worker.postMessage(21); // Output: Result: 42
```

### WebSocket Client

```typescript
// websocket.ts
const ws = new WebSocket("wss://ws.postman-echo.com/raw");

ws.onopen = () => {
  console.log("Connected!");
  ws.send("Hello, WebSocket!");
};

ws.onmessage = (event) => {
  console.log(`Received: ${event.data}`);
  ws.close();
};

ws.onclose = () => {
  console.log("Connection closed");
};
```

### Crypto

```typescript
// crypto.ts
// Generate UUID
console.log(crypto.randomUUID());

// Random bytes
const bytes = crypto.randomBytes(16);
console.log(Array.from(bytes).map(b => b.toString(16).padStart(2, "0")).join(""));

// SHA-256 hash
const data = new TextEncoder().encode("Hello, World!");
const hash = await crypto.subtle.digest("SHA-256", data);
const hashHex = Array.from(new Uint8Array(hash))
  .map(b => b.toString(16).padStart(2, "0"))
  .join("");
console.log(`SHA-256: ${hashHex}`);
```

### Node.js Modules

```typescript
// path.ts
import path from "path";

console.log(path.join("src", "lib", "index.ts"));
console.log(path.resolve(".")); 
console.log(path.dirname("/home/user/file.ts"));
console.log(path.basename("/home/user/file.ts", ".ts"));
console.log(path.extname("file.ts"));

const parsed = path.parse("/home/user/file.ts");
console.log(parsed); // { root, dir, base, name, ext }
```

```typescript
// buffer.ts
import { Buffer } from "buffer";

const buf = Buffer.from("Hello, World!");
console.log(buf.toString("hex"));
console.log(buf.toString("base64"));

const buf2 = Buffer.alloc(10);
buf2.write("Hello");
console.log(buf2.toString());
```

```typescript
// http-server.ts
import http from "http";

const server = http.createServer((req, res) => {
  res.writeHead(200, { "Content-Type": "application/json" });
  res.end(JSON.stringify({ message: "Hello from Node.js http!" }));
});

server.listen(3000, () => {
  console.log("Server running on http://localhost:3000");
});
```

```typescript
// events.ts
import { EventEmitter } from "events";

const emitter = new EventEmitter();

emitter.on("data", (msg) => {
  console.log("Received:", msg);
});

emitter.emit("data", "Hello, Events!");
```

```typescript
// streams.ts
import { Readable, Writable, Transform } from "stream";

const readable = Readable.from(["Hello", " ", "World"]);
const writable = new Writable({
  write(chunk, encoding, callback) {
    console.log(chunk.toString());
    callback();
  },
});

readable.pipe(writable);
```

```typescript
// zlib.ts
import zlib from "zlib";
import { Buffer } from "buffer";

const input = Buffer.from("Hello, compression!");
const compressed = await zlib.gzip(input);
console.log("Compressed size:", compressed.length);

const decompressed = await zlib.gunzip(compressed);
console.log("Decompressed:", decompressed.toString());
```

```typescript
// assert.ts
import assert from "assert";

assert.strictEqual(1 + 1, 2);
assert.deepStrictEqual({ a: 1 }, { a: 1 });
assert.throws(() => { throw new Error("test"); });

console.log("All assertions passed!");
```

### JSX/TSX

```tsx
// app.tsx
function App({ name }: { name: string }) {
  return (
    <html>
      <head><title>Hello</title></head>
      <body>
        <h1>Hello, {name}!</h1>
      </body>
    </html>
  );
}

const html = renderToString(<App name="Viper" />);
console.log(html);
```

### Process & Spawn

```typescript
// spawn.ts
// Run a command
const result = await Viper.spawn("echo", ["Hello from spawn!"]);
console.log(result.stdout);

// Execute shell command
const shell = await Viper.exec("echo $USER");
console.log(shell.stdout);

// Process info
console.log(`PID: ${process.pid}`);
console.log(`CWD: ${process.cwd()}`);
console.log(`Platform: ${process.platform}`);

// Memory usage
const mem = process.memoryUsage();
console.log(`Heap used: ${(mem.heapUsed / 1024 / 1024).toFixed(2)} MB`);
```

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    CLI (clap)                           │
├─────────────────────────────────────────────────────────┤
│                TypeScript Source                        │
├─────────────────────────────────────────────────────────┤
│            OXC Transpiler (TS → JS)                     │
│     - TypeScript parsing & transformation               │
│     - JSX/TSX support                                   │
│     - 50-100x faster than tsc                           │
├─────────────────────────────────────────────────────────┤
│              Boa JS Engine                              │
│     - ECMAScript execution                              │
│     - ES Modules                                        │
│     - Async/await & Promises                            │
├─────────────────────────────────────────────────────────┤
│              boa_runtime                                │
│     - Console, Fetch, URL                               │
│     - Timers, Encoding                                  │
│     - structuredClone                                   │
├─────────────────────────────────────────────────────────┤
│            Viper Extensions                             │
│     - File system APIs                                  │
│     - HTTP server (Viper.serve)                         │
│     - Web Workers & MessageChannel                      │
│     - WebSocket client                                  │
│     - Crypto API                                        │
│     - Process & Spawn                                   │
│     - Node.js built-in modules                          │
│       (assert, buffer, events, http, net, os, path,     │
│        querystring, stream, string_decoder, url,        │
│        util, zlib)                                      │
├─────────────────────────────────────────────────────────┤
│        Module Resolution (oxc_resolver)                 │
│     - node_modules support                              │
│     - package.json exports                              │
│     - CommonJS interop                                  │
├─────────────────────────────────────────────────────────┤
│       Package Manager (Orogene) [optional]              │
│     - npm registry compatible                           │
│     - Parallel downloads                                │
│     - Global cache with hardlinks                       │
└─────────────────────────────────────────────────────────┘
```

## Project Structure

```
viper/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── lib.rs           # Library exports
│   ├── runtime/         # Boa runtime & APIs
│   │   ├── mod.rs       # Runtime core
│   │   ├── worker.rs    # Web Workers
│   │   ├── websocket.rs # WebSocket client
│   │   ├── crypto.rs    # Crypto API
│   │   ├── process.rs   # Process object
│   │   ├── spawn.rs     # Spawn/exec
│   │   ├── server_api.rs # HTTP server
│   │   ├── assert.rs    # Assert module
│   │   ├── buffer.rs    # Buffer module
│   │   ├── events.rs    # EventEmitter
│   │   ├── http.rs      # HTTP module
│   │   ├── net.rs       # TCP networking
│   │   ├── os.rs        # OS utilities
│   │   ├── path.rs      # Path module
│   │   ├── querystring.rs # Query string parsing
│   │   ├── stream.rs    # Stream API
│   │   ├── string_decoder.rs # String decoder
│   │   ├── url.rs       # URL parsing
│   │   ├── util.rs      # Utilities
│   │   └── zlib.rs      # Compression
│   ├── transpiler/      # OXC TypeScript transpiler
│   ├── resolver/        # Module resolution
│   ├── bundler/         # JS bundling
│   ├── fs/              # File system APIs
│   ├── pm/              # Package manager (optional)
│   └── server/          # HTTP server (optional)
├── examples/            # Example TypeScript files
├── types/
│   └── viper.d.ts       # TypeScript definitions
└── Cargo.toml
```

## Dependencies

| Component | Library | Purpose |
|-----------|---------|---------|
| JS Engine | [Boa](https://github.com/boa-dev/boa) | ECMAScript execution |
| Transpiler | [OXC](https://github.com/oxc-project/oxc) | TypeScript/JSX parsing & transformation |
| Resolver | [oxc_resolver](https://crates.io/crates/oxc_resolver) | Node.js-compatible module resolution |
| HTTP | [Hyper](https://github.com/hyperium/hyper) | HTTP client/server |
| WebSocket | [tungstenite](https://github.com/snapview/tungstenite-rs) | WebSocket implementation |
| Package Manager | [Orogene](https://github.com/orogene/orogene) | npm-compatible package management |
| CLI | [clap](https://crates.io/crates/clap) | Command-line argument parsing |

## Performance

Viper leverages Rust's performance for:

- **Transpilation**: OXC is 50-100x faster than TypeScript's `tsc`
- **Startup**: No JIT warm-up, instant execution
- **Package Install**: Orogene is comparable to pnpm/Bun in speed

Note: **Runtime performance** is currently slower than Node.js/Bun because Boa is an interpreter without JIT compilation. This makes Viper best suited for CLI tools, scripts, and I/O-bound workloads rather than CPU-intensive computation.

## Limitations

- **Partial Node.js Compatibility** - Many Node.js built-in modules are now supported (assert, buffer, events, http, net, os, path, querystring, stream, string_decoder, url, util, zlib), but some advanced features may differ from Node.js behavior
- **No npm Lifecycle Scripts** - `postinstall` scripts don't run
- **No Full Node.js Compatibility** - This is not a drop-in Node.js replacement
- **Basic Bundler** - Built-in bundler is simple concatenation. For advanced bundling (tree-shaking, code-splitting), use external tools like esbuild or Rollup
- **Rolldown Not Supported** - The Rolldown bundler integration is disabled due to version incompatibilities between `rolldown_fs` and `oxc_resolver` v11.16

## Why Viper?

This project was created to explore:

1. **Rust + JavaScript Integration** - How well can Rust-based tools work together?
2. **Alternative Runtimes** - What would a non-V8 TypeScript runtime look like?
3. **AI-Assisted Development** - Can AI help build complex systems quickly?

It's **not** meant to replace Node.js, Deno, or Bun. It's an experiment and learning project.

## Acknowledgments

- [Boa](https://github.com/boa-dev/boa) - The incredible Rust JavaScript engine
- [OXC](https://github.com/oxc-project/oxc) - Lightning-fast JavaScript toolchain
- [Orogene](https://github.com/orogene/orogene) - Fast npm-compatible package manager
- [Claude](https://anthropic.com) - AI pair programming assistant that helped build this

## License

MIT
