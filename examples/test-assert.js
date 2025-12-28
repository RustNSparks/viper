// Test without imports - using global assert
console.log('Testing assert module...\n');

// Test assert.ok
console.log('1. Testing assert.ok...');
assert.ok(true);
assert.ok(1);
assert.ok('hello');
console.log('   PASS: assert.ok works with truthy values');

// Test assert.strictEqual
console.log('\n2. Testing assert.strictEqual...');
assert.strictEqual(1, 1);
assert.strictEqual('hello', 'hello');
assert.strictEqual(true, true);
console.log('   PASS: assert.strictEqual works');

// Test assert.notStrictEqual
console.log('\n3. Testing assert.notStrictEqual...');
assert.notStrictEqual(1, 2);
assert.notStrictEqual('hello', 'world');
console.log('   PASS: assert.notStrictEqual works');

// Test assert.fail - should throw
console.log('\n4. Testing assert.fail...');
try {
    assert.fail('This should fail');
    console.log('   FAIL: assert.fail did not throw');
} catch (e) {
    console.log('   PASS: assert.fail throws as expected');
}

// Test assert.throws
console.log('\n5. Testing assert.throws...');
assert.throws(() => {
    throw new Error('expected error');
});
console.log('   PASS: assert.throws works');

// Test assert.doesNotThrow
console.log('\n6. Testing assert.doesNotThrow...');
assert.doesNotThrow(() => {
    return 42;
});
console.log('   PASS: assert.doesNotThrow works');

// Test assert.ifError
console.log('\n7. Testing assert.ifError...');
assert.ifError(null);
assert.ifError(undefined);
console.log('   PASS: assert.ifError works');

// Test AssertionError
console.log('\n8. Testing AssertionError...');
const AssertionError = assert.AssertionError;
const err = new AssertionError({ message: 'test error' });
console.log('   AssertionError name:', err.name);
console.log('   PASS: AssertionError works');

console.log('\n========================================');
console.log('All basic assert tests passed!');
console.log('========================================');
