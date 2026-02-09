---
title: Quick Start
description: Get started with naml in 5 minutes
---

## Hello World

Create a file called `hello.nm`:

```naml
fn main() {
    println("Hello, World!");
}
```

Run it:

```bash
naml run hello.nm
```

That's it! naml uses Cranelift JIT compilation for instant execution during development.

## A Simple Program

Let's build something slightly more interesting. Create `greeting.nm`:

```naml
struct Person {
    name: string,
    age: int
}

pub fn (self: Person) greet() -> string {
    return fmt("Hello, I'm {} and I'm {} years old.", self.name, self.age);
}

fn create_person(name: string, age: int) -> Person {
    return Person { name: name, age: age };
}

fn main() {
    var person: Person = create_person("Alice", 30);
    println(person.greet());

    var x: int = 42;
    var y: int = x + 8;
    println(fmt("The answer is {}", y));
}
```

Key points:
- All variables need type annotations: `var x: int = 42;`
- Structs define custom types
- Methods are defined with `pub fn (self: Type)`
- Functions have explicit parameter and return types
- Use `fmt()` for string formatting

Run it:

```bash
naml run greeting.nm
```

## Development Workflow

### Run (JIT)
Fast iteration with instant compilation:

```bash
naml run main.nm
```

### Run with Cache
Sub-millisecond cold start:

```bash
naml run --cached main.nm
```

### Type Check Only
Verify types without execution:

```bash
naml check main.nm
```

### Build Native Binary
Compile to optimized native executable:

```bash
naml build
./target/release/main
```

## Working with Modules

Create `math.nm`:

```naml
pub fn add(a: int, b: int) -> int {
    return a + b;
}

pub fn multiply(a: int, b: int) -> int {
    return a * b;
}
```

Use it in `main.nm`:

```naml
use math::*;

fn main() {
    var result: int = add(10, 20);
    println(fmt("10 + 20 = {}", result));

    var product: int = multiply(5, 6);
    println(fmt("5 * 6 = {}", product));
}
```

## Using the Standard Library

naml includes a rich standard library:

```naml
use std::random::*;
use std::datetime::*;
use std::threads::*;

fn main() {
    # Random numbers
    var dice: int = random(1, 6);
    println(fmt("Rolled a {}", dice));

    # Date and time
    var now: int = now_ms();
    var current_year: int = year();
    println(fmt("The year is {}", current_year));

    # Concurrency
    var ch: channel<int> = make_channel(10);

    spawn {
        send(ch, 42);
    };

    var value: int = receive(ch);
    println(fmt("Received: {}", value));
}
```

## Error Handling

Functions can throw errors:

```naml
fn divide(a: int, b: int) -> int throws string {
    if (b == 0) {
        throw "Division by zero";
    }
    return a / b;
}

fn main() {
    try {
        var result: int = divide(10, 2);
        println(fmt("Result: {}", result));

        var bad: int = divide(10, 0);  # This throws
    } catch (err: string) {
        println(fmt("Error: {}", err));
    }
}
```

## Next Steps

- Read about [naml's philosophy](/guide/3-philosophy) to understand the design principles
- Learn about [compilation targets](/guide/4-targets) for deploying your code
- Set up [packages and dependencies](/guide/5-packages) for multi-file projects
- Explore the [language reference](/language/1-types) for detailed syntax
- Check out [examples](/examples/) for real-world code
