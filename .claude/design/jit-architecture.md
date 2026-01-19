# JIT Compiler Architecture Design

## Overview

This document describes the design decisions for implementing a complete JIT compiler for the naml programming language using Cranelift.

## Why Cranelift (Not Transpilation)

The old `nam` project used **transpilation to Rust** followed by cargo compilation. We chose **Cranelift JIT** instead because:

| Aspect | Transpilation (old nam) | Cranelift JIT (new naml) |
|--------|-------------------------|--------------------------|
| Startup time | Slow (full rustc) | Fast (JIT compile) |
| Development cycle | Edit → Compile → Run | Edit → Run |
| Debug experience | Rust debugger | Native debugging |
| WASM support | Via wasm-pack | Direct Cranelift WASM backend |
| Memory control | Rust ownership | Direct memory management |
| Code cache | Binary on disk | Native code in memory |

## Architecture Layers

```
┌─────────────────────────────────────────────────────┐
│                  naml Source Code                   │
└─────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────┐
│                     Lexer                           │
│  - Zero-copy tokenization                           │
│  - String interning (lasso)                         │
└─────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────┐
│                     Parser                          │
│  - Recursive descent + Pratt parsing                │
│  - Arena allocation for AST nodes                   │
└─────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────┐
│                  Type Checker                       │
│  - Hindley-Milner style inference                   │
│  - Symbol table construction                        │
│  - Generic type resolution                          │
└─────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────┐
│                 JIT Compiler                        │
│  ┌───────────────────────────────────────────────┐  │
│  │              JitContext                       │  │
│  │  - Cranelift JITModule                        │  │
│  │  - Function registry                          │  │
│  │  - RuntimeContext (memory)                    │  │
│  │  - BuiltinRegistry                            │  │
│  └───────────────────────────────────────────────┘  │
│                        │                            │
│                        ▼                            │
│  ┌───────────────────────────────────────────────┐  │
│  │           FunctionCompiler                    │  │
│  │  - AST → Cranelift IR translation             │  │
│  │  - Variable management (SSA)                  │  │
│  │  - Control flow graph construction            │  │
│  │  - Type-aware code generation                 │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────┐
│               Cranelift Backend                     │
│  - Native ISA (x86-64, aarch64)                    │
│  - Register allocation                              │
│  - Instruction selection                            │
│  - Code emission                                    │
└─────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────┐
│                Native Code                          │
│  - Directly executable machine code                 │
│  - Callable via function pointer                    │
└─────────────────────────────────────────────────────┘
```

## Memory Model

### String Representation
```
┌──────────┬────────────────────────────┐
│ len: u64 │ data: [u8; len]            │
└──────────┴────────────────────────────┘
   8 bytes        len bytes

Pointer points to start (len field)
```

### Array Representation
```
┌──────────┬────────────────────────────┐
│ len: u64 │ elements: [T; len]         │
└──────────┴────────────────────────────┘
   8 bytes     len * sizeof(T) bytes
```

### Struct Representation
```
┌─────────┬─────────┬─────────┬───┐
│ field0  │ field1  │ field2  │...│
└─────────┴─────────┴─────────┴───┘
  Aligned to max field alignment
```

### Enum Representation
```
┌─────────┬──────────────────────────────┐
│ tag: u8 │ payload: [u8; max_variant]   │
└─────────┴──────────────────────────────┘
  1 byte    Size of largest variant
```

### Option<T> Special Case
```
None:  tag = 0, payload unused
Some:  tag = 1, payload = T value
```

## Type Mapping (naml → Cranelift)

| naml Type | Cranelift Type | Notes |
|-----------|---------------|-------|
| int | I64 | Signed 64-bit |
| uint | I64 | Same repr, different semantics |
| float | F64 | IEEE 754 |
| bool | I8 | 0 = false, 1 = true |
| string | Pointer | Points to len-prefixed data |
| [T] | Pointer | Points to len-prefixed array |
| option<T> | Struct | Tag + payload |
| map<K,V> | Pointer | Points to hash table |
| struct | Pointer or Inline | Small structs inline |
| enum | Struct | Tag + max payload |
| fn(...) → T | Pointer | Function pointer |
| promise<T> | Pointer | Future/continuation |

## Compilation Phases

### Phase 1: Declaration Pass
1. Iterate all items in SourceFile
2. For each function:
   - Create Cranelift signature from naml types
   - Declare function in JITModule
   - Store FuncId in registry

### Phase 2: Definition Pass
1. For each function with body:
   - Create FunctionBuilder
   - Create entry block with params
   - Compile function body
   - Finalize and define in module

### Phase 3: Finalization
1. Call `module.finalize_definitions()`
2. All functions now have native code

### Phase 4: Execution
1. Lookup `main` function
2. Get function pointer via `get_finalized_function`
3. Transmute to `fn() -> T` and call

## Function Compilation Strategy

### Variables
- Each naml variable → Cranelift Variable
- Variables are SSA values (single assignment per block)
- `def_var(var, value)` creates new SSA version
- `use_var(var)` gets current SSA value

### Control Flow
- Each branch point creates new blocks
- `brif(cond, then_block, else_block)` for conditionals
- `jump(target)` for unconditional branches
- Blocks must be sealed after all predecessors known

### Loops
```
header_block:
    compute condition
    brif(cond, body_block, exit_block)

body_block:
    body statements
    jump(header_block)

exit_block:
    continue...
```

### Function Calls
1. Compile arguments left-to-right
2. Declare callee in current function
3. Generate `call` instruction
4. Handle return value

## Built-in Functions

Built-ins are Rust functions exported with C ABI:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn naml_println_int(value: i64) {
    println!("{}", value);
}
```

Registered with JITBuilder:
```rust
builder.symbol("naml_println_int", naml_println_int as *const u8);
```

Declared and called like any other function.

## Runtime Support

### Memory Allocation
- Simple bump allocator during execution
- All allocations tracked
- Freed when JitContext drops

### String Interning
- Compile-time strings interned in pool
- Runtime strings allocated fresh

### No Garbage Collection
- Deterministic memory (RAII-style)
- Memory freed at scope exit (future)
- Currently: all freed at program end

## Generics Implementation

**Strategy: Monomorphization**

1. During type checking, collect all concrete instantiations
2. For each instantiation, generate specialized code
3. No runtime type parameters

Example:
```naml
fn identity<T>(x: T) -> T { return x; }

identity(42);        // generates identity_int
identity("hello");   // generates identity_string
```

## Async/Await Implementation

**Strategy: Stackful Coroutines** (simpler than state machines)

1. Each async function gets its own stack
2. `await` saves state and yields to executor
3. Executor manages ready queue
4. Promise<T> wraps coroutine handle

Alternative: CPS transformation (more complex).

## Error Handling (Exceptions)

**Strategy: Return-based with unwinding**

1. Functions that `throws` return Result<T, E>
2. `throw` creates error and returns early
3. No try-catch initially (caller must handle)
4. Future: setjmp/longjmp for true unwinding

## Debug Information

- Cranelift supports DWARF generation
- Future: emit debug info for source-level debugging
- Currently: no debug info

## Testing Strategy

1. **Unit tests** per compilation feature
2. **Integration tests** with small programs
3. **Comprehensive test** with test_parse.rs code
4. **Performance tests** comparing to interpreted baseline

## Future Optimizations

1. **Code caching**: Persist compiled code to disk
2. **Lazy compilation**: Compile functions on first call
3. **Inlining**: Inline small functions
4. **Escape analysis**: Stack-allocate non-escaping objects
5. **SIMD**: Vectorize array operations
