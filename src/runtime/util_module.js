// Node.js util module wrapper
// This provides ESM and CommonJS compatibility for the util module

const u = globalThis.util;

// Export all util functions
export const promisify = u.promisify;
export const callbackify = u.callbackify;
export const format = u.format;
export const formatWithOptions = u.formatWithOptions;
export const inspect = u.inspect;
export const deprecate = u.deprecate;
export const isDeepStrictEqual = u.isDeepStrictEqual;
export const inherits = u.inherits;
export const debuglog = u.debuglog;
export const getSystemErrorName = u.getSystemErrorName;
export const getSystemErrorMap = u.getSystemErrorMap;

// Export types object
export const types = u.types;

// Export inspect.custom symbol for custom inspect functions
export const inspectCustom = Symbol.for('nodejs.util.inspect.custom');

// Default export
export default u;
