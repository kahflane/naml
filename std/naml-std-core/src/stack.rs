use crate::array::{naml_array_new, naml_array_push};
///
/// naml-std-core/stack.rs - Stack Frame Type and Shadow Stack Runtime
///
/// Provides:
/// - `StackFrame` struct representing a single frame in a stack trace
/// - Shadow stack runtime for capturing stack traces at throw time
/// - Functions for stack manipulation and formatting
///
use crate::value::NamlString;

#[repr(C)]
pub struct Stack {
    pub depth: usize,
    pub frames: [StackFrame; 1024],
}

/// Represents a single frame in a stack trace.
/// This is a built-in type in naml accessible as `stack_frame`.
#[repr(C)]
pub struct StackFrame {
    pub function: *const u8, // Raw pointer to function name (static literal)
    pub file: *const u8,     // Raw pointer to file path (static literal)
    pub line: i64,           // Line number
}

// Global shadow stack (exposed for inlining in codegen)
#[unsafe(no_mangle)]
pub static mut NAML_SHADOW_STACK: Stack = Stack {
    frames: [const {
        StackFrame {
            function: std::ptr::null(),
            file: std::ptr::null(),
            line: 0,
        }
    }; 1024],
    depth: 0,
};

/// Push a frame onto the shadow stack (called at function entry)
#[unsafe(no_mangle)]
pub extern "C" fn naml_stack_push(func_name: *const u8, file: *const u8, line: i64) {
    unsafe {
        let d = NAML_SHADOW_STACK.depth;
        if d < 1024 {
            let frame = &mut NAML_SHADOW_STACK.frames[d];
            frame.function = func_name;
            frame.file = file;
            frame.line = line;
            NAML_SHADOW_STACK.depth = d + 1;
        }
    }
}

/// Pop a frame from the shadow stack (called at function exit)
#[unsafe(no_mangle)]
pub extern "C" fn naml_stack_pop() {
    unsafe {
        if NAML_SHADOW_STACK.depth > 0 {
            NAML_SHADOW_STACK.depth -= 1;
        }
    }
}

/// Capture current stack as a naml array of stack_frame
/// Returns pointer to [stack_frame] array
#[unsafe(no_mangle)]
pub extern "C" fn naml_stack_capture() -> *mut u8 {
    unsafe {
        let array = naml_array_new(NAML_SHADOW_STACK.depth);

        // Copy frames in reverse order (most recent first)
        for i in (0..NAML_SHADOW_STACK.depth).rev() {
            let frame = &NAML_SHADOW_STACK.frames[i];
            // Allocate a copy of the frame
            let frame_ptr = {
                let layout = std::alloc::Layout::from_size_align(24, 8).unwrap();
                let ptr = std::alloc::alloc(layout) as *mut StackFrame;
                (*ptr).function = frame.function;
                (*ptr).file = frame.file;
                (*ptr).line = frame.line;
                ptr as i64
            };
            naml_array_push(array, frame_ptr);
        }

        array as *mut u8
    }
}

/// Clear the stack (called on thread init or after unhandled exception)
#[unsafe(no_mangle)]
pub extern "C" fn naml_stack_clear() {
    unsafe {
        NAML_SHADOW_STACK.depth = 0;
    }
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
                    let c_str = std::ffi::CStr::from_ptr((*frame_ptr).function as *const std::ffi::c_char);
                    c_str.to_string_lossy()
                } else {
                    std::borrow::Cow::Borrowed("<unknown>")
                };

                let file = if !(*frame_ptr).file.is_null() {
                    let c_str = std::ffi::CStr::from_ptr((*frame_ptr).file as *const std::ffi::c_char);
                    c_str.to_string_lossy()
                } else {
                    std::borrow::Cow::Borrowed("<unknown>")
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
