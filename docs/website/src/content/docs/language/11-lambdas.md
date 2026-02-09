---
title: Lambdas
description: Learn about lambda expressions, anonymous functions, and higher-order functions in naml
---

Lambdas are anonymous functions that can be assigned to variables, passed as parameters, and returned from functions. They enable functional programming patterns in naml.

## Lambda Expressions

### Basic Lambda Syntax

Define a lambda with the `fn` keyword followed by parameters and body:

```naml
var add: fn(int, int) -> int = fn (a: int, b: int) -> int { a + b };
var square: fn(int) -> int = fn (x: int) -> int { x * x };
```

### Lambda with Multiple Statements

Lambdas can have multiple statements in their body:

```naml
var process: fn(int) -> int = fn (x: int) -> int {
    var doubled: int = x * 2;
    var result: int = doubled + 10;
    return result;
};
```

## Lambdas with Explicit Types

All lambda parameters must have explicit type annotations:

```naml
var double: fn(int) -> int = fn (x: int) -> int {
    x * 2
};

var concat: fn(string, string) -> string = fn (a: string, b: string) -> string {
    return fmt("{}{}", a, b);
};
```

## Lambdas as Function Parameters

Pass lambdas to functions for higher-order programming:

```naml
fn apply(f: fn(int) -> int, x: int) -> int {
    return f(x);
}

var result: int = apply(fn (n: int) -> int { n * n }, 5);  // 25
```

### Map Function Example

```naml
fn map(items: [int], transform: fn(int) -> int) -> [int] {
    use std::collections::{count, push};
    var result: [int] = [];

    for (item: int in items) {
        push(result, transform(item));
    }

    return result;
}

fn main() {
    var numbers: [int] = [1, 2, 3, 4, 5];
    var doubled: [int] = map(numbers, fn (x: int) -> int { x * 2 });

    for (n: int in doubled) {
        println(n);  // 2, 4, 6, 8, 10
    }
}
```

### Filter Function Example

```naml
fn filter(items: [int], predicate: fn(int) -> bool) -> [int] {
    use std::collections::push;
    var result: [int] = [];

    for (item: int in items) {
        if (predicate(item)) {
            push(result, item);
        }
    }

    return result;
}

fn main() {
    var numbers: [int] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    var evens: [int] = filter(numbers, fn (x: int) -> bool {
        x % 2 == 0
    });

    for (n: int in evens) {
        println(n);  // 2, 4, 6, 8, 10
    }
}
```

## Fold/Reduce Pattern

```naml
fn fold(items: [int], initial: int, combine: fn(int, int) -> int) -> int {
    var result: int = initial;

    for (item: int in items) {
        result = combine(result, item);
    }

    return result;
}

fn main() {
    var numbers: [int] = [1, 2, 3, 4, 5];

    // Sum
    var sum: int = fold(numbers, 0, fn (acc: int, x: int) -> int {
        acc + x
    });
    println(sum);  // 15

    // Product
    var product: int = fold(numbers, 1, fn (acc: int, x: int) -> int {
        acc * x
    });
    println(product);  // 120
}
```

## Returning Lambdas from Functions

Functions can return lambdas:

```naml
fn make_adder(n: int) -> fn(int) -> int {
    return fn (x: int) -> int { x + n };
}

fn main() {
    var add5: fn(int) -> int = make_adder(5);
    var add10: fn(int) -> int = make_adder(10);

    println(add5(3));   // 8
    println(add10(3));  // 13
}
```

## Generic Higher-Order Functions

Combine lambdas with generics for maximum flexibility:

```naml
fn apply_twice<T>(f: fn(T) -> T, x: T) -> T {
    return f(f(x));
}

fn main() {
    var double: fn(int) -> int = fn (x: int) -> int { x * 2 };
    var result: int = apply_twice(double, 5);
    println(result);  // 20 (5 * 2 * 2)
}
```

## Collection Processing Examples

### ForEach Pattern

```naml
fn for_each(items: [int], action: fn(int)) {
    for (item: int in items) {
        action(item);
    }
}

fn main() {
    var numbers: [int] = [1, 2, 3, 4, 5];

    for_each(numbers, fn (x: int) {
        println(fmt("Number: {}", x));
    });
}
```

### Find Pattern

```naml
fn find(items: [int], predicate: fn(int) -> bool) -> option<int> {
    for (item: int in items) {
        if (predicate(item)) {
            return some(item);
        }
    }
    return none;
}

fn main() {
    var numbers: [int] = [1, 2, 3, 4, 5];

    var first_even: option<int> = find(numbers, fn (x: int) -> bool {
        x % 2 == 0
    });

    println(first_even ?? -1);  // 2
}
```

### All and Any Patterns

```naml
fn all(items: [int], predicate: fn(int) -> bool) -> bool {
    for (item: int in items) {
        if (not predicate(item)) {
            return false;
        }
    }
    return true;
}

fn any(items: [int], predicate: fn(int) -> bool) -> bool {
    for (item: int in items) {
        if (predicate(item)) {
            return true;
        }
    }
    return false;
}

fn main() {
    var numbers: [int] = [2, 4, 6, 8];

    var all_even: bool = all(numbers, fn (x: int) -> bool {
        x % 2 == 0
    });
    println(all_even);  // true

    var has_odd: bool = any(numbers, fn (x: int) -> bool {
        x % 2 == 1
    });
    println(has_odd);  // false
}
```

## Complete Example: Functional Pipeline

```naml
fn map(items: [int], transform: fn(int) -> int) -> [int] {
    use std::collections::push;
    var result: [int] = [];
    for (item: int in items) {
        push(result, transform(item));
    }
    return result;
}

fn filter(items: [int], predicate: fn(int) -> bool) -> [int] {
    use std::collections::push;
    var result: [int] = [];
    for (item: int in items) {
        if (predicate(item)) {
            push(result, item);
        }
    }
    return result;
}

fn fold(items: [int], initial: int, combine: fn(int, int) -> int) -> int {
    var result: int = initial;
    for (item: int in items) {
        result = combine(result, item);
    }
    return result;
}

fn main() {
    var numbers: [int] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    // Pipeline: filter evens -> square -> sum
    var evens: [int] = filter(numbers, fn (x: int) -> bool {
        x % 2 == 0
    });

    var squared: [int] = map(evens, fn (x: int) -> int {
        x * x
    });

    var sum: int = fold(squared, 0, fn (acc: int, x: int) -> int {
        acc + x
    });

    println(fmt("Sum of squares of evens: {}", sum));  // 220
}
```

## Lambda Limitations

1. **No type inference** - All lambda parameters must have explicit types
2. **No capture by reference** - Lambdas capture variables by value
3. **Explicit return types** - Return types must be specified in function signatures

## Best Practices

1. **Keep lambdas simple** - Short lambdas are easier to read inline
2. **Extract complex lambdas** - Assign complex lambdas to variables with descriptive names
3. **Use descriptive parameter names** - Even in short lambdas, use clear names
4. **Prefer named functions** - For reusable logic, define named functions
5. **Combine with generics** - Use generic functions for reusable higher-order functions
6. **Document lambda parameters** - Add comments for complex lambda signatures

## Comparison with Named Functions

```naml
// Named function
fn double(x: int) -> int {
    return x * 2;
}

// Lambda assigned to variable
var double_lambda: fn(int) -> int = fn (x: int) -> int { x * 2 };

// Inline lambda
var result: int = apply(fn (x: int) -> int { x * 2 }, 5);
```

Use named functions for:
- Reusable logic used in multiple places
- Complex logic that benefits from a descriptive name
- Recursive functions (lambdas cannot be recursive)

Use lambdas for:
- One-off operations passed to higher-order functions
- Simple transformations in map/filter/fold operations
- Callbacks and event handlers
