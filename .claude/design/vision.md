Vision

naml = Go's Simplicity + Rust's Performance + JavaScript's Reach

A scripting language that is:
- Faster than Bun (JS)
- Simple grammar like Go
- Runs everywhere (any OS, CPU, browser)
- Can use Rust libraries directly
- Has Go-like concurrency (goroutines, channels)
- Zero-allocation, zero-copy, zero-GC

Architecture: Transpilation to Rust

naml source → Lexer → Parser → AST → Type Checker → Rust Codegen → cargo build → binary/WASM

Single backend - transpile to Rust for everything:
- Full Rust library access
- Maximum performance (Rust compiler optimizations)
- Zero-GC (Rust handles memory)
- Universal targets (native, WASM, browser)

Target Platforms                                                                                                                                                                                       
┌──────────────┬─────────────────────────────┬────────────────────────────┐                                                                                                                            
│   Platform   │           Command           │           Output           │                                                                                                                            
├──────────────┼─────────────────────────────┼────────────────────────────┤                                                                                                                            
│ Native       │ naml build                  │ Binary executable          │                                                                                                                            
├──────────────┼─────────────────────────────┼────────────────────────────┤                                                                                                                            
│ Native (run) │ naml run                    │ Build + execute            │                                                                                                                            
├──────────────┼─────────────────────────────┼────────────────────────────┤                                                                                                                            
│ Server WASM  │ naml build --target server  │ WASM + WASI                │                                                                                                                            
├──────────────┼─────────────────────────────┼────────────────────────────┤                                                                                                                            
│ Browser WASM │ naml build --target browser │ WASM + wasm-bindgen        │                                                                                                                            
├──────────────┼─────────────────────────────┼────────────────────────────┤                                                                                                                            
│ Watch mode   │ naml watch                  │ WASM + Wasmtime hot reload │                                                                                                                            
└──────────────┴─────────────────────────────┴────────────────────────────┘                                                                                                                            
Current State

What Exists ✅

- Lexer with SIMD optimization
- Parser with all language constructs
- Type checker with inference
- AST for full language

To Remove ❌

- namlc/src/jit/ - Broken Cranelift JIT (delete entire directory)

To Build 🔨

- namlc/src/codegen/ - Rust code generator

 ---                                                                                                                                                                                                    
Implementation Plan

Phase 1: Setup & Cleanup

Goal: Clean slate for Rust codegen

Tasks:
1. Delete namlc/src/jit/ directory (broken, not needed)
2. Remove JIT references from lib.rs and main.rs
3. Create namlc/src/codegen/ directory structure
4. Update CLI to prepare for new commands

Phase 2: Basic Rust Codegen

Goal: naml run hello.naml transpiles to Rust and executes

Files to create:                                                                                                                                                                                       
namlc/src/codegen/                                                                                                                                                                                     
├── mod.rs              # Orchestration, CodeGenerator struct                                                                                                                                          
├── rust/                                                                                                                                                                                              
│   ├── mod.rs          # Rust-specific codegen entry point                                                                                                                                            
│   ├── prelude.rs      # Runtime prelude (print, etc.)                                                                                                                                                
│   ├── types.rs        # naml type → Rust type mapping                                                                                                                                                
│   ├── expressions.rs  # Expression codegen                                                                                                                                                           
│   └── statements.rs   # Statement codegen

Tasks:
1. Implement CodeGenerator::generate() → Rust source string
2. Generate fn main() wrapper
3. Generate variable declarations (let mut)
4. Generate expressions (literals, binary ops, calls)
5. Generate control flow (if, while, for)
6. Generate print/println as println! macro
7. Write to .naml_build/src/main.rs
8. Generate Cargo.toml
9. Run cargo build --release
10. Execute resulting binary

Phase 3: Full Language Support

Goal: All naml constructs transpile correctly

Tasks:
1. Structs → Rust structs
2. Enums → Rust enums
3. Interfaces → Rust traits
4. Methods → impl blocks
5. Generics → Rust generics
6. Arrays → Vec
7. Maps → HashMap<K, V>
8. Option → Option
9. Lambdas → closures
10. Async/await → async/await
11. Spawn → tokio::spawn
12. Channels → tokio::sync::mpsc
13. Exceptions → Result<T, E>

Phase 4: Rust Library Integration

Goal: Use Rust crates from naml

Syntax:                                                                                                                                                                                                
use rust::serde_json;                                                                                                                                                                                  
use rust::reqwest;

Tasks:
1. Parse use rust::* imports
2. Add dependencies to generated Cargo.toml
3. Generate proper Rust use statements

Phase 5: WASM Targets

Goal: naml build --target browser and --target server

Tasks:
1. Browser: wasm-bindgen + wasm-pack
2. Server: WASI target
3. Platform-specific code via #[platforms(...)]

Phase 6: Watch Mode

Goal: naml watch with hot reload

Tasks:
1. File watcher (notify crate)
2. Compile to WASM
3. Execute via Wasmtime
4. Fast reload on change

Phase 7: Package Manager

Goal: naml add, naml publish
                                                                                                                                                                                                        
---                                                                                                                                                                                                    
File Structure (Target)

namlc/src/                                                                                                                                                                                             
├── main.rs           # CLI                                                                                                                                                                            
├── lib.rs            # Library root                                                                                                                                                                   
├── lexer/            # ✅ Done                                                                                                                                                                        
├── parser/           # ✅ Done                                                                                                                                                                        
├── ast/              # ✅ Done                                                                                                                                                                        
├── typechecker/      # ✅ Done                                                                                                                                                                        
├── codegen/          # 🔨 To build                                                                                                                                                                    
│   ├── mod.rs                                                                                                                                                                                         
│   └── rust/                                                                                                                                                                                          
│       ├── mod.rs                                                                                                                                                                                     
│       ├── prelude.rs                                                                                                                                                                                 
│       ├── types.rs                                                                                                                                                                                   
│       ├── expressions.rs                                                                                                                                                                             
│       └── statements.rs                                                                                                                                                                              
├── runner/           # Watch mode + Wasmtime                                                                                                                                                          
└── package/          # Package manager

Build output:                                                                                                                                                                                          
.naml_build/                                                                                                                                                                                           
├── Cargo.toml        # Generated                                                                                                                                                                      
├── src/                                                                                                                                                                                               
│   └── main.rs       # Generated Rust code                                                                                                                                                            
└── target/                                                                                                                                                                                            
└── release/                                                                                                                                                                                       
└── program   # Final binary
                                                                                                                                                                                                        
---                                                                                                                                                                                                    
Verification

Phase 2 Complete When:

$ cargo run -- run examples/hello.naml                                                                                                                                                                 
Hello, World!                                                                                                                                                                                          
Result: 42                                                                                                                                                                                             
The answer!                                                                                                                                                                                            
i = 0                                                                                                                                                                                                  
i = 1                                                                                                                                                                                                  
i = 2

Phase 3 Complete When:

The comprehensive code in namlc/examples/test_parse.rs compiles and runs.

Full Success When:

$ naml run program.naml          # Native execution                                                                                                                                                    
$ naml build --target browser    # Browser WASM                                                                                                                                                        
$ naml build --target server     # Server WASM                                                                                                                                                         
$ naml watch                     # Hot reload dev mode
                                                                                                                                                                                                        
---                                                                                                                                                                                                    
Type Mappings (naml → Rust)                                                                                                                                                                            
┌────────────┬─────────────────────────────────┐                                                                                                                                                       
│    naml    │              Rust               │                                                                                                                                                       
├────────────┼─────────────────────────────────┤                                                                                                                                                       
│ int        │ i64                             │                                                                                                                                                       
├────────────┼─────────────────────────────────┤                                                                                                                                                       
│ uint       │ u64                             │                                                                                                                                                       
├────────────┼─────────────────────────────────┤                                                                                                                                                       
│ float      │ f64                             │                                                                                                                                                       
├────────────┼─────────────────────────────────┤                                                                                                                                                       
│ bool       │ bool                            │                                                                                                                                                       
├────────────┼─────────────────────────────────┤                                                                                                                                                       
│ string     │ String                          │                                                                                                                                                       
├────────────┼─────────────────────────────────┤                                                                                                                                                       
│ bytes      │ Vec<u8>                         │                                                                                                                                                       
├────────────┼─────────────────────────────────┤                                                                                                                                                       
│ [T]        │ Vec<T>                          │                                                                                                                                                       
├────────────┼─────────────────────────────────┤                                                                                                                                                       
│ [T; N]     │ [T; N]                          │                                                                                                                                                       
├────────────┼─────────────────────────────────┤                                                                                                                                                       
│ option<T>  │ Option<T>                       │                                                                                                                                                       
├────────────┼─────────────────────────────────┤                                                                                                                                                       
│ map<K, V>  │ std::collections::HashMap<K, V> │                                                                                                                                                       
├────────────┼─────────────────────────────────┤                                                                                                                                                       
│ channel<T> │ tokio::sync::mpsc::Sender<T>    │                                                                                                                                                       
└────────────┴─────────────────────────────────┘
 ---                                                                                                                                                                                                    
Concurrency Model (Go-like)

naml:                                                                                                                                                                                                  
spawn { ... }                    // Goroutine                                                                                                                                                          
var ch = channel<int>(10);       // Buffered channel                                                                                                                                                   
ch.send(value);                  // Send                                                                                                                                                               
var x = ch.receive();            // Receive

Generated Rust:                                                                                                                                                                                        
tokio::spawn(async { ... });                                                                                                                                                                           
let (tx, rx) = tokio::sync::mpsc::channel(10);                                                                                                                                                         
tx.send(value).await;                                                                                                                                                                                  
let x = rx.recv().await;
                                                                                                                                                                                                        
---                                                                                                                                                                                                    
Reference: Old nam Codegen

The old nam project at /Users/julfikar/Documents/PassionFruit.nosync/nam/namc/src/codegen/ has:
- mod.rs (449 lines) - Main code generator
- statements.rs (1039 lines) - Statement codegen
- expressions.rs (397 lines) - Expression codegen
- types.rs (172 lines) - Type conversion

We can reference this for patterns but build fresh.
                                                                                                                                                                                                        
---                                                                                                                                                                                                    
Notes

- Transpilation gives us full Rust ecosystem access
- Memory management handled by Rust (ownership + borrowing)
- Async runtime: tokio for concurrency
- WASM: wasm-bindgen for browser, WASI for server
- Focus: correctness first, then optimize compilation speed 