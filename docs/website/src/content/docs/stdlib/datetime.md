---
title: "std::datetime"
description: Date and time utilities
---

Date and time utilities for working with timestamps and formatting dates.

## Import

```naml
use std::datetime::*;
```

## Time Functions

### now_ms

Get current Unix timestamp in milliseconds.

```naml
fn now_ms() -> int
```

**Example:**

```naml
var timestamp: int = now_ms();  // 1704067200000
```

### now_s

Get current Unix timestamp in seconds.

```naml
fn now_s() -> int
```

**Example:**

```naml
var timestamp: int = now_s();  // 1704067200
```

## Date Component Extraction

All extraction functions take a timestamp in milliseconds and return UTC values.

### year

Extract year from timestamp.

```naml
fn year(timestamp: int) -> int
```

**Example:**

```naml
var y: int = year(now_ms());  // 2024
```

### month

Extract month from timestamp (1-12).

```naml
fn month(timestamp: int) -> int
```

**Example:**

```naml
var m: int = month(now_ms());  // 1-12
```

### day

Extract day of month from timestamp (1-31).

```naml
fn day(timestamp: int) -> int
```

**Example:**

```naml
var d: int = day(now_ms());  // 1-31
```

### hour

Extract hour from timestamp (0-23).

```naml
fn hour(timestamp: int) -> int
```

**Example:**

```naml
var h: int = hour(now_ms());  // 0-23
```

### minute

Extract minute from timestamp (0-59).

```naml
fn minute(timestamp: int) -> int
```

**Example:**

```naml
var min: int = minute(now_ms());  // 0-59
```

### second

Extract second from timestamp (0-59).

```naml
fn second(timestamp: int) -> int
```

**Example:**

```naml
var sec: int = second(now_ms());  // 0-59
```

### day_of_week

Get day of week (0=Sunday, 6=Saturday).

```naml
fn day_of_week(timestamp: int) -> int
```

**Example:**

```naml
var dow: int = day_of_week(now_ms());
if (dow == 0) {
    println("It's Sunday!");
}
```

## Date Formatting

### format_date

Format timestamp as string using format specifiers.

```naml
fn format_date(timestamp: int, format: string) -> string
```

**Format Specifiers:**
- `YYYY` - 4-digit year
- `MM` - 2-digit month (01-12)
- `DD` - 2-digit day (01-31)
- `HH` - 2-digit hour (00-23)
- `mm` - 2-digit minute (00-59)
- `ss` - 2-digit second (00-59)

**Example:**

```naml
var ts: int = now_ms();
var iso: string = format_date(ts, "YYYY-MM-DD");
// "2024-01-15"

var full: string = format_date(ts, "YYYY-MM-DD HH:mm:ss");
// "2024-01-15 14:30:45"

var custom: string = format_date(ts, "MM/DD/YYYY");
// "01/15/2024"
```

## Complete Example

```naml
use std::datetime::*;

fn main() {
    var ts: int = now_ms();

    println(fmt("Current timestamp: {}", ts));
    println(fmt("Date: {}", format_date(ts, "YYYY-MM-DD")));
    println(fmt("Time: {}", format_date(ts, "HH:mm:ss")));
    println(fmt("Full: {}", format_date(ts, "YYYY-MM-DD HH:mm:ss")));

    println(fmt("Year: {}", year(ts)));
    println(fmt("Month: {}", month(ts)));
    println(fmt("Day: {}", day(ts)));
    println(fmt("Hour: {}", hour(ts)));
    println(fmt("Minute: {}", minute(ts)));
    println(fmt("Second: {}", second(ts)));

    var dow: int = day_of_week(ts);
    var day_names: [string] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
    println(fmt("Day of week: {}", day_names[dow]!));
}
```
