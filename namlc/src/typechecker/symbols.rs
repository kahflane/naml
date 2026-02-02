//!
//! Symbol Table - Global Definitions
//!
//! This module manages the symbol table for type checking. It stores:
//!
//! - Type definitions (structs, enums, interfaces, exceptions)
//! - Function signatures (including methods)
//! - Built-in types and functions
//!
//! The symbol table is built in a first pass over the AST to collect all
//! definitions, then used during type checking to resolve references.
//!

use std::collections::HashMap;

use lasso::Spur;

use super::types::{
    EnumType, FieldType, FunctionType, InterfaceType, MethodType, StructType, Type, TypeParam,
    VariantType,
};
use crate::source::Span;

#[derive(Debug, Clone)]
pub struct FunctionSig {
    pub name: Spur,
    pub type_params: Vec<TypeParam>,
    pub params: Vec<(Spur, Type)>,
    pub return_ty: Type,
    pub throws: Vec<Type>,
    pub is_public: bool,
    pub is_variadic: bool,
    pub span: Span,
    pub module: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MethodSig {
    pub name: Spur,
    pub receiver_ty: Type,
    pub type_params: Vec<TypeParam>,
    pub params: Vec<(Spur, Type)>,
    pub return_ty: Type,
    pub throws: Vec<Type>,
    pub is_public: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TypeDef {
    Struct(StructDef),
    Enum(EnumDef),
    Interface(InterfaceDef),
    Exception(ExceptionDef),
    TypeAlias(TypeAliasDef),
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: Spur,
    pub type_params: Vec<TypeParam>,
    pub fields: Vec<(Spur, Type, bool)>,
    pub implements: Vec<Type>,
    pub is_public: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: Spur,
    pub type_params: Vec<TypeParam>,
    pub variants: Vec<(Spur, Option<Vec<Type>>)>,
    pub is_public: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct InterfaceDef {
    pub name: Spur,
    pub type_params: Vec<TypeParam>,
    pub extends: Vec<Type>,
    pub methods: Vec<InterfaceMethodDef>,
    pub is_public: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct InterfaceMethodDef {
    pub name: Spur,
    pub type_params: Vec<TypeParam>,
    pub params: Vec<(Spur, Type)>,
    pub return_ty: Type,
    pub throws: Vec<Type>,
}

#[derive(Debug, Clone)]
pub struct ExceptionDef {
    pub name: Spur,
    pub fields: Vec<(Spur, Type)>,
    pub is_public: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeAliasDef {
    pub name: Spur,
    pub type_params: Vec<TypeParam>,
    pub aliased_type: Type,
    pub is_public: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ModuleNamespace {
    pub name: Spur,
    pub functions: HashMap<Spur, FunctionSig>,
    pub types: HashMap<Spur, TypeDef>,
    pub submodules: HashMap<Spur, ModuleNamespace>,
}

impl ModuleNamespace {
    pub fn new(name: Spur) -> Self {
        Self {
            name,
            functions: HashMap::new(),
            types: HashMap::new(),
            submodules: HashMap::new(),
        }
    }

    pub fn add_function(&mut self, sig: FunctionSig) {
        self.functions.insert(sig.name, sig);
    }

    pub fn get_function(&self, name: Spur) -> Option<&FunctionSig> {
        self.functions.get(&name)
    }

    pub fn define_type(&mut self, name: Spur, def: TypeDef) {
        self.types.insert(name, def);
    }

    pub fn get_type(&self, name: Spur) -> Option<&TypeDef> {
        self.types.get(&name)
    }

    pub fn submodule(&mut self, name: Spur) -> &mut ModuleNamespace {
        self.submodules
            .entry(name)
            .or_insert_with(|| ModuleNamespace::new(name))
    }

    pub fn define_submodule(&mut self, name: Spur, module: ModuleNamespace) {
        self.submodules.insert(name, module);
    }

    pub fn get_submodule(&self, name: Spur) -> Option<&ModuleNamespace> {
        self.submodules.get(&name)
    }

    pub fn all_functions(&self) -> impl Iterator<Item = &FunctionSig> {
        self.functions.values()
    }

    pub fn all_types(&self) -> impl Iterator<Item = (&Spur, &TypeDef)> {
        self.types.iter()
    }

    pub fn all_submodules(&self) -> impl Iterator<Item = (&Spur, &ModuleNamespace)> {
        self.submodules.iter()
    }
}

impl Default for ModuleNamespace {
    fn default() -> Self {
        Self::new(Spur::default())
    }
}

use std::collections::HashSet;

#[derive(Debug, Clone)]
pub enum ResolvedItem<'a> {
    Function(&'a FunctionSig),
    Type(&'a TypeDef),
    Module(&'a ModuleNamespace),
}

#[derive(Debug)]
pub struct SymbolTable {
    pub root: ModuleNamespace,
    // Global maps for backward compatibility and fast lookup of unqualified names
    // (though in a proper module system, these should be scoped)
    types: HashMap<Spur, TypeDef>,
    functions: HashMap<Spur, FunctionSig>,
    ambiguous_functions: HashSet<Spur>,
    methods: HashMap<Spur, Vec<MethodSig>>,
    pub current_path: Vec<Spur>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            root: ModuleNamespace::new(Spur::default()),
            types: HashMap::new(),
            functions: HashMap::new(),
            ambiguous_functions: HashSet::new(),
            methods: HashMap::new(),
            current_path: Vec::new(),
        }
    }

    pub fn enter_module(&mut self, name: Spur) {
        self.current_path.push(name);
    }

    pub fn exit_module(&mut self) {
        self.current_path.pop();
    }

    fn get_current_module_mut(&mut self) -> &mut ModuleNamespace {
        let mut curr = &mut self.root;
        for &seg in &self.current_path {
            curr = curr.submodule(seg);
        }
        curr
    }

    pub fn register_module(&mut self, name: Spur) -> &mut ModuleNamespace {
        self.root.submodule(name)
    }

    pub fn get_module(&self, name: Spur) -> Option<&ModuleNamespace> {
        self.root.get_submodule(name)
    }

    pub fn get_module_function(&self, module: Spur, func: Spur) -> Option<&FunctionSig> {
        self.root.get_submodule(module)?.get_function(func)
    }

    pub fn has_module(&self, name: Spur) -> bool {
        self.root.get_submodule(name).is_some()
    }

    pub fn define_type(&mut self, name: Spur, def: TypeDef) {
        self.types.insert(name, def.clone());
        self.get_current_module_mut().define_type(name, def);
    }

    pub fn define_module(&mut self, name: Spur, module: ModuleNamespace) {
        self.get_current_module_mut().define_submodule(name, module);
    }

    pub fn resolve_path(&self, path: &[Spur], interner: &lasso::Rodeo) -> Option<ResolvedItem<'_>> {
        if path.is_empty() {
            return None;
        }

        let mut curr_module = &self.root;
        let mut start_idx = 0;

        // Special handling for self and super
        let first = path[0];
        let first_str = interner.resolve(&first);
        if first_str == "self" {
            curr_module = self.get_current_module();
            start_idx = 1;
        } else if first_str == "super" {
            curr_module = self.get_parent_module()?;
            start_idx = 1;
        } else {
            // Check if first segment is a submodule of current module
            if let Some(sub) = self.get_current_module().get_submodule(first) {
                curr_module = sub;
                start_idx = 1;
            } else if let Some(sub) = self.root.get_submodule(first) {
                curr_module = sub;
                start_idx = 1;
            }
        }

        for &seg in &path[start_idx..path.len().saturating_sub(1)] {
            curr_module = curr_module.get_submodule(seg)?;
        }

        let last = path.last()?;
        if let Some(sig) = curr_module.get_function(*last) {
            return Some(ResolvedItem::Function(sig));
        }
        if let Some(def) = curr_module.get_type(*last) {
            return Some(ResolvedItem::Type(def));
        }
        if let Some(sub) = curr_module.get_submodule(*last) {
            return Some(ResolvedItem::Module(sub));
        }

        None
    }

    fn get_current_module(&self) -> &ModuleNamespace {
        let mut curr = &self.root;
        for &seg in &self.current_path {
            curr = curr.get_submodule(seg).expect("Path must be valid");
        }
        curr
    }

    fn get_parent_module(&self) -> Option<&ModuleNamespace> {
        if self.current_path.is_empty() {
            return None;
        }
        let mut curr = &self.root;
        for &seg in &self.current_path[..self.current_path.len() - 1] {
            curr = curr.get_submodule(seg).expect("Path must be valid");
        }
        Some(curr)
    }

    pub fn get_type(&self, name: Spur) -> Option<&TypeDef> {
        self.types.get(&name)
    }

    pub fn define_function(&mut self, sig: FunctionSig) {
        self.functions.insert(sig.name, sig.clone());
        self.get_current_module_mut().add_function(sig);
    }

    pub fn has_function(&self, name: Spur) -> bool {
        self.functions.contains_key(&name)
    }

    pub fn mark_ambiguous(&mut self, name: Spur) {
        self.ambiguous_functions.insert(name);
    }

    pub fn is_ambiguous(&self, name: Spur) -> bool {
        self.ambiguous_functions.contains(&name)
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

    pub fn all_types(&self) -> impl Iterator<Item = (&Spur, &TypeDef)> {
        self.types.iter()
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
                })
                .collect(),
            type_params: def.type_params.clone(),
        }
    }

    pub fn to_function_type(&self, sig: &FunctionSig) -> FunctionType {
        FunctionType {
            params: sig.params.iter().map(|(_, ty)| ty.clone()).collect(),
            returns: Box::new(sig.return_ty.clone()),
            throws: sig.throws.clone(),
            is_variadic: sig.is_variadic,
        }
    }

    pub fn to_exception_type(&self, def: &ExceptionDef) -> Spur {
        def.name
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
            throws: vec![],
            is_public: true,
            is_variadic: false,
            span: Span::dummy(),
            module: None,
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
                type_params: vec![],
                params: vec![],
                return_ty: Type::Float,
                throws: vec![],
                is_public: true,
                span: Span::dummy(),
            },
        );

        assert!(table.get_method(point, distance).is_some());
    }
}
