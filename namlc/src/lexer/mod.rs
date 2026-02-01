//!
//! Lexer Module - Zero-Copy Tokenization
//!
//! This module handles tokenization of naml source code. It produces a
//! stream of tokens that the parser consumes to build the AST.
//!
//! Key design decisions:
//! - Zero-copy: Tokens reference the source string, no allocations per token
//! - String interning: Identifiers and strings stored via lasso::Spur
//! - Whitespace/comments filtered out for fast parsing (no trivia in output)
//!
//! Token categories:
//! - Keywords: fn, var, const, if, while, for, etc.
//! - Identifiers: User-defined names
//! - Literals: Numbers, strings, booleans
//! - Operators: +, -, *, /, ==, etc.
//! - Delimiters: (, ), {, }, [, ], etc.
//! - Trivia: Whitespace, comments (preserved but skippable)
//!

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

#[inline]
fn find_ident_end_simd(bytes: &[u8]) -> usize {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("sse2") {
            return unsafe { find_ident_end_sse2(bytes) };
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        return unsafe { find_ident_end_neon(bytes) };
    }

    #[allow(unreachable_code)]
    find_ident_end_scalar(bytes)
}

#[inline]
fn find_ident_end_scalar(bytes: &[u8]) -> usize {
    for (i, &b) in bytes.iter().enumerate() {
        if !matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_') {
            return i;
        }
    }
    bytes.len()
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
unsafe fn find_ident_end_sse2(bytes: &[u8]) -> usize {
    let len = bytes.len();
    if len < 16 {
        return find_ident_end_scalar(bytes);
    }

    let lower_a = _mm_set1_epi8((b'a' - 1) as i8);
    let lower_z = _mm_set1_epi8((b'z' + 1) as i8);
    let upper_a = _mm_set1_epi8((b'A' - 1) as i8);
    let upper_z = _mm_set1_epi8((b'Z' + 1) as i8);
    let digit_0 = _mm_set1_epi8((b'0' - 1) as i8);
    let digit_9 = _mm_set1_epi8((b'9' + 1) as i8);
    let underscore = _mm_set1_epi8(b'_' as i8);

    let mut i = 0;
    while i + 16 <= len {
        let chunk = _mm_loadu_si128(bytes.as_ptr().add(i) as *const __m128i);

        let is_lower = _mm_and_si128(
            _mm_cmpgt_epi8(chunk, lower_a),
            _mm_cmplt_epi8(chunk, lower_z),
        );
        let is_upper = _mm_and_si128(
            _mm_cmpgt_epi8(chunk, upper_a),
            _mm_cmplt_epi8(chunk, upper_z),
        );
        let is_digit = _mm_and_si128(
            _mm_cmpgt_epi8(chunk, digit_0),
            _mm_cmplt_epi8(chunk, digit_9),
        );
        let is_under = _mm_cmpeq_epi8(chunk, underscore);

        let is_ident = _mm_or_si128(
            _mm_or_si128(is_lower, is_upper),
            _mm_or_si128(is_digit, is_under),
        );

        let mask = _mm_movemask_epi8(is_ident) as u32;
        if mask != 0xFFFF {
            return i + mask.trailing_ones() as usize;
        }
        i += 16;
    }

    i + find_ident_end_scalar(&bytes[i..])
}

#[cfg(target_arch = "aarch64")]
unsafe fn find_ident_end_neon(bytes: &[u8]) -> usize {
    let len = bytes.len();
    if len < 16 {
        return find_ident_end_scalar(bytes);
    }

    unsafe {
        let lower_a = vdupq_n_u8(b'a' - 1);
        let lower_z = vdupq_n_u8(b'z' + 1);
        let upper_a = vdupq_n_u8(b'A' - 1);
        let upper_z = vdupq_n_u8(b'Z' + 1);
        let digit_0 = vdupq_n_u8(b'0' - 1);
        let digit_9 = vdupq_n_u8(b'9' + 1);
        let underscore = vdupq_n_u8(b'_');

        let mut i = 0;
        while i + 16 <= len {
            let chunk = vld1q_u8(bytes.as_ptr().add(i));

            let is_lower = vandq_u8(vcgtq_u8(chunk, lower_a), vcltq_u8(chunk, lower_z));
            let is_upper = vandq_u8(vcgtq_u8(chunk, upper_a), vcltq_u8(chunk, upper_z));
            let is_digit = vandq_u8(vcgtq_u8(chunk, digit_0), vcltq_u8(chunk, digit_9));
            let is_under = vceqq_u8(chunk, underscore);

            let is_ident = vorrq_u8(vorrq_u8(is_lower, is_upper), vorrq_u8(is_digit, is_under));

            let narrowed = vshrn_n_u16(vreinterpretq_u16_u8(is_ident), 4);
            let mask = vget_lane_u64(vreinterpret_u64_u8(narrowed), 0);

            if mask != 0xFFFFFFFFFFFFFFFF {
                for j in 0..16 {
                    if !matches!(bytes[i + j], b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_') {
                        return i + j;
                    }
                }
            }
            i += 16;
        }

        i + find_ident_end_scalar(&bytes[i..])
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
    TemplateLit,
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
    QuestionQuestion,

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
    Use,
    Extern,
    Spawn,
    Throw,
    Throws,
    Try,
    Catch,
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
    Decimal,
    Bool,
    String,
    Bytes,
    Option,
    Map,
    Channel,
    Mutex,
    Rwlock,
    Platforms,
    Native,
    Server,
    Browser,
    Type,
    Locked,
    Rlocked,
    Wlocked,
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
            b'?' if self.peek_byte() == Some(b'?') => {
                self.pos += 1;
                TokenKind::QuestionQuestion
            }
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
            b'`' => self.scan_template_string(),

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
            let raw = &self.source[(start as usize + 1)..(end as usize - 1)];
            let text = if raw.contains('\\') {
                let mut result = String::with_capacity(raw.len());
                let mut chars = raw.chars();
                while let Some(c) = chars.next() {
                    if c == '\\' {
                        match chars.next() {
                            Some('n') => result.push('\n'),
                            Some('t') => result.push('\t'),
                            Some('r') => result.push('\r'),
                            Some('\\') => result.push('\\'),
                            Some('"') => result.push('"'),
                            Some('0') => result.push('\0'),
                            Some(other) => { result.push('\\'); result.push(other); }
                            None => result.push('\\'),
                        }
                    } else {
                        result.push(c);
                    }
                }
                std::borrow::Cow::Owned(result)
            } else {
                std::borrow::Cow::Borrowed(raw)
            };
            let symbol = self.interner.get_or_intern(text.as_ref());
            Token::with_symbol(kind, span, symbol)
        } else if kind == TokenKind::TemplateLit {
            // Template strings: only escape \` (backtick), preserve everything else including newlines
            let raw = &self.source[(start as usize + 1)..(end as usize - 1)];
            let text = if raw.contains("\\`") {
                // Only need to escape backticks in template strings
                raw.replace("\\`", "`")
            } else {
                raw.to_string()
            };
            let symbol = self.interner.get_or_intern(&text);
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

    fn scan_template_string(&mut self) -> TokenKind {
        // Template strings are delimited by backticks and support:
        // - Multi-line content (newlines preserved)
        // - {expression} interpolation (parsed later)
        // - Escape only \` (backtick)
        loop {
            match memchr2(b'`', b'\\', &self.bytes[self.pos..]) {
                Some(offset) => {
                    self.pos += offset;
                    match self.bytes[self.pos] {
                        b'`' => {
                            self.pos += 1;
                            return TokenKind::TemplateLit;
                        }
                        b'\\' => {
                            self.pos += 1;
                            if self.pos < self.bytes.len() {
                                // Only skip next char if it's a backtick (the only escape in templates)
                                if self.bytes[self.pos] == b'`' {
                                    self.pos += 1;
                                }
                            }
                        }
                        _ => unreachable!(),
                    }
                }
                None => {
                    self.pos = self.bytes.len();
                    return TokenKind::Error; // Unterminated template string
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
        // Fast path: SIMD-accelerated ASCII identifier scanning
        let remaining = &self.source.as_bytes()[self.pos..];
        let ident_len = find_ident_end_simd(remaining);

        // Check if we hit a Unicode character
        if ident_len < remaining.len() && remaining[ident_len] > 127 {
            self.pos += ident_len;
            return self.scan_ident_or_keyword_unicode(start);
        }

        self.pos += ident_len;
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
        let bytes = &self.source.as_bytes()[start as usize..self.pos];
        let len = bytes.len();

        match len {
            2 => self.match_keyword_2(bytes),
            3 => self.match_keyword_3(bytes),
            4 => self.match_keyword_4(bytes),
            5 => self.match_keyword_5(bytes),
            6 => self.match_keyword_6(bytes),
            7 => self.match_keyword_7(bytes),
            8 => self.match_keyword_8(bytes),
            9 => self.match_keyword_9(bytes),
            10 => self.match_keyword_10(bytes),
            _ => TokenKind::Ident,
        }
    }

    #[inline]
    fn match_keyword_2(&self, bytes: &[u8]) -> TokenKind {
        let word = u16::from_le_bytes([bytes[0], bytes[1]]);
        match word {
            0x6E66 => TokenKind::Keyword(Keyword::Fn),     // "fn"
            0x6669 => TokenKind::Keyword(Keyword::If),     // "if"
            0x6E69 => TokenKind::Keyword(Keyword::In),     // "in"
            0x7369 => TokenKind::Keyword(Keyword::Is),     // "is"
            0x7361 => TokenKind::Keyword(Keyword::As),     // "as"
            0x726F => TokenKind::Keyword(Keyword::Or),     // "or"
            _ => TokenKind::Ident,
        }
    }

    #[inline]
    fn match_keyword_3(&self, bytes: &[u8]) -> TokenKind {
        let b0 = bytes[0];
        let word = u16::from_le_bytes([bytes[1], bytes[2]]);
        match (b0, word) {
            (b'v', 0x7261) => TokenKind::Keyword(Keyword::Var),   // "var"
            (b'm', 0x7475) => TokenKind::Keyword(Keyword::Mut),   // "mut"
            (b'p', 0x6275) => TokenKind::Keyword(Keyword::Pub),   // "pub"
            (b'f', 0x726F) => TokenKind::Keyword(Keyword::For),   // "for"
            (b't', 0x7972) => TokenKind::Keyword(Keyword::Try),   // "try"
            (b'n', 0x746F) => TokenKind::Keyword(Keyword::Not),   // "not"
            (b'a', 0x646E) => TokenKind::Keyword(Keyword::And),   // "and"
            (b'i', 0x746E) => TokenKind::Keyword(Keyword::Int),   // "int"
            (b'm', 0x7061) => TokenKind::Keyword(Keyword::Map),   // "map"
            (b'u', 0x6573) => TokenKind::Keyword(Keyword::Use),   // "use"
            _ => TokenKind::Ident,
        }
    }

    #[inline]
    fn match_keyword_4(&self, bytes: &[u8]) -> TokenKind {
        let word = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        match word {
            0x65736C65 => TokenKind::Keyword(Keyword::Else),   // "else"
            0x706F6F6C => TokenKind::Keyword(Keyword::Loop),   // "loop"
            0x65736163 => TokenKind::Keyword(Keyword::Case),   // "case"
            0x6D756E65 => TokenKind::Keyword(Keyword::Enum),   // "enum"
            0x6C6F6F62 => TokenKind::Keyword(Keyword::Bool),   // "bool"
            0x656D6F73 => TokenKind::Keyword(Keyword::Some),   // "some"
            0x656E6F6E => TokenKind::Keyword(Keyword::None),   // "none"
            0x65757274 => TokenKind::Keyword(Keyword::True),   // "true"
            0x746E6975 => TokenKind::Keyword(Keyword::Uint),   // "uint"
            0x65707974 => TokenKind::Keyword(Keyword::Type),   // "type"
            _ => TokenKind::Ident,
        }
    }

    #[inline]
    fn match_keyword_5(&self, bytes: &[u8]) -> TokenKind {
        let word = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let b4 = bytes[4];
        match (word, b4) {
            (0x736E6F63, b't') => TokenKind::Keyword(Keyword::Const),  // "const"
            (0x6C696877, b'e') => TokenKind::Keyword(Keyword::While),  // "while"
            (0x61657262, b'k') => TokenKind::Keyword(Keyword::Break),  // "break"
            (0x6F726874, b'w') => TokenKind::Keyword(Keyword::Throw),  // "throw"
            (0x77617073, b'n') => TokenKind::Keyword(Keyword::Spawn),  // "spawn"
            (0x616F6C66, b't') => TokenKind::Keyword(Keyword::Float),  // "float"
            (0x65747962, b's') => TokenKind::Keyword(Keyword::Bytes),  // "bytes"
            (0x736C6166, b'e') => TokenKind::Keyword(Keyword::False),  // "false"
            (0x63746163, b'h') => TokenKind::Keyword(Keyword::Catch),  // "catch"
            (0x6574756D, b'x') => TokenKind::Keyword(Keyword::Mutex),  // "mutex"
            _ => TokenKind::Ident,
        }
    }

    #[inline]
    fn match_keyword_6(&self, bytes: &[u8]) -> TokenKind {
        let word1 = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let word2 = u16::from_le_bytes([bytes[4], bytes[5]]);
        match (word1, word2) {
            (0x75746572, 0x6E72) => TokenKind::Keyword(Keyword::Return),  // "return"
            (0x74697773, 0x6863) => TokenKind::Keyword(Keyword::Switch),  // "switch"
            (0x75727473, 0x7463) => TokenKind::Keyword(Keyword::Struct),  // "struct"
            (0x65747865, 0x6E72) => TokenKind::Keyword(Keyword::Extern),  // "extern"
            (0x6F726874, 0x7377) => TokenKind::Keyword(Keyword::Throws),  // "throws"
            (0x69727473, 0x676E) => TokenKind::Keyword(Keyword::String),  // "string"
            (0x6974616E, 0x6576) => TokenKind::Keyword(Keyword::Native),  // "native"
            (0x76726573, 0x7265) => TokenKind::Keyword(Keyword::Server),  // "server"
            (0x6974706F, 0x6E6F) => TokenKind::Keyword(Keyword::Option),  // "option"
            (0x6B636F6C, 0x6465) => TokenKind::Keyword(Keyword::Locked),  // "locked"
            (0x6F6C7772, 0x6B63) => TokenKind::Keyword(Keyword::Rwlock),  // "rwlock"
            _ => TokenKind::Ident,
        }
    }

    #[inline]
    fn match_keyword_7(&self, bytes: &[u8]) -> TokenKind {
        let word1 = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let word2 = u16::from_le_bytes([bytes[4], bytes[5]]);
        let b6 = bytes[6];
        match (word1, word2, b6) {
            (0x61666564, 0x6C75, b't') => TokenKind::Keyword(Keyword::Default),  // "default"
            (0x6E616863, 0x656E, b'l') => TokenKind::Keyword(Keyword::Channel),  // "channel"
            (0x776F7262, 0x6573, b'r') => TokenKind::Keyword(Keyword::Browser),  // "browser"
            (0x69636564, 0x616D, b'l') => TokenKind::Keyword(Keyword::Decimal),  // "decimal"
            (0x636F6C72, 0x656B, b'd') => TokenKind::Keyword(Keyword::Rlocked),  // "rlocked"
            (0x636F6C77, 0x656B, b'd') => TokenKind::Keyword(Keyword::Wlocked),  // "wlocked"
            _ => TokenKind::Ident,
        }
    }

    #[inline]
    fn match_keyword_8(&self, bytes: &[u8]) -> TokenKind {
        let word1 = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let word2 = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        match (word1, word2) {
            (0x746E6F63, 0x65756E69) => TokenKind::Keyword(Keyword::Continue),   // "continue"
            _ => TokenKind::Ident,
        }
    }

    #[inline]
    fn match_keyword_9(&self, bytes: &[u8]) -> TokenKind {
        let word1 = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let word2 = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let b8 = bytes[8];
        match (word1, word2, b8) {
            (0x65746E69, 0x63616672, b'e') => TokenKind::Keyword(Keyword::Interface),  // "interface"
            (0x65637865, 0x6F697470, b'n') => TokenKind::Keyword(Keyword::Exception),  // "exception"
            (0x6D726F66, 0x6D746167, b's') => TokenKind::Keyword(Keyword::Platforms),  // "platforms"
            _ => TokenKind::Ident,
        }
    }

    #[inline]
    fn match_keyword_10(&self, bytes: &[u8]) -> TokenKind {
        let word1 = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let word2 = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let word3 = u16::from_le_bytes([bytes[8], bytes[9]]);
        match (word1, word2, word3) {
            (0x6C706D69, 0x6E656D65, 0x7374) => TokenKind::Keyword(Keyword::Implements),  // "implements"
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
