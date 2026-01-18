///
/// Source Location and Span Module
///
/// This module provides types for tracking source code locations throughout
/// the compilation pipeline. Every AST node carries a Span indicating where
/// it came from in the original source text.
///
/// Key types:
/// - Span: A range in source code (start offset, end offset, file id)
/// - Spanned: Trait for types that have an associated span
///
/// Design decisions:
/// - Offsets are byte-based, not character-based (faster, works with UTF-8)
/// - File ID allows spans to reference different source files
/// - Spans are Copy for ergonomic use throughout the compiler
///

use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub start: u32,
    pub end: u32,
    pub file_id: u32,
}

impl Span {
    pub const fn new(start: u32, end: u32, file_id: u32) -> Self {
        Self { start, end, file_id }
    }

    pub const fn dummy() -> Self {
        Self { start: 0, end: 0, file_id: 0 }
    }

    pub fn merge(self, other: Span) -> Span {
        debug_assert_eq!(self.file_id, other.file_id, "Cannot merge spans from different files");
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            file_id: self.file_id,
        }
    }

    pub fn len(&self) -> u32 {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn contains(&self, offset: u32) -> bool {
        offset >= self.start && offset < self.end
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}@{}", self.start, self.end, self.file_id)
    }
}

pub trait Spanned {
    fn span(&self) -> Span;
}

impl Spanned for Span {
    fn span(&self) -> Span {
        *self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_merge() {
        let a = Span::new(10, 20, 0);
        let b = Span::new(15, 30, 0);
        let merged = a.merge(b);
        assert_eq!(merged.start, 10);
        assert_eq!(merged.end, 30);
    }

    #[test]
    fn test_span_contains() {
        let span = Span::new(10, 20, 0);
        assert!(span.contains(10));
        assert!(span.contains(15));
        assert!(!span.contains(20));
        assert!(!span.contains(5));
    }
}
