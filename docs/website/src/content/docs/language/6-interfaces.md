---
title: Interfaces
description: Learn about interface definitions, implementations, generic interfaces, and type constraints in naml
---

Interfaces define contracts that types must fulfill. They enable polymorphism and generic constraints in naml.

## Interface Definition

Define an interface with method signatures:

```naml
interface Describable {
    fn describe() -> string;
}

interface Drawable {
    fn draw();
    fn clear();
}
```

### Generic Interfaces

Interfaces can be parameterized with type parameters:

```naml
interface Comparable<T> {
    fn compare(other: T) -> int;
}

interface Container<T> {
    fn add(item: T);
    fn remove() -> option<T>;
    fn size() -> int;
}
```

## Implementing Interfaces

### Basic Implementation

Use `implements` to declare that a struct implements an interface:

```naml
struct Person implements Describable {
    name: string,
    age: int
}

pub fn (self: Person) describe() -> string {
    return fmt("{} is {} years old", self.name, self.age);
}
```

### Multiple Interfaces

A struct can implement multiple interfaces:

```naml
struct Rectangle implements Describable, Drawable {
    width: int,
    height: int
}

pub fn (self: Rectangle) describe() -> string {
    return fmt("Rectangle {}x{}", self.width, self.height);
}

pub fn (self: Rectangle) draw() {
    println("Drawing rectangle");
}

pub fn (self: Rectangle) clear() {
    println("Clearing rectangle");
}
```

### Generic Interface Implementation

Implement generic interfaces by specifying type parameters:

```naml
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
```

## Interface as Type Constraints

### Generic Functions with Constraints

Use interfaces to constrain generic type parameters:

```naml
fn max<T: Comparable<T>>(a: T, b: T) -> T {
    if (a.compare(b) >= 0) {
        return a;
    }
    return b;
}
```

### Multiple Constraints

Require multiple interface implementations:

```naml
fn display_and_compare<T: Describable, T: Comparable<T>>(a: T, b: T) {
    println(a.describe());
    println(b.describe());

    var result: int = a.compare(b);
    if (result > 0) {
        println("First is greater");
    } else if (result < 0) {
        println("Second is greater");
    } else {
        println("Equal");
    }
}
```

## Complete Example

```naml
// Interface definitions
interface Shape {
    fn area() -> float;
    fn perimeter() -> float;
}

interface Drawable {
    fn draw();
}

// Struct implementing Shape and Drawable
struct Circle implements Shape, Drawable {
    radius: float
}

pub fn (self: Circle) area() -> float {
    return 3.14159 * self.radius * self.radius;
}

pub fn (self: Circle) perimeter() -> float {
    return 2.0 * 3.14159 * self.radius;
}

pub fn (self: Circle) draw() {
    println(fmt("Drawing circle with radius {}", self.radius));
}

// Another struct implementing Shape and Drawable
struct Rectangle implements Shape, Drawable {
    width: float,
    height: float
}

pub fn (self: Rectangle) area() -> float {
    return self.width * self.height;
}

pub fn (self: Rectangle) perimeter() -> float {
    return 2.0 * (self.width + self.height);
}

pub fn (self: Rectangle) draw() {
    println(fmt("Drawing rectangle {}x{}", self.width, self.height));
}

// Generic function using interface constraint
fn print_shape_info<T: Shape>(shape: T) {
    println(fmt("Area: {}", shape.area()));
    println(fmt("Perimeter: {}", shape.perimeter()));
}

fn main() {
    var circle: Circle = Circle { radius: 5.0 };
    var rect: Rectangle = Rectangle { width: 10.0, height: 5.0 };

    print_shape_info(circle);
    print_shape_info(rect);

    circle.draw();
    rect.draw();
}
```

## Generic Container Example

```naml
interface Stack<T> {
    fn push(item: T);
    fn pop() -> option<T>;
    fn peek() -> option<T>;
    fn is_empty() -> bool;
}

struct ArrayStack<T> implements Stack<T> {
    items: [T]
}

pub fn (self: ArrayStack<T>) push<T>(item: T) {
    use std::collections::push;
    push(self.items, item);
}

pub fn (self: ArrayStack<T>) pop<T>() -> option<T> {
    use std::collections::pop;
    return pop(self.items);
}

pub fn (self: ArrayStack<T>) peek<T>() -> option<T> {
    use std::collections::count;
    var len: int = count(self.items);
    if (len == 0) {
        return none;
    }
    return some(self.items[len - 1]);
}

pub fn (self: ArrayStack<T>) is_empty<T>() -> bool {
    use std::collections::count;
    return count(self.items) == 0;
}

fn main() {
    var stack: ArrayStack<int> = ArrayStack<int> { items: [] };

    stack.push(1);
    stack.push(2);
    stack.push(3);

    var top: option<int> = stack.peek();
    println(top ?? -1);  // 3

    var popped: option<int> = stack.pop();
    println(popped ?? -1);  // 3

    println(stack.is_empty());  // false
}
```

## Interface Best Practices

1. **Keep interfaces focused** - Define small, cohesive interfaces
2. **Use descriptive names** - Interface names should describe capabilities
3. **Favor composition** - Multiple small interfaces over one large interface
4. **Document requirements** - Use comments to clarify interface contracts
5. **Consider generics** - Use generic interfaces for type-safe containers
