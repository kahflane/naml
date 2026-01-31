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
        NamlType::Named(_) => Some(HeapType::Struct(None)),
        NamlType::Generic(_, _) => Some(HeapType::Struct(None)),
        _ => None,
    }
}