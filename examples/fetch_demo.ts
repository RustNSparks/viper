// Fetch API Demo - HTTP requests built into Viper!

console.log("=== Viper Fetch API Demo ===\n");

// Test 1: Simple GET request
console.log("1. Fetching JSON from API...");
try {
  const response = await fetch("https://jsonplaceholder.typicode.com/todos/1");
  const data = await response.json();
  console.log("Response:", data);
  console.log("✓ Fetch successful\n");
} catch (e) {
  console.error("✗ Fetch failed:", e);
}

// Test 2: Check response status
console.log("2. Checking response status...");
try {
  const response = await fetch("https://httpbin.org/status/200");
  console.log("Status:", response.status);
  console.log("OK:", response.ok);
  console.log("✓ Status check successful\n");
} catch (e) {
  console.error("✗ Status check failed:", e);
}

// Test 3: Fetch text content
console.log("3. Fetching text content...");
try {
  const response = await fetch("https://httpbin.org/robots.txt");
  const text = await response.text();
  console.log("Text content (first 100 chars):");
  console.log(text.substring(0, 100) + "...");
  console.log("✓ Text fetch successful\n");
} catch (e) {
  console.error("✗ Text fetch failed:", e);
}

// Test 4: Multiple concurrent requests
console.log("4. Making concurrent requests...");
try {
  const urls = [
    "https://jsonplaceholder.typicode.com/users/1",
    "https://jsonplaceholder.typicode.com/users/2",
    "https://jsonplaceholder.typicode.com/users/3",
  ];

  const responses = await Promise.all(urls.map((url) => fetch(url)));
  const data = await Promise.all(responses.map((r) => r.json()));

  console.log(
    "Fetched users:",
    data.map((u) => u.name),
  );
  console.log("✓ Concurrent requests successful\n");
} catch (e) {
  console.error("✗ Concurrent requests failed:", e);
}

console.log("=== All fetch tests completed! ===");

export default {};
