---
title: "std::os"
description: Operating system information
---

Operating system information and platform-specific utilities.

## Import

```naml
use std::os::*;
```

## System Information

### hostname

Get system hostname.

```naml
fn hostname() -> string throws OSError
```

**Example:**

```naml
var name: string = hostname() catch e {
    println(e.message);
    return;
};
println(fmt("Hostname: {}", name));
```

### temp_dir

Get system temporary directory.

```naml
fn temp_dir() -> string
```

**Example:**

```naml
var tmp: string = temp_dir();  // "/tmp" on Unix, "C:\Temp" on Windows
```

### home_dir

Get user home directory.

```naml
fn home_dir() -> string throws OSError
```

**Example:**

```naml
var home: string = home_dir() catch e {
    println(e.message);
    return;
};
```

### cache_dir

Get user cache directory.

```naml
fn cache_dir() -> string throws OSError
```

**Example:**

```naml
var cache: string = cache_dir() catch e {
    println(e.message);
    return;
};
// "/home/user/.cache" on Linux, "~/Library/Caches" on macOS
```

### config_dir

Get user configuration directory.

```naml
fn config_dir() -> string throws OSError
```

**Example:**

```naml
var config: string = config_dir() catch e {
    println(e.message);
    return;
};
// "/home/user/.config" on Linux, "~/Library/Application Support" on macOS
```

### executable

Get path to current executable.

```naml
fn executable() -> string throws OSError
```

**Example:**

```naml
var exe: string = executable() catch e {
    println(e.message);
    return;
};
println(fmt("Running from: {}", exe));
```

### pagesize

Get system memory page size.

```naml
fn pagesize() -> int
```

**Example:**

```naml
var page: int = pagesize();  // Usually 4096 on modern systems
println(fmt("Page size: {} bytes", page));
```

## User and Group IDs (Unix)

### getuid

Get real user ID.

```naml
fn getuid() -> int
```

**Example:**

```naml
var uid: int = getuid();
println(fmt("User ID: {}", uid));
```

### geteuid

Get effective user ID.

```naml
fn geteuid() -> int
```

**Example:**

```naml
var euid: int = geteuid();
```

### getgid

Get real group ID.

```naml
fn getgid() -> int
```

**Example:**

```naml
var gid: int = getgid();
```

### getegid

Get effective group ID.

```naml
fn getegid() -> int
```

**Example:**

```naml
var egid: int = getegid();
```

### getgroups

Get supplementary group IDs.

```naml
fn getgroups() -> [int]
```

**Example:**

```naml
var groups: [int] = getgroups();
for (i: int, gid: int in groups) {
    println(fmt("Group: {}", gid));
}
```

## Complete Example

```naml
use std::os::*;

fn main() {
    println("=== System Information ===");

    var host: string = hostname() catch e {
        "unknown";
    };
    println(fmt("Hostname: {}", host));

    println(fmt("Temp dir: {}", temp_dir()));

    var home: string = home_dir() catch e {
        "unknown";
    };
    println(fmt("Home dir: {}", home));

    var cache: string = cache_dir() catch e {
        "unknown";
    };
    println(fmt("Cache dir: {}", cache));

    var config: string = config_dir() catch e {
        "unknown";
    };
    println(fmt("Config dir: {}", config));

    var exe: string = executable() catch e {
        "unknown";
    };
    println(fmt("Executable: {}", exe));

    println(fmt("Page size: {} bytes", pagesize()));

    println("\n=== User Information ===");
    println(fmt("UID: {}", getuid()));
    println(fmt("EUID: {}", geteuid()));
    println(fmt("GID: {}", getgid()));
    println(fmt("EGID: {}", getegid()));

    var groups: [int] = getgroups();
    println(fmt("Groups: {} groups", count(groups)));
}
```
