
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
