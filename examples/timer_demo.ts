// Timer Parallelism Demo
console.log("=== Timer Parallelism Demo ===\n");

const start = Date.now();

function log(msg: string) {
    const elapsed = Date.now() - start;
    console.log(`[${elapsed}ms] ${msg}`);
}

log("Registering timers...");

// Register timers in reverse order of when they should fire
setTimeout(() => log("Timer D fired (500ms delay)"), 500);
setTimeout(() => log("Timer A fired (50ms delay)"), 50);
setTimeout(() => log("Timer C fired (300ms delay)"), 300);
setTimeout(() => log("Timer B fired (100ms delay)"), 100);

log("All timers registered - they run in PARALLEL");
log("Expected order: A (50ms), B (100ms), C (300ms), D (500ms)");
log("");

// Microtasks run before any timer
queueMicrotask(() => log("Microtask 1 - runs BEFORE all timers"));
queueMicrotask(() => log("Microtask 2 - runs BEFORE all timers"));

Promise.resolve().then(() => log("Promise - runs BEFORE all timers"));

log("Synchronous code done, waiting for async...\n");
