// Timers API demonstration (setTimeout, setInterval, queueMicrotask)

console.log("=== Timers API Demo ===\n");

let counter = 0;
const maxTicks = 5;

// setTimeout - delayed execution
console.log("1. setTimeout - delayed execution:");
console.log("  Scheduling task for 100ms...");

setTimeout(() => {
  console.log("  Delayed task executed!\n");

  // Start interval demo after timeout
  startIntervalDemo();
}, 100);

function startIntervalDemo() {
  console.log("2. setInterval - repeated execution:");

  const intervalId = setInterval(() => {
    counter++;
    console.log(`  Tick ${counter}/${maxTicks}`);

    if (counter >= maxTicks) {
      clearInterval(intervalId);
      console.log("  Interval cleared!\n");

      // Continue to next demo
      nestedTimerDemo();
    }
  }, 50);
}

function nestedTimerDemo() {
  console.log("3. Nested timers:");

  setTimeout(() => {
    console.log("  Outer timer (50ms)");
    setTimeout(() => {
      console.log("  Inner timer (25ms)");
      setTimeout(() => {
        console.log("  Innermost timer (10ms)\n");

        // Continue to microtask demo
        microtaskDemo();
      }, 10);
    }, 25);
  }, 50);
}

function microtaskDemo() {
  console.log("4. queueMicrotask - execution order:");
  console.log("  Scheduling microtasks and setTimeout...");

  setTimeout(() => console.log("  [3] setTimeout callback"), 0);

  queueMicrotask(() => {
    console.log("  [1] First microtask");
  });

  queueMicrotask(() => {
    console.log("  [2] Second microtask");
  });

  console.log("  [0] Synchronous code");

  // Final completion
  setTimeout(() => {
    console.log("\n5. Timer with arguments:");

    setTimeout(
      (a: number, b: number) => {
        console.log(`  Received arguments: a=${a}, b=${b}`);
        console.log(`  Sum: ${a + b}`);
        console.log("\n=== Demo Complete ===");
      },
      50,
      10,
      20
    );
  }, 100);
}

console.log("Starting timer demos...\n");
