///
/// Statement AST Nodes
///
/// This module defines all statement types in the naml language. Statements
/// are constructs that perform actions but don't necessarily produce values.
///
/// Key statement categories:
/// - Declarations: var, const
/// - Control flow: if, while, for, loop, switch, break, continue, return
/// - Expression statements: expressions used for side effects
/// - Error handling: throw
///
/// Design notes:
/// - VarStmt supports both `var x` and `var mut x` for mutability
/// - ForStmt supports optional index binding `for (i, val in collection)`
/// - IfStmt vs IfExpr: statements don't require else, expressions do
///

use crate::source::{Span, Spanned};
use super::expressions::{BlockExpr, Expression};
use super::operators::AssignOp;
use super::patterns::Pattern;
use super::types::{Ident, NamlType};

#[derive(Debug, Clone, PartialEq)]
pub enum Statement<'ast> {
    Var(VarStmt<'ast>),
    Const(ConstStmt<'ast>),
    Assign(AssignStmt<'ast>),
    Expression(ExprStmt<'ast>),
    Return(ReturnStmt<'ast>),
    Throw(ThrowStmt<'ast>),
    If(IfStmt<'ast>),
    While(WhileStmt<'ast>),
    For(ForStmt<'ast>),
    Loop(LoopStmt<'ast>),
    Switch(SwitchStmt<'ast>),
    Break(BreakStmt),
    Continue(ContinueStmt),
    Block(BlockStmt<'ast>),
}

impl<'ast> Spanned for Statement<'ast> {
    fn span(&self) -> Span {
        match self {
            Statement::Var(s) => s.span,
            Statement::Const(s) => s.span,
            Statement::Assign(s) => s.span,
            Statement::Expression(s) => s.span,
            Statement::Return(s) => s.span,
            Statement::Throw(s) => s.span,
            Statement::If(s) => s.span,
            Statement::While(s) => s.span,
            Statement::For(s) => s.span,
            Statement::Loop(s) => s.span,
            Statement::Switch(s) => s.span,
            Statement::Break(s) => s.span,
            Statement::Continue(s) => s.span,
            Statement::Block(s) => s.span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VarStmt<'ast> {
    pub name: Ident,
    pub mutable: bool,
    pub ty: Option<NamlType>,
    pub init: Option<Expression<'ast>>,
    pub else_block: Option<BlockStmt<'ast>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstStmt<'ast> {
    pub name: Ident,
    pub ty: Option<NamlType>,
    pub init: Expression<'ast>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssignStmt<'ast> {
    pub target: Expression<'ast>,
    pub op: AssignOp,
    pub value: Expression<'ast>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExprStmt<'ast> {
    pub expr: Expression<'ast>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnStmt<'ast> {
    pub value: Option<Expression<'ast>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThrowStmt<'ast> {
    pub value: Expression<'ast>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt<'ast> {
    pub condition: Expression<'ast>,
    pub then_branch: BlockStmt<'ast>,
    pub else_branch: Option<ElseBranch<'ast>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ElseBranch<'ast> {
    ElseIf(Box<IfStmt<'ast>>),
    Else(BlockStmt<'ast>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhileStmt<'ast> {
    pub condition: Expression<'ast>,
    pub body: BlockStmt<'ast>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForStmt<'ast> {
    pub index: Option<Ident>,
    pub value: Ident,
    pub ty: Option<NamlType>,
    pub iterable: Expression<'ast>,
    pub body: BlockStmt<'ast>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoopStmt<'ast> {
    pub body: BlockStmt<'ast>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchStmt<'ast> {
    pub scrutinee: Expression<'ast>,
    pub cases: Vec<SwitchCase<'ast>>,
    pub default: Option<BlockStmt<'ast>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase<'ast> {
    pub pattern: Pattern<'ast>,
    pub body: BlockStmt<'ast>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BreakStmt {
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContinueStmt {
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockStmt<'ast> {
    pub statements: Vec<Statement<'ast>>,
    pub span: Span,
}

impl<'ast> BlockStmt<'ast> {
    pub fn new(statements: Vec<Statement<'ast>>, span: Span) -> Self {
        Self { statements, span }
    }

    pub fn empty(span: Span) -> Self {
        Self {
            statements: Vec::new(),
            span,
        }
    }
}

impl<'ast> From<BlockExpr<'ast>> for BlockStmt<'ast> {
    fn from(expr: BlockExpr<'ast>) -> Self {
        let mut statements = expr.statements;
        if let Some(tail) = expr.tail {
            statements.push(Statement::Expression(ExprStmt {
                span: tail.span(),
                expr: tail.clone(),
            }));
        }
        BlockStmt {
            statements,
            span: expr.span,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_stmt_empty() {
        let block: BlockStmt = BlockStmt::empty(Span::new(0, 2, 0));
        assert!(block.statements.is_empty());
    }

    #[test]
    fn test_statement_span() {
        let stmt: Statement = Statement::Break(BreakStmt {
            span: Span::new(10, 15, 0),
        });
        assert_eq!(stmt.span(), Span::new(10, 15, 0));
    }
}
