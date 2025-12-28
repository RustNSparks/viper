// Test the zlib module

console.log("=== Zlib Module Test ===\n");

// Test data
const testString = "Hello, World! This is a test of the Viper zlib compression module. ".repeat(10);
const testBuffer = new TextEncoder().encode(testString);

console.log("Original size:", testBuffer.length, "bytes");

// Test gzip
console.log("\n--- Gzip ---");
const gzipped = zlib.gzipSync(testBuffer);
console.log("Gzipped size:", gzipped.length, "bytes");
console.log("Compression ratio:", ((1 - gzipped.length / testBuffer.length) * 100).toFixed(1) + "%");

const gunzipped = zlib.gunzipSync(gzipped);
console.log("Gunzipped size:", gunzipped.length, "bytes");
const gunzippedStr = new TextDecoder().decode(gunzipped);
console.log("Match:", gunzippedStr === testString ? "OK" : "FAILED");

// Test deflate (zlib format)
console.log("\n--- Deflate (zlib format) ---");
const deflated = zlib.deflateSync(testBuffer);
console.log("Deflated size:", deflated.length, "bytes");

const inflated = zlib.inflateSync(deflated);
console.log("Inflated size:", inflated.length, "bytes");
const inflatedStr = new TextDecoder().decode(inflated);
console.log("Match:", inflatedStr === testString ? "OK" : "FAILED");

// Test deflateRaw (raw deflate)
console.log("\n--- DeflateRaw (raw format) ---");
const deflatedRaw = zlib.deflateRawSync(testBuffer);
console.log("DeflateRaw size:", deflatedRaw.length, "bytes");

const inflatedRaw = zlib.inflateRawSync(deflatedRaw);
console.log("InflateRaw size:", inflatedRaw.length, "bytes");
const inflatedRawStr = new TextDecoder().decode(inflatedRaw);
console.log("Match:", inflatedRawStr === testString ? "OK" : "FAILED");

// Test unzip (auto-detect)
console.log("\n--- Unzip (auto-detect) ---");
const unzippedGzip = zlib.unzipSync(gzipped);
console.log("Unzip gzip:", new TextDecoder().decode(unzippedGzip) === testString ? "OK" : "FAILED");

const unzippedDeflate = zlib.unzipSync(deflated);
console.log("Unzip deflate:", new TextDecoder().decode(unzippedDeflate) === testString ? "OK" : "FAILED");

// Test with string input
console.log("\n--- String Input ---");
const gzippedString = zlib.gzipSync("Hello from string!");
const gunzippedString = zlib.gunzipSync(gzippedString);
console.log("String round-trip:", new TextDecoder().decode(gunzippedString));

// Test compression levels
console.log("\n--- Compression Levels ---");
const level1 = zlib.gzipSync(testBuffer, { level: 1 });
const level9 = zlib.gzipSync(testBuffer, { level: 9 });
console.log("Level 1 size:", level1.length, "bytes");
console.log("Level 9 size:", level9.length, "bytes");

// Test CRC32
console.log("\n--- CRC32 ---");
const crc1 = zlib.crc32("hello");
console.log("CRC32 of 'hello':", crc1);

const crc2 = zlib.crc32("world", crc1);
console.log("CRC32 of 'world' (chained):", crc2);

// Test callback-based API
console.log("\n--- Callback API ---");
zlib.gzip(testBuffer, (err, result) => {
    if (err) {
        console.log("Gzip callback error:", err);
    } else {
        console.log("Gzip callback result size:", result.length, "bytes");
    }
});

// Test constants
console.log("\n--- Constants ---");
console.log("Z_NO_COMPRESSION:", zlib.constants.Z_NO_COMPRESSION);
console.log("Z_BEST_SPEED:", zlib.constants.Z_BEST_SPEED);
console.log("Z_BEST_COMPRESSION:", zlib.constants.Z_BEST_COMPRESSION);
console.log("Z_DEFAULT_COMPRESSION:", zlib.constants.Z_DEFAULT_COMPRESSION);

console.log("\n=== Zlib Module Test Complete ===");
