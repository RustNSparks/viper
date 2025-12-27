// Crypto API demonstration

console.log("=== Crypto API Demo ===\n");

// Generate UUID
console.log("1. Generate UUID:");
const uuid = crypto.randomUUID();
console.log(`   UUID: ${uuid}\n`);

// Random bytes
console.log("2. Random bytes:");
const randomBytes = crypto.randomBytes(16);
const hexBytes = Array.from(randomBytes)
  .map((b) => b.toString(16).padStart(2, "0"))
  .join("");
console.log(`   Random bytes (hex): ${hexBytes}`);
console.log(`   Length: ${randomBytes.length} bytes\n`);

// getRandomValues for typed arrays
console.log("3. getRandomValues with Uint8Array:");
const uint8 = new Uint8Array(8);
crypto.getRandomValues(uint8);
console.log(`   Uint8Array: [${Array.from(uint8).join(", ")}]\n`);

console.log("4. getRandomValues with Uint32Array:");
const uint32 = new Uint32Array(4);
crypto.getRandomValues(uint32);
console.log(`   Uint32Array: [${Array.from(uint32).join(", ")}]\n`);

console.log("5. getRandomValues with Int16Array:");
const int16 = new Int16Array(4);
crypto.getRandomValues(int16);
console.log(`   Int16Array: [${Array.from(int16).join(", ")}]\n`);

// Generate multiple UUIDs
console.log("6. Multiple UUIDs:");
for (let i = 0; i < 5; i++) {
  console.log(`   ${i + 1}. ${crypto.randomUUID()}`);
}
console.log("");

// Generate random hex string (like a token)
console.log("7. Random token generation:");
const tokenBytes = crypto.randomBytes(32);
const token = Array.from(tokenBytes)
  .map((b) => b.toString(16).padStart(2, "0"))
  .join("");
console.log(`   Token (64 hex chars): ${token}\n`);

// Random values for different use cases
console.log("8. Use cases:");

// Random integer in range
const randomInt = (min: number, max: number): number => {
  const range = max - min;
  const bytes = new Uint32Array(1);
  crypto.getRandomValues(bytes);
  return min + (bytes[0] % range);
};
console.log(`   Random int (1-100): ${randomInt(1, 100)}`);
console.log(`   Random int (1-100): ${randomInt(1, 100)}`);
console.log(`   Random int (1-100): ${randomInt(1, 100)}`);

// Random boolean
const randomBool = (): boolean => {
  const bytes = new Uint8Array(1);
  crypto.getRandomValues(bytes);
  return bytes[0] % 2 === 0;
};
console.log(`   Random bool: ${randomBool()}`);
console.log(`   Random bool: ${randomBool()}`);

// Random choice from array
const randomChoice = <T>(arr: T[]): T => {
  const bytes = new Uint32Array(1);
  crypto.getRandomValues(bytes);
  return arr[bytes[0] % arr.length];
};
const colors = ["red", "green", "blue", "yellow", "purple"];
console.log(`   Random color: ${randomChoice(colors)}`);
console.log(`   Random color: ${randomChoice(colors)}`);

console.log("\n=== Demo Complete ===");
