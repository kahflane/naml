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
use memchr::{memchr, memchr2};

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;

#[inline]
fn skip_whitespace_simd(bytes: &[u8]) -> usize {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("sse2") {
            return unsafe { skip_whitespace_sse2(bytes) };
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        return unsafe { skip_whitespace_neon(bytes) };
    }

    #[allow(unreachable_code)]
    skip_whitespace_scalar(bytes)
}

#[inline]
fn skip_whitespace_scalar(bytes: &[u8]) -> usize {
    for (i, &b) in bytes.iter().enumerate() {
        if !matches!(b, b' ' | b'\t' | b'\r') {
            return i;
        }
    }
    bytes.len()
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
unsafe fn skip_whitespace_sse2(bytes: &[u8]) -> usize {
    let len = bytes.len();
    if len < 16 {
        return skip_whitespace_scalar(bytes);
    }

    let space = _mm_set1_epi8(b' ' as i8);
    let tab = _mm_set1_epi8(b'\t' as i8);
    let cr = _mm_set1_epi8(b'\r' as i8);

    let mut i = 0;
    while i + 16 <= len {
        let chunk = _mm_loadu_si128(bytes.as_ptr().add(i) as *const __m128i);

        let is_space = _mm_cmpeq_epi8(chunk, space);
        let is_tab = _mm_cmpeq_epi8(chunk, tab);
        let is_cr = _mm_cmpeq_epi8(chunk, cr);

        let is_ws = _mm_or_si128(_mm_or_si128(is_space, is_tab), is_cr);
        let mask = _mm_movemask_epi8(is_ws) as u32;

        if mask != 0xFFFF {
            return i + mask.trailing_ones() as usize;
        }
        i += 16;
    }

    i + skip_whitespace_scalar(&bytes[i..])
}

#[cfg(target_arch = "aarch64")]
unsafe fn skip_whitespace_neon(bytes: &[u8]) -> usize {
    let len = bytes.len();
    if len < 16 {
        return skip_whitespace_scalar(bytes);
    }

    unsafe {
        let space = vdupq_n_u8(b' ');
        let tab = vdupq_n_u8(b'\t');
        let cr = vdupq_n_u8(b'\r');

        let mut i = 0;
        while i + 16 <= len {
            let chunk = vld1q_u8(bytes.as_ptr().add(i));

            let is_space = vceqq_u8(chunk, space);
            let is_tab = vceqq_u8(chunk, tab);
            let is_cr = vceqq_u8(chunk, cr);

            let is_ws = vorrq_u8(vorrq_u8(is_space, is_tab), is_cr);

            let narrowed = vshrn_n_u16(vreinterpretq_u16_u8(is_ws), 4);
            let mask = vget_lane_u64(vreinterpret_u64_u8(narrowed), 0);

            if mask != 0xFFFFFFFFFFFFFFFF {
                for j in 0..16 {
                    if !matches!(bytes[i + j], b' ' | b'\t' | b'\r') {
                        return i + j;
                    }
                }
            }
            i += 16;
        }

        i + skip_whitespace_scalar(&bytes[i..])
    }
}

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
    bytes: &'a [u8],
    pos: usize,
    interner: Rodeo,
    file_id: u32,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
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

    #[inline(always)]
    fn is_eof(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    #[inline(always)]
    fn peek_byte(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    #[inline(always)]
    fn peek_byte2(&self) -> Option<u8> {
        self.bytes.get(self.pos + 1).copied()
    }

    #[inline(always)]
    fn advance_byte(&mut self) -> Option<u8> {
        let b = self.peek_byte()?;
        self.pos += 1;
        Some(b)
    }

    #[inline(always)]
    fn advance_char(&mut self) -> Option<char> {
        let c = self.source[self.pos..].chars().next()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn next_token(&mut self) -> Token {
        let start = self.pos as u32;

        let Some(b) = self.advance_byte() else {
            return Token::new(TokenKind::Eof, Span::new(start, start, self.file_id));
        };

        let kind = match b {
            b' ' | b'\t' | b'\r' => {
                // SIMD-accelerated whitespace skip
                self.pos += skip_whitespace_simd(&self.bytes[self.pos..]);
                TokenKind::Whitespace
            }

            b'\n' => TokenKind::Newline,

            b'/' if self.peek_byte() == Some(b'/') => {
                // SIMD-accelerated newline search
                if let Some(offset) = memchr(b'\n', &self.bytes[self.pos..]) {
                    self.pos += offset;
                } else {
                    self.pos = self.bytes.len();
                }
                TokenKind::Comment
            }

            b'/' if self.peek_byte() == Some(b'*') => {
                self.pos += 1;
                let mut depth = 1;
                while depth > 0 && !self.is_eof() {
                    match (self.peek_byte(), self.peek_byte2()) {
                        (Some(b'/'), Some(b'*')) => {
                            self.pos += 2;
                            depth += 1;
                        }
                        (Some(b'*'), Some(b'/')) => {
                            self.pos += 2;
                            depth -= 1;
                        }
                        _ => {
                            self.pos += 1;
                        }
                    }
                }
                TokenKind::Comment
            }

            b'+' if self.peek_byte() == Some(b'=') => {
                self.pos += 1;
                TokenKind::PlusEq
            }
            b'+' => TokenKind::Plus,

            b'-' if self.peek_byte() == Some(b'>') => {
                self.pos += 1;
                TokenKind::Arrow
            }
            b'-' if self.peek_byte() == Some(b'=') => {
                self.pos += 1;
                TokenKind::MinusEq
            }
            b'-' => TokenKind::Minus,

            b'*' if self.peek_byte() == Some(b'=') => {
                self.pos += 1;
                TokenKind::StarEq
            }
            b'*' => TokenKind::Star,

            b'/' if self.peek_byte() == Some(b'=') => {
                self.pos += 1;
                TokenKind::SlashEq
            }
            b'/' => TokenKind::Slash,

            b'%' if self.peek_byte() == Some(b'=') => {
                self.pos += 1;
                TokenKind::PercentEq
            }
            b'%' => TokenKind::Percent,

            b'^' if self.peek_byte() == Some(b'=') => {
                self.pos += 1;
                TokenKind::CaretEq
            }
            b'^' => TokenKind::Caret,

            b'&' if self.peek_byte() == Some(b'&') => {
                self.pos += 1;
                TokenKind::AndAnd
            }
            b'&' if self.peek_byte() == Some(b'=') => {
                self.pos += 1;
                TokenKind::AmpersandEq
            }
            b'&' => TokenKind::Ampersand,

            b'|' if self.peek_byte() == Some(b'|') => {
                self.pos += 1;
                TokenKind::PipePipe
            }
            b'|' if self.peek_byte() == Some(b'=') => {
                self.pos += 1;
                TokenKind::PipeEq
            }
            b'|' => TokenKind::Pipe,

            b'~' => TokenKind::Tilde,

            b'!' if self.peek_byte() == Some(b'=') => {
                self.pos += 1;
                TokenKind::NotEq
            }
            b'!' => TokenKind::Bang,

            b'=' if self.peek_byte() == Some(b'=') => {
                self.pos += 1;
                TokenKind::EqEq
            }
            b'=' if self.peek_byte() == Some(b'>') => {
                self.pos += 1;
                TokenKind::FatArrow
            }
            b'=' => TokenKind::Eq,

            b'<' if self.peek_byte() == Some(b'<') => {
                self.pos += 1;
                TokenKind::LtLt
            }
            b'<' if self.peek_byte() == Some(b'=') => {
                self.pos += 1;
                TokenKind::LtEq
            }
            b'<' => TokenKind::Lt,

            b'>' if self.peek_byte() == Some(b'>') => {
                self.pos += 1;
                TokenKind::GtGt
            }
            b'>' if self.peek_byte() == Some(b'=') => {
                self.pos += 1;
                TokenKind::GtEq
            }
            b'>' => TokenKind::Gt,

            b'.' if self.peek_byte() == Some(b'.') => {
                self.pos += 1;
                if self.peek_byte() == Some(b'=') {
                    self.pos += 1;
                    TokenKind::DotDotEq
                } else {
                    TokenKind::DotDot
                }
            }
            b'.' => TokenKind::Dot,

            b',' => TokenKind::Comma,

            b':' if self.peek_byte() == Some(b':') => {
                self.pos += 1;
                TokenKind::ColonColon
            }
            b':' => TokenKind::Colon,

            b';' => TokenKind::Semicolon,
            b'?' => TokenKind::Question,

            b'(' => TokenKind::LParen,
            b')' => TokenKind::RParen,
            b'{' => TokenKind::LBrace,
            b'}' => TokenKind::RBrace,
            b'[' => TokenKind::LBracket,
            b']' => TokenKind::RBracket,

            b'@' => TokenKind::At,
            b'#' => TokenKind::Hash,

            b'"' => self.scan_string(),

            b'0'..=b'9' => self.scan_number(start),

            b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.scan_ident_or_keyword(start),

            // Non-ASCII: fall back to char-based handling for Unicode identifiers
            _ if b > 127 => {
                self.pos -= 1; // Undo the advance_byte
                let c = self.advance_char().unwrap();
                if c.is_alphabetic() {
                    self.scan_ident_or_keyword_unicode(start)
                } else {
                    TokenKind::Error
                }
            }

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
        loop {
            // SIMD-accelerated search for quote or backslash
            match memchr2(b'"', b'\\', &self.bytes[self.pos..]) {
                Some(offset) => {
                    // Check for newline before the found character
                    if let Some(nl_offset) = memchr(b'\n', &self.bytes[self.pos..self.pos + offset]) {
                        self.pos += nl_offset;
                        return TokenKind::Error; // Unterminated string
                    }
                    self.pos += offset;
                    match self.bytes[self.pos] {
                        b'"' => {
                            self.pos += 1;
                            return TokenKind::StringLit;
                        }
                        b'\\' => {
                            self.pos += 1;
                            if self.pos < self.bytes.len() {
                                self.pos += 1; // Skip escaped char
                            }
                        }
                        _ => unreachable!(),
                    }
                }
                None => {
                    // Check if there's a newline before EOF
                    if let Some(nl_offset) = memchr(b'\n', &self.bytes[self.pos..]) {
                        self.pos += nl_offset;
                    } else {
                        self.pos = self.bytes.len();
                    }
                    return TokenKind::Error; // Unterminated string
                }
            }
        }
    }

    fn scan_number(&mut self, _start: u32) -> TokenKind {
        while matches!(self.peek_byte(), Some(b'0'..=b'9' | b'_')) {
            self.pos += 1;
        }

        if self.peek_byte() == Some(b'.') && matches!(self.peek_byte2(), Some(b'0'..=b'9')) {
            self.pos += 1;
            while matches!(self.peek_byte(), Some(b'0'..=b'9' | b'_')) {
                self.pos += 1;
            }
            return TokenKind::FloatLit;
        }

        if matches!(self.peek_byte(), Some(b'e' | b'E')) {
            self.pos += 1;
            if matches!(self.peek_byte(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            while matches!(self.peek_byte(), Some(b'0'..=b'9' | b'_')) {
                self.pos += 1;
            }
            return TokenKind::FloatLit;
        }

        TokenKind::IntLit
    }

    fn scan_ident_or_keyword(&mut self, start: u32) -> TokenKind {
        // Fast path: ASCII-only identifiers
        while let Some(b) = self.peek_byte() {
            match b {
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' => {
                    self.pos += 1;
                }
                _ if b > 127 => {
                    // Contains Unicode - switch to slow path
                    return self.scan_ident_or_keyword_unicode(start);
                }
                _ => break,
            }
        }

        self.match_keyword(start)
    }

    fn scan_ident_or_keyword_unicode(&mut self, start: u32) -> TokenKind {
        // Slow path: Unicode identifiers
        while let Some(c) = self.source[self.pos..].chars().next() {
            if c.is_alphanumeric() || c == '_' {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }

        self.match_keyword(start)
    }

    #[inline]
    fn match_keyword(&self, start: u32) -> TokenKind {
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
