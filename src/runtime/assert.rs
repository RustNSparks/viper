use boa_engine::{Context, JsResult, Source, js_string};

/// Register the assert module
pub fn register_assert_module(context: &mut Context) -> JsResult<()> {
    // Create the assert module entirely in JavaScript for simplicity and to avoid stack issues
    let code = r#"
        (function() {
            // AssertionError class
            function AssertionError(options) {
                if (!(this instanceof AssertionError)) {
                    return new AssertionError(options);
                }

                options = options || {};

                let message = options.message;
                if (!message) {
                    if (options.actual !== undefined && options.expected !== undefined) {
                        message = JSON.stringify(options.actual) + ' ' + (options.operator || '==') + ' ' + JSON.stringify(options.expected);
                    } else {
                        message = 'Assertion failed';
                    }
                }

                Error.call(this, message);
                this.message = message;
                this.name = 'AssertionError';
                this.actual = options.actual;
                this.expected = options.expected;
                this.operator = options.operator;
                this.generatedMessage = !options.message;
                this.code = 'ERR_ASSERTION';
            }

            AssertionError.prototype = Object.create(Error.prototype);
            AssertionError.prototype.constructor = AssertionError;
            AssertionError.prototype.name = 'AssertionError';

            AssertionError.prototype.toString = function() {
                return this.name + ': ' + this.message;
            };

            // Helper to throw AssertionError
            function fail(message) {
                throw new AssertionError({ message: message || 'Failed', operator: 'fail' });
            }

            // assert.ok - Tests if value is truthy
            function ok(value, message) {
                if (!value) {
                    throw new AssertionError({
                        message: message || 'The expression evaluated to a falsy value',
                        actual: value,
                        expected: true,
                        operator: 'ok'
                    });
                }
            }

            // assert.equal - Tests shallow equality with ==
            function equal(actual, expected, message) {
                if (actual != expected) {
                    throw new AssertionError({
                        message: message,
                        actual: actual,
                        expected: expected,
                        operator: '=='
                    });
                }
            }

            // assert.notEqual - Tests shallow inequality with !=
            function notEqual(actual, expected, message) {
                if (actual == expected) {
                    throw new AssertionError({
                        message: message,
                        actual: actual,
                        expected: expected,
                        operator: '!='
                    });
                }
            }

            // assert.strictEqual - Tests strict equality with ===
            function strictEqual(actual, expected, message) {
                if (actual !== expected) {
                    throw new AssertionError({
                        message: message,
                        actual: actual,
                        expected: expected,
                        operator: '==='
                    });
                }
            }

            // assert.notStrictEqual - Tests strict inequality with !==
            function notStrictEqual(actual, expected, message) {
                if (actual === expected) {
                    throw new AssertionError({
                        message: message,
                        actual: actual,
                        expected: expected,
                        operator: '!=='
                    });
                }
            }

            // Deep equality helper
            function isDeepEqual(a, b, strict) {
                if (strict ? a === b : a == b) return true;
                if (a === null || b === null) return strict ? a === b : a == b;
                if (typeof a !== 'object' || typeof b !== 'object') return strict ? a === b : a == b;

                // Handle arrays
                if (Array.isArray(a) !== Array.isArray(b)) return false;
                if (Array.isArray(a)) {
                    if (a.length !== b.length) return false;
                    for (let i = 0; i < a.length; i++) {
                        if (!isDeepEqual(a[i], b[i], strict)) return false;
                    }
                    return true;
                }

                // Handle objects
                const aKeys = Object.keys(a);
                const bKeys = Object.keys(b);
                if (aKeys.length !== bKeys.length) return false;

                for (const key of aKeys) {
                    if (!Object.prototype.hasOwnProperty.call(b, key)) return false;
                    if (!isDeepEqual(a[key], b[key], strict)) return false;
                }

                return true;
            }

            // assert.deepEqual - Tests for deep equality
            function deepEqual(actual, expected, message) {
                if (!isDeepEqual(actual, expected, false)) {
                    throw new AssertionError({
                        message: message,
                        actual: actual,
                        expected: expected,
                        operator: 'deepEqual'
                    });
                }
            }

            // assert.notDeepEqual - Tests for deep inequality
            function notDeepEqual(actual, expected, message) {
                if (isDeepEqual(actual, expected, false)) {
                    throw new AssertionError({
                        message: message,
                        actual: actual,
                        expected: expected,
                        operator: 'notDeepEqual'
                    });
                }
            }

            // assert.deepStrictEqual - Tests for deep strict equality
            function deepStrictEqual(actual, expected, message) {
                if (!isDeepEqual(actual, expected, true)) {
                    throw new AssertionError({
                        message: message,
                        actual: actual,
                        expected: expected,
                        operator: 'deepStrictEqual'
                    });
                }
            }

            // assert.notDeepStrictEqual - Tests for deep strict inequality
            function notDeepStrictEqual(actual, expected, message) {
                if (isDeepEqual(actual, expected, true)) {
                    throw new AssertionError({
                        message: message,
                        actual: actual,
                        expected: expected,
                        operator: 'notDeepStrictEqual'
                    });
                }
            }

            // assert.throws - Expects fn to throw an error
            function throws(fn, errorOrMessage, message) {
                if (typeof fn !== 'function') {
                    throw new TypeError('First argument must be a function');
                }

                let threw = false;
                let error = null;

                try {
                    fn();
                } catch (e) {
                    threw = true;
                    error = e;
                }

                if (!threw) {
                    throw new AssertionError({
                        message: message || 'Missing expected exception',
                        operator: 'throws'
                    });
                }

                // Validate the error if a validator is provided
                if (errorOrMessage !== undefined && typeof errorOrMessage !== 'string') {
                    if (typeof errorOrMessage === 'function') {
                        if (!errorOrMessage(error)) {
                            throw new AssertionError({
                                message: message || 'Error did not match validator',
                                actual: error,
                                operator: 'throws'
                            });
                        }
                    } else if (errorOrMessage instanceof RegExp) {
                        if (!errorOrMessage.test(String(error))) {
                            throw new AssertionError({
                                message: message || 'Error did not match pattern',
                                actual: error,
                                operator: 'throws'
                            });
                        }
                    }
                }
            }

            // assert.doesNotThrow - Expects fn not to throw
            function doesNotThrow(fn, errorOrMessage, message) {
                if (typeof fn !== 'function') {
                    throw new TypeError('First argument must be a function');
                }

                try {
                    fn();
                } catch (e) {
                    throw new AssertionError({
                        message: message || 'Got unwanted exception: ' + e,
                        actual: e,
                        operator: 'doesNotThrow'
                    });
                }
            }

            // assert.rejects - Expects promise to reject
            function rejects(promiseOrFn, errorOrMessage, message) {
                let promise;
                if (typeof promiseOrFn === 'function') {
                    promise = promiseOrFn();
                } else {
                    promise = promiseOrFn;
                }

                return Promise.resolve(promise).then(
                    function() {
                        throw new AssertionError({
                            message: message || 'Missing expected rejection',
                            operator: 'rejects'
                        });
                    },
                    function(err) {
                        return err;
                    }
                );
            }

            // assert.doesNotReject - Expects promise not to reject
            function doesNotReject(promiseOrFn, errorOrMessage, message) {
                let promise;
                if (typeof promiseOrFn === 'function') {
                    promise = promiseOrFn();
                } else {
                    promise = promiseOrFn;
                }

                return Promise.resolve(promise).catch(function(err) {
                    throw new AssertionError({
                        message: message || 'Got unwanted rejection: ' + err,
                        actual: err,
                        operator: 'doesNotReject'
                    });
                });
            }

            // assert.match - Expects string to match regexp
            function match(string, regexp, message) {
                if (!(regexp instanceof RegExp)) {
                    throw new TypeError('Second argument must be a RegExp');
                }
                if (!regexp.test(String(string))) {
                    throw new AssertionError({
                        message: message,
                        actual: string,
                        expected: regexp,
                        operator: 'match'
                    });
                }
            }

            // assert.doesNotMatch - Expects string not to match regexp
            function doesNotMatch(string, regexp, message) {
                if (!(regexp instanceof RegExp)) {
                    throw new TypeError('Second argument must be a RegExp');
                }
                if (regexp.test(String(string))) {
                    throw new AssertionError({
                        message: message,
                        actual: string,
                        expected: regexp,
                        operator: 'doesNotMatch'
                    });
                }
            }

            // assert.ifError - Throws if value is truthy
            function ifError(value) {
                if (value !== null && value !== undefined) {
                    if (value instanceof Error) {
                        throw value;
                    }
                    throw new AssertionError({
                        message: 'ifError got unwanted exception: ' + value,
                        actual: value,
                        operator: 'ifError'
                    });
                }
            }

            // Create the assert object (also callable as assert.ok)
            const assert = function(value, message) {
                return ok(value, message);
            };

            // Attach all functions
            assert.AssertionError = AssertionError;
            assert.ok = ok;
            assert.equal = equal;
            assert.notEqual = notEqual;
            assert.strictEqual = strictEqual;
            assert.notStrictEqual = notStrictEqual;
            assert.deepEqual = deepEqual;
            assert.notDeepEqual = notDeepEqual;
            assert.deepStrictEqual = deepStrictEqual;
            assert.notDeepStrictEqual = notDeepStrictEqual;
            assert.fail = fail;
            assert.throws = throws;
            assert.doesNotThrow = doesNotThrow;
            assert.rejects = rejects;
            assert.doesNotReject = doesNotReject;
            assert.match = match;
            assert.doesNotMatch = doesNotMatch;
            assert.ifError = ifError;

            // Create strict mode
            const strict = function(value, message) {
                return ok(value, message);
            };
            strict.AssertionError = AssertionError;
            strict.ok = ok;
            strict.equal = strictEqual;
            strict.notEqual = notStrictEqual;
            strict.strictEqual = strictEqual;
            strict.notStrictEqual = notStrictEqual;
            strict.deepEqual = deepStrictEqual;
            strict.notDeepEqual = notDeepStrictEqual;
            strict.deepStrictEqual = deepStrictEqual;
            strict.notDeepStrictEqual = notDeepStrictEqual;
            strict.fail = fail;
            strict.throws = throws;
            strict.doesNotThrow = doesNotThrow;
            strict.rejects = rejects;
            strict.doesNotReject = doesNotReject;
            strict.match = match;
            strict.doesNotMatch = doesNotMatch;
            strict.ifError = ifError;

            assert.strict = strict;

            return assert;
        })()
    "#;

    let assert_module = context.eval(Source::from_bytes(code))?;

    context
        .global_object()
        .set(js_string!("assert"), assert_module, false, context)?;

    Ok(())
}
