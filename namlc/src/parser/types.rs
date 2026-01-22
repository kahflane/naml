//!
//! Type Annotation Parser
//!
//! Parses type annotations using nom combinators.
//! Supports primitives, arrays, generics, tuples, and function types.
//!

use nom::branch::alt;
use nom::combinator::{map, opt};
use nom::multi::separated_list0;
use nom::sequence::preceded;
use nom::InputTake;

use crate::ast::{Ident, NamlType};
use crate::lexer::{Keyword, TokenKind};
use crate::source::Span;

use super::combinators::*;
use super::input::TokenStream;

pub fn parse_type(input: TokenStream) -> PResult<NamlType> {
    match input.first().map(|t| t.kind) {
        // Primitives
        Some(TokenKind::Keyword(Keyword::Int)) => parse_primitive(input),
        Some(TokenKind::Keyword(Keyword::Uint)) => parse_primitive(input),
        Some(TokenKind::Keyword(Keyword::Float)) => parse_primitive(input),
        Some(TokenKind::Keyword(Keyword::Bool)) => parse_primitive(input),
        Some(TokenKind::Keyword(Keyword::String)) => parse_primitive(input),
        Some(TokenKind::Keyword(Keyword::Bytes)) => parse_primitive(input),
        // Built-in generic types
        Some(TokenKind::Keyword(Keyword::Option)) => parse_option_type(input),
        Some(TokenKind::Keyword(Keyword::Map)) => parse_map_type(input),
        Some(TokenKind::Keyword(Keyword::Channel)) => parse_channel_type(input),
        // Function type
        Some(TokenKind::Keyword(Keyword::Fn)) => parse_fn_type(input),
        // Array type
        Some(TokenKind::LBracket) => parse_array_type(input),
        // Tuple or grouped type
        Some(TokenKind::LParen) => parse_paren_or_tuple_type(input),
        // Named or generic type (user-defined)
        Some(TokenKind::Ident) => parse_named_or_generic_type(input),
        _ => Err(nom::Err::Error(PError {
            input,
            kind: PErrorKind::ExpectedType,
        })),
    }
}

fn parse_primitive(input: TokenStream) -> PResult<NamlType> {
    alt((
        map(keyword(Keyword::Int), |_| NamlType::Int),
        map(keyword(Keyword::Uint), |_| NamlType::Uint),
        map(keyword(Keyword::Float), |_| NamlType::Float),
        map(keyword(Keyword::Bool), |_| NamlType::Bool),
        map(keyword(Keyword::String), |_| NamlType::String),
        map(keyword(Keyword::Bytes), |_| NamlType::Bytes),
    ))(input)
}

fn parse_option_type(input: TokenStream) -> PResult<NamlType> {
    let (input, _) = keyword(Keyword::Option)(input)?;
    let (input, _) = token(TokenKind::Lt)(input)?;
    let (input, inner) = parse_type(input)?;
    let (input, _) = parse_gt(input)?;
    Ok((input, NamlType::option(inner)))
}

fn parse_map_type(input: TokenStream) -> PResult<NamlType> {
    let (input, _) = keyword(Keyword::Map)(input)?;
    let (input, _) = token(TokenKind::Lt)(input)?;
    let (input, key) = parse_type(input)?;
    let (input, _) = token(TokenKind::Comma)(input)?;
    let (input, value) = parse_type(input)?;
    let (input, _) = parse_gt(input)?;
    Ok((input, NamlType::map(key, value)))
}

fn parse_channel_type(input: TokenStream) -> PResult<NamlType> {
    let (input, _) = keyword(Keyword::Channel)(input)?;
    let (input, _) = token(TokenKind::Lt)(input)?;
    let (input, inner) = parse_type(input)?;
    let (input, _) = parse_gt(input)?;
    Ok((input, NamlType::channel(inner)))
}

fn parse_fn_type(input: TokenStream) -> PResult<NamlType> {
    let (input, _) = keyword(Keyword::Fn)(input)?;
    let (input, _) = token(TokenKind::LParen)(input)?;
    let (input, params) = separated_list0(token(TokenKind::Comma), parse_type)(input)?;
    let (input, _) = token(TokenKind::RParen)(input)?;
    let (input, returns) = opt(preceded(token(TokenKind::Arrow), parse_type))(input)?;
    let returns = returns.unwrap_or(NamlType::Unit);
    Ok((input, NamlType::function(params, returns)))
}

fn parse_array_type(input: TokenStream) -> PResult<NamlType> {
    let (input, _) = token(TokenKind::LBracket)(input)?;
    let (input, elem) = parse_type(input)?;
    let (input, size) = opt(preceded(token(TokenKind::Semicolon), int_lit))(input)?;
    let (input, _) = token(TokenKind::RBracket)(input)?;

    match size {
        Some((_, _)) => Ok((input, NamlType::fixed_array(elem, 0))),
        None => Ok((input, NamlType::array(elem))),
    }
}

fn parse_paren_or_tuple_type(input: TokenStream) -> PResult<NamlType> {
    let (input, _) = token(TokenKind::LParen)(input)?;

    if check(TokenKind::RParen)(input) {
        let (input, _) = token(TokenKind::RParen)(input)?;
        return Ok((input, NamlType::Unit));
    }

    let (input, first) = parse_type(input)?;

    if check(TokenKind::RParen)(input) {
        let (input, _) = token(TokenKind::RParen)(input)?;
        return Ok((input, first));
    }

    let (input, _) = token(TokenKind::Comma)(input)?;
    let (input, rest) = separated_list0(token(TokenKind::Comma), parse_type)(input)?;
    let (input, _) = token(TokenKind::RParen)(input)?;

    let mut types = vec![first];
    types.extend(rest);
    Ok((input, NamlType::Generic(
        Ident::new(lasso::Spur::default(), Span::dummy()),
        types,
    )))
}

fn parse_named_or_generic_type(input: TokenStream) -> PResult<NamlType> {
    let (input, name) = ident(input)?;

    if !check(TokenKind::Lt)(input) {
        return Ok((input, NamlType::Named(name)));
    }

    let (input, _) = token(TokenKind::Lt)(input)?;
    let (input, args) = separated_list0(token(TokenKind::Comma), parse_type)(input)?;
    let (input, _) = parse_gt(input)?;

    Ok((input, NamlType::Generic(name, args)))
}

thread_local! {
    static PENDING_GT: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
}

pub fn parse_gt(input: TokenStream) -> PResult<()> {
    PENDING_GT.with(|pg| {
        if pg.get() > 0 {
            pg.set(pg.get() - 1);
            return Ok((input, ()));
        }

        match input.first().map(|t| t.kind) {
            Some(TokenKind::Gt) => {
                let (rest, _) = input.take_split(1);
                Ok((rest, ()))
            }
            Some(TokenKind::GtGt) => {
                let (rest, _) = input.take_split(1);
                pg.set(pg.get() + 1);
                Ok((rest, ()))
            }
            _ => Err(nom::Err::Error(PError {
                input,
                kind: PErrorKind::Expected(TokenKind::Gt),
            })),
        }
    })
}

pub fn reset_pending_gt() {
    PENDING_GT.with(|pg| pg.set(0));
}
