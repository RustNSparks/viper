// High-performance file system demo

console.log("=== Viper File System Demo ===\n");

// Test 1: Write a file
console.log("Writing test file...");
const content = "Hello from Viper!\nThis is a high-performance file system.\n";
await write("test_output.txt", content);
console.log("✓ File written\n");

// Test 2: Read the file back
console.log("Reading file...");
const fileRef = file("test_output.txt");
const text = await fileRef.text();
console.log("File contents:", text);
console.log("✓ File read\n");

// Test 3: Check file properties
console.log("File properties:");
console.log("  - Path:", fileRef.path);
console.log("  - Type:", fileRef.type);
console.log("  - Size:", await fileRef.size(), "bytes");
console.log("  - Exists:", await fileRef.exists());

// Test 4: Write JSON
console.log("\nWriting JSON file...");
const data = {
  name: "Viper",
  version: "0.1.0",
  features: ["TypeScript", "TSX", "FS"],
};
await write("data.json", JSON.stringify(data, null, 2));
console.log("✓ JSON written\n");

// Test 5: Read and parse JSON
console.log("Reading JSON...");
const jsonFile = file("data.json");
const jsonData = await jsonFile.json();
console.log("Parsed data:", jsonData);

// Test 6: FileSink (incremental writing)
console.log("\nUsing FileSink for incremental writing...");
const largeFile = file("large.txt");
const writer = largeFile.writer({ highWaterMark: 1024 });

for (let i = 0; i < 10; i++) {
  writer.write(`Line ${i}: ${"=".repeat(50)}\n`);
}

const bytesWritten = await writer.end();
console.log(`✓ Written ${bytesWritten} bytes incrementally\n`);

// Test 7: Copy a file
console.log("Copying file...");
await write(file("copy.txt"), file("test_output.txt"));
console.log("✓ File copied\n");

// Cleanup
console.log("Cleaning up...");
await file("test_output.txt").delete();
await file("data.json").delete();
await file("large.txt").delete();
await file("copy.txt").delete();
console.log("✓ Cleanup complete\n");

console.log("=== All tests passed! ===");
export default {};
