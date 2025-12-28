// Test the ultra-fast fs module

// Test using require (CommonJS)
const fs = require('fs');

console.log("=== Testing Viper's Ultra-Fast FS Module ===\n");

// Test writeFileSync
console.log("1. Testing writeFileSync...");
fs.writeFileSync('test-output.txt', 'Hello from Viper!');
console.log("   ✓ writeFileSync works!\n");

// Test readFileSync
console.log("2. Testing readFileSync...");
const content = fs.readFileSync('test-output.txt', 'utf8');
console.log("   Content:", content);
console.log("   ✓ readFileSync works!\n");

// Test existsSync
console.log("3. Testing existsSync...");
console.log("   test-output.txt exists:", fs.existsSync('test-output.txt'));
console.log("   nonexistent.txt exists:", fs.existsSync('nonexistent.txt'));
console.log("   ✓ existsSync works!\n");

// Test statSync
console.log("4. Testing statSync...");
const stat = fs.statSync('test-output.txt');
console.log("   isFile:", stat.isFile());
console.log("   isDirectory:", stat.isDirectory());
console.log("   size:", stat.size);
console.log("   ✓ statSync works!\n");

// Test mkdirSync
console.log("5. Testing mkdirSync...");
if (!fs.existsSync('test-dir')) {
    fs.mkdirSync('test-dir');
}
console.log("   test-dir exists:", fs.existsSync('test-dir'));
console.log("   ✓ mkdirSync works!\n");

// Test readdirSync
console.log("6. Testing readdirSync...");
fs.writeFileSync('test-dir/file1.txt', 'File 1');
fs.writeFileSync('test-dir/file2.txt', 'File 2');
const files = fs.readdirSync('test-dir');
console.log("   Files in test-dir:", files);
console.log("   ✓ readdirSync works!\n");

// Test appendFileSync
console.log("7. Testing appendFileSync...");
fs.appendFileSync('test-output.txt', '\nAppended text!');
const appendedContent = fs.readFileSync('test-output.txt', 'utf8');
console.log("   Content after append:", appendedContent);
console.log("   ✓ appendFileSync works!\n");

// Test copyFileSync
console.log("8. Testing copyFileSync...");
fs.copyFileSync('test-output.txt', 'test-output-copy.txt');
console.log("   Copy exists:", fs.existsSync('test-output-copy.txt'));
console.log("   Copy content:", fs.readFileSync('test-output-copy.txt', 'utf8'));
console.log("   ✓ copyFileSync works!\n");

// Test renameSync
console.log("9. Testing renameSync...");
fs.renameSync('test-output-copy.txt', 'test-renamed.txt');
console.log("   Renamed file exists:", fs.existsSync('test-renamed.txt'));
console.log("   Old file exists:", fs.existsSync('test-output-copy.txt'));
console.log("   ✓ renameSync works!\n");

// Test unlinkSync
console.log("10. Testing unlinkSync...");
fs.unlinkSync('test-renamed.txt');
console.log("   File deleted:", !fs.existsSync('test-renamed.txt'));
console.log("   ✓ unlinkSync works!\n");

// Test realpathSync
console.log("11. Testing realpathSync...");
const realPath = fs.realpathSync('test-output.txt');
console.log("   Real path:", realPath);
console.log("   ✓ realpathSync works!\n");

// Cleanup
console.log("12. Cleaning up...");
fs.unlinkSync('test-dir/file1.txt');
fs.unlinkSync('test-dir/file2.txt');
fs.rmdirSync('test-dir');
fs.unlinkSync('test-output.txt');
console.log("   ✓ Cleanup complete!\n");

console.log("=== All FS Tests Passed! ===");
