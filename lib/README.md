# Node.js Module Library

This directory contains Node.js built-in module implementations for Viper.

## Strategy

Viper takes a hybrid approach to Node.js compatibility, inspired by Bun:

### 1. **Rust-Based Core APIs**
Performance-critical primitives are implemented in Rust:
- File system operations (`fs` module core)
- Networking (`net`, `http` low-level)
- Compression (`zlib`)
- Crypto operations
- Buffer manipulation
- Process APIs

### 2. **JavaScript Wrappers**
High-level JavaScript logic wraps Rust primitives for compatibility:
- Event emitters (`events`)
- Streams (`stream`)
- HTTP client/server logic
- Utilities (`util`, `path`)
- Query string parsing

### 3. **Node.js-Inspired Implementations**
We reference Node.js source code (MIT licensed) and reimplement in TypeScript/JavaScript:
- Simplify by removing `internal/*` dependencies
- Adapt to Viper's runtime APIs
- Keep the same public interfaces
- Maintain Node.js MIT license headers

## How Bun Does It

Bun maintains their Node.js compatibility layer in `src/js/node/`:
- **TypeScript implementations** of Node.js modules
- **Reference Node.js source** but reimplement from scratch
- **Use internal Bun APIs** (`$debug`, `$ERR_*`, `$isPromise`, etc.)
- **Keep MIT licenses** from Node.js

Example from Bun's `events.ts`:
```typescript
// Reimplementation of https://nodejs.org/api/events.html
// Reference: https://github.com/nodejs/node/blob/main/lib/events.js
// [MIT License from Node.js]
```

## Our Approach

For Viper, we should:

1. **Keep existing Rust implementations** for performance-critical modules
2. **Add JavaScript layers** for complex logic that doesn't need Rust performance
3. **Reference Node.js and Bun** implementations for compatibility
4. **Use Boa's capabilities** instead of creating custom internal APIs

## Future Work

### Modules to Consider Reimplementing in JavaScript

Current modules that could benefit from Node.js/Bun-style JavaScript implementations:

- **`events`** - Complex event emitter logic (currently custom JS)
- **`stream`** - Stream implementation (currently custom JS)
- **`assert`** - Full assertion suite (currently custom JS with basic deepEqual)
- **`util`** - Utility functions (currently Rust-based)
- **`querystring`** - Query parsing (currently Rust-based)
- **`url`** - URL parsing (currently Rust-based)

### Approach for Each Module

1. **Start with Bun's implementation** as a reference (it's more standalone)
2. **Remove Bun-specific APIs** (`$debug`, `$newCppFunction`, etc.)
3. **Replace with Viper's Rust APIs** where needed
4. **Test against Node.js behavior** to ensure compatibility
5. **Keep performance-critical parts in Rust**

## Example: Events Module

### Current Approach (Custom JS)
```javascript
// src/runtime/events_module.js
class EventEmitter {
  // Custom implementation
}
```

### Better Approach (Inspired by Bun/Node.js)
```javascript
// lib/events.js
// Reimplementation based on Node.js events module
// Reference: https://github.com/nodejs/node/blob/main/lib/events.js
// [Include Node.js MIT License]

class EventEmitter {
  // Implementation closely matching Node.js behavior
  // Use Viper's Rust APIs where beneficial
}
```

## Benefits

1. ✅ **Better Node.js compatibility** - Match Node.js behavior more closely
2. ✅ **Well-tested implementations** - Node.js code is battle-tested
3. ✅ **Less maintenance** - Follow Node.js updates and fixes
4. ✅ **Performance** - Keep Rust for critical paths
5. ✅ **Clear licensing** - MIT license from Node.js is compatible

## Deep Equality Implementation

Currently, `assert.deepEqual` is implemented in JavaScript. Consider:

**Option A: Keep JavaScript** (simplicity)
```javascript
function isDeepEqual(a, b, strict) {
  // Existing JS implementation
}
```

**Option B: Rust implementation** (performance)
```rust
// Implement in src/runtime/assert.rs
pub fn is_deep_equal(a: JsValue, b: JsValue, strict: bool) -> bool {
  // Rust-based deep comparison
}
```

For `deepEqual`, **Option A (JavaScript) is probably better** because:
- It's not performance-critical
- Easier to maintain and debug
- More flexible for edge cases

## License Notes

All Node.js-inspired code must include the Node.js MIT license header:

```
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
```

## References

- [Node.js GitHub - lib/ directory](https://github.com/nodejs/node/tree/main/lib)
- [Bun GitHub - src/js/node/](https://github.com/oven-sh/bun/tree/main/src/js/node)
- [Node.js Events Module](https://github.com/nodejs/node/blob/main/lib/events.js)
- [Bun Events Implementation](https://github.com/oven-sh/bun/blob/main/src/js/node/events.ts)