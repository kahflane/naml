///
/// Abstract Syntax Tree Module
///
/// This module defines the complete AST for the naml programming language.
/// The AST is the intermediate representation produced by the parser and
/// consumed by the type checker and code generators.
///
/// Module structure:
/// - types: Core type system (Ident, NamlType)
/// - literals: Literal values (int, float, string, etc.)
/// - operators: Binary, unary, and assignment operators
/// - expressions: All expression node types
/// - statements: All statement node types
/// - items: Top-level declarations (functions, structs, etc.)
/// - visitor: Visitor pattern for AST traversal
///
/// The root AST node is SourceFile, representing a complete naml source file.
///

pub mod arena;
pub mod expressions;
pub mod items;
pub mod literals;
pub mod operators;
pub mod patterns;
pub mod statements;
pub mod types;
pub mod visitor;

pub use arena::AstArena;
pub use expressions::*;
pub use items::*;
pub use literals::*;
pub use operators::*;
pub use patterns::*;
pub use statements::*;
pub use types::*;
pub use visitor::*;

use crate::source::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct SourceFile<'ast> {
    pub items: Vec<Item<'ast>>,
    pub span: Span,
}

impl<'ast> SourceFile<'ast> {
    pub fn new(items: Vec<Item<'ast>>, span: Span) -> Self {
        Self { items, span }
    }

    pub fn empty() -> Self {
        Self {
            items: Vec::new(),
            span: Span::dummy(),
        }
    }

    pub fn functions(&self) -> impl Iterator<Item = &FunctionItem<'ast>> {
        self.items.iter().filter_map(|item| {
            if let Item::Function(f) = item {
                Some(f)
            } else {
                None
            }
        })
    }

    pub fn structs(&self) -> impl Iterator<Item = &StructItem> {
        self.items.iter().filter_map(|item| {
            if let Item::Struct(s) = item {
                Some(s)
            } else {
                None
            }
        })
    }

    pub fn find_main(&self) -> Option<&FunctionItem<'ast>> {
        self.functions().find(|f| {
            f.name.symbol == lasso::Spur::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_file_empty() {
        let file: SourceFile = SourceFile::empty();
        assert!(file.items.is_empty());
    }

    #[test]
    fn test_source_file_iterators() {
        let file: SourceFile = SourceFile::empty();
        assert_eq!(file.functions().count(), 0);
        assert_eq!(file.structs().count(), 0);
    }
}
