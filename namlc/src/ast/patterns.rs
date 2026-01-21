///
/// Pattern AST Nodes
///
/// Patterns are used in switch cases and destructuring bindings.
/// They can match literals, enum variants, and bind variables.
///
/// Key pattern types:
/// - LiteralPattern: Match a literal value (int, string, etc.)
/// - IdentPattern: Match an identifier (binds or compares)
/// - VariantPattern: Match an enum variant with optional bindings
/// - WildcardPattern: Match anything (the `_` pattern)
///
/// Design decisions:
/// - Each pattern carries its own Span for error reporting
/// - VariantPattern supports both simple (Active) and destructuring (Suspended(reason)) forms
/// - The path in VariantPattern allows qualified names like EnumType.Variant
/// - VariantPattern uses Vec for path and bindings, which allocates on the heap
///

use crate::source::{Span, Spanned};
use super::types::Ident;
use super::literals::Literal;

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern<'ast> {
    /// Match a literal value (int, string, etc)
    Literal(LiteralPattern),
    /// Match an identifier (binds or compares)
    Identifier(IdentPattern),
    /// Match an enum variant: Variant or Variant(a, b)
    Variant(VariantPattern),
    /// Wildcard pattern: _
    Wildcard(WildcardPattern),
    /// PhantomData to use the lifetime (for future extensibility)
    #[doc(hidden)]
    _Phantom(std::marker::PhantomData<&'ast ()>),
}

impl<'ast> Spanned for Pattern<'ast> {
    fn span(&self) -> Span {
        match self {
            Pattern::Literal(p) => p.span,
            Pattern::Identifier(p) => p.span,
            Pattern::Variant(p) => p.span,
            Pattern::Wildcard(p) => p.span,
            Pattern::_Phantom(_) => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LiteralPattern {
    pub value: Literal,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IdentPattern {
    pub ident: Ident,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariantPattern {
    /// The enum type path: e.g., [UserStatus, Suspended]
    pub path: Vec<Ident>,
    /// Bindings for variant data: (reason) binds `reason`
    pub bindings: Vec<Ident>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WildcardPattern {
    pub span: Span,
}

#[cfg(test)]
mod tests {
    use super::*;
    use lasso::Spur;

    #[test]
    fn test_literal_pattern_span() {
        let pattern = Pattern::Literal(LiteralPattern {
            value: Literal::Int(42),
            span: Span::new(0, 2, 0),
        });
        assert_eq!(pattern.span(), Span::new(0, 2, 0));
    }

    #[test]
    fn test_wildcard_pattern_span() {
        let pattern = Pattern::Wildcard(WildcardPattern {
            span: Span::new(10, 11, 0),
        });
        assert_eq!(pattern.span(), Span::new(10, 11, 0));
    }

    #[test]
    fn test_ident_pattern_span() {
        let pattern = Pattern::Identifier(IdentPattern {
            ident: Ident::new(Spur::default(), Span::new(5, 10, 0)),
            span: Span::new(5, 10, 0),
        });
        assert_eq!(pattern.span(), Span::new(5, 10, 0));
    }

    #[test]
    fn test_variant_pattern_span() {
        let pattern = Pattern::Variant(VariantPattern {
            path: vec![
                Ident::new(Spur::default(), Span::new(0, 6, 0)),
            ],
            bindings: vec![],
            span: Span::new(0, 6, 0),
        });
        assert_eq!(pattern.span(), Span::new(0, 6, 0));
    }
}
