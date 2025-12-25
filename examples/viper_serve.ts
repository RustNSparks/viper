// Example: Using Viper.serve() - TypeScript HTTP Server API

console.log("Starting Viper HTTP server from TypeScript...");

const server = Viper.serve({
  port: 8080,
  hostname: "0.0.0.0",
  fetch(request) {
    console.log(`${request.method} ${request.url}`);

    // Simple routing
    if (request.url === "/" || request.url === "http://0.0.0.0:8080/") {
      return {
        status: 200,
        headers: { "content-type": "text/html" },
        body: `
<!DOCTYPE html>
<html>
<head>
    <title>Viper.serve() Demo</title>
    <style>
        body { font-family: Arial, sans-serif; max-width: 800px; margin: 50px auto; padding: 20px; }
        h1 { color: #2563eb; }
        code { background: #f3f4f6; padding: 2px 6px; border-radius: 3px; }
        .endpoint { background: #f9fafb; padding: 15px; margin: 10px 0; border-left: 3px solid #2563eb; }
    </style>
</head>
<body>
    <h1>🐍 Viper.serve() Demo</h1>
    <p>This HTTP server was started from TypeScript using <code>Viper.serve()</code>!</p>

    <h2>Available Endpoints:</h2>
    <div class="endpoint">
        <strong>GET /</strong> - This page
    </div>
    <div class="endpoint">
        <strong>GET /json</strong> - JSON API response
    </div>
    <div class="endpoint">
        <strong>GET /health</strong> - Health check
    </div>

    <h2>Features:</h2>
    <ul>
        <li>Fast HTTP server powered by Axum</li>
        <li>TypeScript-first API</li>
        <li>Similar to Bun.serve() and Deno.serve()</li>
        <li>Built on Viper runtime</li>
    </ul>
</body>
</html>
                `.trim(),
      };
    }

    if (request.url.includes("/json")) {
      return {
        status: 200,
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          message: "Hello from Viper.serve()!",
          timestamp: Date.now(),
          runtime: "Viper",
          poweredBy: "Boa + OXC + Axum",
        }),
      };
    }

    if (request.url.includes("/health")) {
      return {
        status: 200,
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          status: "ok",
          uptime: process.uptime ? process.uptime() : 0,
        }),
      };
    }

    // 404 for unknown routes
    return {
      status: 404,
      headers: { "content-type": "text/html" },
      body: "<h1>404 Not Found</h1><p>The requested page could not be found.</p>",
    };
  },
});

console.log(`🚀 Server started!`);
console.log(`   URL: ${server.url}`);
console.log(`   Hostname: ${server.hostname}`);
console.log(`   Port: ${server.port}`);
console.log("");
console.log("Press Ctrl+C to stop the server");

// Keep the process alive
setInterval(() => {
  // This keeps the event loop running so the server stays alive
}, 1000);
