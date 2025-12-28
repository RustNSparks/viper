// Node.js http module example - 100% compatible API
// Run with: viper examples/13-http-module.ts

import http from 'http';

console.log('=== Node.js HTTP Module Demo ===\n');

// Example 1: Basic HTTP Server
console.log('1. Creating HTTP server...');

const server = http.createServer((req, res) => {
  console.log(`Request: ${req.method} ${req.url}`);

  // Route handling
  if (req.url === '/') {
    res.writeHead(200, { 'Content-Type': 'text/html' });
    res.end(`
      <!DOCTYPE html>
      <html>
        <head><title>Viper HTTP Server</title></head>
        <body>
          <h1>Welcome to Viper!</h1>
          <p>Node.js-compatible HTTP server</p>
          <ul>
            <li><a href="/api/hello">Hello API</a></li>
            <li><a href="/api/info">Server Info</a></li>
            <li><a href="/api/status">Status</a></li>
          </ul>
        </body>
      </html>
    `);
  } else if (req.url === '/api/hello') {
    res.setHeader('Content-Type', 'application/json');
    res.statusCode = 200;
    res.end(JSON.stringify({
      message: 'Hello from Viper!',
      timestamp: Date.now(),
      runtime: 'viper'
    }));
  } else if (req.url === '/api/info') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    const info = {
      server: 'Viper HTTP',
      version: process.version,
      nodeCompatible: true,
      method: req.method,
      headers: req.headers
    };
    res.end(JSON.stringify(info, null, 2));
  } else if (req.url === '/api/status') {
    // Demonstrate different status codes
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({
      status: 'OK',
      statusCode: 200,
      statusText: http.STATUS_CODES[200]
    }));
  } else if (req.url === '/redirect') {
    res.writeHead(302, { 'Location': '/' });
    res.end();
  } else {
    res.writeHead(404, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({
      error: 'Not Found',
      path: req.url,
      statusCode: 404,
      statusText: http.STATUS_CODES[404]
    }));
  }
});

// Server events
server.on('connection', (socket) => {
  console.log('New connection established');
});

server.on('request', (req, res) => {
  console.log(`Handling: ${req.method} ${req.url}`);
});

// Start listening
const PORT = 3000;
const HOST = '127.0.0.1';

server.listen(PORT, HOST, () => {
  console.log(`✓ Server running at http://${HOST}:${PORT}`);
  console.log('\nAvailable endpoints:');
  console.log('  GET  /              - Welcome page');
  console.log('  GET  /api/hello     - JSON greeting');
  console.log('  GET  /api/info      - Server info');
  console.log('  GET  /api/status    - Status check');
  console.log('  GET  /redirect      - Redirect test');
  console.log('\nPress Ctrl+C to stop\n');
});

// Example 2: Making HTTP requests
console.log('2. Testing HTTP client (http.request)...\n');

// Make a request to httpbin.org
const options = {
  hostname: 'httpbin.org',
  port: 80,
  path: '/get?foo=bar',
  method: 'GET',
  headers: {
    'User-Agent': 'Viper-HTTP/1.0'
  }
};

const req = http.request(options, (res) => {
  console.log(`Response: ${res.statusCode} ${res.statusMessage}`);
  console.log('Headers:', JSON.stringify(res.headers, null, 2));

  let data = '';

  res.on('data', (chunk) => {
    data += chunk;
  });

  res.on('end', () => {
    console.log('Response body:', data);
  });
});

req.on('error', (e) => {
  console.error(`Request error: ${e.message}`);
});

req.end();

// Example 3: Using http.get (shorthand)
console.log('\n3. Testing http.get...\n');

http.get('http://httpbin.org/user-agent', (res) => {
  console.log(`GET response: ${res.statusCode}`);

  res.on('data', (chunk) => {
    console.log('Data:', chunk.toString());
  });

  res.on('end', () => {
    console.log('GET request completed');
  });
}).on('error', (e) => {
  console.error(`GET error: ${e.message}`);
});

// Example 4: HTTP Methods
console.log('\n4. Available HTTP methods:');
console.log(http.METHODS.join(', '));

// Example 5: Status codes
console.log('\n5. HTTP Status Codes (sample):');
const sampleCodes = [200, 201, 204, 301, 302, 400, 401, 403, 404, 500, 502, 503];
sampleCodes.forEach(code => {
  console.log(`  ${code}: ${http.STATUS_CODES[code]}`);
});

// Example 6: Using Agent for connection pooling
console.log('\n6. Creating custom HTTP Agent...');

const agent = new http.Agent({
  keepAlive: true,
  maxSockets: 10,
  maxFreeSockets: 5
});

console.log('Agent created with keepAlive enabled');
console.log(`Max sockets: ${agent.maxSockets}`);
console.log(`Max free sockets: ${agent.maxFreeSockets}`);

// Make request with custom agent
const agentReq = http.request({
  hostname: 'httpbin.org',
  path: '/headers',
  agent: agent
}, (res) => {
  console.log(`Agent request status: ${res.statusCode}`);
  res.on('data', () => {});
  res.on('end', () => {
    console.log('Agent request completed');
    agent.destroy(); // Clean up
  });
});

agentReq.end();

// Graceful shutdown
process.on('SIGINT', () => {
  console.log('\nShutting down server...');
  server.close(() => {
    console.log('Server closed');
    process.exit(0);
  });
});
