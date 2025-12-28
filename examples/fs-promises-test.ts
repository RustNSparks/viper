// Test fs/promises API (async operations)

const fsPromises = require('fs/promises');
const fs = require('fs');

console.log("=== Testing fs/promises API ===\n");

async function runTests() {
    // Test writeFile
    console.log("1. Testing fs.promises.writeFile...");
    await fsPromises.writeFile('async-test.txt', 'Hello from async!');
    console.log("   Written to async-test.txt");
    console.log("   ✓ writeFile works!\n");

    // Test readFile
    console.log("2. Testing fs.promises.readFile...");
    const content = await fsPromises.readFile('async-test.txt', 'utf8');
    console.log("   Content:", content);
    console.log("   ✓ readFile works!\n");

    // Test stat
    console.log("3. Testing fs.promises.stat...");
    const stat = await fsPromises.stat('async-test.txt');
    console.log("   isFile:", stat.isFile());
    console.log("   size:", stat.size);
    console.log("   ✓ stat works!\n");

    // Test mkdir
    console.log("4. Testing fs.promises.mkdir...");
    if (!fs.existsSync('async-dir')) {
        await fsPromises.mkdir('async-dir');
    }
    console.log("   Directory created:", fs.existsSync('async-dir'));
    console.log("   ✓ mkdir works!\n");

    // Test readdir
    console.log("5. Testing fs.promises.readdir...");
    await fsPromises.writeFile('async-dir/file1.txt', 'File 1');
    await fsPromises.writeFile('async-dir/file2.txt', 'File 2');
    const files = await fsPromises.readdir('async-dir');
    console.log("   Files:", files);
    console.log("   ✓ readdir works!\n");

    // Test appendFile
    console.log("6. Testing fs.promises.appendFile...");
    await fsPromises.appendFile('async-test.txt', '\nMore content!');
    const updated = await fsPromises.readFile('async-test.txt', 'utf8');
    console.log("   Updated content:", updated);
    console.log("   ✓ appendFile works!\n");

    // Test copyFile
    console.log("7. Testing fs.promises.copyFile...");
    await fsPromises.copyFile('async-test.txt', 'async-copy.txt');
    console.log("   Copy exists:", fs.existsSync('async-copy.txt'));
    console.log("   ✓ copyFile works!\n");

    // Test rename
    console.log("8. Testing fs.promises.rename...");
    await fsPromises.rename('async-copy.txt', 'async-renamed.txt');
    console.log("   Renamed exists:", fs.existsSync('async-renamed.txt'));
    console.log("   Old exists:", fs.existsSync('async-copy.txt'));
    console.log("   ✓ rename works!\n");

    // Test unlink
    console.log("9. Testing fs.promises.unlink...");
    await fsPromises.unlink('async-renamed.txt');
    console.log("   Deleted:", !fs.existsSync('async-renamed.txt'));
    console.log("   ✓ unlink works!\n");

    // Test access
    console.log("10. Testing fs.promises.access...");
    try {
        await fsPromises.access('async-test.txt');
        console.log("   File is accessible");
        console.log("   ✓ access works!\n");
    } catch (e) {
        console.log("   Error:", e.message);
    }

    // Test realpath
    console.log("11. Testing fs.promises.realpath...");
    const realPath = await fsPromises.realpath('async-test.txt');
    console.log("   Real path:", realPath);
    console.log("   ✓ realpath works!\n");

    // Cleanup
    console.log("12. Cleaning up...");
    await fsPromises.unlink('async-dir/file1.txt');
    await fsPromises.unlink('async-dir/file2.txt');
    await fsPromises.rmdir('async-dir');
    await fsPromises.unlink('async-test.txt');
    console.log("   ✓ Cleanup complete!\n");

    console.log("=== All fs/promises Tests Passed! ===");
}

runTests().catch(err => {
    console.error("Test failed:", err);
});
