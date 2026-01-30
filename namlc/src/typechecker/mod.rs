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

use std::path::PathBuf;

use lasso::Rodeo;

use crate::ast::{self, Item, SourceFile, UseItems};
use crate::source::Span;

pub use error::{TypeError, TypeResult};
pub use symbols::SymbolTable;
pub use typed_ast::TypeAnnotations;
pub use types::Type;

pub struct TypeCheckResult {
    pub errors: Vec<TypeError>,
    pub annotations: TypeAnnotations,
    pub symbols: SymbolTable,
    pub imported_modules: Vec<ImportedModule>,
}

use env::TypeEnv;
use infer::TypeInferrer;
use symbols::{
    EnumDef, ExceptionDef, FunctionSig, InterfaceDef, InterfaceMethodDef, MethodSig, StructDef,
    TypeDef,
};
use types::TypeParam;

pub struct ImportedModule {
    pub source_text: String,
    pub file_path: PathBuf,
}

pub struct TypeChecker<'a> {
    symbols: SymbolTable,
    env: TypeEnv,
    interner: &'a Rodeo,
    errors: Vec<TypeError>,
    annotations: TypeAnnotations,
    next_var_id: u32,
    source_dir: Option<PathBuf>,
    imported_modules: Vec<ImportedModule>,
}

struct StdModuleFn {
    name: &'static str,
    type_params: Vec<&'static str>,
    params: Vec<(&'static str, Type)>,
    return_ty: Type,
    is_variadic: bool,
}

impl StdModuleFn {
    fn new(name: &'static str, params: Vec<(&'static str, Type)>, return_ty: Type) -> Self {
        Self { name, type_params: vec![], params, return_ty, is_variadic: false }
    }

    fn generic(name: &'static str, type_params: Vec<&'static str>, params: Vec<(&'static str, Type)>, return_ty: Type) -> Self {
        Self { name, type_params, params, return_ty, is_variadic: false }
    }
}

impl<'a> TypeChecker<'a> {
    pub fn new(interner: &'a Rodeo, source_dir: Option<PathBuf>) -> Self {
        let mut checker = Self {
            symbols: SymbolTable::new(),
            env: TypeEnv::new(),
            interner,
            errors: Vec::new(),
            annotations: TypeAnnotations::new(),
            next_var_id: 0,
            source_dir,
            imported_modules: Vec::new(),
        };
        checker.register_builtins();
        checker
    }

    fn register_builtins(&mut self) {
        use crate::source::Span;

        let builtins: Vec<(&str, bool, Type)> = vec![
            ("print", true, Type::Unit),
            ("println", true, Type::Unit),
            ("warn", true, Type::Unit),
            ("error", true, Type::Unit),
            ("panic", true, Type::Unit),
            ("fmt", true, Type::String),
            ("read_line", false, Type::String),
        ];

        for (name, is_variadic, return_ty) in builtins {
            if let Some(spur) = self.interner.get(name) {
                self.symbols.define_function(FunctionSig {
                    name: spur,
                    type_params: vec![],
                    params: vec![],
                    return_ty,
                    throws: vec![],
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
                throws: vec![],
                is_public: true,
                is_variadic: false,
                span: Span::dummy(),
            });
        }

    }

    pub fn check(&mut self, file: &SourceFile) -> Vec<TypeError> {
        self.collect_definitions(file);
        self.validate_interface_implementations();
        self.check_items(file);
        std::mem::take(&mut self.errors)
    }

    fn validate_interface_implementations(&mut self) {
        let structs: Vec<_> = self.symbols.all_types()
            .filter_map(|(_, def)| {
                if let TypeDef::Struct(s) = def {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .collect();

        for struct_def in structs {
            for impl_ty in &struct_def.implements {
                let interface_name = match impl_ty {
                    Type::Generic(name, _) => *name,
                    Type::Interface(i) => i.name,
                    _ => continue,
                };

                let interface_def = match self.symbols.get_type(interface_name) {
                    Some(TypeDef::Interface(i)) => i.clone(),
                    _ => continue,
                };

                for required_method in &interface_def.methods {
                    let method_name_str = self.interner.resolve(&required_method.name);
                    let has_method = self.symbols.get_method(struct_def.name, required_method.name);

                    if has_method.is_none() {
                        let struct_name = self.interner.resolve(&struct_def.name).to_string();
                        let interface_name_str = self.interner.resolve(&interface_name).to_string();
                        self.errors.push(TypeError::MissingInterfaceMethod {
                            struct_name,
                            interface_name: interface_name_str,
                            method_name: method_name_str.to_string(),
                            span: struct_def.span,
                        });
                    }
                }
            }
        }
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
                Item::Use(u) => self.resolve_use_item(u),
                Item::TopLevelStmt(_) => {}
            }
        }
    }

    fn resolve_use_item(&mut self, use_item: &ast::UseItem) {
        let path_strs: Vec<String> = use_item.path.iter()
            .map(|ident| self.interner.resolve(&ident.symbol).to_string())
            .collect();

        if path_strs.is_empty() {
            return;
        }

        if path_strs[0] == "std" {
            if path_strs.len() < 2 {
                self.errors.push(TypeError::UnknownModule {
                    path: path_strs.join("::"),
                    span: use_item.span,
                });
                return;
            }
            self.resolve_std_module(&path_strs[1], &use_item.items, use_item.span);
        } else {
            self.resolve_local_module(&path_strs, &use_item.items, use_item.span);
        }
    }

    fn resolve_std_module(&mut self, module: &str, items: &UseItems, span: crate::source::Span) {
        let module_fns = match Self::get_std_module_functions(module) {
            Some(fns) => fns,
            None => {
                self.errors.push(TypeError::UnknownModule {
                    path: format!("std.{}", module),
                    span,
                });
                return;
            }
        };

        match items {
            UseItems::All => {
                for module_fn in &module_fns {
                    self.register_std_fn(module_fn);
                }
            }
            UseItems::Specific(entries) => {
                for entry in entries {
                    let entry_name = self.interner.resolve(&entry.name.symbol).to_string();
                    let found = module_fns.iter().find(|f| f.name == entry_name);
                    match found {
                        Some(module_fn) => {
                            self.register_std_fn(module_fn);
                        }
                        None => {
                            self.errors.push(TypeError::UnknownModuleSymbol {
                                module: format!("std.{}", module),
                                symbol: entry_name,
                                span: entry.span,
                            });
                        }
                    }
                }
            }
        }
    }

    fn register_std_fn(&mut self, module_fn: &StdModuleFn) {
        if let Some(spur) = self.interner.get(module_fn.name) {
            let type_params: Vec<_> = module_fn.type_params.iter()
                .map(|tp_name| {
                    let tp_spur = self.interner.get(tp_name).unwrap_or(spur);
                    TypeParam { name: tp_spur, bounds: vec![] }
                })
                .collect();

            let mut return_ty = module_fn.return_ty.clone();
            if let Type::Channel(inner) = &mut return_ty {
                if let Type::Generic(g_spur, _) = inner.as_mut() {
                    if *g_spur == lasso::Spur::default() {
                        if let Some(tp) = type_params.first() {
                            *g_spur = tp.name;
                        }
                    }
                }
            }

            let params: Vec<_> = module_fn.params.iter()
                .map(|(pname, pty)| {
                    let pspur = self.interner.get(pname).unwrap_or(spur);
                    (pspur, pty.clone())
                })
                .collect();

            self.symbols.define_function(FunctionSig {
                name: spur,
                type_params,
                params,
                return_ty,
                throws: vec![],
                is_public: true,
                is_variadic: module_fn.is_variadic,
                span: Span::dummy(),
            });
        }
    }

    fn get_std_module_functions(module: &str) -> Option<Vec<StdModuleFn>> {
        match module {
            "random" => Some(vec![
                StdModuleFn::new("random", vec![("min", Type::Int), ("max", Type::Int)], Type::Int),
                StdModuleFn::new("random_float", vec![], Type::Float),
            ]),
            "io" => Some(vec![
                StdModuleFn::new("read_key", vec![], Type::Int),
                StdModuleFn::new("clear_screen", vec![], Type::Unit),
                StdModuleFn::new("set_cursor", vec![("x", Type::Int), ("y", Type::Int)], Type::Unit),
                StdModuleFn::new("hide_cursor", vec![], Type::Unit),
                StdModuleFn::new("show_cursor", vec![], Type::Unit),
                StdModuleFn::new("terminal_width", vec![], Type::Int),
                StdModuleFn::new("terminal_height", vec![], Type::Int),
            ]),
            "threads" => Some(vec![
                StdModuleFn::new("join", vec![], Type::Unit),
                StdModuleFn::generic("open_channel", vec!["T"], vec![("capacity", Type::Int)],
                    Type::Channel(Box::new(Type::Generic(lasso::Spur::default(), vec![])))),
            ]),
            "datetime" => Some(vec![
                StdModuleFn::new("now_ms", vec![], Type::Int),
                StdModuleFn::new("now_s", vec![], Type::Int),
                StdModuleFn::new("year", vec![("timestamp_ms", Type::Int)], Type::Int),
                StdModuleFn::new("month", vec![("timestamp_ms", Type::Int)], Type::Int),
                StdModuleFn::new("day", vec![("timestamp_ms", Type::Int)], Type::Int),
                StdModuleFn::new("hour", vec![("timestamp_ms", Type::Int)], Type::Int),
                StdModuleFn::new("minute", vec![("timestamp_ms", Type::Int)], Type::Int),
                StdModuleFn::new("second", vec![("timestamp_ms", Type::Int)], Type::Int),
                StdModuleFn::new("day_of_week", vec![("timestamp_ms", Type::Int)], Type::Int),
                StdModuleFn::new("format_date", vec![("timestamp_ms", Type::Int), ("fmt", Type::String)], Type::String),
            ]),
            "metrics" => Some(vec![
                StdModuleFn::new("perf_now", vec![], Type::Int),
                StdModuleFn::new("elapsed_ms", vec![("start_ns", Type::Int)], Type::Int),
                StdModuleFn::new("elapsed_us", vec![("start_ns", Type::Int)], Type::Int),
                StdModuleFn::new("elapsed_ns", vec![("start_ns", Type::Int)], Type::Int),
            ]),
            "strings" => Some(vec![
                StdModuleFn::new("upper", vec![("s", Type::String)], Type::String),
                StdModuleFn::new("lower", vec![("s", Type::String)], Type::String),
                StdModuleFn::new("split", vec![("s", Type::String), ("delim", Type::String)], Type::Array(Box::new(Type::String))),
                StdModuleFn::new("concat", vec![("arr", Type::Array(Box::new(Type::String))), ("delim", Type::String)], Type::String),
                StdModuleFn::new("has", vec![("s", Type::String), ("substr", Type::String)], Type::Bool),
                StdModuleFn::new("starts_with", vec![("s", Type::String), ("prefix", Type::String)], Type::Bool),
                StdModuleFn::new("ends_with", vec![("s", Type::String), ("suffix", Type::String)], Type::Bool),
                StdModuleFn::new("replace", vec![("s", Type::String), ("old", Type::String), ("new", Type::String)], Type::String),
                StdModuleFn::new("replace_all", vec![("s", Type::String), ("old", Type::String), ("new", Type::String)], Type::String),
                StdModuleFn::new("ltrim", vec![("s", Type::String)], Type::String),
                StdModuleFn::new("rtrim", vec![("s", Type::String)], Type::String),
                StdModuleFn::new("substr", vec![("s", Type::String), ("start", Type::Int), ("end", Type::Int)], Type::String),
                StdModuleFn::new("lpad", vec![("s", Type::String), ("len", Type::Int), ("char", Type::String)], Type::String),
                StdModuleFn::new("rpad", vec![("s", Type::String), ("len", Type::Int), ("char", Type::String)], Type::String),
                StdModuleFn::new("repeat", vec![("s", Type::String), ("n", Type::Int)], Type::String),
                StdModuleFn::new("lines", vec![("s", Type::String)], Type::Array(Box::new(Type::String))),
                StdModuleFn::new("chars", vec![("s", Type::String)], Type::Array(Box::new(Type::String))),
            ]),
            "collections" => Some(vec![
                // Access functions
                StdModuleFn::new("first", vec![("arr", Type::Array(Box::new(Type::Int)))], Type::Option(Box::new(Type::Int))),
                StdModuleFn::new("last", vec![("arr", Type::Array(Box::new(Type::Int)))], Type::Option(Box::new(Type::Int))),
                // Aggregation
                StdModuleFn::new("sum", vec![("arr", Type::Array(Box::new(Type::Int)))], Type::Int),
                StdModuleFn::new("min", vec![("arr", Type::Array(Box::new(Type::Int)))], Type::Option(Box::new(Type::Int))),
                StdModuleFn::new("max", vec![("arr", Type::Array(Box::new(Type::Int)))], Type::Option(Box::new(Type::Int))),
                // Transformation
                StdModuleFn::new("reversed", vec![("arr", Type::Array(Box::new(Type::Int)))], Type::Array(Box::new(Type::Int))),
                // Slicing
                StdModuleFn::new("take", vec![("arr", Type::Array(Box::new(Type::Int))), ("n", Type::Int)], Type::Array(Box::new(Type::Int))),
                StdModuleFn::new("drop", vec![("arr", Type::Array(Box::new(Type::Int))), ("n", Type::Int)], Type::Array(Box::new(Type::Int))),
                StdModuleFn::new("slice", vec![("arr", Type::Array(Box::new(Type::Int))), ("start", Type::Int), ("end", Type::Int)], Type::Array(Box::new(Type::Int))),
                // Search
                StdModuleFn::new("index_of", vec![("arr", Type::Array(Box::new(Type::Int))), ("val", Type::Int)], Type::Option(Box::new(Type::Int))),
                StdModuleFn::new("contains", vec![("arr", Type::Array(Box::new(Type::Int))), ("val", Type::Int)], Type::Bool),
                // Lambda-based functions (predicate: fn(int) -> bool)
                StdModuleFn::new("any", vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    ("predicate", Type::Function(types::FunctionType { params: vec![Type::Int], returns: Box::new(Type::Bool), throws: vec![], is_variadic: false })),
                ], Type::Bool),
                StdModuleFn::new("all", vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    ("predicate", Type::Function(types::FunctionType { params: vec![Type::Int], returns: Box::new(Type::Bool), throws: vec![], is_variadic: false })),
                ], Type::Bool),
                StdModuleFn::new("count", vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    ("predicate", Type::Function(types::FunctionType { params: vec![Type::Int], returns: Box::new(Type::Bool), throws: vec![], is_variadic: false })),
                ], Type::Int),
                StdModuleFn::new("apply", vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    ("mapper", Type::Function(types::FunctionType { params: vec![Type::Int], returns: Box::new(Type::Int), throws: vec![], is_variadic: false })),
                ], Type::Array(Box::new(Type::Int))),
                StdModuleFn::new("where", vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    ("predicate", Type::Function(types::FunctionType { params: vec![Type::Int], returns: Box::new(Type::Bool), throws: vec![], is_variadic: false })),
                ], Type::Array(Box::new(Type::Int))),
                StdModuleFn::new("find", vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    ("predicate", Type::Function(types::FunctionType { params: vec![Type::Int], returns: Box::new(Type::Bool), throws: vec![], is_variadic: false })),
                ], Type::Option(Box::new(Type::Int))),
                StdModuleFn::new("find_index", vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    ("predicate", Type::Function(types::FunctionType { params: vec![Type::Int], returns: Box::new(Type::Bool), throws: vec![], is_variadic: false })),
                ], Type::Option(Box::new(Type::Int))),
                StdModuleFn::new("fold", vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    ("initial", Type::Int),
                    ("reducer", Type::Function(types::FunctionType { params: vec![Type::Int, Type::Int], returns: Box::new(Type::Int), throws: vec![], is_variadic: false })),
                ], Type::Int),
                StdModuleFn::new("flatten", vec![("arr", Type::Array(Box::new(Type::Array(Box::new(Type::Int)))))], Type::Array(Box::new(Type::Int))),
                StdModuleFn::new("sort", vec![("arr", Type::Array(Box::new(Type::Int)))], Type::Array(Box::new(Type::Int))),
                StdModuleFn::new("sort_by", vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    ("comparator", Type::Function(types::FunctionType { params: vec![Type::Int, Type::Int], returns: Box::new(Type::Int), throws: vec![], is_variadic: false })),
                ], Type::Array(Box::new(Type::Int))),
            ]),
            _ => None,
        }
    }

    fn resolve_local_module(&mut self, path: &[String], items: &UseItems, span: crate::source::Span) {
        let source_dir = match &self.source_dir {
            Some(d) => d.clone(),
            None => {
                self.errors.push(TypeError::ModuleFileError {
                    path: path.join("::"),
                    reason: "no source directory available for local module resolution".to_string(),
                    span,
                });
                return;
            }
        };

        let mut file_path = source_dir;
        for segment in path {
            file_path.push(segment);
        }
        file_path.set_extension("naml");

        let source_text = match std::fs::read_to_string(&file_path) {
            Ok(s) => s,
            Err(e) => {
                self.errors.push(TypeError::ModuleFileError {
                    path: file_path.display().to_string(),
                    reason: e.to_string(),
                    span,
                });
                return;
            }
        };

        let (tokens, module_interner) = crate::lexer::tokenize(&source_text);
        let arena = crate::ast::AstArena::new();
        let parse_result = crate::parser::parse(&tokens, &source_text, &arena);

        if !parse_result.errors.is_empty() {
            self.errors.push(TypeError::ModuleFileError {
                path: file_path.display().to_string(),
                reason: "parse errors in module file".to_string(),
                span,
            });
            return;
        }

        let mut pub_functions: Vec<(String, Vec<(String, Type)>, Type, bool)> = Vec::new();

        for item in &parse_result.ast.items {
            if let Item::Function(func) = item {
                if func.is_public && func.receiver.is_none() {
                    let name = module_interner.resolve(&func.name.symbol).to_string();
                    let params: Vec<_> = func.params.iter()
                        .map(|p| {
                            let pname = module_interner.resolve(&p.name.symbol).to_string();
                            let pty = self.convert_type(&p.ty);
                            (pname, pty)
                        })
                        .collect();
                    let return_ty = func.return_ty.as_ref()
                        .map(|t| self.convert_type(t))
                        .unwrap_or(Type::Unit);
                    pub_functions.push((name, params, return_ty, false));
                }
            }
        }

        match items {
            UseItems::All => {
                for (name, params, return_ty, is_variadic) in &pub_functions {
                    if let Some(spur) = self.interner.get(name.as_str()) {
                        let params: Vec<_> = params.iter()
                            .map(|(pname, pty)| {
                                let pspur = self.interner.get(pname.as_str()).unwrap_or(spur);
                                (pspur, pty.clone())
                            })
                            .collect();
                        self.symbols.define_function(FunctionSig {
                            name: spur,
                            type_params: vec![],
                            params,
                            return_ty: return_ty.clone(),
                            throws: vec![],
                            is_public: true,
                            is_variadic: *is_variadic,
                            span: crate::source::Span::dummy(),
                        });
                    }
                }
            }
            UseItems::Specific(entries) => {
                for entry in entries {
                    let entry_name = self.interner.resolve(&entry.name.symbol).to_string();
                    let found = pub_functions.iter().find(|(name, _, _, _)| *name == entry_name);
                    match found {
                        Some((_, params, return_ty, is_variadic)) => {
                            let spur = entry.name.symbol;
                            let params: Vec<_> = params.iter()
                                .map(|(pname, pty)| {
                                    let pspur = self.interner.get(pname.as_str()).unwrap_or(spur);
                                    (pspur, pty.clone())
                                })
                                .collect();
                            self.symbols.define_function(FunctionSig {
                                name: spur,
                                type_params: vec![],
                                params,
                                return_ty: return_ty.clone(),
                                throws: vec![],
                                is_public: true,
                                is_variadic: *is_variadic,
                                span: crate::source::Span::dummy(),
                            });
                        }
                        None => {
                            self.errors.push(TypeError::PrivateSymbol {
                                module: path.join("::"),
                                symbol: entry_name,
                                span: entry.span,
                            });
                        }
                    }
                }
            }
        }

        self.imported_modules.push(ImportedModule {
            source_text,
            file_path,
        });
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

        let throws = func.throws.iter().map(|t| self.convert_type(t)).collect();

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

        let throws = ext.throws.iter().map(|t| self.convert_type(t)).collect();

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

        let throws = func.throws.iter().map(|t| self.convert_type(t)).collect();

        self.symbols.define_method(
            type_name,
            MethodSig {
                name: func.name.symbol,
                receiver_ty,
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
                let throws = m.throws.iter().map(|t| self.convert_type(t)).collect();

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
            in_catch_context: false,
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

        let throws = func.throws.iter().map(|t| self.convert_type(t)).collect();

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
            self.env.define(recv.name.symbol, ty, true);
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
                in_catch_context: false,
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
            ast::NamlType::Named(ident) => {
                if let Some(def) = self.symbols.get_type(ident.symbol) {
                    match def {
                        TypeDef::Struct(s) => Type::Struct(self.symbols.to_struct_type(s)),
                        TypeDef::Enum(e) => Type::Enum(self.symbols.to_enum_type(e)),
                        TypeDef::Interface(i) => {
                            Type::Interface(self.symbols.to_interface_type(i))
                        }
                        TypeDef::Exception(e) => Type::Exception(e.name),
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
                    throws: vec![],
                    is_variadic: false,
                })
            }
            ast::NamlType::Inferred => unify::fresh_type_var(&mut 0),
        }
    }
}

pub fn check(file: &SourceFile, interner: &Rodeo) -> Vec<TypeError> {
    check_with_types(file, interner, None).errors
}

pub fn check_with_types(file: &SourceFile, interner: &Rodeo, source_dir: Option<PathBuf>) -> TypeCheckResult {
    let mut checker = TypeChecker::new(interner, source_dir);
    checker.collect_definitions(file);
    checker.validate_interface_implementations();
    checker.check_items(file);

    TypeCheckResult {
        errors: std::mem::take(&mut checker.errors),
        annotations: std::mem::take(&mut checker.annotations),
        symbols: checker.symbols,
        imported_modules: std::mem::take(&mut checker.imported_modules),
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
            "fn main() { if (true) { var x: int = 1; } }",
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_invalid_condition() {
        let errors = check_source(
            "fn main() { if (42) { var x: int = 1; } }",
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
    fn test_type_annotation_required() {
        let errors = check_source(
            "fn main() { var x: int = 42; var y: int = x; }",
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_array_type() {
        let errors = check_source(
            "fn main() { var arr: [int] = [1, 2, 3]; }",
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_lambda() {
        let errors = check_source(
            "fn main() { var f: fn(int) -> int = fn(x: int) -> int { return x + 1; }; }",
        );
        assert!(errors.is_empty());
    }
}
