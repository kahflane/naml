//!
//! Top-Level Item AST Nodes
//!
//! This module defines all top-level items that can appear in a naml source
//! file. Items are declarations that define named entities in the program.
//!
//! Key item types:
//! - FunctionItem: Function and method definitions
//! - StructItem: Struct type definitions with fields
//! - InterfaceItem: Interface/trait definitions
//! - EnumItem: Enum type definitions with variants
//! - ExceptionItem: Exception type definitions
//! - UseItem: Module imports
//! - UseItem: Type/function imports from modules
//! - ExternItem: External function declarations
//!
//! Platform annotations:
//! - Functions can be marked with #[platforms(native, server, browser)]
//! - Platform-specific implementations are handled at codegen time
//!

use crate::source::{Span, Spanned};
use super::statements::{BlockStmt, Statement};
use super::types::{Ident, NamlType};

#[derive(Debug, Clone, PartialEq)]
pub enum Item<'ast> {
    Function(FunctionItem<'ast>),
    Struct(StructItem),
    Interface(InterfaceItem),
    Enum(EnumItem),
    Exception(ExceptionItem),
    Use(UseItem),
    Extern(ExternItem),
    TopLevelStmt(TopLevelStmtItem<'ast>),
}

impl<'ast> Spanned for Item<'ast> {
    fn span(&self) -> Span {
        match self {
            Item::Function(i) => i.span,
            Item::Struct(i) => i.span,
            Item::Interface(i) => i.span,
            Item::Enum(i) => i.span,
            Item::Exception(i) => i.span,
            Item::Use(i) => i.span,
            Item::Extern(i) => i.span,
            Item::TopLevelStmt(i) => i.span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TopLevelStmtItem<'ast> {
    pub stmt: Statement<'ast>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Platform {
    Native,
    Server,
    Browser,
    All,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Platforms {
    pub platforms: Vec<Platform>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericParam {
    pub name: Ident,
    pub bounds: Vec<NamlType>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: Ident,
    pub ty: NamlType,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Receiver {
    pub name: Ident,
    pub ty: NamlType,
    pub mutable: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionItem<'ast> {
    pub name: Ident,
    pub receiver: Option<Receiver>,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Parameter>,
    pub return_ty: Option<NamlType>,
    pub throws: Vec<NamlType>,
    pub is_public: bool,
    pub body: Option<BlockStmt<'ast>>,
    pub platforms: Option<Platforms>,
    pub span: Span,
}

impl<'ast> FunctionItem<'ast> {
    pub fn is_method(&self) -> bool {
        self.receiver.is_some()
    }

    pub fn is_abstract(&self) -> bool {
        self.body.is_none()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: Ident,
    pub ty: NamlType,
    pub is_public: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructItem {
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub implements: Vec<NamlType>,
    pub fields: Vec<StructField>,
    pub is_public: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceMethod {
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Parameter>,
    pub return_ty: Option<NamlType>,
    pub throws: Vec<NamlType>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceItem {
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub extends: Vec<NamlType>,
    pub methods: Vec<InterfaceMethod>,
    pub is_public: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: Ident,
    pub fields: Option<Vec<NamlType>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumItem {
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub variants: Vec<EnumVariant>,
    pub is_public: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExceptionField {
    pub name: Ident,
    pub ty: NamlType,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExceptionItem {
    pub name: Ident,
    pub fields: Vec<ExceptionField>,
    pub is_public: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UseItem {
    pub path: Vec<Ident>,
    pub items: UseItems,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UseItems {
    All,
    Specific(Vec<UseItemEntry>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct UseItemEntry {
    pub name: Ident,
    pub alias: Option<Ident>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExternItem {
    pub name: Ident,
    pub params: Vec<Parameter>,
    pub return_ty: Option<NamlType>,
    pub throws: Vec<NamlType>,
    pub link_name: Option<Ident>,
    pub span: Span,
}

#[cfg(test)]
mod tests {
    use super::*;
    use lasso::Rodeo;

    fn make_ident(rodeo: &mut Rodeo, name: &str) -> Ident {
        Ident::new(rodeo.get_or_intern(name), Span::dummy())
    }

    #[test]
    fn test_function_is_method() {
        let mut rodeo = Rodeo::default();
        let func: FunctionItem = FunctionItem {
            name: make_ident(&mut rodeo, "foo"),
            receiver: None,
            generics: vec![],
            params: vec![],
            return_ty: None,
            throws: vec![],
            is_public: false,
            body: Some(BlockStmt::empty(Span::dummy())),
            platforms: None,
            span: Span::dummy(),
        };
        assert!(!func.is_method());
        assert!(!func.is_abstract());
    }

    #[test]
    fn test_item_span() {
        let mut rodeo = Rodeo::default();
        let item: Item = Item::Exception(ExceptionItem {
            name: make_ident(&mut rodeo, "MyError"),
            fields: vec![],
            is_public: true,
            span: Span::new(0, 50, 0),
        });
        assert_eq!(item.span(), Span::new(0, 50, 0));
    }
}
