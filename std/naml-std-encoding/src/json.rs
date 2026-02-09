///
/// std::encoding::json - JSON Encoding/Decoding
///
/// Provides JSON parsing, serialization, and querying with jq-like path navigation.
/// - decode(s: string) -> json throws DecodeError: Parse JSON string
/// - encode(value: json) -> string: Serialize to JSON
/// - encode_pretty(value: json) -> string: Serialize with indentation
/// - exists(data: json, key: string) -> bool: Check if key exists
/// - path(data: json, jq_path: string) -> json throws PathError: jq-style navigation
/// - keys(data: json) -> [string]: Get object keys
/// - count(data: json) -> int: Get array length or object key count
/// - get_type(data: json) -> int: Get JSON value type discriminant
///
/// JSON type discriminants:
/// - 0: null
/// - 1: bool
/// - 2: number
/// - 3: string
/// - 4: array
/// - 5: object
///

use naml_std_core::value::NamlString;
use naml_std_core::{HeapHeader, HeapTag, NamlStruct};
use serde_json::Value;
use std::alloc::Layout;

/// JSON type discriminants for subtype checking
pub const JSON_TYPE_NULL: i64 = 0;
pub const JSON_TYPE_BOOL: i64 = 1;
pub const JSON_TYPE_NUMBER: i64 = 2;
pub const JSON_TYPE_STRING: i64 = 3;
pub const JSON_TYPE_ARRAY: i64 = 4;
pub const JSON_TYPE_OBJECT: i64 = 5;

/// Runtime representation of a JSON value
/// Uses HeapTag::Custom with a specific marker for JSON values
#[repr(C)]
pub struct NamlJson {
    pub header: HeapHeader,
    value: Value,
}

impl NamlJson {
    pub fn get_value(&self) -> &Value {
        &self.value
    }

    pub fn get_value_mut(&mut self) -> &mut Value {
        &mut self.value
    }
}

/// Create a new NamlJson from a serde_json::Value
pub(crate) fn create_json(value: Value) -> *mut NamlJson {
    unsafe {
        let layout = Layout::new::<NamlJson>();
        let ptr = std::alloc::alloc(layout) as *mut NamlJson;
        std::ptr::write(
            ptr,
            NamlJson {
                header: HeapHeader::new(HeapTag::Json),
                value,
            },
        );
        ptr
    }
}

/// Create a null JSON value
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_json_null() -> *mut NamlJson {
    create_json(Value::Null)
}

/// Decode a JSON string into a NamlJson value
/// Returns via out parameters:
/// tag = 0: success, value = NamlJson pointer
/// tag = 1: error, value = position of parse error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_json_decode(
    s: *const NamlString,
    out_tag: *mut i32,
    out_value: *mut i64,
) {
    if s.is_null() {
        unsafe {
            *out_tag = 0;
            *out_value = create_json(Value::Null) as i64;
        }
        return;
    }

    unsafe {
        let len = (*s).len;
        let data = std::slice::from_raw_parts((*s).data.as_ptr(), len);
        let json_str = std::str::from_utf8_unchecked(data);

        match serde_json::from_str(json_str) {
            Ok(value) => {
                *out_tag = 0;
                *out_value = create_json(value) as i64;
            }
            Err(e) => {
                *out_tag = 1;
                *out_value = e.column() as i64;
            }
        }
    }
}

/// Encode a NamlJson value to a compact JSON string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_json_encode(json: *const NamlJson) -> *mut NamlString {
    if json.is_null() {
        return unsafe { naml_std_core::value::naml_string_new(b"null".as_ptr(), 4) };
    }

    unsafe {
        let value = &(*json).value;
        let json_string = serde_json::to_string(value).unwrap_or_else(|_| "null".to_string());
        naml_std_core::value::naml_string_new(json_string.as_ptr(), json_string.len())
    }
}

/// Encode a NamlJson value to a pretty-printed JSON string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_json_encode_pretty(json: *const NamlJson) -> *mut NamlString {
    if json.is_null() {
        return unsafe { naml_std_core::value::naml_string_new(b"null".as_ptr(), 4) };
    }

    unsafe {
        let value = &(*json).value;
        let json_string =
            serde_json::to_string_pretty(value).unwrap_or_else(|_| "null".to_string());
        naml_std_core::value::naml_string_new(json_string.as_ptr(), json_string.len())
    }
}

/// Check if a key exists in a JSON object
/// Returns 1 if key exists, 0 otherwise
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_json_exists(json: *const NamlJson, key: *const NamlString) -> i64 {
    if json.is_null() || key.is_null() {
        return 0;
    }

    unsafe {
        let value = &(*json).value;
        let key_len = (*key).len;
        let key_data = std::slice::from_raw_parts((*key).data.as_ptr(), key_len);
        let key_str = std::str::from_utf8_unchecked(key_data);

        match value {
            Value::Object(map) => {
                if map.contains_key(key_str) {
                    1
                } else {
                    0
                }
            }
            _ => 0,
        }
    }
}

/// Navigate JSON using a jq-like path expression
/// Supports: .key, [index], .key1.key2, .[0].name, etc.
/// Returns via out parameters:
/// tag = 0: success, value = NamlJson pointer
/// tag = 1: error (PathError)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_json_path(
    json: *const NamlJson,
    path_str: *const NamlString,
    out_tag: *mut i32,
    out_value: *mut i64,
) {
    if json.is_null() {
        unsafe {
            *out_tag = 0;
            *out_value = create_json(Value::Null) as i64;
        }
        return;
    }

    unsafe {
        let value = &(*json).value;

        if path_str.is_null() {
            *out_tag = 0;
            *out_value = create_json(value.clone()) as i64;
            return;
        }

        let path_len = (*path_str).len;
        let path_data = std::slice::from_raw_parts((*path_str).data.as_ptr(), path_len);
        let path = std::str::from_utf8_unchecked(path_data);

        match navigate_path(value, path) {
            Ok(result) => {
                *out_tag = 0;
                *out_value = create_json(result) as i64;
            }
            Err(_) => {
                *out_tag = 1;
                *out_value = 0;
            }
        }
    }
}

/// Navigate a JSON value using a jq-like path
fn navigate_path(value: &Value, path: &str) -> Result<Value, ()> {
    let path = path.trim();
    if path.is_empty() || path == "." {
        return Ok(value.clone());
    }

    let mut current = value.clone();
    let mut remaining = path;

    // Skip leading dot if present
    if remaining.starts_with('.') {
        remaining = &remaining[1..];
    }

    while !remaining.is_empty() {
        remaining = remaining.trim_start();

        if remaining.starts_with('[') {
            // Array index access
            let end = remaining.find(']').ok_or(())?;
            let index_str = &remaining[1..end];
            remaining = &remaining[end + 1..];

            // Skip any trailing dot
            if remaining.starts_with('.') {
                remaining = &remaining[1..];
            }

            let index: usize = index_str.parse().map_err(|_| ())?;
            current = current.get(index).cloned().unwrap_or(Value::Null);
        } else {
            // Object key access
            let key_end = remaining
                .find(|c: char| c == '.' || c == '[')
                .unwrap_or(remaining.len());
            let key = &remaining[..key_end];
            remaining = &remaining[key_end..];

            // Skip trailing dot
            if remaining.starts_with('.') {
                remaining = &remaining[1..];
            }

            if key.is_empty() {
                continue;
            }

            current = current.get(key).cloned().unwrap_or(Value::Null);
        }
    }

    Ok(current)
}

/// Get the keys of a JSON object as a naml array of strings
/// Returns null pointer if not an object
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_json_keys(json: *const NamlJson) -> *mut naml_std_core::NamlArray {
    use naml_std_core::NamlArray;

    if json.is_null() {
        return std::ptr::null_mut();
    }

    unsafe {
        let value = &(*json).value;

        match value {
            Value::Object(map) => {
                let keys: Vec<_> = map
                    .keys()
                    .map(|k| naml_std_core::value::naml_string_new(k.as_ptr(), k.len()) as i64)
                    .collect();

                let arr = naml_std_core::array::naml_array_new(keys.len());
                for (i, key_ptr) in keys.into_iter().enumerate() {
                    naml_std_core::array::naml_array_set(arr, i as i64, key_ptr);
                }
                arr as *mut NamlArray
            }
            _ => std::ptr::null_mut(),
        }
    }
}

/// Get the count of elements (array length or object key count)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_json_count(json: *const NamlJson) -> i64 {
    if json.is_null() {
        return 0;
    }

    unsafe {
        let value = &(*json).value;

        match value {
            Value::Array(arr) => arr.len() as i64,
            Value::Object(map) => map.len() as i64,
            _ => 0,
        }
    }
}

/// Get the type discriminant of a JSON value
/// Returns: 0=null, 1=bool, 2=number, 3=string, 4=array, 5=object
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_json_get_type(json: *const NamlJson) -> i64 {
    if json.is_null() {
        return JSON_TYPE_NULL;
    }

    unsafe {
        let value = &(*json).value;

        match value {
            Value::Null => JSON_TYPE_NULL,
            Value::Bool(_) => JSON_TYPE_BOOL,
            Value::Number(_) => JSON_TYPE_NUMBER,
            Value::String(_) => JSON_TYPE_STRING,
            Value::Array(_) => JSON_TYPE_ARRAY,
            Value::Object(_) => JSON_TYPE_OBJECT,
        }
    }
}

/// Get the type of a JSON value as a human-readable string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_json_type_name(json: *const NamlJson) -> *mut NamlString {
    let type_str = if json.is_null() {
        "null"
    } else {
        unsafe {
            let value = &(*json).value;
            match value {
                Value::Null => "null",
                Value::Bool(_) => "boolean",
                Value::Number(_) => "number",
                Value::String(_) => "string",
                Value::Array(_) => "array",
                Value::Object(_) => "object",
            }
        }
    };

    unsafe { naml_std_core::value::naml_string_new(type_str.as_ptr(), type_str.len()) }
}

/// Index into a JSON value by string key (for objects)
/// Returns a new NamlJson pointer or null json if key doesn't exist
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_json_index_string(
    json: *const NamlJson,
    key: *const NamlString,
) -> *mut NamlJson {
    if json.is_null() || key.is_null() {
        return create_json(Value::Null);
    }

    unsafe {
        let value = &(*json).value;
        let key_len = (*key).len;
        let key_data = std::slice::from_raw_parts((*key).data.as_ptr(), key_len);
        let key_str = std::str::from_utf8_unchecked(key_data);

        let result = value.get(key_str).cloned().unwrap_or(Value::Null);
        create_json(result)
    }
}

/// Index into a JSON value by integer index (for arrays)
/// Returns a new NamlJson pointer or null json if index out of bounds
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_json_index_int(json: *const NamlJson, index: i64) -> *mut NamlJson {
    if json.is_null() || index < 0 {
        return create_json(Value::Null);
    }

    unsafe {
        let value = &(*json).value;
        let result = value.get(index as usize).cloned().unwrap_or(Value::Null);
        create_json(result)
    }
}

/// Cast JSON to int (returns value and success flag)
/// out_value: the int value
/// Returns: 1 if successful cast, 0 if not a number or not an integer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_json_as_int(json: *const NamlJson, out_value: *mut i64) -> i64 {
    if json.is_null() {
        return 0;
    }

    unsafe {
        let value = &(*json).value;

        match value {
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    *out_value = i;
                    1
                } else if let Some(f) = n.as_f64() {
                    *out_value = f as i64;
                    1
                } else {
                    0
                }
            }
            Value::Bool(b) => {
                *out_value = if *b { 1 } else { 0 };
                1
            }
            _ => 0,
        }
    }
}

/// Cast JSON to float (returns value and success flag)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_json_as_float(json: *const NamlJson, out_value: *mut f64) -> i64 {
    if json.is_null() {
        return 0;
    }

    unsafe {
        let value = &(*json).value;

        match value {
            Value::Number(n) => {
                if let Some(f) = n.as_f64() {
                    *out_value = f;
                    1
                } else {
                    0
                }
            }
            _ => 0,
        }
    }
}

/// Cast JSON to bool (returns value and success flag)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_json_as_bool(json: *const NamlJson, out_value: *mut i64) -> i64 {
    if json.is_null() {
        return 0;
    }

    unsafe {
        let value = &(*json).value;

        match value {
            Value::Bool(b) => {
                *out_value = if *b { 1 } else { 0 };
                1
            }
            _ => 0,
        }
    }
}

/// Cast JSON to string (returns NamlString pointer, or null if not a string)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_json_as_string(json: *const NamlJson) -> *mut NamlString {
    if json.is_null() {
        return std::ptr::null_mut();
    }

    unsafe {
        let value = &(*json).value;

        match value {
            Value::String(s) => naml_std_core::value::naml_string_new(s.as_ptr(), s.len()),
            _ => std::ptr::null_mut(),
        }
    }
}

/// Check if JSON value is null
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_json_is_null(json: *const NamlJson) -> i64 {
    if json.is_null() {
        return 1;
    }

    unsafe {
        if (*json).value.is_null() {
            1
        } else {
            0
        }
    }
}

/// Create PathError exception struct
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_path_error_new(path: *const NamlString) -> *mut NamlStruct {
    unsafe {
        let exc = naml_std_core::naml_struct_new(0xFFFF_0004, 1);
        naml_std_core::naml_struct_set_field(exc, 0, path as i64);
        exc
    }
}

/// Create EncodeError exception struct
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_encode_error_new(reason: *const NamlString) -> *mut NamlStruct {
    unsafe {
        let exc = naml_std_core::naml_struct_new(0xFFFF_000B, 1);
        naml_std_core::naml_struct_set_field(exc, 0, reason as i64);
        exc
    }
}

/// Create a JSON value from an int
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_json_from_int(value: i64) -> *mut NamlJson {
    create_json(Value::Number(serde_json::Number::from(value)))
}

/// Create a JSON value from a float
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_json_from_float(value: f64) -> *mut NamlJson {
    create_json(
        serde_json::Number::from_f64(value)
            .map(Value::Number)
            .unwrap_or(Value::Null),
    )
}

/// Create a JSON value from a bool
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_json_from_bool(value: i64) -> *mut NamlJson {
    create_json(Value::Bool(value != 0))
}

/// Create a JSON value from a string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_json_from_string(s: *const NamlString) -> *mut NamlJson {
    if s.is_null() {
        return create_json(Value::Null);
    }

    unsafe {
        let len = (*s).len;
        let data = std::slice::from_raw_parts((*s).data.as_ptr(), len);
        let string = std::str::from_utf8_unchecked(data).to_string();
        create_json(Value::String(string))
    }
}

/// Create an empty JSON array
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_json_array_new() -> *mut NamlJson {
    create_json(Value::Array(Vec::new()))
}

/// Push a value onto a JSON array
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_json_array_push(arr: *mut NamlJson, value: *const NamlJson) {
    if arr.is_null() {
        return;
    }

    unsafe {
        if let Value::Array(ref mut vec) = (*arr).value {
            let val = if value.is_null() {
                Value::Null
            } else {
                (*value).value.clone()
            };
            vec.push(val);
        }
    }
}

/// Create an empty JSON object
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_json_object_new() -> *mut NamlJson {
    create_json(Value::Object(serde_json::Map::new()))
}

/// Set a key-value pair on a JSON object
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_json_object_set(
    obj: *mut NamlJson,
    key: *const NamlString,
    value: *const NamlJson,
) {
    if obj.is_null() || key.is_null() {
        return;
    }

    unsafe {
        if let Value::Object(ref mut map) = (*obj).value {
            let key_len = (*key).len;
            let key_data = std::slice::from_raw_parts((*key).data.as_ptr(), key_len);
            let key_str = std::str::from_utf8_unchecked(key_data).to_string();

            let val = if value.is_null() {
                Value::Null
            } else {
                (*value).value.clone()
            };
            map.insert(key_str, val);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_decode_valid() {
        unsafe {
            let json_str = r#"{"name": "test", "value": 42}"#;
            let s = naml_std_core::value::naml_string_new(json_str.as_ptr(), json_str.len());
            let mut tag: i32 = -1;
            let mut value: i64 = 0;
            naml_json_decode(s, &mut tag, &mut value);
            assert_eq!(tag, 0);
            assert!(value != 0);

            let json = value as *const NamlJson;
            assert_eq!(naml_json_get_type(json), JSON_TYPE_OBJECT);
        }
    }

    #[test]
    fn test_json_decode_invalid() {
        unsafe {
            let json_str = r#"{"name": invalid}"#;
            let s = naml_std_core::value::naml_string_new(json_str.as_ptr(), json_str.len());
            let mut tag: i32 = -1;
            let mut value: i64 = 0;
            naml_json_decode(s, &mut tag, &mut value);
            assert_eq!(tag, 1);
        }
    }

    #[test]
    fn test_json_encode() {
        unsafe {
            let json = create_json(serde_json::json!({"name": "test"}));
            let result = naml_json_encode(json);
            let s = std::slice::from_raw_parts((*result).data.as_ptr(), (*result).len);
            assert_eq!(s, br#"{"name":"test"}"#);
        }
    }

    #[test]
    fn test_json_path() {
        unsafe {
            let json = create_json(serde_json::json!({
                "users": [
                    {"name": "Alice", "age": 30},
                    {"name": "Bob", "age": 25}
                ]
            }));

            let path = ".users[0].name";
            let path_str = naml_std_core::value::naml_string_new(path.as_ptr(), path.len());
            let mut tag: i32 = -1;
            let mut value: i64 = 0;
            naml_json_path(json, path_str, &mut tag, &mut value);
            assert_eq!(tag, 0);

            let result = value as *const NamlJson;
            let result_str = naml_json_as_string(result);
            assert!(!result_str.is_null());
            let s = std::slice::from_raw_parts((*result_str).data.as_ptr(), (*result_str).len);
            assert_eq!(s, b"Alice");
        }
    }

    #[test]
    fn test_json_index() {
        unsafe {
            let json = create_json(serde_json::json!({"name": "test", "items": [1, 2, 3]}));

            // String index
            let key = "name";
            let key_str = naml_std_core::value::naml_string_new(key.as_ptr(), key.len());
            let result = naml_json_index_string(json, key_str);
            assert_eq!(naml_json_get_type(result), JSON_TYPE_STRING);

            // Array access
            let items_key = "items";
            let items_str = naml_std_core::value::naml_string_new(items_key.as_ptr(), items_key.len());
            let items = naml_json_index_string(json, items_str);
            let first = naml_json_index_int(items, 0);
            let mut val: i64 = 0;
            assert_eq!(naml_json_as_int(first, &mut val), 1);
            assert_eq!(val, 1);
        }
    }

    #[test]
    fn test_json_types() {
        unsafe {
            assert_eq!(naml_json_get_type(create_json(Value::Null)), JSON_TYPE_NULL);
            assert_eq!(
                naml_json_get_type(create_json(Value::Bool(true))),
                JSON_TYPE_BOOL
            );
            assert_eq!(
                naml_json_get_type(create_json(serde_json::json!(42))),
                JSON_TYPE_NUMBER
            );
            assert_eq!(
                naml_json_get_type(create_json(Value::String("test".to_string()))),
                JSON_TYPE_STRING
            );
            assert_eq!(
                naml_json_get_type(create_json(serde_json::json!([1, 2, 3]))),
                JSON_TYPE_ARRAY
            );
            assert_eq!(
                naml_json_get_type(create_json(serde_json::json!({"a": 1}))),
                JSON_TYPE_OBJECT
            );
        }
    }
}
