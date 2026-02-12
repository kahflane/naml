---
title: Types
description: Comprehensive guide to primitive types, composite types, and literals in naml
---

naml provides a rich type system with primitive types, composite types, and literals for building robust applications.

## Primitive Types

naml provides the following primitive types with explicit sizes:

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

## Composite Types

### Arrays

**Dynamic arrays** that can grow:

```naml
var numbers: [int] = [1, 2, 3, 4, 5];
var empty: [string] = [];
```

**Fixed-size arrays**:

```naml
var fixed: [int; 5] = [1, 2, 3, 4, 5];
```

**Array operations** (using `std::collections`):

```naml
use std::collections::*;

var first: int = numbers[0];           // Indexing
var len: int = count(numbers);         // Length
push(numbers, 6);                      // Append
var last: option<int> = pop(numbers);  // Remove last, returns option<T>
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

// Null coalescing - returns value or default
var value: int = maybe ?? 0;      // 42
var other: int = nothing ?? -1;   // -1

// Force unwrap - panics if none
var unwrapped: int = maybe!;      // 42

// Handle with else block
var safe: int = maybe else {
    println("Value was none!");
    return;
} ?? 0;
```

### Channels

Communication channels for concurrency (native/server only). Requires `use std::threads::*;`:

```naml
use std::threads::*;

var ch: channel<int> = open_channel(10);  // buffered channel with capacity 10
```

### Mutex

Mutual exclusion locks for shared state between threads (native only). Requires `use std::threads::*;`:

```naml
use std::threads::*;

var counter: mutex<int> = with_mutex(0);    // mutex-protected integer
```

Supported inner types: `int`, `uint`, `float`, `bool`, `string`.

### RwLock

Read-write locks allowing multiple concurrent readers or one exclusive writer (native only). Requires `use std::threads::*;`:

```naml
use std::threads::*;

var stats: rwlock<int> = with_rwlock(0);    // rwlock-protected integer
```

Supported inner types: `int`, `uint`, `float`, `bool`, `string`.

### Atomics

Lock-free atomic types for concurrent programming (native only). Requires `use std::threads::*;`:

```naml
use std::threads::*;

var counter: atomic<int> = with_atomic(0);    // atomic integer
var flags: atomic<uint> = with_atomic(0);     // atomic unsigned integer
var ready: atomic<bool> = with_atomic(false);  // atomic boolean
```

Supported inner types: `int`, `uint`, `bool`.

### Function Types

First-class function types:

```naml
var callback: fn(int, int) -> int;
var predicate: fn(string) -> bool;
```

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

## Type Aliases

Create an alias for an existing type with the `type` keyword:

```naml
type UserID = int;
type Headers = map<string, string>;
type Handler = fn(int) -> int;
```

Aliases can be public and generic:

```naml
pub type Result<T> = option<T>;
pub type Pair<A, B> = (A, B);
```

Use aliases just like the original type:

```naml
type UserID = int;

fn find_user(id: UserID) -> string {
    return fmt("User {}", id);
}

fn main() {
    var id: UserID = 42;
    println(find_user(id));
}
```

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

### Conditional Operators

| Operator | Description |
|----------|-------------|
| `? :` | Ternary conditional |
| `?:` | Elvis (default if falsy) |

The **ternary operator** evaluates a condition and returns one of two values:

```naml
var result: int = condition ? true_value : false_value;

// Examples
var age: int = 25;
var status: int = age >= 18 ? 1 : 0;  // 1 (adult)

// Chained ternary for multiple conditions
var score: int = 85;
var grade: int = score >= 90 ? 4 : score >= 80 ? 3 : score >= 70 ? 2 : 1;
```

The **elvis operator** returns the left value if truthy (non-zero/non-empty), otherwise the right value:

```naml
var result: int = value ?: default;

// Examples
var count: int = 0;
var display: int = count ?: 10;  // 10 (count is falsy)

var items: int = 5;
var actual: int = items ?: 1;    // 5 (items is truthy)

// Chained elvis - returns first truthy value
var first: int = a ?: b ?: c ?: 99;
```

### Type Operators

| Operator | Description |
|----------|-------------|
| `as` | Type casting (infallible) |
| `as?` | Fallible cast (returns option) |
| `is` | Type/variant check |
| `!` | Force unwrap (panics if none) |

**Type casting** with `as`:

```naml
var str: string = number as string;
var num: int = "42" as int;  // Parses string to int
```

**Fallible cast** with `as?` returns `option<T>` - `some(value)` on success, `none` on failure:

```naml
var input: string = "42";
var maybe: option<int> = input as? int;  // some(42)
var value: int = maybe ?? 0;             // 42

var bad: string = "not a number";
var failed: option<int> = bad as? int;   // none
var safe: int = failed ?? -1;            // -1

// Common pattern: parse with default
var parsed: int = user_input as? int ?? 0;
```

**Force unwrap** with `!` extracts the value from an option, panicking if none:

```naml
use std::collections::pop;

var opt: option<int> = some(42);
var value: int = opt!;           // 42

var arr: [int] = [1, 2, 3];
var last: int = pop(arr)!;       // 3 (panics if array was empty)
```

### Null Coalescing

| Operator | Description |
|----------|-------------|
| `??` | Returns left if some, else right |

```naml
var maybe: option<int> = some(42);
var value: int = maybe ?? 0;     // 42

var nothing: option<int> = none;
var other: int = nothing ?? -1;  // -1
```
