// Test the OS module

console.log("=== OS Module Test ===\n");

console.log("os.EOL:", JSON.stringify(os.EOL));
console.log("os.devNull:", os.devNull);
console.log("os.arch():", os.arch());
console.log("os.platform():", os.platform());
console.log("os.type():", os.type());
console.log("os.release():", os.release());
console.log("os.version():", os.version());
console.log("os.machine():", os.machine());
console.log("os.hostname():", os.hostname());
console.log("os.homedir():", os.homedir());
console.log("os.tmpdir():", os.tmpdir());
console.log("os.endianness():", os.endianness());
console.log("os.availableParallelism():", os.availableParallelism());

console.log("\n=== Memory ===");
console.log("os.freemem():", os.freemem(), "bytes (", (os.freemem() / 1024 / 1024 / 1024).toFixed(2), "GB )");
console.log("os.totalmem():", os.totalmem(), "bytes (", (os.totalmem() / 1024 / 1024 / 1024).toFixed(2), "GB )");

console.log("\n=== System ===");
console.log("os.uptime():", os.uptime(), "seconds (", (os.uptime() / 3600).toFixed(2), "hours )");
console.log("os.loadavg():", os.loadavg());

console.log("\n=== CPUs ===");
const cpus = os.cpus();
console.log("CPU count:", cpus.length);
if (cpus.length > 0) {
    console.log("CPU model:", cpus[0].model);
    console.log("CPU speed:", cpus[0].speed, "MHz");
    console.log("CPU times (first core):", cpus[0].times);
}

console.log("\n=== User Info ===");
const userInfo = os.userInfo();
console.log("Username:", userInfo.username);
console.log("Home directory:", userInfo.homedir);
console.log("Shell:", userInfo.shell);
console.log("UID:", userInfo.uid);
console.log("GID:", userInfo.gid);

console.log("\n=== Network Interfaces ===");
const networkInterfaces = os.networkInterfaces();
for (const [name, interfaces] of Object.entries(networkInterfaces)) {
    console.log(`\n${name}:`);
    for (const iface of interfaces as any[]) {
        console.log(`  - ${iface.family}: ${iface.address}`);
        console.log(`    netmask: ${iface.netmask}, internal: ${iface.internal}`);
        if (iface.mac) console.log(`    mac: ${iface.mac}`);
    }
}

console.log("\n=== Priority ===");
console.log("Current process priority:", os.getPriority());

console.log("\n=== Constants ===");
console.log("os.constants.signals.SIGINT:", os.constants.signals.SIGINT);
console.log("os.constants.signals.SIGTERM:", os.constants.signals.SIGTERM);
console.log("os.constants.errno.ENOENT:", os.constants.errno.ENOENT);
console.log("os.constants.priority.PRIORITY_NORMAL:", os.constants.priority.PRIORITY_NORMAL);

console.log("\n=== OS Module Test Complete ===");
