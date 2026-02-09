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

use lasso::{Rodeo, Spur};

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
    TypeAliasDef, TypeDef,
};
use types::TypeParam;

pub struct ImportedModule {
    pub source_text: String,
    pub file_path: PathBuf,
}

pub struct TypeChecker<'a> {
    symbols: SymbolTable,
    env: TypeEnv,
    interner: &'a mut Rodeo,
    errors: Vec<TypeError>,
    annotations: TypeAnnotations,
    next_var_id: u32,
    source_dir: Option<PathBuf>,
    imported_modules: Vec<ImportedModule>,
}

pub struct StdModuleFn {
    pub name: &'static str,
    pub type_params: Vec<&'static str>,
    pub params: Vec<(&'static str, Type)>,
    pub return_ty: Type,
    pub throws: Vec<&'static str>,
    pub is_variadic: bool,
}

impl StdModuleFn {
    fn new(name: &'static str, params: Vec<(&'static str, Type)>, return_ty: Type) -> Self {
        Self {
            name,
            type_params: vec![],
            params,
            return_ty,
            throws: vec![],
            is_variadic: false,
        }
    }

    fn throwing(
        name: &'static str,
        params: Vec<(&'static str, Type)>,
        return_ty: Type,
        throws: Vec<&'static str>,
    ) -> Self {
        Self {
            name,
            type_params: vec![],
            params,
            return_ty,
            throws,
            is_variadic: false,
        }
    }

    fn generic(
        name: &'static str,
        type_params: Vec<&'static str>,
        params: Vec<(&'static str, Type)>,
        return_ty: Type,
    ) -> Self {
        Self {
            name,
            type_params,
            params,
            return_ty,
            throws: vec![],
            is_variadic: false,
        }
    }
}

pub fn get_std_module_functions(module: &str) -> Option<Vec<StdModuleFn>> {
    TypeChecker::get_std_module_functions_impl(module)
}

impl<'a> TypeChecker<'a> {
    pub fn new(interner: &'a mut Rodeo, source_dir: Option<PathBuf>) -> Self {
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

        // Common functions in root (global builtins, no module required)
        let builtins: Vec<(&str, bool, Type)> = vec![
            ("print", true, Type::Unit),
            ("println", true, Type::Unit),
            ("warn", true, Type::Unit),
            ("error", true, Type::Unit),
            ("panic", true, Type::Unit),
            ("fmt", true, Type::String),
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
                    module: None,
                });
            }
        }

        // Register standard exceptions
        let io_error_name = self.interner.get_or_intern("IOError");
        let msg_name = self.interner.get_or_intern("message");
        let path_name = self.interner.get_or_intern("path");
        let code_name = self.interner.get_or_intern("code");

        self.symbols.define_type(
            io_error_name,
            TypeDef::Exception(ExceptionDef {
                name: io_error_name,
                fields: vec![
                    (msg_name, Type::String),
                    (path_name, Type::String),
                    (code_name, Type::Int),
                ],
                is_public: true,
                span: Span::dummy(),
            }),
        );

        let decode_error_name = self.interner.get_or_intern("DecodeError");
        self.symbols.define_type(
            decode_error_name,
            TypeDef::Exception(ExceptionDef {
                name: decode_error_name,
                fields: vec![(msg_name, Type::String)],
                is_public: true,
                span: Span::dummy(),
            }),
        );

        let path_error_name = self.interner.get_or_intern("PathError");
        self.symbols.define_type(
            path_error_name,
            TypeDef::Exception(ExceptionDef {
                name: path_error_name,
                fields: vec![(msg_name, Type::String)],
                is_public: true,
                span: Span::dummy(),
            }),
        );

        let network_error_name = self.interner.get_or_intern("NetworkError");
        self.symbols.define_type(
            network_error_name,
            TypeDef::Exception(ExceptionDef {
                name: network_error_name,
                fields: vec![(msg_name, Type::String)],
                is_public: true,
                span: Span::dummy(),
            }),
        );

        let timeout_error_name = self.interner.get_or_intern("TimeoutError");
        self.symbols.define_type(
            timeout_error_name,
            TypeDef::Exception(ExceptionDef {
                name: timeout_error_name,
                fields: vec![(msg_name, Type::String)],
                is_public: true,
                span: Span::dummy(),
            }),
        );

        let permission_error_name = self.interner.get_or_intern("PermissionError");
        self.symbols.define_type(
            permission_error_name,
            TypeDef::Exception(ExceptionDef {
                name: permission_error_name,
                fields: vec![
                    (msg_name, Type::String),
                    (path_name, Type::String),
                    (code_name, Type::Int),
                ],
                is_public: true,
                span: Span::dummy(),
            }),
        );

        let env_error_name = self.interner.get_or_intern("EnvError");
        let key_name = self.interner.get_or_intern("key");
        self.symbols.define_type(
            env_error_name,
            TypeDef::Exception(ExceptionDef {
                name: env_error_name,
                fields: vec![
                    (msg_name, Type::String),
                    (key_name, Type::String),
                ],
                is_public: true,
                span: Span::dummy(),
            }),
        );

        let os_error_name = self.interner.get_or_intern("OSError");
        self.symbols.define_type(
            os_error_name,
            TypeDef::Exception(ExceptionDef {
                name: os_error_name,
                fields: vec![
                    (msg_name, Type::String),
                    (code_name, Type::Int),
                ],
                is_public: true,
                span: Span::dummy(),
            }),
        );

        let process_error_name = self.interner.get_or_intern("ProcessError");
        self.symbols.define_type(
            process_error_name,
            TypeDef::Exception(ExceptionDef {
                name: process_error_name,
                fields: vec![
                    (msg_name, Type::String),
                    (code_name, Type::Int),
                ],
                is_public: true,
                span: Span::dummy(),
            }),
        );

        let encode_error_name = self.interner.get_or_intern("EncodeError");
        self.symbols.define_type(
            encode_error_name,
            TypeDef::Exception(ExceptionDef {
                name: encode_error_name,
                fields: vec![(msg_name, Type::String)],
                is_public: true,
                span: Span::dummy(),
            }),
        );

        let db_error_name = self.interner.get_or_intern("DBError");
        self.symbols.define_type(
            db_error_name,
            TypeDef::Exception(ExceptionDef {
                name: db_error_name,
                fields: vec![
                    (msg_name, Type::String),
                    (code_name, Type::Int),
                ],
                is_public: true,
                span: Span::dummy(),
            }),
        );

        self.register_std_lib();
    }

    fn register_std_lib(&mut self) {
        let std_spur = self.interner.get_or_intern("std");
        self.symbols.enter_module(std_spur);

        // Populate std submodules from get_std_module_functions_impl
        let modules = vec![
            "random",
            "io",
            "threads",
            "datetime",
            "metrics",
            "strings",
            "collections",
            "collections::arrays",
            "collections::maps",
            "fs",
            "path",
            "encoding",
            "encoding::utf8",
            "encoding::hex",
            "encoding::base64",
            "encoding::url",
            "encoding::json",
            "encoding::toml",
            "encoding::yaml",
            "testing",
            "env",
            "os",
            "process",
            "net",
            "net::tcp",
            "net::tcp::server",
            "net::tcp::client",
            "net::udp",
            "net::http",
            "net::http::client",
            "net::http::server",
            "net::http::middleware",
            "db",
            "db::sqlite",
        ];

        for module in modules {
            if let Some(fns) = get_std_module_functions(module) {
                // Split module name into components for hierarchical entry
                let parts: Vec<&str> = module.split("::").collect();
                for &part in &parts {
                    let part_spur = self.interner.get_or_intern(part);
                    self.symbols.enter_module(part_spur);
                }

                for module_fn in fns {
                    if let Some(sig) = self.create_std_fn_sig(&module_fn, module) {
                        self.symbols.define_module_function(sig);
                    }
                }

                for _ in &parts {
                    self.symbols.exit_module();
                }
            }
        }

        self.symbols.exit_module(); // exit std
    }

    pub fn check(&mut self, file: &SourceFile) -> Vec<TypeError> {
        self.collect_definitions(file);
        self.validate_interface_implementations();
        self.check_items(file);
        std::mem::take(&mut self.errors)
    }

    fn validate_interface_implementations(&mut self) {
        let structs: Vec<_> = self
            .symbols
            .all_types()
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
                    let has_method = self
                        .symbols
                        .get_method(struct_def.name, required_method.name);

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
            self.collect_item_definition(item);
        }
    }

    fn collect_item_definition(&mut self, item: &Item) {
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
            Item::Use(u) => self.resolve_use_item(u),
            Item::Extern(e) => self.collect_extern(e),
            Item::TypeAlias(a) => self.collect_type_alias(a),
            Item::Mod(m) => self.collect_mod(m),
            Item::TopLevelStmt(_) => {}
        }
    }

    fn collect_mod(&mut self, m: &ast::ModuleItem) {
        let name_spur = m.name.symbol;
        self.symbols.enter_module(name_spur);
        if let Some(ref items) = m.body {
            for item in items {
                self.collect_item_definition(item);
            }
        } else {
            self.collect_local_module_as_mod(name_spur, m.span);
        }
        self.symbols.exit_module();
    }

    fn collect_local_module_as_mod(&mut self, name: Spur, span: Span) {
        let name_str = self.interner.resolve(&name).to_string();
        let source_dir = match &self.source_dir {
            Some(d) => d.clone(),
            None => {
                self.errors.push(TypeError::ModuleFileError {
                    path: name_str,
                    reason: "no source directory".to_string(),
                    span,
                });
                return;
            }
        };

        let mut file_path = source_dir.clone();
        file_path.push(&name_str);
        file_path.set_extension("naml");

        if !file_path.exists() {
            file_path = source_dir;
            file_path.push(&name_str);
            file_path.push("mod.naml");
        }

        if !file_path.exists() {
            self.errors.push(TypeError::ModuleFileError {
                path: name_str,
                reason: "file not found".to_string(),
                span,
            });
            return;
        }

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

        let tokens = crate::lexer::tokenize_with_interner(&source_text, self.interner);
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

        let old_dir = self.source_dir.clone();
        if let Some(parent) = file_path.parent() {
            self.source_dir = Some(parent.to_path_buf());
        }

        for item in &parse_result.ast.items {
            self.collect_item_definition(item);
        }

        self.source_dir = old_dir;

        self.imported_modules.push(ImportedModule {
            source_text,
            file_path,
        });
    }

    fn resolve_use_item(&mut self, use_item: &ast::UseItem) {
        let path_spurs: Vec<Spur> = use_item.path.iter().map(|i| i.symbol).collect();
        if path_spurs.is_empty() {
            return;
        }

        let mut functions_to_import = Vec::new();
        let mut types_to_import = Vec::new();
        let mut submodules_to_import = Vec::new();
        let mut import_errors = Vec::new();
        let mut resolved_module_found = false;

        {
            // Immutable borrow scope
            let mut curr_module = &self.symbols.root;
            let mut resolved = true;

            for &seg in &path_spurs {
                if let Some(sub) = curr_module.get_submodule(seg) {
                    curr_module = sub;
                } else {
                    resolved = false;
                    break;
                }
            }

            if resolved {
                resolved_module_found = true;
                match &use_item.items {
                    UseItems::All => {
                        for sig in curr_module.all_functions() {
                            functions_to_import.push(sig.clone());
                        }
                        for (name, def) in curr_module.all_types() {
                            types_to_import.push((*name, def.clone()));
                        }
                        for (name, sub) in curr_module.all_submodules() {
                            submodules_to_import.push((*name, sub.clone()));
                        }
                    }
                    UseItems::Specific(entries) => {
                        for entry in entries {
                            let name = entry.name.symbol;
                            let import_name =
                                entry.alias.as_ref().map(|a| a.symbol).unwrap_or(name);
                            let mut found = false;

                            if let Some(sig) = curr_module.get_function(name) {
                                let mut sig = sig.clone();
                                sig.name = import_name;
                                functions_to_import.push(sig);
                                found = true;
                            }
                            if let Some(def) = curr_module.get_type(name) {
                                types_to_import.push((import_name, def.clone()));
                                found = true;
                            }

                            if !found {
                                let name_str = self.interner.resolve(&name).to_string();
                                let module_name =
                                    self.interner.resolve(&curr_module.name).to_string();
                                import_errors.push(TypeError::UnknownModuleSymbol {
                                    module: module_name,
                                    symbol: name_str,
                                    span: entry.span,
                                });
                            }
                        }
                    }
                }
            }
        }

        if resolved_module_found {
            // Perform the actual imports
            for sig in functions_to_import {
                self.symbols.import_function(sig);
            }
            for (name, def) in types_to_import {
                self.symbols.define_type(name, def);
            }
            for (name, sub) in submodules_to_import {
                self.symbols.define_module(name, sub);
            }
            for err in import_errors {
                self.errors.push(err);
            }
        } else if path_spurs[0] == self.interner.get_or_intern("std") {
            // Already tried pre-populated std, if not found then it's an error
            let path_str = path_spurs
                .iter()
                .map(|&s| self.interner.resolve(&s))
                .collect::<Vec<_>>()
                .join("::");
            self.errors.push(TypeError::UnknownModule {
                path: path_str,
                span: use_item.span,
            });
        } else {
            // Fallback to local module resolution (which might load files)
            let path_strs: Vec<String> = path_spurs
                .iter()
                .map(|&s| self.interner.resolve(&s).to_string())
                .collect();
            self.resolve_local_module(&path_strs, &use_item.items, use_item.span);
        }
    }

    fn create_std_fn_sig(
        &mut self,
        module_fn: &StdModuleFn,
        module_name: &str,
    ) -> Option<FunctionSig> {
        let spur = self.interner.get_or_intern(module_fn.name);

        let type_params: Vec<_> = module_fn
            .type_params
            .iter()
            .map(|tp_name| {
                let tp_spur = self.interner.get_or_intern(tp_name);
                TypeParam {
                    name: tp_spur,
                    bounds: vec![],
                }
            })
            .collect();

        let mut return_ty = module_fn.return_ty.clone();
        Self::fix_default_generic_spur(&mut return_ty, &type_params);

        let params: Vec<_> = module_fn
            .params
            .iter()
            .map(|(pname, pty)| {
                let pspur = self.interner.get_or_intern(pname);
                let mut param_ty = pty.clone();
                Self::fix_default_generic_spur(&mut param_ty, &type_params);
                (pspur, param_ty)
            })
            .collect();

        let throws: Vec<_> = module_fn
            .throws
            .iter()
            .map(|ex_name| {
                let ex_spur = self.interner.get_or_intern(ex_name);
                Type::Exception(ex_spur)
            })
            .collect();

        // Use full module path for qualified function lookup in codegen
        let module = Some(module_name.to_string());

        Some(FunctionSig {
            name: spur,
            type_params,
            params,
            return_ty,
            throws,
            is_public: true,
            is_variadic: module_fn.is_variadic,
            span: Span::dummy(),
            module,
        })
    }

    /// Recursively fix Type::Generic with default spur to use the first type parameter
    fn fix_default_generic_spur(ty: &mut Type, type_params: &[TypeParam]) {
        match ty {
            Type::Generic(g_spur, _) => {
                if *g_spur == lasso::Spur::default() {
                    if let Some(tp) = type_params.first() {
                        *g_spur = tp.name;
                    }
                }
            }
            Type::Channel(inner) => Self::fix_default_generic_spur(inner, type_params),
            Type::Array(inner) => Self::fix_default_generic_spur(inner, type_params),
            Type::Option(inner) => Self::fix_default_generic_spur(inner, type_params),
            Type::Mutex(inner) => Self::fix_default_generic_spur(inner, type_params),
            Type::Rwlock(inner) => Self::fix_default_generic_spur(inner, type_params),
            Type::Atomic(inner) => Self::fix_default_generic_spur(inner, type_params),
            Type::Map(k, v) => {
                Self::fix_default_generic_spur(k, type_params);
                Self::fix_default_generic_spur(v, type_params);
            }
            _ => {}
        }
    }

    fn get_collections_array_functions() -> Vec<StdModuleFn> {
        // Use a default Spur for generic type T
        let generic_t = || Type::Generic(lasso::Spur::default(), vec![]);
        let array_of_t = || Type::Array(Box::new(generic_t()));
        let option_of_t = || Type::Option(Box::new(generic_t()));

        vec![
            // Basic functions (Go-style) - generic over element type T
            StdModuleFn::generic(
                "count",
                vec!["T"],
                vec![("arr", array_of_t())],
                Type::Int,
            ),
            StdModuleFn::generic(
                "reserved",
                vec!["T"],
                vec![("capacity", Type::Int)],
                array_of_t(),
            ),
            StdModuleFn::generic(
                "push",
                vec!["T"],
                vec![("arr", array_of_t()), ("value", generic_t())],
                Type::Unit,
            ),
            StdModuleFn::generic(
                "pop",
                vec!["T"],
                vec![("arr", array_of_t())],
                option_of_t(),
            ),
            StdModuleFn::generic(
                "shift",
                vec!["T"],
                vec![("arr", array_of_t())],
                option_of_t(),
            ),
            StdModuleFn::generic(
                "fill",
                vec!["T"],
                vec![("arr", array_of_t()), ("value", generic_t())],
                Type::Unit,
            ),
            StdModuleFn::generic(
                "clear",
                vec!["T"],
                vec![("arr", array_of_t())],
                Type::Unit,
            ),
            StdModuleFn::generic(
                "get",
                vec!["T"],
                vec![("arr", array_of_t()), ("index", Type::Int)],
                option_of_t(),
            ),
            // Access functions
            StdModuleFn::generic(
                "first",
                vec!["T"],
                vec![("arr", array_of_t())],
                option_of_t(),
            ),
            StdModuleFn::generic(
                "last",
                vec!["T"],
                vec![("arr", array_of_t())],
                option_of_t(),
            ),
            // Aggregation - these only make sense for numeric types, keep as int for now
            StdModuleFn::new(
                "sum",
                vec![("arr", Type::Array(Box::new(Type::Int)))],
                Type::Int,
            ),
            StdModuleFn::new(
                "min",
                vec![("arr", Type::Array(Box::new(Type::Int)))],
                Type::Option(Box::new(Type::Int)),
            ),
            StdModuleFn::new(
                "max",
                vec![("arr", Type::Array(Box::new(Type::Int)))],
                Type::Option(Box::new(Type::Int)),
            ),
            // Transformation - generic
            StdModuleFn::generic(
                "reversed",
                vec!["T"],
                vec![("arr", array_of_t())],
                array_of_t(),
            ),
            // Slicing - generic
            StdModuleFn::generic(
                "take",
                vec!["T"],
                vec![("arr", array_of_t()), ("n", Type::Int)],
                array_of_t(),
            ),
            StdModuleFn::generic(
                "drop",
                vec!["T"],
                vec![("arr", array_of_t()), ("n", Type::Int)],
                array_of_t(),
            ),
            StdModuleFn::generic(
                "slice",
                vec!["T"],
                vec![("arr", array_of_t()), ("start", Type::Int), ("end", Type::Int)],
                array_of_t(),
            ),
            // Search - generic
            StdModuleFn::generic(
                "index_of",
                vec!["T"],
                vec![("arr", array_of_t()), ("val", generic_t())],
                Type::Option(Box::new(Type::Int)),
            ),
            StdModuleFn::generic(
                "contains",
                vec!["T"],
                vec![("arr", array_of_t()), ("val", generic_t())],
                Type::Bool,
            ),
            // Lambda-based functions (predicate: fn(int) -> bool)
            StdModuleFn::new(
                "any",
                vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    (
                        "predicate",
                        Type::Function(types::FunctionType {
                            params: vec![Type::Int],
                            returns: Box::new(Type::Bool),
                            throws: vec![],
                            is_variadic: false,
                        }),
                    ),
                ],
                Type::Bool,
            ),
            StdModuleFn::new(
                "all",
                vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    (
                        "predicate",
                        Type::Function(types::FunctionType {
                            params: vec![Type::Int],
                            returns: Box::new(Type::Bool),
                            throws: vec![],
                            is_variadic: false,
                        }),
                    ),
                ],
                Type::Bool,
            ),
            StdModuleFn::new(
                "count_if",
                vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    (
                        "predicate",
                        Type::Function(types::FunctionType {
                            params: vec![Type::Int],
                            returns: Box::new(Type::Bool),
                            throws: vec![],
                            is_variadic: false,
                        }),
                    ),
                ],
                Type::Int,
            ),
            StdModuleFn::new(
                "apply",
                vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    (
                        "mapper",
                        Type::Function(types::FunctionType {
                            params: vec![Type::Int],
                            returns: Box::new(Type::Int),
                            throws: vec![],
                            is_variadic: false,
                        }),
                    ),
                ],
                Type::Array(Box::new(Type::Int)),
            ),
            StdModuleFn::new(
                "where",
                vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    (
                        "predicate",
                        Type::Function(types::FunctionType {
                            params: vec![Type::Int],
                            returns: Box::new(Type::Bool),
                            throws: vec![],
                            is_variadic: false,
                        }),
                    ),
                ],
                Type::Array(Box::new(Type::Int)),
            ),
            StdModuleFn::new(
                "find",
                vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    (
                        "predicate",
                        Type::Function(types::FunctionType {
                            params: vec![Type::Int],
                            returns: Box::new(Type::Bool),
                            throws: vec![],
                            is_variadic: false,
                        }),
                    ),
                ],
                Type::Option(Box::new(Type::Int)),
            ),
            StdModuleFn::new(
                "find_index",
                vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    (
                        "predicate",
                        Type::Function(types::FunctionType {
                            params: vec![Type::Int],
                            returns: Box::new(Type::Bool),
                            throws: vec![],
                            is_variadic: false,
                        }),
                    ),
                ],
                Type::Option(Box::new(Type::Int)),
            ),
            StdModuleFn::new(
                "fold",
                vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    ("initial", Type::Int),
                    (
                        "reducer",
                        Type::Function(types::FunctionType {
                            params: vec![Type::Int, Type::Int],
                            returns: Box::new(Type::Int),
                            throws: vec![],
                            is_variadic: false,
                        }),
                    ),
                ],
                Type::Int,
            ),
            StdModuleFn::new(
                "flatten",
                vec![(
                    "arr",
                    Type::Array(Box::new(Type::Array(Box::new(Type::Int)))),
                )],
                Type::Array(Box::new(Type::Int)),
            ),
            StdModuleFn::new(
                "sort",
                vec![("arr", Type::Array(Box::new(Type::Int)))],
                Type::Array(Box::new(Type::Int)),
            ),
            StdModuleFn::new(
                "sort_by",
                vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    (
                        "comparator",
                        Type::Function(types::FunctionType {
                            params: vec![Type::Int, Type::Int],
                            returns: Box::new(Type::Int),
                            throws: vec![],
                            is_variadic: false,
                        }),
                    ),
                ],
                Type::Array(Box::new(Type::Int)),
            ),
            // Mutation operations
            StdModuleFn::new(
                "insert",
                vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    ("index", Type::Int),
                    ("value", Type::Int),
                ],
                Type::Unit,
            ),
            StdModuleFn::new(
                "remove_at",
                vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    ("index", Type::Int),
                ],
                Type::Option(Box::new(Type::Int)),
            ),
            StdModuleFn::new(
                "remove",
                vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    ("value", Type::Int),
                ],
                Type::Bool,
            ),
            StdModuleFn::new(
                "swap",
                vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    ("i", Type::Int),
                    ("j", Type::Int),
                ],
                Type::Unit,
            ),
            // Deduplication
            StdModuleFn::new(
                "unique",
                vec![("arr", Type::Array(Box::new(Type::Int)))],
                Type::Array(Box::new(Type::Int)),
            ),
            StdModuleFn::new(
                "compact",
                vec![("arr", Type::Array(Box::new(Type::Int)))],
                Type::Array(Box::new(Type::Int)),
            ),
            // Backward search
            StdModuleFn::new(
                "last_index_of",
                vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    ("val", Type::Int),
                ],
                Type::Option(Box::new(Type::Int)),
            ),
            StdModuleFn::new(
                "find_last",
                vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    (
                        "predicate",
                        Type::Function(types::FunctionType {
                            params: vec![Type::Int],
                            returns: Box::new(Type::Bool),
                            throws: vec![],
                            is_variadic: false,
                        }),
                    ),
                ],
                Type::Option(Box::new(Type::Int)),
            ),
            StdModuleFn::new(
                "find_last_index",
                vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    (
                        "predicate",
                        Type::Function(types::FunctionType {
                            params: vec![Type::Int],
                            returns: Box::new(Type::Bool),
                            throws: vec![],
                            is_variadic: false,
                        }),
                    ),
                ],
                Type::Option(Box::new(Type::Int)),
            ),
            // Array combination
            StdModuleFn::new(
                "concat",
                vec![
                    ("arr1", Type::Array(Box::new(Type::Int))),
                    ("arr2", Type::Array(Box::new(Type::Int))),
                ],
                Type::Array(Box::new(Type::Int)),
            ),
            StdModuleFn::new(
                "zip",
                vec![
                    ("arr1", Type::Array(Box::new(Type::Int))),
                    ("arr2", Type::Array(Box::new(Type::Int))),
                ],
                Type::Array(Box::new(Type::Array(Box::new(Type::Int)))),
            ),
            StdModuleFn::new(
                "unzip",
                vec![(
                    "arr",
                    Type::Array(Box::new(Type::Array(Box::new(Type::Int)))),
                )],
                Type::Array(Box::new(Type::Array(Box::new(Type::Int)))),
            ),
            // Splitting
            StdModuleFn::new(
                "chunk",
                vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    ("size", Type::Int),
                ],
                Type::Array(Box::new(Type::Array(Box::new(Type::Int)))),
            ),
            StdModuleFn::new(
                "partition",
                vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    (
                        "predicate",
                        Type::Function(types::FunctionType {
                            params: vec![Type::Int],
                            returns: Box::new(Type::Bool),
                            throws: vec![],
                            is_variadic: false,
                        }),
                    ),
                ],
                Type::Array(Box::new(Type::Array(Box::new(Type::Int)))),
            ),
            // Set operations
            StdModuleFn::new(
                "intersect",
                vec![
                    ("arr1", Type::Array(Box::new(Type::Int))),
                    ("arr2", Type::Array(Box::new(Type::Int))),
                ],
                Type::Array(Box::new(Type::Int)),
            ),
            StdModuleFn::new(
                "diff",
                vec![
                    ("arr1", Type::Array(Box::new(Type::Int))),
                    ("arr2", Type::Array(Box::new(Type::Int))),
                ],
                Type::Array(Box::new(Type::Int)),
            ),
            StdModuleFn::new(
                "union",
                vec![
                    ("arr1", Type::Array(Box::new(Type::Int))),
                    ("arr2", Type::Array(Box::new(Type::Int))),
                ],
                Type::Array(Box::new(Type::Int)),
            ),
            // Advanced iteration
            StdModuleFn::new(
                "take_while",
                vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    (
                        "predicate",
                        Type::Function(types::FunctionType {
                            params: vec![Type::Int],
                            returns: Box::new(Type::Bool),
                            throws: vec![],
                            is_variadic: false,
                        }),
                    ),
                ],
                Type::Array(Box::new(Type::Int)),
            ),
            StdModuleFn::new(
                "drop_while",
                vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    (
                        "predicate",
                        Type::Function(types::FunctionType {
                            params: vec![Type::Int],
                            returns: Box::new(Type::Bool),
                            throws: vec![],
                            is_variadic: false,
                        }),
                    ),
                ],
                Type::Array(Box::new(Type::Int)),
            ),
            StdModuleFn::new(
                "reject",
                vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    (
                        "predicate",
                        Type::Function(types::FunctionType {
                            params: vec![Type::Int],
                            returns: Box::new(Type::Bool),
                            throws: vec![],
                            is_variadic: false,
                        }),
                    ),
                ],
                Type::Array(Box::new(Type::Int)),
            ),
            StdModuleFn::new(
                "flat_apply",
                vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    (
                        "mapper",
                        Type::Function(types::FunctionType {
                            params: vec![Type::Int],
                            returns: Box::new(Type::Array(Box::new(Type::Int))),
                            throws: vec![],
                            is_variadic: false,
                        }),
                    ),
                ],
                Type::Array(Box::new(Type::Int)),
            ),
            StdModuleFn::new(
                "scan",
                vec![
                    ("arr", Type::Array(Box::new(Type::Int))),
                    ("initial", Type::Int),
                    (
                        "reducer",
                        Type::Function(types::FunctionType {
                            params: vec![Type::Int, Type::Int],
                            returns: Box::new(Type::Int),
                            throws: vec![],
                            is_variadic: false,
                        }),
                    ),
                ],
                Type::Array(Box::new(Type::Int)),
            ),
            // Random
            StdModuleFn::new(
                "shuffle",
                vec![("arr", Type::Array(Box::new(Type::Int)))],
                Type::Array(Box::new(Type::Int)),
            ),
            StdModuleFn::new(
                "sample",
                vec![("arr", Type::Array(Box::new(Type::Int)))],
                Type::Option(Box::new(Type::Int)),
            ),
            StdModuleFn::new(
                "sample_n",
                vec![("arr", Type::Array(Box::new(Type::Int))), ("n", Type::Int)],
                Type::Array(Box::new(Type::Int)),
            ),
        ]
    }

    fn get_collections_map_functions() -> Vec<StdModuleFn> {
        vec![
            // Basic operations
            StdModuleFn::new(
                "count",
                vec![("m", Type::Map(Box::new(Type::String), Box::new(Type::Int)))],
                Type::Int,
            ),
            StdModuleFn::new(
                "contains_key",
                vec![
                    ("m", Type::Map(Box::new(Type::String), Box::new(Type::Int))),
                    ("key", Type::String),
                ],
                Type::Bool,
            ),
            StdModuleFn::new(
                "remove",
                vec![
                    ("m", Type::Map(Box::new(Type::String), Box::new(Type::Int))),
                    ("key", Type::String),
                ],
                Type::Option(Box::new(Type::Int)),
            ),
            StdModuleFn::new(
                "clear",
                vec![("m", Type::Map(Box::new(Type::String), Box::new(Type::Int)))],
                Type::Unit,
            ),
            // Extraction
            StdModuleFn::new(
                "keys",
                vec![("m", Type::Map(Box::new(Type::String), Box::new(Type::Int)))],
                Type::Array(Box::new(Type::String)),
            ),
            StdModuleFn::new(
                "values",
                vec![("m", Type::Map(Box::new(Type::String), Box::new(Type::Int)))],
                Type::Array(Box::new(Type::Int)),
            ),
            StdModuleFn::new(
                "entries",
                vec![("m", Type::Map(Box::new(Type::String), Box::new(Type::Int)))],
                Type::Array(Box::new(Type::Array(Box::new(Type::Int)))),
            ),
            // Lookup
            StdModuleFn::new(
                "first_key",
                vec![("m", Type::Map(Box::new(Type::String), Box::new(Type::Int)))],
                Type::Option(Box::new(Type::String)),
            ),
            StdModuleFn::new(
                "first_value",
                vec![("m", Type::Map(Box::new(Type::String), Box::new(Type::Int)))],
                Type::Option(Box::new(Type::Int)),
            ),
            // Lambda-based functions
            StdModuleFn::new(
                "any",
                vec![
                    ("m", Type::Map(Box::new(Type::String), Box::new(Type::Int))),
                    (
                        "predicate",
                        Type::Function(types::FunctionType {
                            params: vec![Type::String, Type::Int],
                            returns: Box::new(Type::Bool),
                            throws: vec![],
                            is_variadic: false,
                        }),
                    ),
                ],
                Type::Bool,
            ),
            StdModuleFn::new(
                "all",
                vec![
                    ("m", Type::Map(Box::new(Type::String), Box::new(Type::Int))),
                    (
                        "predicate",
                        Type::Function(types::FunctionType {
                            params: vec![Type::String, Type::Int],
                            returns: Box::new(Type::Bool),
                            throws: vec![],
                            is_variadic: false,
                        }),
                    ),
                ],
                Type::Bool,
            ),
            StdModuleFn::new(
                "count_if",
                vec![
                    ("m", Type::Map(Box::new(Type::String), Box::new(Type::Int))),
                    (
                        "predicate",
                        Type::Function(types::FunctionType {
                            params: vec![Type::String, Type::Int],
                            returns: Box::new(Type::Bool),
                            throws: vec![],
                            is_variadic: false,
                        }),
                    ),
                ],
                Type::Int,
            ),
            StdModuleFn::new(
                "fold",
                vec![
                    ("m", Type::Map(Box::new(Type::String), Box::new(Type::Int))),
                    ("initial", Type::Int),
                    (
                        "reducer",
                        Type::Function(types::FunctionType {
                            params: vec![Type::Int, Type::String, Type::Int],
                            returns: Box::new(Type::Int),
                            throws: vec![],
                            is_variadic: false,
                        }),
                    ),
                ],
                Type::Int,
            ),
            // Transformation
            StdModuleFn::new(
                "transform",
                vec![
                    ("m", Type::Map(Box::new(Type::String), Box::new(Type::Int))),
                    (
                        "mapper",
                        Type::Function(types::FunctionType {
                            params: vec![Type::Int],
                            returns: Box::new(Type::Int),
                            throws: vec![],
                            is_variadic: false,
                        }),
                    ),
                ],
                Type::Map(Box::new(Type::String), Box::new(Type::Int)),
            ),
            // Filtering
            StdModuleFn::new(
                "where",
                vec![
                    ("m", Type::Map(Box::new(Type::String), Box::new(Type::Int))),
                    (
                        "predicate",
                        Type::Function(types::FunctionType {
                            params: vec![Type::String, Type::Int],
                            returns: Box::new(Type::Bool),
                            throws: vec![],
                            is_variadic: false,
                        }),
                    ),
                ],
                Type::Map(Box::new(Type::String), Box::new(Type::Int)),
            ),
            StdModuleFn::new(
                "reject",
                vec![
                    ("m", Type::Map(Box::new(Type::String), Box::new(Type::Int))),
                    (
                        "predicate",
                        Type::Function(types::FunctionType {
                            params: vec![Type::String, Type::Int],
                            returns: Box::new(Type::Bool),
                            throws: vec![],
                            is_variadic: false,
                        }),
                    ),
                ],
                Type::Map(Box::new(Type::String), Box::new(Type::Int)),
            ),
            // Combining
            StdModuleFn::new(
                "merge",
                vec![
                    ("a", Type::Map(Box::new(Type::String), Box::new(Type::Int))),
                    ("b", Type::Map(Box::new(Type::String), Box::new(Type::Int))),
                ],
                Type::Map(Box::new(Type::String), Box::new(Type::Int)),
            ),
            StdModuleFn::new(
                "defaults",
                vec![
                    ("m", Type::Map(Box::new(Type::String), Box::new(Type::Int))),
                    (
                        "defs",
                        Type::Map(Box::new(Type::String), Box::new(Type::Int)),
                    ),
                ],
                Type::Map(Box::new(Type::String), Box::new(Type::Int)),
            ),
            StdModuleFn::new(
                "intersect",
                vec![
                    ("a", Type::Map(Box::new(Type::String), Box::new(Type::Int))),
                    ("b", Type::Map(Box::new(Type::String), Box::new(Type::Int))),
                ],
                Type::Map(Box::new(Type::String), Box::new(Type::Int)),
            ),
            StdModuleFn::new(
                "diff",
                vec![
                    ("a", Type::Map(Box::new(Type::String), Box::new(Type::Int))),
                    ("b", Type::Map(Box::new(Type::String), Box::new(Type::Int))),
                ],
                Type::Map(Box::new(Type::String), Box::new(Type::Int)),
            ),
            // Conversion
            StdModuleFn::new(
                "invert",
                vec![("m", Type::Map(Box::new(Type::String), Box::new(Type::Int)))],
                Type::Map(Box::new(Type::Int), Box::new(Type::String)),
            ),
            StdModuleFn::new(
                "from_arrays",
                vec![
                    ("keys", Type::Array(Box::new(Type::String))),
                    ("values", Type::Array(Box::new(Type::Int))),
                ],
                Type::Map(Box::new(Type::String), Box::new(Type::Int)),
            ),
            StdModuleFn::new(
                "from_entries",
                vec![(
                    "pairs",
                    Type::Array(Box::new(Type::Array(Box::new(Type::Int)))),
                )],
                Type::Map(Box::new(Type::String), Box::new(Type::Int)),
            ),
        ]
    }

    fn get_fs_functions() -> Vec<StdModuleFn> {
        vec![
            // File reading
            StdModuleFn::throwing(
                "read",
                vec![("path", Type::String)],
                Type::String,
                vec!["IOError", "PermissionError"],
            ),
            StdModuleFn::throwing(
                "read_bytes",
                vec![("path", Type::String)],
                Type::Bytes,
                vec!["IOError", "PermissionError"],
            ),
            // File writing
            StdModuleFn::throwing(
                "write",
                vec![("path", Type::String), ("content", Type::String)],
                Type::Unit,
                vec!["IOError", "PermissionError"],
            ),
            StdModuleFn::throwing(
                "write_bytes",
                vec![("path", Type::String), ("content", Type::Bytes)],
                Type::Unit,
                vec!["IOError", "PermissionError"],
            ),
            StdModuleFn::throwing(
                "append",
                vec![("path", Type::String), ("content", Type::String)],
                Type::Unit,
                vec!["IOError", "PermissionError"],
            ),
            StdModuleFn::throwing(
                "append_bytes",
                vec![("path", Type::String), ("content", Type::Bytes)],
                Type::Unit,
                vec!["IOError", "PermissionError"],
            ),
            // Existence checks (non-throwing)
            StdModuleFn::new("exists", vec![("path", Type::String)], Type::Bool),
            StdModuleFn::new("is_file", vec![("path", Type::String)], Type::Bool),
            StdModuleFn::new("is_dir", vec![("path", Type::String)], Type::Bool),
            // Directory operations
            StdModuleFn::throwing(
                "list_dir",
                vec![("path", Type::String)],
                Type::Array(Box::new(Type::String)),
                vec!["IOError", "PermissionError"],
            ),
            StdModuleFn::throwing(
                "mkdir",
                vec![("path", Type::String)],
                Type::Unit,
                vec!["IOError", "PermissionError"],
            ),
            StdModuleFn::throwing(
                "mkdir_all",
                vec![("path", Type::String)],
                Type::Unit,
                vec!["IOError", "PermissionError"],
            ),
            // Delete operations
            StdModuleFn::throwing(
                "remove",
                vec![("path", Type::String)],
                Type::Unit,
                vec!["IOError", "PermissionError"],
            ),
            StdModuleFn::throwing(
                "remove_all",
                vec![("path", Type::String)],
                Type::Unit,
                vec!["IOError", "PermissionError"],
            ),
            // Path operations (non-throwing)
            StdModuleFn::new(
                "join",
                vec![("parts", Type::Array(Box::new(Type::String)))],
                Type::String,
            ),
            StdModuleFn::new("dirname", vec![("path", Type::String)], Type::String),
            StdModuleFn::new("basename", vec![("path", Type::String)], Type::String),
            StdModuleFn::new("extension", vec![("path", Type::String)], Type::String),
            StdModuleFn::throwing(
                "absolute",
                vec![("path", Type::String)],
                Type::String,
                vec!["IOError"],
            ),
            // Metadata
            StdModuleFn::throwing(
                "size",
                vec![("path", Type::String)],
                Type::Int,
                vec!["IOError"],
            ),
            StdModuleFn::throwing(
                "modified",
                vec![("path", Type::String)],
                Type::Int,
                vec!["IOError"],
            ),
            // Copy/rename
            StdModuleFn::throwing(
                "copy",
                vec![("src", Type::String), ("dst", Type::String)],
                Type::Unit,
                vec!["IOError"],
            ),
            StdModuleFn::throwing(
                "rename",
                vec![("src", Type::String), ("dst", Type::String)],
                Type::Unit,
                vec!["IOError"],
            ),
            // Memory-mapped file operations
            StdModuleFn::throwing(
                "mmap_open",
                vec![("path", Type::String), ("writable", Type::Bool)],
                Type::Int,
                vec!["IOError"],
            ),
            StdModuleFn::throwing(
                "mmap_len",
                vec![("handle", Type::Int)],
                Type::Int,
                vec!["IOError"],
            ),
            StdModuleFn::throwing(
                "mmap_read_byte",
                vec![("handle", Type::Int), ("offset", Type::Int)],
                Type::Int,
                vec!["IOError"],
            ),
            StdModuleFn::throwing(
                "mmap_write_byte",
                vec![
                    ("handle", Type::Int),
                    ("offset", Type::Int),
                    ("value", Type::Int),
                ],
                Type::Unit,
                vec!["IOError"],
            ),
            StdModuleFn::throwing(
                "mmap_read",
                vec![
                    ("handle", Type::Int),
                    ("offset", Type::Int),
                    ("len", Type::Int),
                ],
                Type::Bytes,
                vec!["IOError"],
            ),
            StdModuleFn::throwing(
                "mmap_write",
                vec![
                    ("handle", Type::Int),
                    ("offset", Type::Int),
                    ("data", Type::Bytes),
                ],
                Type::Unit,
                vec!["IOError"],
            ),
            StdModuleFn::throwing(
                "mmap_flush",
                vec![("handle", Type::Int)],
                Type::Unit,
                vec!["IOError"],
            ),
            StdModuleFn::throwing(
                "mmap_close",
                vec![("handle", Type::Int)],
                Type::Unit,
                vec!["IOError"],
            ),
            // File handle operations
            StdModuleFn::throwing(
                "file_open",
                vec![("path", Type::String), ("mode", Type::String)],
                Type::Int,
                vec!["IOError"],
            ),
            StdModuleFn::throwing(
                "file_close",
                vec![("handle", Type::Int)],
                Type::Unit,
                vec!["IOError"],
            ),
            StdModuleFn::throwing(
                "file_read",
                vec![("handle", Type::Int), ("count", Type::Int)],
                Type::String,
                vec!["IOError"],
            ),
            StdModuleFn::throwing(
                "file_read_line",
                vec![("handle", Type::Int)],
                Type::String,
                vec!["IOError"],
            ),
            StdModuleFn::throwing(
                "file_read_all",
                vec![("handle", Type::Int)],
                Type::String,
                vec!["IOError"],
            ),
            StdModuleFn::throwing(
                "file_write",
                vec![("handle", Type::Int), ("content", Type::String)],
                Type::Int,
                vec!["IOError"],
            ),
            StdModuleFn::throwing(
                "file_write_line",
                vec![("handle", Type::Int), ("content", Type::String)],
                Type::Int,
                vec!["IOError"],
            ),
            StdModuleFn::throwing(
                "file_flush",
                vec![("handle", Type::Int)],
                Type::Unit,
                vec!["IOError"],
            ),
            StdModuleFn::throwing(
                "file_seek",
                vec![
                    ("handle", Type::Int),
                    ("offset", Type::Int),
                    ("whence", Type::Int),
                ],
                Type::Int,
                vec!["IOError"],
            ),
            StdModuleFn::throwing(
                "file_tell",
                vec![("handle", Type::Int)],
                Type::Int,
                vec!["IOError"],
            ),
            StdModuleFn::throwing(
                "file_eof",
                vec![("handle", Type::Int)],
                Type::Bool,
                vec!["IOError"],
            ),
            StdModuleFn::throwing(
                "file_size",
                vec![("handle", Type::Int)],
                Type::Int,
                vec!["IOError"],
            ),
            // Working directory operations
            StdModuleFn::throwing("getwd", vec![], Type::String, vec!["IOError"]),
            StdModuleFn::throwing(
                "chdir",
                vec![("path", Type::String)],
                Type::Unit,
                vec!["IOError", "PermissionError"],
            ),
            // Temp file/directory creation
            StdModuleFn::throwing(
                "create_temp",
                vec![("prefix", Type::String)],
                Type::String,
                vec!["IOError", "PermissionError"],
            ),
            StdModuleFn::throwing(
                "mkdir_temp",
                vec![("prefix", Type::String)],
                Type::String,
                vec!["IOError", "PermissionError"],
            ),
            // Permission and size operations
            StdModuleFn::throwing(
                "chmod",
                vec![("path", Type::String), ("mode", Type::Int)],
                Type::Unit,
                vec!["IOError", "PermissionError"],
            ),
            StdModuleFn::throwing(
                "truncate",
                vec![("path", Type::String), ("size", Type::Int)],
                Type::Unit,
                vec!["IOError", "PermissionError"],
            ),
            // File metadata (stat)
            // Returns [size, mode, modified, created, is_dir, is_file, is_symlink]
            StdModuleFn::throwing(
                "stat",
                vec![("path", Type::String)],
                Type::Array(Box::new(Type::Int)),
                vec!["IOError", "PermissionError"],
            ),
            // Link operations
            StdModuleFn::throwing(
                "symlink",
                vec![("target", Type::String), ("link_path", Type::String)],
                Type::Unit,
                vec!["IOError", "PermissionError"],
            ),
            StdModuleFn::throwing(
                "readlink",
                vec![("path", Type::String)],
                Type::String,
                vec!["IOError"],
            ),
            StdModuleFn::throwing(
                "lstat",
                vec![("path", Type::String)],
                Type::Array(Box::new(Type::Int)),
                vec!["IOError", "PermissionError"],
            ),
            StdModuleFn::throwing(
                "link",
                vec![("src", Type::String), ("dst", Type::String)],
                Type::Unit,
                vec!["IOError", "PermissionError"],
            ),
            // Timestamps
            StdModuleFn::throwing(
                "chtimes",
                vec![
                    ("path", Type::String),
                    ("atime_ms", Type::Int),
                    ("mtime_ms", Type::Int),
                ],
                Type::Unit,
                vec!["IOError", "PermissionError"],
            ),
            // Ownership
            StdModuleFn::throwing(
                "chown",
                vec![
                    ("path", Type::String),
                    ("uid", Type::Int),
                    ("gid", Type::Int),
                ],
                Type::Unit,
                vec!["IOError", "PermissionError"],
            ),
            StdModuleFn::throwing(
                "lchown",
                vec![
                    ("path", Type::String),
                    ("uid", Type::Int),
                    ("gid", Type::Int),
                ],
                Type::Unit,
                vec!["IOError", "PermissionError"],
            ),
            // File comparison
            StdModuleFn::throwing(
                "same_file",
                vec![("path1", Type::String), ("path2", Type::String)],
                Type::Bool,
                vec!["IOError"],
            ),
            // Additional file handle operations
            StdModuleFn::throwing(
                "file_read_at",
                vec![
                    ("handle", Type::Int),
                    ("buf_size", Type::Int),
                    ("offset", Type::Int),
                ],
                Type::String,
                vec!["IOError"],
            ),
            StdModuleFn::throwing(
                "file_write_at",
                vec![
                    ("handle", Type::Int),
                    ("content", Type::String),
                    ("offset", Type::Int),
                ],
                Type::Int,
                vec!["IOError"],
            ),
            StdModuleFn::throwing(
                "file_name",
                vec![("handle", Type::Int)],
                Type::String,
                vec!["IOError"],
            ),
            StdModuleFn::throwing(
                "file_stat",
                vec![("handle", Type::Int)],
                Type::Array(Box::new(Type::Int)),
                vec!["IOError"],
            ),
            StdModuleFn::throwing(
                "file_truncate",
                vec![("handle", Type::Int), ("size", Type::Int)],
                Type::Unit,
                vec!["IOError"],
            ),
            StdModuleFn::throwing(
                "file_chmod",
                vec![("handle", Type::Int), ("mode", Type::Int)],
                Type::Unit,
                vec!["IOError", "PermissionError"],
            ),
            StdModuleFn::throwing(
                "file_chown",
                vec![
                    ("handle", Type::Int),
                    ("uid", Type::Int),
                    ("gid", Type::Int),
                ],
                Type::Unit,
                vec!["IOError", "PermissionError"],
            ),
        ]
    }

    fn get_encoding_utf8_functions() -> Vec<StdModuleFn> {
        vec![
            StdModuleFn::new("encode", vec![("s", Type::String)], Type::Bytes),
            StdModuleFn::throwing(
                "decode",
                vec![("data", Type::Bytes)],
                Type::String,
                vec!["DecodeError"],
            ),
            StdModuleFn::new("is_valid", vec![("data", Type::Bytes)], Type::Bool),
        ]
    }

    fn get_encoding_hex_functions() -> Vec<StdModuleFn> {
        vec![
            StdModuleFn::new("encode", vec![("data", Type::Bytes)], Type::String),
            StdModuleFn::throwing(
                "decode",
                vec![("s", Type::String)],
                Type::Bytes,
                vec!["DecodeError"],
            ),
        ]
    }

    fn get_encoding_base64_functions() -> Vec<StdModuleFn> {
        vec![
            StdModuleFn::new("encode", vec![("data", Type::Bytes)], Type::String),
            StdModuleFn::throwing(
                "decode",
                vec![("s", Type::String)],
                Type::Bytes,
                vec!["DecodeError"],
            ),
        ]
    }

    fn get_encoding_url_functions() -> Vec<StdModuleFn> {
        vec![
            StdModuleFn::new("encode", vec![("s", Type::String)], Type::String),
            StdModuleFn::throwing(
                "decode",
                vec![("s", Type::String)],
                Type::String,
                vec!["DecodeError"],
            ),
        ]
    }

    fn get_encoding_json_functions() -> Vec<StdModuleFn> {
        vec![
            StdModuleFn::throwing(
                "decode",
                vec![("s", Type::String)],
                Type::Json,
                vec!["DecodeError"],
            ),
            StdModuleFn::new("encode", vec![("value", Type::Json)], Type::String),
            StdModuleFn::new("encode_pretty", vec![("value", Type::Json)], Type::String),
            StdModuleFn::new(
                "exists",
                vec![("data", Type::Json), ("key", Type::String)],
                Type::Bool,
            ),
            StdModuleFn::throwing(
                "path",
                vec![("data", Type::Json), ("jq_path", Type::String)],
                Type::Json,
                vec!["PathError"],
            ),
            StdModuleFn::new(
                "keys",
                vec![("data", Type::Json)],
                Type::Array(Box::new(Type::String)),
            ),
            StdModuleFn::new("count", vec![("data", Type::Json)], Type::Int),
            StdModuleFn::new("get_type", vec![("data", Type::Json)], Type::Int),
            StdModuleFn::new("type_name", vec![("data", Type::Json)], Type::String),
            StdModuleFn::new("is_null", vec![("data", Type::Json)], Type::Bool),
        ]
    }

    fn get_encoding_toml_functions() -> Vec<StdModuleFn> {
        vec![
            StdModuleFn::throwing(
                "decode",
                vec![("s", Type::String)],
                Type::Json,
                vec!["DecodeError"],
            ),
            StdModuleFn::throwing(
                "encode",
                vec![("value", Type::Json)],
                Type::String,
                vec!["EncodeError"],
            ),
            StdModuleFn::throwing(
                "encode_pretty",
                vec![("value", Type::Json)],
                Type::String,
                vec!["EncodeError"],
            ),
        ]
    }

    fn get_encoding_yaml_functions() -> Vec<StdModuleFn> {
        vec![
            StdModuleFn::throwing(
                "decode",
                vec![("s", Type::String)],
                Type::Json,
                vec!["DecodeError"],
            ),
            StdModuleFn::throwing(
                "encode",
                vec![("value", Type::Json)],
                Type::String,
                vec!["EncodeError"],
            ),
        ]
    }

    fn get_net_tcp_server_functions() -> Vec<StdModuleFn> {
        vec![
            StdModuleFn::throwing(
                "listen",
                vec![("address", Type::String)],
                Type::Int,
                vec!["NetworkError"],
            ),
            StdModuleFn::throwing(
                "accept",
                vec![("listener", Type::Int)],
                Type::Int,
                vec!["NetworkError"],
            ),
            StdModuleFn::new("close", vec![("listener", Type::Int)], Type::Unit),
            StdModuleFn::new("local_addr", vec![("listener", Type::Int)], Type::String),
        ]
    }

    fn get_net_tcp_client_functions() -> Vec<StdModuleFn> {
        vec![
            StdModuleFn::throwing(
                "connect",
                vec![("address", Type::String)],
                Type::Int,
                vec!["NetworkError", "TimeoutError"],
            ),
            StdModuleFn::throwing(
                "read",
                vec![("socket", Type::Int), ("size", Type::Int)],
                Type::Bytes,
                vec!["NetworkError"],
            ),
            StdModuleFn::throwing(
                "read_all",
                vec![("socket", Type::Int)],
                Type::Bytes,
                vec!["NetworkError"],
            ),
            StdModuleFn::throwing(
                "write",
                vec![("socket", Type::Int), ("data", Type::Bytes)],
                Type::Unit,
                vec!["NetworkError"],
            ),
            StdModuleFn::new("close", vec![("socket", Type::Int)], Type::Unit),
            StdModuleFn::new(
                "set_timeout",
                vec![("socket", Type::Int), ("ms", Type::Int)],
                Type::Unit,
            ),
            StdModuleFn::new("peer_addr", vec![("socket", Type::Int)], Type::String),
        ]
    }

    fn get_net_udp_functions() -> Vec<StdModuleFn> {
        vec![
            StdModuleFn::throwing(
                "bind",
                vec![("address", Type::String)],
                Type::Int,
                vec!["NetworkError"],
            ),
            StdModuleFn::throwing(
                "send",
                vec![
                    ("socket", Type::Int),
                    ("data", Type::Bytes),
                    ("address", Type::String),
                ],
                Type::Unit,
                vec!["NetworkError"],
            ),
            StdModuleFn::throwing(
                "receive",
                vec![("socket", Type::Int), ("size", Type::Int)],
                Type::Bytes,
                vec!["NetworkError"],
            ),
            StdModuleFn::new("close", vec![("socket", Type::Int)], Type::Unit),
            StdModuleFn::new("local_addr", vec![("socket", Type::Int)], Type::String),
        ]
    }

    fn get_net_http_client_functions() -> Vec<StdModuleFn> {
        // Headers type: option<map<string, string>>
        let headers_type = Type::Option(Box::new(Type::Map(
            Box::new(Type::String),
            Box::new(Type::String),
        )));
        vec![
            StdModuleFn::throwing(
                "get",
                vec![("url", Type::String), ("headers", headers_type.clone())],
                Type::Int,
                vec!["NetworkError", "TimeoutError"],
            ),
            StdModuleFn::throwing(
                "post",
                vec![
                    ("url", Type::String),
                    ("body", Type::Bytes),
                    ("headers", headers_type.clone()),
                ],
                Type::Int,
                vec!["NetworkError", "TimeoutError"],
            ),
            StdModuleFn::throwing(
                "put",
                vec![
                    ("url", Type::String),
                    ("body", Type::Bytes),
                    ("headers", headers_type.clone()),
                ],
                Type::Int,
                vec!["NetworkError", "TimeoutError"],
            ),
            StdModuleFn::throwing(
                "patch",
                vec![
                    ("url", Type::String),
                    ("body", Type::Bytes),
                    ("headers", headers_type.clone()),
                ],
                Type::Int,
                vec!["NetworkError", "TimeoutError"],
            ),
            StdModuleFn::throwing(
                "delete",
                vec![("url", Type::String), ("headers", headers_type)],
                Type::Int,
                vec!["NetworkError", "TimeoutError"],
            ),
            StdModuleFn::new("set_timeout", vec![("ms", Type::Int)], Type::Unit),
            // Response accessors
            StdModuleFn::new("status", vec![("response", Type::Int)], Type::Int),
            StdModuleFn::new("body", vec![("response", Type::Int)], Type::Bytes),
        ]
    }

    fn get_net_http_server_functions() -> Vec<StdModuleFn> {
        vec![
            StdModuleFn::new("open_router", vec![], Type::Int),
            StdModuleFn::new(
                "get",
                vec![
                    ("router", Type::Int),
                    ("pattern", Type::String),
                    ("handler", Type::Int),
                ],
                Type::Unit,
            ),
            StdModuleFn::new(
                "post",
                vec![
                    ("router", Type::Int),
                    ("pattern", Type::String),
                    ("handler", Type::Int),
                ],
                Type::Unit,
            ),
            StdModuleFn::new(
                "put",
                vec![
                    ("router", Type::Int),
                    ("pattern", Type::String),
                    ("handler", Type::Int),
                ],
                Type::Unit,
            ),
            StdModuleFn::new(
                "patch",
                vec![
                    ("router", Type::Int),
                    ("pattern", Type::String),
                    ("handler", Type::Int),
                ],
                Type::Unit,
            ),
            StdModuleFn::new(
                "delete",
                vec![
                    ("router", Type::Int),
                    ("pattern", Type::String),
                    ("handler", Type::Int),
                ],
                Type::Unit,
            ),
            StdModuleFn::new(
                "with",
                vec![("router", Type::Int), ("middleware", Type::Int)],
                Type::Unit,
            ),
            StdModuleFn::new(
                "group",
                vec![("router", Type::Int), ("prefix", Type::String)],
                Type::Int,
            ),
            StdModuleFn::new(
                "mount",
                vec![
                    ("router", Type::Int),
                    ("prefix", Type::String),
                    ("sub_router", Type::Int),
                ],
                Type::Unit,
            ),
            StdModuleFn::throwing(
                "serve",
                vec![("address", Type::String), ("router", Type::Int)],
                Type::Unit,
                vec!["NetworkError"],
            ),
        ]
    }

    fn get_net_http_middleware_functions() -> Vec<StdModuleFn> {
        vec![
            StdModuleFn::new("logger", vec![], Type::Int),
            StdModuleFn::new("timeout", vec![("ms", Type::Int)], Type::Int),
            StdModuleFn::new("recover", vec![], Type::Int),
            StdModuleFn::new(
                "cors",
                vec![("origins", Type::Array(Box::new(Type::String)))],
                Type::Int,
            ),
            StdModuleFn::new(
                "rate_limit",
                vec![("requests_per_second", Type::Int)],
                Type::Int,
            ),
            StdModuleFn::new("compress", vec![], Type::Int),
            StdModuleFn::new("request_id", vec![], Type::Int),
        ]
    }

    fn get_std_module_functions_impl(module: &str) -> Option<Vec<StdModuleFn>> {
        match module {
            "random" => Some(vec![
                StdModuleFn::new(
                    "random",
                    vec![("min", Type::Int), ("max", Type::Int)],
                    Type::Int,
                ),
                StdModuleFn::new("random_float", vec![], Type::Float),
            ]),
            "io" => Some(vec![
                StdModuleFn::new("read_line", vec![], Type::String),
                StdModuleFn::new("read_key", vec![], Type::Int),
                StdModuleFn::new("clear_screen", vec![], Type::Unit),
                StdModuleFn::new(
                    "set_cursor",
                    vec![("x", Type::Int), ("y", Type::Int)],
                    Type::Unit,
                ),
                StdModuleFn::new("hide_cursor", vec![], Type::Unit),
                StdModuleFn::new("show_cursor", vec![], Type::Unit),
                StdModuleFn::new("terminal_width", vec![], Type::Int),
                StdModuleFn::new("terminal_height", vec![], Type::Int),
            ]),
            "threads" => Some(vec![
                StdModuleFn::new("sleep", vec![("ms", Type::Int)], Type::Unit),
                StdModuleFn::new("join", vec![], Type::Unit),
                StdModuleFn::generic(
                    "open_channel",
                    vec!["T"],
                    vec![("capacity", Type::Int)],
                    Type::Channel(Box::new(Type::Generic(lasso::Spur::default(), vec![]))),
                ),
                StdModuleFn::generic(
                    "send",
                    vec!["T"],
                    vec![
                        (
                            "ch",
                            Type::Channel(Box::new(Type::Generic(lasso::Spur::default(), vec![]))),
                        ),
                        ("value", Type::Generic(lasso::Spur::default(), vec![])),
                    ],
                    Type::Int,
                ),
                StdModuleFn::generic(
                    "receive",
                    vec!["T"],
                    vec![(
                        "ch",
                        Type::Channel(Box::new(Type::Generic(lasso::Spur::default(), vec![]))),
                    )],
                    Type::Option(Box::new(Type::Generic(lasso::Spur::default(), vec![]))),
                ),
                StdModuleFn::generic(
                    "close",
                    vec!["T"],
                    vec![(
                        "ch",
                        Type::Channel(Box::new(Type::Generic(lasso::Spur::default(), vec![]))),
                    )],
                    Type::Unit,
                ),
                StdModuleFn::generic(
                    "with_mutex",
                    vec!["T"],
                    vec![("value", Type::Generic(lasso::Spur::default(), vec![]))],
                    Type::Mutex(Box::new(Type::Generic(lasso::Spur::default(), vec![]))),
                ),
                StdModuleFn::generic(
                    "with_rwlock",
                    vec!["T"],
                    vec![("value", Type::Generic(lasso::Spur::default(), vec![]))],
                    Type::Rwlock(Box::new(Type::Generic(lasso::Spur::default(), vec![]))),
                ),
                StdModuleFn::generic(
                    "with_atomic",
                    vec!["T"],
                    vec![("value", Type::Generic(lasso::Spur::default(), vec![]))],
                    Type::Atomic(Box::new(Type::Generic(lasso::Spur::default(), vec![]))),
                ),
                StdModuleFn::generic(
                    "atomic_load",
                    vec!["T"],
                    vec![("a", Type::Atomic(Box::new(Type::Generic(lasso::Spur::default(), vec![]))))],
                    Type::Generic(lasso::Spur::default(), vec![]),
                ),
                StdModuleFn::generic(
                    "atomic_store",
                    vec!["T"],
                    vec![
                        ("a", Type::Atomic(Box::new(Type::Generic(lasso::Spur::default(), vec![])))),
                        ("value", Type::Generic(lasso::Spur::default(), vec![])),
                    ],
                    Type::Unit,
                ),
                StdModuleFn::generic(
                    "atomic_add",
                    vec!["T"],
                    vec![
                        ("a", Type::Atomic(Box::new(Type::Generic(lasso::Spur::default(), vec![])))),
                        ("value", Type::Generic(lasso::Spur::default(), vec![])),
                    ],
                    Type::Generic(lasso::Spur::default(), vec![]),
                ),
                StdModuleFn::generic(
                    "atomic_sub",
                    vec!["T"],
                    vec![
                        ("a", Type::Atomic(Box::new(Type::Generic(lasso::Spur::default(), vec![])))),
                        ("value", Type::Generic(lasso::Spur::default(), vec![])),
                    ],
                    Type::Generic(lasso::Spur::default(), vec![]),
                ),
                StdModuleFn::generic(
                    "atomic_inc",
                    vec!["T"],
                    vec![("a", Type::Atomic(Box::new(Type::Generic(lasso::Spur::default(), vec![]))))],
                    Type::Generic(lasso::Spur::default(), vec![]),
                ),
                StdModuleFn::generic(
                    "atomic_dec",
                    vec!["T"],
                    vec![("a", Type::Atomic(Box::new(Type::Generic(lasso::Spur::default(), vec![]))))],
                    Type::Generic(lasso::Spur::default(), vec![]),
                ),
                StdModuleFn::generic(
                    "atomic_cas",
                    vec!["T"],
                    vec![
                        ("a", Type::Atomic(Box::new(Type::Generic(lasso::Spur::default(), vec![])))),
                        ("expected", Type::Generic(lasso::Spur::default(), vec![])),
                        ("new", Type::Generic(lasso::Spur::default(), vec![])),
                    ],
                    Type::Bool,
                ),
                StdModuleFn::generic(
                    "atomic_swap",
                    vec!["T"],
                    vec![
                        ("a", Type::Atomic(Box::new(Type::Generic(lasso::Spur::default(), vec![])))),
                        ("value", Type::Generic(lasso::Spur::default(), vec![])),
                    ],
                    Type::Generic(lasso::Spur::default(), vec![]),
                ),
                StdModuleFn::generic(
                    "atomic_and",
                    vec!["T"],
                    vec![
                        ("a", Type::Atomic(Box::new(Type::Generic(lasso::Spur::default(), vec![])))),
                        ("value", Type::Generic(lasso::Spur::default(), vec![])),
                    ],
                    Type::Generic(lasso::Spur::default(), vec![]),
                ),
                StdModuleFn::generic(
                    "atomic_or",
                    vec!["T"],
                    vec![
                        ("a", Type::Atomic(Box::new(Type::Generic(lasso::Spur::default(), vec![])))),
                        ("value", Type::Generic(lasso::Spur::default(), vec![])),
                    ],
                    Type::Generic(lasso::Spur::default(), vec![]),
                ),
                StdModuleFn::generic(
                    "atomic_xor",
                    vec!["T"],
                    vec![
                        ("a", Type::Atomic(Box::new(Type::Generic(lasso::Spur::default(), vec![])))),
                        ("value", Type::Generic(lasso::Spur::default(), vec![])),
                    ],
                    Type::Generic(lasso::Spur::default(), vec![]),
                ),
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
                StdModuleFn::new(
                    "format_date",
                    vec![("timestamp_ms", Type::Int), ("fmt", Type::String)],
                    Type::String,
                ),
            ]),
            "metrics" => Some(vec![
                StdModuleFn::new("perf_now", vec![], Type::Int),
                StdModuleFn::new("elapsed_ms", vec![("start_ns", Type::Int)], Type::Int),
                StdModuleFn::new("elapsed_us", vec![("start_ns", Type::Int)], Type::Int),
                StdModuleFn::new("elapsed_ns", vec![("start_ns", Type::Int)], Type::Int),
            ]),
            "strings" => Some(vec![
                StdModuleFn::new("len", vec![("s", Type::String)], Type::Int),
                StdModuleFn::new(
                    "char_at",
                    vec![("s", Type::String), ("index", Type::Int)],
                    Type::Int,
                ),
                StdModuleFn::new("upper", vec![("s", Type::String)], Type::String),
                StdModuleFn::new("lower", vec![("s", Type::String)], Type::String),
                StdModuleFn::new(
                    "split",
                    vec![("s", Type::String), ("delim", Type::String)],
                    Type::Array(Box::new(Type::String)),
                ),
                StdModuleFn::new(
                    "concat",
                    vec![
                        ("arr", Type::Array(Box::new(Type::String))),
                        ("delim", Type::String),
                    ],
                    Type::String,
                ),
                StdModuleFn::new(
                    "has",
                    vec![("s", Type::String), ("substr", Type::String)],
                    Type::Bool,
                ),
                StdModuleFn::new(
                    "starts_with",
                    vec![("s", Type::String), ("prefix", Type::String)],
                    Type::Bool,
                ),
                StdModuleFn::new(
                    "ends_with",
                    vec![("s", Type::String), ("suffix", Type::String)],
                    Type::Bool,
                ),
                StdModuleFn::new(
                    "replace",
                    vec![
                        ("s", Type::String),
                        ("old", Type::String),
                        ("new", Type::String),
                    ],
                    Type::String,
                ),
                StdModuleFn::new(
                    "replace_all",
                    vec![
                        ("s", Type::String),
                        ("old", Type::String),
                        ("new", Type::String),
                    ],
                    Type::String,
                ),
                StdModuleFn::new("ltrim", vec![("s", Type::String)], Type::String),
                StdModuleFn::new("rtrim", vec![("s", Type::String)], Type::String),
                StdModuleFn::new(
                    "substr",
                    vec![
                        ("s", Type::String),
                        ("start", Type::Int),
                        ("end", Type::Int),
                    ],
                    Type::String,
                ),
                StdModuleFn::new(
                    "lpad",
                    vec![
                        ("s", Type::String),
                        ("len", Type::Int),
                        ("char", Type::String),
                    ],
                    Type::String,
                ),
                StdModuleFn::new(
                    "rpad",
                    vec![
                        ("s", Type::String),
                        ("len", Type::Int),
                        ("char", Type::String),
                    ],
                    Type::String,
                ),
                StdModuleFn::new(
                    "repeat",
                    vec![("s", Type::String), ("n", Type::Int)],
                    Type::String,
                ),
                StdModuleFn::new(
                    "lines",
                    vec![("s", Type::String)],
                    Type::Array(Box::new(Type::String)),
                ),
                StdModuleFn::new(
                    "chars",
                    vec![("s", Type::String)],
                    Type::Array(Box::new(Type::String)),
                ),
            ]),
            "collections" => Some(vec![]),
            "collections::arrays" => Some(Self::get_collections_array_functions()),
            "collections::maps" => Some(Self::get_collections_map_functions()),
            "env" => Some(vec![
                StdModuleFn::new("getenv", vec![("key", Type::String)], Type::String),
                StdModuleFn::new(
                    "lookup_env",
                    vec![("key", Type::String)],
                    Type::Option(Box::new(Type::String)),
                ),
                StdModuleFn::throwing(
                    "setenv",
                    vec![("key", Type::String), ("value", Type::String)],
                    Type::Unit,
                    vec!["EnvError"],
                ),
                StdModuleFn::throwing(
                    "unsetenv",
                    vec![("key", Type::String)],
                    Type::Unit,
                    vec!["EnvError"],
                ),
                StdModuleFn::throwing(
                    "clearenv",
                    vec![],
                    Type::Unit,
                    vec!["EnvError"],
                ),
                StdModuleFn::new("environ", vec![], Type::Array(Box::new(Type::String))),
                StdModuleFn::new("expand_env", vec![("s", Type::String)], Type::String),
            ]),
            "os" => Some(vec![
                StdModuleFn::throwing(
                    "hostname",
                    vec![],
                    Type::String,
                    vec!["OSError"],
                ),
                StdModuleFn::new("temp_dir", vec![], Type::String),
                StdModuleFn::throwing(
                    "home_dir",
                    vec![],
                    Type::String,
                    vec!["OSError"],
                ),
                StdModuleFn::throwing(
                    "cache_dir",
                    vec![],
                    Type::String,
                    vec!["OSError"],
                ),
                StdModuleFn::throwing(
                    "config_dir",
                    vec![],
                    Type::String,
                    vec!["OSError"],
                ),
                StdModuleFn::throwing(
                    "executable",
                    vec![],
                    Type::String,
                    vec!["OSError"],
                ),
                StdModuleFn::new("pagesize", vec![], Type::Int),
                StdModuleFn::new("getuid", vec![], Type::Int),
                StdModuleFn::new("geteuid", vec![], Type::Int),
                StdModuleFn::new("getgid", vec![], Type::Int),
                StdModuleFn::new("getegid", vec![], Type::Int),
                StdModuleFn::throwing(
                    "getgroups",
                    vec![],
                    Type::Array(Box::new(Type::Int)),
                    vec!["OSError"],
                ),
            ]),
            "process" => Some(vec![
                StdModuleFn::new("getpid", vec![], Type::Int),
                StdModuleFn::new("getppid", vec![], Type::Int),
                StdModuleFn::new("exit", vec![("code", Type::Int)], Type::Unit),
                StdModuleFn::throwing(
                    "pipe_read",
                    vec![],
                    Type::Int,
                    vec!["ProcessError"],
                ),
                StdModuleFn::new("pipe_write", vec![], Type::Int),
                StdModuleFn::throwing(
                    "start_process",
                    vec![("name", Type::String), ("args", Type::Array(Box::new(Type::String)))],
                    Type::Int,
                    vec!["ProcessError"],
                ),
                StdModuleFn::throwing(
                    "find_process",
                    vec![("pid", Type::Int)],
                    Type::Int,
                    vec!["ProcessError"],
                ),
                StdModuleFn::throwing(
                    "wait",
                    vec![("handle", Type::Int)],
                    Type::Array(Box::new(Type::Int)),
                    vec!["ProcessError"],
                ),
                StdModuleFn::throwing(
                    "signal",
                    vec![("handle", Type::Int), ("sig", Type::Int)],
                    Type::Unit,
                    vec!["ProcessError"],
                ),
                StdModuleFn::throwing(
                    "kill",
                    vec![("handle", Type::Int)],
                    Type::Unit,
                    vec!["ProcessError"],
                ),
                StdModuleFn::new("release", vec![("handle", Type::Int)], Type::Unit),
                StdModuleFn::new("SIGHUP", vec![], Type::Int),
                StdModuleFn::new("SIGINT", vec![], Type::Int),
                StdModuleFn::new("SIGQUIT", vec![], Type::Int),
                StdModuleFn::new("SIGKILL", vec![], Type::Int),
                StdModuleFn::new("SIGTERM", vec![], Type::Int),
                StdModuleFn::new("SIGSTOP", vec![], Type::Int),
                StdModuleFn::new("SIGCONT", vec![], Type::Int),
            ]),
            "testing" => Some(vec![
                StdModuleFn::new(
                    "assert",
                    vec![("condition", Type::Bool), ("message", Type::String)],
                    Type::Unit,
                ),
                StdModuleFn::new(
                    "assert_eq",
                    vec![("actual", Type::Int), ("expected", Type::Int), ("message", Type::String)],
                    Type::Unit,
                ),
                StdModuleFn::new(
                    "assert_eq_float",
                    vec![("actual", Type::Float), ("expected", Type::Float), ("message", Type::String)],
                    Type::Unit,
                ),
                StdModuleFn::new(
                    "assert_eq_string",
                    vec![("actual", Type::String), ("expected", Type::String), ("message", Type::String)],
                    Type::Unit,
                ),
                StdModuleFn::new(
                    "assert_eq_bool",
                    vec![("actual", Type::Bool), ("expected", Type::Bool), ("message", Type::String)],
                    Type::Unit,
                ),
                StdModuleFn::new(
                    "assert_neq",
                    vec![("actual", Type::Int), ("expected", Type::Int), ("message", Type::String)],
                    Type::Unit,
                ),
                StdModuleFn::new(
                    "assert_neq_string",
                    vec![("actual", Type::String), ("expected", Type::String), ("message", Type::String)],
                    Type::Unit,
                ),
                StdModuleFn::new(
                    "assert_true",
                    vec![("condition", Type::Bool), ("message", Type::String)],
                    Type::Unit,
                ),
                StdModuleFn::new(
                    "assert_false",
                    vec![("condition", Type::Bool), ("message", Type::String)],
                    Type::Unit,
                ),
                StdModuleFn::new(
                    "assert_gt",
                    vec![("actual", Type::Int), ("expected", Type::Int), ("message", Type::String)],
                    Type::Unit,
                ),
                StdModuleFn::new(
                    "assert_gte",
                    vec![("actual", Type::Int), ("expected", Type::Int), ("message", Type::String)],
                    Type::Unit,
                ),
                StdModuleFn::new(
                    "assert_lt",
                    vec![("actual", Type::Int), ("expected", Type::Int), ("message", Type::String)],
                    Type::Unit,
                ),
                StdModuleFn::new(
                    "assert_lte",
                    vec![("actual", Type::Int), ("expected", Type::Int), ("message", Type::String)],
                    Type::Unit,
                ),
                StdModuleFn::new(
                    "fail",
                    vec![("message", Type::String)],
                    Type::Unit,
                ),
                StdModuleFn::new(
                    "assert_approx",
                    vec![
                        ("actual", Type::Float),
                        ("expected", Type::Float),
                        ("epsilon", Type::Float),
                        ("message", Type::String),
                    ],
                    Type::Unit,
                ),
                StdModuleFn::new(
                    "assert_contains",
                    vec![
                        ("haystack", Type::String),
                        ("needle", Type::String),
                        ("message", Type::String),
                    ],
                    Type::Unit,
                ),
                StdModuleFn::new(
                    "assert_starts_with",
                    vec![
                        ("value", Type::String),
                        ("prefix", Type::String),
                        ("message", Type::String),
                    ],
                    Type::Unit,
                ),
                StdModuleFn::new(
                    "assert_ends_with",
                    vec![
                        ("value", Type::String),
                        ("suffix", Type::String),
                        ("message", Type::String),
                    ],
                    Type::Unit,
                ),
            ]),
            "fs" => Some(Self::get_fs_functions()),
            "path" => Some(vec![
                // Path joining and construction
                StdModuleFn::new(
                    "join",
                    vec![("parts", Type::Array(Box::new(Type::String)))],
                    Type::String,
                ),
                // Path normalization
                StdModuleFn::new("normalize", vec![("path", Type::String)], Type::String),
                // Path type checks
                StdModuleFn::new("is_absolute", vec![("path", Type::String)], Type::Bool),
                StdModuleFn::new("is_relative", vec![("path", Type::String)], Type::Bool),
                StdModuleFn::new("has_root", vec![("path", Type::String)], Type::Bool),
                // Path component extraction
                StdModuleFn::new("dirname", vec![("path", Type::String)], Type::String),
                StdModuleFn::new("basename", vec![("path", Type::String)], Type::String),
                StdModuleFn::new("extension", vec![("path", Type::String)], Type::String),
                StdModuleFn::new("stem", vec![("path", Type::String)], Type::String),
                // Path modification
                StdModuleFn::new(
                    "with_extension",
                    vec![("path", Type::String), ("ext", Type::String)],
                    Type::String,
                ),
                // Path component splitting
                StdModuleFn::new(
                    "components",
                    vec![("path", Type::String)],
                    Type::Array(Box::new(Type::String)),
                ),
                // Platform info
                StdModuleFn::new("separator", vec![], Type::String),
                // Slash conversion
                StdModuleFn::new("to_slash", vec![("path", Type::String)], Type::String),
                StdModuleFn::new("from_slash", vec![("path", Type::String)], Type::String),
                // Path comparison
                StdModuleFn::new(
                    "starts_with",
                    vec![("path", Type::String), ("prefix", Type::String)],
                    Type::Bool,
                ),
                StdModuleFn::new(
                    "ends_with",
                    vec![("path", Type::String), ("suffix", Type::String)],
                    Type::Bool,
                ),
                // Path manipulation
                StdModuleFn::new(
                    "strip_prefix",
                    vec![("path", Type::String), ("prefix", Type::String)],
                    Type::String,
                ),
            ]),
            // Encoding module and submodules
            "encoding" => Some(vec![]),
            "encoding::utf8" => Some(Self::get_encoding_utf8_functions()),
            "encoding::hex" => Some(Self::get_encoding_hex_functions()),
            "encoding::base64" => Some(Self::get_encoding_base64_functions()),
            "encoding::url" => Some(Self::get_encoding_url_functions()),
            "encoding::json" => Some(Self::get_encoding_json_functions()),
            "encoding::toml" => Some(Self::get_encoding_toml_functions()),
            "encoding::yaml" => Some(Self::get_encoding_yaml_functions()),
            // Net module hierarchy - strict: parent modules expose only submodules, not functions
            // Parent modules - no functions, only submodules
            "net" => Some(vec![]),
            "net::tcp" => Some(vec![]),
            "net::http" => Some(vec![]),
            // Leaf modules - specific functions only
            "net::udp" => Some(Self::get_net_udp_functions()),
            "net::tcp::listener" => Some(Self::get_net_tcp_server_functions()),
            "net::tcp::client" => Some(Self::get_net_tcp_client_functions()),
            "net::http::client" => Some(Self::get_net_http_client_functions()),
            "net::http::serve" => Some(Self::get_net_http_server_functions()),
            "net::http::middleware" => Some(Self::get_net_http_middleware_functions()),
            "db" => Some(vec![]),
            "db::sqlite" => Some(Self::get_db_sqlite_functions()),
            _ => None,
        }
    }

    fn get_db_sqlite_functions() -> Vec<StdModuleFn> {
        vec![
            StdModuleFn::throwing(
                "open",
                vec![("path", Type::String)],
                Type::Int,
                vec!["DBError"],
            ),
            StdModuleFn::throwing(
                "open_memory",
                vec![],
                Type::Int,
                vec!["DBError"],
            ),
            StdModuleFn::new("close", vec![("db", Type::Int)], Type::Unit),
            StdModuleFn::throwing(
                "exec",
                vec![("db", Type::Int), ("sql", Type::String)],
                Type::Unit,
                vec!["DBError"],
            ),
            StdModuleFn::throwing(
                "query",
                vec![
                    ("db", Type::Int),
                    ("sql", Type::String),
                    ("params", Type::array(Type::String)),
                ],
                Type::Int,
                vec!["DBError"],
            ),
            StdModuleFn::new("row_count", vec![("rows", Type::Int)], Type::Int),
            StdModuleFn::new(
                "row_at",
                vec![("rows", Type::Int), ("index", Type::Int)],
                Type::Int,
            ),
            StdModuleFn::new(
                "get_string",
                vec![("row", Type::Int), ("col", Type::String)],
                Type::String,
            ),
            StdModuleFn::new(
                "get_int",
                vec![("row", Type::Int), ("col", Type::String)],
                Type::Int,
            ),
            StdModuleFn::new(
                "get_float",
                vec![("row", Type::Int), ("col", Type::String)],
                Type::Float,
            ),
            StdModuleFn::new(
                "get_bool",
                vec![("row", Type::Int), ("col", Type::String)],
                Type::Bool,
            ),
            StdModuleFn::new(
                "is_null",
                vec![("row", Type::Int), ("col", Type::String)],
                Type::Bool,
            ),
            StdModuleFn::new("columns", vec![("rows", Type::Int)], Type::String),
            StdModuleFn::new("column_count", vec![("rows", Type::Int)], Type::Int),
            StdModuleFn::throwing(
                "begin",
                vec![("db", Type::Int)],
                Type::Unit,
                vec!["DBError"],
            ),
            StdModuleFn::throwing(
                "commit",
                vec![("db", Type::Int)],
                Type::Unit,
                vec!["DBError"],
            ),
            StdModuleFn::throwing(
                "rollback",
                vec![("db", Type::Int)],
                Type::Unit,
                vec!["DBError"],
            ),
            StdModuleFn::throwing(
                "prepare",
                vec![("db", Type::Int), ("sql", Type::String)],
                Type::Int,
                vec!["DBError"],
            ),
            StdModuleFn::throwing(
                "bind_string",
                vec![
                    ("stmt", Type::Int),
                    ("index", Type::Int),
                    ("val", Type::String),
                ],
                Type::Unit,
                vec!["DBError"],
            ),
            StdModuleFn::throwing(
                "bind_int",
                vec![
                    ("stmt", Type::Int),
                    ("index", Type::Int),
                    ("val", Type::Int),
                ],
                Type::Unit,
                vec!["DBError"],
            ),
            StdModuleFn::throwing(
                "bind_float",
                vec![
                    ("stmt", Type::Int),
                    ("index", Type::Int),
                    ("val", Type::Float),
                ],
                Type::Unit,
                vec!["DBError"],
            ),
            StdModuleFn::throwing(
                "step",
                vec![("stmt", Type::Int)],
                Type::Unit,
                vec!["DBError"],
            ),
            StdModuleFn::new("reset", vec![("stmt", Type::Int)], Type::Unit),
            StdModuleFn::new("finalize", vec![("stmt", Type::Int)], Type::Unit),
            StdModuleFn::new("changes", vec![("db", Type::Int)], Type::Int),
            StdModuleFn::new("last_insert_id", vec![("db", Type::Int)], Type::Int),
        ]
    }

    fn resolve_local_module(
        &mut self,
        path: &[String],
        items: &UseItems,
        span: crate::source::Span,
    ) {
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

        let tokens = crate::lexer::tokenize_with_interner(&source_text, self.interner);
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
                    let name = self.interner.resolve(&func.name.symbol).to_string();
                    let params: Vec<_> = func
                        .params
                        .iter()
                        .map(|p| {
                            let pname = self.interner.resolve(&p.name.symbol).to_string();
                            let pty = self.convert_type(&p.ty);
                            (pname, pty)
                        })
                        .collect();
                    let return_ty = func
                        .return_ty
                        .as_ref()
                        .map(|t| self.convert_type(t))
                        .unwrap_or(Type::Unit);
                    pub_functions.push((name, params, return_ty, false));
                }
            }
        }

        let module_name = path.last().unwrap();
        let module_spur = match self.interner.get(module_name.as_str()) {
            Some(s) => s,
            None => {
                self.imported_modules.push(ImportedModule {
                    source_text,
                    file_path,
                });
                return;
            }
        };

        match items {
            UseItems::All => {
                for (name, params, return_ty, is_variadic) in &pub_functions {
                    if let Some(spur) = self.interner.get(name.as_str()) {
                        let params: Vec<_> = params
                            .iter()
                            .map(|(pname, pty)| {
                                let pspur = self.interner.get(pname.as_str()).unwrap_or(spur);
                                (pspur, pty.clone())
                            })
                            .collect();
                        let sig = FunctionSig {
                            name: spur,
                            type_params: vec![],
                            params,
                            return_ty: return_ty.clone(),
                            throws: vec![],
                            is_public: true,
                            is_variadic: *is_variadic,
                            span: crate::source::Span::dummy(),
                            module: Some(path.join("::")),
                        };
                        self.symbols
                            .register_module(module_spur)
                            .add_function(sig.clone());
                        self.symbols.import_function(sig.clone());
                    }
                }
            }
            UseItems::Specific(entries) => {
                for entry in entries {
                    let entry_name = self.interner.resolve(&entry.name.symbol).to_string();
                    let found = pub_functions
                        .iter()
                        .find(|(name, _, _, _)| *name == entry_name);
                    match found {
                        Some((_, params, return_ty, is_variadic)) => {
                            let spur = entry.name.symbol;
                            let params: Vec<_> = params
                                .iter()
                                .map(|(pname, pty)| {
                                    let pspur = self.interner.get(pname.as_str()).unwrap_or(spur);
                                    (pspur, pty.clone())
                                })
                                .collect();
                            let sig = FunctionSig {
                                name: spur,
                                type_params: vec![],
                                params,
                                return_ty: return_ty.clone(),
                                throws: vec![],
                                is_public: true,
                                is_variadic: *is_variadic,
                                span: crate::source::Span::dummy(),
                                module: Some(path.join("::")),
                            };
                            self.symbols
                                .register_module(module_spur)
                                .add_function(sig.clone());
                            // Check for duplicate imports and emit error
                            if self.symbols.has_function(sig.name) {
                                self.symbols.mark_ambiguous(sig.name);
                                self.errors.push(TypeError::DuplicateImport {
                                    name: entry_name.clone(),
                                    span: entry.span,
                                });
                            } else {
                                self.symbols.import_function(sig.clone());
                            }
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
            module: None,
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
            module: None,
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

    fn collect_type_alias(&mut self, alias: &ast::TypeAliasItem) {
        let type_params: Vec<TypeParam> = alias
            .generics
            .iter()
            .map(|g| TypeParam {
                name: g.name.symbol,
                bounds: g.bounds.iter().map(|b| self.convert_type(b)).collect(),
            })
            .collect();

        let aliased_type = self.convert_type(&alias.aliased_type);

        self.symbols.define_type(
            alias.name.symbol,
            TypeDef::TypeAlias(TypeAliasDef {
                name: alias.name.symbol,
                type_params,
                aliased_type,
                is_public: alias.is_public,
                span: alias.span,
            }),
        );
    }

    fn check_items(&mut self, file: &SourceFile) {
        // Pass 1: Process all top-level statements (global variables)
        // so they're visible to all functions regardless of source order
        for item in &file.items {
            if let Item::TopLevelStmt(stmt_item) = item {
                self.check_top_level_stmt(stmt_item);
            }
        }

        // Pass 2: Process functions and modules
        for item in &file.items {
            match item {
                Item::Function(func) => self.check_function(func),
                Item::Mod(m) => self.check_mod(m),
                _ => {}
            }
        }
    }

    fn check_mod<'ast>(&mut self, m: &'ast ast::ModuleItem<'ast>) {
        let name_spur = m.name.symbol;
        self.symbols.enter_module(name_spur);
        if let Some(ref items) = m.body {
            for item in items {
                if let Item::TopLevelStmt(stmt_item) = item {
                    self.check_top_level_stmt(stmt_item);
                }
            }
            for item in items {
                match item {
                    Item::Function(func) => self.check_function(func),
                    Item::Mod(inner_m) => self.check_mod(inner_m),
                    _ => {}
                }
            }
        }
        self.symbols.exit_module();
    }

    fn check_top_level_stmt(&mut self, stmt_item: &ast::TopLevelStmtItem) {
        // Top-level statements (including global variable declarations) are checked
        // in the root scope so they're accessible from all functions in the module
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

        self.env.enter_function(return_ty, throws, &type_params);
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
            ast::NamlType::Mutex(inner) => Type::Mutex(Box::new(self.convert_type(inner))),
            ast::NamlType::Rwlock(inner) => Type::Rwlock(Box::new(self.convert_type(inner))),
            ast::NamlType::Atomic(inner) => Type::Atomic(Box::new(self.convert_type(inner))),
            ast::NamlType::Named(ident) => {
                // Check for built-in types first
                let name = self.interner.resolve(&ident.symbol);
                if name == "stack_frame" {
                    return Type::StackFrame;
                }
                if name == "json" {
                    return Type::Json;
                }

                if let Some(def) = self.symbols.get_type(ident.symbol) {
                    match def {
                        TypeDef::Struct(s) => Type::Struct(self.symbols.to_struct_type(s)),
                        TypeDef::Enum(e) => Type::Enum(self.symbols.to_enum_type(e)),
                        TypeDef::Interface(i) => Type::Interface(self.symbols.to_interface_type(i)),
                        TypeDef::Exception(e) => Type::Exception(e.name),
                        TypeDef::TypeAlias(a) => a.aliased_type.clone(),
                    }
                } else {
                    Type::Generic(ident.symbol, Vec::new())
                }
            }
            ast::NamlType::Generic(ident, args) => {
                let converted_args: Vec<Type> = args.iter().map(|a| self.convert_type(a)).collect();

                // Check if this is a type alias with type params
                if let Some(TypeDef::TypeAlias(alias)) = self.symbols.get_type(ident.symbol) {
                    if alias.type_params.len() == converted_args.len() {
                        // Substitute type params with provided args
                        return self.substitute_type_args(
                            &alias.aliased_type,
                            &alias.type_params,
                            &converted_args,
                        );
                    }
                }

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

    fn substitute_type_args(
        &self,
        ty: &Type,
        type_params: &[TypeParam],
        type_args: &[Type],
    ) -> Type {
        match ty {
            Type::Generic(name, args) => {
                // Check if this is one of the type parameters to substitute
                for (i, param) in type_params.iter().enumerate() {
                    if *name == param.name && args.is_empty() {
                        return type_args[i].clone();
                    }
                }
                // Otherwise, recursively substitute in the args
                let new_args = args
                    .iter()
                    .map(|a| self.substitute_type_args(a, type_params, type_args))
                    .collect();
                Type::Generic(*name, new_args)
            }
            Type::Array(inner) => Type::Array(Box::new(self.substitute_type_args(
                inner,
                type_params,
                type_args,
            ))),
            Type::FixedArray(inner, n) => Type::FixedArray(
                Box::new(self.substitute_type_args(inner, type_params, type_args)),
                *n,
            ),
            Type::Option(inner) => Type::Option(Box::new(self.substitute_type_args(
                inner,
                type_params,
                type_args,
            ))),
            Type::Map(k, v) => Type::Map(
                Box::new(self.substitute_type_args(k, type_params, type_args)),
                Box::new(self.substitute_type_args(v, type_params, type_args)),
            ),
            Type::Channel(inner) => Type::Channel(Box::new(self.substitute_type_args(
                inner,
                type_params,
                type_args,
            ))),
            Type::Function(ft) => Type::Function(types::FunctionType {
                params: ft
                    .params
                    .iter()
                    .map(|p| self.substitute_type_args(p, type_params, type_args))
                    .collect(),
                returns: Box::new(self.substitute_type_args(&ft.returns, type_params, type_args)),
                throws: ft
                    .throws
                    .iter()
                    .map(|t| self.substitute_type_args(t, type_params, type_args))
                    .collect(),
                is_variadic: ft.is_variadic,
            }),
            // Primitive types and others don't need substitution
            _ => ty.clone(),
        }
    }
}

pub fn check(file: &SourceFile, interner: &mut Rodeo) -> Vec<TypeError> {
    check_with_types(file, interner, None).errors
}

pub fn check_with_types(
    file: &SourceFile,
    interner: &mut Rodeo,
    source_dir: Option<PathBuf>,
) -> TypeCheckResult {
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
        let (tokens, mut interner) = tokenize(source);
        let arena = AstArena::new();
        let result = parse(&tokens, source, &arena);
        assert!(
            result.errors.is_empty(),
            "Parse errors: {:?}",
            result.errors
        );
        check(&result.ast, &mut interner)
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
        let errors = check_source("fn add(a: int, b: int) -> int { return a + b; }");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_type_mismatch() {
        let errors = check_source("fn main() { var x: int = true; }");
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_undefined_variable() {
        let errors = check_source("fn main() { return x; }");
        assert!(!errors.is_empty());
        assert!(matches!(errors[0], TypeError::UndefinedVariable { .. }));
    }

    #[test]
    fn test_valid_if_statement() {
        let errors = check_source("fn main() { if (true) { var x: int = 1; } }");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_invalid_condition() {
        let errors = check_source("fn main() { if (42) { var x: int = 1; } }");
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_break_outside_loop() {
        let errors = check_source("fn main() { break; }");
        assert!(!errors.is_empty());
        assert!(matches!(errors[0], TypeError::BreakOutsideLoop { .. }));
    }

    #[test]
    fn test_valid_loop() {
        let errors = check_source("fn main() { while (true) { break; } }");
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
        let errors = check_source("fn main() { var x: int = 42; var y: int = x; }");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_array_type() {
        let errors = check_source("fn main() { var arr: [int] = [1, 2, 3]; }");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_lambda() {
        let errors = check_source(
            "fn main() { var f: fn(int) -> int = fn(x: int) -> int { return x + 1; }; }",
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_global_var_in_function() {
        let errors = check_source(
            "var PI: float = 3.14;\nvar SOLAR_MASS: float = 4.0 * PI * PI;\nfn main() { var x: float = SOLAR_MASS; }",
        );
        assert!(errors.is_empty(), "Global variables should be visible inside functions: {:?}", errors);
    }

    #[test]
    fn test_global_var_after_function() {
        let errors = check_source(
            "fn main() { var x: float = GRAVITY; }\nvar GRAVITY: float = 9.81;",
        );
        assert!(errors.is_empty(), "Global variables defined after functions should still be visible: {:?}", errors);
    }
}
