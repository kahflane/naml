///
/// Lexer Module - Zero-Copy Tokenization
///
/// This module handles tokenization of naml source code. It produces a
/// stream of tokens that the parser consumes to build the AST.
///
/// Key design decisions:
/// - Zero-copy: Tokens reference the source string, no allocations per token
/// - String interning: Identifiers and strings stored via lasso::Spur
/// - Whitespace/comments filtered out for fast parsing (no trivia in output)
///
/// Token categories:
/// - Keywords: fn, var, const, if, while, for, etc.
/// - Identifiers: User-defined names
/// - Literals: Numbers, strings, booleans
/// - Operators: +, -, *, /, ==, etc.
/// - Delimiters: (, ), {, }, [, ], etc.
/// - Trivia: Whitespace, comments (preserved but skippable)
///

use crate::source::Span;
use lasso::{Rodeo, Spur};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub symbol: Option<Spur>,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self {
            kind,
            span,
            symbol: None,
        }
    }

    pub fn with_symbol(kind: TokenKind, span: Span, symbol: Spur) -> Self {
        Self {
            kind,
            span,
            symbol: Some(symbol),
        }
    }

    pub fn is_trivia(&self) -> bool {
        matches!(
            self.kind,
            TokenKind::Whitespace | TokenKind::Comment | TokenKind::Newline
        )
    }

    pub fn is_eof(&self) -> bool {
        self.kind == TokenKind::Eof
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    Ident,
    IntLit,
    FloatLit,
    StringLit,
    BytesLit,

    Keyword(Keyword),

    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    Ampersand,
    Pipe,
    Tilde,
    Bang,

    Eq,
    EqEq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    LtLt,
    GtGt,

    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    AmpersandEq,
    PipeEq,
    CaretEq,

    AndAnd,
    PipePipe,

    Dot,
    DotDot,
    DotDotEq,
    Comma,
    Colon,
    ColonColon,
    Semicolon,
    Arrow,
    FatArrow,
    Question,

    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,

    At,
    Hash,

    Whitespace,
    Newline,
    Comment,

    Error,
    Eof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keyword {
    Fn,
    Var,
    Const,
    Mut,
    Pub,
    If,
    Else,
    While,
    For,
    In,
    Loop,
    Break,
    Continue,
    Return,
    Switch,
    Case,
    Default,
    Struct,
    Enum,
    Interface,
    Exception,
    Import,
    Use,
    Extern,
    Async,
    Await,
    Spawn,
    Throw,
    Throws,
    Try,
    As,
    Is,
    Implements,
    Not,
    And,
    Or,
    True,
    False,
    None,
    Some,
    Int,
    Uint,
    Float,
    Bool,
    String,
    Bytes,
    Option,
    Map,
    Channel,
    Promise,
    Platforms,
    Native,
    Server,
    Browser,
}

pub fn tokenize(source: &str) -> (Vec<Token>, Rodeo) {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_all();
    (tokens, lexer.interner)
}

struct Lexer<'a> {
    source: &'a str,
    pos: usize,
    interner: Rodeo,
    file_id: u32,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            pos: 0,
            interner: Rodeo::default(),
            file_id: 0,
        }
    }

    fn tokenize_all(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        while !self.is_eof() {
            let token = self.next_token();
            // Filter trivia at source - parser never sees whitespace/comments
            if !token.is_trivia() {
                tokens.push(token);
            }
        }

        tokens.push(Token::new(
            TokenKind::Eof,
            Span::new(self.pos as u32, self.pos as u32, self.file_id),
        ));

        tokens
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.source.len()
    }

    fn peek(&self) -> Option<char> {
        self.source[self.pos..].chars().next()
    }

    fn peek_next(&self) -> Option<char> {
        let mut chars = self.source[self.pos..].chars();
        chars.next();
        chars.next()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn next_token(&mut self) -> Token {
        let start = self.pos as u32;

        let Some(c) = self.advance() else {
            return Token::new(TokenKind::Eof, Span::new(start, start, self.file_id));
        };

        let kind = match c {
            ' ' | '\t' | '\r' => {
                while matches!(self.peek(), Some(' ' | '\t' | '\r')) {
                    self.advance();
                }
                TokenKind::Whitespace
            }

            '\n' => TokenKind::Newline,

            '/' if self.peek() == Some('/') => {
                while self.peek().is_some() && self.peek() != Some('\n') {
                    self.advance();
                }
                TokenKind::Comment
            }

            '/' if self.peek() == Some('*') => {
                self.advance();
                let mut depth = 1;
                while depth > 0 && !self.is_eof() {
                    match (self.peek(), self.peek_next()) {
                        (Some('/'), Some('*')) => {
                            self.advance();
                            self.advance();
                            depth += 1;
                        }
                        (Some('*'), Some('/')) => {
                            self.advance();
                            self.advance();
                            depth -= 1;
                        }
                        _ => {
                            self.advance();
                        }
                    }
                }
                TokenKind::Comment
            }

            '+' if self.peek() == Some('=') => {
                self.advance();
                TokenKind::PlusEq
            }
            '+' => TokenKind::Plus,

            '-' if self.peek() == Some('>') => {
                self.advance();
                TokenKind::Arrow
            }
            '-' if self.peek() == Some('=') => {
                self.advance();
                TokenKind::MinusEq
            }
            '-' => TokenKind::Minus,

            '*' if self.peek() == Some('=') => {
                self.advance();
                TokenKind::StarEq
            }
            '*' => TokenKind::Star,

            '/' if self.peek() == Some('=') => {
                self.advance();
                TokenKind::SlashEq
            }
            '/' => TokenKind::Slash,

            '%' if self.peek() == Some('=') => {
                self.advance();
                TokenKind::PercentEq
            }
            '%' => TokenKind::Percent,

            '^' if self.peek() == Some('=') => {
                self.advance();
                TokenKind::CaretEq
            }
            '^' => TokenKind::Caret,

            '&' if self.peek() == Some('&') => {
                self.advance();
                TokenKind::AndAnd
            }
            '&' if self.peek() == Some('=') => {
                self.advance();
                TokenKind::AmpersandEq
            }
            '&' => TokenKind::Ampersand,

            '|' if self.peek() == Some('|') => {
                self.advance();
                TokenKind::PipePipe
            }
            '|' if self.peek() == Some('=') => {
                self.advance();
                TokenKind::PipeEq
            }
            '|' => TokenKind::Pipe,

            '~' => TokenKind::Tilde,

            '!' if self.peek() == Some('=') => {
                self.advance();
                TokenKind::NotEq
            }
            '!' => TokenKind::Bang,

            '=' if self.peek() == Some('=') => {
                self.advance();
                TokenKind::EqEq
            }
            '=' if self.peek() == Some('>') => {
                self.advance();
                TokenKind::FatArrow
            }
            '=' => TokenKind::Eq,

            '<' if self.peek() == Some('<') => {
                self.advance();
                TokenKind::LtLt
            }
            '<' if self.peek() == Some('=') => {
                self.advance();
                TokenKind::LtEq
            }
            '<' => TokenKind::Lt,

            '>' if self.peek() == Some('>') => {
                self.advance();
                TokenKind::GtGt
            }
            '>' if self.peek() == Some('=') => {
                self.advance();
                TokenKind::GtEq
            }
            '>' => TokenKind::Gt,

            '.' if self.peek() == Some('.') => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::DotDotEq
                } else {
                    TokenKind::DotDot
                }
            }
            '.' => TokenKind::Dot,

            ',' => TokenKind::Comma,

            ':' if self.peek() == Some(':') => {
                self.advance();
                TokenKind::ColonColon
            }
            ':' => TokenKind::Colon,

            ';' => TokenKind::Semicolon,
            '?' => TokenKind::Question,

            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,

            '@' => TokenKind::At,
            '#' => TokenKind::Hash,

            '"' => self.scan_string(),

            '0'..='9' => self.scan_number(start),

            c if c.is_alphabetic() || c == '_' => self.scan_ident_or_keyword(start),

            _ => TokenKind::Error,
        };

        let end = self.pos as u32;
        let span = Span::new(start, end, self.file_id);

        if kind == TokenKind::Ident {
            let text = &self.source[start as usize..end as usize];
            let symbol = self.interner.get_or_intern(text);
            Token::with_symbol(kind, span, symbol)
        } else if kind == TokenKind::StringLit {
            let text = &self.source[(start as usize + 1)..(end as usize - 1)];
            let symbol = self.interner.get_or_intern(text);
            Token::with_symbol(kind, span, symbol)
        } else {
            Token::new(kind, span)
        }
    }

    fn scan_string(&mut self) -> TokenKind {
        while let Some(c) = self.peek() {
            match c {
                '"' => {
                    self.advance();
                    return TokenKind::StringLit;
                }
                '\\' => {
                    self.advance();
                    self.advance();
                }
                '\n' => return TokenKind::Error,
                _ => {
                    self.advance();
                }
            }
        }
        TokenKind::Error
    }

    fn scan_number(&mut self, _start: u32) -> TokenKind {
        while matches!(self.peek(), Some('0'..='9' | '_')) {
            self.advance();
        }

        if self.peek() == Some('.') && matches!(self.peek_next(), Some('0'..='9')) {
            self.advance();
            while matches!(self.peek(), Some('0'..='9' | '_')) {
                self.advance();
            }
            return TokenKind::FloatLit;
        }

        if matches!(self.peek(), Some('e' | 'E')) {
            self.advance();
            if matches!(self.peek(), Some('+' | '-')) {
                self.advance();
            }
            while matches!(self.peek(), Some('0'..='9' | '_')) {
                self.advance();
            }
            return TokenKind::FloatLit;
        }

        TokenKind::IntLit
    }

    fn scan_ident_or_keyword(&mut self, start: u32) -> TokenKind {
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }

        let text = &self.source[start as usize..self.pos];

        match text {
            "fn" => TokenKind::Keyword(Keyword::Fn),
            "var" => TokenKind::Keyword(Keyword::Var),
            "const" => TokenKind::Keyword(Keyword::Const),
            "mut" => TokenKind::Keyword(Keyword::Mut),
            "pub" => TokenKind::Keyword(Keyword::Pub),
            "if" => TokenKind::Keyword(Keyword::If),
            "else" => TokenKind::Keyword(Keyword::Else),
            "while" => TokenKind::Keyword(Keyword::While),
            "for" => TokenKind::Keyword(Keyword::For),
            "in" => TokenKind::Keyword(Keyword::In),
            "loop" => TokenKind::Keyword(Keyword::Loop),
            "break" => TokenKind::Keyword(Keyword::Break),
            "continue" => TokenKind::Keyword(Keyword::Continue),
            "return" => TokenKind::Keyword(Keyword::Return),
            "switch" => TokenKind::Keyword(Keyword::Switch),
            "case" => TokenKind::Keyword(Keyword::Case),
            "default" => TokenKind::Keyword(Keyword::Default),
            "struct" => TokenKind::Keyword(Keyword::Struct),
            "enum" => TokenKind::Keyword(Keyword::Enum),
            "interface" => TokenKind::Keyword(Keyword::Interface),
            "exception" => TokenKind::Keyword(Keyword::Exception),
            "import" => TokenKind::Keyword(Keyword::Import),
            "use" => TokenKind::Keyword(Keyword::Use),
            "extern" => TokenKind::Keyword(Keyword::Extern),
            "async" => TokenKind::Keyword(Keyword::Async),
            "await" => TokenKind::Keyword(Keyword::Await),
            "spawn" => TokenKind::Keyword(Keyword::Spawn),
            "throw" => TokenKind::Keyword(Keyword::Throw),
            "throws" => TokenKind::Keyword(Keyword::Throws),
            "try" => TokenKind::Keyword(Keyword::Try),
            "as" => TokenKind::Keyword(Keyword::As),
            "is" => TokenKind::Keyword(Keyword::Is),
            "implements" => TokenKind::Keyword(Keyword::Implements),
            "not" => TokenKind::Keyword(Keyword::Not),
            "and" => TokenKind::Keyword(Keyword::And),
            "or" => TokenKind::Keyword(Keyword::Or),
            "true" => TokenKind::Keyword(Keyword::True),
            "false" => TokenKind::Keyword(Keyword::False),
            "none" => TokenKind::Keyword(Keyword::None),
            "some" => TokenKind::Keyword(Keyword::Some),
            "int" => TokenKind::Keyword(Keyword::Int),
            "uint" => TokenKind::Keyword(Keyword::Uint),
            "float" => TokenKind::Keyword(Keyword::Float),
            "bool" => TokenKind::Keyword(Keyword::Bool),
            "string" => TokenKind::Keyword(Keyword::String),
            "bytes" => TokenKind::Keyword(Keyword::Bytes),
            "option" => TokenKind::Keyword(Keyword::Option),
            "map" => TokenKind::Keyword(Keyword::Map),
            "channel" => TokenKind::Keyword(Keyword::Channel),
            "promise" => TokenKind::Keyword(Keyword::Promise),
            "platforms" => TokenKind::Keyword(Keyword::Platforms),
            "native" => TokenKind::Keyword(Keyword::Native),
            "server" => TokenKind::Keyword(Keyword::Server),
            "browser" => TokenKind::Keyword(Keyword::Browser),
            _ => TokenKind::Ident,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_empty() {
        let (tokens, _) = tokenize("");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::Eof);
    }

    #[test]
    fn test_tokenize_operators() {
        let (tokens, _) = tokenize("+ - * / == != < >");
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::EqEq,
                TokenKind::NotEq,
                TokenKind::Lt,
                TokenKind::Gt,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_tokenize_keywords() {
        let (tokens, _) = tokenize("fn var if else while for");
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Keyword(Keyword::Fn),
                TokenKind::Keyword(Keyword::Var),
                TokenKind::Keyword(Keyword::If),
                TokenKind::Keyword(Keyword::Else),
                TokenKind::Keyword(Keyword::While),
                TokenKind::Keyword(Keyword::For),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn test_tokenize_numbers() {
        let (tokens, _) = tokenize("42 3.14 1_000");
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![TokenKind::IntLit, TokenKind::FloatLit, TokenKind::IntLit, TokenKind::Eof]
        );
    }

    #[test]
    fn test_tokenize_string() {
        let (tokens, interner) = tokenize("\"hello world\"");
        assert_eq!(tokens[0].kind, TokenKind::StringLit);
        let symbol = tokens[0].symbol.unwrap();
        assert_eq!(interner.resolve(&symbol), "hello world");
    }

    #[test]
    fn test_tokenize_comment() {
        // Comments are filtered out - parser never sees them
        let (tokens, _) = tokenize("x // comment\ny");
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
        assert!(!kinds.contains(&TokenKind::Comment));
        assert_eq!(kinds, vec![TokenKind::Ident, TokenKind::Ident, TokenKind::Eof]);
    }
}
