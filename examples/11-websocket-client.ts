// WebSocket Client demonstration

console.log("=== WebSocket Client Demo ===\n");

// Connect to a public WebSocket echo server
const ws = new WebSocket("wss://ws.postman-echo.com/raw");

ws.onopen = () => {
  console.log("Connected to WebSocket server!\n");

  // Send text message
  console.log("Sending: Hello, WebSocket!");
  ws.send("Hello, WebSocket!");
};

ws.onmessage = (event) => {
  console.log(`Received: ${event.data}`);

  // Send a few more messages then close
  if (event.data === "Hello, WebSocket!") {
    console.log("\nSending: JSON message");
    ws.send(JSON.stringify({ type: "test", value: 42 }));
  } else if (event.data.includes("test")) {
    console.log("\nSending: Goodbye!");
    ws.send("Goodbye!");
  } else if (event.data === "Goodbye!") {
    console.log("\nClosing connection...");
    ws.close(1000, "Demo complete");
  }
};

ws.onerror = (event) => {
  console.error("WebSocket error:", event);
};

ws.onclose = (event) => {
  console.log(`\nConnection closed: code=${event.code}, reason="${event.reason}"`);
  console.log("Clean close:", event.wasClean);
  console.log("\n=== Demo Complete ===");
};

// Keep process alive for WebSocket events
console.log("Connecting to wss://ws.postman-echo.com/raw ...");
