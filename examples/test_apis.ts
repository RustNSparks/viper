// Test new Viper APIs

console.log("=== Process API ===");
console.log("process.argv:", process.argv);
console.log("process.pid:", process.pid);
console.log("process.platform:", process.platform);
console.log("process.arch:", process.arch);
console.log("process.version:", process.version);
console.log("process.cwd():", process.cwd());

console.log("\n=== Crypto API ===");
console.log("crypto.randomUUID():", crypto.randomUUID());
console.log("crypto.randomUUID():", crypto.randomUUID());

const bytes = crypto.randomBytes(16);
console.log("crypto.randomBytes(16):", bytes);
console.log("bytes.length:", bytes.length);

console.log("\n=== Viper.exec() ===");
const result = Viper.exec("echo Hello from Viper");
console.log("stdout:", result.stdout.trim());
console.log("exitCode:", result.exitCode);
console.log("success:", result.success);

console.log("\n=== Viper.spawn() ===");
const spawn = Viper.spawn("node", ["--version"]);
console.log("node --version:", spawn.text().trim());

console.log("\n=== Viper.$ template ===");
const name = "World";
const greeting = Viper.$`echo Hello ${name}`;
console.log("Viper.$`echo Hello ${name}`:", greeting.toString());

console.log("\n=== Viper.which() ===");
console.log("which node:", Viper.which("node"));

console.log("\n=== Viper.sleep() ===");
console.log("Viper.sleep available:", typeof Viper.sleep === "function");

console.log("\n=== process.env ===");
console.log("PATH exists:", "PATH" in process.env);
console.log("HOME/USERPROFILE:", process.env.HOME || process.env.USERPROFILE);

console.log("\n=== All tests passed! ===");
