// Test Buffer module import
import { Buffer, constants } from 'buffer';

console.log("Buffer module import test:");
console.log("  Buffer.from('test'):", Buffer.from('test').toString());
console.log("  constants.MAX_LENGTH:", constants.MAX_LENGTH);
console.log("  Buffer.isEncoding('utf8'):", Buffer.isEncoding('utf8'));

// Test with node: prefix
import { Buffer as NodeBuffer } from 'node:buffer';
console.log("  NodeBuffer.from('hello'):", NodeBuffer.from('hello').toString());

console.log("\nModule import tests passed!");
