///
/// Parser Module - nom-based Token Parsing
///
/// This module provides the parser for the naml programming language.
/// It uses nom parser combinators to parse a stream of tokens into an AST.
///
/// The parser is structured as follows:
/// - input: TokenStream type for nom integration
/// - combinators: Reusable token-matching combinators
/// - types: Type annotation parsing
/// - expressions: Expression parsing with Pratt precedence
/// - statements: Statement parsing
/// - items: Top-level item parsing
///
/// Entry point: parse() function takes tokens and returns a SourceFile AST.
///

mod combinators;
mod expressions;
mod input;
mod items;
mod statements;
mod types;

pub use combinators::{PError, PErrorKind};
pub use input::TokenStream;

use nom::InputTake;

use crate::ast::SourceFile;
use crate::lexer::Token;
use crate::source::{Span, Spanned};

use combinators::is_eof;
use items::parse_item;
use types::reset_pending_gt;

pub struct ParseResult {
    pub ast: SourceFile,
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

pub fn parse(tokens: &[Token]) -> ParseResult {
    reset_pending_gt();

    let mut items = Vec::with_capacity(32);
    let mut errors = Vec::with_capacity(4);
    let mut input = TokenStream::new(tokens);

    let start_span = input.current_span();

    while !is_eof(input) {
        match parse_item(input) {
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

    fn parse_str(source: &str) -> ParseResult {
        let (tokens, _interner) = tokenize(source);
        parse(&tokens)
    }

    #[test]
    fn test_parse_empty() {
        let result = parse_str("");
        assert!(result.errors.is_empty());
        assert!(result.ast.items.is_empty());
    }

    #[test]
    fn test_parse_simple_function() {
        let result = parse_str("fn main() { }");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.ast.items.len(), 1);
    }

    #[test]
    fn test_parse_function_with_return() {
        let result = parse_str("fn add(a: int, b: int) -> int { return a + b; }");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.ast.items.len(), 1);
    }

    #[test]
    fn test_parse_struct() {
        let result = parse_str("struct Point { x: int, y: int }");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.ast.items.len(), 1);
    }

    #[test]
    fn test_parse_enum() {
        let result = parse_str("enum Color { Red, Green, Blue }");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.ast.items.len(), 1);
    }

    #[test]
    fn test_parse_var_statement() {
        let result = parse_str("fn main() { var x = 42; }");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_parse_if_statement() {
        let result = parse_str("fn main() { if (x > 0) { return 1; } }");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_parse_generic_type() {
        let result = parse_str("fn identity<T>(x: T) -> T { return x; }");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_parse_nested_generics() {
        let result = parse_str("fn test() { var x: Map<string, Option<int>>; }");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_parse_import() {
        let result = parse_str("import std.io;");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
        assert_eq!(result.ast.items.len(), 1);
    }

    #[test]
    fn test_parse_method() {
        let result = parse_str("fn (self: Point) distance() -> float { return 0.0; }");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_parse_tuple_array_type() {
        let result = parse_str("fn zip() -> [(int, int)] { return []; }");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_parse_generic_method() {
        let result = parse_str("fn (self: List<T>) size() -> int { return 0; }");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_parse_range_expression() {
        let result = parse_str("fn test() { for (i in 0..10) { } }");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_parse_map_literal() {
        let result = parse_str(r#"fn test() { var x: map<string, string> = { "key": "value" }; }"#);
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_parse_lambda_in_call() {
        let result = parse_str("fn test() { map_array(arr, |x: int| x * 2); }");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_parse_async_fn() {
        let result = parse_str("pub async fn get() -> int throws Error { return 0; }");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_parse_mut_receiver() {
        let result = parse_str("fn (mut self: List<T>) add(item: T) { }");
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    }
}
