// HTTP Server demonstration using Viper.serve()
// Run with: viper --features server examples/06-http-server.ts

const routes: Record<string, (req: Request) => Response | Promise<Response>> = {
  "/": () => {
    return new Response(
      `<!DOCTYPE html>
<html>
<head><title>Viper Server</title></head>
<body>
  <h1>Welcome to Viper!</h1>
  <p>A fast TypeScript runtime built with Rust.</p>
  <ul>
    <li><a href="/api/hello">API: Hello</a></li>
    <li><a href="/api/time">API: Current Time</a></li>
    <li><a href="/api/echo">API: Echo (POST)</a></li>
  </ul>
</body>
</html>`,
      {
        headers: { "Content-Type": "text/html" },
      }
    );
  },

  "/api/hello": () => {
    return Response.json({
      message: "Hello from Viper!",
      runtime: "viper",
      version: process.version,
    });
  },

  "/api/time": () => {
    return Response.json({
      timestamp: Date.now(),
      iso: new Date().toISOString(),
      timezone: "UTC",
    });
  },

  "/api/echo": async (req: Request) => {
    if (req.method !== "POST") {
      return Response.json(
        { error: "Method not allowed. Use POST." },
        { status: 405 }
      );
    }

    try {
      const body = await req.json();
      return Response.json({
        received: body,
        timestamp: Date.now(),
      });
    } catch {
      return Response.json(
        { error: "Invalid JSON body" },
        { status: 400 }
      );
    }
  },
};

function handleRequest(req: Request): Response | Promise<Response> {
  const url = new URL(req.url);
  const handler = routes[url.pathname];

  if (handler) {
    return handler(req);
  }

  return Response.json(
    { error: "Not Found", path: url.pathname },
    { status: 404 }
  );
}

console.log("=== HTTP Server Demo ===\n");

const server = Viper.serve({
  port: 3000,
  hostname: "127.0.0.1",
  fetch: handleRequest,
});

console.log(`Server running at http://${server.hostname}:${server.port}`);
console.log("\nAvailable routes:");
console.log("  GET  /           - Welcome page");
console.log("  GET  /api/hello  - JSON greeting");
console.log("  GET  /api/time   - Current time");
console.log("  POST /api/echo   - Echo JSON body");
console.log("\nPress Ctrl+C to stop the server");
