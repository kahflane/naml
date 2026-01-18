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
namlc/src/
├── main.rs           # CLI entry point
├── lib.rs            # Library root
├── source/           # Source file handling, spans, diagnostics
├── lexer/            # Tokenization
├── ast/              # Abstract syntax tree definitions
├── parser/           # Parsing
├── typechecker/      # Type system
├── jit/              # Cranelift JIT compilation
├── cache/            # Code caching for fast startup
├── codegen/rust/     # Rust code generation
├── runtime/          # Embedded runtime
├── manifest/         # naml.toml parsing
├── bindings/         # Rust crate auto-binding
└── driver/           # Compilation orchestration
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
promise<T>       # Async promise
```

### Function Syntax
```naml
fn name(param: Type) -> ReturnType { }
fn name<T>(param: T) -> T { }
async fn name() -> T { }
fn name() -> T throws Error { }
pub fn (self: Type) method() -> T { }
```

### Control Flow
```naml
if (cond) { } else { }
while (cond) { }
for (i, val: T in collection) { }
switch (val) { case X: ... default: ... }
spawn { }
```
