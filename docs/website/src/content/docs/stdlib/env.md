---
title: "std::env"
description: Environment variable access
---

Environment variable access and manipulation.

## Import

```naml
use std::env::*;
```

## Functions

### getenv

Get environment variable value, returns empty string if not set.

```naml
fn getenv(name: string) -> string
```

**Example:**

```naml
var home: string = getenv("HOME");
var path: string = getenv("PATH");
```

### lookup_env

Get environment variable as option, returns `none` if not set.

```naml
fn lookup_env(name: string) -> option<string>
```

**Example:**

```naml
var home: string = lookup_env("HOME") ?? "/default";
var custom: string = lookup_env("MY_VAR") ?? "not set";
```

### setenv

Set environment variable.

```naml
fn setenv(name: string, value: string) throws EnvError
```

**Example:**

```naml
setenv("MY_VAR", "my_value") catch e {
    println(e.message);
};
```

### unsetenv

Unset environment variable.

```naml
fn unsetenv(name: string) throws EnvError
```

**Example:**

```naml
unsetenv("MY_VAR") catch e {
    println(e.message);
};
```

### clearenv

Clear all environment variables.

```naml
fn clearenv() throws EnvError
```

**Warning:** This removes all environment variables, which may break system functionality.

**Example:**

```naml
clearenv() catch e {
    println(e.message);
};
```

### environ

Get all environment variables as array of "KEY=VALUE" strings.

```naml
fn environ() -> [string]
```

**Example:**

```naml
var all_vars: [string] = environ();
for (i: int, v: string in all_vars) {
    println(v);
}
```

### expand_env

Expand environment variables in string using $VAR or ${VAR} syntax.

```naml
fn expand_env(s: string) -> string
```

**Example:**

```naml
setenv("GREETING", "Hello") catch e {
    println(e.message);
};
var msg: string = expand_env("$GREETING from ${HOME}");
// "Hello from /home/user"
```

## Complete Example

```naml
use std::env::*;

fn main() {
    var home: string = getenv("HOME");
    println(fmt("HOME = {}", home));

    setenv("MY_APP_VAR", "test_value") catch e {
        println(e.message);
        return;
    };

    var found: string = lookup_env("MY_APP_VAR") ?? "not found";
    println(fmt("MY_APP_VAR = {}", found));

    var expanded: string = expand_env("Running from ${HOME}");
    println(expanded);

    unsetenv("MY_APP_VAR") catch e {
        println(e.message);
    };

    var after: string = lookup_env("MY_APP_VAR") ?? "not found";
    println(fmt("After unset: {}", after));
}
```
