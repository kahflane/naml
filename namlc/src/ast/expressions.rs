///
/// Expression AST Nodes
///
/// This module defines all expression types in the naml language. Expressions
/// are constructs that evaluate to a value.
///
/// Key design decisions:
/// - Wrapper enum with separate structs for each expression type
/// - Each struct carries its own Span for precise error reporting
/// - Box-based nesting for recursive structures
/// - All types implement Spanned trait for uniform span access
///
/// Expression categories:
/// - Atoms: literals, identifiers, grouped expressions
/// - Operators: binary, unary operations
/// - Access: field access, indexing, method calls
/// - Control: if expressions, blocks, spawn, await
/// - Constructors: array literals, map literals, lambdas
///

use crate::source::{Span, Spanned};
use super::literals::Literal;
use super::operators::{BinaryOp, UnaryOp};
use super::types::{Ident, NamlType};

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Literal(LiteralExpr),
    Identifier(IdentExpr),
    Path(PathExpr),
    Binary(BinaryExpr),
    Unary(UnaryExpr),
    Call(CallExpr),
    MethodCall(MethodCallExpr),
    Index(IndexExpr),
    Field(FieldExpr),
    Array(ArrayExpr),
    Map(MapExpr),
    StructLiteral(StructLiteralExpr),
    If(IfExpr),
    Block(BlockExpr),
    Lambda(LambdaExpr),
    Spawn(SpawnExpr),
    Await(AwaitExpr),
    Try(TryExpr),
    Cast(CastExpr),
    Range(RangeExpr),
    Grouped(GroupedExpr),
    Some(SomeExpr),
}

impl Spanned for Expression {
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
pub struct BinaryExpr {
    pub left: Box<Expression>,
    pub op: BinaryOp,
    pub right: Box<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnaryExpr {
    pub op: UnaryOp,
    pub operand: Box<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallExpr {
    pub callee: Box<Expression>,
    pub type_args: Vec<NamlType>,
    pub args: Vec<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodCallExpr {
    pub receiver: Box<Expression>,
    pub method: Ident,
    pub type_args: Vec<NamlType>,
    pub args: Vec<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndexExpr {
    pub base: Box<Expression>,
    pub index: Box<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldExpr {
    pub base: Box<Expression>,
    pub field: Ident,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArrayExpr {
    pub elements: Vec<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapEntry {
    pub key: Expression,
    pub value: Expression,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapExpr {
    pub entries: Vec<MapEntry>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructLiteralField {
    pub name: Ident,
    pub value: Expression,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructLiteralExpr {
    pub name: Ident,
    pub fields: Vec<StructLiteralField>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfExpr {
    pub condition: Box<Expression>,
    pub then_branch: Box<BlockExpr>,
    pub else_branch: Option<ElseExpr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ElseExpr {
    ElseIf(Box<IfExpr>),
    Else(Box<BlockExpr>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockExpr {
    pub statements: Vec<super::statements::Statement>,
    pub tail: Option<Box<Expression>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LambdaParam {
    pub name: Ident,
    pub ty: Option<NamlType>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LambdaExpr {
    pub params: Vec<LambdaParam>,
    pub return_ty: Option<NamlType>,
    pub body: Box<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpawnExpr {
    pub body: Box<BlockExpr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AwaitExpr {
    pub expr: Box<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TryExpr {
    pub expr: Box<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CastExpr {
    pub expr: Box<Expression>,
    pub target_ty: NamlType,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RangeExpr {
    pub start: Option<Box<Expression>>,
    pub end: Option<Box<Expression>>,
    pub inclusive: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupedExpr {
    pub inner: Box<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SomeExpr {
    pub value: Box<Expression>,
    pub span: Span,
}

impl LiteralExpr {
    pub fn new(value: Literal, span: Span) -> Self {
        Self { value, span }
    }
}

impl BinaryExpr {
    pub fn new(left: Expression, op: BinaryOp, right: Expression) -> Self {
        let span = left.span().merge(right.span());
        Self {
            left: Box::new(left),
            op,
            right: Box::new(right),
            span,
        }
    }
}

impl UnaryExpr {
    pub fn new(op: UnaryOp, operand: Expression, span: Span) -> Self {
        Self {
            op,
            operand: Box::new(operand),
            span,
        }
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

    #[test]
    fn test_binary_expr_span_merge() {
        let left = Expression::Literal(LiteralExpr {
            value: Literal::Int(1),
            span: Span::new(0, 1, 0),
        });
        let right = Expression::Literal(LiteralExpr {
            value: Literal::Int(2),
            span: Span::new(4, 5, 0),
        });
        let binary = BinaryExpr::new(left, BinaryOp::Add, right);
        assert_eq!(binary.span, Span::new(0, 5, 0));
    }
}
