///
/// Print Runtime Functions
///
/// Provides low-level print functions for naml programs.
/// These are called by JIT-compiled code (via symbol registration)
/// and linked into AOT binaries (via libnaml_runtime.a).
///
/// Handles: int, float, bool, C-string, newline, and option<T> variants.
///

use crate::NamlString;

#[unsafe(no_mangle)]
pub extern "C" fn naml_print_int(val: i64) {
    print!("{}", val);
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_print_float(val: f64) {
    print!("{}", val);
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_print_bool(val: i64) {
    if val != 0 {
        print!("true");
    } else {
        print!("false");
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_print_str(ptr: *const std::ffi::c_char) {
    if !ptr.is_null() {
        let c_str = unsafe { std::ffi::CStr::from_ptr(ptr) };
        if let Ok(s) = c_str.to_str() {
            print!("{}", s);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn naml_print_newline() {
    println!();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_option_print_int(ptr: *const u8) {
    if ptr.is_null() { print!("none"); return; }
    unsafe {
        let tag = *(ptr as *const i32);
        if tag == 0 { print!("none"); }
        else {
            let val = *(ptr.add(8) as *const i64);
            print!("some({})", val);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_option_print_float(ptr: *const u8) {
    if ptr.is_null() { print!("none"); return; }
    unsafe {
        let tag = *(ptr as *const i32);
        if tag == 0 { print!("none"); }
        else {
            let val = *(ptr.add(8) as *const f64);
            print!("some({})", val);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_option_print_bool(ptr: *const u8) {
    if ptr.is_null() { print!("none"); return; }
    unsafe {
        let tag = *(ptr as *const i32);
        if tag == 0 { print!("none"); }
        else {
            let val = *ptr.add(8);
            if val != 0 { print!("some(true)"); } else { print!("some(false)"); }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_option_print_string(ptr: *const u8) {
    if ptr.is_null() { print!("none"); return; }
    unsafe {
        let tag = *(ptr as *const i32);
        if tag == 0 { print!("none"); }
        else {
            let str_ptr = *(ptr.add(8) as *const *const NamlString);
            if !str_ptr.is_null() {
                print!("some(\"{}\")", (*str_ptr).as_str());
            } else {
                print!("some(null)");
            }
        }
    }
}
