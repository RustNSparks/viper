// Async/Await and Promises demonstration

// Simple async function
async function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

// Async iteration example
async function countdown(from: number): Promise<void> {
  for (let i = from; i > 0; i--) {
    console.log(`Countdown: ${i}`);
    await delay(100);
  }
  console.log("Liftoff!");
}

// Promise.all for parallel execution
async function fetchMultiple(): Promise<void> {
  console.log("Starting parallel tasks...");

  const results = await Promise.all([
    delay(100).then(() => "Task A complete"),
    delay(150).then(() => "Task B complete"),
    delay(50).then(() => "Task C complete"),
  ]);

  results.forEach((result) => console.log(result));
}

// Promise.race example
async function race(): Promise<void> {
  const winner = await Promise.race([
    delay(100).then(() => "Slow"),
    delay(50).then(() => "Fast"),
    delay(75).then(() => "Medium"),
  ]);
  console.log(`Winner: ${winner}`);
}

// Error handling with async/await
async function withErrorHandling(): Promise<void> {
  try {
    await Promise.reject(new Error("Something went wrong"));
  } catch (error) {
    console.log(`Caught error: ${(error as Error).message}`);
  }
}

// Run all demos
async function main(): Promise<void> {
  console.log("=== Async/Await Demo ===\n");

  await countdown(3);
  console.log("");

  await fetchMultiple();
  console.log("");

  await race();
  console.log("");

  await withErrorHandling();

  console.log("\n=== Demo Complete ===");
}

main();
