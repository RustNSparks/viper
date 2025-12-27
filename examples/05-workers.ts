// Web Workers demonstration - parallel execution

// Create worker from inline script
const workerScript = `
  // Worker receives messages from main thread
  self.onmessage = (event) => {
    const { type, data } = event.data;

    switch (type) {
      case "compute":
        // Simulate CPU-intensive work
        let result = 0;
        for (let i = 0; i < data.iterations; i++) {
          result += Math.sqrt(i);
        }
        self.postMessage({ type: "result", value: result });
        break;

      case "echo":
        self.postMessage({ type: "echo", value: data });
        break;

      case "exit":
        self.postMessage({ type: "goodbye" });
        self.close();
        break;
    }
  };

  self.postMessage({ type: "ready" });
`;

// Write worker script to temp file
await write("./worker-temp.ts", workerScript);

console.log("=== Web Workers Demo ===\n");

const worker = new Worker("./worker-temp.ts");

worker.onopen = () => {
  console.log("Worker connection opened");
};

worker.onmessage = (event) => {
  const { type, value } = event.data;

  switch (type) {
    case "ready":
      console.log("Worker is ready!");

      // Send computation task
      console.log("\nSending computation task...");
      worker.postMessage({ type: "compute", data: { iterations: 100000 } });
      break;

    case "result":
      console.log(`Computation result: ${value.toFixed(2)}`);

      // Send echo task
      console.log("\nSending echo task...");
      worker.postMessage({
        type: "echo",
        data: { message: "Hello Worker!", timestamp: Date.now() }
      });
      break;

    case "echo":
      console.log("Echo received:", value);

      // Terminate worker
      console.log("\nTerminating worker...");
      worker.postMessage({ type: "exit" });
      break;

    case "goodbye":
      console.log("Worker said goodbye!");
      worker.unref(); // Allow process to exit
      console.log("\n=== Demo Complete ===");
      break;
  }
};

worker.onerror = (event) => {
  console.error("Worker error:", event.message);
};

worker.onclose = () => {
  console.log("Worker closed");
};
