---
title: "std::testing"
description: Test assertions and utilities
---

Test assertion functions for unit testing and validation.

## Import

```naml
use std::testing::*;
```

## Basic Assertions

### assert

Assert that condition is true.

```naml
fn assert(condition: bool, message: string)
```

**Panics** if condition is false.

**Example:**

```naml
assert(1 + 1 == 2, "basic math should work");
assert(true, "this always passes");
```

### assert_true

Assert that value is true.

```naml
fn assert_true(value: bool, message: string)
```

**Example:**

```naml
assert_true(10 > 5, "10 is greater than 5");
```

### assert_false

Assert that value is false.

```naml
fn assert_false(value: bool, message: string)
```

**Example:**

```naml
assert_false(5 > 10, "5 is not greater than 10");
```

## Equality Assertions

### assert_eq

Assert integer equality.

```naml
fn assert_eq(actual: int, expected: int, message: string)
```

**Example:**

```naml
var result: int = add(3, 4);
assert_eq(result, 7, "3 + 4 should equal 7");
```

### assert_eq_bool

Assert boolean equality.

```naml
fn assert_eq_bool(actual: bool, expected: bool, message: string)
```

**Example:**

```naml
assert_eq_bool(true, true, "booleans should match");
```

### assert_eq_string

Assert string equality.

```naml
fn assert_eq_string(actual: string, expected: string, message: string)
```

**Example:**

```naml
var greeting: string = greet("Alice");
assert_eq_string(greeting, "Hello, Alice!", "greeting should match");
```

### assert_eq_float

Assert float equality (exact).

```naml
fn assert_eq_float(actual: float, expected: float, message: string)
```

**Example:**

```naml
assert_eq_float(1.5, 1.5, "floats should match exactly");
```

## Inequality Assertions

### assert_neq

Assert integer inequality.

```naml
fn assert_neq(actual: int, expected: int, message: string)
```

**Example:**

```naml
assert_neq(42, 43, "42 should not equal 43");
```

### assert_neq_string

Assert string inequality.

```naml
fn assert_neq_string(actual: string, expected: string, message: string)
```

**Example:**

```naml
assert_neq_string("hello", "world", "strings should differ");
```

## Comparison Assertions

### assert_gt

Assert greater than.

```naml
fn assert_gt(actual: int, expected: int, message: string)
```

**Example:**

```naml
assert_gt(10, 5, "10 should be greater than 5");
```

### assert_gte

Assert greater than or equal.

```naml
fn assert_gte(actual: int, expected: int, message: string)
```

**Example:**

```naml
assert_gte(10, 10, "10 should be >= 10");
assert_gte(11, 10, "11 should be >= 10");
```

### assert_lt

Assert less than.

```naml
fn assert_lt(actual: int, expected: int, message: string)
```

**Example:**

```naml
assert_lt(3, 7, "3 should be less than 7");
```

### assert_lte

Assert less than or equal.

```naml
fn assert_lte(actual: int, expected: int, message: string)
```

**Example:**

```naml
assert_lte(5, 5, "5 should be <= 5");
assert_lte(4, 5, "4 should be <= 5");
```

## Float Comparison

### assert_approx

Assert floats are approximately equal within epsilon.

```naml
fn assert_approx(actual: float, expected: float, epsilon: float, message: string)
```

**Example:**

```naml
var result: float = 0.1 + 0.2;
assert_approx(result, 0.3, 0.0001, "0.1 + 0.2 should approximately equal 0.3");

var pi: float = 3.14159265;
assert_approx(pi, 3.14159, 0.00001, "pi approximation");
```

## String Content Assertions

### assert_contains

Assert string contains substring.

```naml
fn assert_contains(actual: string, expected: string, message: string)
```

**Example:**

```naml
var text: string = "The quick brown fox";
assert_contains(text, "brown fox", "should contain substring");
```

### assert_starts_with

Assert string starts with prefix.

```naml
fn assert_starts_with(actual: string, expected: string, message: string)
```

**Example:**

```naml
var text: string = "Hello, World!";
assert_starts_with(text, "Hello", "should start with prefix");
```

### assert_ends_with

Assert string ends with suffix.

```naml
fn assert_ends_with(actual: string, expected: string, message: string)
```

**Example:**

```naml
var text: string = "Hello, World!";
assert_ends_with(text, "World!", "should end with suffix");
```

## Test Failure

### fail

Unconditionally fail a test.

```naml
fn fail(message: string)
```

**Example:**

```naml
if (should_never_happen) {
    fail("This code path should be unreachable");
}
```

## Complete Test Example

```naml
use std::testing::*;

fn add(a: int, b: int) -> int {
    return a + b;
}

fn multiply(a: int, b: int) -> int {
    return a * b;
}

fn greet(name: string) -> string {
    return fmt("Hello, {}!", name);
}

fn main() {
    println("=== Running Tests ===");

    println("Test: add function");
    assert_eq(add(2, 3), 5, "2 + 3 should equal 5");
    assert_eq(add(0, 0), 0, "0 + 0 should equal 0");
    assert_eq(add(-1, 1), 0, "-1 + 1 should equal 0");

    println("Test: multiply function");
    assert_eq(multiply(3, 4), 12, "3 * 4 should equal 12");
    assert_eq(multiply(0, 100), 0, "0 * 100 should equal 0");

    println("Test: greet function");
    assert_eq_string(greet("Alice"), "Hello, Alice!", "greeting Alice");
    assert_eq_string(greet("Bob"), "Hello, Bob!", "greeting Bob");
    assert_contains(greet("World"), "World", "greeting contains name");
    assert_starts_with(greet("Test"), "Hello", "greeting starts with Hello");

    println("Test: comparisons");
    assert_gt(10, 5, "10 > 5");
    assert_lt(3, 7, "3 < 7");
    assert_gte(5, 5, "5 >= 5");
    assert_lte(4, 4, "4 <= 4");

    println("Test: float approximation");
    var circle_area: float = 3.14159 * 2.0 * 2.0;
    assert_approx(circle_area, 12.56636, 0.0001, "circle area");

    println("\n=== All Tests Passed ===");
}
```
