//!
//! AST Arena Allocator
//!
//! This module provides bump allocation for AST nodes, significantly reducing
//! allocation overhead during parsing. Instead of individual Box<T> allocations
//! for each expression node, all AST nodes are allocated from a single arena.
//!
//! Key benefits:
//! - Reduced allocator pressure (single large allocation vs many small ones)
//! - Better cache locality (nodes allocated sequentially in memory)
//! - Faster deallocation (drop entire arena at once)
//! - No individual Box overhead per node
//!
//! Usage:
//! ```ignore
//! let arena = AstArena::new();
//! let left = arena.alloc(expr1);
//! let right = arena.alloc(expr2);
//! // left and right are &'arena Expression references
//! ```
//!

use bumpalo::Bump;

pub struct AstArena {
    bump: Bump,
}

impl AstArena {
    pub fn new() -> Self {
        Self { bump: Bump::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bump: Bump::with_capacity(capacity),
        }
    }

    #[inline]
    pub fn alloc<T>(&self, val: T) -> &T {
        self.bump.alloc(val)
    }

    #[inline]
    pub fn alloc_slice_copy<T: Copy>(&self, vals: &[T]) -> &[T] {
        self.bump.alloc_slice_copy(vals)
    }

    #[inline]
    pub fn alloc_slice_clone<T: Clone>(&self, vals: &[T]) -> &[T] {
        self.bump.alloc_slice_clone(vals)
    }

    pub fn allocated_bytes(&self) -> usize {
        self.bump.allocated_bytes()
    }

    pub fn reset(&mut self) {
        self.bump.reset();
    }
}

impl Default for AstArena {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_alloc() {
        let arena = AstArena::new();
        let val1 = arena.alloc(42i32);
        let val2 = arena.alloc(100i32);
        assert_eq!(*val1, 42);
        assert_eq!(*val2, 100);
    }

    #[test]
    fn test_arena_alloc_slice() {
        let arena = AstArena::new();
        let original = [1, 2, 3, 4, 5];
        let slice = arena.alloc_slice_copy(&original);
        assert_eq!(slice, &original);
    }

    #[test]
    fn test_arena_with_capacity() {
        let arena = AstArena::with_capacity(1024);
        let _ = arena.alloc(42i32);
        assert!(arena.allocated_bytes() >= 4);
    }

    #[test]
    fn test_arena_reset() {
        let mut arena = AstArena::new();
        for i in 0..100 {
            let _ = arena.alloc(i);
        }
        let before = arena.allocated_bytes();
        arena.reset();
        assert!(before > 0);
    }
}
