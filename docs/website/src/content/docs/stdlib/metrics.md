---
title: "std::metrics"
description: Performance measurement utilities
---

High-precision performance measurement utilities.

## Import

```naml
use std::metrics::*;
```

## Functions

### perf_now

Get high-precision timestamp for benchmarking.

```naml
fn perf_now() -> int
```

**Returns:** Timestamp in nanoseconds.

**Example:**

```naml
var start: int = perf_now();
// ... code to benchmark ...
var end: int = perf_now();
var elapsed: int = end - start;
```

### elapsed_ms

Calculate elapsed milliseconds between two timestamps.

```naml
fn elapsed_ms(start: int, end: int) -> int
```

**Example:**

```naml
var start: int = perf_now();
// ... work ...
var end: int = perf_now();
var ms: int = elapsed_ms(start, end);
println(fmt("Took {} ms", ms));
```

### elapsed_us

Calculate elapsed microseconds between two timestamps.

```naml
fn elapsed_us(start: int, end: int) -> int
```

**Example:**

```naml
var start: int = perf_now();
// ... work ...
var end: int = perf_now();
var us: int = elapsed_us(start, end);
println(fmt("Took {} us", us));
```

### elapsed_ns

Calculate elapsed nanoseconds between two timestamps.

```naml
fn elapsed_ns(start: int, end: int) -> int
```

**Example:**

```naml
var start: int = perf_now();
// ... work ...
var end: int = perf_now();
var ns: int = elapsed_ns(start, end);
println(fmt("Took {} ns", ns));
```

## Benchmarking Example

```naml
use std::metrics::*;

fn fibonacci(n: int) -> int {
    if (n <= 1) {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

fn main() {
    var start: int = perf_now();
    var result: int = fibonacci(30);
    var end: int = perf_now();

    println(fmt("fibonacci(30) = {}", result));
    println(fmt("Time: {} ms", elapsed_ms(start, end)));
    println(fmt("Time: {} us", elapsed_us(start, end)));
    println(fmt("Time: {} ns", elapsed_ns(start, end)));
}
```
