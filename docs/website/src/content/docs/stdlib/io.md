---
title: "std::io"
description: Terminal I/O and cursor control
---

Terminal I/O functions for interactive and TUI (terminal user interface) applications.

## Import

```naml
use std::io::*;
```

## Input Functions

### read_line

Read line from stdin (blocking).

```naml
fn read_line() -> string
```

**Note:** This is also available as a built-in function without import.

**Example:**

```naml
println("Enter your name:");
var name: string = read_line();
println(fmt("Hello, {}!", name));
```

### read_key

Non-blocking key read, returns key code or -1 if no key pressed.

```naml
fn read_key() -> int
```

**Returns:** ASCII code of key, or -1 if no input available.

**Example:**

```naml
var key: int = read_key();
if (key == 27) {
    println("ESC pressed");
} else if (key != -1) {
    println(fmt("Key code: {}", key));
}
```

## Screen Control

### clear_screen

Clear terminal screen.

```naml
fn clear_screen()
```

**Example:**

```naml
clear_screen();
println("Fresh screen!");
```

## Cursor Control

### set_cursor

Move cursor to position (0-indexed).

```naml
fn set_cursor(x: int, y: int)
```

**Parameters:**
- `x` - Column (0 = leftmost)
- `y` - Row (0 = topmost)

**Example:**

```naml
set_cursor(10, 5);
println("Text at (10, 5)");
```

### hide_cursor

Hide terminal cursor.

```naml
fn hide_cursor()
```

**Example:**

```naml
hide_cursor();
```

### show_cursor

Show terminal cursor.

```naml
fn show_cursor()
```

**Example:**

```naml
show_cursor();
```

## Terminal Information

### terminal_width

Get terminal width in columns.

```naml
fn terminal_width() -> int
```

**Example:**

```naml
var width: int = terminal_width();
println(fmt("Terminal is {} columns wide", width));
```

### terminal_height

Get terminal height in rows.

```naml
fn terminal_height() -> int
```

**Example:**

```naml
var height: int = terminal_height();
println(fmt("Terminal is {} rows tall", height));
```

## TUI Example

```naml
use std::io::*;

fn main() {
    clear_screen();
    hide_cursor();

    var width: int = terminal_width();
    var height: int = terminal_height();

    set_cursor(width / 2 - 5, height / 2);
    println("Hello, TUI!");

    set_cursor(0, height - 1);
    println("Press any key to exit...");

    var key: int = -1;
    while (key == -1) {
        key = read_key();
    }

    clear_screen();
    show_cursor();
    set_cursor(0, 0);
}
```
