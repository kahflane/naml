---
title: Concurrency
description: Spawn, channels, mutex, rwlock, and atomics in naml
---

## Channels

Basic channel communication between the main thread and spawned tasks:

```naml
use std::threads::*;

fn main() {
    var ch: channel<int> = open_channel(5);

    // Send values
    send(ch, 10);
    send(ch, 20);
    send(ch, 30);

    // Receive values (returns option<T>)
    var v1: int = receive(ch) ?? 0;
    var v2: int = receive(ch) ?? 0;
    var v3: int = receive(ch) ?? 0;
    println("Received: {}, {}, {}", v1, v2, v3);

    // Channel with loop
    var i: int = 0;
    while (i < 5) {
        send(ch, i * 10);
        i = i + 1;
    }

    var sum: int = 0;
    i = 0;
    while (i < 5) {
        sum = sum + (receive(ch) ?? 0);
        i = i + 1;
    }
    println("Sum: {}", sum);

    close(ch);
}
```

## Concurrent Task Processing

A multi-worker system using mutex, rwlock, channels, and spawn together:

```naml
use std::threads::*;

fn main() {
    // Shared state
    var completed: mutex<int> = with_mutex(0);
    var stats: rwlock<int> = with_rwlock(0);

    // Channels for task distribution and result collection
    var tasks: channel<int> = open_channel(20);
    var results: channel<int> = open_channel(20);

    // Spawn 3 workers
    spawn { worker_loop(1, tasks, results, completed, stats); };
    spawn { worker_loop(2, tasks, results, completed, stats); };
    spawn { worker_loop(3, tasks, results, completed, stats); };

    sleep(100);

    // Dispatch 10 tasks
    var i: int = 1;
    while (i <= 10) {
        send(tasks, i * 10);
        i = i + 1;
    }

    close(tasks);
    join();

    // Collect results
    var total: int = 0;
    var count: int = 0;
    while (count < 10) {
        var val: int = receive(results) ?? -1;
        if (val != -1) {
            total = total + val;
            count = count + 1;
        }
    }

    // Read final stats
    locked (c: int in completed) {
        println("Tasks completed: {}", c);
    }
    rlocked (t: int in stats) {
        println("Total input processed: {}", t);
    }
    println("Total output (doubled): {}", total);
}

fn worker_loop(
    id: int,
    tasks: channel<int>,
    results: channel<int>,
    completed: mutex<int>,
    stats: rwlock<int>
) {
    while (true) {
        var value: int = receive(tasks) ?? -1;
        if (value == -1) {
            return;
        }

        sleep(50);
        var result: int = value * 2;

        locked (count: int in completed) {
            count = count + 1;
        }

        wlocked (total: int in stats) {
            total = total + value;
        }

        send(results, result);
    }
}
```

This example demonstrates:
- **Channels** for distributing work and collecting results
- **Mutex** for an exclusive counter
- **RwLock** for shared statistics
- **Spawn** for parallel worker threads
- **Join** to wait for all workers to finish
