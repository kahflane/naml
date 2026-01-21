//!
//! Pattern Parser
//!
//! Parses patterns for switch cases and destructuring.
//! Supports: literals, identifiers, enum variants with bindings, wildcards.
//!
//! Pattern types:
//! - Literal: Match against a literal value (int, float, string, bool, none)
//! - Identifier: Bind a value to a name or match against a constant
//! - Variant: Match an enum variant, optionally with bindings (e.g., Some(x))
//! - Wildcard: Match anything and discard (_)
//!
//! The parser determines pattern type by examining the token:
//! - An identifier "_" is the wildcard pattern
//! - An identifier followed by :: is a path (enum variant)
//! - An identifier followed by ( is a variant pattern with bindings
//! - Other identifiers are identifier patterns (bindings or constants)
//! - Literals (int, float, string, true/false, none) become literal patterns
//!

use nom::InputTake;

use crate::ast::{
    IdentPattern, Literal, LiteralPattern, Pattern, VariantPattern, WildcardPattern,
};
use crate::lexer::{Keyword, TokenKind};

use super::combinators::*;
use super::input::TokenStream;

pub fn parse_pattern<'a, 'ast>(input: TokenStream<'a>) -> PResult<'a, Pattern<'ast>> {
    match input.first().map(|t| t.kind) {
        Some(TokenKind::Ident) => parse_ident_or_variant_pattern(input),
        Some(TokenKind::IntLit) => parse_int_pattern(input),
        Some(TokenKind::FloatLit) => parse_float_pattern(input),
        Some(TokenKind::StringLit) => parse_string_pattern(input),
        Some(TokenKind::Keyword(Keyword::True)) => parse_bool_pattern(input, true),
        Some(TokenKind::Keyword(Keyword::False)) => parse_bool_pattern(input, false),
        Some(TokenKind::Keyword(Keyword::None)) => parse_none_pattern(input),
        _ => Err(nom::Err::Error(PError {
            input,
            kind: PErrorKind::ExpectedExpr,
        })),
    }
}

fn parse_ident_or_variant_pattern<'a, 'ast>(
    input: TokenStream<'a>,
) -> PResult<'a, Pattern<'ast>> {
    let first_tok = input.first().unwrap();
    let first_text = input.span_text(first_tok.span);

    if first_text == "_" {
        let (input, _) = input.take_split(1);
        return Ok((
            input,
            Pattern::Wildcard(WildcardPattern {
                span: first_tok.span,
            }),
        ));
    }

    let (mut input, first) = ident(input)?;
    let start_span = first.span;
    let mut path = vec![first];

    while check(TokenKind::ColonColon)(input) {
        let (new_input, _) = token(TokenKind::ColonColon)(input)?;
        let (new_input, segment) = ident(new_input)?;
        path.push(segment);
        input = new_input;
    }

    if check(TokenKind::LParen)(input) {
        let (new_input, _) = token(TokenKind::LParen)(input)?;
        input = new_input;

        let mut bindings = Vec::new();
        if !check(TokenKind::RParen)(input) {
            let (new_input, binding) = ident(input)?;
            bindings.push(binding);
            input = new_input;

            while check(TokenKind::Comma)(input) {
                let (new_input, _) = token(TokenKind::Comma)(input)?;
                let (new_input, binding) = ident(new_input)?;
                bindings.push(binding);
                input = new_input;
            }
        }

        let (new_input, end) = token(TokenKind::RParen)(input)?;
        let span = start_span.merge(end.span);
        return Ok((
            new_input,
            Pattern::Variant(VariantPattern {
                path,
                bindings,
                span,
            }),
        ));
    }

    if path.len() > 1 {
        let end_span = path.last().unwrap().span;
        return Ok((
            input,
            Pattern::Variant(VariantPattern {
                path,
                bindings: Vec::new(),
                span: start_span.merge(end_span),
            }),
        ));
    }

    Ok((
        input,
        Pattern::Identifier(IdentPattern {
            ident: path.remove(0),
            span: start_span,
        }),
    ))
}

fn parse_int_pattern<'a, 'ast>(input: TokenStream<'a>) -> PResult<'a, Pattern<'ast>> {
    let (input, (value, span)) = int_lit(input)?;
    Ok((
        input,
        Pattern::Literal(LiteralPattern {
            value: Literal::Int(value),
            span,
        }),
    ))
}

fn parse_float_pattern<'a, 'ast>(input: TokenStream<'a>) -> PResult<'a, Pattern<'ast>> {
    let (input, (value, span)) = float_lit(input)?;
    Ok((
        input,
        Pattern::Literal(LiteralPattern {
            value: Literal::Float(value),
            span,
        }),
    ))
}

fn parse_string_pattern<'a, 'ast>(input: TokenStream<'a>) -> PResult<'a, Pattern<'ast>> {
    let (input, (symbol, span)) = string_lit(input)?;
    Ok((
        input,
        Pattern::Literal(LiteralPattern {
            value: Literal::String(symbol),
            span,
        }),
    ))
}

fn parse_bool_pattern<'a, 'ast>(
    input: TokenStream<'a>,
    value: bool,
) -> PResult<'a, Pattern<'ast>> {
    let kw = if value { Keyword::True } else { Keyword::False };
    let (input, tok) = keyword(kw)(input)?;
    Ok((
        input,
        Pattern::Literal(LiteralPattern {
            value: Literal::Bool(value),
            span: tok.span,
        }),
    ))
}

fn parse_none_pattern<'a, 'ast>(input: TokenStream<'a>) -> PResult<'a, Pattern<'ast>> {
    let (input, tok) = keyword(Keyword::None)(input)?;
    Ok((
        input,
        Pattern::Literal(LiteralPattern {
            value: Literal::None,
            span: tok.span,
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    fn parse_pattern_from_source(source: &str) -> Pattern<'static> {
        let (tokens, _interner) = tokenize(source);
        let input = TokenStream::new(&tokens, source);
        let (_, pattern) = parse_pattern(input).expect("Failed to parse pattern");
        pattern
    }

    #[test]
    fn test_wildcard_pattern() {
        let pattern = parse_pattern_from_source("_");
        assert!(matches!(pattern, Pattern::Wildcard(_)));
    }

    #[test]
    fn test_identifier_pattern() {
        let pattern = parse_pattern_from_source("x");
        assert!(matches!(pattern, Pattern::Identifier(_)));
    }

    #[test]
    fn test_int_literal_pattern() {
        let pattern = parse_pattern_from_source("42");
        if let Pattern::Literal(lit) = pattern {
            assert!(matches!(lit.value, Literal::Int(42)));
        } else {
            panic!("Expected literal pattern");
        }
    }

    #[test]
    fn test_string_literal_pattern() {
        let pattern = parse_pattern_from_source("\"hello\"");
        if let Pattern::Literal(lit) = pattern {
            assert!(matches!(lit.value, Literal::String(_)));
        } else {
            panic!("Expected literal pattern");
        }
    }

    #[test]
    fn test_bool_literal_pattern() {
        let pattern = parse_pattern_from_source("true");
        if let Pattern::Literal(lit) = pattern {
            assert!(matches!(lit.value, Literal::Bool(true)));
        } else {
            panic!("Expected literal pattern");
        }
    }

    #[test]
    fn test_none_literal_pattern() {
        let pattern = parse_pattern_from_source("none");
        if let Pattern::Literal(lit) = pattern {
            assert!(matches!(lit.value, Literal::None));
        } else {
            panic!("Expected literal pattern");
        }
    }

    #[test]
    fn test_variant_pattern_simple() {
        let pattern = parse_pattern_from_source("Status::Active");
        if let Pattern::Variant(variant) = pattern {
            assert_eq!(variant.path.len(), 2);
            assert!(variant.bindings.is_empty());
        } else {
            panic!("Expected variant pattern");
        }
    }

    #[test]
    fn test_variant_pattern_with_binding() {
        let pattern = parse_pattern_from_source("Some(value)");
        if let Pattern::Variant(variant) = pattern {
            assert_eq!(variant.path.len(), 1);
            assert_eq!(variant.bindings.len(), 1);
        } else {
            panic!("Expected variant pattern");
        }
    }

    #[test]
    fn test_variant_pattern_with_multiple_bindings() {
        let pattern = parse_pattern_from_source("Result::Ok(a, b)");
        if let Pattern::Variant(variant) = pattern {
            assert_eq!(variant.path.len(), 2);
            assert_eq!(variant.bindings.len(), 2);
        } else {
            panic!("Expected variant pattern");
        }
    }
}
