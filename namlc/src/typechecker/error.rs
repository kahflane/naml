///
/// Type Checker Error Types
///
/// This module defines error types for the type checking phase. Errors
/// carry source location information for precise error reporting.
///
/// Error categories:
/// - TypeMismatch: Expected one type, found another
/// - UndefinedVariable: Variable not found in scope
/// - UndefinedType: Type name not found
/// - UndefinedFunction: Function not found
/// - UndefinedField: Struct field not found
/// - UndefinedMethod: Method not found on type
/// - DuplicateDefinition: Name already defined in scope
/// - InvalidOperation: Operation not valid for types
/// - InferenceFailed: Could not infer type
///

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

    #[error("await outside of async function")]
    AwaitOutsideAsync { span: Span },

    #[error("{message}")]
    Custom { message: String, span: Span },
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
            TypeError::AwaitOutsideAsync { span } => *span,
            TypeError::Custom { span, .. } => *span,
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
