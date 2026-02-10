# naml 🐜

>> "Naml" (Arabic: نمل) is pronounced "num-l" or "nam-l"

A statically-typed programming language with Cranelift JIT compilation, deterministic memory management, and a batteries-included standard library.

## Quick Start

```bash
# Build the compiler
cargo build --release

# Run a program
naml run examples/basic.nm
```

```naml
use std::threads::{spawn, join, with_mutex};
use std::crypto::sha256_hex;

pub struct User {
    pub name: string,
    pub email: string
}

fn hash_password(password: string) -> string {
    return sha256_hex(password as bytes);
}

fn main() {
    var counter: mutex<int> = with_mutex(0);

    var users: [User] = [
        User { name: "Alice", email: "alice@example.com" },
        User { name: "Bob", email: "bob@example.com" }
    ];

    for (i: int, user: User in users) {
        spawn {
            println(fmt("{}: {}", user.name, hash_password(user.email)));
            locked (c: int in counter) {
                c = c + 1;
            }
        };
    }

    join();
}
```

## Features

- **Cranelift JIT** -- compiles to native machine code, no interpreter
- **Reference counting** -- deterministic memory management, no GC pauses
- **Arena allocation** -- inline alloc/free for struct-heavy workloads
- **M:N threading** -- `spawn` blocks with channels, mutexes, atomics
- **Strong typing** -- static type checking with generics, option types, interfaces
- **FFI** -- call C functions directly via `extern fn`
- **Package manager** -- `naml pkg` with git and local dependencies

## Performance

Binary tree benchmark (allocate and deallocate 4M+ nodes):

| Runtime | Time | vs naml |
|---------|------|---------|
| **naml** | **4.25s** | 1.0x |
| Bun | 4.51s | 1.06x slower |
| Node.js | 8.89s | 2.1x slower |
| Go | 11.76s | 2.8x slower |

## Standard Library

| Module | Description |
|--------|-------------|
| `std::strings` | split, join, replace, trim, upper, lower, pad |
| `std::collections` | array and map operations (push, pop, map, filter, reduce) |
| `std::encoding` | JSON, TOML, YAML, Base64, Hex, URL encoding, binary buffers |
| `std::crypto` | SHA-256/512, MD5, HMAC, PBKDF2, secure random |
| `std::net` | HTTP server (Chi-style router, middleware), HTTP client |
| `std::db::sqlite` | SQLite3 with prepared statements and transactions |
| `std::threads` | spawn, join, channels, mutexes, rwlocks, atomics |
| `std::fs` | read, write, copy, move, glob, permissions, memory-mapped files |
| `std::path` | join, normalize, extension, components |
| `std::io` | terminal input, cursor control, raw mode |
| `std::process` | exec, spawn processes, signals, pipes |
| `std::os` | hostname, uid, platform info |
| `std::env` | environment variables |
| `std::datetime` | timestamps, formatting, components |
| `std::timers` | scheduled and recurring timers |
| `std::metrics` | high-resolution timing (ns/us/ms) |
| `std::testing` | assertions |
| `std::random` | random integers, floats |

## Type System

```naml
// Primitives
var x: int = 42;
var pi: float = 3.14;
var name: string = "Alice";
var data: bytes = "raw" as bytes;
var active: bool = true;

// Composite types
var items: [int] = [1, 2, 3];
var fixed: [int; 3] = [1, 2, 3];
var lookup: map<string, int> = {"a": 1, "b": 2};
var maybe: option<string> = some("hello");

// Generics
fn identity<T>(value: T) -> T {
    return value;
}
```

## Concurrency

```naml
use std::threads::{spawn, join, with_channel, send, receive, with_mutex};

var ch: channel<string> = with_channel(10);
var results: mutex<[string]> = with_mutex([]);

for (i: int in range(0, 4)) {
    spawn {
        send(ch, fmt("worker-{}", i));
    };
}

for (i: int in range(0, 4)) {
    var msg: string = receive(ch);
    locked (r: [string] in results) {
        push(r, msg);
    }
}

join();
```

## HTTP Server

```naml
use std::net::http::server::*;
use std::net::http::middleware::*;
use std::encoding::json::{encode};

var router: int = open_router();
with(router, logger());
with(router, cors("*"));

get(router, "/health", fn(req: request) -> response {
    return json_response(200, encode({"status": "ok"}));
});

get(router, "/users/{id}", fn(req: request) -> response {
    var id: string = param(req, "id");
    return json_response(200, encode({"id": id}));
});

serve(":8080", router);
```

## Packages

```toml
# naml.toml
[package]
name = "my-app"
version = "0.1.0"

[dependencies]
utils = { path = "./libs/utils" }
http-helpers = { git = "https://github.com/user/http-helpers", tag = "v1.0" }
```

```bash
naml pkg get    # download dependencies
naml run main.nm
```

## Project Structure

```
naml/
├── namlc/                # Compiler (lexer, parser, typechecker, Cranelift codegen)
├── std/                  # Standard library crates (21 modules)
├── tools/naml-pkg/       # Package manager
├── editors/vscode/       # VS Code extension with LSP
├── examples/             # Example programs
└── docs/                 # Language reference and website
```

## CLI

```bash
naml run file.nm              # Execute with JIT
naml run --release file.nm    # Execute with optimizations
naml check                    # Type check without running
naml pkg init                 # Create new project
naml pkg get                  # Download dependencies
```

## Requirements

- Rust 1.75+
- macOS, Linux, or Windows

## License

MIT
