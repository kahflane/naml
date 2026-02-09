---
title: Basics
description: Fundamental naml programs — variables, functions, structs, methods, and interfaces
---

## Hello World

The simplest naml program:

```naml
fn main() {
    println("Hello, World!");
}
```

Run it with `naml run hello.nm`.

## Variables, Loops, and Conditionals

```naml
fn main() {
    var x: int = 40;
    var y: int = 2;
    var result: int = x + y;

    println("Result: {}", result);

    if (result == 42) {
        println("The answer!");
    }

    var i: int = 0;
    while (i < 3) {
        println("i = {}", i);
        i = i + 1;
    }
}
```

## Functions and Recursion

```naml
fn add(a: int, b: int) -> int {
    return a + b;
}

fn factorial(n: int) -> int {
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}

fn fibonacci(n: int) -> int {
    if (n <= 1) {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

fn sum_array(arr: [int]) -> int {
    var total: int = 0;
    for (item: int in arr) {
        total = total + item;
    }
    return total;
}

fn main() {
    println("Sum: {}", add(10, 20));
    println("5! = {}", factorial(5));
    println("Fib(10) = {}", fibonacci(10));
    println("Array sum: {}", sum_array([1, 2, 3, 4, 5]));
}
```

## Structs and Methods

```naml
struct Point {
    x: int,
    y: int
}

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

pub fn (self: Rectangle) is_square() -> bool {
    return self.width == self.height;
}

fn main() {
    var p: Point = Point { x: 10, y: 20 };
    println("Point: ({}, {})", p.x, p.y);

    var rect: Rectangle = Rectangle { width: 5, height: 10 };
    println("Area: {}", rect.area());
    println("Perimeter: {}", rect.perimeter());
    println("Is square: {}", rect.is_square());
}
```

## Interfaces

```naml
interface Describable {
    fn describe() -> string;
}

struct Point implements Describable {
    x: int,
    y: int
}

pub fn (self: Point) get_x() -> int {
    return self.x;
}

pub fn (self: Point) get_y() -> int {
    return self.y;
}

pub fn (self: Point) describe() -> string {
    return "Point";
}

fn main() {
    var p: Point = Point { x: 10, y: 20 };
    println("({}, {})", p.get_x(), p.get_y());
    println(p.describe());
}
```
