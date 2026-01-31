use crate::codegen::CodegenError;

pub fn convert_cranelift_error(panic_msg: &str, func_name: &str) -> CodegenError {
    // Parse common Cranelift error patterns and convert to user-friendly messages
    if panic_msg.contains("declared type of variable")
        && panic_msg.contains("doesn't match type of value")
    {
        CodegenError::JitCompile(format!(
            "Type mismatch in function '{}': a variable was assigned a value of incompatible type. \
             This usually indicates a type error that wasn't caught during type checking.",
            func_name
        ))
    } else if panic_msg.contains("block") && panic_msg.contains("not sealed") {
        CodegenError::JitCompile(format!(
            "Internal compiler error in function '{}': control flow issue. Please report this bug.",
            func_name
        ))
    } else if panic_msg.contains("undefined value") || panic_msg.contains("undefined variable") {
        CodegenError::JitCompile(format!(
            "Internal compiler error in function '{}': variable used before definition. Please report this bug.",
            func_name
        ))
    } else if panic_msg.contains("signature") {
        CodegenError::JitCompile(format!(
            "Function signature mismatch in '{}': the function was called with incorrect argument types.",
            func_name
        ))
    } else {
        // Generic fallback - sanitize internal terms
        let sanitized = panic_msg
            .replace("var", "variable ")
            .replace("v0", "value")
            .replace("v1", "value")
            .replace("RUST_BACKTRACE", "debug trace");
        CodegenError::JitCompile(format!(
            "Compilation error in function '{}': {}",
            func_name, sanitized
        ))
    }
}
