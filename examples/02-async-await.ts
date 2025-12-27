// Async/Await and Promises demonstration

// Promise basics
async function promiseDemo(): Promise<void> {
  console.log("1. Basic Promise:");
  const result = await Promise.resolve("Hello from Promise!");
  console.log(`   ${result}\n`);
}

// Promise.all for parallel execution
async function parallelDemo(): Promise<void> {
  console.log("2. Promise.all (parallel execution):");

  const results = await Promise.all([
    Promise.resolve("Task A complete"),
    Promise.resolve("Task B complete"),
    Promise.resolve("Task C complete"),
  ]);

  results.forEach((result) => console.log(`   ${result}`));
  console.log("");
}

// Promise.race example
async function raceDemo(): Promise<void> {
  console.log("3. Promise.race:");

  const winner = await Promise.race([
    Promise.resolve("First"),
    Promise.resolve("Second"),
    Promise.resolve("Third"),
  ]);
  console.log(`   Winner: ${winner}\n`);
}

// Promise.allSettled
async function allSettledDemo(): Promise<void> {
  console.log("4. Promise.allSettled:");

  const results = await Promise.allSettled([
    Promise.resolve("Success 1"),
    Promise.reject(new Error("Failed")),
    Promise.resolve("Success 2"),
  ]);

  results.forEach((result, i) => {
    if (result.status === "fulfilled") {
      console.log(`   [${i}] Fulfilled: ${result.value}`);
    } else {
      console.log(`   [${i}] Rejected: ${result.reason.message}`);
    }
  });
  console.log("");
}

// Chaining promises
async function chainingDemo(): Promise<void> {
  console.log("5. Promise chaining:");

  const result = await Promise.resolve(1)
    .then((x) => x + 1)
    .then((x) => x * 2)
    .then((x) => `Result: ${x}`);

  console.log(`   ${result}\n`);
}

// Error handling with async/await
async function errorHandlingDemo(): Promise<void> {
  console.log("6. Error handling:");

  try {
    await Promise.reject(new Error("Something went wrong"));
  } catch (error) {
    console.log(`   Caught: ${(error as Error).message}`);
  }

  // Using .catch()
  const handled = await Promise.reject(new Error("Another error")).catch(
    (e) => `Handled: ${e.message}`,
  );
  console.log(`   ${handled}\n`);
}

// Async function returning values
async function asyncReturnDemo(): Promise<void> {
  console.log("7. Async function return values:");

  async function getData(): Promise<{ name: string; value: number }> {
    return { name: "test", value: 42 };
  }

  const data = await getData();
  console.log(`   Name: ${data.name}, Value: ${data.value}\n`);
}

// Sequential vs Parallel
async function sequentialVsParallel(): Promise<void> {
  console.log("8. Sequential vs Parallel:");

  // Simulated async operations
  const op = (name: string) => Promise.resolve(`${name} done`);

  // Sequential
  const a = await op("A");
  const b = await op("B");
  console.log(`   Sequential: ${a}, ${b}`);

  // Parallel
  const [c, d] = await Promise.all([op("C"), op("D")]);
  console.log(`   Parallel: ${c}, ${d}\n`);
}

// Run all demos
async function main(): Promise<void> {
  console.log("=== Async/Await Demo ===\n");

  await promiseDemo();
  await parallelDemo();
  await raceDemo();
  await allSettledDemo();
  await chainingDemo();
  await errorHandlingDemo();
  await asyncReturnDemo();
  await sequentialVsParallel();

  console.log("=== Demo Complete ===");
}

main();
