#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HeapType {
    String,
    Array(Option<Box<HeapType>>),
    Map(Option<Box<HeapType>>),
    Struct(Option<lasso::Spur>),
    OptionOf(Box<HeapType>),
}

pub fn get_heap_type_resolved(naml_ty: &crate::ast::NamlType, interner: &lasso::Rodeo) -> Option<HeapType> {
    use crate::ast::NamlType;
    match naml_ty {
        NamlType::String => Some(HeapType::String),
        NamlType::Array(elem_ty) => {
            let elem_heap_type = get_heap_type_resolved(elem_ty, interner).map(Box::new);
            Some(HeapType::Array(elem_heap_type))
        }
        NamlType::FixedArray(elem_ty, _) => {
            let elem_heap_type = get_heap_type_resolved(elem_ty, interner).map(Box::new);
            Some(HeapType::Array(elem_heap_type))
        }
        NamlType::Map(_, val_ty) => {
            let val_heap_type = get_heap_type_resolved(val_ty, interner).map(Box::new);
            Some(HeapType::Map(val_heap_type))
        }
        NamlType::Option(inner_ty) => {
            get_heap_type_resolved(inner_ty, interner).map(|ht| HeapType::OptionOf(Box::new(ht)))
        }
        NamlType::Named(ident) => {
            Some(HeapType::Struct(Some(ident.symbol)))
        }
        NamlType::Generic(_, _) => Some(HeapType::Struct(None)),
        _ => None,
    }
}

pub fn heap_type_from_type(
    ty: &crate::typechecker::types::Type,
    _interner: &lasso::Rodeo,
) -> Option<HeapType> {
    use crate::typechecker::types::Type;
    match ty {
        Type::String => Some(HeapType::String),
        Type::Array(elem) | Type::FixedArray(elem, _) => {
            let elem_heap = heap_type_from_type(elem, _interner).map(Box::new);
            Some(HeapType::Array(elem_heap))
        }
        Type::Map(_, val) => {
            let val_heap = heap_type_from_type(val, _interner).map(Box::new);
            Some(HeapType::Map(val_heap))
        }
        Type::Struct(s) => {
            Some(HeapType::Struct(Some(s.name)))
        }
        Type::Option(inner) => {
            heap_type_from_type(inner, _interner).map(|ht| HeapType::OptionOf(Box::new(ht)))
        }
        _ => None,
    }
}
