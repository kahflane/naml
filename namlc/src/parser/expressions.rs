///
/// Expression Parser
///
/// Parses expressions using nom combinators with Pratt-style precedence.
/// Uses arena allocation for all nested expression nodes.
///

use nom::branch::alt;
use nom::combinator::map;
use nom::{InputTake, Slice};

use crate::ast::*;
use crate::lexer::{Keyword, TokenKind};
use crate::source::{Span, Spanned};

use super::combinators::*;
use super::input::TokenStream;
use super::statements::parse_statement;
use super::types::parse_type;

pub fn parse_expression<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Expression<'ast>> {
    pratt_expr(arena, input, 0)
}

fn pratt_expr<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
    min_prec: u8,
) -> PResult<'a, Expression<'ast>> {
    let (mut input, mut left) = parse_unary(arena, input)?;

    loop {
        let Some(op) = peek_binary_op(input) else {
            break;
        };

        let prec = op.precedence();
        if prec < min_prec {
            break;
        }

        input = advance_binary_op(input);
        let (new_input, right) = pratt_expr(arena, input, prec + 1)?;
        input = new_input;

        let span = left.span().merge(right.span());
        left = Expression::Binary(BinaryExpr {
            left: arena.alloc(left),
            op,
            right: arena.alloc(right),
            span,
        });
    }

    Ok((input, left))
}

fn parse_unary<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Expression<'ast>> {
    let start_span = input.current_span();

    let op = match peek_token(input) {
        Some(TokenKind::Minus) => Some(UnaryOp::Neg),
        Some(TokenKind::Bang) => Some(UnaryOp::Not),
        Some(TokenKind::Keyword(Keyword::Not)) => Some(UnaryOp::Not),
        Some(TokenKind::Tilde) => Some(UnaryOp::BitNot),
        _ => None,
    };

    if let Some(op) = op {
        let (input, _) = input.take_split(1);
        let (input, operand) = parse_unary(arena, input)?;
        let span = start_span.merge(operand.span());
        return Ok((
            input,
            Expression::Unary(UnaryExpr {
                op,
                operand: arena.alloc(operand),
                span,
            }),
        ));
    }

    parse_postfix(arena, input)
}

fn parse_postfix<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Expression<'ast>> {
    let (mut input, mut expr) = parse_atom(arena, input)?;

    loop {
        match peek_token(input) {
            Some(TokenKind::LParen) => {
                let (new_input, new_expr) = parse_call(arena, input, expr)?;
                input = new_input;
                expr = new_expr;
            }
            Some(TokenKind::LBracket) => {
                let (new_input, new_expr) = parse_index(arena, input, expr)?;
                input = new_input;
                expr = new_expr;
            }
            Some(TokenKind::Dot) => {
                let (new_input, new_expr) = parse_field_or_method(arena, input, expr)?;
                input = new_input;
                expr = new_expr;
            }
            Some(TokenKind::Keyword(Keyword::As)) => {
                let (new_input, new_expr) = parse_cast(arena, input, expr)?;
                input = new_input;
                expr = new_expr;
            }
            _ => break,
        }
    }

    Ok((input, expr))
}

fn parse_atom<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Expression<'ast>> {
    match input.first().map(|t| t.kind) {
        Some(TokenKind::IntLit) => parse_int_literal(input),
        Some(TokenKind::FloatLit) => parse_float_literal(input),
        Some(TokenKind::StringLit) => parse_string_literal(input),
        Some(TokenKind::Keyword(Keyword::True | Keyword::False)) => parse_bool_literal(input),
        Some(TokenKind::Keyword(Keyword::None)) => parse_none_literal(input),
        Some(TokenKind::Keyword(Keyword::Some)) => parse_some_expr(arena, input),
        Some(TokenKind::Ident) => parse_ident_or_struct(arena, input),
        Some(TokenKind::LParen) => parse_grouped(arena, input),
        Some(TokenKind::LBracket) => parse_array_expr(arena, input),
        Some(TokenKind::LBrace) => parse_block_or_map(arena, input),
        Some(TokenKind::Keyword(Keyword::If)) => parse_if_expr(arena, input),
        Some(TokenKind::Keyword(Keyword::Spawn)) => parse_spawn_expr(arena, input),
        Some(TokenKind::Keyword(Keyword::Await)) => parse_await_expr(arena, input),
        Some(TokenKind::Keyword(Keyword::Try)) => parse_try_expr(arena, input),
        Some(TokenKind::Pipe | TokenKind::PipePipe) => parse_lambda_expr(arena, input),
        _ => Err(nom::Err::Error(PError {
            input,
            kind: PErrorKind::ExpectedExpr,
        })),
    }
}

fn parse_int_literal<'a, 'ast>(input: TokenStream<'a>) -> PResult<'a, Expression<'ast>> {
    let (input, (_, span)) = int_lit(input)?;
    Ok((
        input,
        Expression::Literal(LiteralExpr {
            value: Literal::Int(0),
            span,
        }),
    ))
}

fn parse_float_literal<'a, 'ast>(input: TokenStream<'a>) -> PResult<'a, Expression<'ast>> {
    let (input, (_, span)) = float_lit(input)?;
    Ok((
        input,
        Expression::Literal(LiteralExpr {
            value: Literal::Float(0.0),
            span,
        }),
    ))
}

fn parse_string_literal<'a, 'ast>(input: TokenStream<'a>) -> PResult<'a, Expression<'ast>> {
    let (input, (symbol, span)) = string_lit(input)?;
    Ok((
        input,
        Expression::Literal(LiteralExpr {
            value: Literal::String(symbol),
            span,
        }),
    ))
}

fn parse_bool_literal<'a, 'ast>(input: TokenStream<'a>) -> PResult<'a, Expression<'ast>> {
    alt((
        map(keyword(Keyword::True), |t| {
            Expression::Literal(LiteralExpr {
                value: Literal::Bool(true),
                span: t.span,
            })
        }),
        map(keyword(Keyword::False), |t| {
            Expression::Literal(LiteralExpr {
                value: Literal::Bool(false),
                span: t.span,
            })
        }),
    ))(input)
}

fn parse_none_literal<'a, 'ast>(input: TokenStream<'a>) -> PResult<'a, Expression<'ast>> {
    let (input, tok) = keyword(Keyword::None)(input)?;
    Ok((
        input,
        Expression::Literal(LiteralExpr {
            value: Literal::None,
            span: tok.span,
        }),
    ))
}

fn parse_some_expr<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Expression<'ast>> {
    let (input, start) = keyword(Keyword::Some)(input)?;
    let (input, _) = token(TokenKind::LParen)(input)?;
    let (input, inner) = parse_expression(arena, input)?;
    let (input, end) = token(TokenKind::RParen)(input)?;
    Ok((
        input,
        Expression::Some(SomeExpr {
            value: arena.alloc(inner),
            span: start.span.merge(end.span),
        }),
    ))
}

fn parse_ident_or_struct<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Expression<'ast>> {
    let (input, name) = ident(input)?;

    if check(TokenKind::LBrace)(input) {
        return parse_struct_literal(arena, input, name);
    }

    Ok((
        input,
        Expression::Identifier(IdentExpr {
            span: name.span,
            ident: name,
        }),
    ))
}

fn parse_struct_literal<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
    name: Ident,
) -> PResult<'a, Expression<'ast>> {
    let start_span = name.span;
    let (input, _) = token(TokenKind::LBrace)(input)?;

    let mut fields = Vec::with_capacity(6);
    let mut input = input;

    loop {
        if check(TokenKind::RBrace)(input) {
            break;
        }

        let (new_input, field_name) = ident(input)?;
        let (new_input, _) = token(TokenKind::Colon)(new_input)?;
        let (new_input, value) = parse_expression(arena, new_input)?;
        let span = field_name.span.merge(value.span());
        fields.push(StructLiteralField {
            name: field_name,
            value,
            span,
        });
        input = new_input;

        if !check(TokenKind::Comma)(input) {
            break;
        }
        let (new_input, _) = token(TokenKind::Comma)(input)?;
        input = new_input;
    }

    let (input, end) = token(TokenKind::RBrace)(input)?;
    Ok((
        input,
        Expression::StructLiteral(StructLiteralExpr {
            name,
            fields,
            span: start_span.merge(end.span),
        }),
    ))
}

fn parse_grouped<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Expression<'ast>> {
    let (input, start) = token(TokenKind::LParen)(input)?;

    if check(TokenKind::RParen)(input) {
        let (input, end) = token(TokenKind::RParen)(input)?;
        return Ok((
            input,
            Expression::Literal(LiteralExpr {
                value: Literal::Int(0),
                span: start.span.merge(end.span),
            }),
        ));
    }

    let (input, inner) = parse_expression(arena, input)?;
    let (input, end) = token(TokenKind::RParen)(input)?;

    Ok((
        input,
        Expression::Grouped(GroupedExpr {
            inner: arena.alloc(inner),
            span: start.span.merge(end.span),
        }),
    ))
}

fn parse_array_expr<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Expression<'ast>> {
    let (input, start) = token(TokenKind::LBracket)(input)?;

    let mut elements = Vec::new();
    let mut input = input;

    if !check(TokenKind::RBracket)(input) {
        let (new_input, first) = parse_expression(arena, input)?;
        elements.push(first);
        input = new_input;

        while check(TokenKind::Comma)(input) {
            let (new_input, _) = token(TokenKind::Comma)(input)?;
            if check(TokenKind::RBracket)(new_input) {
                input = new_input;
                break;
            }
            let (new_input, elem) = parse_expression(arena, new_input)?;
            elements.push(elem);
            input = new_input;
        }
    }

    let (input, end) = token(TokenKind::RBracket)(input)?;

    Ok((
        input,
        Expression::Array(ArrayExpr {
            elements,
            span: start.span.merge(end.span),
        }),
    ))
}

fn parse_block_or_map<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Expression<'ast>> {
    let (input, start) = token(TokenKind::LBrace)(input)?;

    if check(TokenKind::RBrace)(input) {
        let (input, end) = token(TokenKind::RBrace)(input)?;
        return Ok((
            input,
            Expression::Map(MapExpr {
                entries: Vec::new(),
                span: start.span.merge(end.span),
            }),
        ));
    }

    if check(TokenKind::StringLit)(input) || check(TokenKind::Ident)(input) {
        let test_input = input.slice(1..);
        if check(TokenKind::Colon)(test_input) {
            return parse_map_entries(arena, input, start.span);
        }
    }

    parse_block_inner(arena, input, start.span)
}

fn parse_map_entries<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
    start_span: Span,
) -> PResult<'a, Expression<'ast>> {
    let mut entries = Vec::new();
    let mut input = input;

    loop {
        if check(TokenKind::RBrace)(input) {
            break;
        }

        let (new_input, key) = parse_expression(arena, input)?;
        let (new_input, _) = token(TokenKind::Colon)(new_input)?;
        let (new_input, value) = parse_expression(arena, new_input)?;
        let span = key.span().merge(value.span());
        entries.push(MapEntry { key, value, span });
        input = new_input;

        if !check(TokenKind::Comma)(input) {
            break;
        }
        let (new_input, _) = token(TokenKind::Comma)(input)?;
        input = new_input;
    }

    let (input, end) = token(TokenKind::RBrace)(input)?;
    Ok((
        input,
        Expression::Map(MapExpr {
            entries,
            span: start_span.merge(end.span),
        }),
    ))
}

fn parse_block_inner<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
    start_span: Span,
) -> PResult<'a, Expression<'ast>> {
    let mut statements = Vec::with_capacity(8);
    let mut input = input;

    while !check(TokenKind::RBrace)(input) && !is_eof(input) {
        let (new_input, stmt) = parse_statement(arena, input)?;
        statements.push(stmt);
        input = new_input;
    }

    let (input, end) = token(TokenKind::RBrace)(input)?;

    Ok((
        input,
        Expression::Block(BlockExpr {
            statements,
            tail: None,
            span: start_span.merge(end.span),
        }),
    ))
}

fn parse_if_expr<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Expression<'ast>> {
    let (input, start) = keyword(Keyword::If)(input)?;
    let (input, _) = token(TokenKind::LParen)(input)?;
    let (input, condition) = parse_expression(arena, input)?;
    let (input, _) = token(TokenKind::RParen)(input)?;

    let (input, then_block) = parse_block(arena, input)?;
    let then_span = then_block.span;
    let then_branch = BlockExpr {
        statements: then_block.statements,
        tail: None,
        span: then_block.span,
    };

    let (input, else_branch) = if check_keyword(Keyword::Else)(input) {
        let (input, _) = keyword(Keyword::Else)(input)?;
        if check_keyword(Keyword::If)(input) {
            let (input, else_if) = parse_if_expr(arena, input)?;
            if let Expression::If(if_expr) = else_if {
                (input, Some(ElseExpr::ElseIf(arena.alloc(if_expr))))
            } else {
                (input, None)
            }
        } else {
            let (input, else_block) = parse_block(arena, input)?;
            let else_block_expr = BlockExpr {
                statements: else_block.statements,
                tail: None,
                span: else_block.span,
            };
            (input, Some(ElseExpr::Else(arena.alloc(else_block_expr))))
        }
    } else {
        (input, None)
    };

    let end_span = else_branch
        .as_ref()
        .map(|e| match e {
            ElseExpr::ElseIf(i) => i.span,
            ElseExpr::Else(b) => b.span,
        })
        .unwrap_or(then_span);

    Ok((
        input,
        Expression::If(IfExpr {
            condition: arena.alloc(condition),
            then_branch: arena.alloc(then_branch),
            else_branch,
            span: start.span.merge(end_span),
        }),
    ))
}

fn parse_spawn_expr<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Expression<'ast>> {
    let (input, start) = keyword(Keyword::Spawn)(input)?;
    let (input, block) = parse_block(arena, input)?;
    let body_span = block.span;
    let span = start.span.merge(body_span);

    let body = BlockExpr {
        statements: block.statements,
        tail: None,
        span: body_span,
    };

    Ok((
        input,
        Expression::Spawn(SpawnExpr {
            body: arena.alloc(body),
            span,
        }),
    ))
}

fn parse_await_expr<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Expression<'ast>> {
    let (input, start) = keyword(Keyword::Await)(input)?;
    let (input, expr) = parse_unary(arena, input)?;
    let span = start.span.merge(expr.span());

    Ok((
        input,
        Expression::Await(AwaitExpr {
            expr: arena.alloc(expr),
            span,
        }),
    ))
}

fn parse_try_expr<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Expression<'ast>> {
    let (input, start) = keyword(Keyword::Try)(input)?;
    let (input, expr) = parse_unary(arena, input)?;
    let span = start.span.merge(expr.span());

    Ok((
        input,
        Expression::Try(TryExpr {
            expr: arena.alloc(expr),
            span,
        }),
    ))
}

fn parse_lambda_expr<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, Expression<'ast>> {
    let start_span = input.current_span();

    let (input, params) = if check(TokenKind::PipePipe)(input) {
        let (input, _) = token(TokenKind::PipePipe)(input)?;
        (input, Vec::new())
    } else {
        let (input, _) = token(TokenKind::Pipe)(input)?;
        let (input, params) = parse_lambda_params(input)?;
        let (input, _) = token(TokenKind::Pipe)(input)?;
        (input, params)
    };

    let (input, return_ty) = if check(TokenKind::Arrow)(input) {
        let (input, _) = token(TokenKind::Arrow)(input)?;
        let (input, ty) = parse_type(input)?;
        (input, Some(ty))
    } else {
        (input, None)
    };

    let (input, body) = parse_expression(arena, input)?;
    let span = start_span.merge(body.span());

    Ok((
        input,
        Expression::Lambda(LambdaExpr {
            params,
            return_ty,
            body: arena.alloc(body),
            span,
        }),
    ))
}

fn parse_lambda_params(input: TokenStream) -> PResult<Vec<LambdaParam>> {
    let mut params = Vec::new();
    let mut input = input;

    if check(TokenKind::Pipe)(input) {
        return Ok((input, params));
    }

    let (new_input, first) = parse_lambda_param(input)?;
    params.push(first);
    input = new_input;

    while check(TokenKind::Comma)(input) {
        let (new_input, _) = token(TokenKind::Comma)(input)?;
        let (new_input, param) = parse_lambda_param(new_input)?;
        params.push(param);
        input = new_input;
    }

    Ok((input, params))
}

fn parse_lambda_param(input: TokenStream) -> PResult<LambdaParam> {
    let (input, name) = ident(input)?;
    let (input, ty) = if check(TokenKind::Colon)(input) {
        let (input, _) = token(TokenKind::Colon)(input)?;
        let (input, ty) = parse_type(input)?;
        (input, Some(ty))
    } else {
        (input, None)
    };
    let span = name.span;
    Ok((input, LambdaParam { name, ty, span }))
}

fn parse_call<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
    callee: Expression<'ast>,
) -> PResult<'a, Expression<'ast>> {
    let start_span = callee.span();
    let (input, _) = token(TokenKind::LParen)(input)?;

    let mut args = Vec::new();
    let mut input = input;

    if !check(TokenKind::RParen)(input) {
        let (new_input, first) = parse_expression(arena, input)?;
        args.push(first);
        input = new_input;

        while check(TokenKind::Comma)(input) {
            let (new_input, _) = token(TokenKind::Comma)(input)?;
            let (new_input, arg) = parse_expression(arena, new_input)?;
            args.push(arg);
            input = new_input;
        }
    }

    let (input, end) = token(TokenKind::RParen)(input)?;

    Ok((
        input,
        Expression::Call(CallExpr {
            callee: arena.alloc(callee),
            type_args: Vec::new(),
            args,
            span: start_span.merge(end.span),
        }),
    ))
}

fn parse_index<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
    base: Expression<'ast>,
) -> PResult<'a, Expression<'ast>> {
    let start_span = base.span();
    let (input, _) = token(TokenKind::LBracket)(input)?;
    let (input, index) = parse_expression(arena, input)?;
    let (input, end) = token(TokenKind::RBracket)(input)?;

    Ok((
        input,
        Expression::Index(IndexExpr {
            base: arena.alloc(base),
            index: arena.alloc(index),
            span: start_span.merge(end.span),
        }),
    ))
}

fn parse_field_or_method<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
    base: Expression<'ast>,
) -> PResult<'a, Expression<'ast>> {
    let start_span = base.span();
    let (input, _) = token(TokenKind::Dot)(input)?;
    let (input, field) = ident(input)?;

    if check(TokenKind::LParen)(input) {
        let (input, _) = token(TokenKind::LParen)(input)?;

        let mut args = Vec::new();
        let mut input = input;

        if !check(TokenKind::RParen)(input) {
            let (new_input, first) = parse_expression(arena, input)?;
            args.push(first);
            input = new_input;

            while check(TokenKind::Comma)(input) {
                let (new_input, _) = token(TokenKind::Comma)(input)?;
                let (new_input, arg) = parse_expression(arena, new_input)?;
                args.push(arg);
                input = new_input;
            }
        }

        let (input, end) = token(TokenKind::RParen)(input)?;

        Ok((
            input,
            Expression::MethodCall(MethodCallExpr {
                receiver: arena.alloc(base),
                method: field,
                type_args: Vec::new(),
                args,
                span: start_span.merge(end.span),
            }),
        ))
    } else {
        Ok((
            input,
            Expression::Field(FieldExpr {
                base: arena.alloc(base),
                field: field.clone(),
                span: start_span.merge(field.span),
            }),
        ))
    }
}

fn parse_cast<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
    expr: Expression<'ast>,
) -> PResult<'a, Expression<'ast>> {
    let start_span = expr.span();
    let (input, _) = keyword(Keyword::As)(input)?;
    let (input, target_ty) = parse_type(input)?;
    let span = start_span.merge(input.current_span());

    Ok((
        input,
        Expression::Cast(CastExpr {
            expr: arena.alloc(expr),
            target_ty,
            span,
        }),
    ))
}

fn peek_binary_op(input: TokenStream) -> Option<BinaryOp> {
    match input.first().map(|t| t.kind) {
        Some(TokenKind::Plus) => Some(BinaryOp::Add),
        Some(TokenKind::Minus) => Some(BinaryOp::Sub),
        Some(TokenKind::Star) => Some(BinaryOp::Mul),
        Some(TokenKind::Slash) => Some(BinaryOp::Div),
        Some(TokenKind::Percent) => Some(BinaryOp::Mod),
        Some(TokenKind::EqEq) => Some(BinaryOp::Eq),
        Some(TokenKind::NotEq) => Some(BinaryOp::NotEq),
        Some(TokenKind::Lt) => Some(BinaryOp::Lt),
        Some(TokenKind::LtEq) => Some(BinaryOp::LtEq),
        Some(TokenKind::Gt) => Some(BinaryOp::Gt),
        Some(TokenKind::GtEq) => Some(BinaryOp::GtEq),
        Some(TokenKind::AndAnd) => Some(BinaryOp::And),
        Some(TokenKind::PipePipe) => Some(BinaryOp::Or),
        Some(TokenKind::Keyword(Keyword::And)) => Some(BinaryOp::And),
        Some(TokenKind::Keyword(Keyword::Or)) => Some(BinaryOp::Or),
        Some(TokenKind::Keyword(Keyword::Is)) => Some(BinaryOp::Is),
        Some(TokenKind::Ampersand) => Some(BinaryOp::BitAnd),
        Some(TokenKind::Pipe) => Some(BinaryOp::BitOr),
        Some(TokenKind::Caret) => Some(BinaryOp::BitXor),
        Some(TokenKind::LtLt) => Some(BinaryOp::Shl),
        Some(TokenKind::GtGt) => Some(BinaryOp::Shr),
        Some(TokenKind::DotDot) => Some(BinaryOp::Range),
        Some(TokenKind::DotDotEq) => Some(BinaryOp::RangeIncl),
        _ => None,
    }
}

fn advance_binary_op(input: TokenStream) -> TokenStream {
    input.slice(1..)
}

pub fn parse_block<'a, 'ast>(
    arena: &'ast AstArena,
    input: TokenStream<'a>,
) -> PResult<'a, BlockStmt<'ast>> {
    let (input, start) = token(TokenKind::LBrace)(input)?;
    let mut statements = Vec::with_capacity(8);
    let mut input = input;

    while !check(TokenKind::RBrace)(input) && !is_eof(input) {
        let (new_input, stmt) = parse_statement(arena, input)?;
        statements.push(stmt);
        input = new_input;
    }

    let (input, end) = token(TokenKind::RBrace)(input)?;

    Ok((
        input,
        BlockStmt {
            statements,
            span: start.span.merge(end.span),
        },
    ))
}
