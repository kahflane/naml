//!
//! namlc - The naml Compiler Library
//!
//! This crate provides the core compiler infrastructure for the naml
//! programming language. It includes:
//!
//! - source: Source file handling, spans, and diagnostics
//! - lexer: Tokenization of naml source code
//! - ast: Abstract syntax tree definitions
//! - parser: Parsing tokens into AST
//! - typechecker: Type system and inference
//! - codegen: Cranelift JIT code generation
//! - runtime: Runtime support (arrays, strings, memory management)
//!
//! Entry points:
//! - `tokenize`: Convert source text into tokens
//! - `parse`: Parse tokens into AST
//! - `check`: Type check an AST
//! - `compile_and_run`: JIT compile and execute
//!

pub mod ast;
pub mod codegen;
pub mod diagnostic;
pub mod lexer;
pub mod linker;
pub mod parser;
pub mod runtime;
pub mod source;
pub mod typechecker;

pub use ast::{AstArena, CompilationTarget};
pub use codegen::compile_and_run;
pub use codegen::compile_to_object;
pub use diagnostic::DiagnosticReporter;
pub use lexer::tokenize;
pub use parser::parse;
pub use source::SourceFile;
pub use typechecker::{check, check_with_types, check_with_types_for_target, TypeCheckResult, ImportedModule, StdModuleFn, get_std_module_functions};
pub use typechecker::symbols::{SymbolTable, FunctionSig, MethodSig, TypeDef, StructDef, EnumDef, ModuleNamespace};

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

fn (self: point) move_by(dx: float, dy: float) {
    self.x = self.x + dx;
    self.y = self.y + dy;
}

var r: rectangle = rectangle { width: 10.0, height: 5.0 };
var c: circle = circle { radius: 3.0 };
var p: point = point { x: 3.0, y: 4.0 };

print("Rectangle area: {}", r.area());
print("Rectangle perimeter: {}", r.perimeter());
print("Circle area: {}", c.area());
print("Circle perimeter: {}", c.perimeter());
print("Point distance squared: {}", p.distance_from_origin());

p.move_by(1.0, 1.0);
print("After move - Point distance squared: {}", p.distance_from_origin());
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
