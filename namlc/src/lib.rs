///
/// namlc - The naml Compiler Library
///
/// This crate provides the core compiler infrastructure for the naml
/// programming language. It includes:
///
/// - source: Source file handling, spans, and diagnostics
/// - lexer: Tokenization of naml source code
/// - ast: Abstract syntax tree definitions
/// - parser: Parsing tokens into AST
/// - typechecker: Type system and inference
/// - codegen: Rust code generation (transpilation)
///
/// Entry points:
/// - `tokenize`: Convert source text into tokens
/// - `parse`: Parse tokens into AST
/// - `check`: Type check an AST
/// - `codegen`: Generate Rust code from AST
///

pub mod ast;
pub mod codegen;
pub mod diagnostic;
pub mod lexer;
pub mod parser;
pub mod source;
pub mod typechecker;

pub use ast::AstArena;
pub use codegen::compile_and_run;
pub use diagnostic::DiagnosticReporter;
pub use lexer::tokenize;
pub use parser::parse;
pub use source::SourceFile;
pub use typechecker::check;

#[test]
fn test_parse_shape_example() {
    let source = r#"
interface shape {
    fn area() -> float;
    fn perimeter() -> float;
}

pub struct rectangle implements shape {
    pub width: float,
    pub height: float
}

pub struct circle implements shape {
    pub radius: float
}

pub fn (self: rectangle) area() -> float {
    return self.width * self.height;
}

pub fn (self: rectangle) perimeter() -> float {
    return 2.0 * (self.width + self.height);
}

pub fn (self: circle) area() -> float {
    return 3.14159 * self.radius * self.radius;
}

pub fn (self: circle) perimeter() -> float {
    return 2.0 * 3.14159 * self.radius;
}

pub struct point {
    pub x: float,
    pub y: float
}

pub fn (self: point) distance_from_origin() -> float {
    return (self.x * self.x + self.y * self.y);
}

fn (mut self: point) move_by(dx: float, dy: float) {
    self.x = self.x + dx;
    self.y = self.y + dy;
}

var r: rectangle = rectangle { width: 10.0, height: 5.0 };
var c: circle = circle { radius: 3.0 };
var mut p: point = point { x: 3.0, y: 4.0 };

printf("Rectangle area: {}", r.area());
printf("Rectangle perimeter: {}", r.perimeter());
printf("Circle area: {}", c.area());
printf("Circle perimeter: {}", c.perimeter());
printf("Point distance squared: {}", p.distance_from_origin());

p.move_by(1.0, 1.0);
printf("After move - Point distance squared: {}", p.distance_from_origin());
"#;

    let (tokens, _interner) = tokenize(source);
    let arena = AstArena::new();
    let result = parse(&tokens, source, &arena);
    if result.errors.is_empty() {
        println!("Parsed {} items successfully", result.ast.items.len());
        assert!(result.ast.items.len() > 10, "Expected at least 10 items");
    } else {
        panic!("Parse errors: {:?}", result.errors);
    }
}
