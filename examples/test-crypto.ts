// Comprehensive Crypto Module Tests

console.log('=== Testing Crypto Module ===\n');

// Test 1: crypto.createHash() with SHA-256
console.log('Test 1: crypto.createHash() - SHA-256');
try {
    const hash = crypto.createHash('sha256');
    hash.update('hello');
    hash.update(' world');
    const digest = hash.digest('hex');
    console.log('  SHA-256 hex:', digest);

    // Expected: b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
    const expected = 'b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9';
    if (digest === expected) {
        console.log('  ✓ SHA-256 hash correct');
    } else {
        console.log('  ✗ SHA-256 hash incorrect');
        console.log('  Expected:', expected);
        console.log('  Got:', digest);
    }
} catch (e) {
    console.error('  ✗ Error:', e);
}
console.log();

// Test 2: crypto.createHash() with different algorithms
console.log('Test 2: Multiple hash algorithms');
try {
    const algorithms = ['sha256', 'sha512', 'sha1', 'md5'];
    const input = 'test';

    for (const algo of algorithms) {
        const hash = crypto.createHash(algo);
        hash.update(input);
        const digest = hash.digest('hex');
        console.log(`  ${algo}:`, digest.substring(0, 32) + '...');
    }
    console.log('  ✓ All hash algorithms working');
} catch (e) {
    console.error('  ✗ Error:', e);
}
console.log();

// Test 3: crypto.createHash() with base64 encoding
console.log('Test 3: Hash with base64 encoding');
try {
    const hash = crypto.createHash('sha256');
    hash.update('hello world');
    const digest = hash.digest('base64');
    console.log('  Base64 digest:', digest);

    // Should be a valid base64 string
    if (typeof digest === 'string' && digest.length > 0) {
        console.log('  ✓ Base64 encoding working');
    } else {
        console.log('  ✗ Base64 encoding failed');
    }
} catch (e) {
    console.error('  ✗ Error:', e);
}
console.log();

// Test 4: crypto.createHash() with buffer output
console.log('Test 4: Hash with buffer output');
try {
    const hash = crypto.createHash('sha256');
    hash.update('hello world');
    const buffer = hash.digest('buffer');
    console.log('  Buffer type:', buffer.constructor.name);
    console.log('  Buffer length:', buffer.length);

    if (buffer instanceof Uint8Array && buffer.length === 32) {
        console.log('  ✓ Buffer output working (32 bytes for SHA-256)');
    } else {
        console.log('  ✗ Buffer output incorrect');
    }
} catch (e) {
    console.error('  ✗ Error:', e);
}
console.log();

// Test 5: crypto.createHmac() with SHA-256
console.log('Test 5: crypto.createHmac() - SHA-256');
try {
    const hmac = crypto.createHmac('sha256', 'secret-key');
    hmac.update('hello world');
    const digest = hmac.digest('hex');
    console.log('  HMAC-SHA256 hex:', digest);

    // HMAC with key 'secret-key' and message 'hello world'
    const expected = '734cc62f32841568f45715aeb9f4d7891324e6d948e4c6c60c0621cdac48623a';
    if (digest === expected) {
        console.log('  ✓ HMAC-SHA256 correct');
    } else {
        console.log('  ✗ HMAC-SHA256 incorrect');
        console.log('  Expected:', expected);
        console.log('  Got:', digest);
    }
} catch (e) {
    console.error('  ✗ Error:', e);
}
console.log();

// Test 6: crypto.createHmac() with different algorithms
console.log('Test 6: HMAC with multiple algorithms');
try {
    const algorithms = ['sha256', 'sha512', 'md5'];
    const key = 'my-secret-key';
    const input = 'test message';

    for (const algo of algorithms) {
        const hmac = crypto.createHmac(algo, key);
        hmac.update(input);
        const digest = hmac.digest('hex');
        console.log(`  HMAC-${algo}:`, digest.substring(0, 32) + '...');
    }
    console.log('  ✓ All HMAC algorithms working');
} catch (e) {
    console.error('  ✗ Error:', e);
}
console.log();

// Test 7: crypto.createHmac() with Uint8Array key
console.log('Test 7: HMAC with Uint8Array key');
try {
    const key = new Uint8Array([1, 2, 3, 4, 5]);
    const hmac = crypto.createHmac('sha256', key);
    hmac.update('test');
    const digest = hmac.digest('hex');
    console.log('  HMAC with binary key:', digest);

    if (digest.length === 64) { // SHA-256 produces 64 hex characters
        console.log('  ✓ HMAC with Uint8Array key working');
    } else {
        console.log('  ✗ HMAC with Uint8Array key failed');
    }
} catch (e) {
    console.error('  ✗ Error:', e);
}
console.log();

// Test 8: crypto.pbkdf2Sync()
console.log('Test 8: crypto.pbkdf2Sync()');
try {
    const password = 'password123';
    const salt = 'salt456';
    const iterations = 1000;
    const keylen = 32;

    const derivedKey = crypto.pbkdf2Sync(password, salt, iterations, keylen, 'sha256');
    console.log('  Derived key type:', derivedKey.constructor.name);
    console.log('  Derived key length:', derivedKey.length);
    console.log('  First 16 bytes (hex):', Array.from(derivedKey.slice(0, 16)).map(b => b.toString(16).padStart(2, '0')).join(''));

    if (derivedKey instanceof Uint8Array && derivedKey.length === keylen) {
        console.log('  ✓ pbkdf2Sync working correctly');
    } else {
        console.log('  ✗ pbkdf2Sync failed');
    }
} catch (e) {
    console.error('  ✗ Error:', e);
}
console.log();

// Test 9: crypto.pbkdf2Sync() with different digests
console.log('Test 9: pbkdf2Sync with different digest algorithms');
try {
    const password = 'test';
    const salt = 'salt';
    const iterations = 100;
    const keylen = 16;

    const digests = ['sha256', 'sha512', 'sha1'];
    for (const digest of digests) {
        const key = crypto.pbkdf2Sync(password, salt, iterations, keylen, digest);
        console.log(`  ${digest}:`, Array.from(key.slice(0, 8)).map(b => b.toString(16).padStart(2, '0')).join('') + '...');
    }
    console.log('  ✓ All pbkdf2 digest algorithms working');
} catch (e) {
    console.error('  ✗ Error:', e);
}
console.log();

// Test 10: crypto.pbkdf2() async
console.log('Test 10: crypto.pbkdf2() async');
try {
    const password = 'password';
    const salt = 'salt';
    const iterations = 500;
    const keylen = 32;

    crypto.pbkdf2(password, salt, iterations, keylen, 'sha256', (err, derivedKey) => {
        if (err) {
            console.error('  ✗ Error:', err);
        } else {
            console.log('  Async derived key length:', derivedKey.length);
            console.log('  Async derived key type:', derivedKey.constructor.name);

            if (derivedKey instanceof Uint8Array && derivedKey.length === keylen) {
                console.log('  ✓ pbkdf2 async working correctly');
            } else {
                console.log('  ✗ pbkdf2 async failed');
            }
        }
    });
} catch (e) {
    console.error('  ✗ Error:', e);
}
console.log();

// Test 11: crypto.pbkdf2() async with optional digest parameter
console.log('Test 11: crypto.pbkdf2() with optional digest');
try {
    const password = 'test';
    const salt = 'test-salt';
    const iterations = 100;
    const keylen = 16;

    // Without digest parameter (should default to sha256)
    crypto.pbkdf2(password, salt, iterations, keylen, (err, derivedKey) => {
        if (err) {
            console.error('  ✗ Error:', err);
        } else {
            console.log('  Default digest key length:', derivedKey.length);

            if (derivedKey instanceof Uint8Array && derivedKey.length === keylen) {
                console.log('  ✓ pbkdf2 with default digest working');
            } else {
                console.log('  ✗ pbkdf2 with default digest failed');
            }
        }
    });
} catch (e) {
    console.error('  ✗ Error:', e);
}
console.log();

// Test 12: crypto.randomUUID()
console.log('Test 12: crypto.randomUUID()');
try {
    const uuid1 = crypto.randomUUID();
    const uuid2 = crypto.randomUUID();
    console.log('  UUID 1:', uuid1);
    console.log('  UUID 2:', uuid2);

    const uuidPattern = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
    if (uuidPattern.test(uuid1) && uuidPattern.test(uuid2) && uuid1 !== uuid2) {
        console.log('  ✓ randomUUID working correctly');
    } else {
        console.log('  ✗ randomUUID failed');
    }
} catch (e) {
    console.error('  ✗ Error:', e);
}
console.log();

// Test 13: crypto.randomBytes()
console.log('Test 13: crypto.randomBytes()');
try {
    const bytes1 = crypto.randomBytes(16);
    const bytes2 = crypto.randomBytes(16);
    console.log('  Bytes 1 length:', bytes1.length);
    console.log('  Bytes 2 length:', bytes2.length);
    console.log('  Bytes 1 type:', bytes1.constructor.name);

    const same = bytes1.every((b, i) => b === bytes2[i]);
    if (bytes1.length === 16 && bytes2.length === 16 && !same) {
        console.log('  ✓ randomBytes working correctly');
    } else {
        console.log('  ✗ randomBytes failed');
    }
} catch (e) {
    console.error('  ✗ Error:', e);
}
console.log();

// Test 14: crypto.getRandomValues()
console.log('Test 14: crypto.getRandomValues()');
try {
    const array = new Uint8Array(16);
    crypto.getRandomValues(array);
    console.log('  Array length:', array.length);
    console.log('  First 8 bytes:', Array.from(array.slice(0, 8)).map(b => b.toString(16).padStart(2, '0')).join(' '));

    const allZeros = array.every(b => b === 0);
    if (array.length === 16 && !allZeros) {
        console.log('  ✓ getRandomValues working correctly');
    } else {
        console.log('  ✗ getRandomValues failed');
    }
} catch (e) {
    console.error('  ✗ Error:', e);
}
console.log();

// Test 15: Chaining hash.update() calls
console.log('Test 15: Chaining hash.update() calls');
try {
    const hash1 = crypto.createHash('sha256');
    const digest1 = hash1.update('hello').update(' ').update('world').digest('hex');

    const hash2 = crypto.createHash('sha256');
    hash2.update('hello world');
    const digest2 = hash2.digest('hex');

    console.log('  Chained digest:', digest1);
    console.log('  Single digest:', digest2);

    if (digest1 === digest2) {
        console.log('  ✓ Hash chaining working correctly');
    } else {
        console.log('  ✗ Hash chaining failed');
    }
} catch (e) {
    console.error('  ✗ Error:', e);
}
console.log();

// Test 16: Chaining hmac.update() calls
console.log('Test 16: Chaining hmac.update() calls');
try {
    const hmac1 = crypto.createHmac('sha256', 'key');
    const digest1 = hmac1.update('part1').update('part2').digest('hex');

    const hmac2 = crypto.createHmac('sha256', 'key');
    hmac2.update('part1part2');
    const digest2 = hmac2.digest('hex');

    console.log('  Chained HMAC:', digest1.substring(0, 32) + '...');
    console.log('  Single HMAC:', digest2.substring(0, 32) + '...');

    if (digest1 === digest2) {
        console.log('  ✓ HMAC chaining working correctly');
    } else {
        console.log('  ✗ HMAC chaining failed');
    }
} catch (e) {
    console.error('  ✗ Error:', e);
}
console.log();

// Test 17: Hash with Uint8Array input
console.log('Test 17: Hash with Uint8Array input');
try {
    const data = new Uint8Array([104, 101, 108, 108, 111]); // 'hello' in ASCII
    const hash = crypto.createHash('sha256');
    hash.update(data);
    const digest = hash.digest('hex');
    console.log('  Hash from Uint8Array:', digest);

    // Compare with string version
    const hash2 = crypto.createHash('sha256');
    hash2.update('hello');
    const digest2 = hash2.digest('hex');

    if (digest === digest2) {
        console.log('  ✓ Hash with Uint8Array working correctly');
    } else {
        console.log('  ✗ Hash with Uint8Array failed');
    }
} catch (e) {
    console.error('  ✗ Error:', e);
}
console.log();

// Wait for async tests to complete
setTimeout(() => {
    console.log('\n=== All crypto module tests completed! ===');
}, 100);
