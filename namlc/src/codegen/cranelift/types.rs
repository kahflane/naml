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
use crate::typechecker::types::Type as TcType;

pub fn naml_to_cranelift(ty: &NamlType) -> Type {
    match ty {
        NamlType::Int => types::I64,
        NamlType::Uint => types::I64,
        NamlType::Float => types::F64,
        NamlType::Bool => types::I8,
        NamlType::String => types::I64,
        NamlType::Bytes => types::I64,
        NamlType::Unit => types::I64,

        NamlType::Array(_) => types::I64,
        NamlType::FixedArray(_, _) => types::I64,
        NamlType::Option(_) => types::I64,
        NamlType::Map(_, _) => types::I64,
        NamlType::Channel(_) => types::I64,

        NamlType::Named(_) => types::I64,
        NamlType::Generic(_, _) => types::I64,
        NamlType::Function { .. } => types::I64,
        NamlType::Decimal { .. } => types::I64,
        NamlType::Inferred => types::I64,
    }
}

/// Convert typechecker Type to Cranelift type
pub fn tc_type_to_cranelift(ty: &TcType) -> Type {
    match ty {
        TcType::Int => types::I64,
        TcType::Uint => types::I64,
        TcType::Float => types::F64,
        TcType::Bool => types::I8,
        TcType::String => types::I64,
        TcType::Bytes => types::I64,
        TcType::Unit => types::I64,
        TcType::Array(_) => types::I64,
        TcType::FixedArray(_, _) => types::I64,
        TcType::Option(_) => types::I64,
        TcType::Map(_, _) => types::I64,
        TcType::Channel(_) => types::I64,
        TcType::Struct(_) => types::I64,
        TcType::Enum(_) => types::I64,
        TcType::Interface(_) => types::I64,
        TcType::Exception(_) => types::I64,
        TcType::Function(_) => types::I64,
        TcType::TypeVar(_) => types::I64,
        TcType::Generic(_, _) => types::I64,
        TcType::Error => types::I64,
        TcType::Never => types::I64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_types() {
        assert_eq!(naml_to_cranelift(&NamlType::Int), types::I64);
        assert_eq!(naml_to_cranelift(&NamlType::Float), types::F64);
        assert_eq!(naml_to_cranelift(&NamlType::Bool), types::I8);
    }
}
