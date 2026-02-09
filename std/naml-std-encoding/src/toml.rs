///
/// std::encoding::toml - TOML Encoding/Decoding
///
/// Provides TOML parsing and serialization using the `json` type as the
/// in-memory representation. This allows TOML data to be queried using
/// the existing `std::encoding::json` query functions (path, keys, etc.).
///
/// - decode(s: string) -> json throws DecodeError: Parse TOML string into json
/// - encode(value: json) -> string: Serialize json to compact TOML
/// - encode_pretty(value: json) -> string: Serialize json to pretty TOML
///

use crate::json::{NamlJson, create_json};
use naml_std_core::value::NamlString;

/// Decode a TOML string into a NamlJson value.
/// The TOML is first parsed by the `toml` crate, then converted to
/// serde_json::Value so it can be stored as json and queried with json functions.
///
/// Returns via out parameters:
/// tag = 0: success, value = NamlJson pointer
/// tag = 1: error, value = error column position
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_toml_decode(
    s: *const NamlString,
    out_tag: *mut i32,
    out_value: *mut i64,
) {
    if s.is_null() {
        unsafe {
            *out_tag = 0;
            *out_value = create_json(serde_json::Value::Null) as i64;
        }
        return;
    }

    unsafe {
        let len = (*s).len;
        let data = std::slice::from_raw_parts((*s).data.as_ptr(), len);
        let toml_str = std::str::from_utf8_unchecked(data);

        match toml_str.parse::<toml::Value>() {
            Ok(toml_value) => {
                let json_value = toml_value_to_json(toml_value);
                *out_tag = 0;
                *out_value = create_json(json_value) as i64;
            }
            Err(e) => {
                *out_tag = 1;
                *out_value = e.span().map_or(0, |s| s.start) as i64;
            }
        }
    }
}

/// Encode a NamlJson value to a TOML string.
/// Returns via out parameters for error handling since not all JSON values
/// can be represented in TOML (e.g., null values, heterogeneous arrays).
///
/// tag = 0: success, value = NamlString pointer
/// tag = 1: error, value = 0
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_toml_encode(
    json: *const NamlJson,
    out_tag: *mut i32,
    out_value: *mut i64,
) {
    if json.is_null() {
        unsafe {
            *out_tag = 1;
            *out_value = 0;
        }
        return;
    }

    unsafe {
        let json_value = (*json).get_value();
        let toml_value = json_to_toml_value(json_value);

        match toml_value {
            Some(tv) => match toml::to_string(&tv) {
                Ok(s) => {
                    *out_tag = 0;
                    *out_value =
                        naml_std_core::value::naml_string_new(s.as_ptr(), s.len()) as i64;
                }
                Err(_) => {
                    *out_tag = 1;
                    *out_value = 0;
                }
            },
            None => {
                *out_tag = 1;
                *out_value = 0;
            }
        }
    }
}

/// Encode a NamlJson value to a pretty-printed TOML string.
/// Same error handling as encode.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_toml_encode_pretty(
    json: *const NamlJson,
    out_tag: *mut i32,
    out_value: *mut i64,
) {
    if json.is_null() {
        unsafe {
            *out_tag = 1;
            *out_value = 0;
        }
        return;
    }

    unsafe {
        let json_value = (*json).get_value();
        let toml_value = json_to_toml_value(json_value);

        match toml_value {
            Some(tv) => match toml::to_string_pretty(&tv) {
                Ok(s) => {
                    *out_tag = 0;
                    *out_value =
                        naml_std_core::value::naml_string_new(s.as_ptr(), s.len()) as i64;
                }
                Err(_) => {
                    *out_tag = 1;
                    *out_value = 0;
                }
            },
            None => {
                *out_tag = 1;
                *out_value = 0;
            }
        }
    }
}

/// Convert a toml::Value to serde_json::Value for storage as NamlJson
fn toml_value_to_json(value: toml::Value) -> serde_json::Value {
    match value {
        toml::Value::String(s) => serde_json::Value::String(s),
        toml::Value::Integer(i) => serde_json::Value::Number(serde_json::Number::from(i)),
        toml::Value::Float(f) => serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        toml::Value::Boolean(b) => serde_json::Value::Bool(b),
        toml::Value::Datetime(dt) => serde_json::Value::String(dt.to_string()),
        toml::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(toml_value_to_json).collect())
        }
        toml::Value::Table(table) => {
            let map: serde_json::Map<String, serde_json::Value> = table
                .into_iter()
                .map(|(k, v)| (k, toml_value_to_json(v)))
                .collect();
            serde_json::Value::Object(map)
        }
    }
}

/// Convert a serde_json::Value to toml::Value for serialization.
/// Returns None if the value cannot be represented in TOML (e.g., null).
fn json_to_toml_value(value: &serde_json::Value) -> Option<toml::Value> {
    match value {
        serde_json::Value::Null => None,
        serde_json::Value::Bool(b) => Some(toml::Value::Boolean(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(toml::Value::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Some(toml::Value::Float(f))
            } else {
                None
            }
        }
        serde_json::Value::String(s) => Some(toml::Value::String(s.clone())),
        serde_json::Value::Array(arr) => {
            let toml_arr: Vec<toml::Value> = arr.iter().filter_map(json_to_toml_value).collect();
            Some(toml::Value::Array(toml_arr))
        }
        serde_json::Value::Object(map) => {
            let mut toml_table = toml::map::Map::new();
            for (k, v) in map {
                if let Some(tv) = json_to_toml_value(v) {
                    toml_table.insert(k.clone(), tv);
                }
            }
            Some(toml::Value::Table(toml_table))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toml_decode_valid() {
        unsafe {
            let toml_str = r#"
[package]
name = "test"
version = "1.0"

[dependencies]
serde = "1.0"
"#;
            let s = naml_std_core::value::naml_string_new(toml_str.as_ptr(), toml_str.len());
            let mut tag: i32 = -1;
            let mut value: i64 = 0;
            naml_encoding_toml_decode(s, &mut tag, &mut value);
            assert_eq!(tag, 0);
            assert!(value != 0);
        }
    }

    #[test]
    fn test_toml_decode_invalid() {
        unsafe {
            let toml_str = r#"[invalid
name = "#;
            let s = naml_std_core::value::naml_string_new(toml_str.as_ptr(), toml_str.len());
            let mut tag: i32 = -1;
            let mut value: i64 = 0;
            naml_encoding_toml_decode(s, &mut tag, &mut value);
            assert_eq!(tag, 1);
        }
    }

    #[test]
    fn test_toml_roundtrip() {
        unsafe {
            let toml_str = "[package]\nname = \"test\"\nversion = \"1.0\"\n";
            let s = naml_std_core::value::naml_string_new(toml_str.as_ptr(), toml_str.len());
            let mut tag: i32 = -1;
            let mut value: i64 = 0;
            naml_encoding_toml_decode(s, &mut tag, &mut value);
            assert_eq!(tag, 0);

            let json = value as *const NamlJson;
            let mut enc_tag: i32 = -1;
            let mut enc_value: i64 = 0;
            naml_encoding_toml_encode(json, &mut enc_tag, &mut enc_value);
            assert_eq!(enc_tag, 0);
            assert!(enc_value != 0);
        }
    }
}
