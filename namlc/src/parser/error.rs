///
/// Parser Error Types
///
/// This module defines error types for the parser. Errors carry source
/// location information (Span) for precise error reporting.
///
/// Error categories:
/// - Expected: A specific token or construct was expected but not found
/// - Unexpected: An unexpected token was encountered
/// - InvalidSyntax: The syntax is malformed
///
/// Errors integrate with miette for rich error display.
///

use crate::lexer::{Keyword, TokenKind};
use crate::source::Span;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ParseError {
    #[error("expected {expected}, found {found:?}")]
    Expected {
        expected: String,
        found: TokenKind,
        span: Span,
    },

    #[error("expected identifier")]
    ExpectedIdent { span: Span },

    #[error("expected expression")]
    ExpectedExpr { span: Span },

    #[error("expected type")]
    ExpectedType { span: Span },

    #[error("expected statement")]
    ExpectedStmt { span: Span },

    #[error("unexpected token {found:?}")]
    Unexpected { found: TokenKind, span: Span },

    #[error("unexpected end of file")]
    UnexpectedEof { span: Span },

    #[error("unclosed delimiter {delimiter}")]
    UnclosedDelimiter { delimiter: char, span: Span },

    #[error("invalid number literal")]
    InvalidNumber { span: Span },

    #[error("invalid escape sequence")]
    InvalidEscape { span: Span },

    #[error("{message}")]
    Custom { message: String, span: Span },
}

impl ParseError {
    pub fn expected(expected: impl Into<String>, found: TokenKind, span: Span) -> Self {
        ParseError::Expected {
            expected: expected.into(),
            found,
            span,
        }
    }

    pub fn expected_token(kind: TokenKind, found: TokenKind, span: Span) -> Self {
        ParseError::Expected {
            expected: format!("{:?}", kind),
            found,
            span,
        }
    }

    pub fn expected_keyword(kw: Keyword, found: TokenKind, span: Span) -> Self {
        ParseError::Expected {
            expected: format!("{:?}", kw),
            found,
            span,
        }
    }

    pub fn expected_ident(span: Span) -> Self {
        ParseError::ExpectedIdent { span }
    }

    pub fn expected_expr(span: Span) -> Self {
        ParseError::ExpectedExpr { span }
    }

    pub fn expected_type(span: Span) -> Self {
        ParseError::ExpectedType { span }
    }

    pub fn unexpected_eof(span: Span) -> Self {
        ParseError::UnexpectedEof { span }
    }

    pub fn custom(message: impl Into<String>, span: Span) -> Self {
        ParseError::Custom {
            message: message.into(),
            span,
        }
    }

    pub fn span(&self) -> Span {
        match self {
            ParseError::Expected { span, .. } => *span,
            ParseError::ExpectedIdent { span } => *span,
            ParseError::ExpectedExpr { span } => *span,
            ParseError::ExpectedType { span } => *span,
            ParseError::ExpectedStmt { span } => *span,
            ParseError::Unexpected { span, .. } => *span,
            ParseError::UnexpectedEof { span } => *span,
            ParseError::UnclosedDelimiter { span, .. } => *span,
            ParseError::InvalidNumber { span } => *span,
            ParseError::InvalidEscape { span } => *span,
            ParseError::Custom { span, .. } => *span,
        }
    }
}

pub type ParseResult<T> = Result<T, ParseError>;
