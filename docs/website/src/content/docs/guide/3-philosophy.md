---
title: Philosophy
description: naml's design principles and philosophy
---

## Design Principles

naml is built around four core principles that shape every decision in the language:

### 1. Zero-Allocation

**Avoid per-operation heap allocations.**

The compiler uses arenas and string interning instead of allocating memory for each token, AST node, or identifier. This makes compilation blazingly fast and predictable.

```naml
# String interning means identical strings share memory
var name1: string = "Alice";
var name2: string = "Alice";  # Same interned string
```

In the runtime, reference counting manages heap objects efficiently without a garbage collector.

### 2. Zero-Copy

**Reference source text directly instead of cloning strings.**

The lexer and parser work with spans (offset + length) into the original source text. This eliminates unnecessary string copies during compilation.

```naml
# The compiler never copies "my_variable_name" during parsing
var my_variable_name: int = 42;
```

This principle extends to runtime operations where possible, using slices and views instead of copies.

### 3. Zero-GC

**No garbage collection. Deterministic memory management.**

naml uses reference counting for heap objects (strings, arrays, maps, structs). Memory is freed immediately when the last reference goes out of scope.

```naml
fn process_data() {
    var data: [int] = [1, 2, 3, 4, 5];
    # Use data...
}  # data is freed here, deterministically
```

Benefits:
- Predictable performance (no GC pauses)
- Low memory overhead
- Works well in resource-constrained environments
- Suitable for real-time systems

### 4. Explicit Types

**Required type annotations for all variables.**

naml deliberately requires type annotations. There is no type inference for variable declarations. This is a feature, not a limitation.

```naml
# Good - clear and explicit
var count: int = 0;
var items: [string] = [];
var result: option<Person> = find_person("Alice");

# Would not compile - missing type annotation
var x = 42;  # Error: type annotation required
```

**Why explicit types?**

1. **Self-documenting code**: Anyone reading the code knows exactly what type each variable holds
2. **Better tooling**: IDEs and editors can provide accurate autocomplete and refactoring
3. **Clearer error messages**: Type errors are caught at the declaration site
4. **No surprises**: The type you write is the type you get, no implicit conversions
5. **Faster compilation**: No expensive type inference algorithms

Function return types and parameter types are also always explicit:

```naml
fn process(items: [string], limit: int) -> option<string> {
    # Clear contract: takes array and int, returns optional string
    if (limit > len(items)) {
        return none;
    }
    return some(items[limit]);
}
```

## Strong Static Typing

All types are checked at compile time. There are no dynamic type checks at runtime (except for option unwrapping and error handling).

```naml
var x: int = 42;
var y: string = "hello";

# This won't compile - type mismatch
x = y;  # Error: cannot assign string to int
```

naml's type system includes:
- Primitives: `int`, `uint`, `float`, `bool`, `string`, `bytes`
- Collections: `[T]` (arrays), `map<K, V>`
- Options: `option<T>`
- Custom types: `struct`, `enum`
- Generics: `option<T>`, `channel<T>`

## Cross-Platform from Day One

Every feature in naml explicitly handles all three target platforms:

- **Native**: Self-contained binaries with embedded runtime
- **Server WASM**: Own runtime that executes WASM (like Bun/Deno)
- **Browser WASM**: Runs in browsers via wasm-bindgen

Platform-specific code is explicit:

```naml
#[platforms(native, server)]
fn read_file(path: string) -> string throws IOError {
    # Native file system access
}

#[platforms(browser)]
fn read_file(path: string) -> string throws IOError {
    # Browser OPFS access
}
```

This design ensures your code is portable from the start. You know which features work where.

## Performance by Default

naml is designed for speed:

- **Cranelift JIT** for instant execution during development
- **Rustc backend** for heavily optimized production binaries
- **Reference counting** with immediate deallocation
- **No hidden allocations** or performance cliffs
- **Explicit concurrency** with spawn blocks and channels

```naml
# Spawning is explicit - no hidden thread pool
spawn {
    process_in_background();
};

# Channels are typed and bounded
var ch: channel<int> = make_channel(100);
```

## Simplicity and Clarity

naml favors explicit, readable code over brevity:

- Type annotations make code self-documenting
- No operator overloading or implicit conversions
- Clear distinction between references and values
- Explicit error handling with `throws` and `try/catch`

```naml
# Clear and explicit
fn divide(a: int, b: int) -> int throws string {
    if (b == 0) {
        throw "Division by zero";
    }
    return a / b;
}

# Usage is also explicit
try {
    var result: int = divide(10, 0);
} catch (err: string) {
    println(err);
}
```

## Summary

naml's philosophy can be summarized as:

> **Fast, explicit, deterministic, and cross-platform.**

These principles guide every feature and design decision in the language. The result is a language that:
- Compiles and runs fast
- Has predictable performance
- Works across platforms
- Is easy to read and maintain
- Scales from small scripts to large applications
