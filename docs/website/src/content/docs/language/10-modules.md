---
title: Modules & Imports
description: Learn about module system, module paths, use statements, and visibility in naml
---

naml uses a hierarchical module system where files and directories map to modules. This enables code organization and namespace management.

## Declaring Modules

Use the `mod` keyword to declare submodules.

### File-based Modules

Declare a module that corresponds to a file or directory:

```naml
// In main.nm
mod math;        // Looks for math.nm or math/mod.nm
mod network;     // Looks for network.nm or network/mod.nm
```

The module system searches for:
1. `math.nm` in the same directory
2. `math/mod.nm` in a subdirectory

### Inline Modules

Define modules inline within a file:

```naml
mod utils {
    pub fn helper() {
        println("Helper function");
    }

    fn internal() {
        // Not visible outside this module
    }
}
```

## Module Paths

Paths use `::` as a separator to traverse the module hierarchy:

### Absolute Paths

Start from the standard library or root:

```naml
std::io::println
std::collections::push
std::threads::spawn
```

### Relative Paths

**Current module** with `self::`:

```naml
self::function_name
self::Type
```

**Parent module** with `super::`:

```naml
super::parent_function
super::ParentType
```

**Root module** with `::`:

```naml
::root_function
::RootType
```

## Use Statements

Import functions, structs, enums, interfaces, and exceptions into the current scope.

### Wildcard Imports

Import all public items from a module:

```naml
use std::io::*;
use std::collections::*;
use math::*;
```

### Specific Imports

Import specific items:

```naml
use std::collections::arrays::{push, count};
use std::io::{println, read_line};
```

### Import with Alias

Rename imports to avoid conflicts:

```naml
use network::tcp::Client as TcpClient;
use network::udp::Client as UdpClient;

var tcp: TcpClient = TcpClient {};
var udp: UdpClient = UdpClient {};
```

## Visibility

### Public Items

Use `pub` to make items visible outside their module:

```naml
pub fn public_function() -> int {
    return 42;
}

pub struct PublicStruct {
    pub field: int  // Public field
}

pub enum PublicEnum {
    Variant1,
    Variant2
}

pub interface PublicInterface {
    fn method();
}

pub exception PublicException {
    message: string
}
```

### Private Items

Items without `pub` are private to their module:

```naml
fn private_function() -> int {
    return 42;
}

struct PrivateStruct {
    field: int
}
```

## Module Organization Examples

### Simple Module Structure

```
project/
├── main.nm
├── math.nm
└── utils.nm
```

**main.nm**:
```naml
mod math;
mod utils;

use math::add;
use utils::*;

fn main() {
    var result: int = add(10, 20);
    println(result);
}
```

**math.nm**:
```naml
pub fn add(a: int, b: int) -> int {
    return a + b;
}

pub fn subtract(a: int, b: int) -> int {
    return a - b;
}
```

**utils.nm**:
```naml
pub fn helper() {
    println("Helper function");
}
```

### Nested Module Structure

```
project/
├── main.nm
└── math/
    ├── mod.nm
    ├── basic.nm
    └── advanced.nm
```

**main.nm**:
```naml
mod math;

use math::basic::add;
use math::advanced::*;

fn main() {
    var result: int = add(10, 20);
    println(result);
}
```

**math/mod.nm**:
```naml
pub mod basic;
pub mod advanced;
```

**math/basic.nm**:
```naml
pub fn add(a: int, b: int) -> int {
    return a + b;
}

pub fn subtract(a: int, b: int) -> int {
    return a - b;
}
```

**math/advanced.nm**:
```naml
pub fn power(base: int, exp: int) -> int {
    var result: int = 1;
    for (i: int in 0..exp) {
        result = result * base;
    }
    return result;
}
```

## Standard Library Modules

naml provides several standard library modules:

### std::random

Random number generation:

```naml
use std::random::*;

var n: int = random(1, 100);      // Random int in [min, max]
var f: float = random_float();     // Random float in [0.0, 1.0)
```

### std::io

Terminal I/O and console control:

```naml
use std::io::*;

var key: int = read_key();         // Non-blocking key read
clear_screen();
set_cursor(10, 5);
hide_cursor();
show_cursor();
var w: int = terminal_width();
var h: int = terminal_height();
```

### std::threads

Concurrency primitives:

```naml
use std::threads::*;

var ch: channel<int> = open_channel(10);
var m: mutex<int> = with_mutex(0);
var rw: rwlock<int> = with_rwlock(0);
var a: atomic<int> = with_atomic(0);
join();
```

### std::collections

Collection operations:

```naml
use std::collections::*;

var arr: [int] = [1, 2, 3];
push(arr, 4);
var last: option<int> = pop(arr);
var len: int = count(arr);
```

## Complete Module Example

**main.nm**:
```naml
mod geometry;
mod utils;

use geometry::shapes::*;
use utils::format::*;

fn main() {
    var circle: Circle = Circle { radius: 5.0 };
    var rect: Rectangle = Rectangle { width: 10.0, height: 5.0 };

    println(format_shape("Circle", circle.area()));
    println(format_shape("Rectangle", rect.area()));
}
```

**geometry/mod.nm**:
```naml
pub mod shapes;
```

**geometry/shapes.nm**:
```naml
pub struct Circle {
    pub radius: float
}

pub fn (self: Circle) area() -> float {
    return 3.14159 * self.radius * self.radius;
}

pub struct Rectangle {
    pub width: float,
    pub height: float
}

pub fn (self: Rectangle) area() -> float {
    return self.width * self.height;
}
```

**utils/mod.nm**:
```naml
pub mod format;
```

**utils/format.nm**:
```naml
pub fn format_shape(name: string, area: float) -> string {
    return fmt("{} area: {}", name, area);
}
```

## External Packages

For code that lives outside your project, naml provides a built-in package manager (`naml pkg`). Packages are declared in `naml.toml` and imported just like local modules:

```toml
[dependencies]
mathlib = { path = "./libs/mathlib" }
json = { git = "https://github.com/naml-lang/json", tag = "v0.1.0" }
```

```naml
use mathlib::*;
use json::{parse, stringify};
```

See the [Package Manager guide](/guide/5-packages/) for full details.

## Best Practices

1. **Organize by feature** - Group related functionality in modules
2. **Use descriptive names** - Module names should describe their purpose
3. **Minimize wildcard imports** - Prefer specific imports for clarity
4. **Keep module trees shallow** - Avoid deep nesting of modules
5. **Export only public APIs** - Keep implementation details private
6. **Use mod.nm for organization** - Re-export submodules in mod.nm
7. **Consistent naming** - Use snake_case for module file names
