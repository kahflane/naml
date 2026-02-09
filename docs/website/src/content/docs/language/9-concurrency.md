---
title: Concurrency
description: Learn about spawn blocks, channels, mutex, rwlock, atomics, and concurrent programming in naml
---

naml provides comprehensive concurrency primitives for building concurrent applications. Most concurrency features require `use std::threads::*;` except for the `spawn` keyword.

## Spawn

Create concurrent tasks with `spawn`. The `spawn` keyword is always available without imports:

```naml
spawn {
    // This runs concurrently
    do_work();
};
```

### Multiple Spawns

```naml
spawn { task1(); };
spawn { task2(); };
spawn { task3(); };
```

### Capturing Variables

Spawn blocks can capture variables from the outer scope:

```naml
var x: int = 10;
spawn {
    println(x);  // Captures x
};
```

## Channels

Channels enable communication between concurrent tasks. Requires `use std::threads::*;`:

```naml
use std::threads::*;

fn main() {
    var ch: channel<int> = open_channel(10);

    spawn {
        send(ch, 42);
    };

    join();

    // receive() returns option<T> - none if channel is closed
    var value: int = receive(ch) ?? 0;
    println(value);
    close(ch);
}
```

### Channel Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `open_channel` | `(capacity: int) -> channel<T>` | Create a buffered channel |
| `send` | `(ch: channel<T>, value: T)` | Send a value (blocks if full) |
| `receive` | `(ch: channel<T>) -> option<T>` | Receive a value (blocks if empty, returns `none` if closed) |
| `close` | `(ch: channel<T>)` | Close the channel |

### Producer-Consumer Example

```naml
use std::threads::*;

fn producer(ch: channel<int>) {
    for (i: int in 0..10) {
        send(ch, i);
    }
}

fn consumer(ch: channel<int>) {
    loop {
        var value: option<int> = receive(ch);
        if (value == none) {
            break;
        }
        println(value!);
    }
}

fn main() {
    var ch: channel<int> = open_channel(5);

    spawn { producer(ch); };
    spawn { consumer(ch); };

    join();
    close(ch);
}
```

## Mutex

Mutual exclusion locks for protecting shared state. Use the `locked` keyword to acquire exclusive access. Requires `use std::threads::*;`:

```naml
use std::threads::*;

fn main() {
    var counter: mutex<int> = with_mutex(0);

    spawn { increment(counter); };
    spawn { increment(counter); };
    spawn { increment(counter); };

    join();

    locked (val: int in counter) {
        println(fmt("counter = {}", val));  // 3
    }
}

fn increment(m: mutex<int>) {
    locked (val: int in m) {
        val = val + 1;
    }
}
```

### Mutex Functions and Syntax

| Function/Syntax | Description |
|-----------------|-------------|
| `with_mutex(value)` | Create a mutex with initial value |
| `locked (name: Type in mutex_var) { ... }` | Acquire exclusive lock, bind inner value to `name`, release on block exit |

Supported inner types: `int`, `uint`, `float`, `bool`, `string`.

## RwLock

Read-write locks allowing multiple concurrent readers or one exclusive writer. Requires `use std::threads::*;`:

```naml
use std::threads::*;

fn main() {
    var stats: rwlock<int> = with_rwlock(0);

    spawn { write_stats(stats); };
    spawn { write_stats(stats); };
    spawn { read_stats(stats); };

    join();

    rlocked (val: int in stats) {
        println(fmt("stats = {}", val));
    }
}

fn write_stats(rw: rwlock<int>) {
    wlocked (val: int in rw) {
        val = val + 10;
    }
}

fn read_stats(rw: rwlock<int>) {
    rlocked (val: int in rw) {
        println(fmt("current stats = {}", val));
    }
}
```

### RwLock Functions and Syntax

| Function/Syntax | Description |
|-----------------|-------------|
| `with_rwlock(value)` | Create a rwlock with initial value |
| `rlocked (name: Type in rwlock_var) { ... }` | Acquire shared read lock, bind inner value to `name` |
| `wlocked (name: Type in rwlock_var) { ... }` | Acquire exclusive write lock, bind inner value to `name` |

Supported inner types: `int`, `uint`, `float`, `bool`, `string`.

## Atomics

Lock-free atomic operations for shared state between threads. All operations use sequential consistency ordering. Requires `use std::threads::*;`:

```naml
use std::threads::*;

fn main() {
    var counter: atomic<int> = with_atomic(0);

    spawn { atomic_add(counter, 1); };
    spawn { atomic_add(counter, 1); };
    spawn { atomic_add(counter, 1); };

    join();
    println(fmt("counter = {}", atomic_load(counter)));  // 3
}
```

### Atomic Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `with_atomic` | `(value: T) -> atomic<T>` | Create an atomic with initial value |
| `atomic_load` | `(a: atomic<T>) -> T` | Read the current value |
| `atomic_store` | `(a: atomic<T>, value: T)` | Write a new value |
| `atomic_add` | `(a: atomic<T>, value: T) -> T` | Add and return old value (`int`/`uint` only) |
| `atomic_sub` | `(a: atomic<T>, value: T) -> T` | Subtract and return old value (`int`/`uint` only) |
| `atomic_inc` | `(a: atomic<T>) -> T` | Increment by 1, return old value (`int`/`uint` only) |
| `atomic_dec` | `(a: atomic<T>) -> T` | Decrement by 1, return old value (`int`/`uint` only) |
| `atomic_cas` | `(a: atomic<T>, expected: T, new: T) -> bool` | Compare-and-swap, returns true on success |
| `atomic_swap` | `(a: atomic<T>, value: T) -> T` | Swap and return old value |
| `atomic_and` | `(a: atomic<T>, value: T) -> T` | Bitwise AND, return old value (`int`/`uint` only) |
| `atomic_or` | `(a: atomic<T>, value: T) -> T` | Bitwise OR, return old value (`int`/`uint` only) |
| `atomic_xor` | `(a: atomic<T>, value: T) -> T` | Bitwise XOR, return old value (`int`/`uint` only) |

Supported inner types: `int`, `uint`, `bool`. Boolean atomics support `with_atomic`, `atomic_load`, `atomic_store`, `atomic_cas`, and `atomic_swap`.

### Compare-and-Swap Example

```naml
use std::threads::*;

fn increment_if_less_than(counter: atomic<int>, max: int) {
    loop {
        var current: int = atomic_load(counter);
        if (current >= max) {
            break;
        }
        var success: bool = atomic_cas(counter, current, current + 1);
        if (success) {
            break;
        }
        // CAS failed, retry
    }
}

fn main() {
    var counter: atomic<int> = with_atomic(0);

    spawn { increment_if_less_than(counter, 100); };
    spawn { increment_if_less_than(counter, 100); };

    join();
    println(atomic_load(counter));
}
```

## Join

Wait for all spawned tasks to complete. Requires `use std::threads::*;`:

```naml
use std::threads::*;

fn main() {
    spawn { task1(); };
    spawn { task2(); };

    join();  // Block until all spawned tasks complete
}
```

### Join Pattern

Always call `join()` after spawning tasks to wait for completion:

```naml
use std::threads::*;

fn main() {
    var counter: atomic<int> = with_atomic(0);

    for (i: int in 0..10) {
        spawn {
            atomic_inc(counter);
        };
    }

    join();  // Wait for all increments to complete

    println(fmt("Final count: {}", atomic_load(counter)));
}
```

## Complete Concurrency Example

```naml
use std::threads::*;

fn worker(id: int, ch: channel<int>, counter: mutex<int>) {
    for (i: int in 0..5) {
        var value: int = receive(ch) ?? -1;
        if (value == -1) {
            break;
        }

        println(fmt("Worker {} received {}", id, value));

        locked (count: int in counter) {
            count = count + value;
        }
    }
}

fn main() {
    var ch: channel<int> = open_channel(10);
    var counter: mutex<int> = with_mutex(0);

    // Spawn workers
    spawn { worker(1, ch, counter); };
    spawn { worker(2, ch, counter); };
    spawn { worker(3, ch, counter); };

    // Send work
    for (i: int in 0..15) {
        send(ch, i);
    }

    join();
    close(ch);

    // Print final result
    locked (count: int in counter) {
        println(fmt("Total: {}", count));
    }
}
```

## Concurrency Best Practices

1. **Always call join()** - Wait for spawned tasks to complete
2. **Close channels** - Close channels when done sending to signal completion
3. **Minimize lock contention** - Keep locked/rlocked/wlocked blocks small
4. **Prefer atomics for counters** - Use atomic operations for simple counters
5. **Use channels for communication** - Prefer message passing over shared state
6. **Handle receive failures** - Check for `none` when receiving from channels
7. **Avoid deadlocks** - Never nest mutex locks, use consistent lock ordering
