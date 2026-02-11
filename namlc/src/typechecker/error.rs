//!
//! Type Checker Error Types
//!
//! This module defines error types for the type checking phase. Errors
//! carry source location information for precise error reporting.
//!
//! Error categories:
//! - TypeMismatch: Expected one type, found another
//! - UndefinedVariable: Variable not found in scope
//! - UndefinedType: Type name not found
//! - UndefinedFunction: Function not found
//! - UndefinedField: Struct field not found
//! - UndefinedMethod: Method not found on type
//! - DuplicateDefinition: Name already defined in scope
//! - InvalidOperation: Operation not valid for types
//! - InferenceFailed: Could not infer type
//!

use crate::source::Span;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum TypeError {
    #[error("type mismatch: expected {expected}, found {found}")]
    TypeMismatch {
        expected: String,
        found: String,
        span: Span,
    },

    #[error("undefined variable '{name}'")]
    UndefinedVariable { name: String, span: Span },

    #[error("undefined type '{name}'")]
    UndefinedType { name: String, span: Span },

    #[error("undefined function '{name}'")]
    UndefinedFunction { name: String, span: Span },

    #[error("undefined field '{field}' on type '{ty}'")]
    UndefinedField {
        ty: String,
        field: String,
        span: Span,
    },

    #[error("undefined method '{method}' on type '{ty}'")]
    UndefinedMethod {
        ty: String,
        method: String,
        span: Span,
    },

    #[error("duplicate definition '{name}'")]
    DuplicateDefinition { name: String, span: Span },

    #[error("duplicate import of '{name}': already imported from another module")]
    DuplicateImport { name: String, span: Span },

    #[error("invalid operation: cannot apply {op} to {ty}")]
    InvalidOperation {
        op: String,
        ty: String,
        span: Span,
    },

    #[error("invalid binary operation: cannot apply {op} to {left} and {right}")]
    InvalidBinaryOp {
        op: String,
        left: String,
        right: String,
        span: Span,
    },

    #[error("cannot infer type")]
    InferenceFailed { span: Span },

    #[error("wrong number of arguments: expected {expected}, found {found}")]
    WrongArgCount {
        expected: usize,
        found: usize,
        span: Span,
    },

    #[error("wrong number of type arguments: expected {expected}, found {found}")]
    WrongTypeArgCount {
        expected: usize,
        found: usize,
        span: Span,
    },

    #[error("type '{ty}' is not callable")]
    NotCallable { ty: String, span: Span },

    #[error("type '{ty}' is not indexable")]
    NotIndexable { ty: String, span: Span },

    #[error("type '{ty}' is not iterable")]
    NotIterable { ty: String, span: Span },

    #[error("cannot assign to immutable variable '{name}'")]
    ImmutableAssignment { name: String, span: Span },

    #[error("cannot use '{feature}' on platform '{platform}'")]
    PlatformMismatch {
        feature: String,
        platform: String,
        available: String,
        span: Span,
    },

    #[error("missing return value")]
    MissingReturn { span: Span },

    #[error("unreachable code")]
    UnreachableCode { span: Span },

    #[error("break outside of loop")]
    BreakOutsideLoop { span: Span },

    #[error("continue outside of loop")]
    ContinueOutsideLoop { span: Span },

    #[error("type '{ty}' does not satisfy bound '{bound}'")]
    BoundNotSatisfied {
        ty: String,
        bound: String,
        span: Span,
    },

    #[error("no bound provides method '{method}' for type parameter '{param}'")]
    NoBoundForMethod {
        param: String,
        method: String,
        span: Span,
    },

    #[error("{message}")]
    Custom { message: String, span: Span },

    #[error("struct '{struct_name}' is missing method '{method_name}' required by interface '{interface_name}'")]
    MissingInterfaceMethod {
        struct_name: String,
        interface_name: String,
        method_name: String,
        span: Span,
    },

    #[error("unknown module '{path}'")]
    UnknownModule { path: String, span: Span },

    #[error("unknown symbol '{symbol}' in module '{module}'")]
    UnknownModuleSymbol {
        module: String,
        symbol: String,
        span: Span,
    },

    #[error("symbol '{symbol}' in module '{module}' is not public")]
    PrivateSymbol {
        module: String,
        symbol: String,
        span: Span,
    },

    #[error("cannot read module file '{path}': {reason}")]
    ModuleFileError {
        path: String,
        reason: String,
        span: Span,
    },

    #[error("uncaught exception '{exception_type}': must be caught or declared in function's throws clause")]
    UncaughtException {
        exception_type: String,
        span: Span,
    },

    #[error("`try` and `catch` cannot be used together")]
    TryWithCatch { span: Span },

    #[error("ambiguous function '{name}': exists in multiple imported modules, use a qualified path")]
    AmbiguousFunction { name: String, span: Span },

    #[error("package error '{package}': {reason}")]
    PackageError {
        package: String,
        reason: String,
        span: Span,
    },
}

impl TypeError {
    pub fn span(&self) -> Span {
        match self {
            TypeError::TypeMismatch { span, .. } => *span,
            TypeError::UndefinedVariable { span, .. } => *span,
            TypeError::UndefinedType { span, .. } => *span,
            TypeError::UndefinedFunction { span, .. } => *span,
            TypeError::UndefinedField { span, .. } => *span,
            TypeError::UndefinedMethod { span, .. } => *span,
            TypeError::DuplicateDefinition { span, .. } => *span,
            TypeError::DuplicateImport { span, .. } => *span,
            TypeError::InvalidOperation { span, .. } => *span,
            TypeError::InvalidBinaryOp { span, .. } => *span,
            TypeError::InferenceFailed { span } => *span,
            TypeError::WrongArgCount { span, .. } => *span,
            TypeError::WrongTypeArgCount { span, .. } => *span,
            TypeError::NotCallable { span, .. } => *span,
            TypeError::NotIndexable { span, .. } => *span,
            TypeError::NotIterable { span, .. } => *span,
            TypeError::ImmutableAssignment { span, .. } => *span,
            TypeError::PlatformMismatch { span, .. } => *span,
            TypeError::MissingReturn { span } => *span,
            TypeError::UnreachableCode { span } => *span,
            TypeError::BreakOutsideLoop { span } => *span,
            TypeError::ContinueOutsideLoop { span } => *span,
            TypeError::BoundNotSatisfied { span, .. } => *span,
            TypeError::NoBoundForMethod { span, .. } => *span,
            TypeError::Custom { span, .. } => *span,
            TypeError::MissingInterfaceMethod { span, .. } => *span,
            TypeError::UnknownModule { span, .. } => *span,
            TypeError::UnknownModuleSymbol { span, .. } => *span,
            TypeError::PrivateSymbol { span, .. } => *span,
            TypeError::ModuleFileError { span, .. } => *span,
            TypeError::UncaughtException { span, .. } => *span,
            TypeError::TryWithCatch { span } => *span,
            TypeError::AmbiguousFunction { span, .. } => *span,
            TypeError::PackageError { span, .. } => *span,
        }
    }

    pub fn type_mismatch(expected: impl Into<String>, found: impl Into<String>, span: Span) -> Self {
        TypeError::TypeMismatch {
            expected: expected.into(),
            found: found.into(),
            span,
        }
    }

    pub fn undefined_var(name: impl Into<String>, span: Span) -> Self {
        TypeError::UndefinedVariable {
            name: name.into(),
            span,
        }
    }

    pub fn undefined_type(name: impl Into<String>, span: Span) -> Self {
        TypeError::UndefinedType {
            name: name.into(),
            span,
        }
    }
}

pub type TypeResult<T> = Result<T, TypeError>;
