---
title: Structs & Enums
description: Learn about struct and enum definitions, instantiation, generic types, and pattern matching in naml
---

Structs and enums are naml's primary data modeling constructs. Structs group related data, while enums represent variants.

## Structs

### Struct Definition

Define a struct with named fields:

```naml
struct Point {
    x: int,
    y: int
}
```

### Public Struct with Public Fields

Use `pub` to make structs and fields visible outside their module:

```naml
pub struct Rectangle {
    pub width: int,
    pub height: int
}
```

### Struct Instantiation

Create struct instances with struct literal syntax:

```naml
var point: Point = Point { x: 10, y: 20 };
var rect: Rectangle = Rectangle { width: 100, height: 50 };
```

### Struct with Interface Implementation

Structs can implement interfaces:

```naml
interface Shape {
    fn area() -> float;
}

struct Circle implements Shape {
    radius: float
}

pub fn (self: Circle) area() -> float {
    return 3.14159 * self.radius * self.radius;
}
```

### Generic Structs

Structs can be parameterized with type parameters:

```naml
pub struct Box<T> {
    pub value: T
}

pub struct Pair<A, B> {
    pub first: A,
    pub second: B
}
```

### Using Generic Structs

Instantiate generic structs by specifying type arguments:

```naml
var int_box: Box<int> = Box<int> { value: 42 };
var pair: Pair<string, int> = Pair<string, int> {
    first: "age",
    second: 25
};
```

### Nested Structs

Structs can contain other structs:

```naml
struct Address {
    street: string,
    city: string,
    zip: string
}

struct Person {
    name: string,
    age: int,
    address: Address
}

var person: Person = Person {
    name: "Alice",
    age: 30,
    address: Address {
        street: "123 Main St",
        city: "Springfield",
        zip: "12345"
    }
};
```

## Enums

### Simple Enum

Define an enum with named variants:

```naml
enum Status {
    Active,
    Inactive,
    Pending
}
```

### Enum with Associated Data

Enum variants can carry associated data:

```naml
enum UserStatus {
    Active,
    Suspended(string),
    Banned(string, int)
}
```

### Generic Enums

Enums can be generic over types:

```naml
enum Result<T, E> {
    Ok(T),
    Err(E)
}

enum Option<T> {
    Some(T),
    None
}
```

### Enum Construction

Create enum instances by specifying the variant:

```naml
var status: Status = Status::Active;
var suspended: UserStatus = UserStatus::Suspended("Policy violation");
var banned: UserStatus = UserStatus::Banned("Spam", 30);
```

### Generic Enum Construction

```naml
var success: Result<int, string> = Result::Ok(42);
var failure: Result<int, string> = Result::Err("Something went wrong");
```

## Pattern Matching

### Enum Pattern Matching

Use `switch` to match enum variants and extract associated data:

```naml
switch (status) {
    case UserStatus::Active: {
        println("User is active");
    }
    case UserStatus::Suspended(reason): {
        println(fmt("Suspended: {}", reason));
    }
    case UserStatus::Banned(reason, days): {
        println(fmt("Banned for {} days: {}", days, reason));
    }
}
```

### Matching Generic Enums

```naml
var result: Result<int, string> = compute();

switch (result) {
    case Result::Ok(value): {
        println(fmt("Success: {}", value));
    }
    case Result::Err(error): {
        println(fmt("Error: {}", error));
    }
}
```

### Wildcard Patterns

Use `_` to match any case:

```naml
switch (status) {
    case Status::Active: {
        // handle active
    }
    case _: {
        // handle all other cases
    }
}
```

## Complete Example

```naml
// Enum definition
enum Message {
    Quit,
    Move(int, int),
    Write(string),
    ChangeColor(int, int, int)
}

// Struct definition
struct Point {
    x: int,
    y: int
}

pub fn (self: Point) display() {
    println(fmt("Point({}, {})", self.x, self.y));
}

// Function to process messages
fn process_message(msg: Message) {
    switch (msg) {
        case Message::Quit: {
            println("Quit message");
        }
        case Message::Move(x, y): {
            var point: Point = Point { x: x, y: y };
            point.display();
        }
        case Message::Write(text): {
            println(fmt("Text: {}", text));
        }
        case Message::ChangeColor(r, g, b): {
            println(fmt("Color: ({}, {}, {})", r, g, b));
        }
    }
}

fn main() {
    var msg1: Message = Message::Quit;
    var msg2: Message = Message::Move(10, 20);
    var msg3: Message = Message::Write("Hello");
    var msg4: Message = Message::ChangeColor(255, 0, 0);

    process_message(msg1);
    process_message(msg2);
    process_message(msg3);
    process_message(msg4);
}
```

## Struct Methods Example

```naml
pub struct Counter {
    pub value: int
}

pub fn (self: Counter) increment() {
    self.value = self.value + 1;
}

pub fn (self: Counter) decrement() {
    self.value = self.value - 1;
}

pub fn (self: Counter) reset() {
    self.value = 0;
}

pub fn (self: Counter) get() -> int {
    return self.value;
}

fn main() {
    var counter: Counter = Counter { value: 0 };

    counter.increment();
    counter.increment();
    println(counter.get());  // 2

    counter.decrement();
    println(counter.get());  // 1

    counter.reset();
    println(counter.get());  // 0
}
```
