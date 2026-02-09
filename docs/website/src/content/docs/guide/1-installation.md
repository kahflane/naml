---
title: Installation
description: How to install naml
---

## Building from Source

naml is built with Rust. You need a recent Rust toolchain installed.

```bash
# Clone the repository
git clone https://github.com/kahflane/naml.git
cd naml

# Build in release mode
cargo build --release

# The binary is at target/release/naml
```

## Verify Installation

```bash
naml --version
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `naml run file.nm` | Execute with JIT (fast dev mode) |
| `naml run --cached file.nm` | Use cached compilation (sub-millisecond start) |
| `naml build` | Build native binary (WIP) |
| `naml build --target server` | Build server WASM (WIP) |
| `naml build --target browser` | Build browser WASM (WIP) |
| `naml check` | Type check only |
| `naml pkg init [name]` | Create a new project |
| `naml pkg get` | Download all dependencies |

## Project Structure

A naml project typically has this structure:

```
my-project/
├── naml.toml          # Project manifest
├── main.nm            # Entry point
└── modules/           # Additional modules
    ├── math.nm
    └── utils.nm
```
