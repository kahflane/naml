//!
//! Cranelift JIT Compiler
//!
//! Compiles naml AST directly to machine code using Cranelift.
//! This eliminates the Rust transpilation step and gives full control
//! over memory management and runtime semantics.
//!

mod array;
mod binop;
mod builtins;
mod channels;
mod context;
mod errors;
mod exceptions;
mod expr;
mod externs;
mod heap;
mod init;
mod io;
mod lambda;
mod literal;
mod map;
mod method;
mod misc;
mod options;
mod pattern;
mod print;
mod runtime;
mod spawns;
mod stmt;
mod strings;
mod structs;
mod types;
mod closures;
mod compiler;
mod decls;
mod decref;
mod excepts;
mod funcs;
mod methods;
mod mono;
mod scan;
mod trampoline;

use std::collections::{HashMap, HashSet};
use indexmap::IndexMap;

use cranelift::prelude::{Block, Variable};
use cranelift_codegen as codegen;
use cranelift_jit::JITModule;
use cranelift_module::FuncId;
use lasso::{Rodeo, Spur};

use crate::ast::{Expression, FunctionItem, Statement};
use crate::codegen::cranelift::heap::HeapType;
use crate::typechecker::TypeAnnotations;

#[derive(Clone)]
pub struct StructDef {
    pub type_id: u32,
    pub fields: Vec<Spur>,
    pub(crate) field_heap_types: Vec<Option<HeapType>>,
}

#[derive(Clone)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<EnumVariantDef>,
    pub size: usize,
}

#[derive(Clone)]
pub struct EnumVariantDef {
    pub name: String,
    pub tag: u32,
    pub field_types: Vec<crate::ast::NamlType>,
    pub data_offset: usize,
}

#[derive(Clone)]
pub struct ExternFn {
    pub link_name: String,
    pub param_types: Vec<crate::ast::NamlType>,
    pub return_type: Option<crate::ast::NamlType>,
}

#[derive(Clone)]
pub struct SpawnBlockInfo {
    pub id: u32,
    pub func_name: String,
    pub captured_vars: Vec<String>,
    pub(crate) captured_heap_types: HashMap<String, HeapType>,
    pub body_ptr: *const crate::ast::BlockExpr<'static>,
}

unsafe impl Send for SpawnBlockInfo {}

#[derive(Clone)]
pub struct LambdaInfo {
    pub id: u32,
    pub func_name: String,
    pub captured_vars: Vec<String>,
    pub param_names: Vec<String>,
    pub body_ptr: *const crate::ast::Expression<'static>,
}

/// Information for inlineable functions
#[derive(Clone)]
pub struct InlineFuncInfo {
    pub func_ptr: *const FunctionItem<'static>,
    pub param_names: Vec<String>,
    pub param_types: Vec<crate::ast::NamlType>,
    pub return_type: Option<crate::ast::NamlType>,
}

unsafe impl Send for InlineFuncInfo {}

pub struct CompileContext<'a> {
    interner: &'a Rodeo,
    module: &'a mut JITModule,
    functions: &'a HashMap<String, FuncId>,
    runtime_funcs: &'a HashMap<String, FuncId>,
    struct_defs: &'a HashMap<Spur, StructDef>,
    enum_defs: &'a HashMap<String, EnumDef>,
    exception_names: &'a HashSet<Spur>,
    extern_fns: &'a HashMap<String, ExternFn>,
    global_vars: &'a IndexMap<String, GlobalVarDef>,
    variables: HashMap<String, Variable>,
    var_heap_types: HashMap<String, HeapType>,
    var_counter: usize,
    block_terminated: bool,
    loop_exit_block: Option<Block>,
    loop_header_block: Option<Block>,
    spawn_blocks: &'a HashMap<u32, SpawnBlockInfo>,
    spawn_body_to_id: &'a HashMap<usize, u32>,
    lambda_blocks: &'a HashMap<u32, LambdaInfo>,
    lambda_body_to_id: &'a HashMap<usize, u32>,
    annotations: &'a TypeAnnotations,
    type_substitutions: HashMap<String, String>,
    func_return_type: Option<cranelift::prelude::Type>,
    release_mode: bool,
    unsafe_mode: bool,
    inline_functions: &'a HashMap<String, InlineFuncInfo>,
    inline_depth: u32,
    inline_exit_block: Option<Block>,
    inline_result_var: Option<Variable>,
    borrowed_vars: HashSet<String>,
    reassigned_vars: HashSet<String>,
}

unsafe impl Send for LambdaInfo {}

fn collect_reassigned_vars(
    stmts: &[Statement<'_>],
    interner: &Rodeo,
    out: &mut HashSet<String>,
) {
    for stmt in stmts {
        scan_reassignments(stmt, interner, out);
    }
}

fn scan_reassignments(
    stmt: &Statement<'_>,
    interner: &Rodeo,
    out: &mut HashSet<String>,
) {
    match stmt {
        Statement::Assign(assign) => {
            if let Expression::Identifier(ident) = &assign.target {
                out.insert(interner.resolve(&ident.ident.symbol).to_string());
            }
        }
        Statement::If(if_stmt) => {
            collect_reassigned_vars(&if_stmt.then_branch.statements, interner, out);
            if let Some(ref else_branch) = if_stmt.else_branch {
                match else_branch {
                    crate::ast::ElseBranch::ElseIf(else_if) => {
                        collect_reassigned_vars(&else_if.then_branch.statements, interner, out);
                    }
                    crate::ast::ElseBranch::Else(else_block) => {
                        collect_reassigned_vars(&else_block.statements, interner, out);
                    }
                }
            }
        }
        Statement::While(w) => collect_reassigned_vars(&w.body.statements, interner, out),
        Statement::For(f) => collect_reassigned_vars(&f.body.statements, interner, out),
        Statement::Loop(l) => collect_reassigned_vars(&l.body.statements, interner, out),
        Statement::Switch(s) => {
            for case in &s.cases {
                collect_reassigned_vars(&case.body.statements, interner, out);
            }
            if let Some(ref default) = s.default {
                collect_reassigned_vars(&default.statements, interner, out);
            }
        }
        Statement::Block(b) => collect_reassigned_vars(&b.statements, interner, out),
        Statement::Locked(l) => collect_reassigned_vars(&l.body.statements, interner, out),
        Statement::Var(v) => {
            if let Some(ref else_block) = v.else_block {
                collect_reassigned_vars(&else_block.statements, interner, out);
            }
        }
        _ => {}
    }
}

fn get_field_access_base_var<'a>(
    expr: &'a Expression<'_>,
    interner: &Rodeo,
) -> Option<String> {
    match expr {
        Expression::Field(f) => {
            if let Expression::Identifier(ident) = f.base {
                Some(interner.resolve(&ident.ident.symbol).to_string())
            } else {
                None
            }
        }
        Expression::ForceUnwrap(uw) => get_field_access_base_var(uw.expr, interner),
        _ => None,
    }
}

// NamlArray struct layout offsets (must match runtime/array.rs)
// NamlArray: header(16) + len(8) + capacity(8) + data(8)
pub(crate) const ARRAY_LEN_OFFSET: i32 = 16;
const ARRAY_CAPACITY_OFFSET: i32 = 24;
const ARRAY_DATA_OFFSET: i32 = 32;

/// Global variable definition for codegen
#[derive(Clone)]
pub struct GlobalVarDef {
    pub data_id: cranelift_module::DataId,
    pub init_expr: *const Expression<'static>,
    pub cl_type: cranelift::prelude::Type,
}

unsafe impl Send for GlobalVarDef {}

pub struct JitCompiler<'a> {
    interner: &'a Rodeo,
    annotations: &'a TypeAnnotations,
    source_info: &'a crate::source::SourceFile,
    module: JITModule,
    ctx: codegen::Context,
    functions: HashMap<String, FuncId>,
    runtime_funcs: HashMap<String, FuncId>,
    struct_defs: HashMap<Spur, StructDef>,
    enum_defs: HashMap<String, EnumDef>,
    exception_names: HashSet<Spur>,
    extern_fns: HashMap<String, ExternFn>,
    global_vars: IndexMap<String, GlobalVarDef>,
    next_type_id: u32,
    spawn_counter: u32,
    spawn_blocks: HashMap<u32, SpawnBlockInfo>,
    spawn_body_to_id: HashMap<usize, u32>,
    lambda_counter: u32,
    lambda_blocks: HashMap<u32, LambdaInfo>,
    lambda_body_to_id: HashMap<usize, u32>,
    generic_functions: HashMap<String, *const FunctionItem<'a>>,
    inline_functions: HashMap<String, InlineFuncInfo>,
    release_mode: bool,
    unsafe_mode: bool,
}

extern "C" fn naml_print_int(val: i64) {
    print!("{}", val);
}

extern "C" fn naml_print_float(val: f64) {
    print!("{}", val);
}

extern "C" fn naml_print_bool(val: i64) {
    if val != 0 {
        print!("true");
    } else {
        print!("false");
    }
}

extern "C" fn naml_print_str(ptr: *const std::ffi::c_char) {
    if !ptr.is_null() {
        let c_str = unsafe { std::ffi::CStr::from_ptr(ptr) };
        if let Ok(s) = c_str.to_str() {
            print!("{}", s);
        }
    }
}

extern "C" fn naml_print_newline() {
    println!();
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_exists() {
        assert!(true);
    }
}
