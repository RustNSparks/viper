// JSX/TSX demonstration - Server-side rendering

// Component types
interface Props {
  children?: any;
  [key: string]: any;
}

// Simple functional components
function Header({ title }: { title: string }) {
  return (
    <header>
      <h1>{title}</h1>
      <nav>
        <a href="/">Home</a>
        <a href="/about">About</a>
        <a href="/contact">Contact</a>
      </nav>
    </header>
  );
}

function Card({ title, children }: { title: string; children?: any }) {
  return (
    <div className="card">
      <h3>{title}</h3>
      <div className="card-body">{children}</div>
    </div>
  );
}

function List({ items }: { items: string[] }) {
  return (
    <ul>
      {items.map((item, i) => (
        <li key={i}>{item}</li>
      ))}
    </ul>
  );
}

function Footer({ year }: { year: number }) {
  return (
    <footer>
      <p>&copy; {year} Viper Runtime. All rights reserved.</p>
    </footer>
  );
}

// Main page component
function Page() {
  const features = [
    "TypeScript & TSX support",
    "Async/Await & Promises",
    "Fetch API",
    "Web Workers",
    "WebSocket client",
    "File system access",
    "HTTP server",
  ];

  return (
    <html lang="en">
      <head>
        <meta charSet="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <title>Viper Runtime</title>
        <style>{`
          body { font-family: system-ui, sans-serif; margin: 2rem; }
          header { border-bottom: 1px solid #ccc; padding-bottom: 1rem; }
          nav a { margin-right: 1rem; }
          .card { border: 1px solid #ddd; padding: 1rem; margin: 1rem 0; border-radius: 8px; }
          footer { margin-top: 2rem; color: #666; }
        `}</style>
      </head>
      <body>
        <Header title="Welcome to Viper" />

        <main>
          <Card title="About Viper">
            <p>
              Viper is a fast TypeScript runtime built with Rust, powered by the
              Boa JavaScript engine and OXC transpiler.
            </p>
          </Card>

          <Card title="Features">
            <List items={features} />
          </Card>

          <Card title="Quick Start">
            <pre>
              <code>{`# Install Viper
cargo install viper

# Run a TypeScript file
viper script.ts

# Start a development server
viper --features server server.ts`}</code>
            </pre>
          </Card>
        </main>

        <Footer year={new Date().getFullYear()} />
      </body>
    </html>
  );
}

// Render to HTML string
console.log("=== JSX/TSX Demo ===\n");
console.log("Rendering page to HTML...\n");

const html = renderToString(<Page />);

console.log("Generated HTML:");
console.log("─".repeat(50));
console.log(html);
console.log("─".repeat(50));
console.log(`\nTotal length: ${html.length} characters`);
console.log("\n=== Demo Complete ===");
