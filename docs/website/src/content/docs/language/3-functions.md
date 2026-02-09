---
title: Functions & Methods
description: Comprehensive guide to functions, methods, receivers, and visibility in naml
---

Functions and methods are the building blocks of naml programs. Functions are standalone, while methods are associated with types via receivers.

## Functions

### Basic Function

Functions are declared with the `fn` keyword, followed by parameters and an optional return type:

```naml
fn add(a: int, b: int) -> int {
    return a + b;
}
```

### Function Without Return Type

If a function doesn't return a value, omit the return type:

```naml
fn greet(name: string) {
    println("Hello, ");
    println(name);
}
```

### Public Functions

Use `pub` to make functions visible outside their module:

```naml
pub fn public_function() -> int {
    return 42;
}
```

### Functions with Exceptions

Functions can declare exceptions they might throw using the `throws` keyword:

```naml
fn divide(a: int, b: int) -> int throws DivisionByZero {
    if (b == 0) {
        throw DivisionByZero("Cannot divide by zero");
    }
    return a / b;
}
```

### Multiple Exception Types

Functions can throw multiple exception types:

```naml
fn process(input: string) -> int throws ParseError, ValidationError {
    // Function body
}
```

## Methods

Methods are functions with a receiver (first parameter is `self`). **Receivers are always mutable** - there is no `mut` keyword on receivers.

### Basic Method

Define methods using `pub fn (self: Type)` syntax:

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

**Important**: Using `mut` on receivers is invalid and will cause a compile error:

```naml
// INVALID - will not compile
pub fn (mut self: Counter) increment() {  // Error: MutNotAllowedOnReceiver
    self.value = self.value + 1;
}
```

### Method Calls

Call methods using dot notation:

```naml
var point: Point = Point { x: 10, y: 20 };
var x: int = point.get_x();    // Method call
var y: int = point.get_y();
```

### Methods with Parameters

Methods can have additional parameters beyond the receiver:

```naml
pub fn (self: Point) distance_to(other: Point) -> float {
    var dx: int = self.x - other.x;
    var dy: int = self.y - other.y;
    return sqrt((dx * dx + dy * dy) as float);
}
```

### Methods Returning Self

Methods can return the receiver for method chaining:

```naml
pub fn (self: Builder) set_name(name: string) -> Builder {
    self.name = name;
    return self;
}

pub fn (self: Builder) set_age(age: int) -> Builder {
    self.age = age;
    return self;
}

// Usage
var builder: Builder = Builder {}
    .set_name("Alice")
    .set_age(30);
```

## Function Parameters

### Passing by Value

All parameters are passed by value in naml. For primitive types (int, bool, etc.), this means copying the value. For heap types (strings, arrays, structs), this means copying a reference.

```naml
fn modify(x: int) {
    x = 100;  // Modifies local copy
}

var a: int = 10;
modify(a);
println(a);  // Still 10
```

### Multiple Parameters

Functions can have multiple parameters:

```naml
fn calculate(a: int, b: int, c: int) -> int {
    return a + b * c;
}
```

## Return Values

### Explicit Return

Use the `return` keyword to return a value:

```naml
fn max(a: int, b: int) -> int {
    if (a > b) {
        return a;
    }
    return b;
}
```

### Early Return

Return early from a function:

```naml
fn process(value: int) -> string {
    if (value < 0) {
        return "negative";
    }
    if (value == 0) {
        return "zero";
    }
    return "positive";
}
```

### Return Without Value

Functions without a return type can use `return` to exit early:

```naml
fn validate(x: int) {
    if (x < 0) {
        println("Invalid!");
        return;
    }
    println("Valid!");
}
```

## Function Examples

### Complete Example with Struct and Methods

```naml
struct Rectangle {
    width: int,
    height: int
}

pub fn (self: Rectangle) area() -> int {
    return self.width * self.height;
}

pub fn (self: Rectangle) perimeter() -> int {
    return 2 * (self.width + self.height);
}

pub fn (self: Rectangle) scale(factor: int) {
    self.width = self.width * factor;
    self.height = self.height * factor;
}

fn main() {
    var rect: Rectangle = Rectangle { width: 10, height: 5 };

    println(rect.area());       // 50
    println(rect.perimeter());  // 30

    rect.scale(2);
    println(rect.area());       // 200
}
```
