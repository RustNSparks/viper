// Path module demonstration

import path from "path";

console.log("=== Path Module Demo ===\n");

// Platform-specific separators
console.log("1. Platform Separators:");
console.log(`  Path separator: "${path.sep}"`);
console.log(`  Delimiter: "${path.delimiter}"\n`);

// Join paths
console.log("2. path.join():");
console.log(`  join("src", "lib", "index.ts"): ${path.join("src", "lib", "index.ts")}`);
console.log(`  join("/home", "user", "..", "admin"): ${path.join("/home", "user", "..", "admin")}\n`);

// Resolve paths
console.log("3. path.resolve():");
console.log(`  resolve("."): ${path.resolve(".")}`);
console.log(`  resolve("src", "index.ts"): ${path.resolve("src", "index.ts")}\n`);

// Normalize paths
console.log("4. path.normalize():");
console.log(`  normalize("src//lib/../utils/./index.ts"): ${path.normalize("src//lib/../utils/./index.ts")}\n`);

// Path components
const testPath = "/home/user/projects/app/src/index.ts";
console.log(`5. Path Components for: "${testPath}"`);
console.log(`  dirname: ${path.dirname(testPath)}`);
console.log(`  basename: ${path.basename(testPath)}`);
console.log(`  basename (no ext): ${path.basename(testPath, ".ts")}`);
console.log(`  extname: ${path.extname(testPath)}\n`);

// Parse path
console.log("6. path.parse():");
const parsed = path.parse(testPath);
console.log(`  root: "${parsed.root}"`);
console.log(`  dir: "${parsed.dir}"`);
console.log(`  base: "${parsed.base}"`);
console.log(`  name: "${parsed.name}"`);
console.log(`  ext: "${parsed.ext}"\n`);

// Format path
console.log("7. path.format():");
const formatted = path.format({
  root: "/",
  dir: "/home/user/docs",
  name: "readme",
  ext: ".md",
});
console.log(`  format({ dir, name, ext }): ${formatted}\n`);

// Absolute path check
console.log("8. path.isAbsolute():");
console.log(`  isAbsolute("/home/user"): ${path.isAbsolute("/home/user")}`);
console.log(`  isAbsolute("./src"): ${path.isAbsolute("./src")}`);
console.log(`  isAbsolute("C:\\\\Users"): ${path.isAbsolute("C:\\Users")}\n`);

// Relative paths
console.log("9. path.relative():");
console.log(`  relative("/home/user/src", "/home/user/docs"): ${path.relative("/home/user/src", "/home/user/docs")}\n`);

// Cross-platform paths
console.log("10. Cross-Platform (posix vs win32):");
console.log(`  posix.join("a", "b", "c"): ${path.posix.join("a", "b", "c")}`);
console.log(`  win32.join("a", "b", "c"): ${path.win32.join("a", "b", "c")}`);

console.log("\n=== Demo Complete ===");
