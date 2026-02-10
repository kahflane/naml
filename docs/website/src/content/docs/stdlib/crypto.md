---
title: "std::crypto"
description: Cryptographic hashing, HMAC, key derivation, and secure random bytes
---

Cryptographic primitives built on RustCrypto. Native platform only.

## Import

```naml
use std::crypto::*;
```

## Hash Functions

All hash functions accept `bytes` input and return either raw `bytes` or a lowercase hex `string`.

### md5

```naml
fn md5(data: bytes) -> bytes
fn md5_hex(data: bytes) -> string
```

**Example:**

```naml
var hash: string = md5_hex("hello world" as bytes);
// "5eb63bbbe01eeed093cb22bb8f5acdc3"
```

### sha1

```naml
fn sha1(data: bytes) -> bytes
fn sha1_hex(data: bytes) -> string
```

**Example:**

```naml
var hash: string = sha1_hex("hello world" as bytes);
// "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed"
```

### sha256

```naml
fn sha256(data: bytes) -> bytes
fn sha256_hex(data: bytes) -> string
```

**Example:**

```naml
var hash: string = sha256_hex("hello world" as bytes);
// "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
```

### sha512

```naml
fn sha512(data: bytes) -> bytes
fn sha512_hex(data: bytes) -> string
```

**Example:**

```naml
var hash: string = sha512_hex("hello world" as bytes);
```

## HMAC

Message authentication codes using HMAC-SHA256 and HMAC-SHA512. Verification uses constant-time comparison.

### hmac_sha256

```naml
fn hmac_sha256(key: bytes, data: bytes) -> bytes
fn hmac_sha256_hex(key: bytes, data: bytes) -> string
```

**Example:**

```naml
var key: bytes = "secret-key" as bytes;
var data: bytes = "message to authenticate" as bytes;
var mac: string = hmac_sha256_hex(key, data);
```

### hmac_sha512

```naml
fn hmac_sha512(key: bytes, data: bytes) -> bytes
fn hmac_sha512_hex(key: bytes, data: bytes) -> string
```

### hmac_verify_sha256

Verify HMAC-SHA256 with constant-time comparison. Returns `true` if valid.

```naml
fn hmac_verify_sha256(key: bytes, data: bytes, mac: bytes) -> bool
```

**Example:**

```naml
var key: bytes = "secret" as bytes;
var data: bytes = "amount=100&to=alice" as bytes;

var mac: bytes = hmac_sha256(key, data);
var valid: bool = hmac_verify_sha256(key, data, mac);  // true

var tampered: bytes = "amount=999&to=alice" as bytes;
var forged: bool = hmac_verify_sha256(key, tampered, mac);  // false
```

### hmac_verify_sha512

```naml
fn hmac_verify_sha512(key: bytes, data: bytes, mac: bytes) -> bool
```

## Key Derivation

### pbkdf2_sha256

Derive a key from a password using PBKDF2 with HMAC-SHA256.

```naml
fn pbkdf2_sha256(password: bytes, salt: bytes, iterations: int, key_len: int) -> bytes
```

| Param | Type | Description |
|-------|------|-------------|
| password | bytes | The password to derive from |
| salt | bytes | Salt value (should be unique per user) |
| iterations | int | Number of iterations (higher = slower + more secure) |
| key_len | int | Desired output key length in bytes |

**Example:**

```naml
var password: bytes = "correct horse battery staple" as bytes;
var salt: bytes = random_bytes(16);
var key: bytes = pbkdf2_sha256(password, salt, 100000, 32);
```

## Secure Random

### random_bytes

Generate cryptographically secure random bytes using OS entropy.

```naml
fn random_bytes(n: int) -> bytes
```

**Example:**

```naml
var token: bytes = random_bytes(32);
var nonce: bytes = random_bytes(16);
```

## Full Example

HMAC-signed API request with verification:

```naml
use std::crypto::*;
use std::encoding::hex::encode;

fn sign_request(key: bytes, payload: bytes) -> bytes {
    return hmac_sha256(key, payload);
}

fn verify_request(key: bytes, payload: bytes, signature: bytes) -> bool {
    return hmac_verify_sha256(key, payload, signature);
}

fn main() {
    var api_key: bytes = random_bytes(32);
    var payload: bytes = "user=alice&action=transfer&amount=100" as bytes;

    var signature: bytes = sign_request(api_key, payload);
    println(fmt("Signature: {}", hmac_sha256_hex(api_key, payload)));

    var valid: bool = verify_request(api_key, payload, signature);
    println(fmt("Valid: {}", valid));  // true

    var tampered: bytes = "user=alice&action=transfer&amount=999" as bytes;
    var forged: bool = verify_request(api_key, tampered, signature);
    println(fmt("Tampered: {}", forged));  // false
}
```
