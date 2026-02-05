///
/// naml-std-testing - Testing and Assertion Utilities
///
/// Provides assertion functions for testing naml programs:
///
/// ## Core Assertions (Issue #141)
/// - `assert(condition, message)` - Panics if condition is false
/// - `assert_eq(actual, expected, message)` - Panics if two ints are not equal
/// - `assert_eq_float(actual, expected, message)` - Panics if two floats are not equal
/// - `assert_eq_string(actual, expected, message)` - Panics if two strings are not equal
/// - `assert_eq_bool(actual, expected, message)` - Panics if two bools are not equal
/// - `assert_neq(actual, expected, message)` - Panics if two ints are equal
/// - `assert_neq_string(actual, expected, message)` - Panics if two strings are equal
/// - `assert_true(condition, message)` - Panics if not true
/// - `assert_false(condition, message)` - Panics if not false
/// - `assert_gt(actual, expected, message)` - Panics if actual <= expected
/// - `assert_gte(actual, expected, message)` - Panics if actual < expected
/// - `assert_lt(actual, expected, message)` - Panics if actual >= expected
/// - `assert_lte(actual, expected, message)` - Panics if actual > expected
/// - `fail(message)` - Unconditionally panics
///
/// ## Float & String Assertions (Issue #142)
/// - `assert_approx(actual, expected, epsilon, message)` - Float approximate comparison
/// - `assert_contains(haystack, needle, message)` - String contains substring
/// - `assert_starts_with(value, prefix, message)` - String starts with prefix
/// - `assert_ends_with(value, suffix, message)` - String ends with suffix
///

use naml_std_core::NamlString;

unsafe fn string_from_naml(s: *const NamlString) -> String {
    if s.is_null() {
        return String::new();
    }
    unsafe {
        let slice = std::slice::from_raw_parts((*s).data.as_ptr(), (*s).len);
        String::from_utf8_lossy(slice).into_owned()
    }
}

fn assertion_fail(name: &str, detail: &str, message: &str) -> ! {
    eprintln!("Assertion failed [{}]: {}. {}", name, detail, message);
    std::process::exit(1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_testing_assert(condition: i64, message: *const NamlString) {
    if condition == 0 {
        let msg = unsafe { string_from_naml(message) };
        assertion_fail("assert", "condition was false", &msg);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_testing_assert_eq(
    actual: i64,
    expected: i64,
    message: *const NamlString,
) {
    if actual != expected {
        let msg = unsafe { string_from_naml(message) };
        assertion_fail(
            "assert_eq",
            &format!("expected {}, got {}", expected, actual),
            &msg,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_testing_assert_eq_float(
    actual: f64,
    expected: f64,
    message: *const NamlString,
) {
    if actual != expected {
        let msg = unsafe { string_from_naml(message) };
        assertion_fail(
            "assert_eq_float",
            &format!("expected {}, got {}", expected, actual),
            &msg,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_testing_assert_eq_string(
    actual: *const NamlString,
    expected: *const NamlString,
    message: *const NamlString,
) {
    let actual_str = unsafe { string_from_naml(actual) };
    let expected_str = unsafe { string_from_naml(expected) };
    if actual_str != expected_str {
        let msg = unsafe { string_from_naml(message) };
        assertion_fail(
            "assert_eq_string",
            &format!("expected \"{}\", got \"{}\"", expected_str, actual_str),
            &msg,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_testing_assert_eq_bool(
    actual: i64,
    expected: i64,
    message: *const NamlString,
) {
    let a = actual != 0;
    let e = expected != 0;
    if a != e {
        let msg = unsafe { string_from_naml(message) };
        assertion_fail(
            "assert_eq_bool",
            &format!("expected {}, got {}", e, a),
            &msg,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_testing_assert_neq(
    actual: i64,
    expected: i64,
    message: *const NamlString,
) {
    if actual == expected {
        let msg = unsafe { string_from_naml(message) };
        assertion_fail(
            "assert_neq",
            &format!("expected values to differ, both are {}", actual),
            &msg,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_testing_assert_neq_string(
    actual: *const NamlString,
    expected: *const NamlString,
    message: *const NamlString,
) {
    let actual_str = unsafe { string_from_naml(actual) };
    let expected_str = unsafe { string_from_naml(expected) };
    if actual_str == expected_str {
        let msg = unsafe { string_from_naml(message) };
        assertion_fail(
            "assert_neq_string",
            &format!("expected values to differ, both are \"{}\"", actual_str),
            &msg,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_testing_assert_true(condition: i64, message: *const NamlString) {
    if condition == 0 {
        let msg = unsafe { string_from_naml(message) };
        assertion_fail("assert_true", "expected true, got false", &msg);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_testing_assert_false(condition: i64, message: *const NamlString) {
    if condition != 0 {
        let msg = unsafe { string_from_naml(message) };
        assertion_fail("assert_false", "expected false, got true", &msg);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_testing_assert_gt(
    actual: i64,
    expected: i64,
    message: *const NamlString,
) {
    if actual <= expected {
        let msg = unsafe { string_from_naml(message) };
        assertion_fail(
            "assert_gt",
            &format!("expected {} > {}", actual, expected),
            &msg,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_testing_assert_gte(
    actual: i64,
    expected: i64,
    message: *const NamlString,
) {
    if actual < expected {
        let msg = unsafe { string_from_naml(message) };
        assertion_fail(
            "assert_gte",
            &format!("expected {} >= {}", actual, expected),
            &msg,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_testing_assert_lt(
    actual: i64,
    expected: i64,
    message: *const NamlString,
) {
    if actual >= expected {
        let msg = unsafe { string_from_naml(message) };
        assertion_fail(
            "assert_lt",
            &format!("expected {} < {}", actual, expected),
            &msg,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_testing_assert_lte(
    actual: i64,
    expected: i64,
    message: *const NamlString,
) {
    if actual > expected {
        let msg = unsafe { string_from_naml(message) };
        assertion_fail(
            "assert_lte",
            &format!("expected {} <= {}", actual, expected),
            &msg,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_testing_fail(message: *const NamlString) {
    let msg = unsafe { string_from_naml(message) };
    assertion_fail("fail", "unconditional failure", &msg);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_testing_assert_approx(
    actual: f64,
    expected: f64,
    epsilon: f64,
    message: *const NamlString,
) {
    let diff = (actual - expected).abs();
    if diff > epsilon {
        let msg = unsafe { string_from_naml(message) };
        assertion_fail(
            "assert_approx",
            &format!(
                "expected {} ± {}, got {} (diff: {})",
                expected, epsilon, actual, diff
            ),
            &msg,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_testing_assert_contains(
    haystack: *const NamlString,
    needle: *const NamlString,
    message: *const NamlString,
) {
    let h = unsafe { string_from_naml(haystack) };
    let n = unsafe { string_from_naml(needle) };
    if !h.contains(&n) {
        let msg = unsafe { string_from_naml(message) };
        assertion_fail(
            "assert_contains",
            &format!("\"{}\" does not contain \"{}\"", h, n),
            &msg,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_testing_assert_starts_with(
    value: *const NamlString,
    prefix: *const NamlString,
    message: *const NamlString,
) {
    let v = unsafe { string_from_naml(value) };
    let p = unsafe { string_from_naml(prefix) };
    if !v.starts_with(&p) {
        let msg = unsafe { string_from_naml(message) };
        assertion_fail(
            "assert_starts_with",
            &format!("\"{}\" does not start with \"{}\"", v, p),
            &msg,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_testing_assert_ends_with(
    value: *const NamlString,
    suffix: *const NamlString,
    message: *const NamlString,
) {
    let v = unsafe { string_from_naml(value) };
    let s = unsafe { string_from_naml(suffix) };
    if !v.ends_with(&s) {
        let msg = unsafe { string_from_naml(message) };
        assertion_fail(
            "assert_ends_with",
            &format!("\"{}\" does not end with \"{}\"", v, s),
            &msg,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use naml_std_core::naml_string_new;

    unsafe fn make_str(s: &str) -> *mut NamlString {
        unsafe { naml_string_new(s.as_ptr(), s.len()) }
    }

    #[test]
    fn test_assert_passes() {
        unsafe { naml_testing_assert(1, make_str("should pass")) };
    }

    #[test]
    fn test_assert_eq_passes() {
        unsafe { naml_testing_assert_eq(42, 42, make_str("should pass")) };
    }

    #[test]
    fn test_assert_neq_passes() {
        unsafe { naml_testing_assert_neq(1, 2, make_str("should pass")) };
    }

    #[test]
    fn test_assert_true_passes() {
        unsafe { naml_testing_assert_true(1, make_str("should pass")) };
    }

    #[test]
    fn test_assert_false_passes() {
        unsafe { naml_testing_assert_false(0, make_str("should pass")) };
    }

    #[test]
    fn test_assert_gt_passes() {
        unsafe { naml_testing_assert_gt(5, 3, make_str("should pass")) };
    }

    #[test]
    fn test_assert_gte_passes() {
        unsafe { naml_testing_assert_gte(5, 5, make_str("should pass")) };
    }

    #[test]
    fn test_assert_lt_passes() {
        unsafe { naml_testing_assert_lt(3, 5, make_str("should pass")) };
    }

    #[test]
    fn test_assert_lte_passes() {
        unsafe { naml_testing_assert_lte(5, 5, make_str("should pass")) };
    }

    #[test]
    fn test_assert_eq_string_passes() {
        unsafe {
            naml_testing_assert_eq_string(make_str("hello"), make_str("hello"), make_str("ok"));
        }
    }

    #[test]
    fn test_assert_neq_string_passes() {
        unsafe {
            naml_testing_assert_neq_string(make_str("hello"), make_str("world"), make_str("ok"));
        }
    }

    #[test]
    fn test_assert_approx_passes() {
        unsafe {
            naml_testing_assert_approx(3.14159, 3.14, 0.01, make_str("pi approx"));
        }
    }

    #[test]
    fn test_assert_contains_passes() {
        unsafe {
            naml_testing_assert_contains(
                make_str("hello world"),
                make_str("world"),
                make_str("ok"),
            );
        }
    }

    #[test]
    fn test_assert_starts_with_passes() {
        unsafe {
            naml_testing_assert_starts_with(
                make_str("hello world"),
                make_str("hello"),
                make_str("ok"),
            );
        }
    }

    #[test]
    fn test_assert_ends_with_passes() {
        unsafe {
            naml_testing_assert_ends_with(
                make_str("hello world"),
                make_str("world"),
                make_str("ok"),
            );
        }
    }
}
