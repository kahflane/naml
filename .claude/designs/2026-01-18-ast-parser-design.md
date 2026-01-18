# AST and Parser Design Document

Date: 2026-01-18
Phase: 1 - Foundation

## Design Decisions

### 1. AST Node Memory Layout
**Choice: Box-based (not Arena-based)**
- Each nested node is heap-allocated via `Box<T>`
- Simple, idiomatic Rust, easy to implement
- Can optimize to arena-based later if profiling shows need

### 2. Expression Design
**Choice: Wrapper enum with separate structs**
```rust
enum Expression {
    Binary(BinaryExpr),
    Call(CallExpr),
}
struct BinaryExpr { left: Box<Expression>, op: BinaryOp, right: Box<Expression>, span: Span }
```
- Each node type gets its own `Span` naturally
- Cleaner when adding type annotations later

### 3. Span Attachment Strategy
**Choice: Span field in each struct**
- Every struct has `span: Span` field
- Matches existing `Token` pattern in lexer
- Implement `Spanned` trait for uniform access

### 4. Identifier Representation
**Choice: Ident struct everywhere**
```rust
struct Ident { symbol: Spur, span: Span }
```
- Name always carries its source location
- Better error messages ("undefined variable `foo` at line X")

### 5. Type System
**No `Any` type** - Strong typing only
- All types must be known at compile time
- No dynamic typing escape hatch

### 6. Parser Library
**Choice: nom**
- Battle-tested parser combinators (most popular in Rust ecosystem)
- Zero-copy by design, fast compile times
- Integrates naturally with existing lexer via token stream
- No separate build step or grammar files
- Manual Pratt parser implementation for operator precedence

### 7. Entry Point
**Programs start with `main()` function**
- Parser looks for `main()` as entry point
- Similar to Rust/Go/C conventions

---

## AST Module Structure

```
namlc/src/ast/
├── mod.rs          # Module root, SourceFile struct
├── types.rs        # NamlType enum
├── literals.rs     # Literal enum
├── operators.rs    # BinaryOp, UnaryOp, AssignOp
├── expressions.rs  # Expression enum + all expr structs
├── statements.rs   # Statement enum + all stmt structs
├── items.rs        # Item enum + top-level definitions
└── visitor.rs      # Visitor trait for traversal
```

---

## Type System (`ast/types.rs`)

```rust
pub struct Ident { pub symbol: Spur, pub span: Span }

pub enum NamlType {
    // Primitives
    Int, Uint, Float, Bool, String, Bytes, Unit,
    Decimal { precision: u8, scale: u8 },

    // Composite
    Array(Box<NamlType>),              // [T]
    FixedArray(Box<NamlType>, usize),  // [T; N]
    Option(Box<NamlType>),             // option<T>
    Map(Box<NamlType>, Box<NamlType>), // map<K, V>
    Channel(Box<NamlType>),            // channel<T>
    Promise(Box<NamlType>),            // promise<T>

    // User-defined references
    Named(Ident),                      // struct/enum/interface name
    Generic(Ident, Vec<NamlType>),     // Name<T, U>

    // Function type
    Function { params: Vec<NamlType>, returns: Box<NamlType> },

    // Inference placeholder
    Inferred,
}
```

---

## Literals and Operators

### Literals (`ast/literals.rs`)
```rust
pub enum Literal {
    Int(i64),
    UInt(u64),
    Float(f64),
    Bool(bool),
    String(Spur),      // Interned string content
    Bytes(Vec<u8>),    // Raw byte data
}
```
Note: No `Nil` - use `option<T>` with `None` instead.

### Operators (`ast/operators.rs`)
```rust
pub enum BinaryOp {
    // Arithmetic
    Add, Sub, Mul, Div, Mod,
    // Comparison
    Eq, NotEq, Lt, LtEq, Gt, GtEq,
    // Logical
    And, Or,
    // Bitwise
    BitAnd, BitOr, BitXor, Shl, Shr,
    // Range
    Range, RangeIncl,
    // Type check
    Is,
}

pub enum UnaryOp {
    Neg,    // -x
    Not,    // not x / !x
    BitNot, // ~x
}

pub enum AssignOp {
    Assign,
    AddAssign, SubAssign, MulAssign, DivAssign, ModAssign,
    BitAndAssign, BitOrAssign, BitXorAssign,
}
```

---

## Expressions (`ast/expressions.rs`)

```rust
pub enum Expression {
    Literal(LiteralExpr),
    Identifier(IdentExpr),
    Binary(BinaryExpr),
    Unary(UnaryExpr),
    Call(CallExpr),
    MethodCall(MethodCallExpr),
    Index(IndexExpr),
    Field(FieldExpr),
    Array(ArrayExpr),
    Map(MapExpr),
    If(IfExpr),
    Block(BlockExpr),
    Lambda(LambdaExpr),
    Spawn(SpawnExpr),
    Await(AwaitExpr),
    Try(TryExpr),
    Cast(CastExpr),
    Range(RangeExpr),
    Grouped(GroupedExpr),
}

// Each has span field
pub struct BinaryExpr { pub left: Box<Expression>, pub op: BinaryOp, pub right: Box<Expression>, pub span: Span }
pub struct CallExpr { pub callee: Box<Expression>, pub args: Vec<Expression>, pub span: Span }
// ... etc
```

---

## Statements (`ast/statements.rs`)

```rust
pub enum Statement {
    Var(VarStmt),
    Const(ConstStmt),
    Assign(AssignStmt),
    Expression(ExprStmt),
    Return(ReturnStmt),
    Throw(ThrowStmt),
    If(IfStmt),
    While(WhileStmt),
    For(ForStmt),
    Loop(LoopStmt),
    Switch(SwitchStmt),
    Break(BreakStmt),
    Continue(ContinueStmt),
    Block(BlockStmt),
}

pub struct VarStmt {
    pub name: Ident,
    pub mutable: bool,           // var vs var mut
    pub ty: Option<NamlType>,
    pub init: Option<Expression>,
    pub span: Span
}

pub struct ForStmt {
    pub index: Option<Ident>,    // i in `for (i, val in ...)`
    pub value: Ident,
    pub ty: Option<NamlType>,
    pub iterable: Expression,
    pub body: BlockStmt,
    pub span: Span
}

pub enum ElseBranch {
    ElseIf(Box<IfStmt>),
    Else(BlockStmt),
}
// ... etc
```

---

## Items (`ast/items.rs`)

```rust
pub enum Item {
    Function(FunctionItem),
    Struct(StructItem),
    Interface(InterfaceItem),
    Enum(EnumItem),
    Exception(ExceptionItem),
    Import(ImportItem),
    Use(UseItem),
    Extern(ExternItem),
}

pub struct FunctionItem {
    pub name: Ident,
    pub receiver: Option<Receiver>,      // (self: Type) for methods
    pub generics: Vec<GenericParam>,
    pub params: Vec<Parameter>,
    pub return_ty: Option<NamlType>,
    pub throws: Option<NamlType>,
    pub is_async: bool,
    pub is_public: bool,
    pub body: Option<BlockStmt>,
    pub platforms: Option<Platforms>,
    pub span: Span,
}

pub struct StructItem {
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub implements: Vec<NamlType>,
    pub fields: Vec<StructField>,
    pub is_public: bool,
    pub span: Span,
}

pub enum Platform {
    Native,
    Server,
    Browser,
    All,
}
// ... etc
```

---

## Parser Design (using nom)

### Dependencies
```toml
nom = "7"
```

### Structure
```
namlc/src/parser/
├── mod.rs          # Entry point, token stream type, parse()
├── tokens.rs       # Token matching helpers
├── types.rs        # Type annotation parsing
├── expressions.rs  # Expression combinators with pratt parsing
├── statements.rs   # Statement combinators
├── items.rs        # Top-level item combinators
└── error.rs        # ParseError types with span info
```

### Token Stream Type
```rust
use nom::{IResult, InputTake, InputLength};

/// Wrapper around token slice for nom compatibility
#[derive(Clone, Copy)]
pub struct TokenStream<'a> {
    pub tokens: &'a [Token],
    pub start: usize,
}

impl<'a> InputLength for TokenStream<'a> {
    fn input_len(&self) -> usize { self.tokens.len() }
}

impl<'a> InputTake for TokenStream<'a> {
    fn take(&self, count: usize) -> Self {
        TokenStream { tokens: &self.tokens[..count], start: self.start }
    }
    fn take_split(&self, count: usize) -> (Self, Self) {
        let (prefix, suffix) = self.tokens.split_at(count);
        (TokenStream { tokens: suffix, start: self.start + count },
         TokenStream { tokens: prefix, start: self.start })
    }
}

pub type ParseResult<'a, O> = IResult<TokenStream<'a>, O, ParseError>;
```

### Token Matching Helpers
```rust
use nom::combinator::verify;
use nom::bytes::complete::take;

pub fn token(kind: TokenKind) -> impl Fn(TokenStream) -> ParseResult<Token> {
    move |input| {
        let (rest, taken) = take(1usize)(input)?;
        if taken.tokens[0].kind == kind {
            Ok((rest, taken.tokens[0].clone()))
        } else {
            Err(nom::Err::Error(ParseError::expected(kind, taken.tokens[0].span)))
        }
    }
}

pub fn keyword(kw: Keyword) -> impl Fn(TokenStream) -> ParseResult<Token> {
    token(TokenKind::Keyword(kw))
}

pub fn ident(input: TokenStream) -> ParseResult<Ident> {
    let (rest, taken) = take(1usize)(input)?;
    match &taken.tokens[0].kind {
        TokenKind::Ident => Ok((rest, Ident {
            symbol: taken.tokens[0].symbol.unwrap(),
            span: taken.tokens[0].span,
        })),
        _ => Err(nom::Err::Error(ParseError::expected_ident(taken.tokens[0].span))),
    }
}
```

### Expression Parsing (Manual Pratt Parser)
```rust
use nom::branch::alt;
use nom::sequence::{preceded, delimited};
use nom::multi::many0;

fn precedence(op: &BinaryOp) -> u8 {
    match op {
        BinaryOp::Or => 1,
        BinaryOp::And => 2,
        BinaryOp::Eq | BinaryOp::NotEq => 3,
        BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => 4,
        BinaryOp::BitOr => 5,
        BinaryOp::BitXor => 6,
        BinaryOp::BitAnd => 7,
        BinaryOp::Shl | BinaryOp::Shr => 8,
        BinaryOp::Add | BinaryOp::Sub => 9,
        BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => 10,
        _ => 0,
    }
}

fn atom(input: TokenStream) -> ParseResult<Expression> {
    alt((literal, ident_expr, grouped, array_expr, if_expr, spawn_expr))(input)
}

fn postfix(input: TokenStream) -> ParseResult<Expression> {
    let (input, mut expr) = atom(input)?;
    let (input, ops) = many0(alt((call_args, index_bracket, field_access)))(input)?;
    for op in ops { expr = apply_postfix(expr, op); }
    Ok((input, expr))
}

fn binary_op(input: TokenStream) -> ParseResult<BinaryOp> {
    alt((
        map(token(TokenKind::Plus), |_| BinaryOp::Add),
        map(token(TokenKind::Minus), |_| BinaryOp::Sub),
        map(token(TokenKind::Star), |_| BinaryOp::Mul),
        map(token(TokenKind::Slash), |_| BinaryOp::Div),
        map(token(TokenKind::EqEq), |_| BinaryOp::Eq),
        map(token(TokenKind::NotEq), |_| BinaryOp::NotEq),
        // ... other operators
    ))(input)
}

pub fn expression(input: TokenStream) -> ParseResult<Expression> {
    pratt_expr(input, 0)
}

fn pratt_expr(input: TokenStream, min_prec: u8) -> ParseResult<Expression> {
    let (mut input, mut left) = postfix(input)?;

    loop {
        let Ok((next_input, op)) = binary_op(input.clone()) else { break };
        let prec = precedence(&op);
        if prec < min_prec { break }

        input = next_input;
        let (next_input, right) = pratt_expr(input, prec + 1)?;
        input = next_input;

        let span = left.span().merge(right.span());
        left = Expression::Binary(BinaryExpr {
            left: Box::new(left), op, right: Box::new(right), span
        });
    }
    Ok((input, left))
}
```

### Statement Parsing
```rust
use nom::branch::alt;

pub fn statement(input: TokenStream) -> ParseResult<Statement> {
    alt((
        var_stmt,
        const_stmt,
        return_stmt,
        throw_stmt,
        if_stmt,
        while_stmt,
        for_stmt,
        switch_stmt,
        break_stmt,
        continue_stmt,
        expr_stmt,
    ))(input)
}

fn var_stmt(input: TokenStream) -> ParseResult<Statement> {
    let (input, _) = keyword(Keyword::Var)(input)?;
    let (input, mutable) = opt(keyword(Keyword::Mut))(input)?;
    let (input, name) = ident(input)?;
    let (input, ty) = opt(preceded(token(TokenKind::Colon), type_annotation))(input)?;
    let (input, init) = opt(preceded(token(TokenKind::Eq), expression))(input)?;
    let (input, _) = token(TokenKind::Semicolon)(input)?;
    Ok((input, Statement::Var(VarStmt {
        name, mutable: mutable.is_some(), ty, init, span: name.span,
    })))
}
```

---

## Visitor Pattern (`ast/visitor.rs`)

```rust
pub trait Visitor {
    fn visit_item(&mut self, item: &Item) { walk_item(self, item) }
    fn visit_stmt(&mut self, stmt: &Statement) { walk_stmt(self, stmt) }
    fn visit_expr(&mut self, expr: &Expression) { walk_expr(self, expr) }
    fn visit_type(&mut self, ty: &NamlType) { walk_type(self, ty) }
    fn visit_ident(&mut self, ident: &Ident) {}
}

pub fn walk_item<V: Visitor>(v: &mut V, item: &Item) { ... }
pub fn walk_stmt<V: Visitor>(v: &mut V, stmt: &Statement) { ... }
pub fn walk_expr<V: Visitor>(v: &mut V, expr: &Expression) { ... }
pub fn walk_type<V: Visitor>(v: &mut V, ty: &NamlType) { ... }
```

---

## Root AST Node

```rust
/// Root of a parsed naml file
pub struct SourceFile {
    pub items: Vec<Item>,
    pub span: Span,
}
```

---

## Implementation Order

1. `ast/types.rs` - Ident and NamlType
2. `ast/literals.rs` - Literal enum
3. `ast/operators.rs` - BinaryOp, UnaryOp, AssignOp
4. `ast/expressions.rs` - All expression structs and enum
5. `ast/statements.rs` - All statement structs and enum
6. `ast/items.rs` - All item structs and enum
7. `ast/visitor.rs` - Visitor trait
8. `ast/mod.rs` - Re-exports and SourceFile
9. `parser/mod.rs` - Entry point and helpers
10. `parser/types.rs` - Type parsing
11. `parser/expressions.rs` - Expression parsing
12. `parser/statements.rs` - Statement parsing
13. `parser/items.rs` - Item parsing

---

## Code Rules Reminder

- All files under 1000 lines
- Block comments only at file top
- Zero-copy where possible (use Spur for strings)
- Implement Spanned trait for all AST nodes
- Comprehensive tests in each module
