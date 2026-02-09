---
title: "std::threads"
description: Concurrency primitives for multi-threaded programming
---

Concurrency primitives including channels, mutexes, read-write locks, atomics, and thread management.

## Import

```naml
use std::threads::*;
```

## Platform Support

Most threading features are **native-only**. Atomics and basic concurrency work on server WASM.

## Thread Management

### sleep

Pause execution for milliseconds.

```naml
fn sleep(ms: int)
```

**Example:**

```naml
sleep(1000);  // Sleep for 1 second
```

### join

Wait for all spawned tasks to complete.

```naml
fn join()
```

**Example:**

```naml
spawn { task1(); };
spawn { task2(); };
join();  // Block until both tasks finish
```

## Channels

Thread-safe message passing for communication between concurrent tasks.

### open_channel

Create a buffered channel.

```naml
fn open_channel<T>(capacity: int) -> channel<T>
```

**Parameters:**
- `capacity` - Buffer size for pending messages

**Example:**

```naml
var ch: channel<int> = open_channel(10);
```

### send

Send a value through a channel (blocks if full).

```naml
fn send<T>(ch: channel<T>, value: T)
```

**Example:**

```naml
send(ch, 42);
```

### receive

Receive a value from channel (blocks if empty, returns `none` if closed).

```naml
fn receive<T>(ch: channel<T>) -> option<T>
```

**Example:**

```naml
var value: int = receive(ch) ?? 0;
```

### close

Close a channel.

```naml
fn close<T>(ch: channel<T>)
```

**Example:**

```naml
close(ch);
```

### Channel Usage Example

```naml
use std::threads::*;

fn main() {
    var ch: channel<int> = open_channel(10);

    spawn {
        send(ch, 42);
    };

    join();
    var result: int = receive(ch) ?? 0;
    println(result);
    close(ch);
}
```

## Mutex

Mutual exclusion locks for protecting shared state.

### with_mutex

Create a mutex with initial value.

```naml
fn with_mutex<T>(value: T) -> mutex<T>
```

**Supported types:** `int`, `uint`, `float`, `bool`, `string`

**Example:**

```naml
var counter: mutex<int> = with_mutex(0);
```

### locked

Acquire exclusive lock and access inner value.

```naml
locked (name: Type in mutex_var) {
    // Access and modify inner value
}
```

**Example:**

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

## RwLock

Read-write locks allowing multiple concurrent readers or one exclusive writer.

### with_rwlock

Create a rwlock with initial value.

```naml
fn with_rwlock<T>(value: T) -> rwlock<T>
```

**Supported types:** `int`, `uint`, `float`, `bool`, `string`

**Example:**

```naml
var stats: rwlock<int> = with_rwlock(0);
```

### rlocked

Acquire shared read lock.

```naml
rlocked (name: Type in rwlock_var) {
    // Read-only access to inner value
}
```

**Example:**

```naml
rlocked (val: int in stats) {
    println(fmt("current value = {}", val));
}
```

### wlocked

Acquire exclusive write lock.

```naml
wlocked (name: Type in rwlock_var) {
    // Exclusive write access to inner value
}
```

**Example:**

```naml
wlocked (val: int in stats) {
    val = val + 10;
}
```

### RwLock Usage Example

```naml
use std::threads::*;

fn main() {
    var stats: rwlock<int> = with_rwlock(0);

    spawn { write_stats(stats); };
    spawn { write_stats(stats); };
    spawn { read_stats(stats); };

    join();

    rlocked (val: int in stats) {
        println(fmt("final stats = {}", val));
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

## Atomics

Lock-free atomic operations for high-performance concurrent programming.

### with_atomic

Create an atomic with initial value.

```naml
fn with_atomic<T>(value: T) -> atomic<T>
```

**Supported types:** `int`, `uint`, `bool`

**Example:**

```naml
var counter: atomic<int> = with_atomic(0);
var flag: atomic<bool> = with_atomic(false);
```

### atomic_load

Read the current value.

```naml
fn atomic_load<T>(a: atomic<T>) -> T
```

**Example:**

```naml
var value: int = atomic_load(counter);
```

### atomic_store

Write a new value.

```naml
fn atomic_store<T>(a: atomic<T>, value: T)
```

**Example:**

```naml
atomic_store(counter, 10);
```

### atomic_add

Add and return old value (int/uint only).

```naml
fn atomic_add<T>(a: atomic<T>, value: T) -> T
```

**Example:**

```naml
var old: int = atomic_add(counter, 5);
```

### atomic_sub

Subtract and return old value (int/uint only).

```naml
fn atomic_sub<T>(a: atomic<T>, value: T) -> T
```

**Example:**

```naml
var old: int = atomic_sub(counter, 3);
```

### atomic_inc

Increment by 1, return old value (int/uint only).

```naml
fn atomic_inc<T>(a: atomic<T>) -> T
```

**Example:**

```naml
var old: int = atomic_inc(counter);
```

### atomic_dec

Decrement by 1, return old value (int/uint only).

```naml
fn atomic_dec<T>(a: atomic<T>) -> T
```

**Example:**

```naml
var old: int = atomic_dec(counter);
```

### atomic_cas

Compare-and-swap, returns true on success.

```naml
fn atomic_cas<T>(a: atomic<T>, expected: T, new: T) -> bool
```

**Example:**

```naml
var swapped: bool = atomic_cas(counter, 0, 1);
if (swapped) {
    println("Value was 0, now is 1");
}
```

### atomic_swap

Swap and return old value.

```naml
fn atomic_swap<T>(a: atomic<T>, value: T) -> T
```

**Example:**

```naml
var old: int = atomic_swap(counter, 100);
```

### atomic_and

Bitwise AND, return old value (int/uint only).

```naml
fn atomic_and<T>(a: atomic<T>, value: T) -> T
```

**Example:**

```naml
var old: int = atomic_and(flags, 0xFF);
```

### atomic_or

Bitwise OR, return old value (int/uint only).

```naml
fn atomic_or<T>(a: atomic<T>, value: T) -> T
```

**Example:**

```naml
var old: int = atomic_or(flags, 0x01);
```

### atomic_xor

Bitwise XOR, return old value (int/uint only).

```naml
fn atomic_xor<T>(a: atomic<T>, value: T) -> T
```

**Example:**

```naml
var old: int = atomic_xor(flags, 0x10);
```

### Atomic Usage Example

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

## Memory Ordering

All atomic operations use **sequential consistency** ordering, providing the strongest memory ordering guarantees.
