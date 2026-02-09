---
title: File I/O
description: File reading, writing, directories, and file handles in naml
---

## File System Basics

Reading, writing, copying, and managing files:

```naml
use std::fs::*;

fn main() {
    var test_dir: string = "/tmp/naml_fs_test";
    var test_file: string = "/tmp/naml_fs_test/hello.txt";

    // Create directory
    mkdir_all(test_dir) catch e {
        println("Error: {}", e.message);
    };

    // Write to file
    write(test_file, "Hello, naml!") catch e {
        println("Error: {}", e.message);
    };

    // Read file
    var content: string = read(test_file) catch e {
        println("Error: {}", e.message);
    };
    println("Content: {}", content);

    // Append
    append(test_file, "\nAppended line.") catch e {
        println("Error: {}", e.message);
    };

    // File properties
    println("exists: {}", exists(test_file));
    println("is_file: {}", is_file(test_file));
    println("is_dir: {}", is_dir(test_file));

    var file_size: int = size(test_file) catch e {
        println("Error: {}", e.message);
    };
    println("Size: {} bytes", file_size);

    // Binary data
    var bytes_file: string = "/tmp/naml_fs_test/data.bin";
    write_bytes(bytes_file, "Binary data!" as bytes) catch e {
        println("Error: {}", e.message);
    };

    // Copy and rename
    copy(test_file, "/tmp/naml_fs_test/copy.txt") catch e {
        println("Error: {}", e.message);
    };
    rename("/tmp/naml_fs_test/copy.txt", "/tmp/naml_fs_test/renamed.txt") catch e {
        println("Error: {}", e.message);
    };

    // Cleanup
    remove_all(test_dir) catch e {
        println("Error: {}", e.message);
    };
}
```

Key patterns:
- All file operations throw `IOError` and must be caught with `catch`
- Use `write` / `read` for strings, `write_bytes` / `read_bytes` for binary data
- `mkdir_all` creates directories recursively
- `remove_all` deletes directories and their contents
