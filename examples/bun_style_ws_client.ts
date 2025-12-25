// Client for testing the Bun-style WebSocket server

console.log("=== WebSocket Client for Bun-style Server ===\n");

const ws = new WebSocket("ws://localhost:3000/ws");

ws.onopen = () => {
  console.log("✅ Connected to ws://localhost:3000/ws");

  // Send test messages
  ws.send("Hello from Viper client!");

  setTimeout(() => ws.send("How's the weather?"), 1000);
  setTimeout(() => ws.send("WebSockets are awesome!"), 2000);

  // Close after 3.5 seconds
  setTimeout(() => {
    console.log("\n👋 Closing connection...");
    ws.close();
  }, 3500);
};

ws.onmessage = (event) => {
  console.log("📨 Server:", event.data);
};

ws.onerror = (event) => {
  console.error("❌ Error:", event.message || "Connection failed");
  console.log("\nMake sure server is running:");
  console.log("  ./target/release/viper ./examples/bun_style_ws_server.ts");
};

ws.onclose = () => {
  console.log("✅ Connection closed");
  console.log("\n=== Test Complete ===");
};

console.log("Connecting to ws://localhost:3000/ws...\n");
