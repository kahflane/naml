///
/// Code Generation Module
///
/// This module handles JIT compilation of naml AST using Cranelift.
/// The generated machine code is executed directly without transpilation.
///
/// Pipeline:
/// 1. Convert AST to Cranelift IR
/// 2. JIT compile to native machine code
/// 3. Execute directly
///

pub mod cranelift;

use lasso::Rodeo;
use thiserror::Error;

use crate::ast::SourceFile;
use crate::typechecker::{SymbolTable, TypeAnnotations};

#[derive(Debug, Error)]
pub enum CodegenError {
    #[error("JIT compilation failed: {0}")]
    JitCompile(String),

    #[error("Execution failed: {0}")]
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
    symbols: &SymbolTable,
) -> Result<(), CodegenError> {
    let mut jit = cranelift::JitCompiler::new(interner, annotations, symbols)?;
    jit.compile(ast)?;
    jit.run_main()
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_exists() {
        assert!(true);
    }
}
