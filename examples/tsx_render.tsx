// TSX HTML Rendering Demo

interface PageProps {
  title: string;
  description: string;
}

const Page = (props: PageProps) => {
  return (
    <html lang="en">
      <head>
        <meta charset="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>{props.title}</title>
        <style>{`
          body {
            font-family: system-ui, -apple-system, sans-serif;
            max-width: 800px;
            margin: 0 auto;
            padding: 2rem;
            line-height: 1.6;
          }
          .card {
            border: 1px solid #e0e0e0;
            border-radius: 8px;
            padding: 1.5rem;
            margin: 1rem 0;
            background: #f9f9f9;
          }
          .button {
            background: #0070f3;
            color: white;
            border: none;
            padding: 0.5rem 1rem;
            border-radius: 4px;
            cursor: pointer;
          }
        `}</style>
      </head>
      <body>
        <header>
          <h1>{props.title}</h1>
          <p>{props.description}</p>
        </header>

        <main>
          <div className="card">
            <h2>About Viper</h2>
            <p>
              Viper is a TypeScript runtime powered by Boa JS engine and OXC transpiler.
              It provides full support for TypeScript and TSX out of the box!
            </p>
          </div>

          <div className="card">
            <h2>Features</h2>
            <ul>
              <li>TypeScript transpilation with OXC</li>
              <li>TSX/JSX support with classic runtime</li>
              <li>Component-based development</li>
              <li>HTML rendering with renderToString</li>
              <li>Type-safe props and interfaces</li>
            </ul>
          </div>

          <div className="card">
            <h2>Get Started</h2>
            <p>Run TSX files directly:</p>
            <pre><code>viper your-file.tsx</code></pre>
            <br />
            <button className="button">Learn More</button>
          </div>
        </main>

        <footer>
          <hr />
          <p>Built with Viper Runtime | Powered by Boa & OXC</p>
        </footer>
      </body>
    </html>
  );
};

// Create the page
const page = (
  <Page
    title="Welcome to Viper TSX Runtime!"
    description="A powerful TypeScript and TSX runtime built with Rust"
  />
);

// Render to HTML string
const html = renderToString(page);

console.log("=== Rendered HTML ===\n");
console.log(html);

console.log("\n\n=== TSX Rendering Success! ===");
console.log("You can now build server-side rendered applications with Viper!");
