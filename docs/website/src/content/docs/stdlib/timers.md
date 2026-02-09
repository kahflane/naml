---
title: "std::timers"
description: Timeout, interval, and cron-style scheduling
---

Timer functions for delayed execution, repeating tasks, and cron-style scheduling.

## Import

```naml
use std::timers::*;
use std::threads::{open_channel, send, receive, close, join, sleep};  // For examples
```

## Timeout

### set_timeout

Execute callback after delay.

```naml
fn set_timeout(callback: fn(), delay_ms: int) -> int
```

**Parameters:**
- `callback` - Function to execute (can capture variables)
- `delay_ms` - Delay in milliseconds

**Returns:** Timer handle.

**Example:**

```naml
var ch: channel<int> = open_channel(1);

set_timeout(fn() {
    send(ch, 42);
}, 1000);  // Execute after 1 second

var result: int = receive(ch) ?? 0;
println(fmt("Received: {}", result));
close(ch);
```

### cancel_timeout

Cancel a pending timeout.

```naml
fn cancel_timeout(handle: int)
```

**Example:**

```naml
var timer: int = set_timeout(fn() {
    println("This won't execute");
}, 5000);

cancel_timeout(timer);  // Cancel before it fires
```

## Interval

### set_interval

Execute callback repeatedly at fixed intervals.

```naml
fn set_interval(callback: fn(), interval_ms: int) -> int
```

**Parameters:**
- `callback` - Function to execute on each tick
- `interval_ms` - Interval in milliseconds

**Returns:** Interval handle.

**Example:**

```naml
var ch: channel<int> = open_channel(10);
var count: int = 0;

var iv: int = set_interval(fn() {
    count = count + 1;
    send(ch, count);
}, 100);  // Tick every 100ms

sleep(500);  // Let it run for 500ms
cancel_interval(iv);
join();
close(ch);

// Process ticks
var done: bool = false;
while (!done) {
    var tick: int = receive(ch) ?? -1;
    if (tick == -1) {
        done = true;
    } else {
        println(fmt("Tick: {}", tick));
    }
}
```

### cancel_interval

Cancel a repeating interval.

```naml
fn cancel_interval(handle: int)
```

**Example:**

```naml
var iv: int = set_interval(fn() {
    println("Tick");
}, 1000);

sleep(5000);
cancel_interval(iv);  // Stop after 5 seconds
```

## Cron Scheduling

### schedule

Schedule callback using cron expression.

```naml
fn schedule(callback: fn(), cron_expr: string) -> int throws ScheduleError
```

**Cron format:** `second minute hour day-of-month month day-of-week`

**Cron fields:**
- `second` - 0-59
- `minute` - 0-59
- `hour` - 0-23
- `day-of-month` - 1-31
- `month` - 1-12
- `day-of-week` - 0-6 (0=Sunday)

**Special characters:**
- `*` - Any value
- `,` - Value list (e.g., `1,15,30`)
- `-` - Range (e.g., `1-5`)
- `/` - Step (e.g., `*/5` = every 5 units)

**Returns:** Job handle.

**Throws:** `ScheduleError` if cron expression is invalid.

**Example:**

```naml
// Run every second
var job1: int = schedule(fn() {
    println("Every second");
}, "* * * * * *") catch e {
    println(e.message);
    return;
};

// Run every minute at second 0
var job2: int = schedule(fn() {
    println("Every minute");
}, "0 * * * *") catch e {
    println(e.message);
    return;
};

// Run at 3:00 AM every day
var job3: int = schedule(fn() {
    println("Daily at 3 AM");
}, "0 0 3 * * *") catch e {
    println(e.message);
    return;
};

// Run every 5 minutes
var job4: int = schedule(fn() {
    println("Every 5 minutes");
}, "0 */5 * * * *") catch e {
    println(e.message);
    return;
};
```

### cancel_schedule

Cancel a scheduled job.

```naml
fn cancel_schedule(handle: int)
```

**Example:**

```naml
var job: int = schedule(fn() {
    println("Scheduled task");
}, "0 * * * *") catch e {
    println(e.message);
    return;
};

// Later, cancel the job
cancel_schedule(job);
```

### next_run

Get next scheduled run time as Unix timestamp in milliseconds.

```naml
fn next_run(handle: int) -> int
```

**Returns:** Next execution timestamp, or 0 if job is cancelled/invalid.

**Example:**

```naml
var job: int = schedule(fn() {
    println("Task");
}, "0 * * * *") catch e {
    println(e.message);
    return;
};

var next: int = next_run(job);
if (next > 0) {
    println(fmt("Next run at: {}", next));
}
```

## Complete Example

```naml
use std::timers::*;
use std::threads::{open_channel, send, receive, close, join, sleep};
use std::datetime::{now_ms};

fn main() {
    println("=== Timer Demo ===");

    // Test 1: Timeout
    println("\nTest 1: Timeout after 100ms");
    var ch1: channel<int> = open_channel(1);
    set_timeout(fn() {
        send(ch1, 42);
    }, 100);
    var val1: int = receive(ch1) ?? 0;
    println(fmt("Received: {}", val1));
    close(ch1);

    // Test 2: Cancel timeout
    println("\nTest 2: Cancel timeout");
    var ch2: channel<int> = open_channel(1);
    var t2: int = set_timeout(fn() {
        send(ch2, 999);
    }, 500);
    cancel_timeout(t2);
    sleep(600);
    join();
    close(ch2);
    var val2: int = receive(ch2) ?? -1;
    println(fmt("After cancel: {} (should be -1)", val2));

    // Test 3: Interval
    println("\nTest 3: Interval every 80ms");
    var ch3: channel<string> = open_channel(10);
    var iv: int = set_interval(fn() {
        send(ch3, "tick");
    }, 80);

    sleep(350);
    cancel_interval(iv);
    sleep(100);
    join();
    close(ch3);

    var ticks: int = 0;
    var done: bool = false;
    while (!done) {
        var tick: string = receive(ch3) ?? "done";
        if (tick == "done") {
            done = true;
        } else {
            ticks = ticks + 1;
        }
    }
    println(fmt("Ticked {} times", ticks));

    // Test 4: Cron schedule
    println("\nTest 4: Cron schedule");
    var ch4: channel<int> = open_channel(5);
    var job: int = schedule(fn() {
        send(ch4, 1);
    }, "* * * * * *") catch e {  // Every second
        println(fmt("Schedule error: {}", e.message));
        return;
    };

    println(fmt("Job scheduled, next run: {}", next_run(job)));

    sleep(1500);
    join();
    close(ch4);

    var fires: int = 0;
    var done2: bool = false;
    while (!done2) {
        var fire: int = receive(ch4) ?? -1;
        if (fire == -1) {
            done2 = true;
        } else {
            fires = fires + 1;
        }
    }
    println(fmt("Job fired {} times", fires));

    cancel_schedule(job);
    println(fmt("After cancel, next run: {}", next_run(job)));

    println("\n=== Demo Complete ===");
}
```

## Common Cron Patterns

```naml
// Every second
"* * * * * *"

// Every minute
"0 * * * *"

// Every hour at minute 0
"0 0 * * * *"

// Every day at midnight
"0 0 0 * * *"

// Every Monday at 9 AM
"0 0 9 * * 1"

// Every 15 minutes
"0 */15 * * * *"

// Every weekday at 8:30 AM
"0 30 8 * * 1-5"

// First day of every month at noon
"0 0 12 1 * *"
```
