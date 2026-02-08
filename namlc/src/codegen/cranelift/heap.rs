#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HeapType {
    String,
    Array(Option<Box<HeapType>>),
    Map(Option<Box<HeapType>>),
    Struct(Option<String>),
}

pub fn get_heap_type(naml_ty: &crate::ast::NamlType) -> Option<HeapType> {
    use crate::ast::NamlType;
    match naml_ty {
        NamlType::String => Some(HeapType::String),
        NamlType::Array(elem_ty) => {
            let elem_heap_type = get_heap_type(elem_ty).map(Box::new);
            Some(HeapType::Array(elem_heap_type))
        }
        NamlType::FixedArray(elem_ty, _) => {
            let elem_heap_type = get_heap_type(elem_ty).map(Box::new);
            Some(HeapType::Array(elem_heap_type))
        }
        NamlType::Map(_, val_ty) => {
            let val_heap_type = get_heap_type(val_ty).map(Box::new);
            Some(HeapType::Map(val_heap_type))
        }
        NamlType::Option(inner_ty) => get_heap_type(inner_ty),
        NamlType::Named(_) => Some(HeapType::Struct(None)),
        NamlType::Generic(_, _) => Some(HeapType::Struct(None)),
        _ => None,
    }
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
        NamlType::Option(inner_ty) => get_heap_type_resolved(inner_ty, interner),
        NamlType::Named(ident) => {
            let name = interner.resolve(&ident.symbol).to_string();
            Some(HeapType::Struct(Some(name)))
        }
        NamlType::Generic(_, _) => Some(HeapType::Struct(None)),
        _ => None,
    }
}
