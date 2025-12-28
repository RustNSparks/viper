// Stream Module Tests

console.log('=== Testing Stream Module ===\n');

// Test 1: Readable stream
console.log('Test 1: Readable stream');
try {
    const { Readable } = require('stream');

    const readable = new Readable({
        read(size) {
            this.push('Hello ');
            this.push('World!');
            this.push(null); // End the stream
        }
    });

    let data = '';
    readable.on('data', (chunk) => {
        data += chunk.toString();
    });
    readable.on('end', () => {
        console.log('  Received:', data);
        if (data === 'Hello World!') {
            console.log('  ✓ Readable stream working');
        } else {
            console.log('  ✗ Unexpected data');
        }
    });
} catch (e) {
    console.error('  ✗ Error:', e.message);
}
console.log();

// Test 2: Writable stream
console.log('Test 2: Writable stream');
try {
    const { Writable } = require('stream');

    let written = '';
    const writable = new Writable({
        write(chunk, encoding, callback) {
            written += chunk.toString();
            callback();
        }
    });

    writable.write('Hello ');
    writable.write('Stream!');
    writable.end();

    writable.on('finish', () => {
        console.log('  Written:', written);
        if (written === 'Hello Stream!') {
            console.log('  ✓ Writable stream working');
        } else {
            console.log('  ✗ Unexpected data');
        }
    });
} catch (e) {
    console.error('  ✗ Error:', e.message);
}
console.log();

// Test 3: Transform stream
console.log('Test 3: Transform stream');
try {
    const { Transform } = require('stream');

    const upperCase = new Transform({
        transform(chunk, encoding, callback) {
            callback(null, chunk.toString().toUpperCase());
        }
    });

    let result = '';
    upperCase.on('data', (chunk) => {
        result += chunk.toString();
    });
    upperCase.on('end', () => {
        console.log('  Transformed:', result);
        if (result === 'HELLO TRANSFORM!') {
            console.log('  ✓ Transform stream working');
        } else {
            console.log('  ✗ Unexpected result:', result);
        }
    });

    upperCase.write('hello ');
    upperCase.write('transform!');
    upperCase.end();
} catch (e) {
    console.error('  ✗ Error:', e.message);
}
console.log();

// Test 4: PassThrough stream
console.log('Test 4: PassThrough stream');
try {
    const { PassThrough } = require('stream');

    const passThrough = new PassThrough();

    let data = '';
    passThrough.on('data', (chunk) => {
        data += chunk.toString();
    });
    passThrough.on('end', () => {
        console.log('  Passed through:', data);
        if (data === 'Pass Through Test') {
            console.log('  ✓ PassThrough stream working');
        } else {
            console.log('  ✗ Unexpected data');
        }
    });

    passThrough.write('Pass ');
    passThrough.write('Through ');
    passThrough.write('Test');
    passThrough.end();
} catch (e) {
    console.error('  ✗ Error:', e.message);
}
console.log();

// Test 5: Duplex stream
console.log('Test 5: Duplex stream');
try {
    const { Duplex } = require('stream');

    const duplex = new Duplex({
        read(size) {
            this.push('from read');
            this.push(null);
        },
        write(chunk, encoding, callback) {
            console.log('  Duplex received:', chunk.toString());
            callback();
        }
    });

    duplex.on('data', (chunk) => {
        console.log('  Duplex output:', chunk.toString());
    });

    duplex.write('to write');
    duplex.end();

    console.log('  ✓ Duplex stream created');
} catch (e) {
    console.error('  ✗ Error:', e.message);
}
console.log();

// Test 6: Stream module exports
console.log('Test 6: Stream module exports');
try {
    const stream = require('stream');

    const exports = ['Readable', 'Writable', 'Duplex', 'Transform', 'PassThrough', 'pipeline', 'finished'];
    let allPresent = true;

    for (const exp of exports) {
        if (typeof stream[exp] === 'undefined') {
            console.log('  ✗ Missing:', exp);
            allPresent = false;
        }
    }

    if (allPresent) {
        console.log('  ✓ All stream exports present');
    }
} catch (e) {
    console.error('  ✗ Error:', e.message);
}
console.log();

// Test 7: Pipe streams
console.log('Test 7: Pipe streams');
try {
    const { Readable, Transform, Writable } = require('stream');

    const source = new Readable({
        read() {
            this.push('pipe test');
            this.push(null);
        }
    });

    const transform = new Transform({
        transform(chunk, encoding, callback) {
            callback(null, chunk.toString().toUpperCase());
        }
    });

    let result = '';
    const sink = new Writable({
        write(chunk, encoding, callback) {
            result += chunk.toString();
            callback();
        }
    });

    sink.on('finish', () => {
        console.log('  Piped result:', result);
        if (result === 'PIPE TEST') {
            console.log('  ✓ Pipe working');
        } else {
            console.log('  ✗ Unexpected result');
        }
    });

    source.pipe(transform).pipe(sink);
} catch (e) {
    console.error('  ✗ Error:', e.message);
}
console.log();

console.log('=== Stream tests completed! ===');
