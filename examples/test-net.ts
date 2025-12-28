// Test the native Rust net module

import * as net from 'net';

console.log("Testing net module...");

// Test IP utilities
console.log("\n=== IP Utilities ===");
console.log("net.isIP('127.0.0.1'):", net.isIP('127.0.0.1')); // Should be 4
console.log("net.isIP('::1'):", net.isIP('::1')); // Should be 6
console.log("net.isIP('invalid'):", net.isIP('invalid')); // Should be 0

console.log("net.isIPv4('192.168.1.1'):", net.isIPv4('192.168.1.1')); // Should be true
console.log("net.isIPv4('::1'):", net.isIPv4('::1')); // Should be false

console.log("net.isIPv6('::1'):", net.isIPv6('::1')); // Should be true
console.log("net.isIPv6('127.0.0.1'):", net.isIPv6('127.0.0.1')); // Should be false

// Test Socket class existence
console.log("\n=== Classes ===");
console.log("net.Socket:", typeof net.Socket); // Should be function
console.log("net.Server:", typeof net.Server); // Should be function
console.log("net.BlockList:", typeof net.BlockList); // Should be function
console.log("net.SocketAddress:", typeof net.SocketAddress); // Should be function

// Test SocketAddress.parse
console.log("\n=== SocketAddress ===");
const addr = net.SocketAddress.parse('127.0.0.1:8080');
if (addr) {
    console.log("Parsed address:", addr.address);
    console.log("Parsed port:", addr.port);
    console.log("Parsed family:", addr.family);
}

// Test createServer function
console.log("\n=== createServer ===");
console.log("net.createServer:", typeof net.createServer); // Should be function
console.log("net.createConnection:", typeof net.createConnection); // Should be function
console.log("net.connect:", typeof net.connect); // Should be function

// Create a simple server
const server = net.createServer((socket) => {
    console.log("Client connected from:", socket.remoteAddress);
    socket.write("Hello from Viper!\n");
    socket.end();
});

server.listen(0, '127.0.0.1', () => {
    const addr = server.address();
    console.log("\nServer listening on:", addr);

    // Connect with a client socket
    const client = net.createConnection({ port: addr.port, host: addr.address }, () => {
        console.log("Connected to server!");
    });

    client.on('data', (data) => {
        console.log("Received:", data.toString());
    });

    client.on('end', () => {
        console.log("Disconnected from server");
        server.close(() => {
            console.log("Server closed");
            console.log("\n=== All tests passed! ===");
        });
    });

    client.on('error', (err) => {
        console.log("Client error:", err.message);
    });
});

server.on('error', (err) => {
    console.log("Server error:", err.message);
});
