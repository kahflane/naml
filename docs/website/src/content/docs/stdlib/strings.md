---
title: "std::strings"
description: String manipulation and analysis functions
---

String manipulation and analysis functions for text processing.

## Import

```naml
use std::strings::*;
```

## Functions

### len

Get the length of a string in bytes.

```naml
fn len(s: string) -> int
```

**Example:**

```naml
var length: int = len("hello");  // 5
```

### char_at

Get the character at a specific index.

```naml
fn char_at(s: string, index: int) -> string
```

**Example:**

```naml
var ch: string = char_at("hello", 1);  // "e"
```

### upper

Convert a string to uppercase.

```naml
fn upper(s: string) -> string
```

**Example:**

```naml
var result: string = upper("hello world");  // "HELLO WORLD"
```

### lower

Convert a string to lowercase.

```naml
fn lower(s: string) -> string
```

**Example:**

```naml
var result: string = lower("HELLO WORLD");  // "hello world"
```

### split

Split a string by a delimiter.

```naml
fn split(s: string, delimiter: string) -> [string]
```

**Example:**

```naml
var parts: [string] = split("a,b,c,d", ",");  // ["a", "b", "c", "d"]
```

### concat

Join an array of strings with a separator.

```naml
fn concat(parts: [string], separator: string) -> string
```

**Example:**

```naml
var joined: string = concat(["a", "b", "c"], "-");  // "a-b-c"
```

### has

Check if a string contains a substring.

```naml
fn has(s: string, substring: string) -> bool
```

**Example:**

```naml
var found: bool = has("hello world", "world");  // true
```

### starts_with

Check if a string starts with a prefix.

```naml
fn starts_with(s: string, prefix: string) -> bool
```

**Example:**

```naml
var result: bool = starts_with("hello world", "hello");  // true
```

### ends_with

Check if a string ends with a suffix.

```naml
fn ends_with(s: string, suffix: string) -> bool
```

**Example:**

```naml
var result: bool = ends_with("hello world", "world");  // true
```

### replace

Replace the first occurrence of a substring.

```naml
fn replace(s: string, from: string, to: string) -> string
```

**Example:**

```naml
var result: string = replace("foo bar foo", "foo", "baz");  // "baz bar foo"
```

### replace_all

Replace all occurrences of a substring.

```naml
fn replace_all(s: string, from: string, to: string) -> string
```

**Example:**

```naml
var result: string = replace_all("foo bar foo", "foo", "baz");  // "baz bar baz"
```

### ltrim

Remove leading whitespace.

```naml
fn ltrim(s: string) -> string
```

**Example:**

```naml
var result: string = ltrim("   hello");  // "hello"
```

### rtrim

Remove trailing whitespace.

```naml
fn rtrim(s: string) -> string
```

**Example:**

```naml
var result: string = rtrim("hello   ");  // "hello"
```

### substr

Extract a substring from start to end index.

```naml
fn substr(s: string, start: int, end: int) -> string
```

**Example:**

```naml
var result: string = substr("Hello World", 0, 5);  // "Hello"
var word: string = substr("Hello World", 6, 11);   // "World"
```

### lpad

Pad a string on the left to a target length.

```naml
fn lpad(s: string, length: int, pad: string) -> string
```

**Example:**

```naml
var result: string = lpad("42", 5, "0");  // "00042"
```

### rpad

Pad a string on the right to a target length.

```naml
fn rpad(s: string, length: int, pad: string) -> string
```

**Example:**

```naml
var result: string = rpad("hi", 5, ".");  // "hi..."
```

### repeat

Repeat a string n times.

```naml
fn repeat(s: string, count: int) -> string
```

**Example:**

```naml
var result: string = repeat("ab", 5);  // "ababababab"
```

### lines

Split a string into lines.

```naml
fn lines(s: string) -> [string]
```

**Example:**

```naml
var text: string = "line1\nline2\nline3";
var lns: [string] = lines(text);  // ["line1", "line2", "line3"]
```

### chars

Split a string into individual characters.

```naml
fn chars(s: string) -> [string]
```

**Example:**

```naml
var characters: [string] = chars("abc");  // ["a", "b", "c"]
```
