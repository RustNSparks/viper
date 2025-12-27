// File System API demonstration

console.log("=== File System Demo ===\n");

const testDir = "./test-output";
const testFile = `${testDir}/hello.txt`;
const jsonFile = `${testDir}/data.json`;

// Create directory
console.log("1. Creating test directory...");
await mkdir(testDir, { recursive: true });
console.log(`Created: ${testDir}\n`);

// Write text file
console.log("2. Writing text file...");
await write(testFile, "Hello from Viper!\nThis is a test file.");
console.log(`Written: ${testFile}\n`);

// Write JSON file
console.log("3. Writing JSON file...");
const data = {
  name: "Viper",
  version: "1.0.0",
  features: ["typescript", "async", "fetch"],
};
await write(jsonFile, JSON.stringify(data, null, 2));
console.log(`Written: ${jsonFile}\n`);

// Check if files exist using file() API
console.log("4. Checking file existence...");
console.log(`${testFile} exists: ${await file(testFile).exists()}`);
console.log(`${jsonFile} exists: ${await file(jsonFile).exists()}`);
console.log(
  `./nonexistent.txt exists: ${await file("./nonexistent.txt").exists()}\n`,
);

// Read text file using file() API
console.log("5. Reading text file...");
const content = await file(testFile).text();
console.log(`Content:\n${content}\n`);

// Read JSON file using file() API
console.log("6. Reading JSON with file() API...");
const jsonContent = await file(jsonFile).text();
const parsed = JSON.parse(jsonContent);
console.log("Parsed JSON:", parsed, "\n");

// Get file size
console.log("7. File size...");
const size = await file(testFile).size();
console.log(`Size of ${testFile}: ${size} bytes\n`);

// Write and read binary-like data
console.log("8. Writing more files...");
await write("./test-output/numbers.txt", "1\n2\n3\n4\n5");
const numbers = await file("./test-output/numbers.txt").text();
console.log(
  "Numbers file content:",
  numbers.trim().split("\n").join(", "),
  "\n",
);

// Cleanup
console.log("9. Cleaning up test directory...");
await rmdir(testDir, { recursive: true });
console.log(`Removed: ${testDir}\n`);

console.log("=== Demo Complete ===");
export default {};
