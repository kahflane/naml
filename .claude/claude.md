# naml Programming Language - Development Guidelines

## Project Overview

naml is a fast, cross-platform programming language targeting:
- **Native**: Self-contained binary with embedded runtime
- **Server WASM**: Own runtime that executes WASM (like Bun/Deno)
- **Browser WASM**: Runs in browsers via wasm-bindgen

## Code Rules

### File Size Limit
All source files must stay **under 1000 lines**. If a file approaches this limit, split it into logical submodules.

### Comments
Use **block comments only** at the top of files for 80-100 lines. No inline comments.
Comments should be written in **Markdown**.
Comments must start with a triple-slash (`///`).

```rust
##
## This module handles tokenization of naml source code.
## It uses a zero-copy cursor over the input string.
##

fn tokenize() { ... }
```

### Performance Requirements
- **Zero-allocation**: Use arenas and string interning, not per-token allocations
- **Zero-copy**: Reference source text directly, don't clone strings
- **Zero-GC**: No garbage collection, deterministic memory management

### Platform Matrix
Every feature must explicitly handle all three target platforms:

| Feature | Native | Server | Browser |
|---------|--------|--------|---------|
| File I/O | std::fs | WASI | OPFS |
| Networking | std::net | WASI sockets | fetch API |
| Concurrency | threads | single-threaded | async-only |
| Crypto | ring/rustcrypto | WASI crypto | WebCrypto |

Use platform attributes:
```naml
#[platforms(native, server)]
fn read_file(path: string) -> string throws IOError;

#[platforms(browser)]
fn read_file(path: string) -> string throws IOError {
    return opfs_read(path);
}
```

## Architecture

### Compilation Pipeline
```
Source (.naml)
    ↓
  Lexer (zero-copy tokenization)
    ↓
   AST (typed syntax tree)
    ↓
  Parser (recursive descent + Pratt)
    ↓
Type Checker (inference + validation)
    ↓
    ├─→ Cranelift JIT (naml run) → Execute directly
    │         ↓
    │   Code Cache (for <1ms cold start)
    │
    └─→ Rust Codegen (naml build) → rustc → Binary/WASM
```

### Directory Structure
```
naml/
├── Cargo.toml            # Workspace root
├── namlc/                # Compiler crate
│   └── src/
│       ├── main.rs       # CLI entry point
│       ├── lib.rs        # Library root
│       ├── source/       # Source file handling, spans, diagnostics
│       ├── lexer/        # Tokenization
│       ├── ast/          # Abstract syntax tree definitions
│       ├── parser/       # Parsing
│       ├── typechecker/  # Type system
│       ├── codegen/      # Cranelift JIT compilation
│       ├── runtime/      # Runtime re-exports + array/map/bytes
│       ├── manifest/     # naml.toml parsing
│       └── driver/       # Compilation orchestration
│
└── std/                  # Standard library crates
    ├── naml-std-core/    # Core types (HeapHeader, NamlString, NamlStruct)
    ├── naml-std-random/  # Random number generation
    ├── naml-std-io/      # Terminal I/O and console control
    ├── naml-std-threads/ # M:N scheduler and channels
    ├── naml-std-datetime/# Date and time utilities
    └── naml-std-metrics/ # Performance measurement
```

### Standard Library Architecture
```
naml-std-core     → Base types shared by all std crates
    ↑
    ├── naml-std-random   → random(min, max), random_float()
    ├── naml-std-io       → read_key(), terminal_*, cursor control
    ├── naml-std-threads  → spawn, channels, join
    ├── naml-std-datetime → now_ms(), year(), format_date()
    └── naml-std-metrics  → perf_now(), elapsed_ms/us/ns()
```

## Testing

### Test Requirements
- Unit tests for each module (`#[cfg(test)]`)
- Integration tests in `tests/` directory
- Snapshot tests using `insta` for parser output
- Platform-specific tests for native/server/browser

### Running Tests
```bash
cargo test                          # All tests
cargo test --lib                    # Unit tests only
cargo test --test integration       # Integration tests
```

## CLI Commands

```bash
naml run file.naml              # Execute with JIT (fast dev mode)
naml run --cached file.naml     # Use cached compilation (<1ms start)
naml build                      # Build native binary
naml build --target server      # Build server WASM
naml build --target browser     # Build browser WASM
naml check                      # Type check only
naml init                       # Create new project
naml test                       # Run tests
```

## Dependencies

### Core
- `lasso` - String interning (zero-allocation identifiers)
- `cranelift-*` - JIT compilation
- `thiserror` / `miette` - Error handling
- `clap` - CLI parsing
- `libc` - C library bindings for runtime

### Standard Library
- `naml-std-core` - Core runtime types (HeapHeader, NamlString, NamlStruct)
- `naml-std-random` - Random number generation (std::random)
- `naml-std-io` - Terminal I/O (std::io)
- `naml-std-threads` - Concurrency primitives (std::threads)
- `naml-std-datetime` - Date and time utilities (std::datetime)
- `naml-std-metrics` - Performance measurement (std::metrics)

### Serialization
- `serde` / `toml` - Config files
- `blake3` - Cache hashing

### Targets
- `tokio` - Async runtime (native/server)
- `wasm-bindgen` - Browser WASM

## Conventions

### Naming
- Types: `PascalCase` (e.g., `NamlType`, `Expression`)
- Functions: `snake_case` (e.g., `parse_expression`)
- Constants: `SCREAMING_SNAKE_CASE` (e.g., `MAX_RECURSION_DEPTH`)

### Error Handling
- Use `Result<T, E>` for fallible operations
- Define specific error types with `thiserror`
- Provide source spans for all diagnostics

### Module Organization
- Each directory has a `mod.rs` that re-exports public items
- Keep related functionality together
- Prefer many small files over few large files

## Quick Reference

### Type Syntax
```naml
int, uint, float, bool, string, bytes
[T]              # Dynamic array
[T; N]           # Fixed array
option<T>        # Optional
map<K, V>        # Map
channel<T>       # Channel (native/server only)
promise<T>       # Promise
```

### Function Syntax
```naml
fn name(param: Type) -> ReturnType { }
fn name<T>(param: T) -> T { }
fn name() -> T throws Error { }
pub fn (self: Type) method() -> T { }
```

### Control Flow
```naml
if (cond) { } else { }
while (cond) { }
for (i: int, val: T in collection) { }
switch (val) { case X: ... default: ... }
spawn { }
```

## Adding New Standard Library Modules

1. Create a new crate in `std/naml-std-<name>/`
2. Add to workspace members in root `Cargo.toml`
3. Add workspace dependency: `naml-std-<name> = { path = "std/naml-std-<name>" }`
4. If it depends on core types, add `naml-std-core.workspace = true` to its dependencies
5. Add dependency to `namlc/Cargo.toml`
6. Re-export in `namlc/src/runtime/mod.rs`: `pub use naml_std_<name>::*;`
7. Register functions in `namlc/src/typechecker/mod.rs` (get_std_module_functions)
8. Register runtime symbols in `namlc/src/codegen/cranelift/mod.rs`

## Testing and Examples - NEVER Take the Happy Path

### Why This Matters
Simple tests hide bugs. The mutex/rwlock implementation passed all simple tests but crashed when:
- Calling user-defined functions from spawn blocks
- Passing captured variables as function arguments
- Combining multiple features (spawn + channels + mutex + function calls)

### Mandatory Testing Rules

**1. Always test the complex case FIRST**
```naml
// BAD - This will pass even with critical bugs
var m: mutex<int> = with_mutex(0);
locked (v: int in m) { v = v + 1; }
println(v);  // "Works!"

// GOOD - This exposes real bugs
spawn {
    worker_function(mutex_var, channel_var, rwlock_var);
};
```

**2. Combine multiple features together**
- Spawn + channels + mutex + rwlock + function calls
- Loops + conditionals + error handling
- Multiple threads accessing shared state

**3. Pass values through function calls**
Don't just use variables in the same scope - pass them to functions called from spawn blocks.

**4. Test with multiple workers/iterations**
```naml
// Spawn 3+ workers, process 10+ tasks
// Single iterations hide race conditions
```

**5. Verify final state matches expectations**
```naml
// Always check: actual == expected
println(fmt("Total: {}, Expected: {}", actual, expected));
```

### Before Marking Any Feature Complete
1. Create a complex real-world example in `examples/`
2. The example MUST use spawn if the feature involves shared state
3. The example MUST call user-defined functions with the new types
4. The example MUST verify correctness (not just "it runs")
5. If the complex test fails, DO NOT simplify - fix the root cause
