---
title: Variables
description: Learn about variable declarations, mutability, constants, and the variable with else block pattern in naml
---

naml requires explicit type annotations for all variable declarations. This design choice ensures code is always explicit and self-documenting.

## Mutable Variables

Use `var` to declare mutable variables. Variables declared with `var` are **always mutable** - there is no `mut` keyword needed. **Type annotation is always required**:

```naml
var x: int = 10;
x = 20;              // OK - var is mutable by default
var y: float = 3.14;
var z: int = 30;
```

### Important Notes

Type inference is not supported. The following is **invalid**:

```naml
// INVALID - will not compile:
var x = 10;           // Error: ExpectedTypeAnnotation
var name = "Alice";   // Error: ExpectedTypeAnnotation
var mut x: int = 10;  // Error: MutNotAllowedOnVar (var is already mutable)
```

### Syntax

**Required syntax**: `var name: Type = value;`

```naml
var x: int = 42;
var y: float = 3.14;
var z: string = "hello";
var pair: Pair<int, int> = Pair { first: 1, second: 2 };
var arr: [int] = [];
var m: map<string, int> = {};
```

## Constants

Constants are immutable values that must be initialized at declaration. Like variables, **constants require type annotations**:

```naml
const PI: float = 3.14159;
const MAX_SIZE: int = 1000;
```

## Variable with Else Block

Handle initialization failures with the `else` block pattern. This is useful when working with optional values that might fail:

```naml
var value: int = get_optional() else {
    // Handle none case
    return -1; // this stops execution
} ?? 0; // alternate value
```

The `else` block executes when the value is `none`, and you can use `??` to provide a default value.

### Example with Real Function

```naml
use std::collections::pop;

var arr: [int] = [1, 2, 3];
var last: int = pop(arr) else {
    println("Array was empty!");
    return;
} ?? -1;
```

## Why Required Type Annotations?

naml deliberately requires explicit type annotations for several reasons:

1. **Explicit types make code easier to read and understand**
2. **No guessing about what type a variable holds**
3. **Better error messages when types don't match**
4. **Self-documenting code without additional comments**

### Example Comparison

```naml
// naml (explicit and clear)
var count: int = 0;
var name: string = "Alice";
var active: bool = true;

// What you can't do (would be ambiguous)
var count = 0;        // Error: what type? int? uint? float?
var name = "Alice";   // Error: ExpectedTypeAnnotation
```

## Variable Scope

Variables are scoped to their containing block:

```naml
fn example() {
    var x: int = 10;

    if (true) {
        var y: int = 20;
        println(x);  // OK - x is in outer scope
        println(y);  // OK - y is in this scope
    }

    // println(y);  // Error - y is not in scope
}
```

## Shadowing

Variables can be shadowed in inner scopes:

```naml
var x: int = 10;

if (true) {
    var x: string = "hello";  // Shadows outer x
    println(x);  // Prints "hello"
}

println(x);  // Prints 10 - original x
```
