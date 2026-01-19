# naml Architecture

## Vision
**naml = Go's Simplicity + Rust's Performance + JavaScript's Reach**

## Core Principles
1. **Faster than Bun** - Compile to native code via Rust
2. **Simple grammar** - Go-like syntax, easy to read/write
3. **Universal** - Runs on any OS, CPU, browser
4. **Rust interop** - Use Rust libraries directly
5. **Go-like concurrency** - spawn, channels
6. **Zero-GC** - Rust handles memory

## Compilation Pipeline

```
┌─────────────────┐
│  source.naml    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│     Lexer       │  Zero-copy tokenization, SIMD optimized
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│     Parser      │  Recursive descent + Pratt parsing
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│      AST        │  Typed syntax tree, arena allocated
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Type Checker   │  Hindley-Milner inference
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Rust Codegen   │  AST → Rust source code
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  cargo build    │  Rust compiler + LLVM
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Binary/WASM    │  Native executable or WebAssembly
└─────────────────┘
```

## Target Platforms

| Target | Command | Output | Runtime |
|--------|---------|--------|---------|
| Native | `naml build` | Binary | None |
| Native (run) | `naml run` | Execute | None |
| Server WASM | `naml build --target server` | .wasm | Wasmtime/Node |
| Browser WASM | `naml build --target browser` | .wasm | Browser |
| Watch mode | `naml watch` | Hot reload | Wasmtime |

## Directory Structure

```
namlc/src/
├── main.rs           # CLI entry point
├── lib.rs            # Library exports
├── source/           # Source file handling
├── lexer/            # Tokenization
├── ast/              # AST definitions
├── parser/           # Parsing
├── typechecker/      # Type system
├── codegen/          # Rust code generation
│   ├── mod.rs        # Orchestration
│   └── rust/         # Rust-specific codegen
│       ├── mod.rs
│       ├── prelude.rs
│       ├── types.rs
│       ├── expressions.rs
│       └── statements.rs
├── runner/           # Watch mode + Wasmtime
└── package/          # Package manager
```

## Build Output

```
.naml_build/
├── Cargo.toml        # Generated manifest
├── src/
│   └── main.rs       # Generated Rust code
└── target/
    └── release/
        └── program   # Final executable
```
