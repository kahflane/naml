---
title: "std::random"
description: Random number generation
---

Random number generation utilities.

## Import

```naml
use std::random::*;
```

## Functions

### random

Generate random integer in range [min, max] (inclusive).

```naml
fn random(min: int, max: int) -> int
```

**Example:**

```naml
var dice: int = random(1, 6);  // 1-6
var percent: int = random(0, 100);  // 0-100
```

### random_float

Generate random float in range [0.0, 1.0).

```naml
fn random_float() -> float
```

**Example:**

```naml
var r: float = random_float();  // 0.0 <= r < 1.0
var scaled: float = random_float() * 100.0;  // 0.0 <= scaled < 100.0
```

## Usage Example

```naml
use std::random::*;

fn main() {
    println("Rolling dice:");
    var i: int = 0;
    while (i < 5) {
        var roll: int = random(1, 6);
        println(fmt("  Roll {}: {}", i + 1, roll));
        i = i + 1;
    }

    println("\nRandom percentages:");
    var j: int = 0;
    while (j < 3) {
        var pct: float = random_float() * 100.0;
        println(fmt("  {}", pct));
        j = j + 1;
    }
}
```
