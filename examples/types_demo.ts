// TypeScript Type Definitions Demo
// This file demonstrates Viper's TypeScript type definitions
// You should get full autocompletion and type checking in VS Code!

// ============================================================================
// Runtime Information
// ============================================================================

console.log(`Running on ${__VIPER_RUNTIME__} v${__VIPER_VERSION__}`);

// ============================================================================
// File System API with Type Safety
// ============================================================================

// The `file()` function returns a ViperFile with full type definitions
const configFile = file("package.json");

// TypeScript knows about all the methods on ViperFile
const content = await configFile.text();
const json = await configFile.json<{ name: string; version: string }>();
const exists = await configFile.exists();
const size = await configFile.size();

console.log("Config:", json);
console.log("Exists:", exists);
console.log("Size:", size, "bytes");

// ============================================================================
// Type-safe write operations
// ============================================================================

// Write string to file
await write("output.txt", "Hello, Viper!");

// Write JSON with type inference
const data = {
  name: "Viper",
  version: "0.1.0",
  features: ["TypeScript", "JSX", "File System"],
};
await write("data.json", JSON.stringify(data, null, 2));

// Copy files (both arguments accept string | ViperFile)
await write(file("backup.json"), file("data.json"));

// ============================================================================
// FileSink with Type Safety
// ============================================================================

const outputFile = file("stream.txt");
const writer = outputFile.writer({
  highWaterMark: 8192, // TypeScript knows this is optional and expects a number
});

// TypeScript knows write() accepts string | ArrayBuffer | ArrayBufferView
writer.write("Line 1\n");
writer.write(new TextEncoder().encode("Line 2\n"));

await writer.flush();
const bytesWritten = await writer.end();
console.log(`Wrote ${bytesWritten} bytes`);

// ============================================================================
// Standard Web APIs
// ============================================================================

// Console API (fully typed)
console.log("Regular log");
console.error("Error message");
console.warn("Warning");
console.time("timer");
console.timeEnd("timer");

// Timers (returns number)
const timeoutId = setTimeout(() => {
  console.log("Timeout!");
}, 100);
clearTimeout(timeoutId);

// URL API
const url = new URL("https://example.com/path?query=value");
console.log("Hostname:", url.hostname);
console.log("Pathname:", url.pathname);
// Note: URLSearchParams not yet implemented in Boa
// console.log("Search:", url.searchParams.get("query"));

// Text Encoding
const encoder = new TextEncoder();
const decoder = new TextDecoder();
const encoded = encoder.encode("Hello");
const decoded = decoder.decode(encoded);
console.log("Encoded/Decoded:", decoded);

// Structured Clone
const original = { nested: { data: [1, 2, 3] } };
const cloned = structuredClone(original);
console.log("Cloned:", cloned);

// ============================================================================
// Microtask
// ============================================================================

queueMicrotask(() => {
  console.log("Microtask executed");
});

// ============================================================================
// Type Checking Examples
// ============================================================================

// ✓ Valid: string path
await write("valid.txt", "content");

// ✓ Valid: ViperFile
await write(file("valid.txt"), "content");

// ✗ TypeScript Error: number is not assignable to string | ViperFile
// await write(123, "content");

// ✓ Valid: json() returns Promise<any> by default
const anyJson = await file("data.json").json();

// ✓ Valid: json() with generic type
interface Config {
  name: string;
  version: string;
}
const typedJson = await file("data.json").json<Config>();
console.log(typedJson.name); // TypeScript knows this is a string

// Cleanup
await file("output.txt").delete();
await file("data.json").delete();
await file("backup.json").delete();
await file("stream.txt").delete();
console.log(process.env);

console.log("\n✓ All type-safe operations completed!");
export {};
