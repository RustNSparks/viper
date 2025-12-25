// JSX with Type Definitions Demo
// This demonstrates that JSX elements are fully typed!

// ============================================================================
// Simple JSX Components with Type Safety
// ============================================================================

// Function components are just functions that return JSX elements
function Greeting(props: { name: string; age?: number }) {
  return (
    <div className="greeting">
      <h1>Hello, {props.name}!</h1>
      {props.age && <p>Age: {props.age}</p>}
    </div>
  );
}

// Component with children
function Card(props: { title: string; children: any }) {
  return (
    <div className="card">
      <h2>{props.title}</h2>
      <div className="card-body">{props.children}</div>
    </div>
  );
}

// ============================================================================
// Type-safe JSX Usage
// ============================================================================

// TypeScript knows about all standard HTML elements and their props
const page = (
  <html>
    <head>
      <title>Viper JSX Demo</title>
      <style>{`
        .greeting { color: blue; }
        .card { border: 1px solid #ccc; padding: 1rem; }
      `}</style>
    </head>
    <body>
      <h1>Viper JSX with Type Definitions</h1>

      {/* TypeScript validates prop types */}
      <Greeting name="Viper" age={1} />

      {/* Component composition */}
      <Card title="Features">
        <ul>
          <li>Full TypeScript support</li>
          <li>JSX/TSX transpilation</li>
          <li>Type-safe file system API</li>
        </ul>
      </Card>

      {/* TypeScript knows about className, id, etc. */}
      <div className="footer" id="main-footer">
        <p>Powered by Viper Runtime</p>
      </div>
    </body>
  </html>
);

// ============================================================================
// Render to HTML String
// ============================================================================

const html = renderToString(page);
console.log(html);

// Write to file with type-safe file system API
await write("output.html", html);
console.log("\n✓ HTML written to output.html");

// Read it back (demonstrating the file API works with generated content)
const outputFile = file("output.html");
const fileSize = await outputFile.size();
console.log(`File size: ${fileSize} bytes`);

// Cleanup
await outputFile.delete();
console.log("✓ Cleanup complete");
