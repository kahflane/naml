///
/// Literal Value Definitions
///
/// This module defines literal values that can appear in naml source code.
/// Literals are the atomic values like numbers, strings, and booleans.
///
/// Design decisions:
/// - No Nil literal - use option<T> with None instead
/// - String content is interned via Spur for zero-allocation
/// - Bytes stored as Vec<u8> for raw byte data
/// - Separate Int (signed) and UInt (unsigned) for type safety
///

use lasso::Spur;

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    UInt(u64),
    Float(f64),
    Bool(bool),
    String(Spur),
    Bytes(Vec<u8>),
    None,
}

impl Literal {
    pub fn is_numeric(&self) -> bool {
        matches!(self, Literal::Int(_) | Literal::UInt(_) | Literal::Float(_))
    }

    pub fn is_integer(&self) -> bool {
        matches!(self, Literal::Int(_) | Literal::UInt(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_numeric() {
        assert!(Literal::Int(42).is_numeric());
        assert!(Literal::UInt(42).is_numeric());
        assert!(Literal::Float(3.14).is_numeric());
        assert!(!Literal::Bool(true).is_numeric());
    }

    #[test]
    fn test_is_integer() {
        assert!(Literal::Int(-5).is_integer());
        assert!(Literal::UInt(5).is_integer());
        assert!(!Literal::Float(5.0).is_integer());
    }
}
