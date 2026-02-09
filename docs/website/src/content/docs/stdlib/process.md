---
title: "std::process"
description: Process management and signals
---

Process management, spawning, and signal handling.

## Import

```naml
use std::process::*;
```

## Current Process

### getpid

Get current process ID.

```naml
fn getpid() -> int
```

**Example:**

```naml
var pid: int = getpid();
println(fmt("My PID: {}", pid));
```

### getppid

Get parent process ID.

```naml
fn getppid() -> int
```

**Example:**

```naml
var ppid: int = getppid();
println(fmt("Parent PID: {}", ppid));
```

### exit

Exit process with status code.

```naml
fn exit(code: int)
```

**Example:**

```naml
exit(0);  // Success
exit(1);  // Error
```

## Pipes

### pipe_read

Read from pipe file descriptor.

```naml
fn pipe_read(fd: int) -> string throws ProcessError
```

**Example:**

```naml
var output: string = pipe_read(fd) catch e {
    println(e.message);
    return;
};
```

### pipe_write

Write to pipe file descriptor.

```naml
fn pipe_write(fd: int, data: string) throws ProcessError
```

**Example:**

```naml
pipe_write(fd, "hello") catch e {
    println(e.message);
};
```

## Process Management

### start_process

Start a new process.

```naml
fn start_process(command: string, args: [string]) -> int throws ProcessError
```

**Returns:** Process handle.

**Example:**

```naml
var proc: int = start_process("ls", ["-la"]) catch e {
    println(e.message);
    return;
};
```

### find_process

Find process by PID.

```naml
fn find_process(pid: int) -> int throws ProcessError
```

**Returns:** Process handle.

**Example:**

```naml
var proc: int = find_process(1234) catch e {
    println(e.message);
    return;
};
```

### wait

Wait for process to complete.

```naml
fn wait(process: int) -> int throws ProcessError
```

**Returns:** Exit status.

**Example:**

```naml
var status: int = wait(proc) catch e {
    println(e.message);
    return;
};
println(fmt("Process exited with status: {}", status));
```

### signal

Send signal to process.

```naml
fn signal(process: int, sig: int) throws ProcessError
```

**Example:**

```naml
signal(proc, SIGTERM) catch e {
    println(e.message);
};
```

### kill

Forcefully kill process (sends SIGKILL).

```naml
fn kill(process: int) throws ProcessError
```

**Example:**

```naml
kill(proc) catch e {
    println(e.message);
};
```

### release

Release process handle.

```naml
fn release(process: int)
```

**Example:**

```naml
release(proc);
```

## Signal Constants

The following signal constants are available:

```naml
const SIGHUP: int = 1;     // Hangup
const SIGINT: int = 2;     // Interrupt (Ctrl+C)
const SIGQUIT: int = 3;    // Quit
const SIGKILL: int = 9;    // Kill (cannot be caught)
const SIGTERM: int = 15;   // Terminate
const SIGSTOP: int = 19;   // Stop (cannot be caught)
const SIGCONT: int = 18;   // Continue
```

**Example:**

```naml
use std::process::*;

fn main() {
    var proc: int = start_process("sleep", ["10"]) catch e {
        println(e.message);
        return;
    };

    println("Started process, sending SIGTERM...");
    signal(proc, SIGTERM) catch e {
        println(e.message);
    };

    var status: int = wait(proc) catch e {
        println(e.message);
        return;
    };

    println(fmt("Process exit status: {}", status));
    release(proc);
}
```

## Complete Example

```naml
use std::process::*;

fn main() {
    println(fmt("Current PID: {}", getpid()));
    println(fmt("Parent PID: {}", getppid()));

    var proc: int = start_process("echo", ["Hello from subprocess"]) catch e {
        println(fmt("Failed to start: {}", e.message));
        return;
    };

    var status: int = wait(proc) catch e {
        println(fmt("Wait failed: {}", e.message));
        return;
    };

    println(fmt("Process exited with status: {}", status));
    release(proc);
}
```
