// Basic Hello World example demonstrating Viper's TypeScript support

console.log("Hello from Viper!");
console.log(`Running on ${process.platform} (${process.arch})`);
console.log(`Viper version: ${process.version}`);

// TypeScript features work out of the box
interface User {
  name: string;
  age: number;
}

const user: User = { name: "Alice", age: 30 };
console.log(`User: ${user.name}, Age: ${user.age}`);

// Modern JavaScript features
const numbers = [1, 2, 3, 4, 5];
const doubled = numbers.map((n) => n * 2);
console.log(`Doubled: ${doubled.join(", ")}`);

// Destructuring and spread
const { name, ...rest } = user;
console.log(`Name: ${name}, Rest:`, rest);

// Template literals and tagged templates
const greeting = `Welcome, ${user.name}!`;
console.log(greeting);
