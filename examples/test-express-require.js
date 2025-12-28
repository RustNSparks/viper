// Test which modules Express needs
console.log('Testing Express dependencies...\n');

try {
    console.log('1. Testing events module...');
    const events = require('events');
    console.log('   ✓ events loaded');
} catch (e) {
    console.log('   ✗ events failed:', e.message);
}

try {
    console.log('2. Testing body-parser...');
    const bodyParser = require('body-parser');
    console.log('   ✓ body-parser loaded');
} catch (e) {
    console.log('   ✗ body-parser failed:', e.message);
}

try {
    console.log('3. Testing merge-descriptors...');
    const merge = require('merge-descriptors');
    console.log('   ✓ merge-descriptors loaded');
} catch (e) {
    console.log('   ✗ merge-descriptors failed:', e.message);
}

try {
    console.log('4. Testing router...');
    const Router = require('router');
    console.log('   ✓ router loaded');
} catch (e) {
    console.log('   ✗ router failed:', e.message);
}

try {
    console.log('5. Testing express itself...');
    const express = require('express');
    console.log('   ✓ express loaded');
    console.log('   express type:', typeof express);
} catch (e) {
    console.log('   ✗ express failed:', e.message);
}

console.log('\nDone!');
