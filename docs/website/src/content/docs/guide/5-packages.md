---
title: Package Manager
description: Manage dependencies and share code with naml pkg
---

naml includes a built-in package manager (`naml pkg`) for managing dependencies. Packages can be sourced from Git repositories or local directories.

## Creating a Project

Scaffold a new naml project:

```bash
naml pkg init my-project
cd my-project
```

This creates:

```
my-project/
├── naml.toml    # Project manifest
└── main.nm      # Entry point
```

Run it immediately:

```bash
naml run main.nm
```

## Project Manifest

Every naml project has a `naml.toml` file that declares metadata and dependencies:

```toml
[package]
name = "my-project"
version = "0.1.0"
description = "My naml project"
authors = ["Your Name"]
license = "MIT"

[dependencies]
json = { git = "https://github.com/naml-lang/json", tag = "v0.1.0" }
utils = { path = "../shared/utils" }
```

### Package Metadata

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Package name |
| `version` | Yes | Semantic version string |
| `description` | No | Short description |
| `authors` | No | List of author names |
| `license` | No | License identifier |

## Dependency Sources

### Git Dependencies

Pull packages from any Git repository:

```toml
[dependencies]
json = { git = "https://github.com/naml-lang/json", tag = "v0.1.0" }
http = { git = "https://github.com/naml-lang/http", branch = "main" }
crypto = { git = "https://github.com/naml-lang/crypto", rev = "abc1234" }
```

Supported reference types:

| Reference | Description |
|-----------|-------------|
| `tag` | Git tag (recommended for stable versions) |
| `branch` | Git branch name |
| `rev` | Specific commit hash |
| *(none)* | Default branch |

### Local Path Dependencies

Reference packages on your local filesystem:

```toml
[dependencies]
mathlib = { path = "./libs/mathlib" }
shared = { path = "../shared-utils" }
```

Local paths are resolved relative to the `naml.toml` file location.

## Using Packages

Import functions from a package with `use`:

```naml
use mathlib::*;

fn main() {
    var result: int = add(10, 20);
    println(fmt("Result: {}", result));
}
```

You can also import specific items:

```naml
use mathlib::{add, multiply};
```

## Writing a Package

A package is any directory with a `naml.toml` and a `main.nm` entry point. All functions marked `pub` are available to consumers.

**libs/mathlib/naml.toml**:

```toml
[package]
name = "mathlib"
version = "0.1.0"
```

**libs/mathlib/main.nm**:

```naml
pub fn add(a: int, b: int) -> int {
    return a + b;
}

pub fn multiply(a: int, b: int) -> int {
    return a * b;
}
```

Only `pub` functions are exported. Functions without `pub` remain private to the package.

## CLI Commands

| Command | Description |
|---------|-------------|
| `naml pkg init [name]` | Create a new project with manifest and entry point |
| `naml pkg get` | Download and cache all dependencies |

When you run `naml run`, dependencies are resolved automatically if a `naml.toml` is present. You only need `naml pkg get` to pre-download packages or update the cache.

## Transitive Dependencies

Packages can declare their own dependencies. The resolver downloads the full dependency tree automatically and detects circular dependencies.

```
my-project
├── json (git)
│   └── encoding (git)
└── utils (local)
```

Diamond dependencies (multiple packages depending on the same package) are deduplicated when the source and version match.

## Cache

Downloaded Git packages are cached globally so they only need to be fetched once:

| Platform | Cache Location |
|----------|---------------|
| macOS | `~/Library/Caches/naml/packages/` |
| Linux | `~/.cache/naml/packages/` |
| Windows | `%LOCALAPPDATA%\naml\packages\` |

Local path dependencies are used directly from their location and are not cached.

## Project Structure Example

A typical project with dependencies:

```
my-project/
├── naml.toml
├── main.nm
└── libs/
    ├── mathlib/
    │   ├── naml.toml
    │   └── main.nm
    └── utils/
        ├── naml.toml
        └── main.nm
```

## Best Practices

1. **Pin Git dependencies** with `tag` for reproducible builds
2. **Use local paths** during development, switch to Git for releases
3. **Keep packages focused** on a single responsibility
4. **Export only what consumers need** with `pub`
5. **Include a `naml.toml`** in every package, even if it has no dependencies
