// Viper Ultra-Fast HTTP Server Demo
// Run with: cargo run --features server --release -- examples/server_demo.ts
//
// This server uses a single-threaded architecture with direct JS callback
// invocation for maximum performance - similar to Bun and Deno.

let requestCount = 0;
const startTime = Date.now();

Viper.serve({
  port: 8080,
  hostname: "127.0.0.1",

  fetch(request) {
    requestCount++;
    const url = request.url;
    const method = request.method;

    // Home page
    if (url === "/" || url === "") {
      const uptime = Math.floor((Date.now() - startTime) / 1000);
      return new Response(`
<!DOCTYPE html>
<html>
<head>
    <title>Viper Server</title>
    <style>
        body { font-family: system-ui, sans-serif; max-width: 800px; margin: 50px auto; padding: 20px; background: #f5f5f5; }
        h1 { color: #2563eb; }
        .card { background: white; padding: 20px; border-radius: 8px; margin: 15px 0; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        code { background: #e5e7eb; padding: 2px 6px; border-radius: 3px; }
        .stats { display: flex; gap: 20px; }
        .stat { text-align: center; }
        .stat-value { font-size: 24px; font-weight: bold; color: #2563eb; }
    </style>
</head>
<body>
    <h1>Viper Ultra-Fast HTTP Server</h1>

    <div class="card">
        <h2>Server Stats</h2>
        <div class="stats">
            <div class="stat">
                <div class="stat-value">${requestCount}</div>
                <div>Requests Served</div>
            </div>
            <div class="stat">
                <div class="stat-value">${uptime}s</div>
                <div>Uptime</div>
            </div>
        </div>
    </div>

    <div class="card">
        <h2>API Endpoints</h2>
        <p><code>GET /</code> - This page (dynamic)</p>
        <p><code>GET /api/hello</code> - JSON greeting</p>
        <p><code>GET /api/time</code> - Current timestamp</p>
        <p><code>GET /api/stats</code> - Server statistics</p>
        <p><code>GET /health</code> - Health check</p>
        <p><code>POST /api/echo</code> - Echo request body</p>
    </div>

    <div class="card">
        <h2>Architecture</h2>
        <ul>
            <li>Single-threaded async (like Bun/Deno)</li>
            <li>Direct JS callback invocation per request</li>
            <li>No thread synchronization overhead</li>
            <li>Powered by Hyper + Boa JS Engine</li>
        </ul>
    </div>
</body>
</html>
            `);
    }

    // JSON API endpoints
    if (url === "/api/hello") {
      return Response.json({
        message: "Hello from Viper!",
        runtime: "Viper",
        engine: "Boa",
        transpiler: "OXC",
      });
    }

    if (url === "/api/time") {
      return Response.json({
        timestamp: Date.now(),
        iso: new Date().toISOString(),
        requestNumber: requestCount,
      });
    }

    if (url === "/api/stats") {
      return Response.json({
        requests: requestCount,
        uptime: Date.now() - startTime,
        uptimeSeconds: Math.floor((Date.now() - startTime) / 1000),
      });
    }

    if (url === "/health") {
      return Response.json({ status: "ok" });
    }

    // Echo endpoint for POST requests
    if (url === "/api/echo" && method === "POST") {
      const body = request._body || "";
      return new Response(body, {
        headers: { "content-type": "application/json" },
      });
    }

    // 404 for unknown routes
    return new Response("Not Found", { status: 404 });
  },
});
