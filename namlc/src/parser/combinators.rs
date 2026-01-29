//!
//! Base Combinators for Token Parsing
//!
//! Reusable nom combinators for matching tokens, keywords, and identifiers.
//!

use nom::error::{ErrorKind, ParseError};
use nom::{IResult, InputTake};

use crate::ast::Ident;
use crate::lexer::{Keyword, Token, TokenKind};
use crate::source::Span;

use super::input::TokenStream;

pub type PResult<'a, O> = IResult<TokenStream<'a>, O, PError<'a>>;

#[derive(Debug, Clone)]
pub struct PError<'a> {
    pub input: TokenStream<'a>,
    pub kind: PErrorKind,
}

#[derive(Debug, Clone)]
pub enum PErrorKind {
    Expected(TokenKind),
    ExpectedKeyword(Keyword),
    ExpectedIdent,
    ExpectedExpr,
    ExpectedType,
    ExpectedTypeAnnotation,
    ExpectedStatement,
    ExpectedItem,
    MutNotAllowedOnVar,
    MutNotAllowedOnReceiver,
    Nom(ErrorKind),
}

impl<'a> ParseError<TokenStream<'a>> for PError<'a> {
    fn from_error_kind(input: TokenStream<'a>, kind: ErrorKind) -> Self {
        PError {
            input,
            kind: PErrorKind::Nom(kind),
        }
    }

    fn append(_input: TokenStream<'a>, _kind: ErrorKind, other: Self) -> Self {
        other
    }
}

pub fn token(kind: TokenKind) -> impl Fn(TokenStream) -> PResult<Token> {
    move |input: TokenStream| {
        match input.first() {
            Some(tok) if tok.kind == kind => {
                let (rest, _) = input.take_split(1);
                Ok((rest, *tok))
            }
            _ => Err(nom::Err::Error(PError {
                input,
                kind: PErrorKind::Expected(kind),
            })),
        }
    }
}

pub fn keyword(kw: Keyword) -> impl Fn(TokenStream) -> PResult<Token> {
    move |input: TokenStream| {
        match input.first() {
            Some(tok) if tok.kind == TokenKind::Keyword(kw) => {
                let (rest, _) = input.take_split(1);
                Ok((rest, *tok))
            }
            _ => Err(nom::Err::Error(PError {
                input,
                kind: PErrorKind::ExpectedKeyword(kw),
            })),
        }
    }
}

pub fn ident(input: TokenStream) -> PResult<Ident> {
    match input.first() {
        Some(tok) if tok.kind == TokenKind::Ident => {
            let (rest, _) = input.take_split(1);
            let symbol = tok.symbol.expect("Ident must have symbol");
            Ok((rest, Ident::new(symbol, tok.span)))
        }
        _ => Err(nom::Err::Error(PError {
            input,
            kind: PErrorKind::ExpectedIdent,
        })),
    }
}

pub fn int_lit(input: TokenStream) -> PResult<(i64, Span)> {
    match input.first() {
        Some(tok) if tok.kind == TokenKind::IntLit => {
            let (rest, _) = input.take_split(1);
            let text = input.span_text(tok.span);
            let value: i64 = text.replace('_', "").parse().unwrap_or(0);
            Ok((rest, (value, tok.span)))
        }
        _ => Err(nom::Err::Error(PError {
            input,
            kind: PErrorKind::Expected(TokenKind::IntLit),
        })),
    }
}

pub fn float_lit(input: TokenStream) -> PResult<(f64, Span)> {
    match input.first() {
        Some(tok) if tok.kind == TokenKind::FloatLit => {
            let (rest, _) = input.take_split(1);
            let text = input.span_text(tok.span);
            let value: f64 = text.replace('_', "").parse().unwrap_or(0.0);
            Ok((rest, (value, tok.span)))
        }
        _ => Err(nom::Err::Error(PError {
            input,
            kind: PErrorKind::Expected(TokenKind::FloatLit),
        })),
    }
}

pub fn string_lit(input: TokenStream) -> PResult<(lasso::Spur, Span)> {
    match input.first() {
        Some(tok) if tok.kind == TokenKind::StringLit => {
            let (rest, _) = input.take_split(1);
            let symbol = tok.symbol.expect("StringLit must have symbol");
            Ok((rest, (symbol, tok.span)))
        }
        _ => Err(nom::Err::Error(PError {
            input,
            kind: PErrorKind::Expected(TokenKind::StringLit),
        })),
    }
}

pub fn peek_token(input: TokenStream) -> Option<TokenKind> {
    input.first().map(|t| t.kind)
}

pub fn check(kind: TokenKind) -> impl Fn(TokenStream) -> bool {
    move |input: TokenStream| {
        input.first().map(|t| t.kind == kind).unwrap_or(false)
    }
}

pub fn check_keyword(kw: Keyword) -> impl Fn(TokenStream) -> bool {
    move |input: TokenStream| {
        input
            .first()
            .map(|t| t.kind == TokenKind::Keyword(kw))
            .unwrap_or(false)
    }
}

pub fn is_eof(input: TokenStream) -> bool {
    input.is_empty() || input.first().map(|t| t.kind == TokenKind::Eof).unwrap_or(true)
}
