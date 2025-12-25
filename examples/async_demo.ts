// Async/Await and Timer Demo for Viper TypeScript Runtime

console.log("=== Viper TypeScript Runtime Demo ===");
// console.log("Runtime:", __VIPER_RUNTIME__);
// console.log("Version:", __VIPER_VERSION__);
console.log("");

// Test Promise support
console.log("1. Testing Promises...");
const promise = new Promise<string>((resolve) => {
  resolve("Promise resolved!");
});

promise.then((msg: string) => {
  console.log("   ", msg);
});

// Test URL API
console.log("");
console.log("2. Testing URL API...");
const url = new URL("https://example.com/path?name=viper&version=1");
console.log("   Host:", url.hostname);
console.log("   Path:", url.pathname);
console.log("   Search:", url.search);

// Test TextEncoder/TextDecoder
console.log("");
console.log("3. Testing TextEncoder/TextDecoder...");
const encoder = new TextEncoder();
const decoder = new TextDecoder();
const encoded = encoder.encode("Hello, Viper!");
console.log("   Encoded length:", encoded.length, "bytes");
const decoded = decoder.decode(encoded);
console.log("   Decoded:", decoded);

// Test structuredClone
console.log("");
console.log("4. Testing structuredClone...");
const original = { name: "Viper", features: ["TypeScript", "Async", "Fast"] };
const cloned = structuredClone(original);
cloned.name = "Cloned";
console.log("   Original:", original.name);
console.log("   Cloned:", cloned.name);

// Test setTimeout
console.log("");
console.log("5. Testing setTimeout...");
console.log("   Scheduling callback for 100ms...");

setTimeout(() => {
  console.log("   Timer fired! (10000ms)");
}, 10000);

setTimeout(() => {
  console.log("   Second timer fired! (200ms)");
  console.log("");
  console.log("=== Demo Complete ===");
}, 200);

// Test queueMicrotask
console.log("");
console.log("6. Testing queueMicrotask...");
queueMicrotask(() => {
  console.log("   Microtask executed!");
});

console.log("   (Microtask will run before timers)");

// Final synchronous output
console.log("");
console.log("Synchronous code complete. Waiting for async operations...");
