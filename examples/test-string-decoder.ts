// Test string_decoder module - Node.js compatible string decoding

console.log("=== String Decoder Module Test ===\n");

// Test basic UTF-8 decoding
console.log("--- Basic UTF-8 ---");
const decoder = new StringDecoder("utf8");
console.log("decoder.encoding:", decoder.encoding);

// Single byte ASCII
const ascii = Buffer.from([0x48, 0x65, 0x6c, 0x6c, 0x6f]); // "Hello"
console.log("ASCII 'Hello':", decoder.write(ascii));

// 2-byte UTF-8 (¢ = 0xC2 0xA2)
const cent = Buffer.from([0xc2, 0xa2]);
console.log("Cent sign (¢):", decoder.write(cent));

// 3-byte UTF-8 (€ = 0xE2 0x82 0xAC)
const euro = Buffer.from([0xe2, 0x82, 0xac]);
console.log("Euro sign (€):", decoder.write(euro));

// 4-byte UTF-8 (😀 = 0xF0 0x9F 0x98 0x80)
const emoji = Buffer.from([0xf0, 0x9f, 0x98, 0x80]);
console.log("Emoji (😀):", decoder.write(emoji));

// Test multi-byte buffering
console.log("\n--- Multi-byte Buffering ---");
const decoder2 = new StringDecoder("utf8");

// Write the Euro sign in 3 separate calls
console.log("Writing Euro sign byte by byte:");
let result = decoder2.write(Buffer.from([0xe2]));
console.log("  After 0xE2:", JSON.stringify(result), "(empty - waiting for more bytes)");

result = decoder2.write(Buffer.from([0x82]));
console.log("  After 0x82:", JSON.stringify(result), "(empty - still waiting)");

result = decoder2.end(Buffer.from([0xac]));
console.log("  After 0xAC (end):", JSON.stringify(result), "(€)");

// Test incomplete sequence handling
console.log("\n--- Incomplete Sequence Handling ---");
const decoder3 = new StringDecoder("utf8");
decoder3.write(Buffer.from([0xe2, 0x82])); // Incomplete Euro
const endResult = decoder3.end(); // Should produce replacement character
console.log("Incomplete sequence at end:", JSON.stringify(endResult), "(replacement char)");

// Test Latin1 encoding
console.log("\n--- Latin1 Encoding ---");
const latin1Decoder = new StringDecoder("latin1");
const latin1Bytes = Buffer.from([0x48, 0xe9, 0x6c, 0x6c, 0xf6]); // "Héllö" in Latin1
console.log("Latin1 'Héllö':", latin1Decoder.write(latin1Bytes));

// Test Hex encoding
console.log("\n--- Hex Encoding ---");
const hexDecoder = new StringDecoder("hex");
const hexBytes = Buffer.from([0x48, 0x65, 0x6c, 0x6c, 0x6f]);
console.log("Hex of 'Hello':", hexDecoder.write(hexBytes));

// Test ASCII encoding
console.log("\n--- ASCII Encoding ---");
const asciiDecoder = new StringDecoder("ascii");
const asciiHigh = Buffer.from([0x48, 0xc9, 0x6c, 0x6c, 0xf6]); // High bytes should be masked
console.log("ASCII with high bytes:", asciiDecoder.write(asciiHigh));

// Test UTF-16LE encoding
console.log("\n--- UTF-16LE Encoding ---");
const utf16Decoder = new StringDecoder("utf16le");
// "Hi" in UTF-16LE: H=0x48 0x00, i=0x69 0x00
const utf16Bytes = Buffer.from([0x48, 0x00, 0x69, 0x00]);
console.log("UTF-16LE 'Hi':", utf16Decoder.write(utf16Bytes));

// Test UTF-16LE with odd byte
console.log("\n--- UTF-16LE Odd Byte Buffering ---");
const utf16Decoder2 = new StringDecoder("utf16le");
let part1 = utf16Decoder2.write(Buffer.from([0x48])); // Just first byte of 'H'
console.log("After first byte:", JSON.stringify(part1));
let part2 = utf16Decoder2.write(Buffer.from([0x00, 0x69, 0x00])); // Rest of 'H' + 'i'
console.log("After remaining bytes:", part2);

// Test reusing decoder after end()
console.log("\n--- Decoder Reuse ---");
const reuseDecoder = new StringDecoder("utf8");
console.log("First use:", reuseDecoder.end(Buffer.from("Hello")));
console.log("Second use:", reuseDecoder.end(Buffer.from("World")));

// Test with different input types
console.log("\n--- Different Input Types ---");
const inputDecoder = new StringDecoder("utf8");
console.log("From Buffer:", inputDecoder.write(Buffer.from("Buffer input")));
console.log("From Uint8Array:", inputDecoder.write(new Uint8Array([0x55, 0x69, 0x6e, 0x74, 0x38]))); // "Uint8"

console.log("\n=== String Decoder Module Test Complete ===");
