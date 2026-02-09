---
title: Generics
description: Learn about generic functions, structs, interfaces, constraints, and type parameters in naml
---

Generics enable writing reusable, type-safe code that works with multiple types. naml supports generic functions, structs, and interfaces with optional constraints.

## Generic Functions

### Basic Generic Function

Define generic functions with type parameters in angle brackets:

```naml
fn identity<T>(x: T) -> T {
    return x;
}
```

### Multiple Type Parameters

Use multiple type parameters for complex generic functions:

```naml
fn swap<A, B>(pair: Pair<A, B>) -> Pair<B, A> {
    return Pair<B, A> { first: pair.second, second: pair.first };
}
```

### Generic Function Examples

```naml
fn first<T>(items: [T]) -> option<T> {
    use std::collections::count;
    if (count(items) == 0) {
        return none;
    }
    return some(items[0]);
}

fn map<T, R>(items: [T], transform: fn(T) -> R) -> [R] {
    var result: [R] = [];
    for (item: T in items) {
        use std::collections::push;
        push(result, transform(item));
    }
    return result;
}
```

## Generic Structs

### Basic Generic Struct

Define generic structs with type parameters:

```naml
pub struct Box<T> {
    pub value: T
}
```

### Multiple Type Parameters

```naml
pub struct Pair<A, B> {
    pub first: A,
    pub second: B
}
```

### Using Generic Structs

```naml
var int_box: Box<int> = Box<int> { value: 42 };
var string_box: Box<string> = Box<string> { value: "hello" };

var pair: Pair<string, int> = Pair<string, int> {
    first: "age",
    second: 25
};
```

### Generic Container Example

```naml
use std::collections::*;

pub struct Container<T> {
    pub items: [T]
}

pub fn (self: Container<T>) add<T>(item: T) {
    push(self.items, item);
}

pub fn (self: Container<T>) remove<T>() -> option<T> {
    return pop(self.items);
}

pub fn (self: Container<T>) size<T>() -> int {
    return count(self.items);
}
```

## Generic Methods

Methods on generic structs can use the struct's type parameters:

```naml
pub struct Stack<T> {
    items: [T]
}

pub fn (self: Stack<T>) push<T>(item: T) {
    use std::collections::push;
    push(self.items, item);
}

pub fn (self: Stack<T>) pop<T>() -> option<T> {
    use std::collections::pop;
    return pop(self.items);
}

pub fn (self: Stack<T>) peek<T>() -> option<T> {
    use std::collections::count;
    var len: int = count(self.items);
    if (len == 0) {
        return none;
    }
    return some(self.items[len - 1]);
}
```

## Generic Constraints

### Interface Constraints

Constrain type parameters to types that implement specific interfaces:

```naml
fn max<T: Comparable<T>>(a: T, b: T) -> T {
    if (a.compare(b) >= 0) {
        return a;
    }
    return b;
}
```

### Generic Sort Function

```naml
fn sort<T: Comparable<T>>(items: [T]) -> [T] {
    // Bubble sort implementation
    use std::collections::count;
    var len: int = count(items);

    for (i: int in 0..len) {
        for (j: int in 0..(len - i - 1)) {
            if (items[j].compare(items[j + 1]) > 0) {
                var temp: T = items[j];
                items[j] = items[j + 1];
                items[j + 1] = temp;
            }
        }
    }

    return items;
}
```

### Multiple Constraints

```naml
fn display_sorted<T: Describable, T: Comparable<T>>(items: [T]) {
    var sorted: [T] = sort(items);
    for (item: T in sorted) {
        println(item.describe());
    }
}
```

## Generic Interfaces

Define generic interfaces for type-safe contracts:

```naml
interface Comparable<T> {
    fn compare(other: T) -> int;
}

interface Iterator<T> {
    fn next() -> option<T>;
    fn has_next() -> bool;
}

interface Collection<T> {
    fn add(item: T);
    fn remove(item: T) -> bool;
    fn contains(item: T) -> bool;
    fn size() -> int;
}
```

## Complete Example

```naml
// Generic interface
interface Comparable<T> {
    fn compare(other: T) -> int;
}

// Generic struct
pub struct Pair<T: Comparable<T>> {
    pub first: T,
    pub second: T
}

// Generic method with constraint
pub fn (self: Pair<T>) max<T: Comparable<T>>() -> T {
    if (self.first.compare(self.second) >= 0) {
        return self.first;
    }
    return self.second;
}

pub fn (self: Pair<T>) min<T: Comparable<T>>() -> T {
    if (self.first.compare(self.second) <= 0) {
        return self.first;
    }
    return self.second;
}

// Implementing Comparable for custom type
struct Number implements Comparable<Number> {
    value: int
}

pub fn (self: Number) compare(other: Number) -> int {
    if (self.value > other.value) {
        return 1;
    }
    if (self.value < other.value) {
        return -1;
    }
    return 0;
}

fn main() {
    var n1: Number = Number { value: 10 };
    var n2: Number = Number { value: 20 };

    var pair: Pair<Number> = Pair<Number> { first: n1, second: n2 };

    var max: Number = pair.max();
    var min: Number = pair.min();

    println(fmt("Max: {}", max.value));  // 20
    println(fmt("Min: {}", min.value));  // 10
}
```

## Generic Result Type Example

```naml
enum Result<T, E> {
    Ok(T),
    Err(E)
}

fn divide(a: int, b: int) -> Result<int, string> {
    if (b == 0) {
        return Result::Err("Division by zero");
    }
    return Result::Ok(a / b);
}

fn unwrap_or<T, E>(result: Result<T, E>, default: T) -> T {
    switch (result) {
        case Result::Ok(value): {
            return value;
        }
        case Result::Err(_): {
            return default;
        }
    }
}

fn main() {
    var r1: Result<int, string> = divide(10, 2);
    var r2: Result<int, string> = divide(10, 0);

    var v1: int = unwrap_or(r1, -1);  // 5
    var v2: int = unwrap_or(r2, -1);  // -1

    println(v1);
    println(v2);
}
```

## Best Practices

1. **Use descriptive type parameter names** - `T` for generic types, `K`/`V` for keys/values, `E` for errors
2. **Add constraints when needed** - Use interface constraints to ensure type safety
3. **Keep generics simple** - Avoid overly complex generic hierarchies
4. **Document type parameters** - Use comments to explain generic parameters
5. **Test with multiple types** - Ensure generic code works with various type instantiations
