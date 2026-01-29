NAML Language - Complete Project Plan

Overview

Create a new programming language called naml (file extension .naml) in a fresh project at /Users/julfikar/Documents/PassionFruit.nosync/naml.

Core Goals

- Performance: Faster than Bun (JavaScript runtime)
- Simplicity: Go-like small grammar
- Cross-platform: Native binary, Server WASM runtime, Browser WASM
- Zero overhead: Zero-allocation, zero-copy, zero-GC compiler/runtime
- Concurrency: Go-like spawn blocks and channels
- Direct execution: naml run file.naml works instantly (like Bun)
- Edge-ready: Sub-millisecond cold start for edge functions
- Easy Rust FFI: Call Rust crates with zero complexity

Execution Modes

1. Dev mode: naml run file.naml - JIT compile with Cranelift for instant feedback
2. Production: naml build - AOT compile to native binary or WASM
3. Edge mode: naml run --edge - Use cached compiled code for <1ms cold start

Target Platforms

1. Native - Self-contained binary with embedded runtime
2. Server WASM - Own runtime that executes WASM (like Bun/Deno)
3. Browser WASM - Runs in browsers via wasm-bindgen

---
Fast Startup Strategy (Edge Functions)

For edge functions, cold start time is critical. Target: <1ms cold start.

Approach: Cranelift JIT with Code Caching

First Run:
Source (.naml) → Parse → Type Check → Cranelift JIT → Execute
↓
Cache compiled code

Subsequent Runs:
Load cached code → Execute (skip parse/compile)
Cold start: ~1ms

Implementation Details

1. Cranelift for JIT (not LLVM)
- LLVM: 100-500ms compile time (too slow for edge)
- Cranelift: 1-10ms compile time (fast enough for dev)
- Output quality: ~80% of LLVM optimization level
2. Code Cache Format
- Store compiled machine code + metadata
- Hash-based cache invalidation (source content hash)
- Location: ~/.naml/cache/ or environment-configured
3. Precompilation Option
   naml precompile src/        # Pre-JIT all files for instant startup
   naml run --precompiled      # Use only cached code
4. Edge Deployment
- Ship precompiled cache with deployment
- Zero compile time at runtime
- Similar to Cloudflare Workers' V8 isolate snapshots

---
Simple Rust FFI (Zero Complexity)

Goal: Call any Rust crate with minimal boilerplate.

Simplest Possible Syntax

// Just import and use - no manual bindings needed
use rust "serde_json" as json;
use rust "reqwest" as http;

fn main() {
var data = json.from_str('{"name": "test"}');
var response = http.get("https://api.example.com").send();
}

How It Works

1. Auto-Binding Generation
   use rust "crate_name" as alias;
   ↓
   At compile time:
- Download crate from crates.io
- Parse public API (using syn)
- Generate naml bindings automatically
- Cache bindings for reuse
2. Type Mapping (Automatic)
   | Rust Type      | naml Type                   |
   |----------------|-----------------------------|
   | String, &str   | string                      |
   | Vec<u8>, &[u8] | bytes                       |
   | i64            | int                         |
   | f64            | float                       |
   | bool           | bool                        |
   | Option<T>      | option<T>                   |
   | Result<T, E>   | returns T, throws exception |
   | HashMap<K,V>   | map<K, V>                   |

3. Error Handling (Automatic)
   // Rust Result<T, E> becomes naml throws
   fn parse(s: string) -> Data throws JsonError {
   return json.from_str(s);  // auto-converts Result to throws
   }
4. Async (Automatic)
   // Rust async fn becomes naml async fn
   async fn fetch(url: string) -> string {
   return await http.get(url).text();
   }

Alternative: Explicit FFI (When Needed)

// For fine-grained control
extern "rust" mod serde_json {
fn from_str(s: string) -> Value throws JsonError;
fn to_string(v: Value) -> string throws JsonError;
}

No Complexity Principles

1. No manual type marshaling - All conversions automatic
2. No memory management - Rust owns the data, naml borrows
3. No unsafe blocks - Safe by default
4. No build.rs - Bindings generated at compile time
5. No extern "C" - Direct Rust interop, not C ABI

---
Project Structure

/Users/julfikar/Documents/PassionFruit.nosync/naml/
├── Cargo.toml                    # Workspace manifest
├── naml.toml                     # Example project manifest
├── CLAUDE.md                     # Project instructions
│
├── namlc/                        # Compiler crate
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs               # CLI entry (~200 lines)
│       ├── lib.rs                # Library root
│       │
│       ├── source/               # Source management
│       │   ├── mod.rs
│       │   ├── file.rs           # Source file handling
│       │   ├── span.rs           # Location tracking
│       │   └── diagnostics.rs    # Error reporting
│       │
│       ├── lexer/                # Tokenization
│       │   ├── mod.rs            # Token enum
│       │   ├── cursor.rs         # Zero-copy cursor
│       │   ├── keywords.rs       # Keyword table
│       │   ├── literals.rs       # Number/string parsing
│       │   └── tokenize.rs       # Main tokenizer
│       │
│       ├── ast/                  # Abstract Syntax Tree
│       │   ├── mod.rs
│       │   ├── types.rs          # NamlType enum
│       │   ├── literals.rs       # Literal variants
│       │   ├── operators.rs      # Binary/unary ops
│       │   ├── expressions.rs    # Expression enum
│       │   ├── statements.rs     # Statement enum
│       │   ├── items.rs          # Top-level items
│       │   └── visitor.rs        # AST visitor trait
│       │
│       ├── parser/               # Parsing
│       │   ├── mod.rs            # Parser struct
│       │   ├── types.rs          # Type parsing
│       │   ├── literals.rs       # Literal parsing
│       │   ├── expressions.rs    # Expression parsing
│       │   ├── precedence.rs     # Operator precedence
│       │   ├── statements.rs     # Statement parsing
│       │   ├── items.rs          # Top-level parsing
│       │   └── attributes.rs     # Attribute parsing
│       │
│       ├── typechecker/          # Type system
│       │   ├── mod.rs            # TypeChecker struct
│       │   ├── errors.rs         # TypeError enum
│       │   ├── symbols.rs        # Symbol table
│       │   ├── inference.rs      # Type inference
│       │   ├── unify.rs          # Type unification
│       │   ├── checker.rs        # Statement checking
│       │   ├── expressions.rs    # Expression checking
│       │   ├── generics.rs       # Generic resolution
│       │   └── platform.rs       # Platform constraints
│       │
│       ├── jit/                  # Cranelift JIT (primary execution)
│       │   ├── mod.rs            # JIT engine entry point
│       │   ├── compiler.rs       # AST -> Cranelift IR
│       │   ├── types.rs          # Type lowering to Cranelift
│       │   ├── builtins.rs       # Built-in function implementations
│       │   └── runtime.rs        # JIT runtime support
│       │
│       ├── cache/                # Code cache for fast startup
│       │   ├── mod.rs            # Cache manager
│       │   ├── format.rs         # Cache file format
│       │   └── hash.rs           # Content hashing
│       │
│       ├── codegen/              # Code generation (naml build)
│       │   ├── mod.rs            # Target enum, Backend trait
│       │   ├── rust/             # Rust backend (AOT)
│       │   │   ├── mod.rs
│       │   │   ├── types.rs
│       │   │   ├── expressions.rs
│       │   │   ├── statements.rs
│       │   │   ├── items.rs
│       │   │   └── platform.rs
│       │   └── wasm/             # Direct WASM generation (future)
│       │
│       ├── runtime/              # Embedded runtime
│       │   ├── mod.rs            # RuntimeRegistry
│       │   ├── memory/           # Arena allocator, string interning
│       │   ├── native/           # Native Rust implementations
│       │   └── browser/          # JS implementations
│       │
│       ├── manifest/             # naml.toml parsing
│       │   ├── mod.rs
│       │   ├── config.rs
│       │   └── dependencies.rs
│       │
│       ├── bindings/             # Rust crate auto-binding generation
│       │   ├── mod.rs            # Binding generator entry
│       │   ├── parser.rs         # Parse Rust crate API (using syn)
│       │   ├── mapper.rs         # Rust -> naml type mapping
│       │   └── codegen.rs        # Generate binding code
│       │
│       └── driver/               # Compilation driver
│           ├── mod.rs
│           └── session.rs
│
├── naml_stdlib/                  # Standard library
│   ├── std/
│   │   ├── io.naml
│   │   ├── fmt.naml
│   │   ├── fs.naml
│   │   ├── path.naml
│   │   ├── os.naml
│   │   ├── http.naml
│   │   ├── net.naml
│   │   ├── ws.naml
│   │   ├── json.naml
│   │   ├── collections.naml
│   │   ├── crypto.naml
│   │   ├── rand.naml
│   │   ├── time.naml
│   │   ├── async.naml
│   │   ├── sync.naml
│   │   ├── channel.naml          # NEW: Go-style channels
│   │   └── opfs.naml             # NEW: Browser storage
│   ├── errors/
│   └── js/
│
├── examples/
│   ├── hello.naml
│   ├── http_server.naml
│   ├── concurrency.naml
│   └── channels.naml
│
└── tests/

---
Implementation Phases

Phase 1: Foundation (Week 1-2)

Goal: Basic compiler infrastructure - parse and type-check naml syntax

1. Project Setup
- Create /Users/julfikar/Documents/PassionFruit.nosync/naml/ directory
- Initialize Cargo workspace with namlc crate
- Create CLAUDE.md with project rules (1000 line limit, block comments only)
- Set up directory structure
2. Lexer (namlc/src/lexer/)
- Zero-copy tokenization with cursor
- All keywords and operators
- String interning for identifiers (using lasso crate)
- Literal parsing (int, uint, float, decimal, string, bool, date, time)
3. AST Definitions (namlc/src/ast/)
- NamlType enum with all 20+ type variants
- Expression enum (literals, binary/unary ops, calls, access)
- Statement enum (var, const, if, while, for, switch, etc.)
- Item enum (fn, struct, interface, enum, exception, extern)
4. Parser (namlc/src/parser/)
- Recursive descent with Pratt parsing for expressions
- All statement types with semicolon handling
- Function definitions with receivers and generics
- Struct, interface, enum, exception definitions
- Platform attributes (#[platforms(...)])
- Error recovery for better diagnostics

Phase 2: Type System (Week 3-4)

Goal: Full type checking with generics and platform constraints

1. Symbol Table (namlc/src/typechecker/symbols.rs)
- Scoped variable lookup with shadowing
- Function signatures with generics
- Type definitions (struct, enum, interface)
- Module imports tracking
2. Type Checker (namlc/src/typechecker/)
- Expression type inference (Hindley-Milner style)
- Statement validation
- Generic type parameters with bounds
- Interface satisfaction checking
- Platform constraint validation at compile time
3. Error Reporting (namlc/src/source/diagnostics.rs)
- Clear error messages with source locations
- Multiple error accumulation (don't fail on first error)
- Suggestions for common mistakes

Phase 3: Cranelift JIT Engine (Week 5-6)

Goal: naml run file.naml executes with <10ms startup (first run), <1ms (cached)

1. Cranelift JIT Compiler (namlc/src/jit/)
- AST → Cranelift IR generation
- Direct native code emission (no interpreter needed)
- Full language support from day 1
- Why Cranelift: Fast compile (1-10ms), decent output, pure Rust
2. Code Cache (namlc/src/cache/)
- Cache compiled native code to disk
- Hash-based invalidation (source content hash)
- First run: compile + cache
- Subsequent runs: load from cache (~1ms)
3. Runtime (namlc/src/runtime/)
- Memory arena for allocations
- String interning
- Built-in type implementations
- FFI bridge for Rust crate calls

Phase 4: Rust Code Generation (Week 7-8)

Goal: naml build produces optimized native binary or WASM

1. Rust Backend (namlc/src/codegen/rust/)
- Type conversion (naml -> Rust)
- Expression generation
- Statement generation
- Platform-specific code paths (cfg attributes)
2. Multi-Target Support
- Target::Native - Rust std library, threads
- Target::Server - WASM with WASI for server-side
- Target::Browser - WASM with wasm-bindgen, async-only
3. Build Pipeline
- Generate Cargo.toml for compiled project
- Invoke rustc/cargo for final compilation
- WASM optimization (wasm-opt)

Phase 5: Standard Library (Week 9-10)

Goal: Core stdlib modules working on all platforms

1. Core Modules (naml_stdlib/std/)
- io.naml, fmt.naml - I/O and formatting
- fs.naml, path.naml - File system (native/server)
- http.naml, net.naml - Networking
- json.naml - JSON parsing
- collections.naml - Data structures
- crypto.naml, rand.naml - Crypto and random
- time.naml - Time operations
2. Platform-Specific Modules
- opfs.naml - Browser Origin Private File System
- Full fs/net for native and server
3. FFI System
- extern "rust" for Rust crates
- extern "js" for browser JavaScript

Phase 6: Concurrency (Week 11-12)

Goal: Go-like concurrency model

1. Spawn Blocks
- Native/Server: std::thread::spawn or tokio tasks
- Browser: spawn_local for async tasks
2. Channels (channel.naml)
- Bounded and unbounded channels
- Send/receive operations
- Select statement for multiple channels
3. Sync Primitives (sync.naml)
- Mutex, RwLock
- WaitGroup
- Async sleep/delay

Phase 7: Package Manager & Polish (Week 13-14)

Goal: Complete developer experience

1. Manifest System (naml.toml)
- Package metadata
- Dependency specification
- Build configuration per target
2. CLI Commands
- naml init - Create new project
- naml add <pkg> - Add dependency
- naml build - Compile to binary/WASM
- naml run - Execute directly (interpreter/JIT)
- naml check - Type check without building
- naml watch - Hot reload dev server
- naml test - Run tests
3. Rust Crate Bindings (Future enhancement)
- Auto-generate naml bindings from Rust crates
- Type mapping and error conversion

---
Syntax Reference

Variables and Constants

var x: int = 42;              // Mutable
const PI: float = 3.14159;    // Constant (immutable)
pub const MAX: int = 100;     // Public constant

Primitive Types
┌──────────────┬──────────────────────────────┐
│     Type     │         Description          │
├──────────────┼──────────────────────────────┤
│ int          │ 64-bit signed integer        │
├──────────────┼──────────────────────────────┤
│ uint         │ 64-bit unsigned integer      │
├──────────────┼──────────────────────────────┤
│ float        │ 64-bit floating point        │
├──────────────┼──────────────────────────────┤
│ decimal(p,s) │ Decimal with precision/scale │
├──────────────┼──────────────────────────────┤
│ bool         │ Boolean                      │
├──────────────┼──────────────────────────────┤
│ string       │ String                       │
├──────────────┼──────────────────────────────┤
│ bytes        │ Binary data                  │
├──────────────┼──────────────────────────────┤
│ ()           │ Unit type                    │
└──────────────┴──────────────────────────────┘
Composite Types

[int; 5]              // Fixed-size array
[string]              // Dynamic array
option<T>             // Optional value
map<K, V>             // Key-value map
channel<T>            // Channel (native/node)
promise<T>            // Async promise

Functions

fn add(a: int, b: int) -> int {
return a + b;
}

fn identity<T>(value: T) -> T {
return value;
}

async fn fetch(url: string) -> string {
return await http_get(url);
}

fn divide(a: int, b: int) -> int throws MathError {
if (b == 0) {
throw MathError.DivisionByZero;
}
return a / b;
}

Structs and Methods

pub struct rectangle implements shape {
pub width: float,
pub height: float
}

pub fn (self: rectangle) area() -> float {
return self.width * self.height;
}

fn (mut self: point) move_by(dx: float, dy: float) {
self.x = self.x + dx;
self.y = self.y + dy;
}

Interfaces

interface shape {
fn area() -> float;
fn perimeter() -> float;
}

Enums and Exceptions

enum Color { Red, Green, Blue }

exception IOError {
FileNotFound(path: string),
PermissionDenied(path: string),
}

Exception Handling

fn process(path: string) -> Data throws IOError {
var file = try open(path);
return try parse(file);
}

var result = divide(a, b) catch e {
if (e is MathError.DivisionByZero) {
return 0;
}
};

Control Flow

if (x > 5) { } else if (x == 5) { } else { }

while (count < 5) { count = count + 1; }

for (i, val: int in range(0, 5)) { print(val); }

switch (x) {
case 1: print("one");
case 2, 3: print("two or three");
default: print("other");
}

Concurrency

spawn {
print("Running concurrently");
}

var ch: channel<int> = channel();
spawn { ch.send(42); }
var val = ch.recv();

Platform Attributes

#[platforms(all)]
pub fn fetch(url: string) -> response;

#[platforms(native, server)]
pub fn read_file(path: string) -> string throws io_error;

#[platforms(browser)]
extern "js" {
fn __http_fetch(method: string, url: string) -> response;
}

Imports

import std.fs;
import std.http;

var content = std.fs.read_file("data.txt");

Rust Crate Integration (Simple FFI)

// Auto-binding - just use it!
use rust "serde_json" as json;
use rust "reqwest" as http;

fn main() {
var data = json.from_str('{"name": "test"}');
var response = http.get("https://api.example.com").send();
}

---
Code Rules (CLAUDE.md)

1. File Size: All files must stay under 1000 lines
2. Comments: Block comments only at file top, no inline comments
3. Performance: Zero-allocation, zero-copy patterns
4. Platform: Always handle all three targets (native, server, browser)
5. Testing: Tests for all platforms per feature

---
CLI Commands

naml run file.naml              # Execute directly (interpreter/JIT) - FAST
naml build                      # Build for default target (native)
naml build --target native      # Build native binary
naml build --target server      # Build server WASM (own runtime)
naml build --target browser     # Build browser WASM
naml check                      # Type check without building
naml watch                      # Hot reload dev server
naml init                       # Create new project
naml add <package>              # Add dependency
naml test                       # Run tests

---
Decisions Made

1. Start fresh - Clean slate, no code from existing nam project
2. Own runtime - naml has its own execution runtime (not dependent on Node.js)
3. Cranelift JIT - Primary execution via Cranelift (not tree-walking interpreter)
4. Code caching - Cache compiled code for <1ms cold start (edge functions)
5. All platforms from start - Native, Server WASM, Browser WASM
6. Simple Rust FFI - use rust "crate" syntax with auto-binding generation
7. Zero FFI complexity - Automatic type mapping, no manual marshaling

Open Questions

1. Package registry: Build custom registry or integrate with existing (crates.io-style)?
2. Server WASM runtime: Use wasmtime internally or build custom WASM executor?
3. Hot reload strategy: File watching + incremental recompilation approach?

---
Immediate Next Steps

Step 1: Project Initialization

mkdir -p /Users/julfikar/Documents/PassionFruit.nosync/naml
cd /Users/julfikar/Documents/PassionFruit.nosync/naml
cargo init --name namlc namlc

Step 2: Create CLAUDE.md

- File size limit: 1000 lines max
- Block comments only (no inline)
- Platform matrix for all features
- Performance requirements

Step 3: Implement Lexer

- Token enum with all keywords/operators
- Zero-copy cursor over source text
- String interning with lasso crate
- Comprehensive literal parsing

Step 4: Define AST

- NamlType enum (all type variants)
- Expression enum
- Statement enum
- Item enum (top-level definitions)

Step 5: Build Parser

- Recursive descent + Pratt parsing
- Error recovery
- Full syntax support

Step 6: Implement Type Checker

- Symbol table with scopes
- Type inference
- Generic support
- Platform validation

Step 7: Build Interpreter

- Tree-walking execution
- Built-in functions
- Basic I/O
- naml run command working

Step 8: Add Rust Codegen

- AST -> Rust source
- Multi-target support
- naml build command working

---
Verification Plan

1. Unit Tests: Each module has tests (#[cfg(test)])
2. Integration Tests: End-to-end in tests/ directory
3. Platform Tests: Verify features work on native, server, browser
4. Benchmarks: Compare against Bun for:
- Startup time
- Fibonacci computation
- HTTP server throughput
- JSON parsing
5. Example Programs: All examples compile and run correctly