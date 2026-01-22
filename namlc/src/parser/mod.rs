//!
//! Parser Module - nom-based Token Parsing
//!
//! This module provides the parser for the naml programming language.
//! It uses nom parser combinators to parse a stream of tokens into an AST.
//!
//! The parser is structured as follows:
//! - input: TokenStream type for nom integration
//! - combinators: Reusable token-matching combinators
//! - types: Type annotation parsing
//! - expressions: Expression parsing with Pratt precedence
//! - statements: Statement parsing
//! - items: Top-level item parsing
//!
//! Entry point: parse() function takes tokens and returns a SourceFile AST.
//!

mod combinators;
mod expressions;
mod input;
mod items;
mod patterns;
mod statements;
mod types;

pub use patterns::parse_pattern;

pub use combinators::{PError, PErrorKind};
pub use input::TokenStream;

use nom::InputTake;

use crate::ast::{AstArena, SourceFile};
use crate::lexer::Token;
use crate::source::{Span, Spanned};

use combinators::is_eof;
use items::parse_item;
use types::reset_pending_gt;

pub struct ParseResult<'ast> {
    pub ast: SourceFile<'ast>,
    pub errors: Vec<ParseError>,
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl ParseError {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

pub fn parse<'ast>(tokens: &[Token], source: &str, arena: &'ast AstArena) -> ParseResult<'ast> {
    reset_pending_gt();

    let mut items = Vec::with_capacity(32);
    let mut errors = Vec::with_capacity(4);
    let mut input = TokenStream::new(tokens, source);

    let start_span = input.current_span();

    while !is_eof(input) {
        match parse_item(arena, input) {
            Ok((rest, item)) => {
                items.push(item);
                input = rest;
            }
            Err(e) => {
                let (err_span, err_msg) = match &e {
                    nom::Err::Error(pe) | nom::Err::Failure(pe) => {
                        let span = pe.input.current_span();
                        let msg = format!("{:?}", pe.kind);
                        (span, msg)
                    }
                    nom::Err::Incomplete(_) => (input.current_span(), "Incomplete input".to_string()),
                };

                errors.push(ParseError::new(err_msg, err_span));

                if !input.is_empty() {
                    let (rest, _) = input.take_split(1);
                    input = rest;
                }
            }
        }
    }

    let end_span = if items.is_empty() {
        start_span
    } else {
        items.last().map(|i| i.span()).unwrap_or(start_span)
    };

    ParseResult {
        ast: SourceFile::new(items, start_span.merge(end_span)),
        errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    fn assert_parses(source: &str) {
        let (tokens, _interner) = tokenize(source);
        let arena = AstArena::new();
        let result = parse(&tokens, source, &arena);
        assert!(result.errors.is_empty(), "Parse errors for '{}': {:?}", source, result.errors);
    }

    fn assert_parses_items(source: &str, expected_items: usize) {
        let (tokens, _interner) = tokenize(source);
        let arena = AstArena::new();
        let result = parse(&tokens, source, &arena);
        assert!(result.errors.is_empty(), "Parse errors for '{}': {:?}", source, result.errors);
        assert_eq!(result.ast.items.len(), expected_items);
    }

    #[test]
    fn test_parse_empty() {
        let source = "";
        let (tokens, _interner) = tokenize(source);
        let arena = AstArena::new();
        let result = parse(&tokens, source, &arena);
        assert!(result.errors.is_empty());
        assert!(result.ast.items.is_empty());
    }

    #[test]
    fn test_parse_simple_function() {
        assert_parses_items("fn main() { }", 1);
    }

    #[test]
    fn test_parse_function_with_return() {
        assert_parses_items("fn add(a: int, b: int) -> int { return a + b; }", 1);
    }

    #[test]
    fn test_parse_struct() {
        assert_parses_items("struct Point { x: int, y: int }", 1);
    }

    #[test]
    fn test_parse_enum() {
        assert_parses_items("enum Color { Red, Green, Blue }", 1);
    }

    #[test]
    fn test_parse_var_statement() {
        assert_parses("fn main() { var x = 42; }");
    }

    #[test]
    fn test_parse_if_statement() {
        assert_parses("fn main() { if (x > 0) { return 1; } }");
    }

    #[test]
    fn test_parse_generic_type() {
        assert_parses("fn identity<T>(x: T) -> T { return x; }");
    }

    #[test]
    fn test_parse_nested_generics() {
        assert_parses("fn test() { var x: Map<string, Option<int>>; }");
    }

    #[test]
    fn test_parse_import() {
        assert_parses_items("import std.io;", 1);
    }

    #[test]
    fn test_parse_method() {
        assert_parses("fn (self: Point) distance() -> float { return 0.0; }");
    }

    #[test]
    fn test_parse_tuple_array_type() {
        assert_parses("fn zip() -> [(int, int)] { return []; }");
    }

    #[test]
    fn test_parse_generic_method() {
        assert_parses("fn (self: List<T>) size() -> int { return 0; }");
    }

    #[test]
    fn test_parse_range_expression() {
        assert_parses("fn test() { for (i in 0..10) { } }");
    }

    #[test]
    fn test_parse_map_literal() {
        assert_parses(r#"fn test() { var x: map<string, string> = { "key": "value" }; }"#);
    }

    #[test]
    fn test_parse_lambda_in_call() {
        assert_parses("fn test() { map_array(arr, fn(x: int) -> int { return x * 2; }); }");
    }

    #[test]
    fn test_parse_mut_receiver() {
        assert_parses("fn (mut self: List<T>) add(item: T) { }");
    }
}
