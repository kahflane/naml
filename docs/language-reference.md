# naml 🐜 Language Reference

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
16. [Built-in Functions](#built-in-functions)
17. [Standard Library](#standard-library)
18. [Concurrency](#concurrency)
19. [Pattern Matching](#pattern-matching)
20. [Modules and Imports](#modules-and-imports)
21. [External Functions](#external-functions)
22. [Comments](#comments)
23. [Keywords](#keywords)

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
- Strong static typing with **required type annotations**

---

## Primitive Types

naml provides the following primitive types:

| Type | Description | Size |
|------|-------------|------|
| `int` | Signed integer | 64-bit |
| `uint` | Unsigned integer | 64-bit |
| `float` | Floating-point number | 64-bit |
| `decimal` | Decimal number with precision/scale | 64-bit |
| `bool` | Boolean value | - |
| `string` | UTF-8 encoded text | heap |
| `bytes` | Raw binary data | heap |

### Examples

```naml
var age: int = 25;
var count: uint = 100;
var pi: float = 3.14159;
var price: decimal = 19.99;           // Default precision(10, 2)
var precise: decimal(18, 6) = 3.141592;  // Custom precision
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
var first: int = numbers[0];      // Indexing
var len: int = numbers.len();     // Length
numbers.push(6);                   // Append
var last: option<int> = numbers.pop();  // Remove last, returns option<T>
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
var value: int = maybe ?? 0;  // Returns 42
var other: int = nothing ?? -1;  // Returns -1
```

### Channels

Communication channels for concurrency (native/server only). Requires `use std::threads::*;`:

```naml
use std::threads::*;

var ch: channel<int> = open_channel(10);  // buffered channel with capacity 10
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
var a: int = 42;
var b: int = 1_000_000;      // Underscores for readability
var c: int = -100;

// Floats
var pi: float = 3.14159;
var scientific: float = 1.5e10;
```

### String Literals

```naml
var greeting: string = "Hello, World!";
var with_quotes: string = "She said \"hi\"";
var with_newline: string = "Line 1\nLine 2";
```

Supported escape sequences: `\n` (newline), `\t` (tab), `\r` (carriage return), `\\` (backslash), `\"` (quote), `\0` (null).

### Boolean Literals

```naml
var yes: bool = true;
var no: bool = false;
```

### Option Literals

```naml
var present: option<int> = some(42);
var absent: option<int> = none;
```

### Array Literals

```naml
var numbers: [int] = [1, 2, 3, 4, 5];
var strings: [string] = ["a", "b", "c"];
var nested: [[int]] = [[1, 2], [3, 4]];
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
var sum: int = 10 + 5;       // 15
var diff: int = 10 - 5;      // 5
var prod: int = 10 * 5;      // 50
var quot: int = 10 / 5;      // 2
var rem: int = 10 % 3;       // 1
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
var eq: bool = 5 == 5;        // true
var neq: bool = 5 != 3;       // true
var lt: bool = 3 < 5;         // true
var lte: bool = 5 <= 5;       // true
var gt: bool = 5 > 3;         // true
var gte: bool = 5 >= 5;       // true
```

### Logical Operators

| Operator | Description |
|----------|-------------|
| `and`, `&&` | Logical AND |
| `or`, `\|\|` | Logical OR |
| `not`, `!` | Logical NOT |

```naml
var both: bool = true and false;    // false
var either: bool = true or false;   // true
var negated: bool = not true;       // false
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
var band: int = 5 & 3;       // 1
var bor: int = 5 | 3;        // 7
var bxor: int = 5 ^ 3;       // 6
var lshift: int = 1 << 4;    // 16
var rshift: int = 16 >> 2;   // 4
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
var x: int = 10;
x += 5;     // x = 15
x -= 3;     // x = 12
x *= 2;     // x = 24
```

### Range Operators

| Operator | Description |
|----------|-------------|
| `..` | Exclusive range |
| `..=` | Inclusive range |

Ranges are used in for loops:

```naml
for (i: int in 0..5) { }     // 0, 1, 2, 3, 4
for (i: int in 0..=5) { }    // 0, 1, 2, 3, 4, 5
```

### Other Operators

| Operator | Description |
|----------|-------------|
| `??` | Null coalescing |
| `as` | Type casting |
| `is` | Type/variant check |

```naml
var value: int = optional ?? default;
var str: string = number as string;
```

---

## Variables

### Mutable Variables

Use `var` to declare mutable variables. Variables declared with `var` are **always mutable** - there is no `mut` keyword needed. **Type annotation is always required**:

```naml
var x: int = 10;
x = 20;              // OK - var is mutable by default
var y: float = 3.14;
var z: int = 30;
```

**Important**: Type inference is not supported. The following is **invalid**:

```naml
// INVALID - will not compile:
var x = 10;           // Error: ExpectedTypeAnnotation
var name = "Alice";   // Error: ExpectedTypeAnnotation
var mut x: int = 10;  // Error: MutNotAllowedOnVar (var is already mutable)
```

### Variable with Else Block

Handle initialization failures:

```naml
var value: int = get_optional() else {
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

**Note**: `if` is a statement, not an expression. It cannot return a value directly. Use a variable assignment inside the branches instead:

```naml
var result: string;
if (x > 0) {
    result = "positive";
} else {
    result = "non-positive";
}
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
for (i: int in 0..10) {
    // i: 0, 1, 2, ..., 9
}

for (i: int in 0..=10) {
    // i: 0, 1, 2, ..., 10 (inclusive)
}
```

Iterate over collections:

```naml
for (item: string in array) {
    // iterate items
}

for (index: int, item: string in array) {
    // index and item
}
```

### Break and Continue

```naml
for (i: int in 0..100) {
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

Methods are functions with a receiver (first parameter is `self`). **Receivers are always mutable** - there is no `mut` keyword on receivers:

### Basic Method

```naml
pub fn (self: Point) get_x() -> int {
    return self.x;
}

pub fn (self: Point) get_y() -> int {
    return self.y;
}
```

### Mutating Methods

All methods can mutate `self` fields directly since receivers are always mutable:

```naml
pub fn (self: Counter) increment() {
    self.value = self.value + 1;
}

pub fn (self: Counter) reset() {
    self.value = 0;
}
```

**Note**: Using `mut` on receivers is invalid and will cause a compile error.

### Method Calls

```naml
var point: Point = Point { x: 10, y: 20 };
var x: int = point.get_x();    // Method call
var y: int = point.get_y();
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
var point: Point = Point { x: 10, y: 20 };
var rect: Rectangle = Rectangle { width: 100, height: 50 };
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
var int_box: Box<int> = Box<int> { value: 42 };
var pair: Pair<string, int> = Pair<string, int> { first: "age", second: 25 };
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
var status: Status = Status::Active;
var suspended: UserStatus = UserStatus::Suspended("Policy violation");
```

### Enum Pattern Matching

```naml
switch (status) {
    case UserStatus::Active: {
        println("User is active");
    }
    case UserStatus::Suspended(reason): {
        println("Suspended: {}", reason);
    }
    case UserStatus::Banned(reason, days): {
        println("Banned for days: {}", days);
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
        var ex: DivisionByZero = DivisionByZero("Cannot divide by zero");
        ex.dividend = a;
        throw ex;
    }
    return a / b;
}
```

### Catching Exceptions

```naml
var result: int = divide(10, 0) catch e {
    println("Error: ");
    println(e.message());
} ?? -1;
```

### Try Expression

```naml
var value: int = try risky_operation();
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
var add: fn(int, int) -> int = fn (a: int, b: int) -> int { a + b };
var square: fn(int) -> int = fn (x: int) -> int { x * x };
```

### Lambdas with Explicit Types

```naml
var double: fn(int) -> int = fn (x: int) -> int { x * 2 };
```

### Lambdas as Parameters

```naml
fn apply(f: fn(int) -> int, x: int) -> int {
    return f(x);
}

var result: int = apply(fn (n: int) -> int { n * n }, 5);  // 25
```

---

## Built-in Functions

These functions are always available without any import:

| Function | Signature | Description |
|----------|-----------|-------------|
| `print` | `(format: string, args...)` | Print to stdout (no newline). Supports `{}` placeholders. |
| `println` | `(format: string, args...)` | Print to stdout with newline. Supports `{}` placeholders. |
| `fmt` | `(format: string, args...) -> string` | Format string with `{}` placeholders, returns result. |
| `warn` | `(format: string, args...)` | Print to stderr with `warning:` prefix. |
| `error` | `(format: string, args...)` | Print to stderr with `error:` prefix. |
| `panic` | `(format: string, args...)` | Print to stderr with `panic:` prefix, then abort. |
| `read_line` | `() -> string` | Blocking read from stdin until newline. |
| `sleep` | `(ms: int)` | Pause execution for `ms` milliseconds. |

```naml
print("Hello, {}!\n", name);
println("x = {}, y = {}", x, y);
var msg: string = fmt("Score: {}", score);
warn("deprecated feature used");
panic("unreachable code");
var input: string = read_line();
sleep(1000);
```

---

## Standard Library

### std::random

```naml
use std::random::*;

var n: int = random(1, 100);      // Random int in [min, max]
var f: float = random_float();     // Random float in [0.0, 1.0)
```

### std::io

Terminal I/O functions for interactive and TUI applications:

```naml
use std::io::*;

var key: int = read_key();         // Non-blocking key read (-1 if none)
clear_screen();                     // Clear terminal
set_cursor(x, y);                  // Move cursor (0-indexed)
hide_cursor();
show_cursor();
var w: int = terminal_width();
var h: int = terminal_height();
```

### std::threads

Concurrency primitives (see [Concurrency](#concurrency)):

```naml
use std::threads::*;

var ch: channel<int> = open_channel(10);
join();
```

---

## Concurrency

Channels and join require `use std::threads::*;`. The `spawn` keyword is always available.

### Spawn

Create concurrent tasks with `spawn`:

```naml
spawn {
    // This runs concurrently
    do_work();
};
```

### Channels

Communicate between concurrent tasks. Requires `use std::threads::*;`:

```naml
use std::threads::*;

fn main() {
    var ch: channel<int> = open_channel(10);

    spawn {
        ch.send(42);
    };

    var value: int = ch.receive();
    println(value);
    ch.close();
}
```

Channel methods: `.send(value)`, `.receive()`, `.close()`, `.len()`.

### Join

Wait for all spawned tasks to complete:

```naml
use std::threads::*;

fn main() {
    spawn { task1(); };
    spawn { task2(); };

    join();  // Block until all spawned tasks complete
}
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
        println("Success: {}", value);
    }
    case Result::Err(error): {
        println("Error: {}", error);
    }
}
```

---

## Modules and Imports

### Standard Library Modules

Import standard library modules with `use std::<module>::*;` or specific items:

```naml
use std::random::*;         // random(min, max), random_float()
use std::io::*;             // read_key(), clear_screen(), set_cursor(), etc.
use std::threads::*;        // open_channel(), join()
```

Specific imports:

```naml
use std::random::{random};
use std::threads::{open_channel, join};
```

### Local Modules

Import functions from other `.naml` files in the same directory:

```naml
// Imports all pub fn from ./math.naml
use math::*;

// Import specific functions
use helpers::{validate, format_output};
```

Only `pub fn` declarations (without receivers) are importable from local modules. Structs, enums, and methods cannot be imported.

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
var x: int = 10;  // Inline comment
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
`fn`, `var`, `const`, `pub`, `struct`, `enum`, `interface`, `exception`, `extern`

> **Note**: The `mut` keyword is reserved but not used. Variables (`var`) and method receivers are mutable by default.

### Control Flow Keywords
`if`, `else`, `while`, `for`, `in`, `loop`, `break`, `continue`, `return`, `switch`, `case`, `default`

### Error Handling Keywords
`throw`, `throws`, `try`, `catch`

### Type Keywords
`int`, `uint`, `float`, `decimal`, `bool`, `string`, `bytes`, `option`, `map`, `channel`

### Boolean/Option Keywords
`true`, `false`, `none`, `some`

### Logical Keywords
`and`, `or`, `not`

### Other Keywords
`spawn`, `as`, `is`, `implements`, `use`, `platforms`, `native`, `server`, `browser`

---

## Type System

### Required Type Annotations

naml requires explicit type annotations for all variable and constant declarations. This is a deliberate design choice to ensure code is always explicit and self-documenting.

**Syntax**: `var name: Type = value;`

```naml
var x: int = 42;
var y: float = 3.14;
var z: string = "hello";
var pair: Pair<int, int> = Pair { first: 1, second: 2 };
var arr: [int] = [];
var m: map<string, int>;
```

**Constants also require type annotations**:

```naml
const PI: float = 3.14159;
const MAX_SIZE: int = 1000;
```

**Why no type inference?**
- Explicit types make code easier to read and understand
- No guessing about what type a variable holds
- Better error messages when types don't match
- Self-documenting code without additional comments

---

## Best Practices

1. **Use descriptive type names** to make code self-documenting
2. **Handle errors** with exceptions and catch blocks
3. **Use meaningful names** for variables and functions
4. **Document public APIs** with documentation comments
5. **Keep functions small** and focused on one task

---

## Example Program

```naml
///
/// A simple example demonstrating naml features
///

use std::threads::*;

struct Person {
    name: string,
    age: int
}

pub fn (self: Person) greet() -> string {
    return fmt("Hello, I am {}", self.name);
}

pub fn (self: Person) is_adult() -> bool {
    return self.age >= 18;
}

exception InvalidAge {
    value: int
}

fn create_person(name: string, age: int) -> Person throws InvalidAge {
    if (age < 0) {
        var ex: InvalidAge = InvalidAge("Age cannot be negative");
        ex.value = age;
        throw ex;
    }
    return Person { name: name, age: age };
}

fn main() {
    var person: Person = create_person("Alice", 25) catch e {
        println("Error creating person");
    } ?? Person { name: "Unknown", age: 0 };

    println(person.greet());

    if (person.is_adult()) {
        println("This person is an adult");
    } else {
        println("This person is a minor");
    }

    // Arrays and iteration
    var numbers: [int] = [1, 2, 3, 4, 5];
    for (i: int, num: int in numbers) {
        println(num);
    }

    // Concurrency
    var ch: channel<int> = open_channel(1);

    spawn {
        ch.send(42);
    };

    join();
    var result: int = ch.receive();
    println(result);
    ch.close();
}
```
