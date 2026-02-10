///
/// naml-std-crypto - Cryptographic Operations
///
/// Provides cryptographic primitives for naml programs using the RustCrypto ecosystem:
///
/// - **Hashing**: MD5, SHA-1, SHA-256, SHA-512 (raw bytes + hex string variants)
/// - **HMAC**: SHA-256 and SHA-512 message authentication with constant-time verify
/// - **KDF**: PBKDF2-SHA-256 key derivation
/// - **Random**: Cryptographically secure random byte generation
///
/// All functions operate on `NamlBytes` (raw binary) and `NamlString` (UTF-8 text).
/// Heap objects are reference-counted and follow naml's ownership model.
///

pub mod hash;
pub mod hmac_mod;
pub mod kdf;
pub mod random;

pub use hash::*;
pub use hmac_mod::*;
pub use kdf::*;
pub use random::*;
