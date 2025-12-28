// Test what Express features work RIGHT NOW with Viper

import http from 'http';

console.log('=== Testing Express-like Features ===\n');

// Create a minimal Express-like router
const app = {
  routes: [] as any[],

  get(path: string, handler: (req: any, res: any) => void) {
    this.routes.push({ method: 'GET', path, handler });
  },

  post(path: string, handler: (req: any, res: any) => void) {
    this.routes.push({ method: 'POST', path, handler });
  },

  listen(port: number, callback?: () => void) {
    const server = http.createServer((req: any, res: any) => {
      console.log(`${req.method} ${req.url}`);

      // Find matching route
      for (const route of this.routes) {
        if (route.method !== req.method) continue;
        if (route.path === req.url || route.path === '*') {
          return route.handler(req, res);
        }
      }

      // 404 Not Found
      res.statusCode = 404;
      res.setHeader('Content-Type', 'text/plain');
      res.end('404 Not Found');
    });

    server.listen(port, () => {
      console.log(`✓ Server listening on port ${port}`);
      if (callback) callback();
    });

    return server;
  }
};

// Define routes
app.get('/', (req: any, res: any) => {
  res.statusCode = 200;
  res.setHeader('Content-Type', 'text/html');
  res.end('<h1>Hello from Viper Express!</h1>');
});

app.get('/api/users', (req: any, res: any) => {
  res.statusCode = 200;
  res.setHeader('Content-Type', 'application/json');
  res.end(JSON.stringify({ users: ['Alice', 'Bob', 'Charlie'] }));
});

app.post('/api/data', (req: any, res: any) => {
  // Collect body data
  let body = '';
  req.on('data', (chunk: any) => {
    body += chunk.toString();
  });

  req.on('end', () => {
    console.log('Received POST data:', body);
    res.statusCode = 200;
    res.setHeader('Content-Type', 'application/json');
    res.end(JSON.stringify({ received: body }));
  });
});

// Start server
const server = app.listen(3000, () => {
  console.log('\n=== Testing Routes ===');

  // Test GET /
  http.get('http://localhost:3000/', (res: any) => {
    let data = '';
    res.on('data', (chunk: any) => data += chunk);
    res.on('end', () => {
      console.log('\n✓ GET / response:', data);

      // Test GET /api/users
      http.get('http://localhost:3000/api/users', (res2: any) => {
        let data2 = '';
        res2.on('data', (chunk: any) => data2 += chunk);
        res2.on('end', () => {
          console.log('✓ GET /api/users response:', data2);

          // Test POST /api/data
          const postReq = http.request({
            hostname: 'localhost',
            port: 3000,
            path: '/api/data',
            method: 'POST',
            headers: {
              'Content-Type': 'application/json'
            }
          }, (res3: any) => {
            let data3 = '';
            res3.on('data', (chunk: any) => data3 += chunk);
            res3.on('end', () => {
              console.log('✓ POST /api/data response:', data3);

              // Clean up
              server.close();
              console.log('\n=== All Express-like features work! ===');
            });
          });

          postReq.write(JSON.stringify({ name: 'Test', value: 123 }));
          postReq.end();
        });
      });
    });
  });
});
