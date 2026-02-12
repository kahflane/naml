---
title: Installation
description: How to install naml
---

## Pre-built Binaries

Download the latest release for your platform from [GitHub Releases](https://github.com/kahflane/naml/releases).

| Platform | Architecture | Download |
|----------|-------------|----------|
| Linux | x86_64 | `naml-vX.X.X-x86_64-unknown-linux-gnu.tar.gz` |
| Linux | ARM64 | `naml-vX.X.X-aarch64-unknown-linux-gnu.tar.gz` |
| macOS | Intel | `naml-vX.X.X-x86_64-apple-darwin.tar.gz` |
| macOS | Apple Silicon | `naml-vX.X.X-aarch64-apple-darwin.tar.gz` |
| Windows | x86_64 | `naml-vX.X.X-x86_64-pc-windows-msvc.zip` |

Extract and add to your PATH:

```bash
# macOS / Linux
tar xzf naml-*.tar.gz
sudo mv naml naml-lsp /usr/local/bin/

# Windows — extract the zip and add the folder to your PATH
```

Each release includes both `naml` (compiler) and `naml-lsp` (language server).

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
| `naml run file.nm` | Execute with JIT |
| `naml run --release file.nm` | Optimized JIT (disables shadow stack) |
| `naml run --unsafe file.nm` | Skip array bounds checking |
| `naml build` | Build native binary |
| `naml build --target server` | Build server WASM (WIP) |
| `naml build --target browser` | Build browser WASM (WIP) |
| `naml check` | Type check only |
| `naml pkg init [name]` | Create a new project |
| `naml pkg get` | Download all dependencies |

Flags can be combined: `naml run --release --unsafe file.nm`

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
