///
/// naml-std-core/stack.rs - Stack Frame Type and Shadow Stack Runtime
///
/// Provides:
/// - `StackFrame` struct representing a single frame in a stack trace
/// - Shadow stack runtime for capturing stack traces at throw time
/// - Functions for stack manipulation and formatting
///

use crate::value::NamlString;
use crate::array::{naml_array_new, naml_array_push};
use std::cell::RefCell;

/// Represents a single frame in a stack trace.
/// This is a built-in type in naml accessible as `stack_frame`.
#[repr(C)]
pub struct StackFrame {
    pub function: *mut NamlString,  // Function name
    pub file: *mut NamlString,      // File path
    pub line: i64,                  // Line number
}

// Thread-local shadow stack that mirrors the call stack
thread_local! {
    static CALL_STACK: RefCell<Vec<StackFrame>> = RefCell::new(Vec::with_capacity(64));
}

/// Push a frame onto the shadow stack (called at function entry)
#[unsafe(no_mangle)]
pub extern "C" fn naml_stack_push(
    func_name: *mut NamlString,
    file: *mut NamlString,
    line: i64,
) {
    CALL_STACK.with(|stack| {
        stack.borrow_mut().push(StackFrame {
            function: func_name,
            file,
            line,
        });
    });
}

/// Pop a frame from the shadow stack (called at function exit)
#[unsafe(no_mangle)]
pub extern "C" fn naml_stack_pop() {
    CALL_STACK.with(|stack| {
        stack.borrow_mut().pop();
    });
}

/// Capture current stack as a naml array of stack_frame
/// Returns pointer to [stack_frame] array
#[unsafe(no_mangle)]
pub extern "C" fn naml_stack_capture() -> *mut u8 {
    CALL_STACK.with(|stack| {
        let frames = stack.borrow();
        let array = unsafe { naml_array_new(frames.len()) };

        // Copy frames in reverse order (most recent first)
        for frame in frames.iter().rev() {
            // Allocate a copy of the frame
            let frame_ptr = unsafe {
                let layout = std::alloc::Layout::from_size_align(24, 8).unwrap();
                let ptr = std::alloc::alloc(layout) as *mut StackFrame;
                (*ptr).function = frame.function;
                (*ptr).file = frame.file;
                (*ptr).line = frame.line;
                ptr as i64
            };
            unsafe { naml_array_push(array, frame_ptr) };
        }

        array as *mut u8
    })
}

/// Clear the stack (called on thread init or after unhandled exception)
#[unsafe(no_mangle)]
pub extern "C" fn naml_stack_clear() {
    CALL_STACK.with(|stack| {
        stack.borrow_mut().clear();
    });
}

/// Format stack trace as a string
/// Takes pointer to [stack_frame] array, returns NamlString pointer
#[unsafe(no_mangle)]
pub extern "C" fn naml_stack_format(stack_ptr: *mut u8) -> *mut NamlString {
    use crate::array::NamlArray;

    if stack_ptr.is_null() {
        let empty = b"Stack trace: (empty)\n";
        return unsafe { crate::value::naml_string_new(empty.as_ptr(), empty.len()) };
    }

    unsafe {
        let array = stack_ptr as *mut NamlArray;
        let len = (*array).len;

        let mut result = String::from("Stack trace:\n");

        for i in 0..len {
            let frame_ptr = *(*array).data.add(i) as *const StackFrame;
            if !frame_ptr.is_null() {
                let func = if !(*frame_ptr).function.is_null() {
                    let func_str = (*frame_ptr).function;
                    let data = (*func_str).data.as_ptr();
                    let len = (*func_str).len;
                    std::str::from_utf8_unchecked(std::slice::from_raw_parts(data, len))
                } else {
                    "<unknown>"
                };

                let file = if !(*frame_ptr).file.is_null() {
                    let file_str = (*frame_ptr).file;
                    let data = (*file_str).data.as_ptr();
                    let len = (*file_str).len;
                    std::str::from_utf8_unchecked(std::slice::from_raw_parts(data, len))
                } else {
                    "<unknown>"
                };

                let line = (*frame_ptr).line;
                result.push_str(&format!("  at {} ({}:{})\n", func, file, line));
            }
        }

        // Convert to NamlString
        let bytes = result.as_bytes();
        crate::value::naml_string_new(bytes.as_ptr(), bytes.len())
    }
}
