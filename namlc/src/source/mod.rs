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
/// - SourceFile: Holds source text with line/column lookup
///
/// Design decisions:
/// - Offsets are byte-based, not character-based (faster, works with UTF-8)
/// - File ID allows spans to reference different source files
/// - Spans are Copy for ergonomic use throughout the compiler
/// - SourceFile caches line starts for O(log n) offset-to-line conversion
///

use std::fmt;
use std::sync::Arc;

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

#[derive(Clone)]
pub struct SourceFile {
    pub name: Arc<str>,
    pub source: Arc<str>,
    line_starts: Vec<u32>,
}

impl SourceFile {
    pub fn new(name: impl Into<Arc<str>>, source: impl Into<Arc<str>>) -> Self {
        let source: Arc<str> = source.into();
        let line_starts = Self::compute_line_starts(&source);
        Self {
            name: name.into(),
            source,
            line_starts,
        }
    }

    fn compute_line_starts(source: &str) -> Vec<u32> {
        let mut starts = vec![0];
        for (i, c) in source.char_indices() {
            if c == '\n' {
                starts.push((i + 1) as u32);
            }
        }
        starts
    }

    pub fn line_col(&self, offset: u32) -> (usize, usize) {
        let line_idx = self.line_starts
            .binary_search(&offset)
            .unwrap_or_else(|i| i.saturating_sub(1));
        let line = line_idx + 1;
        let col = (offset - self.line_starts[line_idx]) as usize + 1;
        (line, col)
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    pub fn line_start(&self, line: usize) -> Option<u32> {
        if line == 0 || line > self.line_starts.len() {
            None
        } else {
            Some(self.line_starts[line - 1])
        }
    }

    pub fn line_text(&self, line: usize) -> Option<&str> {
        if line == 0 || line > self.line_starts.len() {
            return None;
        }
        let start = self.line_starts[line - 1] as usize;
        let end = if line < self.line_starts.len() {
            (self.line_starts[line] as usize).saturating_sub(1)
        } else {
            self.source.len()
        };
        Some(&self.source[start..end])
    }

    pub fn span_text(&self, span: Span) -> &str {
        &self.source[span.start as usize..span.end as usize]
    }
}

impl fmt::Debug for SourceFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SourceFile({}, {} lines)", self.name, self.line_count())
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

    #[test]
    fn test_source_file_line_col() {
        let source = "fn main() {\n    return 42;\n}";
        let sf = SourceFile::new("test.naml", source);

        assert_eq!(sf.line_col(0), (1, 1));
        assert_eq!(sf.line_col(3), (1, 4));
        assert_eq!(sf.line_col(12), (2, 1));
        assert_eq!(sf.line_col(16), (2, 5));
    }

    #[test]
    fn test_source_file_line_text() {
        let source = "line one\nline two\nline three";
        let sf = SourceFile::new("test.naml", source);

        assert_eq!(sf.line_text(1), Some("line one"));
        assert_eq!(sf.line_text(2), Some("line two"));
        assert_eq!(sf.line_text(3), Some("line three"));
        assert_eq!(sf.line_text(0), None);
        assert_eq!(sf.line_text(4), None);
    }

    #[test]
    fn test_source_file_span_text() {
        let source = "fn main() { return 42; }";
        let sf = SourceFile::new("test.naml", source);
        let span = Span::new(3, 7, 0);
        assert_eq!(sf.span_text(span), "main");
    }
}
