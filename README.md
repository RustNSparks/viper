# Viper

A fast TypeScript runtime written in Rust, powered by [Boa](https://github.com/boa-dev/boa) (JS engine) and [OXC](https://github.com/oxc-project/oxc) (TypeScript transpiler).

> **Note:** This is an experimental project built with extensive AI assistance (Claude) to explore what's possible when combining Rust's performance with modern JavaScript tooling. It's a fun experiment to see how far we can push a custom TypeScript runtime!

## Features

### What Works

- **TypeScript/TSX Execution** - Run `.ts` and `.tsx` files directly without compilation step
- **Fast Transpilation** - OXC-powered TypeScript to JavaScript conversion (50-100x faster than tsc)
- **ES Modules** - Full ESM support with `import`/`export`
- **CommonJS Interop** - Automatic CJS to ESM wrapping for npm packages
- **JSX/TSX Support** - Built-in JSX runtime with `renderToString()`
- **Web APIs** - `console`, `fetch`, `URL`, `TextEncoder/Decoder`, `setTimeout/setInterval`
- **File System** - Bun-style `file()`, `write()` APIs
- **Async/Await** - Full Promise support with event loop
- **npm Packages** - Works with pure ESM packages like `date-fns`
- **REPL** - Interactive TypeScript shell
- **Bundler** - Basic bundling support

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

### What Doesn't Work (Yet)

- **Node.js Built-ins** - No `fs`, `path`, `http`, `events` modules (Express won't work)
- **Named ESM Exports** - Some complex re-exports don't resolve (use default exports as workaround)
- **npm Scripts** - `postinstall` and lifecycle scripts not supported
- **Full Node.js Compatibility** - This is not a Node.js replacement

## Installation

### From Source

```bash
# Clone the repo
git clone https://github.com/user/viper
cd viper

# Build release binary
cargo build --release

# Optional: with package manager
cargo build --release --features pm

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

### HTTP Server (requires `--features server`)

```bash
cargo build --release --features server
viper serve --port 3000
```

## Examples

### Hello World

```typescript
// hello.ts
const message: string = "Hello from Viper!";
console.log(message);
```

```bash
viper run hello.ts
```

### Async/Await

```typescript
// async.ts
async function fetchData() {
  const response = await fetch("https://api.github.com/users/octocat");
  const data = await response.json();
  console.log(data.login);
}

fetchData();
```

### File System

```typescript
// fs.ts
const content = await file("./data.txt").text();
console.log(content);

await write("./output.txt", "Hello, World!");
```

### Using npm Packages

```typescript
// Works with ESM packages like date-fns
import formatModule from "date-fns/format";
import addDaysModule from "date-fns/addDays";

const format = formatModule.format;
const addDays = addDaysModule.addDays;

const today = new Date();
console.log(format(today, "yyyy-MM-dd"));
console.log(format(addDays(today, 7), "yyyy-MM-dd"));
```

### JSX/TSX

```tsx
// app.tsx
function App() {
  return (
    <div>
      <h1>Hello from Viper!</h1>
      <p>TSX just works.</p>
    </div>
  );
}

console.log(renderToString(<App />));
```

### Timers

```typescript
// timers.ts
console.log("Starting...");

setTimeout(() => {
  console.log("After 1 second");
}, 1000);

let count = 0;
const interval = setInterval(() => {
  count++;
  console.log(`Tick ${count}`);
  if (count >= 3) {
    clearInterval(interval);
  }
}, 500);
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
│     - console, fetch, URL                               │
│     - setTimeout, setInterval                           │
│     - TextEncoder, TextDecoder                          │
├─────────────────────────────────────────────────────────┤
│            Viper Extensions                             │
│     - File system APIs                                  │
│     - process.env, process.argv                         │
│     - crypto.randomUUID()                               │
│     - WebSocket (client)                                │
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

## Dependencies

| Component | Library | Purpose |
|-----------|---------|---------|
| JS Engine | [Boa](https://github.com/boa-dev/boa) | ECMAScript execution |
| Transpiler | [OXC](https://github.com/oxc-project/oxc) | TypeScript/JSX parsing & transformation |
| Resolver | [oxc_resolver](https://crates.io/crates/oxc_resolver) | Node.js-compatible module resolution |
| Bundler | [Rolldown](https://github.com/rolldown/rolldown) | Rust-based Rollup-compatible bundler (planned) |
| Package Manager | [Orogene](https://github.com/orogene/orogene) | npm-compatible package management |
| CLI | [clap](https://crates.io/crates/clap) | Command-line argument parsing |
| HTTP Server | [Axum](https://github.com/tokio-rs/axum) | HTTP server (optional) |

## Performance

Viper leverages Rust's performance for:

- **Transpilation**: OXC is 50-100x faster than TypeScript's `tsc`
- **Startup**: No JIT warm-up like V8, instant execution
- **Package Install**: Orogene is comparable to pnpm/Bun in speed

However, **runtime performance** is currently slower than Node.js/Bun because Boa is an interpreter without JIT compilation.

## Project Structure

```
viper/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── lib.rs           # Library exports
│   ├── runtime/         # Boa runtime & APIs
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

## Why Viper?

This project was created to explore:

1. **Rust + JavaScript Integration** - How well can Rust-based tools work together?
2. **Alternative Runtimes** - What would a non-V8 TypeScript runtime look like?
3. **AI-Assisted Development** - Can AI help build complex systems quickly?

It's **not** meant to replace Node.js, Deno, or Bun. It's an experiment and learning project.

## Acknowledgments

- [Boa](https://github.com/boa-dev/boa) - The incredible Rust JavaScript engine
- [OXC](https://github.com/oxc-project/oxc) - Lightning-fast JavaScript toolchain
- [Rolldown](https://github.com/rolldown/rolldown) - Rust-based Rollup-compatible bundler
- [Orogene](https://github.com/orogene/orogene) - Fast npm-compatible package manager
- [Claude](https://anthropic.com) - AI pair programming assistant that helped build this

## License

MIT
