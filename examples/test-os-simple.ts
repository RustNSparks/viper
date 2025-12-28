// OS Module Test

console.log("=== Basic Info ===");
console.log("os.arch():", os.arch());
console.log("os.platform():", os.platform());
console.log("os.type():", os.type());
console.log("os.hostname():", os.hostname());
console.log("os.homedir():", os.homedir());
console.log("os.tmpdir():", os.tmpdir());

console.log("\n=== Memory ===");
console.log(
  "os.freemem():",
  (os.freemem() / 1024 / 1024 / 1024).toFixed(2),
  "GB",
);
console.log(
  "os.totalmem():",
  (os.totalmem() / 1024 / 1024 / 1024).toFixed(2),
  "GB",
);

console.log("\n=== System ===");
console.log("os.uptime():", (os.uptime() / 3600).toFixed(2), "hours");
console.log("os.endianness():", os.endianness());
console.log("os.availableParallelism():", os.availableParallelism());

console.log("\n=== CPUs ===");
const cpus = os.cpus();
console.log("CPU count:", cpus.length);
if (cpus.length > 0) {
  console.log("Model:", cpus[0].model);
  console.log("Speed:", cpus[0].speed, "MHz");
  console.log("Times:", JSON.stringify(cpus[0].times));
}

console.log("\n=== User Info ===");
const user = os.userInfo();
console.log("Username:", user.username);
console.log("UID:", user.uid);

console.log("\n=== Network Interfaces ===");
const nets = os.networkInterfaces();
const netNames = Object.keys(nets);
console.log("Interface count:", netNames.length);
for (const name of netNames.slice(0, 3)) {
  console.log("-", name + ":", nets[name].length, "addresses");
}

console.log("\n=== Constants ===");
console.log("SIGINT:", os.constants.signals.SIGINT);
console.log("ENOENT:", os.constants.errno.ENOENT);

console.log("\nDone!");
