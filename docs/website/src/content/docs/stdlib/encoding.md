---
title: "std::encoding"
description: UTF-8, hex, base64, URL, JSON, TOML, YAML, and binary data encoding
---

Encoding and decoding utilities for various data formats.

## Import

```naml
use std::encoding::*;
use std::encoding::utf8::*;
use std::encoding::hex::*;
use std::encoding::base64::*;
use std::encoding::url::*;
use std::encoding::json::*;
use std::encoding::toml::*;
use std::encoding::yaml::*;
use std::encoding::binary::*;
```

## UTF-8

### encode

Encode string to UTF-8 bytes.

```naml
fn encode(s: string) -> bytes
```

**Example:**

```naml
var data: bytes = utf8::encode("Hello, 世界");
```

### decode

Decode UTF-8 bytes to string.

```naml
fn decode(data: bytes) -> string throws DecodeError
```

**Example:**

```naml
var text: string = utf8::decode(data) catch e {
    println(e.message);
    return;
};
```

### validate

Check if bytes are valid UTF-8.

```naml
fn validate(data: bytes) -> bool
```

**Example:**

```naml
var valid: bool = utf8::validate(data);
```

## Hex Encoding

### encode

Encode bytes to hexadecimal string.

```naml
fn encode(data: bytes) -> string
```

**Example:**

```naml
var hex_str: string = hex::encode("Hello" as bytes);
// "48656c6c6f"
```

### decode

Decode hexadecimal string to bytes.

```naml
fn decode(hex: string) -> bytes throws DecodeError
```

**Example:**

```naml
var data: bytes = hex::decode("48656c6c6f") catch e {
    println(e.message);
    return;
};
```

## Base64

### encode

Encode bytes to base64 string.

```naml
fn encode(data: bytes) -> string
```

**Example:**

```naml
var b64: string = base64::encode("Hello" as bytes);
// "SGVsbG8="
```

### decode

Decode base64 string to bytes.

```naml
fn decode(b64: string) -> bytes throws DecodeError
```

**Example:**

```naml
var data: bytes = base64::decode("SGVsbG8=") catch e {
    println(e.message);
    return;
};
```

## URL Encoding

### encode

URL-encode string.

```naml
fn encode(s: string) -> string
```

**Example:**

```naml
var encoded: string = url::encode("hello world&foo=bar");
// "hello%20world%26foo%3Dbar"
```

### decode

URL-decode string.

```naml
fn decode(s: string) -> string
```

**Example:**

```naml
var decoded: string = url::decode("hello%20world");
// "hello world"
```

## JSON

### decode

Parse JSON string to json type.

```naml
fn decode(json_str: string) -> json throws DecodeError
```

**Example:**

```naml
var data: json = json::decode(`{"name":"Alice","age":30}`) catch e {
    println(e.message);
    return;
};
```

### encode

Convert json to compact JSON string.

```naml
fn encode(data: json) -> string
```

**Example:**

```naml
var json_str: string = json::encode(data);
```

### encode_pretty

Convert json to pretty-printed JSON string.

```naml
fn encode_pretty(data: json) -> string
```

**Example:**

```naml
var pretty: string = json::encode_pretty(data);
```

### exists

Check if key exists in JSON object.

```naml
fn exists(data: json, key: string) -> bool
```

**Example:**

```naml
var has_name: bool = json::exists(user, "name");
```

### path

Navigate JSON using jq-style path.

```naml
fn path(data: json, path: string) -> json throws PathError
```

**Path syntax:** `.key`, `.key[index]`, `.key.nested`

**Example:**

```naml
var name: json = json::path(data, ".users[0].name") catch e {
    println(e.message);
    return;
};
```

### keys

Get object keys as array.

```naml
fn keys(data: json) -> [string]
```

**Example:**

```naml
var object_keys: [string] = json::keys(user);
```

### count

Get array length or object key count.

```naml
fn count(data: json) -> int
```

**Example:**

```naml
var length: int = json::count(array);
```

### get_type

Get JSON type as integer.

```naml
fn get_type(data: json) -> int
```

**Returns:** Type code (0=null, 1=bool, 2=number, 3=string, 4=array, 5=object).

**Example:**

```naml
var type_code: int = json::get_type(value);
```

### type_name

Get JSON type as string.

```naml
fn type_name(data: json) -> string
```

**Returns:** `"null"`, `"bool"`, `"number"`, `"string"`, `"array"`, or `"object"`.

**Example:**

```naml
var type_str: string = json::type_name(value);  // "string"
```

### is_null

Check if JSON value is null.

```naml
fn is_null(data: json) -> bool
```

**Example:**

```naml
if (json::is_null(value)) {
    println("Value is null");
}
```

### JSON Type Checking

Use `is` operator with JSON variant types:

```naml
if (value is json_string) {
    var s: string = value as string;
}

if (value is json_number) {
    var n: int = value as int;
}

if (value is json_bool) {
    var b: int = value as int;  // 0 or 1
}

if (value is json_null) {
    println("null value");
}

if (value is json_array) {
    // Array operations
}

if (value is json_object) {
    // Object operations
}
```

## TOML

### decode

Parse TOML string to json.

```naml
fn decode(toml_str: string) -> json throws DecodeError
```

**Example:**

```naml
var config: json = toml::decode(`
[server]
host = "localhost"
port = 8080
`) catch e {
    println(e.message);
    return;
};
```

### encode

Convert json to TOML string.

```naml
fn encode(data: json) -> string
```

**Example:**

```naml
var toml_str: string = toml::encode(config);
```

## YAML

### decode

Parse YAML string to json.

```naml
fn decode(yaml_str: string) -> json throws DecodeError
```

**Example:**

```naml
var data: json = yaml::decode(`
users:
  - name: Alice
    age: 30
`) catch e {
    println(e.message);
    return;
};
```

### encode

Convert json to YAML string.

```naml
fn encode(data: json) -> string
```

**Example:**

```naml
var yaml_str: string = yaml::encode(data);
```

## Binary Data

Low-level binary data manipulation.

### alloc

Allocate binary buffer with capacity.

```naml
fn alloc(capacity: int) -> bytes
```

**Example:**

```naml
var buffer: bytes = binary::alloc(1024);
```

### from_string

Convert string to bytes.

```naml
fn from_string(s: string) -> bytes
```

**Example:**

```naml
var data: bytes = binary::from_string("hello");
```

### len

Get buffer length.

```naml
fn len(data: bytes) -> int
```

### capacity

Get buffer capacity.

```naml
fn capacity(data: bytes) -> int
```

### clear

Clear buffer contents.

```naml
fn clear(data: bytes)
```

### append

Append bytes to buffer.

```naml
fn append(dest: bytes, src: bytes)
```

### resize

Resize buffer to new length.

```naml
fn resize(data: bytes, new_len: int)
```

### fill

Fill buffer with byte value.

```naml
fn fill(data: bytes, value: int)
```

### slice

Extract slice from buffer.

```naml
fn slice(data: bytes, start: int, end: int) -> bytes
```

### concat

Concatenate two byte buffers.

```naml
fn concat(a: bytes, b: bytes) -> bytes
```

### index_of

Find first occurrence of byte sequence.

```naml
fn index_of(data: bytes, pattern: bytes) -> option<int>
```

### contains

Check if buffer contains pattern.

```naml
fn contains(data: bytes, pattern: bytes) -> bool
```

### starts_with

Check if buffer starts with prefix.

```naml
fn starts_with(data: bytes, prefix: bytes) -> bool
```

### ends_with

Check if buffer ends with suffix.

```naml
fn ends_with(data: bytes, suffix: bytes) -> bool
```

### equals

Compare two buffers for equality.

```naml
fn equals(a: bytes, b: bytes) -> bool
```

### copy_within

Copy data within buffer.

```naml
fn copy_within(data: bytes, src_start: int, src_end: int, dest: int)
```

## Binary Read/Write

Functions for reading and writing primitive types in little-endian (le) or big-endian (be) byte order.

### Read Functions

```naml
fn read_u8(data: bytes, offset: int) -> int
fn read_i8(data: bytes, offset: int) -> int
fn read_u16_le(data: bytes, offset: int) -> int
fn read_u16_be(data: bytes, offset: int) -> int
fn read_i16_le(data: bytes, offset: int) -> int
fn read_i16_be(data: bytes, offset: int) -> int
fn read_u32_le(data: bytes, offset: int) -> int
fn read_u32_be(data: bytes, offset: int) -> int
fn read_i32_le(data: bytes, offset: int) -> int
fn read_i32_be(data: bytes, offset: int) -> int
fn read_u64_le(data: bytes, offset: int) -> int
fn read_u64_be(data: bytes, offset: int) -> int
fn read_i64_le(data: bytes, offset: int) -> int
fn read_i64_be(data: bytes, offset: int) -> int
fn read_f32_le(data: bytes, offset: int) -> float
fn read_f32_be(data: bytes, offset: int) -> float
fn read_f64_le(data: bytes, offset: int) -> float
fn read_f64_be(data: bytes, offset: int) -> float
```

### Write Functions

```naml
fn write_u8(data: bytes, offset: int, value: int)
fn write_i8(data: bytes, offset: int, value: int)
fn write_u16_le(data: bytes, offset: int, value: int)
fn write_u16_be(data: bytes, offset: int, value: int)
fn write_i16_le(data: bytes, offset: int, value: int)
fn write_i16_be(data: bytes, offset: int, value: int)
fn write_u32_le(data: bytes, offset: int, value: int)
fn write_u32_be(data: bytes, offset: int, value: int)
fn write_i32_le(data: bytes, offset: int, value: int)
fn write_i32_be(data: bytes, offset: int, value: int)
fn write_u64_le(data: bytes, offset: int, value: int)
fn write_u64_be(data: bytes, offset: int, value: int)
fn write_i64_le(data: bytes, offset: int, value: int)
fn write_i64_be(data: bytes, offset: int, value: int)
fn write_f32_le(data: bytes, offset: int, value: float)
fn write_f32_be(data: bytes, offset: int, value: float)
fn write_f64_le(data: bytes, offset: int, value: float)
fn write_f64_be(data: bytes, offset: int, value: float)
```

**Example:**

```naml
var buffer: bytes = binary::alloc(8);
binary::write_u32_le(buffer, 0, 12345);
binary::write_u32_le(buffer, 4, 67890);

var val1: int = binary::read_u32_le(buffer, 0);  // 12345
var val2: int = binary::read_u32_le(buffer, 4);  // 67890
```
