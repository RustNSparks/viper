// Buffer module tests

console.log("=== Buffer Module Tests ===\n");

// Test Buffer.alloc
console.log("1. Buffer.alloc tests:");
const buf1 = Buffer.alloc(10);
console.log("  Buffer.alloc(10):", buf1.length, "bytes, all zeros:", buf1[0] === 0 && buf1[9] === 0);

const buf2 = Buffer.alloc(5, 0x41);
console.log("  Buffer.alloc(5, 0x41):", buf2.toString(), "(should be AAAAA)");

const buf3 = Buffer.alloc(11, 'hello', 'utf8');
console.log("  Buffer.alloc(11, 'hello'):", buf3.toString(), "(should be hellohelloh)");

// Test Buffer.allocUnsafe
console.log("\n2. Buffer.allocUnsafe tests:");
const buf4 = Buffer.allocUnsafe(10);
console.log("  Buffer.allocUnsafe(10):", buf4.length, "bytes");

// Test Buffer.from
console.log("\n3. Buffer.from tests:");
const buf5 = Buffer.from('Hello, World!');
console.log("  Buffer.from('Hello, World!'):", buf5.toString());

const buf6 = Buffer.from([72, 101, 108, 108, 111]);
console.log("  Buffer.from([72, 101, 108, 108, 111]):", buf6.toString(), "(should be Hello)");

const buf7 = Buffer.from('48656c6c6f', 'hex');
console.log("  Buffer.from('48656c6c6f', 'hex'):", buf7.toString(), "(should be Hello)");

const buf8 = Buffer.from('SGVsbG8=', 'base64');
console.log("  Buffer.from('SGVsbG8=', 'base64'):", buf8.toString(), "(should be Hello)");

// Test Buffer.concat
console.log("\n4. Buffer.concat tests:");
const buf9 = Buffer.concat([Buffer.from('Hello'), Buffer.from(' '), Buffer.from('World')]);
console.log("  Buffer.concat([...]):", buf9.toString(), "(should be Hello World)");

// Test Buffer.byteLength
console.log("\n5. Buffer.byteLength tests:");
console.log("  Buffer.byteLength('Hello'):", Buffer.byteLength('Hello'), "(should be 5)");
console.log("  Buffer.byteLength('Hello', 'utf8'):", Buffer.byteLength('Hello', 'utf8'), "(should be 5)");
console.log("  Buffer.byteLength('48656c6c6f', 'hex'):", Buffer.byteLength('48656c6c6f', 'hex'), "(should be 5)");

// Test Buffer.compare
console.log("\n6. Buffer.compare tests:");
const bufA = Buffer.from('ABC');
const bufB = Buffer.from('ABC');
const bufC = Buffer.from('ABD');
console.log("  Buffer.compare(ABC, ABC):", Buffer.compare(bufA, bufB), "(should be 0)");
console.log("  Buffer.compare(ABC, ABD):", Buffer.compare(bufA, bufC), "(should be -1)");

// Test Buffer.isBuffer
console.log("\n7. Buffer.isBuffer tests:");
console.log("  Buffer.isBuffer(Buffer.alloc(5)):", Buffer.isBuffer(Buffer.alloc(5)), "(should be true)");
console.log("  Buffer.isBuffer('string'):", Buffer.isBuffer('string'), "(should be false)");

// Test Buffer.isEncoding
console.log("\n8. Buffer.isEncoding tests:");
console.log("  Buffer.isEncoding('utf8'):", Buffer.isEncoding('utf8'), "(should be true)");
console.log("  Buffer.isEncoding('hex'):", Buffer.isEncoding('hex'), "(should be true)");
console.log("  Buffer.isEncoding('base64'):", Buffer.isEncoding('base64'), "(should be true)");
console.log("  Buffer.isEncoding('invalid'):", Buffer.isEncoding('invalid'), "(should be false)");

// Test read/write methods
console.log("\n9. Read/Write method tests:");
const buf10 = Buffer.alloc(8);
buf10.writeUInt8(255, 0);
buf10.writeUInt16BE(0x1234, 1);
buf10.writeUInt16LE(0x1234, 3);
buf10.writeUInt32BE(0x12345678, 4);
console.log("  writeUInt8(255, 0) then readUInt8(0):", buf10.readUInt8(0), "(should be 255)");
console.log("  writeUInt16BE(0x1234, 1) then readUInt16BE(1):", buf10.readUInt16BE(1).toString(16), "(should be 1234)");
console.log("  writeUInt16LE(0x1234, 3) then readUInt16LE(3):", buf10.readUInt16LE(3).toString(16), "(should be 1234)");

// Test toString with encoding
console.log("\n10. toString encoding tests:");
const buf11 = Buffer.from('Hello');
console.log("  buf.toString('utf8'):", buf11.toString('utf8'));
console.log("  buf.toString('hex'):", buf11.toString('hex'), "(should be 48656c6c6f)");
console.log("  buf.toString('base64'):", buf11.toString('base64'), "(should be SGVsbG8=)");

// Test fill
console.log("\n11. fill tests:");
const buf12 = Buffer.alloc(5);
buf12.fill('a');
console.log("  buf.fill('a'):", buf12.toString(), "(should be aaaaa)");

// Test copy
console.log("\n12. copy tests:");
const buf13 = Buffer.from('Hello');
const buf14 = Buffer.alloc(10);
buf13.copy(buf14);
console.log("  buf.copy(target):", buf14.toString('utf8', 0, 5), "(should be Hello)");

// Test slice
console.log("\n13. slice tests:");
const buf15 = Buffer.from('Hello World');
const sliced = buf15.slice(0, 5);
console.log("  buf.slice(0, 5):", sliced.toString(), "(should be Hello)");

// Test equals
console.log("\n14. equals tests:");
const buf16 = Buffer.from('Hello');
const buf17 = Buffer.from('Hello');
const buf18 = Buffer.from('World');
console.log("  buf('Hello').equals(buf('Hello')):", buf16.equals(buf17), "(should be true)");
console.log("  buf('Hello').equals(buf('World')):", buf16.equals(buf18), "(should be false)");

// Test indexOf
console.log("\n15. indexOf tests:");
const buf19 = Buffer.from('Hello World');
console.log("  buf.indexOf('World'):", buf19.indexOf('World'), "(should be 6)");
console.log("  buf.indexOf('x'):", buf19.indexOf('x'), "(should be -1)");

// Test toJSON
console.log("\n16. toJSON tests:");
const buf20 = Buffer.from('Hi');
const json = buf20.toJSON();
console.log("  buf.toJSON():", JSON.stringify(json), "(should have type: 'Buffer' and data array)");

console.log("\n=== All Buffer tests completed! ===");
