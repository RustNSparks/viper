// Test Buffer with require
const { Buffer } = require('buffer');

console.log("Buffer require test:");
console.log("  Buffer.from('test'):", Buffer.from('test').toString());
console.log("  Buffer.isBuffer(Buffer.alloc(5)):", Buffer.isBuffer(Buffer.alloc(5)));

const nodeBuffer = require('node:buffer');
console.log("  require('node:buffer').Buffer.from('hello'):", nodeBuffer.Buffer.from('hello').toString());

console.log("\nrequire() tests passed!");
