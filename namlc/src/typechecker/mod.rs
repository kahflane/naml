//!
//! Type Checker Module
//!
//! This module provides type checking for naml programs. The type checker:
//!
//! 1. Collects all type and function definitions (first pass)
//! 2. Validates type definitions and builds the symbol table
//! 3. Type checks all function bodies and expressions
//! 4. Reports type errors with source locations
//!
//! The type checker uses Hindley-Milner style type inference with
//! unification. Type variables are created for unknown types and bound
//! during inference.
//!
//! Entry point: `check()` function takes an AST and returns errors
//!

pub mod env;
pub mod error;
pub mod generics;
pub mod infer;
pub mod symbols;
pub mod typed_ast;
pub mod types;
pub mod unify;

use lasso::Rodeo;

use crate::ast::{self, Item, SourceFile};

pub use error::{TypeError, TypeResult};
pub use symbols::SymbolTable;
pub use typed_ast::TypeAnnotations;
pub use types::Type;

pub struct TypeCheckResult {
    pub errors: Vec<TypeError>,
    pub annotations: TypeAnnotations,
    pub symbols: SymbolTable,
}

use env::TypeEnv;
use infer::TypeInferrer;
use symbols::{
    EnumDef, ExceptionDef, FunctionSig, InterfaceDef, InterfaceMethodDef, MethodSig, StructDef,
    TypeDef,
};
use types::TypeParam;

pub struct TypeChecker<'a> {
    symbols: SymbolTable,
    env: TypeEnv,
    interner: &'a Rodeo,
    errors: Vec<TypeError>,
    annotations: TypeAnnotations,
    next_var_id: u32,
}

impl<'a> TypeChecker<'a> {
    pub fn new(interner: &'a Rodeo) -> Self {
        let mut checker = Self {
            symbols: SymbolTable::new(),
            env: TypeEnv::new(),
            interner,
            errors: Vec::new(),
            annotations: TypeAnnotations::new(),
            next_var_id: 0,
        };
        checker.register_builtins();
        checker
    }

    fn register_builtins(&mut self) {
        use crate::source::Span;

        let builtins: Vec<(&str, bool, Type)> = vec![
            ("print", true, Type::Unit),
            ("println", true, Type::Unit),
            ("printf", true, Type::Unit),
            ("read_line", false, Type::String),
            // Concurrency builtins (no params)
            ("wait_all", false, Type::Unit),
        ];

        for (name, is_variadic, return_ty) in builtins {
            if let Some(spur) = self.interner.get(name) {
                self.symbols.define_function(FunctionSig {
                    name: spur,
                    type_params: vec![],
                    params: vec![],
                    return_ty,
                    throws: None,
                    is_public: true,
                    is_variadic,
                    span: Span::dummy(),
                });
            }
        }

        // Register sleep(ms: int) -> Unit
        if let Some(spur) = self.interner.get("sleep") {
            self.symbols.define_function(FunctionSig {
                name: spur,
                type_params: vec![],
                params: vec![(spur, Type::Int)], // ms parameter
                return_ty: Type::Unit,
                throws: None,
                is_public: true,
                is_variadic: false,
                span: Span::dummy(),
            });
        }

        // Register make_channel as a generic function: make_channel<T>(capacity: int) -> channel<T>
        if let Some(spur) = self.interner.get("make_channel") {
            // Create a type parameter T
            let t_spur = self.interner.get("T").unwrap_or(spur); // Use T if interned, fallback to name
            let type_param = TypeParam {
                name: t_spur,
                bounds: vec![],
            };

            self.symbols.define_function(FunctionSig {
                name: spur,
                type_params: vec![type_param.clone()],
                params: vec![(spur, Type::Int)], // capacity parameter
                return_ty: Type::Channel(Box::new(Type::Generic(t_spur, vec![]))),
                throws: None,
                is_public: true,
                is_variadic: false,
                span: Span::dummy(),
            });
        }
    }

    pub fn check(&mut self, file: &SourceFile) -> Vec<TypeError> {
        self.collect_definitions(file);
        self.check_items(file);
        std::mem::take(&mut self.errors)
    }

    fn collect_definitions(&mut self, file: &SourceFile) {
        for item in &file.items {
            match item {
                Item::Function(func) => {
                    if func.receiver.is_some() {
                        self.collect_method(func);
                    } else {
                        self.collect_function(func);
                    }
                }
                Item::Struct(s) => self.collect_struct(s),
                Item::Enum(e) => self.collect_enum(e),
                Item::Interface(i) => self.collect_interface(i),
                Item::Exception(e) => self.collect_exception(e),
                Item::Extern(e) => self.collect_extern(e),
                Item::Import(_) | Item::Use(_) | Item::TopLevelStmt(_) => {}
            }
        }
    }

    fn collect_function(&mut self, func: &ast::FunctionItem) {
        let type_params = func
            .generics
            .iter()
            .map(|g| TypeParam {
                name: g.name.symbol,
                bounds: g.bounds.iter().map(|b| self.convert_type(b)).collect(),
            })
            .collect();

        let params = func
            .params
            .iter()
            .map(|p| (p.name.symbol, self.convert_type(&p.ty)))
            .collect();

        let return_ty = func
            .return_ty
            .as_ref()
            .map(|t| self.convert_type(t))
            .unwrap_or(Type::Unit);

        let throws = func.throws.as_ref().map(|t| self.convert_type(t));

        self.symbols.define_function(FunctionSig {
            name: func.name.symbol,
            type_params,
            params,
            return_ty,
            throws,
            is_public: func.is_public,
            is_variadic: false,
            span: func.span,
        });
    }

    fn collect_extern(&mut self, ext: &ast::ExternItem) {
        let params = ext
            .params
            .iter()
            .map(|p| (p.name.symbol, self.convert_type(&p.ty)))
            .collect();

        let return_ty = ext
            .return_ty
            .as_ref()
            .map(|t| self.convert_type(t))
            .unwrap_or(Type::Unit);

        let throws = ext.throws.as_ref().map(|t| self.convert_type(t));

        self.symbols.define_function(FunctionSig {
            name: ext.name.symbol,
            type_params: Vec::new(),
            params,
            return_ty,
            throws,
            is_public: true,
            is_variadic: false,
            span: ext.span,
        });
    }

    fn collect_method(&mut self, func: &ast::FunctionItem) {
        let recv = func.receiver.as_ref().unwrap();
        let receiver_ty = self.convert_type(&recv.ty);

        let type_name = match &receiver_ty {
            Type::Generic(name, _) => *name,
            Type::Struct(s) => s.name,
            _ => return,
        };

        let type_params = func
            .generics
            .iter()
            .map(|g| TypeParam {
                name: g.name.symbol,
                bounds: g.bounds.iter().map(|b| self.convert_type(b)).collect(),
            })
            .collect();

        let params = func
            .params
            .iter()
            .map(|p| (p.name.symbol, self.convert_type(&p.ty)))
            .collect();

        let return_ty = func
            .return_ty
            .as_ref()
            .map(|t| self.convert_type(t))
            .unwrap_or(Type::Unit);

        let throws = func.throws.as_ref().map(|t| self.convert_type(t));

        self.symbols.define_method(
            type_name,
            MethodSig {
                name: func.name.symbol,
                receiver_ty,
                receiver_mutable: recv.mutable,
                type_params,
                params,
                return_ty,
                throws,
                is_public: func.is_public,
                span: func.span,
            },
        );
    }

    fn collect_struct(&mut self, s: &ast::StructItem) {
        let type_params = s
            .generics
            .iter()
            .map(|g| TypeParam {
                name: g.name.symbol,
                bounds: g.bounds.iter().map(|b| self.convert_type(b)).collect(),
            })
            .collect();

        let fields = s
            .fields
            .iter()
            .map(|f| (f.name.symbol, self.convert_type(&f.ty), f.is_public))
            .collect();

        let implements = s.implements.iter().map(|t| self.convert_type(t)).collect();

        self.symbols.define_type(
            s.name.symbol,
            TypeDef::Struct(StructDef {
                name: s.name.symbol,
                type_params,
                fields,
                implements,
                is_public: s.is_public,
                span: s.span,
            }),
        );
    }

    fn collect_enum(&mut self, e: &ast::EnumItem) {
        let type_params = e
            .generics
            .iter()
            .map(|g| TypeParam {
                name: g.name.symbol,
                bounds: g.bounds.iter().map(|b| self.convert_type(b)).collect(),
            })
            .collect();

        let variants = e
            .variants
            .iter()
            .map(|v| {
                let fields = v
                    .fields
                    .as_ref()
                    .map(|fs| fs.iter().map(|t| self.convert_type(t)).collect());
                (v.name.symbol, fields)
            })
            .collect();

        self.symbols.define_type(
            e.name.symbol,
            TypeDef::Enum(EnumDef {
                name: e.name.symbol,
                type_params,
                variants,
                is_public: e.is_public,
                span: e.span,
            }),
        );
    }

    fn collect_interface(&mut self, i: &ast::InterfaceItem) {
        let type_params = i
            .generics
            .iter()
            .map(|g| TypeParam {
                name: g.name.symbol,
                bounds: g.bounds.iter().map(|b| self.convert_type(b)).collect(),
            })
            .collect();

        let extends = i.extends.iter().map(|t| self.convert_type(t)).collect();

        let methods = i
            .methods
            .iter()
            .map(|m| {
                let method_type_params = m
                    .generics
                    .iter()
                    .map(|g| TypeParam {
                        name: g.name.symbol,
                        bounds: g.bounds.iter().map(|b| self.convert_type(b)).collect(),
                    })
                    .collect();
                let params = m
                    .params
                    .iter()
                    .map(|p| (p.name.symbol, self.convert_type(&p.ty)))
                    .collect();
                let return_ty = m
                    .return_ty
                    .as_ref()
                    .map(|t| self.convert_type(t))
                    .unwrap_or(Type::Unit);
                let throws = m.throws.as_ref().map(|t| self.convert_type(t));

                InterfaceMethodDef {
                    name: m.name.symbol,
                    type_params: method_type_params,
                    params,
                    return_ty,
                    throws,
                }
            })
            .collect();

        self.symbols.define_type(
            i.name.symbol,
            TypeDef::Interface(InterfaceDef {
                name: i.name.symbol,
                type_params,
                extends,
                methods,
                is_public: i.is_public,
                span: i.span,
            }),
        );
    }

    fn collect_exception(&mut self, e: &ast::ExceptionItem) {
        let fields = e
            .fields
            .iter()
            .map(|f| (f.name.symbol, self.convert_type(&f.ty)))
            .collect();

        self.symbols.define_type(
            e.name.symbol,
            TypeDef::Exception(ExceptionDef {
                name: e.name.symbol,
                fields,
                is_public: e.is_public,
                span: e.span,
            }),
        );
    }

    fn check_items(&mut self, file: &SourceFile) {
        for item in &file.items {
            match item {
                Item::Function(func) => self.check_function(func),
                Item::TopLevelStmt(stmt_item) => self.check_top_level_stmt(stmt_item),
                _ => {}
            }
        }
    }

    fn check_top_level_stmt(&mut self, stmt_item: &ast::TopLevelStmtItem) {
        self.env.push_scope();

        let mut inferrer = TypeInferrer {
            env: &mut self.env,
            symbols: &self.symbols,
            interner: self.interner,
            next_var_id: &mut self.next_var_id,
            errors: &mut self.errors,
            annotations: &mut self.annotations,
            switch_scrutinee: None,
        };

        inferrer.check_stmt(&stmt_item.stmt);

        self.env.pop_scope();
    }

    fn check_function(&mut self, func: &ast::FunctionItem) {
        let return_ty = func
            .return_ty
            .as_ref()
            .map(|t| self.convert_type(t))
            .unwrap_or(Type::Unit);

        let throws = func.throws.as_ref().map(|t| self.convert_type(t));

        // Get type params from the function signature (if it was collected)
        let type_params = if func.receiver.is_some() {
            // Method: look up in method signature
            let recv = func.receiver.as_ref().unwrap();
            let recv_ty = self.convert_type(&recv.ty);
            let type_name = match &recv_ty {
                Type::Generic(name, _) => Some(*name),
                Type::Struct(s) => Some(s.name),
                _ => None,
            };
            type_name
                .and_then(|tn| self.symbols.get_method(tn, func.name.symbol))
                .map(|m| m.type_params.clone())
                .unwrap_or_default()
        } else {
            // Function: look up in function signature
            self.symbols
                .get_function(func.name.symbol)
                .map(|f| f.type_params.clone())
                .unwrap_or_default()
        };

        self.env
            .enter_function(return_ty, throws, &type_params);
        self.env.push_scope();

        if let Some(recv) = &func.receiver {
            let ty = self.convert_type(&recv.ty);
            self.env.define(recv.name.symbol, ty, recv.mutable);
        }

        for param in &func.params {
            let ty = self.convert_type(&param.ty);
            self.env.define(param.name.symbol, ty, false);
        }

        if let Some(body) = &func.body {
            let mut inferrer = TypeInferrer {
                env: &mut self.env,
                symbols: &self.symbols,
                interner: self.interner,
                next_var_id: &mut self.next_var_id,
                errors: &mut self.errors,
                annotations: &mut self.annotations,
                switch_scrutinee: None,
            };

            for stmt in &body.statements {
                inferrer.check_stmt(stmt);
            }
        }

        self.env.pop_scope();
        self.env.exit_function();
    }

    fn convert_type(&self, ast_ty: &ast::NamlType) -> Type {
        match ast_ty {
            ast::NamlType::Int => Type::Int,
            ast::NamlType::Uint => Type::Uint,
            ast::NamlType::Float => Type::Float,
            ast::NamlType::Bool => Type::Bool,
            ast::NamlType::String => Type::String,
            ast::NamlType::Bytes => Type::Bytes,
            ast::NamlType::Unit => Type::Unit,
            ast::NamlType::Decimal { .. } => Type::Float,
            ast::NamlType::Array(inner) => Type::Array(Box::new(self.convert_type(inner))),
            ast::NamlType::FixedArray(inner, n) => {
                Type::FixedArray(Box::new(self.convert_type(inner)), *n)
            }
            ast::NamlType::Option(inner) => Type::Option(Box::new(self.convert_type(inner))),
            ast::NamlType::Map(k, v) => Type::Map(
                Box::new(self.convert_type(k)),
                Box::new(self.convert_type(v)),
            ),
            ast::NamlType::Channel(inner) => Type::Channel(Box::new(self.convert_type(inner))),
            ast::NamlType::Promise(inner) => Type::Promise(Box::new(self.convert_type(inner))),
            ast::NamlType::Named(ident) => {
                if let Some(def) = self.symbols.get_type(ident.symbol) {
                    match def {
                        TypeDef::Struct(s) => Type::Struct(self.symbols.to_struct_type(s)),
                        TypeDef::Enum(e) => Type::Enum(self.symbols.to_enum_type(e)),
                        TypeDef::Interface(i) => {
                            Type::Interface(self.symbols.to_interface_type(i))
                        }
                        TypeDef::Exception(_) => Type::Generic(ident.symbol, Vec::new()),
                    }
                } else {
                    Type::Generic(ident.symbol, Vec::new())
                }
            }
            ast::NamlType::Generic(ident, args) => {
                let converted_args = args.iter().map(|a| self.convert_type(a)).collect();
                Type::Generic(ident.symbol, converted_args)
            }
            ast::NamlType::Function { params, returns } => {
                let param_types = params.iter().map(|p| self.convert_type(p)).collect();
                Type::Function(types::FunctionType {
                    params: param_types,
                    returns: Box::new(self.convert_type(returns)),
                    throws: None,
                    is_variadic: false,
                })
            }
            ast::NamlType::Inferred => unify::fresh_type_var(&mut 0),
        }
    }
}

pub fn check(file: &SourceFile, interner: &Rodeo) -> Vec<TypeError> {
    check_with_types(file, interner).errors
}

pub fn check_with_types(file: &SourceFile, interner: &Rodeo) -> TypeCheckResult {
    let mut checker = TypeChecker::new(interner);
    checker.collect_definitions(file);
    checker.check_items(file);

    TypeCheckResult {
        errors: std::mem::take(&mut checker.errors),
        annotations: std::mem::take(&mut checker.annotations),
        symbols: checker.symbols,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::AstArena;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    fn check_source(source: &str) -> Vec<TypeError> {
        let (tokens, interner) = tokenize(source);
        let arena = AstArena::new();
        let result = parse(&tokens, source, &arena);
        assert!(result.errors.is_empty(), "Parse errors: {:?}", result.errors);
        check(&result.ast, &interner)
    }

    #[test]
    fn test_check_empty() {
        let errors = check_source("");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_check_valid_function() {
        let errors = check_source("fn main() {}");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_check_valid_arithmetic() {
        let errors = check_source(
            "fn add(a: int, b: int) -> int { return a + b; }",
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_type_mismatch() {
        let errors = check_source(
            "fn main() { var x: int = true; }",
        );
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_undefined_variable() {
        let errors = check_source(
            "fn main() { return x; }",
        );
        assert!(!errors.is_empty());
        assert!(matches!(errors[0], TypeError::UndefinedVariable { .. }));
    }

    #[test]
    fn test_valid_if_statement() {
        let errors = check_source(
            "fn main() { if (true) { var x = 1; } }",
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_invalid_condition() {
        let errors = check_source(
            "fn main() { if (42) { var x = 1; } }",
        );
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_break_outside_loop() {
        let errors = check_source(
            "fn main() { break; }",
        );
        assert!(!errors.is_empty());
        assert!(matches!(errors[0], TypeError::BreakOutsideLoop { .. }));
    }

    #[test]
    fn test_valid_loop() {
        let errors = check_source(
            "fn main() { while (true) { break; } }",
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_valid_struct() {
        let errors = check_source(
            "struct Point { x: int, y: int }
             fn main() {}",
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_valid_method() {
        let errors = check_source(
            "struct Point { x: int, y: int }
             fn (self: Point) origin() -> bool { return self.x == 0; }
             fn main() {}",
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_type_inference() {
        let errors = check_source(
            "fn main() { var x = 42; var y: int = x; }",
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_array_type() {
        let errors = check_source(
            "fn main() { var arr = [1, 2, 3]; }",
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_lambda() {
        let errors = check_source(
            "fn main() { var f = fn(x: int) -> int { return x + 1; }; }",
        );
        assert!(errors.is_empty());
    }
}
