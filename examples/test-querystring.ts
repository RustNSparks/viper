// Test the querystring module

console.log("=== QueryString Module Test ===\n");

// Test parse
console.log("--- parse ---");
const parsed1 = querystring.parse("foo=bar&abc=xyz&abc=123");
console.log("parse('foo=bar&abc=xyz&abc=123'):");
console.log("  foo:", parsed1.foo);
console.log("  abc:", JSON.stringify(parsed1.abc));

const parsed2 = querystring.parse("name=John%20Doe&city=New+York");
console.log("\nparse('name=John%20Doe&city=New+York'):");
console.log("  name:", parsed2.name);
console.log("  city:", parsed2.city);

const parsed3 = querystring.parse("a=1;b=2;c=3", ";");
console.log("\nparse('a=1;b=2;c=3', ';'):");
console.log("  a:", parsed3.a);
console.log("  b:", parsed3.b);
console.log("  c:", parsed3.c);

const parsed4 = querystring.parse("x:10|y:20|z:30", "|", ":");
console.log("\nparse('x:10|y:20|z:30', '|', ':'):");
console.log("  x:", parsed4.x);
console.log("  y:", parsed4.y);
console.log("  z:", parsed4.z);

// Test stringify
console.log("\n--- stringify ---");
const str1 = querystring.stringify({ foo: "bar", baz: ["qux", "quux"], corge: "" });
console.log("stringify({ foo: 'bar', baz: ['qux', 'quux'], corge: '' }):");
console.log("  " + str1);

const str2 = querystring.stringify({ foo: "bar", baz: "qux" }, ";", ":");
console.log("\nstringify({ foo: 'bar', baz: 'qux' }, ';', ':'):");
console.log("  " + str2);

const str3 = querystring.stringify({ name: "John Doe", city: "New York" });
console.log("\nstringify({ name: 'John Doe', city: 'New York' }):");
console.log("  " + str3);

// Test escape
console.log("\n--- escape ---");
console.log("escape('hello world'):", querystring.escape("hello world"));
console.log("escape('foo=bar&baz=qux'):", querystring.escape("foo=bar&baz=qux"));
console.log("escape('特殊字符'):", querystring.escape("特殊字符"));

// Test unescape
console.log("\n--- unescape ---");
console.log("unescape('hello+world'):", querystring.unescape("hello+world"));
console.log("unescape('foo%3Dbar%26baz%3Dqux'):", querystring.unescape("foo%3Dbar%26baz%3Dqux"));
console.log("unescape('John%20Doe'):", querystring.unescape("John%20Doe"));

// Test aliases
console.log("\n--- aliases ---");
const decoded = querystring.decode("a=1&b=2");
console.log("decode('a=1&b=2'):", JSON.stringify(decoded));

const encoded = querystring.encode({ a: 1, b: 2 });
console.log("encode({ a: 1, b: 2 }):", encoded);

// Test round-trip
console.log("\n--- round-trip ---");
const original = { name: "John Doe", tags: ["a", "b", "c"], count: 42 };
const stringified = querystring.stringify(original);
console.log("Original:", JSON.stringify(original));
console.log("Stringified:", stringified);
const reparsed = querystring.parse(stringified);
console.log("Reparsed:", JSON.stringify(reparsed));

// Test edge cases
console.log("\n--- edge cases ---");
console.log("parse(''):", JSON.stringify(querystring.parse("")));
console.log("parse('='):", JSON.stringify(querystring.parse("=")));
console.log("parse('a'):", JSON.stringify(querystring.parse("a")));
console.log("parse('a='):", JSON.stringify(querystring.parse("a=")));
console.log("parse('=b'):", JSON.stringify(querystring.parse("=b")));
console.log("stringify({}):", querystring.stringify({}));
console.log("stringify(null):", querystring.stringify(null));

console.log("\n=== QueryString Module Test Complete ===");
