// Fetch API demonstration - HTTP client functionality

async function main() {
  console.log("=== Fetch API Demo ===\n");

  // Simple GET request
  console.log("1. Simple GET request:");
  const response = await fetch("https://httpbin.org/get");
  const data = await response.json();
  console.log(`Status: ${response.status} ${response.statusText}`);
  console.log(`Origin: ${data.origin}\n`);

  // GET with headers
  console.log("2. GET with custom headers:");
  const withHeaders = await fetch("https://httpbin.org/headers", {
    headers: {
      "X-Custom-Header": "Viper-Runtime",
      "User-Agent": "Viper/1.0",
    },
  });
  const headerData = await withHeaders.json();
  console.log("Headers received by server:", headerData.headers, "\n");

  // POST request with JSON body
  console.log("3. POST with JSON body:");
  const postResponse = await fetch("https://httpbin.org/post", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      message: "Hello from Viper!",
      timestamp: Date.now(),
    }),
  });
  const postData = await postResponse.json();
  console.log("Posted data:", postData.json, "\n");

  // Response as text
  console.log("4. Response as text:");
  const textResponse = await fetch("https://httpbin.org/robots.txt");
  const text = await textResponse.text();
  console.log(text);

  // Using URL object
  console.log("5. Using URL with query parameters:");
  const url = new URL("https://httpbin.org/get");
  url.searchParams.set("name", "viper");
  url.searchParams.set("version", "1.0");
  const urlResponse = await fetch(url);
  const urlData = await urlResponse.json();
  console.log("Query args:", urlData.args, "\n");

  console.log("=== Demo Complete ===");
}

main();
