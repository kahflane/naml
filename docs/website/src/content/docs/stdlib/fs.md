---
title: "std::fs"
description: File and directory operations
---

File and directory operations for reading, writing, and managing the file system.

## Import

```naml
use std::fs::*;
```

## Error Handling

All file system operations throw `IOError` on failure.

## File Operations

### read

Read entire file as string.

```naml
fn read(path: string) -> string throws IOError
```

**Example:**

```naml
var content: string = read("/tmp/hello.txt") catch e {
    println(e.message);
    return;
};
```

### read_bytes

Read entire file as bytes.

```naml
fn read_bytes(path: string) -> bytes throws IOError
```

**Example:**

```naml
var data: bytes = read_bytes("/tmp/data.bin") catch e {
    println(e.message);
    return;
};
```

### write

Write string to file, creating or overwriting.

```naml
fn write(path: string, content: string) throws IOError
```

**Example:**

```naml
write("/tmp/hello.txt", "Hello, naml!") catch e {
    println(e.message);
};
```

### write_bytes

Write bytes to file, creating or overwriting.

```naml
fn write_bytes(path: string, data: bytes) throws IOError
```

**Example:**

```naml
var data: bytes = "Binary data" as bytes;
write_bytes("/tmp/data.bin", data) catch e {
    println(e.message);
};
```

### append

Append string to file.

```naml
fn append(path: string, content: string) throws IOError
```

**Example:**

```naml
append("/tmp/log.txt", "New log entry\n") catch e {
    println(e.message);
};
```

### append_bytes

Append bytes to file.

```naml
fn append_bytes(path: string, data: bytes) throws IOError
```

**Example:**

```naml
var more: bytes = " more data" as bytes;
append_bytes("/tmp/data.bin", more) catch e {
    println(e.message);
};
```

### exists

Check if path exists.

```naml
fn exists(path: string) -> bool
```

**Example:**

```naml
if (exists("/tmp/hello.txt")) {
    println("File exists");
}
```

### is_file

Check if path is a regular file.

```naml
fn is_file(path: string) -> bool
```

**Example:**

```naml
if (is_file("/tmp/hello.txt")) {
    println("Is a file");
}
```

### is_dir

Check if path is a directory.

```naml
fn is_dir(path: string) -> bool
```

**Example:**

```naml
if (is_dir("/tmp")) {
    println("Is a directory");
}
```

### size

Get file size in bytes.

```naml
fn size(path: string) -> int throws IOError
```

**Example:**

```naml
var bytes: int = size("/tmp/hello.txt") catch e {
    println(e.message);
    return;
};
```

### modified

Get last modified time as Unix timestamp.

```naml
fn modified(path: string) -> int throws IOError
```

**Example:**

```naml
var mtime: int = modified("/tmp/hello.txt") catch e {
    println(e.message);
    return;
};
```

### copy

Copy file from source to destination.

```naml
fn copy(from: string, to: string) throws IOError
```

**Example:**

```naml
copy("/tmp/source.txt", "/tmp/dest.txt") catch e {
    println(e.message);
};
```

### rename

Rename or move a file or directory.

```naml
fn rename(from: string, to: string) throws IOError
```

**Example:**

```naml
rename("/tmp/old.txt", "/tmp/new.txt") catch e {
    println(e.message);
};
```

## Directory Operations

### list_dir

List directory contents, returns array of names.

```naml
fn list_dir(path: string) -> [string] throws IOError
```

**Example:**

```naml
var entries: [string] = list_dir("/tmp") catch e {
    println(e.message);
    return;
};
```

### mkdir

Create a directory.

```naml
fn mkdir(path: string) throws IOError
```

**Example:**

```naml
mkdir("/tmp/test_dir") catch e {
    println(e.message);
};
```

### mkdir_all

Create directory and all parent directories.

```naml
fn mkdir_all(path: string) throws IOError
```

**Example:**

```naml
mkdir_all("/tmp/a/b/c") catch e {
    println(e.message);
};
```

### remove

Remove a file or empty directory.

```naml
fn remove(path: string) throws IOError
```

**Example:**

```naml
remove("/tmp/file.txt") catch e {
    println(e.message);
};
```

### remove_all

Remove file or directory recursively.

```naml
fn remove_all(path: string) throws IOError
```

**Example:**

```naml
remove_all("/tmp/test_dir") catch e {
    println(e.message);
};
```

### getwd

Get current working directory.

```naml
fn getwd() -> string throws IOError
```

**Example:**

```naml
var cwd: string = getwd() catch e {
    println(e.message);
    return;
};
```

### chdir

Change current working directory.

```naml
fn chdir(path: string) throws IOError
```

**Example:**

```naml
chdir("/tmp") catch e {
    println(e.message);
};
```

### chmod

Change file permissions (Unix mode).

```naml
fn chmod(path: string, mode: int) throws IOError
```

**Example:**

```naml
chmod("/tmp/script.sh", 0o755) catch e {
    println(e.message);
};
```

### truncate

Truncate file to specified size.

```naml
fn truncate(path: string, size: int) throws IOError
```

**Example:**

```naml
truncate("/tmp/file.txt", 100) catch e {
    println(e.message);
};
```

### stat

Get file metadata as map.

```naml
fn stat(path: string) -> map<string, int> throws IOError
```

**Returns:** Map with keys: `size`, `modified`, `accessed`, `created`, `mode`, `is_file`, `is_dir`.

**Example:**

```naml
var info: map<string, int> = stat("/tmp/file.txt") catch e {
    println(e.message);
    return;
};
var file_size: int = info["size"]!;
```

### create_temp

Create temporary file, returns path.

```naml
fn create_temp(prefix: string) -> string throws IOError
```

**Example:**

```naml
var temp_path: string = create_temp("naml_") catch e {
    println(e.message);
    return;
};
```

### mkdir_temp

Create temporary directory, returns path.

```naml
fn mkdir_temp(prefix: string) -> string throws IOError
```

**Example:**

```naml
var temp_dir: string = mkdir_temp("naml_dir_") catch e {
    println(e.message);
    return;
};
```

## File Handle Operations

Low-level file handle operations for fine-grained control.

### file_open

Open file and return handle.

```naml
fn file_open(path: string, mode: string) -> int throws IOError
```

**Modes:** `"r"` (read), `"w"` (write), `"a"` (append), `"r+"` (read+write), `"w+"` (write+read), `"a+"` (append+read).

**Example:**

```naml
var fd: int = file_open("/tmp/file.txt", "r") catch e {
    println(e.message);
    return;
};
```

### file_read

Read from file handle into bytes.

```naml
fn file_read(fd: int, size: int) -> bytes throws IOError
```

**Example:**

```naml
var data: bytes = file_read(fd, 1024) catch e {
    println(e.message);
    return;
};
```

### file_write

Write bytes to file handle.

```naml
fn file_write(fd: int, data: bytes) -> int throws IOError
```

**Returns:** Number of bytes written.

**Example:**

```naml
var written: int = file_write(fd, "hello" as bytes) catch e {
    println(e.message);
    return;
};
```

### file_seek

Seek to position in file.

```naml
fn file_seek(fd: int, offset: int, whence: int) -> int throws IOError
```

**Whence:** `0` (start), `1` (current), `2` (end).

**Example:**

```naml
var pos: int = file_seek(fd, 0, 0) catch e {
    println(e.message);
    return;
};
```

### file_tell

Get current position in file.

```naml
fn file_tell(fd: int) -> int throws IOError
```

**Example:**

```naml
var pos: int = file_tell(fd) catch e {
    println(e.message);
    return;
};
```

### file_flush

Flush buffered writes to disk.

```naml
fn file_flush(fd: int) throws IOError
```

**Example:**

```naml
file_flush(fd) catch e {
    println(e.message);
};
```

### file_close

Close file handle.

```naml
fn file_close(fd: int) throws IOError
```

**Example:**

```naml
file_close(fd) catch e {
    println(e.message);
};
```

### file_size

Get size of open file.

```naml
fn file_size(fd: int) -> int throws IOError
```

**Example:**

```naml
var bytes: int = file_size(fd) catch e {
    println(e.message);
    return;
};
```

### file_truncate

Truncate open file to size.

```naml
fn file_truncate(fd: int, size: int) throws IOError
```

**Example:**

```naml
file_truncate(fd, 100) catch e {
    println(e.message);
};
```

### file_sync

Sync file data and metadata to disk.

```naml
fn file_sync(fd: int) throws IOError
```

**Example:**

```naml
file_sync(fd) catch e {
    println(e.message);
};
```

### file_chmod

Change permissions of open file.

```naml
fn file_chmod(fd: int, mode: int) throws IOError
```

**Example:**

```naml
file_chmod(fd, 0o644) catch e {
    println(e.message);
};
```

### file_chown

Change owner of open file.

```naml
fn file_chown(fd: int, uid: int, gid: int) throws IOError
```

**Example:**

```naml
file_chown(fd, 1000, 1000) catch e {
    println(e.message);
};
```

## Memory-Mapped Files

### mmap_open

Open memory-mapped file for reading.

```naml
fn mmap_open(path: string) -> int throws IOError
```

**Returns:** Memory map handle.

**Example:**

```naml
var mmap: int = mmap_open("/tmp/large.dat") catch e {
    println(e.message);
    return;
};
```

### mmap_read

Read bytes from memory-mapped file.

```naml
fn mmap_read(mmap: int, offset: int, size: int) -> bytes throws IOError
```

**Example:**

```naml
var data: bytes = mmap_read(mmap, 0, 1024) catch e {
    println(e.message);
    return;
};
```

### mmap_size

Get size of memory-mapped file.

```naml
fn mmap_size(mmap: int) -> int throws IOError
```

**Example:**

```naml
var total: int = mmap_size(mmap) catch e {
    println(e.message);
    return;
};
```

### mmap_close

Close memory-mapped file.

```naml
fn mmap_close(mmap: int) throws IOError
```

**Example:**

```naml
mmap_close(mmap) catch e {
    println(e.message);
};
```

## Symbolic Links

### symlink

Create symbolic link.

```naml
fn symlink(target: string, link: string) throws IOError
```

**Example:**

```naml
symlink("/tmp/target.txt", "/tmp/link.txt") catch e {
    println(e.message);
};
```

### readlink

Read symbolic link target.

```naml
fn readlink(link: string) -> string throws IOError
```

**Example:**

```naml
var target: string = readlink("/tmp/link.txt") catch e {
    println(e.message);
    return;
};
```

### is_symlink

Check if path is a symbolic link.

```naml
fn is_symlink(path: string) -> bool
```

**Example:**

```naml
if (is_symlink("/tmp/link.txt")) {
    println("Is a symbolic link");
}
```
