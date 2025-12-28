// Test URL module - Node.js compatible URL utilities

console.log("=== URL Module Test ===\n");

// Test WHATWG URL API (provided by boa_runtime)
console.log("--- WHATWG URL API ---");
const myURL = new URL(
  "https://user:pass@sub.example.com:8080/p/a/t/h?query=string#hash",
);
console.log("URL:", myURL.href);
console.log("  protocol:", myURL.protocol);
console.log("  username:", myURL.username);
console.log("  password:", myURL.password);
console.log("  hostname:", myURL.hostname);
console.log("  port:", myURL.port);
console.log("  pathname:", myURL.pathname);
console.log("  search:", myURL.search);
console.log("  hash:", myURL.hash);
console.log("  origin:", myURL.origin);

// Test URLSearchParams (native Rust implementation)
console.log("\n--- URLSearchParams ---");
const params = new URLSearchParams("foo=bar&baz=qux&foo=baz");
console.log("params.get('foo'):", params.get("foo"));
console.log("params.getAll('foo'):", JSON.stringify(params.getAll("foo")));
console.log("params.has('baz'):", params.has("baz"));
console.log("params.has('nonexistent'):", params.has("nonexistent"));
console.log("params.toString():", params.toString());
console.log("params.size:", params.size);

// Test append
params.append("new", "value");
console.log("After append('new', 'value'):", params.toString());

// Test set
params.set("foo", "updated");
console.log("After set('foo', 'updated'):", params.toString());

// Test delete
params.delete("baz");
console.log("After delete('baz'):", params.toString());

// Test sort
params.sort();
console.log("After sort():", params.toString());

// Test constructor variants
console.log("\n--- URLSearchParams constructors ---");
const fromObj = new URLSearchParams({ a: "1", b: "2" });
console.log("From object:", fromObj.toString());

const fromArray = new URLSearchParams([
  ["x", "1"],
  ["y", "2"],
  ["x", "3"],
]);
console.log("From array:", fromArray.toString());

const fromParams = new URLSearchParams(fromObj);
console.log("From URLSearchParams:", fromParams.toString());

// Test iteration
console.log("\n--- URLSearchParams iteration ---");
const iterParams = new URLSearchParams("a=1&b=2&c=3");
console.log("forEach:");
iterParams.forEach((value, key) => {
  console.log(`  ${key}: ${value}`);
});

console.log("keys():");
for (const key of iterParams.keys()) {
  console.log(`  ${key}`);
}

console.log("values():");
for (const value of iterParams.values()) {
  console.log(`  ${value}`);
}

console.log("entries():");
for (const [key, value] of iterParams.entries()) {
  console.log(`  ${key}=${value}`);
}

// Test url.parse (legacy API)
console.log("\n--- url.parse (legacy) ---");
const parsed = url.parse(
  "https://user:pass@example.com:8080/path/to/file?query=value#section",
);
console.log("url.parse result:");
console.log("  protocol:", parsed.protocol);
console.log("  auth:", parsed.auth);
console.log("  host:", parsed.host);
console.log("  hostname:", parsed.hostname);
console.log("  port:", parsed.port);
console.log("  pathname:", parsed.pathname);
console.log("  search:", parsed.search);
console.log("  query:", parsed.query);
console.log("  hash:", parsed.hash);
console.log("  slashes:", parsed.slashes);
console.log("  path:", parsed.path);

// Test url.parse with parseQueryString=true
console.log("\n--- url.parse with parseQueryString=true ---");
const parsedWithQuery = url.parse(
  "http://example.com/path?name=John&age=30&tags=a&tags=b",
  true,
);
console.log("query object:", JSON.stringify(parsedWithQuery.query));

// Test url.format
console.log("\n--- url.format ---");
const formatted = url.format({
  protocol: "https",
  hostname: "example.com",
  port: 443,
  pathname: "/some/path",
  query: { page: "1", format: "json" },
});
console.log("url.format result:", formatted);

// Test url.format with URL object
const urlObj = new URL("https://example.org/test?foo=bar#section");
console.log("url.format(URL object):", url.format(urlObj));

// Test url.resolve
console.log("\n--- url.resolve ---");
console.log(
  "url.resolve('/one/two/three', 'four'):",
  url.resolve("/one/two/three", "four"),
);
console.log(
  "url.resolve('http://example.com/', '/one'):",
  url.resolve("http://example.com/", "/one"),
);
console.log(
  "url.resolve('http://example.com/one', '/two'):",
  url.resolve("http://example.com/one", "/two"),
);

// Test url.domainToASCII
console.log("\n--- url.domainToASCII ---");
console.log(
  "url.domainToASCII('example.com'):",
  url.domainToASCII("example.com"),
);
console.log(
  "url.domainToASCII('xn--nxasmq5b'):",
  url.domainToASCII("xn--nxasmq5b"),
);

// Test url.domainToUnicode
console.log("\n--- url.domainToUnicode ---");
console.log(
  "url.domainToUnicode('xn--nxasmq5b'):",
  url.domainToUnicode("xn--nxasmq5b"),
);
console.log(
  "url.domainToUnicode('example.com'):",
  url.domainToUnicode("example.com"),
);

// Test url.fileURLToPath (Windows)
console.log("\n--- url.fileURLToPath ---");
try {
  console.log(
    "url.fileURLToPath('file:///C:/path/to/file.txt'):",
    url.fileURLToPath("file:///C:/path/to/file.txt"),
  );
  console.log(
    "url.fileURLToPath('file:///home/user/file.txt'):",
    url.fileURLToPath("file:///home/user/file.txt"),
  );
} catch (e) {
  console.log("fileURLToPath error:", e.message);
}

// Test url.pathToFileURL
console.log("\n--- url.pathToFileURL ---");
try {
  const fileUrl1 = url.pathToFileURL("C:\\Users\\test\\file.txt");
  console.log(
    "url.pathToFileURL('C:\\\\Users\\\\test\\\\file.txt'):",
    fileUrl1.href,
  );

  const fileUrl2 = url.pathToFileURL("/home/user/file.txt");
  console.log("url.pathToFileURL('/home/user/file.txt'):", fileUrl2.href);
} catch (e) {
  console.log("pathToFileURL error:", e.message);
}

// Test url.urlToHttpOptions
console.log("\n--- url.urlToHttpOptions ---");
const httpUrl = new URL(
  "https://user:pass@example.org:8080/path?query=value#hash",
);
const httpOptions = url.urlToHttpOptions(httpUrl);
console.log("urlToHttpOptions result:");
console.log("  protocol:", httpOptions.protocol);
console.log("  hostname:", httpOptions.hostname);
console.log("  port:", httpOptions.port);
console.log("  pathname:", httpOptions.pathname);
console.log("  path:", httpOptions.path);
console.log("  auth:", httpOptions.auth);
console.log("  search:", httpOptions.search);
console.log("  hash:", httpOptions.hash);

// Test URL and URLSearchParams are available on url module
console.log("\n--- Module exports ---");
console.log("url.URL === URL:", url.URL === URL);
console.log(
  "url.URLSearchParams === URLSearchParams:",
  url.URLSearchParams === URLSearchParams,
);

// Edge cases
console.log("\n--- Edge cases ---");
console.log("url.parse(''):", JSON.stringify(url.parse("")));
console.log(
  "url.parse('//host/path'):",
  JSON.stringify(url.parse("//host/path")),
);
console.log(
  "url.parse('path/to/file'):",
  JSON.stringify(url.parse("path/to/file")),
);

console.log("\n=== URL Module Test Complete ===");
