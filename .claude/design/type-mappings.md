# Type Mappings (naml → Rust)

## Primitive Types

| naml | Rust | Notes |
|------|------|-------|
| `int` | `i64` | Signed 64-bit integer |
| `uint` | `u64` | Unsigned 64-bit integer |
| `float` | `f64` | IEEE 754 double precision |
| `bool` | `bool` | true/false |
| `string` | `String` | UTF-8 string (owned) |
| `bytes` | `Vec<u8>` | Byte array |
| `unit` | `()` | Unit type |

## Collection Types

| naml | Rust | Notes |
|------|------|-------|
| `[T]` | `Vec<T>` | Dynamic array |
| `[T; N]` | `[T; N]` | Fixed-size array |
| `map<K, V>` | `HashMap<K, V>` | Hash map |

## Optional & Result

| naml | Rust | Notes |
|------|------|-------|
| `option<T>` | `Option<T>` | Some/None |
| `some(x)` | `Some(x)` | Wrap value |
| `none` | `None` | No value |

## Concurrency Types

| naml | Rust | Notes |
|------|------|-------|
| `channel<T>` | `mpsc::Sender<T>` / `mpsc::Receiver<T>` | tokio channels |
| `promise<T>` | `impl Future<Output = T>` | Async future |

## Function Types

| naml | Rust | Notes |
|------|------|-------|
| `fn(A) -> B` | `fn(A) -> B` | Function pointer |
| `\|x\| expr` | `\|x\| expr` | Closure |
| `async fn` | `async fn` | Async function |

## User-Defined Types

### Structs
```naml
struct Point {
    x: int,
    y: int
}
```
→
```rust
struct Point {
    x: i64,
    y: i64,
}
```

### Enums
```naml
enum Option<T> {
    Some(T),
    None
}
```
→
```rust
enum Option<T> {
    Some(T),
    None,
}
```

### Interfaces → Traits
```naml
interface Comparable<T> {
    fn compare(other: T) -> int;
}
```
→
```rust
trait Comparable<T> {
    fn compare(&self, other: T) -> i64;
}
```

## Reference Types

| naml | Rust | Notes |
|------|------|-------|
| Value | Owned | Default |
| `&T` | `&T` | Immutable borrow |
| `&mut T` | `&mut T` | Mutable borrow |

## Platform-Specific

| naml | Native Rust | Browser Rust |
|------|-------------|--------------|
| File I/O | `std::fs` | OPFS via wasm-bindgen |
| HTTP | `reqwest` | `fetch` via wasm-bindgen |
| Time | `std::time` | `js_sys::Date` |
