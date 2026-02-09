///
/// std::encoding::yaml - YAML Encoding/Decoding
///
/// Provides YAML parsing and serialization using the `json` type as the
/// in-memory representation. This allows YAML data to be queried using
/// the existing `std::encoding::json` query functions (path, keys, etc.).
///
/// - decode(s: string) -> json throws DecodeError: Parse YAML string into json
/// - encode(value: json) -> string throws EncodeError: Serialize json to YAML
///

use crate::json::{NamlJson, create_json};
use naml_std_core::value::NamlString;

/// Decode a YAML string into a NamlJson value.
/// The YAML is parsed by serde_yaml, then converted to serde_json::Value
/// for storage as json.
///
/// Returns via out parameters:
/// tag = 0: success, value = NamlJson pointer
/// tag = 1: error, value = 0
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_yaml_decode(
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
        let yaml_str = std::str::from_utf8_unchecked(data);

        match serde_yaml::from_str::<serde_yaml::Value>(yaml_str) {
            Ok(yaml_value) => {
                let json_value = yaml_value_to_json(yaml_value);
                *out_tag = 0;
                *out_value = create_json(json_value) as i64;
            }
            Err(_) => {
                *out_tag = 1;
                *out_value = 0;
            }
        }
    }
}

/// Encode a NamlJson value to a YAML string.
/// Returns via out parameters for error handling.
///
/// tag = 0: success, value = NamlString pointer
/// tag = 1: error, value = 0
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encoding_yaml_encode(
    json: *const NamlJson,
    out_tag: *mut i32,
    out_value: *mut i64,
) {
    if json.is_null() {
        unsafe {
            let s = "null\n";
            *out_tag = 0;
            *out_value =
                naml_std_core::value::naml_string_new(s.as_ptr(), s.len()) as i64;
        }
        return;
    }

    unsafe {
        let json_value = (*json).get_value();
        let yaml_value = json_to_yaml_value(json_value);

        match serde_yaml::to_string(&yaml_value) {
            Ok(s) => {
                *out_tag = 0;
                *out_value =
                    naml_std_core::value::naml_string_new(s.as_ptr(), s.len()) as i64;
            }
            Err(_) => {
                *out_tag = 1;
                *out_value = 0;
            }
        }
    }
}

/// Convert a serde_yaml::Value to serde_json::Value for storage as NamlJson
fn yaml_value_to_json(value: serde_yaml::Value) -> serde_json::Value {
    match value {
        serde_yaml::Value::Null => serde_json::Value::Null,
        serde_yaml::Value::Bool(b) => serde_json::Value::Bool(b),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                serde_json::Value::Number(serde_json::Number::from(i))
            } else if let Some(u) = n.as_u64() {
                serde_json::Value::Number(serde_json::Number::from(u))
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::Null
            }
        }
        serde_yaml::Value::String(s) => serde_json::Value::String(s),
        serde_yaml::Value::Sequence(seq) => {
            serde_json::Value::Array(seq.into_iter().map(yaml_value_to_json).collect())
        }
        serde_yaml::Value::Mapping(map) => {
            let obj: serde_json::Map<String, serde_json::Value> = map
                .into_iter()
                .filter_map(|(k, v)| {
                    let key = match k {
                        serde_yaml::Value::String(s) => s,
                        serde_yaml::Value::Number(n) => n.to_string(),
                        serde_yaml::Value::Bool(b) => b.to_string(),
                        _ => return None,
                    };
                    Some((key, yaml_value_to_json(v)))
                })
                .collect();
            serde_json::Value::Object(obj)
        }
        serde_yaml::Value::Tagged(tagged) => yaml_value_to_json(tagged.value),
    }
}

/// Convert a serde_json::Value to serde_yaml::Value for serialization
fn json_to_yaml_value(value: &serde_json::Value) -> serde_yaml::Value {
    match value {
        serde_json::Value::Null => serde_yaml::Value::Null,
        serde_json::Value::Bool(b) => serde_yaml::Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                serde_yaml::Value::Number(serde_yaml::Number::from(i))
            } else if let Some(f) = n.as_f64() {
                serde_yaml::Value::Number(serde_yaml::Number::from(f))
            } else {
                serde_yaml::Value::Null
            }
        }
        serde_json::Value::String(s) => serde_yaml::Value::String(s.clone()),
        serde_json::Value::Array(arr) => {
            serde_yaml::Value::Sequence(arr.iter().map(json_to_yaml_value).collect())
        }
        serde_json::Value::Object(map) => {
            let yaml_map: serde_yaml::Mapping = map
                .iter()
                .map(|(k, v)| {
                    (
                        serde_yaml::Value::String(k.clone()),
                        json_to_yaml_value(v),
                    )
                })
                .collect();
            serde_yaml::Value::Mapping(yaml_map)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yaml_decode_valid() {
        unsafe {
            let yaml_str = "name: test\nversion: \"1.0\"\nitems:\n  - a\n  - b\n";
            let s = naml_std_core::value::naml_string_new(yaml_str.as_ptr(), yaml_str.len());
            let mut tag: i32 = -1;
            let mut value: i64 = 0;
            naml_encoding_yaml_decode(s, &mut tag, &mut value);
            assert_eq!(tag, 0);
            assert!(value != 0);
        }
    }

    #[test]
    fn test_yaml_decode_invalid() {
        unsafe {
            let yaml_str = ":\n  - :\n    - : [";
            let s = naml_std_core::value::naml_string_new(yaml_str.as_ptr(), yaml_str.len());
            let mut tag: i32 = -1;
            let mut value: i64 = 0;
            naml_encoding_yaml_decode(s, &mut tag, &mut value);
            assert_eq!(tag, 1);
        }
    }

    #[test]
    fn test_yaml_roundtrip() {
        unsafe {
            let yaml_str = "name: test\ncount: 42\n";
            let s = naml_std_core::value::naml_string_new(yaml_str.as_ptr(), yaml_str.len());
            let mut tag: i32 = -1;
            let mut value: i64 = 0;
            naml_encoding_yaml_decode(s, &mut tag, &mut value);
            assert_eq!(tag, 0);

            let json = value as *const NamlJson;
            let mut enc_tag: i32 = -1;
            let mut enc_value: i64 = 0;
            naml_encoding_yaml_encode(json, &mut enc_tag, &mut enc_value);
            assert_eq!(enc_tag, 0);
            assert!(enc_value != 0);
        }
    }

    #[test]
    fn test_yaml_null_handling() {
        unsafe {
            let yaml_str = "value: null\n";
            let s = naml_std_core::value::naml_string_new(yaml_str.as_ptr(), yaml_str.len());
            let mut tag: i32 = -1;
            let mut value: i64 = 0;
            naml_encoding_yaml_decode(s, &mut tag, &mut value);
            assert_eq!(tag, 0);

            let json = value as *const NamlJson;
            let mut enc_tag: i32 = -1;
            let mut enc_value: i64 = 0;
            naml_encoding_yaml_encode(json, &mut enc_tag, &mut enc_value);
            assert_eq!(enc_tag, 0);
        }
    }
}
