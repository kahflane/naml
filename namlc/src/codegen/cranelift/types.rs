//!
//! Type Mappings (naml -> Cranelift)
//!
//! Maps naml types to Cranelift IR types:
//! - int -> I64
//! - uint -> I64
//! - float -> F64
//! - bool -> I64 (0 or 1)
//! - string -> I64 (pointer)
//!

use cranelift::prelude::types;
use cranelift::prelude::Type;

use crate::ast::NamlType;

pub fn naml_to_cranelift(ty: &NamlType) -> Type {
    match ty {
        NamlType::Int => types::I64,
        NamlType::Uint => types::I64,
        NamlType::Float => types::F64,
        NamlType::Bool => types::I64,
        NamlType::String => types::I64,
        NamlType::Bytes => types::I64,
        NamlType::Unit => types::I64,

        NamlType::Array(_) => types::I64,
        NamlType::FixedArray(_, _) => types::I64,
        NamlType::Option(_) => types::I64,
        NamlType::Map(_, _) => types::I64,
        NamlType::Channel(_) => types::I64,
        NamlType::Promise(_) => types::I64,

        NamlType::Named(_) => types::I64,
        NamlType::Generic(_, _) => types::I64,
        NamlType::Function { .. } => types::I64,
        NamlType::Decimal { .. } => types::I64,
        NamlType::Inferred => types::I64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_types() {
        assert_eq!(naml_to_cranelift(&NamlType::Int), types::I64);
        assert_eq!(naml_to_cranelift(&NamlType::Float), types::F64);
        assert_eq!(naml_to_cranelift(&NamlType::Bool), types::I64);
    }
}
