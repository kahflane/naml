//!
//! Typed AST Annotations
//!
//! This module provides a structure to store resolved type information for
//! expressions during type checking. The TypeAnnotations map allows the code
//! generator to look up the type of any expression by its source span.
//!
//! Design decisions:
//! - Uses Span as key (Copy, Hash) for zero-copy lookup without AST modification
//! - Stores resolved types (TypeVar bindings followed) for codegen use
//! - Tracks additional metadata like lvalue status and clone requirements
//! - Separate from AST to maintain clean separation between parse and check phases
//!
//! Usage flow:
//! 1. TypeInferrer records annotations during inference via annotate()
//! 2. TypeChecker returns TypeAnnotations alongside errors
//! 3. Codegen uses get_type() and needs_clone() for type-aware generation
//!

use std::collections::HashMap;

use crate::source::Span;
use super::types::Type;

#[derive(Debug, Clone)]
pub struct ExprTypeInfo {
    pub ty: Type,
    pub is_lvalue: bool,
    pub needs_clone: bool,
}

impl ExprTypeInfo {
    pub fn new(ty: Type) -> Self {
        Self {
            ty,
            is_lvalue: false,
            needs_clone: false,
        }
    }

    pub fn with_lvalue(mut self, is_lvalue: bool) -> Self {
        self.is_lvalue = is_lvalue;
        self
    }

    pub fn with_clone(mut self, needs_clone: bool) -> Self {
        self.needs_clone = needs_clone;
        self
    }
}

#[derive(Debug, Default)]
pub struct TypeAnnotations {
    expr_types: HashMap<Span, ExprTypeInfo>,
}

impl TypeAnnotations {
    pub fn new() -> Self {
        Self {
            expr_types: HashMap::new(),
        }
    }

    pub fn annotate(&mut self, span: Span, info: ExprTypeInfo) {
        self.expr_types.insert(span, info);
    }

    pub fn annotate_type(&mut self, span: Span, ty: Type) {
        self.expr_types.insert(span, ExprTypeInfo::new(ty));
    }

    pub fn get_type(&self, span: Span) -> Option<&Type> {
        self.expr_types.get(&span).map(|info| &info.ty)
    }

    pub fn get_info(&self, span: Span) -> Option<&ExprTypeInfo> {
        self.expr_types.get(&span)
    }

    pub fn needs_clone(&self, span: Span) -> bool {
        self.expr_types
            .get(&span)
            .map_or(false, |info| info.needs_clone)
    }

    pub fn is_lvalue(&self, span: Span) -> bool {
        self.expr_types
            .get(&span)
            .map_or(false, |info| info.is_lvalue)
    }

    pub fn len(&self) -> usize {
        self.expr_types.len()
    }

    pub fn is_empty(&self) -> bool {
        self.expr_types.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_annotations_basic() {
        let mut annotations = TypeAnnotations::new();
        let span = Span::new(0, 10, 0);

        annotations.annotate_type(span, Type::Int);

        assert_eq!(annotations.get_type(span), Some(&Type::Int));
        assert!(!annotations.needs_clone(span));
        assert!(!annotations.is_lvalue(span));
    }

    #[test]
    fn test_annotations_with_info() {
        let mut annotations = TypeAnnotations::new();
        let span = Span::new(0, 10, 0);

        let info = ExprTypeInfo::new(Type::String)
            .with_lvalue(true)
            .with_clone(true);
        annotations.annotate(span, info);

        assert_eq!(annotations.get_type(span), Some(&Type::String));
        assert!(annotations.needs_clone(span));
        assert!(annotations.is_lvalue(span));
    }

    #[test]
    fn test_annotations_missing() {
        let annotations = TypeAnnotations::new();
        let span = Span::new(0, 10, 0);

        assert_eq!(annotations.get_type(span), None);
        assert!(!annotations.needs_clone(span));
    }
}
