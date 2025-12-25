// Comprehensive TSX Demo for Viper Runtime

// Define TypeScript interfaces
interface ButtonProps {
  label: string;
  onClick?: () => void;
}

interface CardProps {
  title: string;
  description: string;
  children?: any;
}

// Button component
const Button = (props: ButtonProps) => {
  return (
    <button onClick={props.onClick}>
      {props.label}
    </button>
  );
};

// Card component with children
const Card = (props: CardProps) => {
  return (
    <div className="card">
      <h2>{props.title}</h2>
      <p>{props.description}</p>
      {props.children && <div className="card-content">{props.children}</div>}
    </div>
  );
};

// App component
const App = () => {
  return (
    <div className="app">
      <h1>Welcome to Viper TSX Runtime!</h1>
      <Card
        title="Feature 1"
        description="TypeScript support with type checking during development"
      >
        <p>You can write TypeScript with full type safety!</p>
      </Card>

      <Card
        title="Feature 2"
        description="JSX/TSX support for component-based development"
      >
        <ul>
          <li>Component composition</li>
          <li>Props and children</li>
          <li>Type-safe components</li>
        </ul>
      </Card>

      <Button label="Click Me!" onClick={() => console.log("Button clicked!")} />

      <div>
        <p>Nested elements work great:</p>
        <div>
          <span>Level 1</span>
          <div>
            <span>Level 2</span>
            <div>
              <span>Level 3</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

// Render the app
const app = <App />;

console.log("=== Viper TSX Demo ===\n");
console.log("App structure:");
console.log(JSON.stringify(app, (key, value) => {
  // Handle symbols
  if (typeof value === 'symbol') {
    return value.toString();
  }
  return value;
}, 2));

console.log("\n=== Success! ===");
console.log("TSX is fully functional in Viper runtime!");
