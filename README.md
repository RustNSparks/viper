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

### Networking

- **HTTP Client** - Full Fetch API support
- **HTTP Server** - `Viper.serve()` for creating HTTP servers (requires `--features server`)
- **WebSocket Client** - Full WebSocket API with binary and text message support

### Workers

- **Web Workers** - Multi-threaded execution with `new Worker()`
- **Message Passing** - `postMessage()` / `onmessage` with structured clone
- **MessageChannel** - `MessageChannel` and `MessagePort` for bidirectional communication
- **Transferables** - `ArrayBuffer` transfer support

### File System

- **Bun-style API** - `file()`, `write()`, `readFile()`, `exists()`, `mkdir()`, `readDir()`, `stat()`

### Process & System

- **Process API** - `process.argv`, `process.env`, `process.cwd()`, `process.exit()`, `process.platform`, `process.arch`
- **Spawn/Exec** - `Viper.spawn()`, `Viper.exec()` for running external commands
- **Path Module** - Node.js compatible `path` module (`join`, `resolve`, `dirname`, `basename`, etc.)

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

### Path Module

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
│     - Path module                                       │
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
│   │   ├── path.rs      # Path module
│   │   └── server_api.rs # HTTP server
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

- **No Node.js Built-in Modules** - `fs`, `http`, `events`, etc. are not implemented (use Viper's APIs instead)
- **No npm Lifecycle Scripts** - `postinstall` scripts don't run
- **No Full Node.js Compatibility** - This is not a drop-in Node.js replacement

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
