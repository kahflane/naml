//!
//! Statement Parser
//!
//! Parses statements using nom combinators.
//!

use nom::InputTake;

use crate::ast::*;
use crate::lexer::{Keyword, TokenKind};
use crate::source::Spanned;

use super::combinators::*;
use super::expressions::{parse_block, parse_expression};
use super::input::TokenStream;
use super::types::parse_type;

pub fn parse_statement<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Statement<'ast>> {
    match input.first().map(|t| t.kind) {
        Some(TokenKind::Keyword(Keyword::Var)) => parse_var_stmt(arena, input),
        Some(TokenKind::Keyword(Keyword::Const)) => parse_const_stmt(arena, input),
        Some(TokenKind::Keyword(Keyword::Return)) => parse_return_stmt(arena, input),
        Some(TokenKind::Keyword(Keyword::Throw)) => parse_throw_stmt(arena, input),
        Some(TokenKind::Keyword(Keyword::Break)) => parse_break_stmt(input),
        Some(TokenKind::Keyword(Keyword::Continue)) => parse_continue_stmt(input),
        Some(TokenKind::Keyword(Keyword::If)) => parse_if_stmt(arena, input),
        Some(TokenKind::Keyword(Keyword::While)) => parse_while_stmt(arena, input),
        Some(TokenKind::Keyword(Keyword::For)) => parse_for_stmt(arena, input),
        Some(TokenKind::Keyword(Keyword::Loop)) => parse_loop_stmt(arena, input),
        Some(TokenKind::Keyword(Keyword::Switch)) => parse_switch_stmt(arena, input),
        Some(TokenKind::Keyword(Keyword::Locked)) => parse_locked_stmt(arena, input, LockKind::Exclusive),
        Some(TokenKind::Keyword(Keyword::Rlocked)) => parse_locked_stmt(arena, input, LockKind::Read),
        Some(TokenKind::Keyword(Keyword::Wlocked)) => parse_locked_stmt(arena, input, LockKind::Write),
        Some(TokenKind::LBrace) => parse_block_stmt(arena, input),
        _ => parse_expr_or_assign_stmt(arena, input),
    }
}

fn parse_var_stmt<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Statement<'ast>> {
    let (input, start) = keyword(Keyword::Var)(input)?;

    // var is mutable by default - mut is not allowed
    if check_keyword(Keyword::Mut)(input) {
        return Err(nom::Err::Error(PError {
            input,
            kind: PErrorKind::MutNotAllowedOnVar,
        }));
    }
    let mutable = true;

    let (input, name) = ident(input)?;

    // Type annotation is required: var x: Type = value;
    if !check(TokenKind::Colon)(input) {
        return Err(nom::Err::Error(PError {
            input,
            kind: PErrorKind::ExpectedTypeAnnotation,
        }));
    }
    let (input, _) = token(TokenKind::Colon)(input)?;
    let (input, parsed_ty) = parse_type(input)?;
    let ty = Some(parsed_ty);

    let (input, init) = if check(TokenKind::Eq)(input) {
        let (input, _) = token(TokenKind::Eq)(input)?;
        let (input, expr) = parse_expression(arena, input)?;
        (input, Some(expr))
    } else {
        (input, None)
    };

    let (input, else_block) = if check_keyword(Keyword::Else)(input) {
        let (input, _) = keyword(Keyword::Else)(input)?;
        let (input, block) = parse_block_stmt(arena, input)?;
        if let Statement::Block(block_stmt) = block {
            (input, Some(block_stmt))
        } else {
            (input, None)
        }
    } else {
        (input, None)
    };

    let input = if else_block.is_none() {
        let (input, _) = token(TokenKind::Semicolon)(input)?;
        input
    } else {
        input
    };

    Ok((
        input,
        Statement::Var(VarStmt {
            name,
            mutable,
            ty,
            init,
            else_block,
            span: start.span,
        }),
    ))
}

fn parse_const_stmt<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Statement<'ast>> {
    let (input, start) = keyword(Keyword::Const)(input)?;
    let (input, name) = ident(input)?;

    // Type annotation is required: const x: Type = value;
    if !check(TokenKind::Colon)(input) {
        return Err(nom::Err::Error(PError {
            input,
            kind: PErrorKind::ExpectedTypeAnnotation,
        }));
    }
    let (input, _) = token(TokenKind::Colon)(input)?;
    let (input, parsed_ty) = parse_type(input)?;
    let ty = Some(parsed_ty);

    let (input, _) = token(TokenKind::Eq)(input)?;
    let (input, init) = parse_expression(arena, input)?;
    let (input, _) = token(TokenKind::Semicolon)(input)?;

    Ok((
        input,
        Statement::Const(ConstStmt {
            name,
            ty,
            init,
            span: start.span,
        }),
    ))
}

fn parse_return_stmt<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Statement<'ast>> {
    let (input, start) = keyword(Keyword::Return)(input)?;

    let (input, value) = if !check(TokenKind::Semicolon)(input) {
        let (input, expr) = parse_expression(arena, input)?;
        (input, Some(expr))
    } else {
        (input, None)
    };

    let (input, _) = token(TokenKind::Semicolon)(input)?;

    Ok((
        input,
        Statement::Return(ReturnStmt {
            value,
            span: start.span,
        }),
    ))
}

fn parse_throw_stmt<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Statement<'ast>> {
    let (input, start) = keyword(Keyword::Throw)(input)?;
    let (input, value) = parse_expression(arena, input)?;
    let (input, _) = token(TokenKind::Semicolon)(input)?;

    Ok((
        input,
        Statement::Throw(ThrowStmt {
            value,
            span: start.span,
        }),
    ))
}

fn parse_break_stmt<'a, 'ast>(input: TokenStream<'a>) -> PResult<'a, Statement<'ast>> {
    let (input, tok) = keyword(Keyword::Break)(input)?;
    let (input, _) = token(TokenKind::Semicolon)(input)?;
    Ok((input, Statement::Break(BreakStmt { span: tok.span })))
}

fn parse_continue_stmt<'a, 'ast>(input: TokenStream<'a>) -> PResult<'a, Statement<'ast>> {
    let (input, tok) = keyword(Keyword::Continue)(input)?;
    let (input, _) = token(TokenKind::Semicolon)(input)?;
    Ok((input, Statement::Continue(ContinueStmt { span: tok.span })))
}

fn parse_if_stmt<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Statement<'ast>> {
    let (input, start) = keyword(Keyword::If)(input)?;
    let (input, _) = token(TokenKind::LParen)(input)?;
    let (input, condition) = parse_expression(arena, input)?;
    let (input, _) = token(TokenKind::RParen)(input)?;

    let (input, then_block) = parse_block(arena, input)?;

    let (input, else_branch) = if check_keyword(Keyword::Else)(input) {
        let (input, _) = keyword(Keyword::Else)(input)?;
        if check_keyword(Keyword::If)(input) {
            let (input, else_if) = parse_if_stmt(arena, input)?;
            if let Statement::If(if_stmt) = else_if {
                (input, Some(ElseBranch::ElseIf(Box::new(if_stmt))))
            } else {
                (input, None)
            }
        } else {
            let (input, else_block) = parse_block(arena, input)?;
            (input, Some(ElseBranch::Else(else_block)))
        }
    } else {
        (input, None)
    };

    let end_span = else_branch
        .as_ref()
        .map(|e| match e {
            ElseBranch::ElseIf(i) => i.span,
            ElseBranch::Else(b) => b.span,
        })
        .unwrap_or(then_block.span);

    Ok((
        input,
        Statement::If(IfStmt {
            condition,
            then_branch: then_block,
            else_branch,
            span: start.span.merge(end_span),
        }),
    ))
}

fn parse_while_stmt<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Statement<'ast>> {
    let (input, start) = keyword(Keyword::While)(input)?;
    let (input, _) = token(TokenKind::LParen)(input)?;
    let (input, condition) = parse_expression(arena, input)?;
    let (input, _) = token(TokenKind::RParen)(input)?;
    let (input, body) = parse_block(arena, input)?;
    let body_span = body.span;

    Ok((
        input,
        Statement::While(WhileStmt {
            condition,
            body,
            span: start.span.merge(body_span),
        }),
    ))
}

fn parse_for_stmt<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Statement<'ast>> {
    let (input, start) = keyword(Keyword::For)(input)?;
    let (input, _) = token(TokenKind::LParen)(input)?;

    let (input, first) = ident(input)?;
    let (input, first_ty) = if check(TokenKind::Colon)(input) {
        let (input, _) = token(TokenKind::Colon)(input)?;
        let (input, ty) = parse_type(input)?;
        (input, Some(ty))
    } else {
        (input, None)
    };

    let (input, index, index_ty, value, value_ty) = if check(TokenKind::Comma)(input) {
        let (input, _) = token(TokenKind::Comma)(input)?;
        let (input, second) = ident(input)?;
        let (input, second_ty) = if check(TokenKind::Colon)(input) {
            let (input, _) = token(TokenKind::Colon)(input)?;
            let (input, ty) = parse_type(input)?;
            (input, Some(ty))
        } else {
            (input, None)
        };
        (input, Some(first), first_ty, second, second_ty)
    } else {
        (input, None, None, first, first_ty)
    };

    let (input, _) = keyword(Keyword::In)(input)?;
    let (input, iterable) = parse_expression(arena, input)?;
    let (input, _) = token(TokenKind::RParen)(input)?;
    let (input, body) = parse_block(arena, input)?;
    let body_span = body.span;

    Ok((
        input,
        Statement::For(ForStmt {
            index,
            index_ty,
            value,
            value_ty,
            iterable,
            body,
            span: start.span.merge(body_span),
        }),
    ))
}

fn parse_loop_stmt<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Statement<'ast>> {
    let (input, start) = keyword(Keyword::Loop)(input)?;
    let (input, body) = parse_block(arena, input)?;
    let body_span = body.span;

    Ok((
        input,
        Statement::Loop(LoopStmt {
            body,
            span: start.span.merge(body_span),
        }),
    ))
}

fn parse_switch_stmt<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Statement<'ast>> {
    let (input, start) = keyword(Keyword::Switch)(input)?;
    let (input, _) = token(TokenKind::LParen)(input)?;
    let (input, scrutinee) = parse_expression(arena, input)?;
    let (input, _) = token(TokenKind::RParen)(input)?;
    let (input, _) = token(TokenKind::LBrace)(input)?;

    let mut cases = Vec::new();
    let mut default = None;
    let mut input = input;

    loop {
        if check(TokenKind::RBrace)(input) {
            break;
        }

        if check_keyword(Keyword::Case)(input) {
            let (new_input, _) = keyword(Keyword::Case)(input)?;
            let (new_input, pattern) = super::parse_pattern(new_input)?;
            let pattern_span = pattern.span();
            let (new_input, _) = token(TokenKind::Colon)(new_input)?;
            let (new_input, body) = parse_block(arena, new_input)?;
            let body_span = body.span;
            cases.push(SwitchCase {
                pattern,
                body,
                span: pattern_span.merge(body_span),
            });
            input = new_input;
        } else if check_keyword(Keyword::Default)(input) {
            let (new_input, _) = keyword(Keyword::Default)(input)?;
            let (new_input, _) = token(TokenKind::Colon)(new_input)?;
            let (new_input, body) = parse_block(arena, new_input)?;
            default = Some(body);
            input = new_input;
        } else {
            break;
        }
    }

    let (input, end) = token(TokenKind::RBrace)(input)?;

    Ok((
        input,
        Statement::Switch(SwitchStmt {
            scrutinee,
            cases,
            default,
            span: start.span.merge(end.span),
        }),
    ))
}

fn parse_locked_stmt<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
    kind: LockKind,
) -> PResult<'a, Statement<'ast>> {
    let (input, start) = match kind {
        LockKind::Exclusive => keyword(Keyword::Locked)(input)?,
        LockKind::Read => keyword(Keyword::Rlocked)(input)?,
        LockKind::Write => keyword(Keyword::Wlocked)(input)?,
    };

    let (input, _) = token(TokenKind::LParen)(input)?;

    let (input, binding) = ident(input)?;
    let (input, binding_ty) = if check(TokenKind::Colon)(input) {
        let (input, _) = token(TokenKind::Colon)(input)?;
        let (input, ty) = parse_type(input)?;
        (input, Some(ty))
    } else {
        (input, None)
    };

    let (input, _) = keyword(Keyword::In)(input)?;
    let (input, mutex) = parse_expression(arena, input)?;
    let (input, _) = token(TokenKind::RParen)(input)?;
    let (input, body) = parse_block(arena, input)?;
    let body_span = body.span;

    Ok((
        input,
        Statement::Locked(LockedStmt {
            kind,
            binding,
            binding_ty,
            mutex,
            body,
            span: start.span.merge(body_span),
        }),
    ))
}

fn parse_block_stmt<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Statement<'ast>> {
    if !check(TokenKind::LBrace)(input) {
        return Err(nom::Err::Error(PError {
            input,
            kind: PErrorKind::ExpectedStatement,
        }));
    }

    let (input, block) = parse_block(arena, input)?;
    Ok((input, Statement::Block(block)))
}

fn parse_expr_or_assign_stmt<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Statement<'ast>> {
    let (input, expr) = parse_expression(arena, input)?;

    let assign_op = match peek_token(input) {
        Some(TokenKind::Eq) => Some(AssignOp::Assign),
        Some(TokenKind::PlusEq) => Some(AssignOp::AddAssign),
        Some(TokenKind::MinusEq) => Some(AssignOp::SubAssign),
        Some(TokenKind::StarEq) => Some(AssignOp::MulAssign),
        Some(TokenKind::SlashEq) => Some(AssignOp::DivAssign),
        Some(TokenKind::PercentEq) => Some(AssignOp::ModAssign),
        Some(TokenKind::AmpersandEq) => Some(AssignOp::BitAndAssign),
        Some(TokenKind::PipeEq) => Some(AssignOp::BitOrAssign),
        Some(TokenKind::CaretEq) => Some(AssignOp::BitXorAssign),
        _ => None,
    };

    if let Some(op) = assign_op {
        let (input, _) = input.take_split(1);
        let (input, value) = parse_expression(arena, input)?;
        let (input, _) = token(TokenKind::Semicolon)(input)?;
        let span = expr.span().merge(value.span());

        Ok((
            input,
            Statement::Assign(AssignStmt {
                target: expr,
                op,
                value,
                span,
            }),
        ))
    } else {
        let (input, _) = token(TokenKind::Semicolon)(input)?;
        let span = expr.span();

        Ok((
            input,
            Statement::Expression(ExprStmt { expr, span }),
        ))
    }
}
