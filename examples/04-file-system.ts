// File System API demonstration

async function main() {
  console.log("=== File System Demo ===\n");

  const testDir = "./test-output";
  const testFile = `${testDir}/hello.txt`;
  const jsonFile = `${testDir}/data.json`;

  // Create directory
  console.log("1. Creating directory...");
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

  // Check if files exist
  console.log("4. Checking file existence...");
  console.log(`${testFile} exists: ${await exists(testFile)}`);
  console.log(`${jsonFile} exists: ${await exists(jsonFile)}`);
  console.log(`./nonexistent.txt exists: ${await exists("./nonexistent.txt")}\n`);

  // Read text file
  console.log("5. Reading text file...");
  const content = await readFile(testFile);
  console.log(`Content:\n${content}\n`);

  // Read JSON file using file() API
  console.log("6. Reading JSON with file() API...");
  const jsonContent = await file(jsonFile).text();
  const parsed = JSON.parse(jsonContent);
  console.log("Parsed JSON:", parsed, "\n");

  // Get file stats
  console.log("7. File statistics...");
  const stats = await stat(testFile);
  console.log(`Is file: ${stats.isFile}`);
  console.log(`Is directory: ${stats.isDirectory}`);
  console.log(`Size: ${stats.size} bytes\n`);

  // List directory contents
  console.log("8. Directory listing...");
  const files = await readDir(testDir);
  console.log(`Files in ${testDir}:`, files, "\n");

  // Read as ArrayBuffer
  console.log("9. Reading as ArrayBuffer...");
  const buffer = await file(testFile).arrayBuffer();
  console.log(`Buffer size: ${buffer.byteLength} bytes`);
  const bytes = new Uint8Array(buffer);
  console.log(`First 10 bytes: [${Array.from(bytes.slice(0, 10)).join(", ")}]\n`);

  console.log("=== Demo Complete ===");
}

main();
