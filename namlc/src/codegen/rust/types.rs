///
/// Type Mappings (naml → Rust)
///
/// Converts naml types to their Rust equivalents:
/// - int → i64
/// - uint → u64
/// - float → f64
/// - bool → bool
/// - string → String
/// - [T] → Vec<T>
/// - option<T> → Option<T>
/// - map<K, V> → std::collections::HashMap<K, V>
///

use lasso::Rodeo;

use crate::ast::NamlType;

pub fn naml_to_rust(ty: &NamlType, interner: &Rodeo) -> String {
    naml_to_rust_in_context(ty, interner, None)
}

pub fn naml_to_rust_in_struct(ty: &NamlType, interner: &Rodeo, struct_name: &str) -> String {
    naml_to_rust_in_context(ty, interner, Some(struct_name))
}

fn type_references_struct(ty: &NamlType, interner: &Rodeo, struct_name: &str) -> bool {
    match ty {
        NamlType::Named(ident) => interner.resolve(&ident.symbol) == struct_name,
        NamlType::Generic(name, _) => interner.resolve(&name.symbol) == struct_name,
        _ => false,
    }
}

fn naml_to_rust_in_context(ty: &NamlType, interner: &Rodeo, current_struct: Option<&str>) -> String {
    match ty {
        NamlType::Int => "i64".to_string(),
        NamlType::Uint => "u64".to_string(),
        NamlType::Float => "f64".to_string(),
        NamlType::Bool => "bool".to_string(),
        NamlType::String => "String".to_string(),
        NamlType::Bytes => "Vec<u8>".to_string(),
        NamlType::Unit => "()".to_string(),

        NamlType::Array(elem_ty) => {
            let elem = naml_to_rust_in_context(elem_ty, interner, current_struct);
            if let Some(struct_name) = current_struct {
                if type_references_struct(elem_ty, interner, struct_name) {
                    return format!("Vec<Box<{}>>", elem);
                }
            }
            format!("Vec<{}>", elem)
        }

        NamlType::FixedArray(elem_ty, size) => {
            let elem = naml_to_rust_in_context(elem_ty, interner, current_struct);
            format!("[{}; {}]", elem, size)
        }

        NamlType::Option(inner_ty) => {
            let inner = naml_to_rust_in_context(inner_ty, interner, current_struct);
            if let Some(struct_name) = current_struct {
                if type_references_struct(inner_ty, interner, struct_name) {
                    return format!("Option<Box<{}>>", inner);
                }
            }
            format!("Option<{}>", inner)
        }

        NamlType::Map(key_ty, val_ty) => {
            let key = naml_to_rust_in_context(key_ty, interner, current_struct);
            let val = naml_to_rust_in_context(val_ty, interner, current_struct);
            format!("std::collections::HashMap<{}, {}>", key, val)
        }

        NamlType::Channel(inner_ty) => {
            let inner = naml_to_rust_in_context(inner_ty, interner, current_struct);
            format!("tokio::sync::mpsc::Sender<{}>", inner)
        }

        NamlType::Promise(inner_ty) => {
            let inner = naml_to_rust_in_context(inner_ty, interner, current_struct);
            format!("std::pin::Pin<Box<dyn std::future::Future<Output = {}> + Send>>", inner)
        }

        NamlType::Named(ident) => {
            interner.resolve(&ident.symbol).to_string()
        }

        NamlType::Generic(name, type_args) => {
            let base_name = interner.resolve(&name.symbol);
            let args: Vec<String> = type_args
                .iter()
                .map(|t| naml_to_rust_in_context(t, interner, current_struct))
                .collect();
            format!("{}<{}>", base_name, args.join(", "))
        }

        NamlType::Function { params, returns } => {
            let param_types: Vec<String> = params
                .iter()
                .map(|t| naml_to_rust_in_context(t, interner, current_struct))
                .collect();
            let return_type = naml_to_rust_in_context(returns, interner, current_struct);
            format!("fn({}) -> {}", param_types.join(", "), return_type)
        }

        NamlType::Decimal { precision, scale } => {
            format!("Decimal<{}, {}>", precision, scale)
        }

        NamlType::Inferred => "/* inferred */".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lasso::Rodeo;

    #[test]
    fn test_primitive_types() {
        let interner = Rodeo::new();

        assert_eq!(naml_to_rust(&NamlType::Int, &interner), "i64");
        assert_eq!(naml_to_rust(&NamlType::Uint, &interner), "u64");
        assert_eq!(naml_to_rust(&NamlType::Float, &interner), "f64");
        assert_eq!(naml_to_rust(&NamlType::Bool, &interner), "bool");
        assert_eq!(naml_to_rust(&NamlType::String, &interner), "String");
    }

    #[test]
    fn test_array_type() {
        let interner = Rodeo::new();

        let arr_ty = NamlType::Array(Box::new(NamlType::Int));
        assert_eq!(naml_to_rust(&arr_ty, &interner), "Vec<i64>");
    }

    #[test]
    fn test_option_type() {
        let interner = Rodeo::new();

        let opt_ty = NamlType::Option(Box::new(NamlType::String));
        assert_eq!(naml_to_rust(&opt_ty, &interner), "Option<String>");
    }

    #[test]
    fn test_map_type() {
        let interner = Rodeo::new();

        let map_ty = NamlType::Map(
            Box::new(NamlType::String),
            Box::new(NamlType::Int),
        );
        assert_eq!(
            naml_to_rust(&map_ty, &interner),
            "std::collections::HashMap<String, i64>"
        );
    }
}
