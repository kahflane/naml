---
title: Control Flow
description: Learn about conditional statements, loops, pattern matching, and control flow keywords in naml
---

naml provides comprehensive control flow constructs including conditionals, loops, and pattern matching.

## If/Else

Conditional branching with `if`, `else if`, and `else`:

```naml
if (condition) {
    // then branch
} else if (other_condition) {
    // else if branch
} else {
    // else branch
}
```

### Important Note

`if` is a **statement**, not an expression. It cannot return a value directly. Use variable assignment inside the branches instead:

```naml
var result: string;
if (x > 0) {
    result = "positive";
} else {
    result = "non-positive";
}
```

### Examples

```naml
var age: int = 25;

if (age >= 18) {
    println("Adult");
} else {
    println("Minor");
}

// Multiple conditions
var score: int = 85;
if (score >= 90) {
    println("A");
} else if (score >= 80) {
    println("B");
} else if (score >= 70) {
    println("C");
} else {
    println("F");
}
```

## While Loop

Repeat while a condition is true:

```naml
while (condition) {
    // loop body
}
```

### Example

```naml
var count: int = 0;
while (count < 10) {
    println(count);
    count = count + 1;
}
```

## Loop (Infinite)

Create an infinite loop that runs until explicitly broken:

```naml
loop {
    // runs forever until break
    if (done) {
        break;
    }
}
```

### Example

```naml
var i: int = 0;
loop {
    println(i);
    i = i + 1;
    if (i >= 5) {
        break;
    }
}
```

## For Loop

### Range Iteration

Iterate over exclusive ranges with `..`:

```naml
for (i: int in 0..10) {
    // i: 0, 1, 2, ..., 9
}
```

Iterate over inclusive ranges with `..=`:

```naml
for (i: int in 0..=10) {
    // i: 0, 1, 2, ..., 10
}
```

### Collection Iteration

Iterate over array elements:

```naml
for (item: string in array) {
    // iterate items
}
```

Iterate with both index and value:

```naml
for (index: int, item: string in array) {
    // index and item
}
```

### Examples

```naml
// Range iteration
for (i: int in 0..5) {
    println(i);  // 0, 1, 2, 3, 4
}

// Array iteration
var names: [string] = ["Alice", "Bob", "Charlie"];
for (name: string in names) {
    println(name);
}

// With index
for (i: int, name: string in names) {
    println(fmt("{}. {}", i, name));
}
```

## Break and Continue

### Break

Exit a loop immediately:

```naml
for (i: int in 0..100) {
    if (i == 50) {
        break;  // Exit loop
    }
    println(i);
}
```

### Continue

Skip to the next iteration:

```naml
for (i: int in 0..100) {
    if (i % 2 == 0) {
        continue;  // Skip even numbers
    }
    println(i);  // Only prints odd numbers
}
```

### Example

```naml
var numbers: [int] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

for (num: int in numbers) {
    if (num == 5) {
        break;  // Stop at 5
    }
    if (num % 2 == 0) {
        continue;  // Skip even numbers
    }
    println(num);  // Prints 1, 3
}
```

## Switch/Case

Pattern matching with `switch`:

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

### Pattern Matching Types

**Literal patterns**:

```naml
switch (value) {
    case 1: { }
    case "hello": { }
    case true: { }
    default: { }
}
```

**Identifier pattern** (binds value):

```naml
switch (value) {
    case x: {
        // x is bound to value
        println(x);
    }
}
```

**Wildcard pattern**:

```naml
switch (value) {
    case _: {
        // matches anything
    }
}
```

### Enum Pattern Matching

Pattern matching with enums:

```naml
switch (status) {
    case Status::Active: {
        // handle active
    }
    case Status::Suspended(reason): {
        // handle suspended, bind reason
        println(reason);
    }
    case _: {
        // wildcard match
    }
}
```

### Example with Enum Destructuring

```naml
enum UserStatus {
    Active,
    Suspended(string),
    Banned(string, int)
}

var status: UserStatus = UserStatus::Suspended("Policy violation");

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

## Return

Exit a function and optionally return a value:

```naml
fn example() -> int {
    return 42;
}

fn no_return() {
    return;  // Return with no value
}
```

### Early Return

Return early based on conditions:

```naml
fn validate(x: int) -> bool {
    if (x < 0) {
        return false;
    }
    if (x > 100) {
        return false;
    }
    return true;
}
```

## Complete Control Flow Example

```naml
fn process_numbers(numbers: [int]) {
    for (i: int, num: int in numbers) {
        // Skip negative numbers
        if (num < 0) {
            continue;
        }

        // Stop at 100
        if (num >= 100) {
            println("Reached limit");
            break;
        }

        // Process based on value
        switch (num) {
            case 0: {
                println("Zero");
            }
            case 1: {
                println("One");
            }
            default: {
                if (num % 2 == 0) {
                    println(fmt("{} is even", num));
                } else {
                    println(fmt("{} is odd", num));
                }
            }
        }
    }
}

fn main() {
    var nums: [int] = [0, 1, 5, 10, -3, 50, 100, 200];
    process_numbers(nums);
}
```
