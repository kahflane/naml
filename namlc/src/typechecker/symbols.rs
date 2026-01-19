///
/// Symbol Table - Global Definitions
///
/// This module manages the symbol table for type checking. It stores:
///
/// - Type definitions (structs, enums, interfaces, exceptions)
/// - Function signatures (including methods)
/// - Built-in types and functions
///
/// The symbol table is built in a first pass over the AST to collect all
/// definitions, then used during type checking to resolve references.
///

use std::collections::HashMap;

use lasso::Spur;

use super::types::{
    EnumType, FieldType, FunctionType, InterfaceType, MethodType, StructType, Type, VariantType,
};
use crate::source::Span;

#[derive(Debug, Clone)]
pub struct FunctionSig {
    pub name: Spur,
    pub type_params: Vec<Spur>,
    pub params: Vec<(Spur, Type)>,
    pub return_ty: Type,
    pub throws: Option<Type>,
    pub is_async: bool,
    pub is_public: bool,
    pub is_variadic: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MethodSig {
    pub name: Spur,
    pub receiver_ty: Type,
    pub receiver_mutable: bool,
    pub type_params: Vec<Spur>,
    pub params: Vec<(Spur, Type)>,
    pub return_ty: Type,
    pub throws: Option<Type>,
    pub is_async: bool,
    pub is_public: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TypeDef {
    Struct(StructDef),
    Enum(EnumDef),
    Interface(InterfaceDef),
    Exception(ExceptionDef),
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: Spur,
    pub type_params: Vec<Spur>,
    pub fields: Vec<(Spur, Type, bool)>,
    pub implements: Vec<Type>,
    pub is_public: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: Spur,
    pub type_params: Vec<Spur>,
    pub variants: Vec<(Spur, Option<Vec<Type>>)>,
    pub is_public: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct InterfaceDef {
    pub name: Spur,
    pub type_params: Vec<Spur>,
    pub extends: Vec<Type>,
    pub methods: Vec<InterfaceMethodDef>,
    pub is_public: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct InterfaceMethodDef {
    pub name: Spur,
    pub type_params: Vec<Spur>,
    pub params: Vec<(Spur, Type)>,
    pub return_ty: Type,
    pub throws: Option<Type>,
    pub is_async: bool,
}

#[derive(Debug, Clone)]
pub struct ExceptionDef {
    pub name: Spur,
    pub fields: Vec<(Spur, Type)>,
    pub is_public: bool,
    pub span: Span,
}

#[derive(Debug)]
pub struct SymbolTable {
    types: HashMap<Spur, TypeDef>,
    functions: HashMap<Spur, FunctionSig>,
    methods: HashMap<Spur, Vec<MethodSig>>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            types: HashMap::new(),
            functions: HashMap::new(),
            methods: HashMap::new(),
        }
    }

    pub fn define_type(&mut self, name: Spur, def: TypeDef) {
        self.types.insert(name, def);
    }

    pub fn get_type(&self, name: Spur) -> Option<&TypeDef> {
        self.types.get(&name)
    }

    pub fn define_function(&mut self, sig: FunctionSig) {
        self.functions.insert(sig.name, sig);
    }

    pub fn get_function(&self, name: Spur) -> Option<&FunctionSig> {
        self.functions.get(&name)
    }

    pub fn define_method(&mut self, type_name: Spur, method: MethodSig) {
        self.methods.entry(type_name).or_default().push(method);
    }

    pub fn get_methods(&self, type_name: Spur) -> Option<&Vec<MethodSig>> {
        self.methods.get(&type_name)
    }

    pub fn get_method(&self, type_name: Spur, method_name: Spur) -> Option<&MethodSig> {
        self.methods
            .get(&type_name)?
            .iter()
            .find(|m| m.name == method_name)
    }

    pub fn to_struct_type(&self, def: &StructDef) -> StructType {
        StructType {
            name: def.name,
            fields: def
                .fields
                .iter()
                .map(|(name, ty, is_public)| FieldType {
                    name: *name,
                    ty: ty.clone(),
                    is_public: *is_public,
                })
                .collect(),
            type_params: def.type_params.clone(),
            type_args: Vec::new(),
        }
    }

    pub fn to_enum_type(&self, def: &EnumDef) -> EnumType {
        EnumType {
            name: def.name,
            variants: def
                .variants
                .iter()
                .map(|(name, fields)| VariantType {
                    name: *name,
                    fields: fields.clone(),
                })
                .collect(),
            type_params: def.type_params.clone(),
            type_args: Vec::new(),
        }
    }

    pub fn to_interface_type(&self, def: &InterfaceDef) -> InterfaceType {
        InterfaceType {
            name: def.name,
            methods: def
                .methods
                .iter()
                .map(|m| MethodType {
                    name: m.name,
                    params: m.params.iter().map(|(_, ty)| ty.clone()).collect(),
                    returns: m.return_ty.clone(),
                    throws: m.throws.clone(),
                    is_async: m.is_async,
                })
                .collect(),
            type_params: def.type_params.clone(),
        }
    }

    pub fn to_function_type(&self, sig: &FunctionSig) -> FunctionType {
        FunctionType {
            params: sig.params.iter().map(|(_, ty)| ty.clone()).collect(),
            returns: Box::new(sig.return_ty.clone()),
            throws: sig.throws.clone().map(Box::new),
            is_async: sig.is_async,
            is_variadic: sig.is_variadic,
        }
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::Span;
    use lasso::Rodeo;

    #[test]
    fn test_define_lookup_function() {
        let mut rodeo = Rodeo::default();
        let main = rodeo.get_or_intern("main");

        let mut table = SymbolTable::new();
        table.define_function(FunctionSig {
            name: main,
            type_params: vec![],
            params: vec![],
            return_ty: Type::Unit,
            throws: None,
            is_async: false,
            is_public: true,
            is_variadic: false,
            span: Span::dummy(),
        });

        assert!(table.get_function(main).is_some());
    }

    #[test]
    fn test_define_lookup_type() {
        let mut rodeo = Rodeo::default();
        let point = rodeo.get_or_intern("Point");
        let x = rodeo.get_or_intern("x");
        let y = rodeo.get_or_intern("y");

        let mut table = SymbolTable::new();
        table.define_type(
            point,
            TypeDef::Struct(StructDef {
                name: point,
                type_params: vec![],
                fields: vec![(x, Type::Int, true), (y, Type::Int, true)],
                implements: vec![],
                is_public: true,
                span: Span::dummy(),
            }),
        );

        assert!(table.get_type(point).is_some());
    }

    #[test]
    fn test_define_lookup_method() {
        let mut rodeo = Rodeo::default();
        let point = rodeo.get_or_intern("Point");
        let distance = rodeo.get_or_intern("distance");

        let mut table = SymbolTable::new();
        table.define_method(
            point,
            MethodSig {
                name: distance,
                receiver_ty: Type::Struct(StructType {
                    name: point,
                    fields: vec![],
                    type_params: vec![],
                    type_args: vec![],
                }),
                receiver_mutable: false,
                type_params: vec![],
                params: vec![],
                return_ty: Type::Float,
                throws: None,
                is_async: false,
                is_public: true,
                span: Span::dummy(),
            },
        );

        assert!(table.get_method(point, distance).is_some());
    }
}
