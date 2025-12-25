// Example TypeScript file for Viper runtime

interface Person {
    name: string;
    age: number;
}

function greet(person: Person): string {
    return `Hello, ${person.name}! You are ${person.age} years old.`;
}

const user: Person = {
    name: "Alice",
    age: 30
};

console.log(greet(user));

// TypeScript type features
type StringOrNumber = string | number;

const values: StringOrNumber[] = [1, "hello", 42, "world"];
console.log("Values:", values);

// Arrow functions with type annotations
const add = (a: number, b: number): number => a + b;
console.log("2 + 3 =", add(2, 3));

// Generic function
function identity<T>(arg: T): T {
    return arg;
}

console.log("Identity:", identity<string>("TypeScript is awesome!"));

// Final result
"Viper TypeScript Runtime works!"
