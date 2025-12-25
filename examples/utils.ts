// Utility module - demonstrates ES module exports

export function add(a: number, b: number): number {
  return a + b;
}

export function multiply(a: number, b: number): number {
  return a * b;
}

export const PI = 3.14159;

export interface User {
  name: string;
  age: number;
}

export function greet(user: User): string {
  return `Hello, ${user.name}! You are ${user.age} years old.`;
}

// Default export
export default function subtract(a: number, b: number): number {
  return a - b;
}
