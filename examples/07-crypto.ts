// Crypto API demonstration

async function main() {
  console.log("=== Crypto API Demo ===\n");

  // Generate UUID
  console.log("1. Generate UUID:");
  const uuid = crypto.randomUUID();
  console.log(`UUID: ${uuid}\n`);

  // Random bytes
  console.log("2. Random bytes:");
  const randomBytes = crypto.randomBytes(16);
  const hexBytes = Array.from(randomBytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
  console.log(`Random bytes (hex): ${hexBytes}\n`);

  // getRandomValues for typed arrays
  console.log("3. getRandomValues:");
  const array = new Uint32Array(4);
  crypto.getRandomValues(array);
  console.log(`Random Uint32Array: [${Array.from(array).join(", ")}]\n`);

  // SHA-256 hash
  console.log("4. SHA-256 hash:");
  const message = "Hello, Viper!";
  const encoder = new TextEncoder();
  const data = encoder.encode(message);
  const hashBuffer = await crypto.subtle.digest("SHA-256", data);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  const hashHex = hashArray.map((b) => b.toString(16).padStart(2, "0")).join("");
  console.log(`Message: "${message}"`);
  console.log(`SHA-256: ${hashHex}\n`);

  // SHA-512 hash
  console.log("5. SHA-512 hash:");
  const hash512 = await crypto.subtle.digest("SHA-512", data);
  const hash512Hex = Array.from(new Uint8Array(hash512))
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
  console.log(`SHA-512: ${hash512Hex.slice(0, 64)}...`);
  console.log(`(${hash512Hex.length / 2} bytes)\n`);

  // Multiple random UUIDs
  console.log("6. Multiple UUIDs:");
  for (let i = 0; i < 3; i++) {
    console.log(`  ${crypto.randomUUID()}`);
  }

  console.log("\n=== Demo Complete ===");
}

main();
