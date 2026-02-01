//!
//! AST Type Definitions
//!
//! This module defines the core type system for naml's AST. All types that
//! can appear in type annotations are represented here.
//!
//! Key types:
//! - Ident: An identifier with its source location (uses string interning)
//! - NamlType: The complete type system including primitives, composites,
//!   generics, and function types
//!
//! Design decisions:
//! - No Any type - naml is strongly typed with no dynamic escape hatch
//! - Ident carries its Span for better error messages
//! - Box-based nesting for simplicity (can optimize to arena later)
//! - Inferred placeholder for type inference pass
//!

use lasso::Spur;
use crate::source::Span;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ident {
    pub symbol: Spur,
    pub span: Span,
}

impl Ident {
    pub fn new(symbol: Spur, span: Span) -> Self {
        Self { symbol, span }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NamlType {
    Int,
    Uint,
    Float,
    Bool,
    String,
    Bytes,
    Unit,
    Decimal { precision: u8, scale: u8 },

    Array(Box<NamlType>),
    FixedArray(Box<NamlType>, usize),
    Option(Box<NamlType>),
    Map(Box<NamlType>, Box<NamlType>),
    Channel(Box<NamlType>),
    Mutex(Box<NamlType>),
    Rwlock(Box<NamlType>),

    Named(Ident),
    Generic(Ident, Vec<NamlType>),

    Function {
        params: Vec<NamlType>,
        returns: Box<NamlType>,
    },

    Inferred,
}

impl NamlType {
    pub fn array(inner: NamlType) -> Self {
        NamlType::Array(Box::new(inner))
    }

    pub fn fixed_array(inner: NamlType, size: usize) -> Self {
        NamlType::FixedArray(Box::new(inner), size)
    }

    pub fn option(inner: NamlType) -> Self {
        NamlType::Option(Box::new(inner))
    }

    pub fn map(key: NamlType, value: NamlType) -> Self {
        NamlType::Map(Box::new(key), Box::new(value))
    }

    pub fn channel(inner: NamlType) -> Self {
        NamlType::Channel(Box::new(inner))
    }

    pub fn mutex(inner: NamlType) -> Self {
        NamlType::Mutex(Box::new(inner))
    }

    pub fn rwlock(inner: NamlType) -> Self {
        NamlType::Rwlock(Box::new(inner))
    }

    pub fn function(params: Vec<NamlType>, returns: NamlType) -> Self {
        NamlType::Function {
            params,
            returns: Box::new(returns),
        }
    }

    pub fn is_primitive(&self) -> bool {
        matches!(
            self,
            NamlType::Int
                | NamlType::Uint
                | NamlType::Float
                | NamlType::Bool
                | NamlType::String
                | NamlType::Bytes
                | NamlType::Unit
                | NamlType::Decimal { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nested_types() {
        let inner = NamlType::Int;
        let arr = NamlType::array(inner);
        let opt = NamlType::option(arr);

        match opt {
            NamlType::Option(inner) => match *inner {
                NamlType::Array(elem) => assert_eq!(*elem, NamlType::Int),
                _ => panic!("Expected Array"),
            },
            _ => panic!("Expected Option"),
        }
    }

    #[test]
    fn test_is_primitive() {
        assert!(NamlType::Int.is_primitive());
        assert!(NamlType::String.is_primitive());
        assert!(!NamlType::array(NamlType::Int).is_primitive());
    }
}
