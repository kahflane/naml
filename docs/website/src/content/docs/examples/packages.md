---
title: Packages
description: Using local and Git dependencies with naml pkg
---

## Local Package Dependency

A project that uses a local math library as a package dependency.

### Project Structure

```
pkg-demo/
├── naml.toml
├── main.nm
└── libs/
    └── mathlib/
        ├── naml.toml
        └── main.nm
```

### Project Manifest

**naml.toml**:

```toml
[package]
name = "pkg-demo"
version = "0.1.0"
description = "Demonstrates using naml pkg with local dependencies"

[dependencies]
mathlib = { path = "./libs/mathlib" }
```

### Library Package

**libs/mathlib/naml.toml**:

```toml
[package]
name = "mathlib"
version = "0.1.0"
description = "Simple math utilities library"
```

**libs/mathlib/main.nm**:

```naml
pub fn add(a: int, b: int) -> int {
    return a + b;
}

pub fn multiply(a: int, b: int) -> int {
    return a * b;
}

pub fn factorial(n: int) -> int {
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}
```

### Main Program

**main.nm**:

```naml
use mathlib::*;

fn main() {
    var result: int = add(10, 20);
    println(fmt("10 + 20 = {}", result));

    var product: int = multiply(6, 7);
    println(fmt("6 * 7 = {}", product));

    var fact: int = factorial(5);
    println(fmt("5! = {}", fact));
}
```

### Running

```bash
naml run main.nm
```

Output:

```
10 + 20 = 30
6 * 7 = 42
5! = 120
```

## Multi-Package Project

A larger project with multiple local dependencies:

### Project Structure

```
my-app/
├── naml.toml
├── main.nm
└── libs/
    ├── math/
    │   ├── naml.toml
    │   └── main.nm
    └── strings/
        ├── naml.toml
        └── main.nm
```

### Manifest

**naml.toml**:

```toml
[package]
name = "my-app"
version = "0.1.0"

[dependencies]
math = { path = "./libs/math" }
strings = { path = "./libs/strings" }
```

### String Utilities Package

**libs/strings/main.nm**:

```naml
pub fn repeat(s: string, n: int) -> string {
    var result: string = "";
    var i: int = 0;
    while (i < n) {
        result = result + s;
        i = i + 1;
    }
    return result;
}

pub fn pad_left(s: string, width: int, ch: string) -> string {
    var padding: int = width - length(s);
    if (padding <= 0) {
        return s;
    }
    return repeat(ch, padding) + s;
}
```

### Main Program

**main.nm**:

```naml
use math::*;
use strings::*;

fn main() {
    var nums: [int] = [1, 2, 3, 4, 5];
    var total: int = 0;
    for (i: int, n: int in nums) {
        total = total + n;
    }

    println(fmt("Sum: {}", total));
    println(fmt("Padded: {}", pad_left("42", 8, "0")));
}
```

## Git Dependencies

Using a package from a Git repository:

```toml
[package]
name = "web-app"
version = "0.1.0"

[dependencies]
json = { git = "https://github.com/naml-lang/json", tag = "v0.1.0" }
http = { git = "https://github.com/naml-lang/http", branch = "main" }
```

```naml
use json::*;
use http::*;

fn main() {
    # Use functions exported by the json and http packages
}
```

Run `naml pkg get` to download Git dependencies, or let `naml run` fetch them automatically.
