# naml Language Reference

A comprehensive guide to the naml programming language syntax and features.

---

## Table of Contents

1. [Overview](#overview)
2. [Primitive Types](#primitive-types)
3. [Composite Types](#composite-types)
4. [Literals](#literals)
5. [Operators](#operators)
6. [Variables](#variables)
7. [Control Flow](#control-flow)
8. [Functions](#functions)
9. [Methods](#methods)
10. [Structs](#structs)
11. [Enums](#enums)
12. [Interfaces](#interfaces)
13. [Exceptions](#exceptions)
14. [Generics](#generics)
15. [Lambdas](#lambdas)
16. [Concurrency](#concurrency)
17. [Pattern Matching](#pattern-matching)
18. [Modules and Imports](#modules-and-imports)
19. [External Functions](#external-functions)
20. [Comments](#comments)
21. [Keywords](#keywords)

---

## Overview

naml is a fast, cross-platform programming language designed for:

- **Native**: Self-contained binaries with embedded runtime
- **Server WASM**: Server-side WebAssembly execution
- **Browser WASM**: Client-side browser execution

Key design principles:
- Zero-allocation where possible
- Zero-copy string handling
- Deterministic memory management (no GC)
- Strong static typing with inference

---

## Primitive Types

naml provides the following primitive types:

| Type | Description | Size |
|------|-------------|------|
| `int` | Signed integer | 64-bit |
| `uint` | Unsigned integer | 64-bit |
| `float` | Floating-point number | 64-bit |
| `bool` | Boolean value | - |
| `string` | UTF-8 encoded text | heap |
| `bytes` | Raw binary data | heap |

### Examples

```naml
var age: int = 25;
var count: uint = 100;
var pi: float = 3.14159;
var active: bool = true;
var name: string = "Alice";
var data: bytes = "hello" as bytes;
```

---

## Composite Types

### Arrays

Dynamic arrays that can grow:

```naml
var numbers: [int] = [1, 2, 3, 4, 5];
var empty: [string] = [];
```

Fixed-size arrays:

```naml
var fixed: [int; 5] = [1, 2, 3, 4, 5];
```

Array operations:

```naml
var first = numbers[0];      // Indexing
var len = numbers.len();     // Length
numbers.push(6);             // Append
var last = numbers.pop();    // Remove last
```

### Maps

Key-value collections:

```naml
var ages: map<string, int>;
```

### Options

Optional values (nullable):

```naml
var maybe: option<int> = some(42);
var nothing: option<int> = none;

// Null coalescing
var value = maybe ?? 0;  // Returns 42
var other = nothing ?? -1;  // Returns -1
```

### Channels

Communication channels for concurrency (native/server only):

```naml
var ch: channel<int>;
```

### Function Types

First-class function types:

```naml
var callback: fn(int, int) -> int;
var predicate: fn(string) -> bool;
```

---

## Literals

### Numeric Literals

```naml
// Integers
var a = 42;
var b = 1_000_000;      // Underscores for readability
var c = -100;

// Floats
var pi = 3.14159;
var scientific = 1.5e10;
```

### String Literals

```naml
var greeting = "Hello, World!";
var with_quotes = "She said \"hi\"";
var with_newline = "Line 1\nLine 2";
```

### Boolean Literals

```naml
var yes = true;
var no = false;
```

### Option Literals

```naml
var present = some(42);
var absent = none;
```

### Array Literals

```naml
var numbers = [1, 2, 3, 4, 5];
var strings = ["a", "b", "c"];
var nested = [[1, 2], [3, 4]];
```

---

## Operators

### Arithmetic Operators

| Operator | Description |
|----------|-------------|
| `+` | Addition |
| `-` | Subtraction |
| `*` | Multiplication |
| `/` | Division |
| `%` | Modulo (remainder) |

```naml
var sum = 10 + 5;       // 15
var diff = 10 - 5;      // 5
var prod = 10 * 5;      // 50
var quot = 10 / 5;      // 2
var rem = 10 % 3;       // 1
```

### Comparison Operators

| Operator | Description |
|----------|-------------|
| `==` | Equal |
| `!=` | Not equal |
| `<` | Less than |
| `<=` | Less than or equal |
| `>` | Greater than |
| `>=` | Greater than or equal |

```naml
var eq = 5 == 5;        // true
var neq = 5 != 3;       // true
var lt = 3 < 5;         // true
var lte = 5 <= 5;       // true
var gt = 5 > 3;         // true
var gte = 5 >= 5;       // true
```

### Logical Operators

| Operator | Description |
|----------|-------------|
| `and`, `&&` | Logical AND |
| `or`, `\|\|` | Logical OR |
| `not`, `!` | Logical NOT |

```naml
var both = true and false;    // false
var either = true or false;   // true
var negated = not true;       // false
```

### Bitwise Operators

| Operator | Description |
|----------|-------------|
| `&` | Bitwise AND |
| `\|` | Bitwise OR |
| `^` | Bitwise XOR |
| `~` | Bitwise NOT |
| `<<` | Left shift |
| `>>` | Right shift |

```naml
var band = 5 & 3;       // 1
var bor = 5 | 3;        // 7
var bxor = 5 ^ 3;       // 6
var lshift = 1 << 4;    // 16
var rshift = 16 >> 2;   // 4
```

### Assignment Operators

| Operator | Description |
|----------|-------------|
| `=` | Assignment |
| `+=` | Add and assign |
| `-=` | Subtract and assign |
| `*=` | Multiply and assign |
| `/=` | Divide and assign |
| `%=` | Modulo and assign |
| `&=` | Bitwise AND and assign |
| `\|=` | Bitwise OR and assign |
| `^=` | Bitwise XOR and assign |

```naml
var x = 10;
x += 5;     // x = 15
x -= 3;     // x = 12
x *= 2;     // x = 24
```

### Range Operators

| Operator | Description |
|----------|-------------|
| `..` | Exclusive range |
| `..=` | Inclusive range |

```naml
var exclusive = 0..5;    // 0, 1, 2, 3, 4
var inclusive = 0..=5;   // 0, 1, 2, 3, 4, 5
```

### Other Operators

| Operator | Description |
|----------|-------------|
| `??` | Null coalescing |
| `as` | Type casting |
| `is` | Type/variant check |

```naml
var value = optional ?? default;
var str = number as string;
```

---

## Variables

### Mutable Variables

Use `var` to declare mutable variables:

```naml
var x: int = 10;
var y = 20;          // Type inferred
var mut z = 30;      // Explicitly mutable
```

### Variable with Else Block

Handle initialization failures:

```naml
var value = get_optional() else {
    // Handle none case
    return -1;
};
```

---

## Control Flow

### If/Else

```naml
if (condition) {
    // then branch
} else if (other_condition) {
    // else if branch
} else {
    // else branch
}
```

If as expression:

```naml
var result = if (x > 0) { "positive" } else { "non-positive" };
```

### While Loop

```naml
while (condition) {
    // loop body
}
```

### Loop (Infinite)

```naml
loop {
    // runs forever until break
    if (done) {
        break;
    }
}
```

### For Loop

Iterate over ranges:

```naml
for (i in 0..10) {
    // i: 0, 1, 2, ..., 9
}

for (i in 0..=10) {
    // i: 0, 1, 2, ..., 10 (inclusive)
}
```

Iterate over collections:

```naml
for (item in array) {
    // iterate items
}

for (index, item in array) {
    // index and item
}
```

### Break and Continue

```naml
for (i in 0..100) {
    if (i == 50) {
        break;      // Exit loop
    }
    if (i % 2 == 0) {
        continue;   // Skip to next iteration
    }
}
```

### Switch/Case

```naml
switch (value) {
    case 1: {
        // handle 1
    }
    case 2: {
        // handle 2
    }
    default: {
        // default case
    }
}
```

Pattern matching with enums:

```naml
switch (status) {
    case Status::Active: {
        // handle active
    }
    case Status::Suspended(reason): {
        // handle suspended, bind reason
    }
    case _: {
        // wildcard match
    }
}
```

### Return

```naml
fn example() -> int {
    return 42;
}

fn no_return() {
    return;     // Return with no value
}
```

---

## Functions

### Basic Function

```naml
fn add(a: int, b: int) -> int {
    return a + b;
}
```

### Function Without Return Type

```naml
fn greet(name: string) {
    println("Hello, ");
    println(name);
}
```

### Public Functions

```naml
pub fn public_function() -> int {
    return 42;
}
```

### Functions with Exceptions

```naml
fn divide(a: int, b: int) -> int throws DivisionByZero {
    if (b == 0) {
        throw DivisionByZero("Cannot divide by zero");
    }
    return a / b;
}

// Multiple exception types
fn process(input: string) -> int throws ParseError, ValidationError {
    // ...
}
```

---

## Methods

Methods are functions with a receiver (first parameter is `self`):

### Basic Method

```naml
pub fn (self: Point) get_x() -> int {
    return self.x;
}

pub fn (self: Point) get_y() -> int {
    return self.y;
}
```

### Mutable Methods

```naml
pub fn (mut self: Counter) increment() {
    self.value = self.value + 1;
}

pub fn (mut self: Counter) reset() {
    self.value = 0;
}
```

### Method Calls

```naml
var point = Point { x: 10, y: 20 };
var x = point.get_x();    // Method call
var y = point.get_y();
```

---

## Structs

### Struct Definition

```naml
struct Point {
    x: int,
    y: int
}
```

### Public Struct with Public Fields

```naml
pub struct Rectangle {
    pub width: int,
    pub height: int
}
```

### Struct Instantiation

```naml
var point = Point { x: 10, y: 20 };
var rect = Rectangle { width: 100, height: 50 };
```

### Struct with Interface Implementation

```naml
struct Circle implements Shape {
    radius: float
}

pub fn (self: Circle) area() -> float {
    return 3.14159 * self.radius * self.radius;
}
```

### Generic Structs

```naml
pub struct Box<T> {
    pub value: T
}

pub struct Pair<A, B> {
    pub first: A,
    pub second: B
}
```

Usage:

```naml
var int_box = Box<int> { value: 42 };
var pair = Pair<string, int> { first: "age", second: 25 };
```

---

## Enums

### Simple Enum

```naml
enum Status {
    Active,
    Inactive,
    Pending
}
```

### Enum with Associated Data

```naml
enum UserStatus {
    Active,
    Suspended(string),
    Banned(string, int)
}

enum Result<T, E> {
    Ok(T),
    Err(E)
}
```

### Enum Construction

```naml
var status = Status::Active;
var suspended = UserStatus::Suspended("Policy violation");
```

### Enum Pattern Matching

```naml
switch (status) {
    case UserStatus::Active: {
        println("User is active");
    }
    case UserStatus::Suspended(reason): {
        println("Suspended: ");
        println(reason);
    }
    case UserStatus::Banned(reason, days): {
        println("Banned for days: ");
        print_int(days);
    }
}
```

---

## Interfaces

### Interface Definition

```naml
interface Describable {
    fn describe() -> string;
}

interface Comparable<T> {
    fn compare(other: T) -> int;
}
```

### Implementing Interfaces

```naml
struct Person implements Describable {
    name: string,
    age: int
}

pub fn (self: Person) describe() -> string {
    return self.name;
}
```

### Generic Interface with Bounds

```naml
fn max_value<T: Comparable<T>>(a: T, b: T) -> T {
    if (a.compare(b) > 0) {
        return a;
    }
    return b;
}
```

---

## Exceptions

### Exception Definition

```naml
exception DivisionByZero {
    dividend: int
}

exception ValidationError {
    field: string,
    message: string
}
```

### Throwing Exceptions

```naml
fn divide(a: int, b: int) -> int throws DivisionByZero {
    if (b == 0) {
        var ex = DivisionByZero("Cannot divide by zero");
        ex.dividend = a;
        throw ex;
    }
    return a / b;
}
```

### Catching Exceptions

```naml
var result = divide(10, 0) catch e {
    println("Error: ");
    println(e.message());
} ?? -1;
```

### Try Expression

```naml
var value = try risky_operation();
```

---

## Generics

### Generic Functions

```naml
fn identity<T>(x: T) -> T {
    return x;
}

fn swap<A, B>(pair: Pair<A, B>) -> Pair<B, A> {
    return Pair<B, A> { first: pair.second, second: pair.first };
}
```

### Generic Structs

```naml
pub struct Container<T> {
    pub items: [T]
}

pub fn (self: Container<T>) add<T>(item: T) {
    self.items.push(item);
}
```

### Generic Constraints

```naml
fn max<T: Comparable<T>>(a: T, b: T) -> T {
    if (a.compare(b) >= 0) {
        return a;
    }
    return b;
}
```

---

## Lambdas

### Lambda Expressions

```naml
var add = fn (a: int, b: int) -> int { a + b };
var square = fn (x: int) -> int { x * x };
```

### Type Inference in Lambdas

```naml
var double = fn (x) { x * 2 };
```

### Lambdas as Parameters

```naml
fn apply(f: fn(int) -> int, x: int) -> int {
    return f(x);
}

var result = apply(fn (n) { n * n }, 5);  // 25
```

---

## Concurrency

### Spawn

Create concurrent tasks with `spawn`:

```naml
spawn {
    // This runs concurrently
    do_work();
};

spawn {
    // Another concurrent task
    process_data();
};
```

### Channels

Communicate between concurrent tasks:

```naml
var ch: channel<int>;

spawn {
    ch.send(42);
};

var value = ch.receive();
```

### Wait All

Wait for all spawned tasks to complete:

```naml
spawn { task1(); };
spawn { task2(); };
spawn { task3(); };

wait_all();  // Block until all complete
```

---

## Pattern Matching

### In Switch Statements

```naml
switch (value) {
    // Literal patterns
    case 1: { }
    case "hello": { }
    case true: { }

    // Identifier pattern (binds value)
    case x: { }

    // Enum variant patterns
    case Status::Active: { }
    case Status::Suspended(reason): { }

    // Wildcard pattern
    case _: { }

    // Default
    default: { }
}
```

### Enum Destructuring

```naml
switch (result) {
    case Result::Ok(value): {
        println("Success: ");
        print_int(value);
    }
    case Result::Err(error): {
        println("Error: ");
        println(error);
    }
}
```

---

## Modules and Imports

### Import Module

```naml
import mymodule;
import my.nested.module;
import other_module as om;
```

### Use Statement

```naml
use mymodule;
```

---

## External Functions

### Extern Declaration

Declare external (FFI) functions:

```naml
extern fn abs(x: int) -> int;
extern fn strlen(s: string) -> int;
```

### Extern with Exceptions

```naml
extern fn risky_c_func() -> int throws CError;
```

---

## Comments

### Single-Line Comments

```naml
// This is a single-line comment
var x = 10;  // Inline comment
```

### Block Comments

```naml
/* This is a
   block comment */

/* Nested /* comments */ are supported */
```

### Documentation Comments

```naml
///
/// This function calculates the sum of two integers.
///
/// Parameters:
/// - a: The first integer
/// - b: The second integer
///
/// Returns: The sum of a and b
///
fn add(a: int, b: int) -> int {
    return a + b;
}
```

---

## Keywords

### Declaration Keywords
`fn`, `var`, `const`, `mut`, `pub`, `struct`, `enum`, `interface`, `exception`, `extern`

### Control Flow Keywords
`if`, `else`, `while`, `for`, `in`, `loop`, `break`, `continue`, `return`, `switch`, `case`, `default`

### Error Handling Keywords
`throw`, `throws`, `try`, `catch`

### Type Keywords
`int`, `uint`, `float`, `bool`, `string`, `bytes`, `option`, `map`, `channel`

### Boolean/Option Keywords
`true`, `false`, `none`, `some`

### Logical Keywords
`and`, `or`, `not`

### Other Keywords
`spawn`, `as`, `is`, `implements`, `use`, `import`, `platforms`, `native`, `server`, `browser`

---

## Type System

### Type Inference

naml has powerful type inference:

```naml
var x = 42;                              // int
var y = 3.14;                            // float
var z = "hello";                         // string
var pair = Pair { first: 1, second: 2 }; // Pair<int, int>
```

### Type Annotations

Explicit type annotations when needed:

```naml
var x: int = 42;
var arr: [int] = [];
var map: map<string, int>;
```

---

## Best Practices

1. **Use type inference** when types are obvious
2. **Add type annotations** for function signatures and complex expressions
3. **Handle errors** with exceptions and catch blocks
4. **Use meaningful names** for variables and functions
5. **Document public APIs** with documentation comments
6. **Keep functions small** and focused on one task

---

## Example Program

```naml
///
/// A simple example demonstrating naml features
///

extern fn println(s: string);
extern fn print_int(x: int);

struct Person {
    name: string,
    age: int
}

pub fn (self: Person) greet() -> string {
    return "Hello, I am " + self.name;
}

pub fn (self: Person) is_adult() -> bool {
    return self.age >= 18;
}

exception InvalidAge {
    value: int
}

fn create_person(name: string, age: int) -> Person throws InvalidAge {
    if (age < 0) {
        var ex = InvalidAge("Age cannot be negative");
        ex.value = age;
        throw ex;
    }
    return Person { name: name, age: age };
}

fn main() {
    var person = create_person("Alice", 25) catch e {
        println("Error creating person");
    } ?? Person { name: "Unknown", age: 0 };

    println(person.greet());

    if (person.is_adult()) {
        println("This person is an adult");
    } else {
        println("This person is a minor");
    }

    // Using generics
    var numbers = [1, 2, 3, 4, 5];
    for (i, num in numbers) {
        print_int(num);
    }

    // Concurrency
    var ch: channel<int>;

    spawn {
        ch.send(42);
    };

    var result = ch.receive();
    print_int(result);
}
```
