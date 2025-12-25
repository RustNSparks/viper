// Bundled by Viper
// Format: Esm
// Entry points:
//   - .\examples\fs_test.ts


// === .\examples\fs_test.ts ===
// Simple fs test without top-level await
console.log("=== File System Test ===\n");
// Test async function
async function testFileSystem() {
	console.log("1. Writing file...");
	const content = "Hello from Viper!\nThis is a test file.";
	await write("test.txt", content);
	console.log("   ✓ File written\n");
	console.log("2. Reading file...");
	const f = file("test.txt");
	const text = await f.text();
	console.log("   Content:", text);
	console.log("   ✓ File read\n");
	console.log("3. Checking file properties...");
	console.log("   Path:", f.path);
	console.log("   Type:", f.type);
	console.log("   Exists:", await f.exists());
	console.log("   Size:", await f.size(), "bytes\n");
	console.log("4. Writing JSON...");
	const data = {
		name: "Viper",
		version: "0.1.0"
	};
	await write("data.json", JSON.stringify(data, null, 2));
	console.log("   ✓ JSON written\n");
	console.log("5. Reading JSON...");
	const jsonFile = file("data.json");
	const jsonData = await jsonFile.json();
	console.log("   Data:", jsonData);
	console.log("   ✓ JSON parsed\n");
	console.log("6. Using FileSink...");
	const sink = file("sink.txt").writer({ highWaterMark: 64 });
	sink.write("Line 1\n");
	sink.write("Line 2\n");
	sink.write("Line 3\n");
	await sink.end();
	console.log("   ✓ FileSink complete\n");
	console.log("7. Copying file...");
	await write(file("copy.txt"), file("test.txt"));
	console.log("   ✓ File copied\n");
	console.log("   ✓ Cleanup complete\n");
	console.log("=== All tests passed! ===");
}
// Run the test
testFileSystem().catch((err) => {
	console.error("Test failed:", err);
});

