//!
//! Code Generation Module
//!
//! This module handles JIT compilation of naml AST using Cranelift.
//! The generated machine code is executed directly without transpilation.
//!
//! Pipeline:
//! 1. Convert AST to Cranelift IR
//! 2. JIT compile to native machine code
//! 3. Execute directly
//!

pub mod cranelift;

use lasso::Rodeo;
use thiserror::Error;

use crate::ast::SourceFile;
use crate::source::SourceFile as SourceInfo;
use crate::typechecker::{ImportedModule, TypeAnnotations};

#[derive(Debug, Error)]
pub enum CodegenError {
    #[error("Compilation failed: {0}")]
    JitCompile(String),

    #[error("Panic #> {0}")]
    Execution(String),

    #[error("Unsupported feature: {0}")]
    Unsupported(String),

    #[error("Type error: {0}")]
    TypeError(String),
}

pub fn compile_and_run(
    ast: &SourceFile<'_>,
    interner: &Rodeo,
    annotations: &TypeAnnotations,
    imported_modules: &[ImportedModule],
    source_info: &SourceInfo,
    release: bool,
    unsafe_mode: bool,
) -> Result<(), CodegenError> {
    let mut jit = cranelift::JitCompiler::new(interner, annotations, source_info, release, unsafe_mode)?;
    for module in imported_modules {
        jit.compile_module_source(&module.source_text)?;
    }
    jit.compile(ast)?;
    jit.run_main()
}

pub fn compile_to_object(
    ast: &SourceFile<'_>,
    interner: &Rodeo,
    annotations: &TypeAnnotations,
    imported_modules: &[ImportedModule],
    source_info: &SourceInfo,
    output: &std::path::Path,
    release: bool,
    unsafe_mode: bool,
) -> Result<(), CodegenError> {
    let mut compiler = cranelift::JitCompiler::new_aot(
        interner, annotations, source_info, release, unsafe_mode,
    )?;
    for module in imported_modules {
        compiler.compile_module_source(&module.source_text)?;
    }
    compiler.compile(ast)?;
    compiler.emit_object(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exists() {
        assert!(true);
    }

    #[test]
    fn test_aot_emits_object_file() {
        let source = "fn main() {\n    var x: int = 42;\n    println(x);\n}\n";
        let source_info = crate::source::SourceFile::new("test.nm".to_string(), source.to_string());
        let (tokens, mut interner) = crate::lexer::tokenize(source);
        let arena = crate::ast::AstArena::new();
        let parse_result = crate::parser::parse(&tokens, source, &arena);
        assert!(parse_result.errors.is_empty(), "parse errors");
        let type_result = crate::typechecker::check_with_types(
            &parse_result.ast,
            &mut interner,
            None,
            None,
        );
        assert!(type_result.errors.is_empty(), "type errors");

        let output = std::env::temp_dir().join("naml_test_aot.o");
        compile_to_object(
            &parse_result.ast,
            &interner,
            &type_result.annotations,
            &type_result.imported_modules,
            &source_info,
            &output,
            false,
            false,
        )
        .expect("AOT compilation failed");

        let metadata = std::fs::metadata(&output).expect("object file not created");
        assert!(metadata.len() > 100, "object file too small: {} bytes", metadata.len());
        std::fs::remove_file(&output).ok();
    }
}
