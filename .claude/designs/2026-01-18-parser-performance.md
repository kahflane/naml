Parser Performance Optimization

Goal

Improve parser throughput from ~5.5 MB/s to 100+ MB/s (competitive with Bun/swc).

Current Performance

Total time: 4.68ms
Throughput: 5.51 MB/s

Root Cause Analysis

Critical Bottleneck: Redundant Trivia Skipping

skip_trivia() is called thousands of times per file, each doing a linear scan:

// Called 58+ times in parser code, plus inside loops
pub fn skip_trivia(input: TokenStream) -> TokenStream {
let mut i = 0;
while i < input.tokens.len() && input.tokens[i].is_trivia() {
i += 1;  // Linear scan on every call
}
// ...
}

Example waste: In parse_atom() with 14 alt() branches:
- Each failing branch calls skip_trivia
- For parsing {, trivia is scanned 14 times before reaching parse_block_or_map

Secondary Issues

1. Token cloning on every match: tok.clone()
2. No vector pre-sizing: 22+ Vec::new() without capacity
3. Excessive backtracking: alt() tries all branches sequentially

Implementation Plan

Phase 1: Pre-filter Trivia Tokens (HIGH IMPACT)

The fix: Remove trivia tokens during lexing, not during parsing.

File: namlc/src/lexer/mod.rs

// Before: returns all tokens including trivia
pub fn tokenize(source: &str) -> (Vec<Token>, Rodeo)

// After: returns only significant tokens (no whitespace/comments)
pub fn tokenize(source: &str) -> (Vec<Token>, Rodeo)
// Trivia filtered out at source - parser never sees it

Implementation:
1. Filter trivia at the end of tokenize():
   let tokens: Vec<Token> = tokens
   .into_iter()
   .filter(|t| !t.is_trivia())
   .collect();
2. Remove ALL skip_trivia() calls from parser
3. Remove skip_trivia function from combinators.rs

Expected impact: 3-5x speedup (eliminates ~10,000 redundant scans per file)

Phase 2: Lookahead-Based Dispatch (MEDIUM IMPACT)

File: namlc/src/parser/expressions.rs

Replace sequential alt() with token-based dispatch:

// Before: 14 sequential attempts
alt((parse_int_literal, parse_float_literal, ...))

// After: direct dispatch based on first token
fn parse_atom(input: TokenStream) -> PResult<Expression> {
match input.first().map(|t| &t.kind) {
Some(TokenKind::IntLit) => parse_int_literal(input),
Some(TokenKind::FloatLit) => parse_float_literal(input),
Some(TokenKind::StringLit) => parse_string_literal(input),
Some(TokenKind::Keyword(Keyword::True | Keyword::False)) => parse_bool_literal(input),
Some(TokenKind::Keyword(Keyword::None)) => parse_none_literal(input),
Some(TokenKind::Keyword(Keyword::Some)) => parse_some_expr(input),
Some(TokenKind::Ident) => parse_ident_or_struct(input),
Some(TokenKind::LParen) => parse_grouped(input),
Some(TokenKind::LBracket) => parse_array_expr(input),
Some(TokenKind::LBrace) => parse_block_or_map(input),
Some(TokenKind::Keyword(Keyword::If)) => parse_if_expr(input),
Some(TokenKind::Keyword(Keyword::Spawn)) => parse_spawn_expr(input),
Some(TokenKind::Keyword(Keyword::Await)) => parse_await_expr(input),
Some(TokenKind::Keyword(Keyword::Try)) => parse_try_expr(input),
Some(TokenKind::Pipe | TokenKind::PipePipe) => parse_lambda_expr(input),
_ => Err(nom::Err::Error(PError { ... })),
}
}

Apply same pattern to:
- parse_statement() in statements.rs
- parse_type() in types.rs
- parse_item() in items.rs

Expected impact: 1.5-2x speedup (eliminates backtracking)

Phase 3: Return References Instead of Clones (MEDIUM IMPACT)

File: namlc/src/parser/combinators.rs

// Before: clones token
pub fn token(kind: TokenKind) -> impl Fn(TokenStream) -> PResult<Token> {
// ...
Ok((rest, tok.clone()))  // CLONE
}

// After: return span + kind (no clone needed)
pub fn token(kind: TokenKind) -> impl Fn(TokenStream) -> PResult<(Span, TokenKind)> {
// ...
Ok((rest, (tok.span, tok.kind)))  // Copy, no clone
}

Or: Make Token derive Copy by changing literal: Option<Spur> handling.

Expected impact: 1.2-1.5x speedup

Phase 4: Pre-size Vectors (LOW IMPACT)

Files: expressions.rs, statements.rs, items.rs

// Before
let mut fields = Vec::new();

// After: estimate typical sizes
let mut fields = Vec::with_capacity(8);  // Most structs have <8 fields
let mut statements = Vec::with_capacity(16);  // Most blocks have <16 statements
let mut params = Vec::with_capacity(4);  // Most functions have <4 params

Expected impact: 1.1x speedup

Files to Modify
┌─────────────────────────────────┬──────────────────────────────────────────────────────────────────┐
│              File               │                             Changes                              │
├─────────────────────────────────┼──────────────────────────────────────────────────────────────────┤
│ namlc/src/lexer/mod.rs          │ Filter trivia tokens before returning                            │
├─────────────────────────────────┼──────────────────────────────────────────────────────────────────┤
│ namlc/src/parser/combinators.rs │ Remove skip_trivia, return references                            │
├─────────────────────────────────┼──────────────────────────────────────────────────────────────────┤
│ namlc/src/parser/expressions.rs │ Match-based dispatch, remove skip_trivia calls, pre-size vectors │
├─────────────────────────────────┼──────────────────────────────────────────────────────────────────┤
│ namlc/src/parser/statements.rs  │ Match-based dispatch, remove skip_trivia calls                   │
├─────────────────────────────────┼──────────────────────────────────────────────────────────────────┤
│ namlc/src/parser/types.rs       │ Match-based dispatch, remove skip_trivia calls                   │
├─────────────────────────────────┼──────────────────────────────────────────────────────────────────┤
│ namlc/src/parser/items.rs       │ Match-based dispatch, remove skip_trivia calls                   │
└─────────────────────────────────┴──────────────────────────────────────────────────────────────────┘
Verification

1. Run existing tests: cargo test --lib
2. Run benchmark: cargo run --example test_parse --release
3. Target metrics:
- Phase 1 alone: ~20-30 MB/s
- All phases: ~100+ MB/s
- Compare with: hyperfine 'cargo run --example test_parse --release'

Performance Comparison Targets
┌───────────────────┬────────────┬────────────────────────┐
│      Parser       │ Throughput │         Notes          │
├───────────────────┼────────────┼────────────────────────┤
│ naml (current)    │ 5.5 MB/s   │ Trivia overhead        │
├───────────────────┼────────────┼────────────────────────┤
│ naml (Phase 1)    │ ~25 MB/s   │ Trivia filtered        │
├───────────────────┼────────────┼────────────────────────┤
│ naml (all phases) │ ~100 MB/s  │ Optimized              │
├───────────────────┼────────────┼────────────────────────┤
│ swc               │ ~200 MB/s  │ Rust, highly optimized │
├───────────────────┼────────────┼────────────────────────┤
│ Bun               │ ~300 MB/s  │ Zig, SIMD              │
└───────────────────┴────────────┴────────────────────────┘
Future Optimizations (Out of Scope)

- SIMD-accelerated lexer (like simdjson)
- Arena allocation for AST nodes
- Parallel parsing of independent files
- Incremental parsing for IDE support