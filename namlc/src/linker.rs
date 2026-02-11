///
/// Linker Module
///
/// Handles linking of AOT-compiled naml object files with the runtime
/// static library to produce standalone native executables.
///
/// Invokes the system C compiler (cc) as the linker driver with
/// platform-specific flags for required system libraries.
///

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::codegen::CodegenError;

pub fn link(
    object_file: &Path,
    output: &Path,
    runtime_lib: &Path,
) -> Result<(), CodegenError> {
    let mut cmd = Command::new("cc");

    cmd.arg(object_file);

    if cfg!(target_os = "macos") {
        cmd.arg(format!("-Wl,-force_load,{}", runtime_lib.display()));
    } else {
        cmd.arg("-Wl,--whole-archive")
            .arg(runtime_lib)
            .arg("-Wl,--no-whole-archive");
    }

    if cfg!(target_os = "macos") {
        cmd.args(["-framework", "CoreFoundation"]);
        cmd.args(["-framework", "Security"]);
        cmd.args(["-framework", "SystemConfiguration"]);
        cmd.arg("-liconv");
    } else if cfg!(target_os = "linux") {
        cmd.args(["-lpthread", "-ldl", "-lm"]);
    }

    cmd.arg("-o").arg(output);

    let result = cmd.output().map_err(|e| {
        CodegenError::JitCompile(format!("Failed to invoke linker (cc): {}", e))
    })?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(CodegenError::JitCompile(format!("Linking failed:\n{}", stderr)));
    }

    Ok(())
}

pub fn find_runtime_lib() -> Result<PathBuf, CodegenError> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let lib = dir.join("libnaml_runtime.a");
            if lib.exists() {
                return Ok(lib);
            }

            if let Some(parent) = dir.parent() {
                let lib = parent.join("lib").join("libnaml_runtime.a");
                if lib.exists() {
                    return Ok(lib);
                }
            }
        }
    }

    Err(CodegenError::JitCompile(
        "Could not find libnaml_runtime.a. Build it with: cargo build -p naml-runtime".to_string(),
    ))
}
