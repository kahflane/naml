///
/// TokenStream Input Type for nom
///
/// This module provides a custom input type that wraps a slice of tokens.
/// nom requires specific traits to be implemented for custom input types.
///

use std::iter::Enumerate;
use std::slice::Iter;

use nom::{InputIter, InputLength, InputTake, Needed, Slice};

use crate::lexer::Token;
use crate::source::Span;

#[derive(Debug, Clone, Copy)]
pub struct TokenStream<'a> {
    pub tokens: &'a [Token],
    pub source: &'a str,
    pub start: usize,
}

impl<'a> TokenStream<'a> {
    pub fn new(tokens: &'a [Token], source: &'a str) -> Self {
        Self { tokens, source, start: 0 }
    }

    pub fn span_text(&self, span: Span) -> &'a str {
        &self.source[span.start as usize..span.end as usize]
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    pub fn first(&self) -> Option<&'a Token> {
        self.tokens.first()
    }

    pub fn current_span(&self) -> Span {
        self.tokens.first().map(|t| t.span).unwrap_or(Span::dummy())
    }
}

impl<'a> InputLength for TokenStream<'a> {
    fn input_len(&self) -> usize {
        self.tokens.len()
    }
}

impl<'a> InputTake for TokenStream<'a> {
    fn take(&self, count: usize) -> Self {
        TokenStream {
            tokens: &self.tokens[..count],
            source: self.source,
            start: self.start,
        }
    }

    fn take_split(&self, count: usize) -> (Self, Self) {
        let (prefix, suffix) = self.tokens.split_at(count);
        (
            TokenStream {
                tokens: suffix,
                source: self.source,
                start: self.start + count,
            },
            TokenStream {
                tokens: prefix,
                source: self.source,
                start: self.start,
            },
        )
    }
}

impl<'a> InputIter for TokenStream<'a> {
    type Item = &'a Token;
    type Iter = Enumerate<Self::IterElem>;
    type IterElem = Iter<'a, Token>;

    fn iter_indices(&self) -> Self::Iter {
        self.tokens.iter().enumerate()
    }

    fn iter_elements(&self) -> Self::IterElem {
        self.tokens.iter()
    }

    fn position<P>(&self, predicate: P) -> Option<usize>
    where
        P: Fn(Self::Item) -> bool,
    {
        self.tokens.iter().position(predicate)
    }

    fn slice_index(&self, count: usize) -> Result<usize, Needed> {
        if self.tokens.len() >= count {
            Ok(count)
        } else {
            Err(Needed::new(count - self.tokens.len()))
        }
    }
}

impl<'a> Slice<std::ops::RangeFrom<usize>> for TokenStream<'a> {
    fn slice(&self, range: std::ops::RangeFrom<usize>) -> Self {
        TokenStream {
            tokens: &self.tokens[range.start..],
            source: self.source,
            start: self.start + range.start,
        }
    }
}

impl<'a> Slice<std::ops::RangeTo<usize>> for TokenStream<'a> {
    fn slice(&self, range: std::ops::RangeTo<usize>) -> Self {
        TokenStream {
            tokens: &self.tokens[..range.end],
            source: self.source,
            start: self.start,
        }
    }
}

impl<'a> Slice<std::ops::Range<usize>> for TokenStream<'a> {
    fn slice(&self, range: std::ops::Range<usize>) -> Self {
        TokenStream {
            tokens: &self.tokens[range.start..range.end],
            source: self.source,
            start: self.start + range.start,
        }
    }
}

impl<'a> Slice<std::ops::RangeFull> for TokenStream<'a> {
    fn slice(&self, _: std::ops::RangeFull) -> Self {
        *self
    }
}
