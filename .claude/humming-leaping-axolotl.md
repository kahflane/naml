# naml Language Syntax Reference

## Types

### Primitive Types
| Type | Description |
|------|-------------|
| `int` | Signed 64-bit integer |
| `uint` | Unsigned 64-bit integer |
| `float` | 64-bit floating-point |
| `bool` | Boolean (`true`/`false`) |
| `string` | UTF-8 text string |
| `bytes` | Raw byte data |
| `unit` | Unit type (void) |

### Collection Types
| Syntax | Description |
|--------|-------------|
| `[T]` | Dynamic array |
| `[T; N]` | Fixed-size array |
| `map<K, V>` | Hash map |
| `option<T>` | Optional value |

### Concurrency Types
| Syntax | Description |
|--------|-------------|
| `channel<T>` | Message channel (native/server) |
| `promise<T>` | Async promise |

### User-Defined Types
- `StructName` - Named struct/enum types
- `Generic<T>` - Generic types with type parameters

---

## Literals

```naml
42              // int
42u             // uint
3.14            // float
true, false     // bool
"hello"         // string
b"bytes"        // bytes
none            // option none
```

---

## Operators

### Arithmetic
`+`, `-`, `*`, `/`, `%`

### Comparison
`==`, `!=`, `<`, `<=`, `>`, `>=`, `is`

### Logical
`&&`, `||`, `!`

### Bitwise
`&`, `|`, `^`, `~`, `<<`, `>>`

### Range
`..` (exclusive), `..=` (inclusive)

### Assignment
`=`, `+=`, `-=`, `*=`, `/=`, `%=`, `&=`, `|=`, `^=`

---

## Expressions

### Function & Method Calls
```naml
func(arg1, arg2)
func<Type>(arg)
obj.method(arg)
obj.method<Type>(arg)
```

### Access & Indexing
```naml
struct.field
array[index]
map["key"]
Module::Item
Enum::Variant
```

### Collection Literals
```naml
[1, 2, 3]                           // Array
{key: value, key2: value2}          // Map
StructName { field: value }         // Struct
some(value)                         // Option some
```

### Lambda/Closure
```naml
|x: int, y: int| -> int { x + y }
|x| x * 2
```

### Control Flow Expressions
```naml
if (cond) { a } else { b }          // If expression
{ statements; result }              // Block expression
1..10                               // Range
```

### Async/Concurrency
```naml
spawn { block }                     // Spawn task
await promise                       // Await result
```

### Error Handling
```naml
try expr                            // Propagate error
expr catch e { handler }            // Catch error
expr as Type                        // Type cast
```

---

## Statements

### Variable Declaration
```naml
var name: Type = value;
var mut name: Type = value;         // Mutable
const NAME: Type = value;
```

### Control Flow
```naml
// If statement
if (condition) {
    ...
} else if (condition2) {
    ...
} else {
    ...
}

// While loop
while (condition) {
    ...
}

// For loop
for (item in collection) { ... }
for (i, item in collection) { ... }   // With index

// Infinite loop
loop {
    ...
}

// Switch/Pattern matching
switch (value) {
    case Pattern: ...
    case Variant(x): ...
    default: ...
}

// Loop control
break;
continue;
return value;
```

### Error Handling
```naml
throw error;
```

---

## Top-Level Items

### Functions
```naml
fn name(param: Type) -> ReturnType {
    body
}

async fn name() -> Promise<T> { }
pub fn name() { }                    // Public
fn name() throws Error { }           // Throws

#[platforms(native, server, browser)]
fn platform_specific() { }
```

### Methods
```naml
pub fn (self: Type) method() -> ReturnType { }
pub fn (mut self: Type) mutating_method() { }
```

### Generics
```naml
fn identity<T>(x: T) -> T { x }
struct Container<T> { value: T }
```

### Structs
```naml
struct Point {
    x: int,
    y: int
}

pub struct Person {
    pub name: string,
    age: int
}
```

### Enums
```naml
enum Color {
    Red,
    Green,
    Blue
}

enum Result {
    Ok(value),
    Error(message)
}
```

### Interfaces
```naml
interface Drawable {
    fn draw(self: Self);
}

interface Extended extends Base {
    fn extra(self: Self);
}
```

### Exceptions
```naml
exception NetworkError {
    code: int,
    message: string
}
```

### Imports
```naml
import module::path;
import module::path as alias;
use module::{Item1, Item2};
use module::*;
```

### External Functions
```naml
extern fn c_function(arg: int) -> int;
extern fn custom() #[link_name = "actual_name"];
```

---

## Patterns (for Switch)

```naml
case 42:                    // Literal pattern
case name:                  // Identifier pattern
case Variant:               // Unit variant
case Variant(x, y):         // Variant with destructuring
case _:                     // Wildcard (default)
```

---

## Built-in Methods

### Array Methods
- `.len()` - Get length
- `.push(value)` - Append element
- `.pop()` - Remove last element
- `array[index]` - Index access

### Map Methods
- `.len()` - Get length
- `.contains(key)` - Check key exists
- `map[key]` - Index access
- `map[key] = value` - Set value

### Option Methods
- `.is_some()` - Check if Some
- `.is_none()` - Check if None
- `.or_default(default)` - Get value or default
- `.unwrap()` - Get value (panics if None)

### String Methods
- `.len()` - Get length

---

## Special Syntax

### Platform Annotations
```naml
#[platforms(native, server)]
fn file_io() { }

#[platforms(browser)]
fn browser_api() { }
```

### Comments
```naml
// Single line comment

//!
//! Block comment (triple slash)
//! Used at file/function level
//!
```

---

## Operator Precedence (High to Low)

1. `*`, `/`, `%`
2. `+`, `-`
3. `<<`, `>>`
4. `&`
5. `^`
6. `|`
7. `<`, `<=`, `>`, `>=`, `is`
8. `==`, `!=`
9. `&&`
10. `||`
11. `..`, `..=`
