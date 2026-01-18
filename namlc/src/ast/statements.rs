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
use super::types::{Ident, NamlType};

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Var(VarStmt),
    Const(ConstStmt),
    Assign(AssignStmt),
    Expression(ExprStmt),
    Return(ReturnStmt),
    Throw(ThrowStmt),
    If(IfStmt),
    While(WhileStmt),
    For(ForStmt),
    Loop(LoopStmt),
    Switch(SwitchStmt),
    Break(BreakStmt),
    Continue(ContinueStmt),
    Block(BlockStmt),
}

impl Spanned for Statement {
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
pub struct VarStmt {
    pub name: Ident,
    pub mutable: bool,
    pub ty: Option<NamlType>,
    pub init: Option<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstStmt {
    pub name: Ident,
    pub ty: Option<NamlType>,
    pub init: Expression,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssignStmt {
    pub target: Expression,
    pub op: AssignOp,
    pub value: Expression,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExprStmt {
    pub expr: Expression,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnStmt {
    pub value: Option<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThrowStmt {
    pub value: Expression,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt {
    pub condition: Expression,
    pub then_branch: BlockStmt,
    pub else_branch: Option<ElseBranch>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ElseBranch {
    ElseIf(Box<IfStmt>),
    Else(BlockStmt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhileStmt {
    pub condition: Expression,
    pub body: BlockStmt,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForStmt {
    pub index: Option<Ident>,
    pub value: Ident,
    pub ty: Option<NamlType>,
    pub iterable: Expression,
    pub body: BlockStmt,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoopStmt {
    pub body: BlockStmt,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchStmt {
    pub scrutinee: Expression,
    pub cases: Vec<SwitchCase>,
    pub default: Option<BlockStmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase {
    pub pattern: Expression,
    pub body: BlockStmt,
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
pub struct BlockStmt {
    pub statements: Vec<Statement>,
    pub span: Span,
}

impl BlockStmt {
    pub fn new(statements: Vec<Statement>, span: Span) -> Self {
        Self { statements, span }
    }

    pub fn empty(span: Span) -> Self {
        Self {
            statements: Vec::new(),
            span,
        }
    }
}

impl From<BlockExpr> for BlockStmt {
    fn from(expr: BlockExpr) -> Self {
        let mut statements = expr.statements;
        if let Some(tail) = expr.tail {
            statements.push(Statement::Expression(ExprStmt {
                span: tail.span(),
                expr: *tail,
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
        let block = BlockStmt::empty(Span::new(0, 2, 0));
        assert!(block.statements.is_empty());
    }

    #[test]
    fn test_statement_span() {
        let stmt = Statement::Break(BreakStmt {
            span: Span::new(10, 15, 0),
        });
        assert_eq!(stmt.span(), Span::new(10, 15, 0));
    }
}
