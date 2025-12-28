// Comprehensive test for Node.js util module in Viper

import util from 'util';
import { promisify, callbackify, format, inspect, types } from 'util';

console.log('=== Testing Node.js util module ===\n');

// Test 1: util.format() - Printf-like formatting
console.log('Test 1: util.format()');
console.log(util.format('Hello %s', 'World'));
console.log(util.format('Number: %d, String: %s', 42, 'test'));
console.log(util.format('Integer: %i, Float: %f', 3.14, 2.71828));
console.log(util.format('JSON: %j', { name: 'Alice', age: 30 }));
console.log(util.format('Object: %o', { x: 1, y: 2 }));
console.log(util.format('Percent: %%'));
console.log(util.format('Extra args:', 1, 2, 3));
console.log();

// Test 2: util.promisify() - Convert callback to Promise
console.log('Test 2: util.promisify()');
function callbackFn(arg: string, callback: (err: Error | null, result?: string) => void) {
    setTimeout(() => {
        if (arg === 'error') {
            callback(new Error('Test error'));
        } else {
            callback(null, `Result: ${arg}`);
        }
    }, 10);
}

const promisifiedFn = promisify(callbackFn);

promisifiedFn('success').then(result => {
    console.log('Promisify success:', result);
}).catch(err => {
    console.error('Promisify error:', err.message);
});

promisifiedFn('error').catch(err => {
    console.log('Promisify caught error:', err.message);
});

setTimeout(() => {
    console.log();

    // Test 3: util.callbackify() - Convert Promise to callback
    console.log('Test 3: util.callbackify()');

    async function asyncFn(arg: string): Promise<string> {
        if (arg === 'error') {
            throw new Error('Async error');
        }
        return `Async result: ${arg}`;
    }

    const callbackifiedFn = callbackify(asyncFn);

    callbackifiedFn('success', (err, result) => {
        if (err) {
            console.error('Callbackify error:', err.message);
        } else {
            console.log('Callbackify success:', result);
        }
    });

    callbackifiedFn('error', (err, result) => {
        if (err) {
            console.log('Callbackify caught error:', err.message);
        } else {
            console.log('Callbackify result:', result);
        }
    });

    setTimeout(() => {
        console.log();

        // Test 4: util.inspect() - Object inspection
        console.log('Test 4: util.inspect()');
        const obj = {
            name: 'Alice',
            age: 30,
            hobbies: ['reading', 'coding'],
            nested: { x: 1, y: 2 }
        };
        console.log('Inspect:', inspect(obj));
        console.log('Inspect string:', inspect('hello'));
        console.log('Inspect number:', inspect(42));
        console.log('Inspect null:', inspect(null));
        console.log('Inspect undefined:', inspect(undefined));
        console.log();

        // Test 5: util.types - Type checking
        console.log('Test 5: util.types');
        console.log('isArrayBuffer:', types.isArrayBuffer(new ArrayBuffer(8)));
        console.log('isArrayBuffer (wrong):', types.isArrayBuffer([]));
        console.log('isTypedArray:', types.isTypedArray(new Uint8Array(8)));
        console.log('isTypedArray (wrong):', types.isTypedArray([]));
        console.log('isDate:', types.isDate(new Date()));
        console.log('isDate (wrong):', types.isDate('2023-01-01'));
        console.log('isRegExp:', types.isRegExp(/test/));
        console.log('isRegExp (wrong):', types.isRegExp('test'));
        console.log('isPromise:', types.isPromise(Promise.resolve()));
        console.log('isPromise (wrong):', types.isPromise({}));
        console.log('isMap:', types.isMap(new Map()));
        console.log('isSet:', types.isSet(new Set()));
        console.log('isInt8Array:', types.isInt8Array(new Int8Array(4)));
        console.log('isUint8Array:', types.isUint8Array(new Uint8Array(4)));
        console.log('isFloat32Array:', types.isFloat32Array(new Float32Array(4)));
        console.log('isFloat64Array:', types.isFloat64Array(new Float64Array(4)));
        console.log();

        // Test 6: util.isDeepStrictEqual() - Deep equality
        console.log('Test 6: util.isDeepStrictEqual()');
        console.log('Equal primitives:', util.isDeepStrictEqual(42, 42));
        console.log('Different primitives:', util.isDeepStrictEqual(42, 43));
        console.log('Equal strings:', util.isDeepStrictEqual('hello', 'hello'));
        console.log('Different strings:', util.isDeepStrictEqual('hello', 'world'));
        console.log('null === null:', util.isDeepStrictEqual(null, null));
        console.log('undefined === undefined:', util.isDeepStrictEqual(undefined, undefined));
        console.log('NaN === NaN:', util.isDeepStrictEqual(NaN, NaN));
        console.log();

        // Test 7: util.deprecate() - Deprecation warnings
        console.log('Test 7: util.deprecate()');
        const deprecatedFn = util.deprecate(
            function oldFunction() {
                return 'This is deprecated';
            },
            'oldFunction() is deprecated. Use newFunction() instead.'
        );
        console.log('Calling deprecated function:', deprecatedFn());
        console.log('Calling again (no warning):', deprecatedFn());
        console.log();

        // Test 8: util.inherits() - Prototype inheritance
        console.log('Test 8: util.inherits()');
        function Parent(this: any) {
            this.name = 'Parent';
        }
        Parent.prototype.greet = function() {
            return `Hello from ${this.name}`;
        };

        function Child(this: any) {
            Parent.call(this);
            this.name = 'Child';
        }
        util.inherits(Child, Parent);

        const child = new (Child as any)();
        console.log('Child inherits:', child.greet());
        console.log('super_ exists:', (Child as any).super_ === Parent);
        console.log();

        // Test 9: util.getSystemErrorName() - Error codes
        console.log('Test 9: util.getSystemErrorName()');
        console.log('Error -2:', util.getSystemErrorName(-2));
        console.log('Error -13:', util.getSystemErrorName(-13));
        console.log('Error -17:', util.getSystemErrorName(-17));
        console.log('Unknown error:', util.getSystemErrorName(-999));
        console.log();

        // Test 10: util.getSystemErrorMap() - All error codes
        console.log('Test 10: util.getSystemErrorMap()');
        const errorMap = util.getSystemErrorMap();
        console.log('Error map is Map:', errorMap instanceof Map);
        console.log('Error map size:', errorMap.size);
        console.log();

        // Test 11: util.debuglog() - Debug logging
        console.log('Test 11: util.debuglog()');
        const debug = util.debuglog('test');
        console.log('Debug enabled:', debug.enabled);
        debug('This is a debug message');
        console.log();

        // Test 12: CommonJS require() support
        console.log('Test 12: CommonJS require() support');
        const utilRequire = require('util');
        console.log('require("util") works:', typeof utilRequire.format === 'function');
        console.log('require("node:util") works:', typeof require('node:util').format === 'function');
        console.log();

        console.log('=== All util module tests completed! ===');
    }, 100);
}, 100);
