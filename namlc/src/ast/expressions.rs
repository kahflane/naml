//!
//! Expression AST Nodes
//!
//! This module defines all expression types in the naml language. Expressions
//! are constructs that evaluate to a value.
//!
//! Key design decisions:
//! - Wrapper enum with separate structs for each expression type
//! - Each struct carries its own Span for precise error reporting
//! - Arena-allocated references for recursive structures (zero Box overhead)
//! - All types implement Spanned trait for uniform span access
//!
//! Expression categories:
//! - Atoms: literals, identifiers, grouped expressions
//! - Operators: binary, unary operations
//! - Access: field access, indexing, method calls
//! - Control: if expressions, blocks, spawn, await
//! - Constructors: array literals, map literals, lambdas
//!

use crate::source::{Span, Spanned};
use super::literals::Literal;
use super::operators::{BinaryOp, UnaryOp};
use super::types::{Ident, NamlType};

#[derive(Debug, Clone, PartialEq)]
pub enum Expression<'ast> {
    Literal(LiteralExpr),
    Identifier(IdentExpr),
    Path(PathExpr),
    Binary(BinaryExpr<'ast>),
    Unary(UnaryExpr<'ast>),
    Call(CallExpr<'ast>),
    MethodCall(MethodCallExpr<'ast>),
    Index(IndexExpr<'ast>),
    Field(FieldExpr<'ast>),
    Array(ArrayExpr<'ast>),
    Map(MapExpr<'ast>),
    StructLiteral(StructLiteralExpr<'ast>),
    If(IfExpr<'ast>),
    Block(BlockExpr<'ast>),
    Lambda(LambdaExpr<'ast>),
    Spawn(SpawnExpr<'ast>),
    Await(AwaitExpr<'ast>),
    Try(TryExpr<'ast>),
    Catch(CatchExpr<'ast>),
    Cast(CastExpr<'ast>),
    Range(RangeExpr<'ast>),
    Grouped(GroupedExpr<'ast>),
    Some(SomeExpr<'ast>),
}

impl<'ast> Spanned for Expression<'ast> {
    fn span(&self) -> Span {
        match self {
            Expression::Literal(e) => e.span,
            Expression::Identifier(e) => e.span,
            Expression::Path(e) => e.span,
            Expression::Binary(e) => e.span,
            Expression::Unary(e) => e.span,
            Expression::Call(e) => e.span,
            Expression::MethodCall(e) => e.span,
            Expression::Index(e) => e.span,
            Expression::Field(e) => e.span,
            Expression::Array(e) => e.span,
            Expression::Map(e) => e.span,
            Expression::StructLiteral(e) => e.span,
            Expression::If(e) => e.span,
            Expression::Block(e) => e.span,
            Expression::Lambda(e) => e.span,
            Expression::Spawn(e) => e.span,
            Expression::Await(e) => e.span,
            Expression::Try(e) => e.span,
            Expression::Catch(e) => e.span,
            Expression::Cast(e) => e.span,
            Expression::Range(e) => e.span,
            Expression::Grouped(e) => e.span,
            Expression::Some(e) => e.span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LiteralExpr {
    pub value: Literal,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IdentExpr {
    pub ident: Ident,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PathExpr {
    pub segments: Vec<Ident>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BinaryExpr<'ast> {
    pub left: &'ast Expression<'ast>,
    pub op: BinaryOp,
    pub right: &'ast Expression<'ast>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnaryExpr<'ast> {
    pub op: UnaryOp,
    pub operand: &'ast Expression<'ast>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallExpr<'ast> {
    pub callee: &'ast Expression<'ast>,
    pub type_args: Vec<NamlType>,
    pub args: Vec<Expression<'ast>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodCallExpr<'ast> {
    pub receiver: &'ast Expression<'ast>,
    pub method: Ident,
    pub type_args: Vec<NamlType>,
    pub args: Vec<Expression<'ast>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndexExpr<'ast> {
    pub base: &'ast Expression<'ast>,
    pub index: &'ast Expression<'ast>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldExpr<'ast> {
    pub base: &'ast Expression<'ast>,
    pub field: Ident,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArrayExpr<'ast> {
    pub elements: Vec<Expression<'ast>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapEntry<'ast> {
    pub key: Expression<'ast>,
    pub value: Expression<'ast>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapExpr<'ast> {
    pub entries: Vec<MapEntry<'ast>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructLiteralField<'ast> {
    pub name: Ident,
    pub value: Expression<'ast>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructLiteralExpr<'ast> {
    pub name: Ident,
    pub fields: Vec<StructLiteralField<'ast>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfExpr<'ast> {
    pub condition: &'ast Expression<'ast>,
    pub then_branch: &'ast BlockExpr<'ast>,
    pub else_branch: Option<ElseExpr<'ast>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ElseExpr<'ast> {
    ElseIf(&'ast IfExpr<'ast>),
    Else(&'ast BlockExpr<'ast>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockExpr<'ast> {
    pub statements: Vec<super::statements::Statement<'ast>>,
    pub tail: Option<&'ast Expression<'ast>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LambdaParam {
    pub name: Ident,
    pub ty: Option<NamlType>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LambdaExpr<'ast> {
    pub params: Vec<LambdaParam>,
    pub return_ty: Option<NamlType>,
    pub body: &'ast Expression<'ast>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpawnExpr<'ast> {
    pub body: &'ast BlockExpr<'ast>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AwaitExpr<'ast> {
    pub expr: &'ast Expression<'ast>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TryExpr<'ast> {
    pub expr: &'ast Expression<'ast>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CatchExpr<'ast> {
    pub expr: &'ast Expression<'ast>,
    pub error_binding: Ident,
    pub handler: &'ast BlockExpr<'ast>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CastExpr<'ast> {
    pub expr: &'ast Expression<'ast>,
    pub target_ty: NamlType,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RangeExpr<'ast> {
    pub start: Option<&'ast Expression<'ast>>,
    pub end: Option<&'ast Expression<'ast>>,
    pub inclusive: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupedExpr<'ast> {
    pub inner: &'ast Expression<'ast>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SomeExpr<'ast> {
    pub value: &'ast Expression<'ast>,
    pub span: Span,
}

impl LiteralExpr {
    pub fn new(value: Literal, span: Span) -> Self {
        Self { value, span }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expression_span() {
        let lit = Expression::Literal(LiteralExpr {
            value: Literal::Int(42),
            span: Span::new(0, 2, 0),
        });
        assert_eq!(lit.span(), Span::new(0, 2, 0));
    }
}
