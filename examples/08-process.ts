// Process API and environment demonstration

console.log("=== Process API Demo ===\n");

// Basic process info
console.log("1. Process Information:");
console.log(`  PID: ${process.pid}`);
console.log(`  Parent PID: ${process.ppid}`);
console.log(`  Platform: ${process.platform}`);
console.log(`  Architecture: ${process.arch}`);
console.log(`  Version: ${process.version}`);
console.log(`  Executable: ${process.execPath}`);
console.log(`  Current Directory: ${process.cwd()}\n`);

// Version details
console.log("2. Runtime Versions:");
console.log(`  Viper: ${process.versions.viper}`);
console.log(`  Boa Engine: ${process.versions.boa}`);
console.log(`  OXC Transpiler: ${process.versions.oxc}\n`);

// Command line arguments
console.log("3. Command Line Arguments:");
process.argv.forEach((arg, index) => {
  console.log(`  [${index}] ${arg}`);
});
console.log("");

// Environment variables (sample)
console.log("4. Environment Variables (sample):");
const envKeys = Object.keys(process.env).slice(0, 5);
envKeys.forEach((key) => {
  const value = process.env[key] || "";
  const display = value.length > 40 ? value.slice(0, 40) + "..." : value;
  console.log(`  ${key}: ${display}`);
});
console.log("");

// Memory usage
console.log("5. Memory Usage:");
const mem = process.memoryUsage();
console.log(`  RSS: ${(mem.rss / 1024 / 1024).toFixed(2)} MB`);
console.log(`  Heap Total: ${(mem.heapTotal / 1024 / 1024).toFixed(2)} MB`);
console.log(`  Heap Used: ${(mem.heapUsed / 1024 / 1024).toFixed(2)} MB`);
console.log(`  External: ${(mem.external / 1024 / 1024).toFixed(2)} MB\n`);

// CPU usage
console.log("6. CPU Usage:");
const cpu = process.cpuUsage();
console.log(`  User: ${(cpu.user / 1000).toFixed(2)} ms`);
console.log(`  System: ${(cpu.system / 1000).toFixed(2)} ms\n`);

// Uptime
console.log("7. Process Uptime:");
console.log(`  Uptime: ${process.uptime().toFixed(3)} seconds\n`);

// High-resolution time
console.log("8. High-Resolution Time:");
const start = process.hrtime();
let sum = 0;
for (let i = 0; i < 1000000; i++) sum += i;
const end = process.hrtime(start);
console.log(`  1M iterations took: ${end[0]}s ${(end[1] / 1000000).toFixed(3)}ms`);
console.log(`  Result: ${sum}\n`);

// nextTick
console.log("9. process.nextTick:");
console.log("  Scheduling nextTick...");
process.nextTick(() => {
  console.log("  nextTick callback executed!");
  console.log("\n=== Demo Complete ===");
});
console.log("  This runs before nextTick callback");
