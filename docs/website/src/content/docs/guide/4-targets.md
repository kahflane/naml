---
title: Compilation Targets
description: Building naml for different platforms
---

## Overview

naml supports three compilation targets from a single codebase:

1. **Native**: Self-contained binaries for desktop and servers
2. **Server WASM**: WebAssembly with own runtime (like Bun/Deno)
3. **Browser WASM**: WebAssembly for web browsers

## Native Target

The default target. Produces standalone executables for your platform.

### Development (JIT)

For fast iteration, use Cranelift JIT compilation:

```bash
naml run main.nm              # Instant execution
naml run --cached main.nm     # Sub-millisecond cold start
```

The JIT compiler provides near-instant startup, making the edit-run cycle extremely fast.

### Production (Optimized Binary)

For deployment, compile to native code via rustc:

```bash
naml build
./target/release/main
```

This produces a highly optimized, self-contained binary with the naml runtime embedded.

**Native features:**
- Full file system access (`std::fs`)
- Network sockets (`std::net`)
- Multi-threading with OS threads
- All standard library modules
- System calls and FFI

## Server WASM Target

Compile to WebAssembly for server environments. The runtime executes WASM modules similar to Bun or Deno.

```bash
naml build --target server
```

**Server WASM features:**
- WASI file system access
- WASI socket support
- Single-threaded execution model
- Sandboxed environment
- Portable across platforms

**Use cases:**
- Edge computing (Cloudflare Workers, Fastly Compute)
- Serverless functions
- Microservices
- Plugin systems

## Browser WASM Target

Compile to WebAssembly for web browsers via wasm-bindgen.

```bash
naml build --target browser
```

**Browser WASM features:**
- Origin Private File System (OPFS)
- Fetch API for networking
- Async-only execution
- WebCrypto API
- Access to DOM (via bindings)

**Use cases:**
- Web applications
- Interactive documentation
- Client-side tools
- Progressive web apps

## Platform Feature Matrix

Different features are available on different platforms:

| Feature | Native | Server WASM | Browser WASM |
|---------|--------|-------------|--------------|
| **File I/O** | std::fs | WASI FS | OPFS |
| **Networking** | std::net | WASI sockets | fetch API |
| **Concurrency** | OS threads | Single-threaded | async only |
| **Crypto** | ring/rustcrypto | WASI crypto | WebCrypto |
| **Random** | OS entropy | WASI random | crypto.random |
| **Time** | std::time | WASI clock | performance.now |
| **Console I/O** | Terminal | stdout/stderr | console.log |

## Platform Attributes

Use platform attributes to write platform-specific code:

```naml
#[platforms(native, server)]
fn read_config(path: string) -> string throws IOError {
    # WASI and native file systems
    return fs::read_to_string(path);
}

#[platforms(browser)]
fn read_config(path: string) -> string throws IOError {
    # Browser OPFS
    return opfs::read(path);
}
```

Multiple platforms can share an implementation:

```naml
#[platforms(native, server, browser)]
fn compute_hash(data: string) -> string {
    # This works on all platforms
    return hash::sha256(data);
}
```

## Platform Detection at Runtime

Check the current platform at runtime:

```naml
fn main() {
    if (PLATFORM == "native") {
        println("Running on native");
    } else if (PLATFORM == "server") {
        println("Running on server WASM");
    } else if (PLATFORM == "browser") {
        println("Running in browser");
    }
}
```

## Compilation Pipeline

```
Source (.nm)
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

**Development flow:**
1. Edit your `.nm` files
2. Run with `naml run` (JIT, instant)
3. Iterate rapidly

**Production flow:**
1. Test with `naml run --cached`
2. Build with `naml build` (or `--target server/browser`)
3. Deploy the optimized binary/WASM

## Cross-Platform Best Practices

### 1. Use Platform Attributes Early

Don't write platform-specific code inline. Use attributes from the start:

```naml
# Good
#[platforms(native, server)]
fn save_file(path: string, data: string) {
    fs::write(path, data);
}

#[platforms(browser)]
fn save_file(path: string, data: string) {
    opfs::write(path, data);
}

# Bad - platform checks scattered everywhere
fn save_file(path: string, data: string) {
    if (PLATFORM == "browser") {
        opfs::write(path, data);
    } else {
        fs::write(path, data);
    }
}
```

### 2. Abstract Platform Differences

Create a common interface for platform-specific operations:

```naml
# storage.nm - Platform-agnostic storage API

#[platforms(native, server)]
pub fn store(key: string, value: string) throws IOError {
    fs::write(fmt("storage/{}", key), value);
}

#[platforms(browser)]
pub fn store(key: string, value: string) throws IOError {
    opfs::write(key, value);
}

pub fn load(key: string) -> option<string> {
    try {
        return some(platform_load(key));
    } catch (err: IOError) {
        return none;
    }
}

#[platforms(native, server)]
fn platform_load(key: string) -> string throws IOError {
    return fs::read_to_string(fmt("storage/{}", key));
}

#[platforms(browser)]
fn platform_load(key: string) -> string throws IOError {
    return opfs::read(key);
}
```

### 3. Test on All Targets

Before release, test your code on all supported platforms:

```bash
# Test native
naml test

# Test server WASM
naml build --target server
wasmtime target/server/main.wasm

# Test browser WASM
naml build --target browser
# Serve and test in browser
```

### 4. Document Platform Requirements

Clearly document which platforms your library or application supports:

```naml
##
## # Authentication Module
##
## Provides user authentication and session management.
##
## **Platforms**: native, server
##
## Browser is not supported due to secure storage requirements.
##

#[platforms(native, server)]
pub fn authenticate(user: string, password: string) -> bool {
    # Implementation
}
```

## Examples

### Simple Cross-Platform App

```naml
use std::random::*;

fn main() {
    var dice: int = random(1, 6);
    println(fmt("You rolled a {}", dice));
}
```

This works on all platforms:
```bash
naml run dice.nm                    # Native JIT
naml build && ./target/release/dice # Native binary
naml build --target server          # Server WASM
naml build --target browser         # Browser WASM
```

### Platform-Specific Networking

```naml
#[platforms(native, server)]
fn fetch_data(url: string) -> string throws HTTPError {
    return http::get(url);
}

#[platforms(browser)]
fn fetch_data(url: string) -> string throws HTTPError {
    # Use browser fetch API
    return browser::fetch(url);
}

fn main() {
    try {
        var data: string = fetch_data("https://api.example.com");
        println(data);
    } catch (err: HTTPError) {
        println(fmt("Error: {}", err));
    }
}
```

## Summary

naml's multi-target compilation lets you:
- Write once, deploy anywhere (with platform abstractions)
- Choose the right target for each use case
- Use JIT for development, optimized binaries for production
- Explicitly handle platform differences

Start with native for the fastest development cycle, then add other targets as needed.
