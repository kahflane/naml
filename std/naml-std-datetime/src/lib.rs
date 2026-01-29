//!
//! naml-std-datetime - Date and Time Utilities
//!
//! Provides time and date operations for naml programs.
//!
//! ## Functions
//!
//! - `now_ms() -> int` - Current Unix timestamp in milliseconds
//! - `now_s() -> int` - Current Unix timestamp in seconds
//! - `year(timestamp_ms: int) -> int` - Extract year from timestamp
//! - `month(timestamp_ms: int) -> int` - Extract month (1-12)
//! - `day(timestamp_ms: int) -> int` - Extract day of month (1-31)
//! - `hour(timestamp_ms: int) -> int` - Extract hour (0-23)
//! - `minute(timestamp_ms: int) -> int` - Extract minute (0-59)
//! - `second(timestamp_ms: int) -> int` - Extract second (0-59)
//! - `day_of_week(timestamp_ms: int) -> int` - Day of week (0=Sun, 6=Sat)
//! - `format_date(timestamp_ms: int, fmt: string) -> string` - Format timestamp
//!
//! ## Example
//!
//! ```naml
//! use std::datetime::*;
//!
//! fn main() {
//!     var ts: int = now_ms();
//!     println("Year: {}", year(ts));
//!     println("Date: {}", format_date(ts, "YYYY-MM-DD"));
//! }
//! ```
//!

use std::time::{SystemTime, UNIX_EPOCH};

/// Get current Unix timestamp in milliseconds
#[unsafe(no_mangle)]
pub extern "C" fn naml_datetime_now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Get current Unix timestamp in seconds
#[unsafe(no_mangle)]
pub extern "C" fn naml_datetime_now_s() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn timestamp_to_components(timestamp_ms: i64) -> (i32, u32, u32, u32, u32, u32) {
    let total_secs = timestamp_ms / 1000;
    let days_since_epoch = (total_secs / 86400) as i32;
    let time_of_day = (total_secs % 86400) as u32;

    let hour = time_of_day / 3600;
    let minute = (time_of_day % 3600) / 60;
    let second = time_of_day % 60;

    let (year, month, day) = days_to_ymd(days_since_epoch + 719468);

    (year, month, day, hour, minute, second)
}

fn days_to_ymd(days: i32) -> (i32, u32, u32) {
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = (days - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i32 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    (year, m, d)
}

/// Extract year from timestamp (milliseconds since Unix epoch)
#[unsafe(no_mangle)]
pub extern "C" fn naml_datetime_year(timestamp_ms: i64) -> i64 {
    let (year, _, _, _, _, _) = timestamp_to_components(timestamp_ms);
    year as i64
}

/// Extract month (1-12) from timestamp
#[unsafe(no_mangle)]
pub extern "C" fn naml_datetime_month(timestamp_ms: i64) -> i64 {
    let (_, month, _, _, _, _) = timestamp_to_components(timestamp_ms);
    month as i64
}

/// Extract day of month (1-31) from timestamp
#[unsafe(no_mangle)]
pub extern "C" fn naml_datetime_day(timestamp_ms: i64) -> i64 {
    let (_, _, day, _, _, _) = timestamp_to_components(timestamp_ms);
    day as i64
}

/// Extract hour (0-23) from timestamp
#[unsafe(no_mangle)]
pub extern "C" fn naml_datetime_hour(timestamp_ms: i64) -> i64 {
    let (_, _, _, hour, _, _) = timestamp_to_components(timestamp_ms);
    hour as i64
}

/// Extract minute (0-59) from timestamp
#[unsafe(no_mangle)]
pub extern "C" fn naml_datetime_minute(timestamp_ms: i64) -> i64 {
    let (_, _, _, _, minute, _) = timestamp_to_components(timestamp_ms);
    minute as i64
}

/// Extract second (0-59) from timestamp
#[unsafe(no_mangle)]
pub extern "C" fn naml_datetime_second(timestamp_ms: i64) -> i64 {
    let (_, _, _, _, _, second) = timestamp_to_components(timestamp_ms);
    second as i64
}

/// Get day of week (0=Sunday, 6=Saturday)
#[unsafe(no_mangle)]
pub extern "C" fn naml_datetime_day_of_week(timestamp_ms: i64) -> i64 {
    let days_since_epoch = timestamp_ms / 1000 / 86400;
    ((days_since_epoch + 4) % 7) as i64
}

/// Format timestamp to string using format specifiers
/// Supported: YYYY, MM, DD, HH, mm, ss
#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_datetime_format(
    timestamp_ms: i64,
    fmt: *const naml_std_core::NamlString,
) -> *mut naml_std_core::NamlString {
    let (year, month, day, hour, minute, second) = timestamp_to_components(timestamp_ms);

    let format_str = if fmt.is_null() {
        "YYYY-MM-DD HH:mm:ss"
    } else {
        unsafe { (*fmt).as_str() }
    };

    let result = format_str
        .replace("YYYY", &format!("{:04}", year))
        .replace("MM", &format!("{:02}", month))
        .replace("DD", &format!("{:02}", day))
        .replace("HH", &format!("{:02}", hour))
        .replace("mm", &format!("{:02}", minute))
        .replace("ss", &format!("{:02}", second));

    unsafe { naml_std_core::naml_string_new(result.as_ptr(), result.len()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_now_ms() {
        let ts = naml_datetime_now_ms();
        assert!(ts > 1700000000000);
    }

    #[test]
    fn test_now_s() {
        let ts = naml_datetime_now_s();
        assert!(ts > 1700000000);
    }

    #[test]
    fn test_epoch() {
        let ts = 0i64;
        assert_eq!(naml_datetime_year(ts), 1970);
        assert_eq!(naml_datetime_month(ts), 1);
        assert_eq!(naml_datetime_day(ts), 1);
        assert_eq!(naml_datetime_hour(ts), 0);
        assert_eq!(naml_datetime_minute(ts), 0);
        assert_eq!(naml_datetime_second(ts), 0);
        assert_eq!(naml_datetime_day_of_week(ts), 4);
    }

    #[test]
    fn test_known_date() {
        let ts = 1704067200000i64;
        assert_eq!(naml_datetime_year(ts), 2024);
        assert_eq!(naml_datetime_month(ts), 1);
        assert_eq!(naml_datetime_day(ts), 1);
        assert_eq!(naml_datetime_day_of_week(ts), 1);
    }

    #[test]
    fn test_format() {
        let ts = 1704067200000i64;
        unsafe {
            let fmt = naml_std_core::naml_string_new(b"YYYY-MM-DD".as_ptr(), 10);
            let result = naml_datetime_format(ts, fmt);
            assert_eq!((*result).as_str(), "2024-01-01");
            naml_std_core::naml_string_decref(fmt);
            naml_std_core::naml_string_decref(result);
        }
    }
}
