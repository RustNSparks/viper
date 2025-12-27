// Process spawning and shell execution

async function main() {
  console.log("=== Process Spawn Demo ===\n");

  // Simple command execution
  console.log("1. Simple command (echo):");
  const echo = await Viper.spawn("echo", ["Hello from spawned process!"]);
  console.log(`  stdout: ${echo.stdout.trim()}`);
  console.log(`  exit code: ${echo.exitCode}\n`);

  // Execute shell command
  console.log("2. Shell command execution:");
  const shell = await Viper.exec(
    process.platform === "win32" ? "echo %USERNAME%" : "echo $USER"
  );
  console.log(`  Current user: ${shell.stdout.trim()}`);
  console.log(`  exit code: ${shell.exitCode}\n`);

  // List directory
  console.log("3. Directory listing:");
  const ls = await Viper.spawn(
    process.platform === "win32" ? "cmd" : "ls",
    process.platform === "win32" ? ["/c", "dir", "/b"] : ["-la"]
  );
  const files = ls.stdout.trim().split("\n").slice(0, 5);
  files.forEach((f) => console.log(`  ${f}`));
  if (ls.stdout.trim().split("\n").length > 5) {
    console.log("  ...");
  }
  console.log("");

  // Get system info
  console.log("4. System information:");
  if (process.platform === "win32") {
    const info = await Viper.exec("systeminfo | findstr /C:\"OS Name\"");
    console.log(`  ${info.stdout.trim()}`);
  } else {
    const info = await Viper.exec("uname -a");
    console.log(`  ${info.stdout.trim()}`);
  }
  console.log("");

  // Working directory option
  console.log("5. Custom working directory:");
  const pwd = await Viper.spawn(
    process.platform === "win32" ? "cmd" : "pwd",
    process.platform === "win32" ? ["/c", "cd"] : [],
    { cwd: process.platform === "win32" ? "C:\\" : "/" }
  );
  console.log(`  Working dir: ${pwd.stdout.trim()}\n`);

  // Handle command errors
  console.log("6. Error handling:");
  try {
    const result = await Viper.spawn("nonexistent-command-12345", []);
    console.log(`  Exit code: ${result.exitCode}`);
  } catch (error) {
    console.log(`  Caught error: Command not found (expected)\n`);
  }

  console.log("=== Demo Complete ===");
}

main();
