---
title: "Standard Library"
description: Complete reference for naml's standard library modules
---

The naml standard library provides a comprehensive set of modules for common programming tasks.

## Available Modules

### String & Text Processing
- **[std::strings](/stdlib/strings)** - String manipulation and analysis
- **[std::encoding](/stdlib/encoding)** - UTF-8, hex, base64, URL, JSON, TOML, YAML, and binary data encoding

### Data Structures
- **[std::collections](/stdlib/collections)** - Array and map operations with functional programming support

### File System & Paths
- **[std::fs](/stdlib/fs)** - File and directory operations
- **[std::path](/stdlib/path)** - Cross-platform path manipulation

### Networking
- **[std::net](/stdlib/net)** - TCP, UDP, and HTTP client/server APIs

### Database
- **[std::db::sqlite](/stdlib/db-sqlite)** - SQLite3 database integration

### Concurrency
- **[std::threads](/stdlib/threads)** - Channels, mutex, rwlock, atomics, and thread management

### Date & Time
- **[std::datetime](/stdlib/datetime)** - Date and time utilities
- **[std::timers](/stdlib/timers)** - Timeout, interval, and cron-style scheduling

### System & Environment
- **[std::env](/stdlib/env)** - Environment variable access
- **[std::os](/stdlib/os)** - Operating system information
- **[std::process](/stdlib/process)** - Process management and signals

### Input/Output
- **[std::io](/stdlib/io)** - Terminal I/O and cursor control
- **[std::random](/stdlib/random)** - Random number generation

### Testing & Metrics
- **[std::testing](/stdlib/testing)** - Test assertions and utilities
- **[std::metrics](/stdlib/metrics)** - Performance measurement

## Usage

Import modules using the `use` statement:

```naml
use std::strings::*;
use std::collections::arrays::{push, pop, count};
use std::fs::{read, write, exists};
```

## Platform Compatibility

Most standard library modules work across all platforms (native, server WASM, browser WASM). Platform-specific restrictions are noted in each module's documentation.
