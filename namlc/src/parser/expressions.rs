///
/// Expression Parser
///
/// Parses expressions using nom combinators with Pratt-style precedence.
///

use nom::branch::alt;
use nom::combinator::map;
use nom::multi::separated_list0;
use nom::{InputTake, Slice};

use crate::ast::*;
use crate::lexer::{Keyword, TokenKind};
use crate::source::{Span, Spanned};

use super::combinators::*;
use super::input::TokenStream;
use super::statements::parse_statement;
use super::types::parse_type;

pub fn parse_expression(input: TokenStream) -> PResult<Expression> {
    pratt_expr(input, 0)
}

fn pratt_expr(input: TokenStream, min_prec: u8) -> PResult<Expression> {
    let (mut input, mut left) = parse_unary(input)?;

    loop {
        input = skip_trivia(input);
        let Some(op) = peek_binary_op(input) else {
            break;
        };

        let prec = op.precedence();
        if prec < min_prec {
            break;
        }

        input = advance_binary_op(input);
        let (new_input, right) = pratt_expr(input, prec + 1)?;
        input = new_input;

        let span = left.span().merge(right.span());
        left = Expression::Binary(BinaryExpr {
            left: Box::new(left),
            op,
            right: Box::new(right),
            span,
        });
    }

    Ok((input, left))
}

fn parse_unary(input: TokenStream) -> PResult<Expression> {
    let input = skip_trivia(input);
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
        let (input, operand) = parse_unary(input)?;
        let span = start_span.merge(operand.span());
        return Ok((
            input,
            Expression::Unary(UnaryExpr {
                op,
                operand: Box::new(operand),
                span,
            }),
        ));
    }

    parse_postfix(input)
}

fn parse_postfix(input: TokenStream) -> PResult<Expression> {
    let (mut input, mut expr) = parse_atom(input)?;

    loop {
        input = skip_trivia(input);
        match peek_token(input) {
            Some(TokenKind::LParen) => {
                let (new_input, new_expr) = parse_call(input, expr)?;
                input = new_input;
                expr = new_expr;
            }
            Some(TokenKind::LBracket) => {
                let (new_input, new_expr) = parse_index(input, expr)?;
                input = new_input;
                expr = new_expr;
            }
            Some(TokenKind::Dot) => {
                let (new_input, new_expr) = parse_field_or_method(input, expr)?;
                input = new_input;
                expr = new_expr;
            }
            Some(TokenKind::Keyword(Keyword::As)) => {
                let (new_input, new_expr) = parse_cast(input, expr)?;
                input = new_input;
                expr = new_expr;
            }
            _ => break,
        }
    }

    Ok((input, expr))
}

fn parse_atom(input: TokenStream) -> PResult<Expression> {
    let input = skip_trivia(input);
    let _start_span = input.current_span();

    if check(TokenKind::Pipe)(input) || check(TokenKind::PipePipe)(input) {
        return parse_lambda_expr(input);
    }

    alt((
        parse_int_literal,
        parse_float_literal,
        parse_string_literal,
        parse_bool_literal,
        parse_none_literal,
        parse_some_expr,
        parse_ident_or_struct,
        parse_grouped,
        parse_array_expr,
        parse_block_or_map,
        parse_if_expr,
        parse_spawn_expr,
        parse_await_expr,
        parse_try_expr,
    ))(input)
}

fn parse_int_literal(input: TokenStream) -> PResult<Expression> {
    let (input, (_, span)) = int_lit(input)?;
    Ok((
        input,
        Expression::Literal(LiteralExpr {
            value: Literal::Int(0),
            span,
        }),
    ))
}

fn parse_float_literal(input: TokenStream) -> PResult<Expression> {
    let (input, (_, span)) = float_lit(input)?;
    Ok((
        input,
        Expression::Literal(LiteralExpr {
            value: Literal::Float(0.0),
            span,
        }),
    ))
}

fn parse_string_literal(input: TokenStream) -> PResult<Expression> {
    let (input, (symbol, span)) = string_lit(input)?;
    Ok((
        input,
        Expression::Literal(LiteralExpr {
            value: Literal::String(symbol),
            span,
        }),
    ))
}

fn parse_bool_literal(input: TokenStream) -> PResult<Expression> {
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

fn parse_none_literal(input: TokenStream) -> PResult<Expression> {
    let (input, tok) = keyword(Keyword::None)(input)?;
    Ok((
        input,
        Expression::Literal(LiteralExpr {
            value: Literal::None,
            span: tok.span,
        }),
    ))
}

fn parse_some_expr(input: TokenStream) -> PResult<Expression> {
    let (input, start) = keyword(Keyword::Some)(input)?;
    let (input, _) = token(TokenKind::LParen)(input)?;
    let (input, inner) = parse_expression(input)?;
    let (input, end) = token(TokenKind::RParen)(input)?;
    Ok((
        input,
        Expression::Some(SomeExpr {
            value: Box::new(inner),
            span: start.span.merge(end.span),
        }),
    ))
}

fn parse_ident_or_struct(input: TokenStream) -> PResult<Expression> {
    let (input, name) = ident(input)?;
    let input = skip_trivia(input);

    if check(TokenKind::LBrace)(input) {
        return parse_struct_literal(input, name);
    }

    Ok((
        input,
        Expression::Identifier(IdentExpr {
            span: name.span,
            ident: name,
        }),
    ))
}

fn parse_struct_literal(input: TokenStream, name: Ident) -> PResult<Expression> {
    let start_span = name.span;
    let (input, _) = token(TokenKind::LBrace)(input)?;

    let mut fields = Vec::new();
    let mut input = input;

    loop {
        input = skip_trivia(input);
        if check(TokenKind::RBrace)(input) {
            break;
        }

        let (new_input, field_name) = ident(input)?;
        let (new_input, _) = token(TokenKind::Colon)(new_input)?;
        let (new_input, value) = parse_expression(new_input)?;
        let span = field_name.span.merge(value.span());
        fields.push(StructLiteralField {
            name: field_name,
            value,
            span,
        });
        input = new_input;

        input = skip_trivia(input);
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

fn parse_grouped(input: TokenStream) -> PResult<Expression> {
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

    let (input, inner) = parse_expression(input)?;
    let (input, end) = token(TokenKind::RParen)(input)?;

    Ok((
        input,
        Expression::Grouped(GroupedExpr {
            inner: Box::new(inner),
            span: start.span.merge(end.span),
        }),
    ))
}

fn parse_array_expr(input: TokenStream) -> PResult<Expression> {
    let (input, start) = token(TokenKind::LBracket)(input)?;
    let (input, elements) = separated_list0(token(TokenKind::Comma), parse_expression)(input)?;
    let (input, end) = token(TokenKind::RBracket)(input)?;

    Ok((
        input,
        Expression::Array(ArrayExpr {
            elements,
            span: start.span.merge(end.span),
        }),
    ))
}

fn parse_block_or_map(input: TokenStream) -> PResult<Expression> {
    let (input, start) = token(TokenKind::LBrace)(input)?;
    let input = skip_trivia(input);

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
        let test_input = skip_trivia(input.slice(1..));
        if check(TokenKind::Colon)(test_input) {
            return parse_map_entries(input, start.span);
        }
    }

    parse_block_inner(input, start.span)
}

fn parse_map_entries(input: TokenStream, start_span: Span) -> PResult<Expression> {
    let mut entries = Vec::new();
    let mut input = input;

    loop {
        input = skip_trivia(input);
        if check(TokenKind::RBrace)(input) {
            break;
        }

        let (new_input, key) = parse_expression(input)?;
        let (new_input, _) = token(TokenKind::Colon)(new_input)?;
        let (new_input, value) = parse_expression(new_input)?;
        let span = key.span().merge(value.span());
        entries.push(MapEntry { key, value, span });
        input = new_input;

        input = skip_trivia(input);
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

fn parse_block_inner(input: TokenStream, start_span: Span) -> PResult<Expression> {
    let mut statements = Vec::new();
    let mut input = input;

    while !check(TokenKind::RBrace)(input) && !is_eof(input) {
        input = skip_trivia(input);
        if check(TokenKind::RBrace)(input) {
            break;
        }
        let (new_input, stmt) = parse_statement(input)?;
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

fn parse_if_expr(input: TokenStream) -> PResult<Expression> {
    let (input, start) = keyword(Keyword::If)(input)?;
    let (input, _) = token(TokenKind::LParen)(input)?;
    let (input, condition) = parse_expression(input)?;
    let (input, _) = token(TokenKind::RParen)(input)?;

    let (input, then_block) = parse_block(input)?;
    let then_span = then_block.span;
    let then_branch = BlockExpr {
        statements: then_block.statements,
        tail: None,
        span: then_block.span,
    };

    let (input, else_branch) = if check_keyword(Keyword::Else)(input) {
        let (input, _) = keyword(Keyword::Else)(input)?;
        if check_keyword(Keyword::If)(input) {
            let (input, else_if) = parse_if_expr(input)?;
            if let Expression::If(if_expr) = else_if {
                (input, Some(ElseExpr::ElseIf(Box::new(if_expr))))
            } else {
                (input, None)
            }
        } else {
            let (input, else_block) = parse_block(input)?;
            (input, Some(ElseExpr::Else(Box::new(BlockExpr {
                statements: else_block.statements,
                tail: None,
                span: else_block.span,
            }))))
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
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch,
            span: start.span.merge(end_span),
        }),
    ))
}

fn parse_spawn_expr(input: TokenStream) -> PResult<Expression> {
    let (input, start) = keyword(Keyword::Spawn)(input)?;
    let (input, block) = parse_block(input)?;
    let body_span = block.span;
    let span = start.span.merge(body_span);

    Ok((
        input,
        Expression::Spawn(SpawnExpr {
            body: Box::new(BlockExpr {
                statements: block.statements,
                tail: None,
                span: body_span,
            }),
            span,
        }),
    ))
}

fn parse_await_expr(input: TokenStream) -> PResult<Expression> {
    let (input, start) = keyword(Keyword::Await)(input)?;
    let (input, expr) = parse_unary(input)?;
    let span = start.span.merge(expr.span());

    Ok((
        input,
        Expression::Await(AwaitExpr {
            expr: Box::new(expr),
            span,
        }),
    ))
}

fn parse_try_expr(input: TokenStream) -> PResult<Expression> {
    let (input, start) = keyword(Keyword::Try)(input)?;
    let (input, expr) = parse_unary(input)?;
    let span = start.span.merge(expr.span());

    Ok((
        input,
        Expression::Try(TryExpr {
            expr: Box::new(expr),
            span,
        }),
    ))
}

fn parse_lambda_expr(input: TokenStream) -> PResult<Expression> {
    let start_span = input.current_span();

    let (input, params) = if check(TokenKind::PipePipe)(input) {
        let (input, _) = token(TokenKind::PipePipe)(input)?;
        (input, Vec::new())
    } else {
        let (input, _) = token(TokenKind::Pipe)(input)?;
        let (input, params) = separated_list0(token(TokenKind::Comma), parse_lambda_param)(input)?;
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

    let (input, body) = parse_expression(input)?;
    let span = start_span.merge(body.span());

    Ok((
        input,
        Expression::Lambda(LambdaExpr {
            params,
            return_ty,
            body: Box::new(body),
            span,
        }),
    ))
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

fn parse_call(input: TokenStream, callee: Expression) -> PResult<Expression> {
    let start_span = callee.span();
    let (input, _) = token(TokenKind::LParen)(input)?;
    let (input, args) = separated_list0(token(TokenKind::Comma), parse_expression)(input)?;
    let (input, end) = token(TokenKind::RParen)(input)?;

    Ok((
        input,
        Expression::Call(CallExpr {
            callee: Box::new(callee),
            type_args: Vec::new(),
            args,
            span: start_span.merge(end.span),
        }),
    ))
}

fn parse_index(input: TokenStream, base: Expression) -> PResult<Expression> {
    let start_span = base.span();
    let (input, _) = token(TokenKind::LBracket)(input)?;
    let (input, index) = parse_expression(input)?;
    let (input, end) = token(TokenKind::RBracket)(input)?;

    Ok((
        input,
        Expression::Index(IndexExpr {
            base: Box::new(base),
            index: Box::new(index),
            span: start_span.merge(end.span),
        }),
    ))
}

fn parse_field_or_method(input: TokenStream, base: Expression) -> PResult<Expression> {
    let start_span = base.span();
    let (input, _) = token(TokenKind::Dot)(input)?;
    let (input, field) = ident(input)?;

    if check(TokenKind::LParen)(input) {
        let (input, _) = token(TokenKind::LParen)(input)?;
        let (input, args) = separated_list0(token(TokenKind::Comma), parse_expression)(input)?;
        let (input, end) = token(TokenKind::RParen)(input)?;

        Ok((
            input,
            Expression::MethodCall(MethodCallExpr {
                receiver: Box::new(base),
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
                base: Box::new(base),
                field: field.clone(),
                span: start_span.merge(field.span),
            }),
        ))
    }
}

fn parse_cast(input: TokenStream, expr: Expression) -> PResult<Expression> {
    let start_span = expr.span();
    let (input, _) = keyword(Keyword::As)(input)?;
    let (input, target_ty) = parse_type(input)?;
    let span = start_span.merge(input.current_span());

    Ok((
        input,
        Expression::Cast(CastExpr {
            expr: Box::new(expr),
            target_ty,
            span,
        }),
    ))
}

fn peek_binary_op(input: TokenStream) -> Option<BinaryOp> {
    let input = skip_trivia(input);
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
    let input = skip_trivia(input);
    input.slice(1..)
}

pub fn parse_block(input: TokenStream) -> PResult<BlockStmt> {
    let (input, start) = token(TokenKind::LBrace)(input)?;
    let mut statements = Vec::new();
    let mut input = input;

    while !check(TokenKind::RBrace)(input) && !is_eof(input) {
        input = skip_trivia(input);
        if check(TokenKind::RBrace)(input) {
            break;
        }
        let (new_input, stmt) = parse_statement(input)?;
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
