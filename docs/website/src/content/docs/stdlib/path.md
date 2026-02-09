---
title: "std::path"
description: Cross-platform path manipulation
---

Cross-platform path manipulation utilities for working with file system paths.

## Import

```naml
use std::path::*;
```

## Path Construction

### join

Join path components into a single path.

```naml
fn join(parts: [string]) -> string
```

**Example:**

```naml
var parts: [string] = ["home", "user", "documents", "file.txt"];
var path: string = join(parts);  // "home/user/documents/file.txt"
```

### normalize

Normalize a path, resolving `.` and `..`.

```naml
fn normalize(path: string) -> string
```

**Example:**

```naml
var messy: string = "a/b/../c/./d";
var clean: string = normalize(messy);  // "a/c/d"
```

## Path Type Checks

### is_absolute

Check if path is absolute.

```naml
fn is_absolute(path: string) -> bool
```

**Example:**

```naml
var abs: bool = is_absolute("/home/user/file.txt");  // true
var rel: bool = is_absolute("relative/path");        // false
```

### is_relative

Check if path is relative.

```naml
fn is_relative(path: string) -> bool
```

**Example:**

```naml
var rel: bool = is_relative("relative/path");  // true
```

### has_root

Check if path has a root component.

```naml
fn has_root(path: string) -> bool
```

**Example:**

```naml
var has: bool = has_root("/home/user");  // true
```

## Path Components

### dirname

Get parent directory.

```naml
fn dirname(path: string) -> string
```

**Example:**

```naml
var dir: string = dirname("/home/user/documents/report.pdf");
// "/home/user/documents"
```

### basename

Get filename with extension.

```naml
fn basename(path: string) -> string
```

**Example:**

```naml
var name: string = basename("/home/user/documents/report.pdf");
// "report.pdf"
```

### extension

Get file extension without dot.

```naml
fn extension(path: string) -> string
```

**Example:**

```naml
var ext: string = extension("/home/user/documents/report.pdf");
// "pdf"
```

### stem

Get filename without extension.

```naml
fn stem(path: string) -> string
```

**Example:**

```naml
var name: string = stem("/home/user/documents/report.pdf");
// "report"
```

### with_extension

Replace file extension.

```naml
fn with_extension(path: string, ext: string) -> string
```

**Example:**

```naml
var new_path: string = with_extension("/docs/readme.md", "txt");
// "/docs/readme.txt"
```

### components

Split path into components.

```naml
fn components(path: string) -> [string]
```

**Example:**

```naml
var parts: [string] = components("/usr/local/bin");
// ["/", "usr", "local", "bin"]
```

## Platform-Specific

### separator

Get platform path separator.

```naml
fn separator() -> string
```

**Returns:** `"/"` on Unix, `"\"` on Windows.

**Example:**

```naml
var sep: string = separator();  // "/"
```

### to_slash

Convert path to forward slashes.

```naml
fn to_slash(path: string) -> string
```

**Example:**

```naml
var unix_style: string = to_slash("a\\b\\c");  // "a/b/c"
```

### from_slash

Convert path from forward slashes to platform separator.

```naml
fn from_slash(path: string) -> string
```

**Example:**

```naml
var platform_path: string = from_slash("a/b/c");
// "a/b/c" on Unix, "a\b\c" on Windows
```

## Path Comparison

### starts_with

Check if path starts with base.

```naml
fn starts_with(path: string, base: string) -> bool
```

**Example:**

```naml
var result: bool = starts_with("/home/user/docs/file.txt", "/home/user");
// true
```

### ends_with

Check if path ends with suffix.

```naml
fn ends_with(path: string, suffix: string) -> bool
```

**Example:**

```naml
var result: bool = ends_with("/home/user/docs/file.txt", "file.txt");
// true
```

### strip_prefix

Remove prefix from path.

```naml
fn strip_prefix(path: string, prefix: string) -> string
```

**Example:**

```naml
var relative: string = strip_prefix("/home/user/docs/file.txt", "/home/user");
// "docs/file.txt"
```
