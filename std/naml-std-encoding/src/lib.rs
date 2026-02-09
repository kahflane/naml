///
/// naml-std-encoding - Encoding/Decoding Operations
///
/// This module provides encoding and decoding functions for common formats:
/// - utf8: String <-> bytes UTF-8 conversion
/// - hex: Bytes <-> hex string conversion
/// - base64: Bytes <-> base64 string conversion
/// - url: URL percent-encoding/decoding
/// - json: JSON parsing and serialization
///
/// All decode functions can throw DecodeError on invalid input.
///

pub mod utf8;
pub mod hex;
pub mod base64;
pub mod url;
pub mod json;
pub mod toml;
pub mod yaml;

pub use utf8::*;
pub use hex::*;
pub use base64::*;
pub use url::*;
pub use json::*;
pub use toml::*;
pub use yaml::*;

use naml_std_core::value::NamlString;

/// Create a DecodeError exception struct
/// Returns a pointer to a struct with: message (string), position (int)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_decode_error_new(
    message: *const NamlString,
    position: i64,
) -> *mut naml_std_core::NamlStruct {
    unsafe {
        let exc = naml_std_core::naml_struct_new(0xFFFF_0003, 2);
        naml_std_core::naml_struct_set_field(exc, 0, message as i64);
        naml_std_core::naml_struct_set_field(exc, 1, position);
        exc
    }
}
