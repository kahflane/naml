# naml Language - Development Todo List

## Current Status
- [x] Project setup with Cargo workspace
- [x] Source module (file, span, diagnostics)
- [x] Lexer module (cursor, keywords, literals, tokenize)

---

## Phase 1: Foundation (In Progress)

### AST Definitions (`namlc/src/ast/`)
- [ ] Create `ast/mod.rs` - Module root with re-exports
- [ ] Create `ast/types.rs` - NamlType enum with all type variants
  - Primitives: int, uint, float, decimal, bool, string, bytes, unit
  - Composites: array, fixed array, option, map, channel, promise
  - User-defined: struct, enum, interface references
- [ ] Create `ast/literals.rs` - Literal value variants
  - Int, UInt, Float, Decimal, Bool, String, Bytes
  - Array, Map literals
- [ ] Create `ast/operators.rs` - Binary and unary operators
  - Binary: arithmetic, comparison, logical, bitwise
  - Unary: negation, not, reference, dereference
- [ ] Create `ast/expressions.rs` - Expression enum
  - Literal, Identifier, Binary, Unary
  - Call, MethodCall, Index, Field access
  - If expression, Match expression
  - Lambda, Spawn, Channel operations
- [ ] Create `ast/statements.rs` - Statement enum
  - Variable declaration (var, const)
  - Assignment, Return, Throw
  - If, While, For, Switch
  - Expression statement, Block
- [ ] Create `ast/items.rs` - Top-level item definitions
  - Function, Struct, Interface, Enum, Exception
  - Import, Use, Extern declarations
  - Platform attributes
- [ ] Create `ast/visitor.rs` - AST visitor trait for traversal

### Parser (`namlc/src/parser/`)
- [ ] Create `parser/mod.rs` - Parser struct with token stream management
- [ ] Create `parser/types.rs` - Type annotation parsing
- [ ] Create `parser/literals.rs` - Literal expression parsing
- [ ] Create `parser/expressions.rs` - Expression parsing with Pratt algorithm
- [ ] Create `parser/precedence.rs` - Operator precedence table
- [ ] Create `parser/statements.rs` - Statement parsing
- [ ] Create `parser/items.rs` - Top-level item parsing
- [ ] Create `parser/attributes.rs` - Platform attribute parsing (#[platforms(...)])
- [ ] Implement error recovery for better diagnostics

---

## Phase 2: Type System

### Symbol Table (`namlc/src/typechecker/symbols.rs`)
- [ ] Implement scoped variable lookup with shadowing
- [ ] Track function signatures with generics
- [ ] Store type definitions (struct, enum, interface)
- [ ] Module imports tracking

### Type Checker (`namlc/src/typechecker/`)
- [ ] Create `typechecker/mod.rs` - TypeChecker struct
- [ ] Create `typechecker/errors.rs` - TypeError enum
- [ ] Create `typechecker/symbols.rs` - Symbol table
- [ ] Create `typechecker/inference.rs` - Type inference (Hindley-Milner)
- [ ] Create `typechecker/unify.rs` - Type unification
- [ ] Create `typechecker/checker.rs` - Statement validation
- [ ] Create `typechecker/expressions.rs` - Expression type checking
- [ ] Create `typechecker/generics.rs` - Generic type resolution
- [ ] Create `typechecker/platform.rs` - Platform constraint validation

### Error Reporting Enhancements
- [ ] Add source location to all diagnostics
- [ ] Implement multiple error accumulation
- [ ] Add suggestions for common mistakes

---

## Phase 3: Cranelift JIT Engine

### JIT Compiler (`namlc/src/jit/`)
- [ ] Create `jit/mod.rs` - JIT engine entry point
- [ ] Create `jit/compiler.rs` - AST to Cranelift IR generation
- [ ] Create `jit/types.rs` - Type lowering to Cranelift types
- [ ] Create `jit/builtins.rs` - Built-in function implementations
- [ ] Create `jit/runtime.rs` - JIT runtime support

### Code Cache (`namlc/src/cache/`)
- [ ] Create `cache/mod.rs` - Cache manager
- [ ] Create `cache/format.rs` - Cache file format (machine code + metadata)
- [ ] Create `cache/hash.rs` - Content hashing with blake3
- [ ] Implement cache invalidation strategy
- [ ] Add precompilation support (`naml precompile`)

### Runtime (`namlc/src/runtime/`)
- [ ] Create `runtime/mod.rs` - RuntimeRegistry
- [ ] Create `runtime/memory/` - Arena allocator, string interning
- [ ] Create `runtime/native/` - Native Rust implementations
- [ ] Create `runtime/browser/` - Browser JS implementations

---

## Phase 4: Rust Code Generation

### Rust Backend (`namlc/src/codegen/rust/`)
- [ ] Create `codegen/mod.rs` - Target enum, Backend trait
- [ ] Create `codegen/rust/mod.rs` - Rust backend entry
- [ ] Create `codegen/rust/types.rs` - naml -> Rust type conversion
- [ ] Create `codegen/rust/expressions.rs` - Expression generation
- [ ] Create `codegen/rust/statements.rs` - Statement generation
- [ ] Create `codegen/rust/items.rs` - Top-level item generation
- [ ] Create `codegen/rust/platform.rs` - Platform-specific code paths (cfg)

### Multi-Target Support
- [ ] Target::Native - Rust std library, threads
- [ ] Target::Server - WASM with WASI
- [ ] Target::Browser - WASM with wasm-bindgen, async-only

### Build Pipeline
- [ ] Generate Cargo.toml for compiled projects
- [ ] Invoke rustc/cargo for final compilation
- [ ] Add WASM optimization (wasm-opt)

---

## Phase 5: Standard Library

### Core Modules (`naml_stdlib/std/`)
- [ ] Create `io.naml` - I/O operations
- [ ] Create `fmt.naml` - Formatting utilities
- [ ] Create `fs.naml` - File system (native/server)
- [ ] Create `path.naml` - Path manipulation
- [ ] Create `os.naml` - OS interaction
- [ ] Create `http.naml` - HTTP client/server
- [ ] Create `net.naml` - Networking primitives
- [ ] Create `ws.naml` - WebSocket support
- [ ] Create `json.naml` - JSON parsing/serialization
- [ ] Create `collections.naml` - Data structures
- [ ] Create `crypto.naml` - Cryptography
- [ ] Create `rand.naml` - Random number generation
- [ ] Create `time.naml` - Time operations

### Platform-Specific Modules
- [ ] Create `opfs.naml` - Browser Origin Private File System
- [ ] Create `channel.naml` - Go-style channels (native/server only)
- [ ] Create `sync.naml` - Synchronization primitives
- [ ] Create `async.naml` - Async utilities

### FFI System
- [ ] Implement `extern "rust"` for Rust crates
- [ ] Implement `extern "js"` for browser JavaScript

---

## Phase 6: Concurrency

### Spawn Blocks
- [ ] Native: std::thread::spawn implementation
- [ ] Server: tokio task spawning
- [ ] Browser: spawn_local for async tasks

### Channels (`channel.naml`)
- [ ] Bounded channels
- [ ] Unbounded channels
- [ ] Send/receive operations
- [ ] Select statement for multiple channels

### Sync Primitives (`sync.naml`)
- [ ] Mutex implementation
- [ ] RwLock implementation
- [ ] WaitGroup
- [ ] Async sleep/delay

---

## Phase 7: Package Manager & Polish

### Manifest System (`namlc/src/manifest/`)
- [ ] Create `manifest/mod.rs` - Manifest parser entry
- [ ] Create `manifest/config.rs` - Package configuration
- [ ] Create `manifest/dependencies.rs` - Dependency resolution
- [ ] Parse naml.toml files

### CLI Commands
- [ ] `naml init` - Create new project
- [ ] `naml add <pkg>` - Add dependency
- [ ] `naml build` - Compile to binary/WASM
- [ ] `naml run` - Execute directly (JIT)
- [ ] `naml run --cached` - Use cached compilation
- [ ] `naml check` - Type check without building
- [ ] `naml watch` - Hot reload dev server
- [ ] `naml test` - Run tests
- [ ] `naml precompile` - Pre-JIT all files

### Rust Crate Bindings (`namlc/src/bindings/`)
- [ ] Create `bindings/mod.rs` - Binding generator entry
- [ ] Create `bindings/parser.rs` - Parse Rust crate API (using syn)
- [ ] Create `bindings/mapper.rs` - Rust -> naml type mapping
- [ ] Create `bindings/codegen.rs` - Generate binding code

### Driver (`namlc/src/driver/`)
- [ ] Create `driver/mod.rs` - Compilation orchestration
- [ ] Create `driver/session.rs` - Compilation session management

---

## Testing & Examples

### Unit Tests
- [ ] Lexer tests
- [ ] Parser tests (snapshot with insta)
- [ ] Type checker tests
- [ ] JIT compiler tests
- [ ] Codegen tests

### Integration Tests (`tests/`)
- [ ] End-to-end compilation tests
- [ ] Platform-specific tests (native, server, browser)

### Examples (`examples/`)
- [ ] Create `hello.naml` - Hello world
- [ ] Create `http_server.naml` - HTTP server example
- [ ] Create `concurrency.naml` - Spawn and concurrent operations
- [ ] Create `channels.naml` - Channel communication example

### Benchmarks
- [ ] Startup time comparison vs Bun
- [ ] Fibonacci computation benchmark
- [ ] HTTP server throughput
- [ ] JSON parsing performance

---

## Immediate Next Steps

1. **AST Module** - Define all AST node types
2. **Parser** - Build recursive descent parser with Pratt expressions
3. **Basic CLI** - Get `naml run hello.naml` working with print
4. **Type Checker** - Implement core type inference
5. **JIT** - Add Cranelift compilation for basic programs

---

## Notes

- All files must stay under 1000 lines
- Use block comments only (no inline comments)
- Zero-allocation, zero-copy patterns where possible
- Every feature must handle all three platforms (native, server, browser)