//!
//! Cranelift JIT Compiler
//!
//! Compiles naml AST directly to machine code using Cranelift.
//! This eliminates the Rust transpilation step and gives full control
//! over memory management and runtime semantics.
//!

mod array;
mod builtins;
mod context;
mod errors;
mod map;
mod pattern;
mod stmt;
mod types;
mod expr;
mod method;
mod literal;
mod print;
mod misc;
mod externs;
mod options;
mod lambda;
mod exceptions;
mod channels;
mod spawns;
mod structs;
mod strings;
mod io;
mod binop;
mod heap;
mod runtime;

use std::collections::{HashMap, HashSet};
use std::panic;

use cranelift::prelude::*;
use cranelift_codegen::ir::{AtomicRmwOp};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};
use lasso::Rodeo;

use crate::ast::{
    Expression, FunctionItem, Item,SourceFile,
    Statement
};
use crate::codegen::cranelift::errors::convert_cranelift_error;
use crate::codegen::cranelift::expr::compile_expression;
use crate::codegen::cranelift::map::call_map_set;
use crate::codegen::cranelift::stmt::compile_statement;
use crate::codegen::CodegenError;
use crate::codegen::cranelift::heap::{get_heap_type, HeapType};
use crate::codegen::cranelift::runtime::{emit_cleanup_all_vars, emit_stack_push, emit_stack_pop};
use crate::typechecker::{Type, TypeAnnotations};

#[derive(Clone)]
pub struct StructDef {
    pub type_id: u32,
    pub fields: Vec<String>,
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

pub struct CompileContext<'a> {
    interner: &'a Rodeo,
    module: &'a mut JITModule,
    functions: &'a HashMap<String, FuncId>,
    runtime_funcs: &'a HashMap<String, FuncId>,
    struct_defs: &'a HashMap<String, StructDef>,
    enum_defs: &'a HashMap<String, EnumDef>,
    exception_names: &'a HashSet<String>,
    extern_fns: &'a HashMap<String, ExternFn>,
    variables: HashMap<String, Variable>,
    var_heap_types: HashMap<String, HeapType>,
    var_counter: usize,
    block_terminated: bool,
    loop_exit_block: Option<Block>,
    loop_header_block: Option<Block>,
    spawn_blocks: &'a HashMap<u32, SpawnBlockInfo>,
    current_spawn_id: u32,
    lambda_blocks: &'a HashMap<u32, LambdaInfo>,
    current_lambda_id: u32,
    annotations: &'a TypeAnnotations,
    type_substitutions: HashMap<String, String>,
    func_return_type: Option<cranelift::prelude::Type>,
}

unsafe impl Send for LambdaInfo {}

// NamlArray struct layout offsets (must match runtime/array.rs)
// NamlArray: header(16) + len(8) + capacity(8) + data(8)
pub(crate) const ARRAY_LEN_OFFSET: i32 = 16;
const ARRAY_CAPACITY_OFFSET: i32 = 24;
const ARRAY_DATA_OFFSET: i32 = 32;

pub struct JitCompiler<'a> {
    interner: &'a Rodeo,
    annotations: &'a TypeAnnotations,
    source_info: &'a crate::source::SourceFile,
    module: JITModule,
    ctx: codegen::Context,
    functions: HashMap<String, FuncId>,
    runtime_funcs: HashMap<String, FuncId>,
    struct_defs: HashMap<String, StructDef>,
    enum_defs: HashMap<String, EnumDef>,
    exception_names: HashSet<String>,
    extern_fns: HashMap<String, ExternFn>,
    next_type_id: u32,
    spawn_counter: u32,
    spawn_blocks: HashMap<u32, SpawnBlockInfo>,
    lambda_counter: u32,
    lambda_blocks: HashMap<u32, LambdaInfo>,
    generic_functions: HashMap<String, *const FunctionItem<'a>>,
}

impl<'a> JitCompiler<'a> {
    pub fn new(
        interner: &'a Rodeo,
        annotations: &'a TypeAnnotations,
        source_info: &'a crate::source::SourceFile,
    ) -> Result<Self, CodegenError> {
        let mut flag_builder = settings::builder();
        flag_builder.set("use_colocated_libcalls", "false").unwrap();
        flag_builder.set("is_pic", "false").unwrap();

        let isa_builder = cranelift_native::builder().map_err(|e| {
            CodegenError::JitCompile(format!("Failed to create ISA builder: {}", e))
        })?;

        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .map_err(|e| CodegenError::JitCompile(format!("Failed to create ISA: {}", e)))?;

        let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());

        // Print builtins
        builder.symbol("naml_print_int", naml_print_int as *const u8);
        builder.symbol("naml_print_float", naml_print_float as *const u8);
        builder.symbol("naml_print_bool", naml_print_bool as *const u8);
        builder.symbol("naml_print_str", naml_print_str as *const u8);
        builder.symbol("naml_print_newline", naml_print_newline as *const u8);

        // Array runtime functions
        builder.symbol(
            "naml_array_new",
            crate::runtime::naml_array_new as *const u8,
        );
        builder.symbol(
            "naml_array_from",
            crate::runtime::naml_array_from as *const u8,
        );
        builder.symbol(
            "naml_array_push",
            crate::runtime::naml_array_push as *const u8,
        );
        builder.symbol(
            "naml_array_get",
            crate::runtime::naml_array_get as *const u8,
        );
        builder.symbol(
            "naml_array_set",
            crate::runtime::naml_array_set as *const u8,
        );
        builder.symbol(
            "naml_array_len",
            crate::runtime::naml_array_len as *const u8,
        );
        builder.symbol(
            "naml_array_pop",
            crate::runtime::naml_array_pop as *const u8,
        );
        builder.symbol(
            "naml_array_is_empty",
            crate::runtime::naml_array_is_empty as *const u8,
        );
        builder.symbol(
            "naml_array_shift",
            crate::runtime::naml_array_shift as *const u8,
        );
        builder.symbol(
            "naml_array_fill",
            crate::runtime::naml_array_fill as *const u8,
        );
        builder.symbol(
            "naml_array_clear",
            crate::runtime::naml_array_clear as *const u8,
        );
        builder.symbol(
            "naml_array_first",
            crate::runtime::naml_array_first as *const u8,
        );
        builder.symbol(
            "naml_array_last",
            crate::runtime::naml_array_last as *const u8,
        );
        builder.symbol(
            "naml_array_sum",
            crate::runtime::naml_array_sum as *const u8,
        );
        builder.symbol(
            "naml_array_min",
            crate::runtime::naml_array_min as *const u8,
        );
        builder.symbol(
            "naml_array_max",
            crate::runtime::naml_array_max as *const u8,
        );
        builder.symbol(
            "naml_array_reverse",
            crate::runtime::naml_array_reverse as *const u8,
        );
        builder.symbol(
            "naml_array_reversed",
            crate::runtime::naml_array_reversed as *const u8,
        );
        builder.symbol(
            "naml_array_take",
            crate::runtime::naml_array_take as *const u8,
        );
        builder.symbol(
            "naml_array_drop",
            crate::runtime::naml_array_drop as *const u8,
        );
        builder.symbol(
            "naml_array_slice",
            crate::runtime::naml_array_slice as *const u8,
        );
        builder.symbol(
            "naml_array_index_of",
            crate::runtime::naml_array_index_of as *const u8,
        );
        builder.symbol(
            "naml_array_contains",
            crate::runtime::naml_array_contains as *const u8,
        );
        builder.symbol(
            "naml_array_any",
            crate::runtime::naml_array_any as *const u8,
        );
        builder.symbol(
            "naml_array_all",
            crate::runtime::naml_array_all as *const u8,
        );
        builder.symbol(
            "naml_array_count_if",
            crate::runtime::naml_array_count_if as *const u8,
        );
        builder.symbol(
            "naml_array_map",
            crate::runtime::naml_array_map as *const u8,
        );
        builder.symbol(
            "naml_array_filter",
            crate::runtime::naml_array_filter as *const u8,
        );
        builder.symbol(
            "naml_array_find",
            crate::runtime::naml_array_find as *const u8,
        );
        builder.symbol(
            "naml_array_find_index",
            crate::runtime::naml_array_find_index as *const u8,
        );
        builder.symbol(
            "naml_array_fold",
            crate::runtime::naml_array_fold as *const u8,
        );
        builder.symbol(
            "naml_array_flatten",
            crate::runtime::naml_array_flatten as *const u8,
        );
        builder.symbol(
            "naml_array_sort",
            crate::runtime::naml_array_sort as *const u8,
        );
        builder.symbol(
            "naml_array_sort_by",
            crate::runtime::naml_array_sort_by as *const u8,
        );
        builder.symbol(
            "naml_array_print",
            crate::runtime::naml_array_print as *const u8,
        );
        builder.symbol(
            "naml_array_print_strings",
            crate::runtime::naml_array_print_strings as *const u8,
        );
        builder.symbol(
            "naml_array_incref",
            crate::runtime::naml_array_incref as *const u8,
        );
        builder.symbol(
            "naml_array_decref",
            crate::runtime::naml_array_decref as *const u8,
        );
        builder.symbol(
            "naml_array_decref_strings",
            crate::runtime::naml_array_decref_strings as *const u8,
        );
        builder.symbol(
            "naml_array_decref_arrays",
            crate::runtime::naml_array_decref_arrays as *const u8,
        );
        builder.symbol(
            "naml_array_decref_maps",
            crate::runtime::naml_array_decref_maps as *const u8,
        );
        builder.symbol(
            "naml_array_decref_structs",
            crate::runtime::naml_array_decref_structs as *const u8,
        );
        // New array functions - Mutation
        builder.symbol(
            "naml_array_insert",
            crate::runtime::naml_array_insert as *const u8,
        );
        builder.symbol(
            "naml_array_remove_at",
            crate::runtime::naml_array_remove_at as *const u8,
        );
        builder.symbol(
            "naml_array_remove",
            crate::runtime::naml_array_remove as *const u8,
        );
        builder.symbol(
            "naml_array_swap",
            crate::runtime::naml_array_swap as *const u8,
        );
        // Deduplication
        builder.symbol(
            "naml_array_unique",
            crate::runtime::naml_array_unique as *const u8,
        );
        builder.symbol(
            "naml_array_compact",
            crate::runtime::naml_array_compact as *const u8,
        );
        // Backward search
        builder.symbol(
            "naml_array_last_index_of",
            crate::runtime::naml_array_last_index_of as *const u8,
        );
        builder.symbol(
            "naml_array_find_last",
            crate::runtime::naml_array_find_last as *const u8,
        );
        builder.symbol(
            "naml_array_find_last_index",
            crate::runtime::naml_array_find_last_index as *const u8,
        );
        // Array combination
        builder.symbol(
            "naml_array_concat",
            crate::runtime::naml_array_concat as *const u8,
        );
        builder.symbol(
            "naml_array_zip",
            crate::runtime::naml_array_zip as *const u8,
        );
        builder.symbol(
            "naml_array_unzip",
            crate::runtime::naml_array_unzip as *const u8,
        );
        // Splitting
        builder.symbol(
            "naml_array_chunk",
            crate::runtime::naml_array_chunk as *const u8,
        );
        builder.symbol(
            "naml_array_partition",
            crate::runtime::naml_array_partition as *const u8,
        );
        // Set operations
        builder.symbol(
            "naml_array_intersect",
            crate::runtime::naml_array_intersect as *const u8,
        );
        builder.symbol(
            "naml_array_diff",
            crate::runtime::naml_array_diff as *const u8,
        );
        builder.symbol(
            "naml_array_union",
            crate::runtime::naml_array_union as *const u8,
        );
        // Advanced iteration
        builder.symbol(
            "naml_array_take_while",
            crate::runtime::naml_array_take_while as *const u8,
        );
        builder.symbol(
            "naml_array_drop_while",
            crate::runtime::naml_array_drop_while as *const u8,
        );
        builder.symbol(
            "naml_array_reject",
            crate::runtime::naml_array_reject as *const u8,
        );
        builder.symbol(
            "naml_array_flat_apply",
            crate::runtime::naml_array_flat_apply as *const u8,
        );
        builder.symbol(
            "naml_array_scan",
            crate::runtime::naml_array_scan as *const u8,
        );
        // Random
        builder.symbol(
            "naml_array_shuffle",
            crate::runtime::naml_array_shuffle as *const u8,
        );
        builder.symbol(
            "naml_array_sample",
            crate::runtime::naml_array_sample as *const u8,
        );
        builder.symbol(
            "naml_array_sample_n",
            crate::runtime::naml_array_sample_n as *const u8,
        );

        // Struct operations
        builder.symbol(
            "naml_struct_new",
            crate::runtime::naml_struct_new as *const u8,
        );
        builder.symbol(
            "naml_struct_incref",
            crate::runtime::naml_struct_incref as *const u8,
        );
        builder.symbol(
            "naml_struct_decref",
            crate::runtime::naml_struct_decref as *const u8,
        );
        builder.symbol(
            "naml_struct_free",
            crate::runtime::naml_struct_free as *const u8,
        );
        builder.symbol(
            "naml_struct_get_field",
            crate::runtime::naml_struct_get_field as *const u8,
        );
        builder.symbol(
            "naml_struct_set_field",
            crate::runtime::naml_struct_set_field as *const u8,
        );

        // Scheduler operations
        builder.symbol("naml_spawn", crate::runtime::naml_spawn as *const u8);
        builder.symbol(
            "naml_spawn_closure",
            crate::runtime::naml_spawn_closure as *const u8,
        );
        builder.symbol(
            "naml_alloc_closure_data",
            crate::runtime::naml_alloc_closure_data as *const u8,
        );
        builder.symbol("naml_wait_all", crate::runtime::naml_wait_all as *const u8);
        builder.symbol("naml_sleep", crate::runtime::naml_sleep as *const u8);
        builder.symbol("naml_random", crate::runtime::naml_random as *const u8);
        builder.symbol(
            "naml_random_float",
            crate::runtime::naml_random_float as *const u8,
        );

        // Diagnostic builtins
        builder.symbol("naml_warn", crate::runtime::naml_warn as *const u8);
        builder.symbol("naml_error", crate::runtime::naml_error as *const u8);
        builder.symbol("naml_panic", crate::runtime::naml_panic as *const u8);
        builder.symbol(
            "naml_panic_unwrap",
            crate::runtime::naml_panic_unwrap as *const u8,
        );
        builder.symbol(
            "naml_string_concat",
            crate::runtime::naml_string_concat as *const u8,
        );

        // I/O builtins
        builder.symbol(
            "naml_read_line",
            crate::runtime::naml_read_line as *const u8,
        );
        builder.symbol("naml_read_key", crate::runtime::naml_read_key as *const u8);
        builder.symbol(
            "naml_clear_screen",
            crate::runtime::naml_clear_screen as *const u8,
        );
        builder.symbol(
            "naml_set_cursor",
            crate::runtime::naml_set_cursor as *const u8,
        );
        builder.symbol(
            "naml_hide_cursor",
            crate::runtime::naml_hide_cursor as *const u8,
        );
        builder.symbol(
            "naml_show_cursor",
            crate::runtime::naml_show_cursor as *const u8,
        );
        builder.symbol(
            "naml_terminal_width",
            crate::runtime::naml_terminal_width as *const u8,
        );
        builder.symbol(
            "naml_terminal_height",
            crate::runtime::naml_terminal_height as *const u8,
        );

        // Datetime operations
        builder.symbol(
            "naml_datetime_now_ms",
            crate::runtime::naml_datetime_now_ms as *const u8,
        );
        builder.symbol(
            "naml_datetime_now_s",
            crate::runtime::naml_datetime_now_s as *const u8,
        );
        builder.symbol(
            "naml_datetime_year",
            crate::runtime::naml_datetime_year as *const u8,
        );
        builder.symbol(
            "naml_datetime_month",
            crate::runtime::naml_datetime_month as *const u8,
        );
        builder.symbol(
            "naml_datetime_day",
            crate::runtime::naml_datetime_day as *const u8,
        );
        builder.symbol(
            "naml_datetime_hour",
            crate::runtime::naml_datetime_hour as *const u8,
        );
        builder.symbol(
            "naml_datetime_minute",
            crate::runtime::naml_datetime_minute as *const u8,
        );
        builder.symbol(
            "naml_datetime_second",
            crate::runtime::naml_datetime_second as *const u8,
        );
        builder.symbol(
            "naml_datetime_day_of_week",
            crate::runtime::naml_datetime_day_of_week as *const u8,
        );
        builder.symbol(
            "naml_datetime_format",
            crate::runtime::naml_datetime_format as *const u8,
        );

        // Metrics operations
        builder.symbol(
            "naml_metrics_perf_now",
            crate::runtime::naml_metrics_perf_now as *const u8,
        );
        builder.symbol(
            "naml_metrics_elapsed_ms",
            crate::runtime::naml_metrics_elapsed_ms as *const u8,
        );
        builder.symbol(
            "naml_metrics_elapsed_us",
            crate::runtime::naml_metrics_elapsed_us as *const u8,
        );
        builder.symbol(
            "naml_metrics_elapsed_ns",
            crate::runtime::naml_metrics_elapsed_ns as *const u8,
        );

        // Channel operations
        builder.symbol(
            "naml_channel_new",
            crate::runtime::naml_channel_new as *const u8,
        );
        builder.symbol(
            "naml_channel_send",
            crate::runtime::naml_channel_send as *const u8,
        );
        builder.symbol(
            "naml_channel_receive",
            crate::runtime::naml_channel_receive as *const u8,
        );
        builder.symbol(
            "naml_channel_close",
            crate::runtime::naml_channel_close as *const u8,
        );
        builder.symbol(
            "naml_channel_len",
            crate::runtime::naml_channel_len as *const u8,
        );
        builder.symbol(
            "naml_channel_incref",
            crate::runtime::naml_channel_incref as *const u8,
        );
        builder.symbol(
            "naml_channel_decref",
            crate::runtime::naml_channel_decref as *const u8,
        );

        // Map operations
        builder.symbol("naml_map_new", crate::runtime::naml_map_new as *const u8);
        builder.symbol("naml_map_set", crate::runtime::naml_map_set as *const u8);
        builder.symbol(
            "naml_map_set_string",
            crate::runtime::naml_map_set_string as *const u8,
        );
        builder.symbol(
            "naml_map_set_array",
            crate::runtime::naml_map_set_array as *const u8,
        );
        builder.symbol(
            "naml_map_set_map",
            crate::runtime::naml_map_set_map as *const u8,
        );
        builder.symbol(
            "naml_map_set_struct",
            crate::runtime::naml_map_set_struct as *const u8,
        );
        builder.symbol("naml_map_get", crate::runtime::naml_map_get as *const u8);
        builder.symbol(
            "naml_map_contains",
            crate::runtime::naml_map_contains as *const u8,
        );
        builder.symbol("naml_map_len", crate::runtime::naml_map_len as *const u8);
        builder.symbol(
            "naml_map_incref",
            crate::runtime::naml_map_incref as *const u8,
        );
        builder.symbol(
            "naml_map_decref",
            crate::runtime::naml_map_decref as *const u8,
        );
        builder.symbol(
            "naml_map_decref_strings",
            crate::runtime::naml_map_decref_strings as *const u8,
        );
        builder.symbol(
            "naml_map_decref_arrays",
            crate::runtime::naml_map_decref_arrays as *const u8,
        );
        builder.symbol(
            "naml_map_decref_maps",
            crate::runtime::naml_map_decref_maps as *const u8,
        );
        builder.symbol(
            "naml_map_decref_structs",
            crate::runtime::naml_map_decref_structs as *const u8,
        );

        // Map collection operations (from naml-std-collections)
        builder.symbol("naml_map_count", crate::runtime::naml_map_count as *const u8);
        builder.symbol("naml_map_contains_key", crate::runtime::naml_map_contains_key as *const u8);
        builder.symbol("naml_map_remove", crate::runtime::naml_map_remove as *const u8);
        builder.symbol("naml_map_clear", crate::runtime::naml_map_clear as *const u8);
        builder.symbol("naml_map_keys", crate::runtime::naml_map_keys as *const u8);
        builder.symbol("naml_map_values", crate::runtime::naml_map_values as *const u8);
        builder.symbol("naml_map_entries", crate::runtime::naml_map_entries as *const u8);
        builder.symbol("naml_map_first_key", crate::runtime::naml_map_first_key as *const u8);
        builder.symbol("naml_map_first_value", crate::runtime::naml_map_first_value as *const u8);
        builder.symbol("naml_map_any", crate::runtime::naml_map_any as *const u8);
        builder.symbol("naml_map_all", crate::runtime::naml_map_all as *const u8);
        builder.symbol("naml_map_count_if", crate::runtime::naml_map_count_if as *const u8);
        builder.symbol("naml_map_fold", crate::runtime::naml_map_fold as *const u8);
        builder.symbol("naml_map_transform", crate::runtime::naml_map_transform as *const u8);
        builder.symbol("naml_map_where", crate::runtime::naml_map_where as *const u8);
        builder.symbol("naml_map_reject", crate::runtime::naml_map_reject as *const u8);
        builder.symbol("naml_map_merge", crate::runtime::naml_map_merge as *const u8);
        builder.symbol("naml_map_defaults", crate::runtime::naml_map_defaults as *const u8);
        builder.symbol("naml_map_intersect", crate::runtime::naml_map_intersect as *const u8);
        builder.symbol("naml_map_diff", crate::runtime::naml_map_diff as *const u8);
        builder.symbol("naml_map_invert", crate::runtime::naml_map_invert as *const u8);
        builder.symbol("naml_map_from_arrays", crate::runtime::naml_map_from_arrays as *const u8);
        builder.symbol("naml_map_from_entries", crate::runtime::naml_map_from_entries as *const u8);

        // File system operations (from naml-std-fs)
        builder.symbol("naml_fs_read", crate::runtime::naml_fs_read as *const u8);
        builder.symbol("naml_fs_read_bytes", crate::runtime::naml_fs_read_bytes as *const u8);
        builder.symbol("naml_fs_write", crate::runtime::naml_fs_write as *const u8);
        builder.symbol("naml_fs_append", crate::runtime::naml_fs_append as *const u8);
        builder.symbol("naml_fs_write_bytes", crate::runtime::naml_fs_write_bytes as *const u8);
        builder.symbol("naml_fs_append_bytes", crate::runtime::naml_fs_append_bytes as *const u8);
        builder.symbol("naml_fs_exists", crate::runtime::naml_fs_exists as *const u8);
        builder.symbol("naml_fs_is_file", crate::runtime::naml_fs_is_file as *const u8);
        builder.symbol("naml_fs_is_dir", crate::runtime::naml_fs_is_dir as *const u8);
        builder.symbol("naml_fs_list_dir", crate::runtime::naml_fs_list_dir as *const u8);
        builder.symbol("naml_fs_mkdir", crate::runtime::naml_fs_mkdir as *const u8);
        builder.symbol("naml_fs_mkdir_all", crate::runtime::naml_fs_mkdir_all as *const u8);
        builder.symbol("naml_fs_remove", crate::runtime::naml_fs_remove as *const u8);
        builder.symbol("naml_fs_remove_all", crate::runtime::naml_fs_remove_all as *const u8);
        builder.symbol("naml_fs_join", crate::runtime::naml_fs_join as *const u8);
        builder.symbol("naml_fs_dirname", crate::runtime::naml_fs_dirname as *const u8);
        builder.symbol("naml_fs_basename", crate::runtime::naml_fs_basename as *const u8);
        builder.symbol("naml_fs_extension", crate::runtime::naml_fs_extension as *const u8);
        builder.symbol("naml_fs_absolute", crate::runtime::naml_fs_absolute as *const u8);
        builder.symbol("naml_fs_size", crate::runtime::naml_fs_size as *const u8);
        builder.symbol("naml_fs_modified", crate::runtime::naml_fs_modified as *const u8);
        builder.symbol("naml_fs_copy", crate::runtime::naml_fs_copy as *const u8);
        builder.symbol("naml_fs_rename", crate::runtime::naml_fs_rename as *const u8);
        builder.symbol("naml_io_error_new", crate::runtime::naml_io_error_new as *const u8);

        // Memory-mapped file operations
        builder.symbol("naml_fs_mmap_open", crate::runtime::naml_fs_mmap_open as *const u8);
        builder.symbol("naml_fs_mmap_len", crate::runtime::naml_fs_mmap_len as *const u8);
        builder.symbol("naml_fs_mmap_read_byte", crate::runtime::naml_fs_mmap_read_byte as *const u8);
        builder.symbol("naml_fs_mmap_write_byte", crate::runtime::naml_fs_mmap_write_byte as *const u8);
        builder.symbol("naml_fs_mmap_read", crate::runtime::naml_fs_mmap_read as *const u8);
        builder.symbol("naml_fs_mmap_write", crate::runtime::naml_fs_mmap_write as *const u8);
        builder.symbol("naml_fs_mmap_flush", crate::runtime::naml_fs_mmap_flush as *const u8);
        builder.symbol("naml_fs_mmap_close", crate::runtime::naml_fs_mmap_close as *const u8);

        // File handle operations
        builder.symbol("naml_fs_file_open", crate::runtime::naml_fs_file_open as *const u8);
        builder.symbol("naml_fs_file_close", crate::runtime::naml_fs_file_close as *const u8);
        builder.symbol("naml_fs_file_read", crate::runtime::naml_fs_file_read as *const u8);
        builder.symbol("naml_fs_file_read_line", crate::runtime::naml_fs_file_read_line as *const u8);
        builder.symbol("naml_fs_file_read_all", crate::runtime::naml_fs_file_read_all as *const u8);
        builder.symbol("naml_fs_file_write", crate::runtime::naml_fs_file_write as *const u8);
        builder.symbol("naml_fs_file_write_line", crate::runtime::naml_fs_file_write_line as *const u8);
        builder.symbol("naml_fs_file_flush", crate::runtime::naml_fs_file_flush as *const u8);
        builder.symbol("naml_fs_file_seek", crate::runtime::naml_fs_file_seek as *const u8);
        builder.symbol("naml_fs_file_tell", crate::runtime::naml_fs_file_tell as *const u8);
        builder.symbol("naml_fs_file_eof", crate::runtime::naml_fs_file_eof as *const u8);
        builder.symbol("naml_fs_file_size", crate::runtime::naml_fs_file_size as *const u8);

        // Path operations (from naml-std-path)
        builder.symbol("naml_path_join", crate::runtime::naml_path_join as *const u8);
        builder.symbol("naml_path_normalize", crate::runtime::naml_path_normalize as *const u8);
        builder.symbol("naml_path_is_absolute", crate::runtime::naml_path_is_absolute as *const u8);
        builder.symbol("naml_path_is_relative", crate::runtime::naml_path_is_relative as *const u8);
        builder.symbol("naml_path_has_root", crate::runtime::naml_path_has_root as *const u8);
        builder.symbol("naml_path_dirname", crate::runtime::naml_path_dirname as *const u8);
        builder.symbol("naml_path_basename", crate::runtime::naml_path_basename as *const u8);
        builder.symbol("naml_path_extension", crate::runtime::naml_path_extension as *const u8);
        builder.symbol("naml_path_stem", crate::runtime::naml_path_stem as *const u8);
        builder.symbol("naml_path_with_extension", crate::runtime::naml_path_with_extension as *const u8);
        builder.symbol("naml_path_components", crate::runtime::naml_path_components as *const u8);
        builder.symbol("naml_path_separator", crate::runtime::naml_path_separator as *const u8);
        builder.symbol("naml_path_to_slash", crate::runtime::naml_path_to_slash as *const u8);
        builder.symbol("naml_path_from_slash", crate::runtime::naml_path_from_slash as *const u8);
        builder.symbol("naml_path_starts_with", crate::runtime::naml_path_starts_with as *const u8);
        builder.symbol("naml_path_ends_with", crate::runtime::naml_path_ends_with as *const u8);
        builder.symbol("naml_path_strip_prefix", crate::runtime::naml_path_strip_prefix as *const u8);

        // Exception handling
        builder.symbol(
            "naml_exception_set",
            crate::runtime::naml_exception_set as *const u8,
        );
        builder.symbol(
            "naml_exception_get",
            crate::runtime::naml_exception_get as *const u8,
        );
        builder.symbol(
            "naml_exception_clear",
            crate::runtime::naml_exception_clear as *const u8,
        );
        builder.symbol(
            "naml_exception_check",
            crate::runtime::naml_exception_check as *const u8,
        );

        // Stack trace functions
        builder.symbol(
            "naml_stack_push",
            crate::runtime::naml_stack_push as *const u8,
        );
        builder.symbol(
            "naml_stack_pop",
            crate::runtime::naml_stack_pop as *const u8,
        );
        builder.symbol(
            "naml_stack_capture",
            crate::runtime::naml_stack_capture as *const u8,
        );
        builder.symbol(
            "naml_stack_clear",
            crate::runtime::naml_stack_clear as *const u8,
        );
        builder.symbol(
            "naml_stack_format",
            crate::runtime::naml_stack_format as *const u8,
        );

        // String operations
        builder.symbol(
            "naml_string_from_cstr",
            crate::runtime::naml_string_from_cstr as *const u8,
        );
        builder.symbol(
            "naml_string_print",
            crate::runtime::naml_string_print as *const u8,
        );
        builder.symbol(
            "naml_string_eq",
            crate::runtime::naml_string_eq as *const u8,
        );
        builder.symbol(
            "naml_string_incref",
            crate::runtime::naml_string_incref as *const u8,
        );
        builder.symbol(
            "naml_string_decref",
            crate::runtime::naml_string_decref as *const u8,
        );
        builder.symbol(
            "naml_string_char_at",
            crate::runtime::naml_string_char_at as *const u8,
        );
        builder.symbol(
            "naml_string_char_len",
            crate::runtime::naml_string_char_len as *const u8,
        );
        builder.symbol(
            "naml_string_is_empty",
            crate::runtime::naml_string_is_empty as *const u8,
        );
        builder.symbol(
            "naml_string_trim",
            crate::runtime::naml_string_trim as *const u8,
        );
        builder.symbol(
            "naml_string_upper",
            crate::runtime::naml_string_upper as *const u8,
        );
        builder.symbol(
            "naml_string_lower",
            crate::runtime::naml_string_lower as *const u8,
        );
        builder.symbol(
            "naml_string_contains",
            crate::runtime::naml_string_contains as *const u8,
        );
        builder.symbol(
            "naml_string_starts_with",
            crate::runtime::naml_string_starts_with as *const u8,
        );
        builder.symbol(
            "naml_string_ends_with",
            crate::runtime::naml_string_ends_with as *const u8,
        );
        builder.symbol(
            "naml_string_replace",
            crate::runtime::naml_string_replace as *const u8,
        );
        builder.symbol(
            "naml_string_replace_all",
            crate::runtime::naml_string_replace_all as *const u8,
        );
        builder.symbol(
            "naml_string_split",
            crate::runtime::naml_string_split as *const u8,
        );
        builder.symbol(
            "naml_string_join",
            crate::runtime::naml_string_join as *const u8,
        );
        builder.symbol(
            "naml_string_ltrim",
            crate::runtime::naml_string_ltrim as *const u8,
        );
        builder.symbol(
            "naml_string_rtrim",
            crate::runtime::naml_string_rtrim as *const u8,
        );
        builder.symbol(
            "naml_string_substr",
            crate::runtime::naml_string_substr as *const u8,
        );
        builder.symbol(
            "naml_string_lpad",
            crate::runtime::naml_string_lpad as *const u8,
        );
        builder.symbol(
            "naml_string_rpad",
            crate::runtime::naml_string_rpad as *const u8,
        );
        builder.symbol(
            "naml_string_repeat",
            crate::runtime::naml_string_repeat as *const u8,
        );
        builder.symbol(
            "naml_string_lines",
            crate::runtime::naml_string_lines as *const u8,
        );
        builder.symbol(
            "naml_string_chars",
            crate::runtime::naml_string_chars as *const u8,
        );

        // Type conversion operations
        builder.symbol(
            "naml_int_to_string",
            crate::runtime::naml_int_to_string as *const u8,
        );
        builder.symbol(
            "naml_float_to_string",
            crate::runtime::naml_float_to_string as *const u8,
        );
        builder.symbol(
            "naml_string_to_int",
            crate::runtime::naml_string_to_int as *const u8,
        );
        builder.symbol(
            "naml_string_to_float",
            crate::runtime::naml_string_to_float as *const u8,
        );
        builder.symbol(
            "naml_string_try_to_int",
            crate::runtime::naml_string_try_to_int as *const u8,
        );
        builder.symbol(
            "naml_string_try_to_float",
            crate::runtime::naml_string_try_to_float as *const u8,
        );

        // Bytes operations
        builder.symbol(
            "naml_bytes_new",
            crate::runtime::naml_bytes_new as *const u8,
        );
        builder.symbol(
            "naml_bytes_from",
            crate::runtime::naml_bytes_from as *const u8,
        );
        builder.symbol(
            "naml_bytes_len",
            crate::runtime::naml_bytes_len as *const u8,
        );
        builder.symbol(
            "naml_bytes_get",
            crate::runtime::naml_bytes_get as *const u8,
        );
        builder.symbol(
            "naml_bytes_set",
            crate::runtime::naml_bytes_set as *const u8,
        );
        builder.symbol(
            "naml_bytes_incref",
            crate::runtime::naml_bytes_incref as *const u8,
        );
        builder.symbol(
            "naml_bytes_decref",
            crate::runtime::naml_bytes_decref as *const u8,
        );
        builder.symbol(
            "naml_bytes_to_string",
            crate::runtime::naml_bytes_to_string as *const u8,
        );
        builder.symbol(
            "naml_string_to_bytes",
            crate::runtime::naml_string_to_bytes as *const u8,
        );

        let module = JITModule::new(builder);
        let ctx = module.make_context();

        // Built-in option type (polymorphic, treat as Option<i64> for now)
        let mut enum_defs = HashMap::new();
        enum_defs.insert(
            "option".to_string(),
            EnumDef {
                name: "option".to_string(),
                variants: vec![
                    EnumVariantDef {
                        name: "none".to_string(),
                        tag: 0,
                        field_types: vec![],
                        data_offset: 8,
                    },
                    EnumVariantDef {
                        name: "some".to_string(),
                        tag: 1,
                        field_types: vec![crate::ast::NamlType::Int],
                        data_offset: 8,
                    },
                ],
                size: 16, // 8 (tag+pad) + 8 (data)
            },
        );

        let mut compiler = Self {
            interner,
            annotations,
            source_info,
            module,
            ctx,
            functions: HashMap::new(),
            runtime_funcs: HashMap::new(),
            struct_defs: HashMap::new(),
            enum_defs,
            exception_names: HashSet::new(),
            extern_fns: HashMap::new(),
            next_type_id: 0,
            spawn_counter: 0,
            spawn_blocks: HashMap::new(),
            lambda_counter: 0,
            lambda_blocks: HashMap::new(),
            generic_functions: HashMap::new(),
        };
        compiler.declare_runtime_functions()?;
        compiler.register_builtin_exceptions();
        Ok(compiler)
    }

    /// Register built-in exception types and struct types
    fn register_builtin_exceptions(&mut self) {
        // IOError exception from std::fs module
        // Fields: path (string), code (int)
        // Note: message is implicit at offset 0 for all exceptions
        self.exception_names.insert("IOError".to_string());
        self.struct_defs.insert(
            "IOError".to_string(),
            StructDef {
                type_id: 0xFFFF_0001, // Reserved type ID for IOError
                fields: vec!["path".to_string(), "code".to_string()],
                field_heap_types: vec![Some(HeapType::String), None], // path is string, code is int
            },
        );

        // stack_frame built-in type for exception stack traces
        // Fields: function (string), file (string), line (int)
        self.struct_defs.insert(
            "stack_frame".to_string(),
            StructDef {
                type_id: 0xFFFF_0002, // Reserved type ID for stack_frame
                fields: vec![
                    "function".to_string(),
                    "file".to_string(),
                    "line".to_string(),
                ],
                field_heap_types: vec![Some(HeapType::String), Some(HeapType::String), None],
            },
        );
    }

    fn declare_runtime_functions(&mut self) -> Result<(), CodegenError> {
        let ptr = self.module.target_config().pointer_type();
        let i64t = cranelift::prelude::types::I64;
        let f64t = cranelift::prelude::types::F64;
        let i32t = cranelift::prelude::types::I32;

        let declare = |module: &mut JITModule,
                       cache: &mut HashMap<String, FuncId>,
                       name: &str,
                       params: &[cranelift::prelude::Type],
                       returns: &[cranelift::prelude::Type]|
         -> Result<(), CodegenError> {
            let mut sig = module.make_signature();
            for &p in params {
                sig.params.push(AbiParam::new(p));
            }
            for &r in returns {
                sig.returns.push(AbiParam::new(r));
            }
            let func_id = module
                .declare_function(name, Linkage::Import, &sig)
                .map_err(|e| {
                    CodegenError::JitCompile(format!("Failed to declare {}: {}", name, e))
                })?;
            cache.insert(name.to_string(), func_id);
            Ok(())
        };

        // Print functions
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_print_int",
            &[i64t],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_print_float",
            &[f64t],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_print_bool",
            &[i64t],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_print_str",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_print",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_print_newline",
            &[],
            &[],
        )?;

        // String functions
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_concat",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_eq",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_from_cstr",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_char_len",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_char_at",
            &[ptr, i64t],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_is_empty",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_trim",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_upper",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_lower",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_contains",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_starts_with",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_ends_with",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_replace",
            &[ptr, ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_replace_all",
            &[ptr, ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_split",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_join",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_ltrim",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_rtrim",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_substr",
            &[ptr, i64t, i64t],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_lpad",
            &[ptr, i64t, ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_rpad",
            &[ptr, i64t, ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_repeat",
            &[ptr, i64t],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_lines",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_chars",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_incref",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_decref",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_to_bytes",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_bytes_to_string",
            &[ptr],
            &[ptr],
        )?;

        // Type conversion
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_int_to_string",
            &[i64t],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_float_to_string",
            &[f64t],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_to_int",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_to_float",
            &[ptr],
            &[f64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_try_to_int",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_string_try_to_float",
            &[ptr, ptr],
            &[i64t],
        )?;

        // I/O
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_read_line",
            &[],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_read_key",
            &[],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_clear_screen",
            &[],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_set_cursor",
            &[i64t, i64t],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_hide_cursor",
            &[],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_show_cursor",
            &[],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_terminal_width",
            &[],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_terminal_height",
            &[],
            &[i64t],
        )?;

        // Array functions
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_new",
            &[i64t],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_from",
            &[ptr, i64t],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_push",
            &[ptr, i64t],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_get",
            &[ptr, i64t],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_set",
            &[ptr, i64t, i64t],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_len",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_pop",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_is_empty",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_shift",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_fill",
            &[ptr, i64t],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_clear",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_first",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_last",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_sum",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_min",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_max",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_reverse",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_reversed",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_take",
            &[ptr, i64t],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_drop",
            &[ptr, i64t],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_slice",
            &[ptr, i64t, i64t],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_index_of",
            &[ptr, i64t],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_contains",
            &[ptr, i64t],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_any",
            &[ptr, i64t, i64t],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_all",
            &[ptr, i64t, i64t],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_count_if",
            &[ptr, i64t, i64t],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_map",
            &[ptr, i64t, i64t],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_filter",
            &[ptr, i64t, i64t],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_find",
            &[ptr, i64t, i64t, ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_find_index",
            &[ptr, i64t, i64t],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_fold",
            &[ptr, i64t, i64t, i64t],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_flatten",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_sort",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_sort_by",
            &[ptr, i64t, i64t],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_print",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_print_strings",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_incref",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_decref",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_decref_strings",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_decref_arrays",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_decref_maps",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_decref_structs",
            &[ptr],
            &[],
        )?;
        // New array functions - Mutation
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_insert",
            &[ptr, i64t, i64t],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_remove_at",
            &[ptr, i64t],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_remove",
            &[ptr, i64t],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_swap",
            &[ptr, i64t, i64t],
            &[],
        )?;
        // Deduplication
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_unique",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_compact",
            &[ptr],
            &[ptr],
        )?;
        // Backward search
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_last_index_of",
            &[ptr, i64t],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_find_last",
            &[ptr, i64t, i64t, ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_find_last_index",
            &[ptr, i64t, i64t],
            &[i64t],
        )?;
        // Array combination
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_concat",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_zip",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_unzip",
            &[ptr],
            &[ptr],
        )?;
        // Splitting
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_chunk",
            &[ptr, i64t],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_partition",
            &[ptr, i64t, i64t],
            &[ptr],
        )?;
        // Set operations
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_intersect",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_diff",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_union",
            &[ptr, ptr],
            &[ptr],
        )?;
        // Advanced iteration
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_take_while",
            &[ptr, i64t, i64t],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_drop_while",
            &[ptr, i64t, i64t],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_reject",
            &[ptr, i64t, i64t],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_flat_apply",
            &[ptr, i64t, i64t],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_scan",
            &[ptr, i64t, i64t, i64t],
            &[ptr],
        )?;
        // Random
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_shuffle",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_sample",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_array_sample_n",
            &[ptr, i64t],
            &[ptr],
        )?;

        // Map functions
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_new",
            &[i64t],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_set",
            &[ptr, i64t, i64t],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_set_string",
            &[ptr, i64t, i64t],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_set_array",
            &[ptr, i64t, i64t],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_set_map",
            &[ptr, i64t, i64t],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_set_struct",
            &[ptr, i64t, i64t],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_get",
            &[ptr, i64t],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_contains",
            &[ptr, i64t],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_len",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_incref",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_decref",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_decref_strings",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_decref_arrays",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_decref_maps",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_decref_structs",
            &[ptr],
            &[],
        )?;

        // Map collection functions (from naml-std-collections)
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_count",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_contains_key",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_remove",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_clear",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_keys",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_values",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_entries",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_first_key",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_first_value",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_any",
            &[ptr, ptr, ptr],  // map, func_ptr, data_ptr
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_all",
            &[ptr, ptr, ptr],  // map, func_ptr, data_ptr
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_count_if",
            &[ptr, ptr, ptr],  // map, func_ptr, data_ptr
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_fold",
            &[ptr, i64t, ptr, ptr],  // map, initial, func_ptr, data_ptr
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_transform",
            &[ptr, ptr, ptr],  // map, func_ptr, data_ptr
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_where",
            &[ptr, ptr, ptr],  // map, func_ptr, data_ptr
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_reject",
            &[ptr, ptr, ptr],  // map, func_ptr, data_ptr
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_merge",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_defaults",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_intersect",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_diff",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_invert",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_from_arrays",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_map_from_entries",
            &[ptr],
            &[ptr],
        )?;

        // Struct functions
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_struct_new",
            &[i32t, i32t],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_struct_get_field",
            &[ptr, i32t],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_struct_set_field",
            &[ptr, i32t, i64t],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_struct_incref",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_struct_decref",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_struct_free",
            &[ptr],
            &[],
        )?;

        // Channel functions
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_channel_new",
            &[i64t],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_channel_send",
            &[ptr, i64t],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_channel_receive",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_channel_close",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_channel_len",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_channel_incref",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_channel_decref",
            &[ptr],
            &[],
        )?;

        // Scheduler/runtime
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_spawn",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_spawn_closure",
            &[ptr, ptr, i64t],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_alloc_closure_data",
            &[i64t],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_wait_all",
            &[],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_sleep",
            &[i64t],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_random",
            &[i64t, i64t],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_random_float",
            &[],
            &[f64t],
        )?;

        // Diagnostics
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_warn",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_error",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_panic",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_panic_unwrap",
            &[],
            &[],
        )?;

        // Exception handling
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_exception_set",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_exception_get",
            &[],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_exception_clear",
            &[],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_exception_check",
            &[],
            &[i64t],
        )?;

        // File system operations
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_read", &[ptr], &[ptr])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_read_bytes", &[ptr], &[ptr])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_write", &[ptr, ptr], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_append", &[ptr, ptr], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_write_bytes", &[ptr, ptr], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_append_bytes", &[ptr, ptr], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_exists", &[ptr], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_is_file", &[ptr], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_is_dir", &[ptr], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_list_dir", &[ptr], &[ptr])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_mkdir", &[ptr], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_mkdir_all", &[ptr], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_remove", &[ptr], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_remove_all", &[ptr], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_join", &[ptr], &[ptr])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_dirname", &[ptr], &[ptr])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_basename", &[ptr], &[ptr])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_extension", &[ptr], &[ptr])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_absolute", &[ptr], &[ptr])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_size", &[ptr], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_modified", &[ptr], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_copy", &[ptr, ptr], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_rename", &[ptr, ptr], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_io_error_new", &[ptr, ptr, i64t], &[ptr])?;

        // Memory-mapped file operations
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_mmap_open", &[ptr, i64t], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_mmap_len", &[i64t], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_mmap_read_byte", &[i64t, i64t], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_mmap_write_byte", &[i64t, i64t, i64t], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_mmap_read", &[i64t, i64t, i64t], &[ptr])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_mmap_write", &[i64t, i64t, ptr], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_mmap_flush", &[i64t], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_mmap_close", &[i64t], &[i64t])?;

        // File handle operations
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_file_open", &[ptr, ptr], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_file_close", &[i64t], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_file_read", &[i64t, i64t], &[ptr])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_file_read_line", &[i64t], &[ptr])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_file_read_all", &[i64t], &[ptr])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_file_write", &[i64t, ptr], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_file_write_line", &[i64t, ptr], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_file_flush", &[i64t], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_file_seek", &[i64t, i64t, i64t], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_file_tell", &[i64t], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_file_eof", &[i64t], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_fs_file_size", &[i64t], &[i64t])?;

        // Path operations
        declare(&mut self.module, &mut self.runtime_funcs, "naml_path_join", &[ptr], &[ptr])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_path_normalize", &[ptr], &[ptr])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_path_is_absolute", &[ptr], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_path_is_relative", &[ptr], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_path_has_root", &[ptr], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_path_dirname", &[ptr], &[ptr])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_path_basename", &[ptr], &[ptr])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_path_extension", &[ptr], &[ptr])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_path_stem", &[ptr], &[ptr])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_path_with_extension", &[ptr, ptr], &[ptr])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_path_components", &[ptr], &[ptr])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_path_separator", &[], &[ptr])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_path_to_slash", &[ptr], &[ptr])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_path_from_slash", &[ptr], &[ptr])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_path_starts_with", &[ptr, ptr], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_path_ends_with", &[ptr, ptr], &[i64t])?;
        declare(&mut self.module, &mut self.runtime_funcs, "naml_path_strip_prefix", &[ptr, ptr], &[ptr])?;

        // Bytes operations
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_bytes_new",
            &[i64t],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_bytes_from",
            &[ptr, i64t],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_bytes_len",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_bytes_get",
            &[ptr, i64t],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_bytes_set",
            &[ptr, i64t, i64t],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_bytes_incref",
            &[ptr],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_bytes_decref",
            &[ptr],
            &[],
        )?;

        // Datetime operations
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_datetime_now_ms",
            &[],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_datetime_now_s",
            &[],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_datetime_year",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_datetime_month",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_datetime_day",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_datetime_hour",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_datetime_minute",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_datetime_second",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_datetime_day_of_week",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_datetime_format",
            &[i64t, ptr],
            &[ptr],
        )?;

        // Metrics operations
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_metrics_perf_now",
            &[],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_metrics_elapsed_ms",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_metrics_elapsed_us",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_metrics_elapsed_ns",
            &[i64t],
            &[i64t],
        )?;

        // Stack trace functions
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_stack_push",
            &[ptr, ptr, i64t],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_stack_pop",
            &[],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_stack_capture",
            &[],
            &[ptr],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_stack_clear",
            &[],
            &[],
        )?;
        declare(
            &mut self.module,
            &mut self.runtime_funcs,
            "naml_stack_format",
            &[ptr],
            &[ptr],
        )?;

        Ok(())
    }

    pub fn compile(&mut self, ast: &'a SourceFile<'a>) -> Result<(), CodegenError> {
        for item in &ast.items {
            if let crate::ast::Item::Struct(struct_item) = item {
                let name = self.interner.resolve(&struct_item.name.symbol).to_string();
                let mut fields = Vec::new();
                let mut field_heap_types = Vec::new();

                for f in &struct_item.fields {
                    fields.push(self.interner.resolve(&f.name.symbol).to_string());
                    field_heap_types.push(get_heap_type(&f.ty));
                }

                let type_id = self.next_type_id;
                self.next_type_id += 1;

                self.struct_defs.insert(
                    name,
                    StructDef {
                        type_id,
                        fields,
                        field_heap_types,
                    },
                );
            }
        }

        // Collect exception definitions (treated like structs for codegen)
        for item in &ast.items {
            if let crate::ast::Item::Exception(exception_item) = item {
                let name = self
                    .interner
                    .resolve(&exception_item.name.symbol)
                    .to_string();
                let mut fields = Vec::new();
                let mut field_heap_types = Vec::new();

                for f in &exception_item.fields {
                    fields.push(self.interner.resolve(&f.name.symbol).to_string());
                    field_heap_types.push(get_heap_type(&f.ty));
                }

                let type_id = self.next_type_id;
                self.next_type_id += 1;

                // Exception treated as a struct with its fields
                self.exception_names.insert(name.clone());
                self.struct_defs.insert(
                    name,
                    StructDef {
                        type_id,
                        fields,
                        field_heap_types,
                    },
                );
            }
        }

        // Collect enum definitions
        for item in &ast.items {
            if let crate::ast::Item::Enum(enum_item) = item {
                let name = self.interner.resolve(&enum_item.name.symbol).to_string();
                let mut variants = Vec::new();
                let mut max_data_size: usize = 0;

                for (tag, variant) in enum_item.variants.iter().enumerate() {
                    let variant_name = self.interner.resolve(&variant.name.symbol).to_string();
                    let field_types = variant.fields.clone().unwrap_or_default();
                    let data_size = field_types.len() * 8; // Each field is 8 bytes
                    max_data_size = max_data_size.max(data_size);

                    variants.push(EnumVariantDef {
                        name: variant_name,
                        tag: tag as u32,
                        field_types,
                        data_offset: 8, // After tag + padding
                    });
                }

                // Align to 8 bytes
                let size = 8 + max_data_size.div_ceil(8) * 8;

                self.enum_defs.insert(
                    name.clone(),
                    EnumDef {
                        name,
                        variants,
                        size,
                    },
                );
            }
        }

        // Collect extern function declarations
        for item in &ast.items {
            if let crate::ast::Item::Extern(extern_item) = item {
                let name = self.interner.resolve(&extern_item.name.symbol).to_string();
                let link_name = if let Some(ref ln) = extern_item.link_name {
                    self.interner.resolve(&ln.symbol).to_string()
                } else {
                    name.clone()
                };

                let param_types: Vec<_> = extern_item.params.iter().map(|p| p.ty.clone()).collect();

                self.extern_fns.insert(
                    name,
                    ExternFn {
                        link_name,
                        param_types,
                        return_type: extern_item.return_ty.clone(),
                    },
                );
            }
        }

        // Generate per-struct decref functions for structs with heap fields
        self.generate_struct_decref_functions()?;

        // Scan for spawn blocks and collect captured variable info
        for item in &ast.items {
            if let Item::Function(f) = item
                && let Some(ref body) = f.body
            {
                self.scan_for_spawn_blocks(body)?;
            }
        }

        // Declare spawn trampolines
        for (id, info) in &self.spawn_blocks.clone() {
            self.declare_spawn_trampoline(*id, info)?;
        }

        // Compile spawn trampolines (must be done before regular functions)
        for info in self.spawn_blocks.clone().values() {
            self.compile_spawn_trampoline(info)?;
        }

        // Declare lambda functions
        for (id, info) in &self.lambda_blocks.clone() {
            self.declare_lambda_function(info)?;
            let _ = id; // suppress unused warning
        }

        // Compile lambda functions (must be done before regular functions)
        for info in self.lambda_blocks.clone().values() {
            self.compile_lambda_function(info)?;
        }

        // Declare all functions first (standalone and methods)
        // Skip generic functions - they will be monomorphized
        for item in &ast.items {
            if let Item::Function(f) = item {
                let is_generic = !f.generics.is_empty();
                if is_generic && f.receiver.is_none() {
                    // Store generic function for later monomorphization
                    let name = self.interner.resolve(&f.name.symbol).to_string();
                    self.generic_functions.insert(name, f as *const _);
                } else if f.receiver.is_none() {
                    self.declare_function(f)?;
                } else {
                    self.declare_method(f)?;
                }
            }
        }

        // Process monomorphizations - declare and compile specialized versions
        self.process_monomorphizations()?;

        // Compile standalone functions (skip generic functions)
        for item in &ast.items {
            if let Item::Function(f) = item
                && f.receiver.is_none()
                && f.body.is_some()
                && f.generics.is_empty()
            {
                self.compile_function(f)?;
            }
        }

        // Compile methods
        for item in &ast.items {
            if let Item::Function(f) = item
                && f.receiver.is_some()
                && f.body.is_some()
            {
                self.compile_method(f)?;
            }
        }

        Ok(())
    }

    pub fn compile_module_source(&mut self, source: &str) -> Result<(), CodegenError> {
        let (tokens, mut module_interner) = crate::lexer::tokenize(source);
        let arena = crate::ast::AstArena::new();
        let parse_result = crate::parser::parse(&tokens, source, &arena);
        if !parse_result.errors.is_empty() {
            return Err(CodegenError::JitCompile(
                "parse errors in imported module".into(),
            ));
        }

        let type_result =
            crate::typechecker::check_with_types(&parse_result.ast, &mut module_interner, None);

        let saved_interner = self.interner;
        let saved_annotations = self.annotations;
        self.interner = unsafe { std::mem::transmute::<&Rodeo, &Rodeo>(&module_interner) };
        self.annotations = unsafe {
            std::mem::transmute::<&TypeAnnotations, &TypeAnnotations>(&type_result.annotations)
        };

        for item in &parse_result.ast.items {
            if let Item::Function(f) = item {
                if f.is_public && f.receiver.is_none() && f.body.is_some() && f.generics.is_empty()
                {
                    self.declare_function(f)?;
                }
            }
        }
        for item in &parse_result.ast.items {
            if let Item::Function(f) = item {
                if f.is_public && f.receiver.is_none() && f.body.is_some() && f.generics.is_empty()
                {
                    self.compile_function(f)?;
                }
            }
        }

        self.interner = saved_interner;
        self.annotations = saved_annotations;
        Ok(())
    }

    fn generate_struct_decref_functions(&mut self) -> Result<(), CodegenError> {
        // Collect structs that need specialized decref functions
        let structs_with_heap_fields: Vec<(String, StructDef)> = self
            .struct_defs
            .iter()
            .filter(|(_, def)| def.field_heap_types.iter().any(|ht| ht.is_some()))
            .map(|(name, def)| (name.clone(), def.clone()))
            .collect();

        for (struct_name, struct_def) in structs_with_heap_fields {
            self.generate_struct_decref(&struct_name, &struct_def)?;
        }

        Ok(())
    }

    fn generate_struct_decref(
        &mut self,
        struct_name: &str,
        struct_def: &StructDef,
    ) -> Result<(), CodegenError> {
        let ptr_type = self.module.target_config().pointer_type();
        let func_name = format!("naml_struct_decref_{}", struct_name);

        // Function signature: fn(struct_ptr: *mut NamlStruct)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(ptr_type));

        let func_id = self
            .module
            .declare_function(&func_name, Linkage::Local, &sig)
            .map_err(|e| {
                CodegenError::JitCompile(format!("Failed to declare {}: {}", func_name, e))
            })?;

        // Store for later reference
        self.functions.insert(func_name.clone(), func_id);

        self.ctx.func.signature = sig;

        let mut builder_ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);

        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        let struct_ptr = builder.block_params(entry_block)[0];

        // Check if ptr is null
        let zero = builder.ins().iconst(ptr_type, 0);
        let is_null = builder.ins().icmp(IntCC::Equal, struct_ptr, zero);

        let null_block = builder.create_block();
        let decref_block = builder.create_block();

        builder
            .ins()
            .brif(is_null, null_block, &[], decref_block, &[]);

        // Null case: just return
        builder.switch_to_block(null_block);
        builder.seal_block(null_block);
        builder.ins().return_(&[]);

        // Non-null case: decref the struct
        builder.switch_to_block(decref_block);
        builder.seal_block(decref_block);

        // Call atomic decref on refcount (at offset 0 in HeapHeader)
        // HeapHeader layout: refcount (8 bytes), tag (1 byte), pad (7 bytes)
        // Use atomic_rmw to safely decrement refcount in multi-threaded scenarios
        let one = builder.ins().iconst(cranelift::prelude::types::I64, 1);
        let old_refcount = builder.ins().atomic_rmw(
            cranelift::prelude::types::I64,
            MemFlags::new(),
            AtomicRmwOp::Sub,
            struct_ptr,
            one,
        );

        // Check if old refcount was 1 (meaning it's now 0 and we should free)
        let should_free = builder.ins().icmp(IntCC::Equal, old_refcount, one);

        let free_block = builder.create_block();
        let done_block = builder.create_block();

        builder
            .ins()
            .brif(should_free, free_block, &[], done_block, &[]);
        builder.switch_to_block(free_block);
        builder.seal_block(free_block);
        builder.ins().fence();

        // Struct memory layout after header:
        // - type_id: u32 (offset 16)
        // - field_count: u32 (offset 20)
        // - fields[]: i64 (offset 24+)
        let base_field_offset: i32 = 24; // sizeof(HeapHeader) + type_id + field_count

        for (field_idx, heap_type) in struct_def.field_heap_types.iter().enumerate() {
            if let Some(ht) = heap_type {
                let field_offset = base_field_offset + (field_idx as i32 * 8);
                let field_val = builder.ins().load(
                    cranelift::prelude::types::I64,
                    MemFlags::new(),
                    struct_ptr,
                    field_offset,
                );

                let field_is_null = builder.ins().icmp(IntCC::Equal, field_val, zero);
                let decref_field_block = builder.create_block();
                let next_field_block = builder.create_block();

                builder.ins().brif(
                    field_is_null,
                    next_field_block,
                    &[],
                    decref_field_block,
                    &[],
                );
                builder.switch_to_block(decref_field_block);
                builder.seal_block(decref_field_block);

                let decref_func_name = match ht {
                    HeapType::String => "naml_string_decref",
                    HeapType::Array(None) => "naml_array_decref",
                    HeapType::Array(Some(elem_type)) => match elem_type.as_ref() {
                        HeapType::String => "naml_array_decref_strings",
                        HeapType::Array(_) => "naml_array_decref_arrays",
                        HeapType::Map(_) => "naml_array_decref_maps",
                        HeapType::Struct(_) => "naml_array_decref_structs",
                    },
                    HeapType::Map(None) => "naml_map_decref",
                    HeapType::Map(Some(val_type)) => match val_type.as_ref() {
                        HeapType::String => "naml_map_decref_strings",
                        HeapType::Array(_) => "naml_map_decref_arrays",
                        HeapType::Map(_) => "naml_map_decref_maps",
                        HeapType::Struct(_) => "naml_map_decref_structs",
                    },
                    HeapType::Struct(None) => "naml_struct_decref",
                    HeapType::Struct(Some(_)) => "naml_struct_decref",
                };

                let decref_func_id =
                    *self.runtime_funcs.get(decref_func_name).ok_or_else(|| {
                        CodegenError::JitCompile(format!(
                            "Unknown runtime function: {}",
                            decref_func_name
                        ))
                    })?;
                let decref_func_ref = self
                    .module
                    .declare_func_in_func(decref_func_id, builder.func);
                builder.ins().call(decref_func_ref, &[field_val]);
                builder.ins().jump(next_field_block, &[]);

                builder.switch_to_block(next_field_block);
                builder.seal_block(next_field_block);
            }
        }

        // Call naml_struct_free to deallocate the struct memory
        let free_func_id = *self.runtime_funcs.get("naml_struct_free").ok_or_else(|| {
            CodegenError::JitCompile("Unknown runtime function: naml_struct_free".to_string())
        })?;
        let free_func_ref = self.module.declare_func_in_func(free_func_id, builder.func);
        builder.ins().call(free_func_ref, &[struct_ptr]);
        builder.ins().jump(done_block, &[]);

        // Done block: return
        builder.switch_to_block(done_block);
        builder.seal_block(done_block);
        builder.ins().return_(&[]);

        builder.finalize();

        let func_name_clone = func_name.clone();
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            self.module.define_function(func_id, &mut self.ctx)
        }));

        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                return Err(CodegenError::JitCompile(format!(
                    "Failed to define {}: {}",
                    func_name, e
                )));
            }
            Err(panic_info) => {
                let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown internal error".to_string()
                };
                return Err(convert_cranelift_error(&panic_msg, &func_name_clone));
            }
        }

        self.ctx.clear();

        Ok(())
    }

    fn process_monomorphizations(&mut self) -> Result<(), CodegenError> {
        let monomorphizations: Vec<_> = self
            .annotations
            .get_monomorphizations()
            .values()
            .cloned()
            .collect();

        for mono_info in monomorphizations {
            let func_name = self.interner.resolve(&mono_info.function_name).to_string();

            // Get the generic function AST
            let func_ptr = match self.generic_functions.get(&func_name) {
                Some(ptr) => *ptr,
                None => continue,
            };

            let func = unsafe { &*func_ptr };
            let mut type_substitutions = HashMap::new();
            for (param, arg_ty) in func.generics.iter().zip(mono_info.type_args.iter()) {
                let param_name = self.interner.resolve(&param.name.symbol).to_string();
                let concrete_name = self.mangle_type_name(arg_ty);
                type_substitutions.insert(param_name, concrete_name);
            }

            // Declare the monomorphized function
            self.declare_monomorphized_function(func, &mono_info.mangled_name)?;

            // Compile the monomorphized function with type substitutions
            self.compile_monomorphized_function(func, &mono_info.mangled_name, type_substitutions)?;
        }

        Ok(())
    }

    fn mangle_type_name(&self, ty: &Type) -> String {
        match ty {
            Type::Int => "int".to_string(),
            Type::Uint => "uint".to_string(),
            Type::Float => "float".to_string(),
            Type::Bool => "bool".to_string(),
            Type::String => "string".to_string(),
            Type::Bytes => "bytes".to_string(),
            Type::Unit => "unit".to_string(),
            Type::Struct(s) => self.interner.resolve(&s.name).to_string(),
            Type::Enum(e) => self.interner.resolve(&e.name).to_string(),
            Type::Generic(name, _) => self.interner.resolve(name).to_string(),
            _ => "unknown".to_string(),
        }
    }

    fn declare_monomorphized_function(
        &mut self,
        func: &FunctionItem<'_>,
        mangled_name: &str,
    ) -> Result<FuncId, CodegenError> {
        let mut sig = self.module.make_signature();

        sig.params
            .push(AbiParam::new(cranelift::prelude::types::I64));

        for param in &func.params {
            let ty = types::naml_to_cranelift(&param.ty);
            sig.params.push(AbiParam::new(ty));
        }

        if let Some(ref return_ty) = func.return_ty {
            let ty = types::naml_to_cranelift(return_ty);
            sig.returns.push(AbiParam::new(ty));
        }

        let func_id = self
            .module
            .declare_function(mangled_name, Linkage::Export, &sig)
            .map_err(|e| {
                CodegenError::JitCompile(format!(
                    "Failed to declare monomorphized function '{}': {}",
                    mangled_name, e
                ))
            })?;

        self.functions.insert(mangled_name.to_string(), func_id);

        Ok(func_id)
    }

    fn compile_monomorphized_function(
        &mut self,
        func: &FunctionItem<'_>,
        mangled_name: &str,
        type_substitutions: HashMap<String, String>,
    ) -> Result<(), CodegenError> {
        let func_id = *self.functions.get(mangled_name).ok_or_else(|| {
            CodegenError::JitCompile(format!(
                "Monomorphized function '{}' not declared",
                mangled_name
            ))
        })?;

        self.ctx.func.signature = self
            .module
            .declarations()
            .get_function_decl(func_id)
            .signature
            .clone();
        self.ctx.func.name = cranelift_codegen::ir::UserFuncName::user(0, func_id.as_u32());

        let mut builder_ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);

        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        let func_return_type = func
            .return_ty
            .as_ref()
            .map(|ty| types::naml_to_cranelift(ty));

        let mut ctx = CompileContext {
            interner: self.interner,
            module: &mut self.module,
            functions: &self.functions,
            runtime_funcs: &self.runtime_funcs,
            struct_defs: &self.struct_defs,
            enum_defs: &self.enum_defs,
            exception_names: &self.exception_names,
            extern_fns: &self.extern_fns,
            variables: HashMap::new(),
            var_heap_types: HashMap::new(),
            var_counter: 0,
            block_terminated: false,
            loop_exit_block: None,
            loop_header_block: None,
            spawn_blocks: &self.spawn_blocks,
            current_spawn_id: 0,
            lambda_blocks: &self.lambda_blocks,
            current_lambda_id: 0,
            annotations: self.annotations,
            type_substitutions,
            func_return_type,
        };

        for (i, param) in func.params.iter().enumerate() {
            let param_name = self.interner.resolve(&param.name.symbol).to_string();
            let val = builder.block_params(entry_block)[i + 1];
            let var = Variable::new(ctx.var_counter);
            ctx.var_counter += 1;
            let ty = types::naml_to_cranelift(&param.ty);
            builder.declare_var(var, ty);
            builder.def_var(var, val);
            ctx.variables.insert(param_name, var);
        }

        if let Some(ref body) = func.body {
            for stmt in &body.statements {
                compile_statement(&mut ctx, &mut builder, stmt)?;
                if ctx.block_terminated {
                    break;
                }
            }
        }

        if !ctx.block_terminated && func.return_ty.is_none() {
            emit_cleanup_all_vars(&mut ctx, &mut builder, None)?;
            builder.ins().return_(&[]);
        }

        builder.finalize();

        let mangled_name_clone = mangled_name.to_string();
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            self.module.define_function(func_id, &mut self.ctx)
        }));

        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                return Err(CodegenError::JitCompile(format!(
                    "Failed to define monomorphized function '{}': {}",
                    mangled_name, e
                )));
            }
            Err(panic_info) => {
                let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown internal error".to_string()
                };
                return Err(convert_cranelift_error(&panic_msg, &mangled_name_clone));
            }
        }

        self.module.clear_context(&mut self.ctx);

        Ok(())
    }

    fn scan_for_spawn_blocks(
        &mut self,
        block: &crate::ast::BlockStmt<'_>,
    ) -> Result<(), CodegenError> {
        for stmt in &block.statements {
            self.scan_statement_for_spawns(stmt)?;
        }
        Ok(())
    }

    fn scan_statement_for_spawns(&mut self, stmt: &Statement<'_>) -> Result<(), CodegenError> {
        match stmt {
            Statement::Expression(expr_stmt) => {
                self.scan_expression_for_spawns(&expr_stmt.expr)?;
            }
            Statement::If(if_stmt) => {
                self.scan_expression_for_spawns(&if_stmt.condition)?;
                self.scan_for_spawn_blocks(&if_stmt.then_branch)?;
                if let Some(ref else_branch) = if_stmt.else_branch {
                    match else_branch {
                        crate::ast::ElseBranch::ElseIf(elif) => {
                            self.scan_statement_for_spawns(&Statement::If(*elif.clone()))?;
                        }
                        crate::ast::ElseBranch::Else(block) => {
                            self.scan_for_spawn_blocks(block)?;
                        }
                    }
                }
            }
            Statement::While(while_stmt) => {
                self.scan_expression_for_spawns(&while_stmt.condition)?;
                self.scan_for_spawn_blocks(&while_stmt.body)?;
            }
            Statement::For(for_stmt) => {
                self.scan_expression_for_spawns(&for_stmt.iterable)?;
                self.scan_for_spawn_blocks(&for_stmt.body)?;
            }
            Statement::Loop(loop_stmt) => {
                self.scan_for_spawn_blocks(&loop_stmt.body)?;
            }
            Statement::Switch(switch_stmt) => {
                self.scan_expression_for_spawns(&switch_stmt.scrutinee)?;
                for case in &switch_stmt.cases {
                    self.scan_for_spawn_blocks(&case.body)?;
                }
                if let Some(ref default) = switch_stmt.default {
                    self.scan_for_spawn_blocks(default)?;
                }
            }
            Statement::Block(block) => {
                self.scan_for_spawn_blocks(block)?;
            }
            Statement::Var(var_stmt) => {
                if let Some(ref init) = var_stmt.init {
                    self.scan_expression_for_spawns(init)?;
                }
            }
            Statement::Assign(assign_stmt) => {
                self.scan_expression_for_spawns(&assign_stmt.value)?;
            }
            Statement::Return(ret_stmt) => {
                if let Some(ref value) = ret_stmt.value {
                    self.scan_expression_for_spawns(value)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn scan_expression_for_spawns(&mut self, expr: &Expression<'_>) -> Result<(), CodegenError> {
        match expr {
            Expression::Spawn(spawn_expr) => {
                // Found a spawn block - collect captured variables
                let captured = self.collect_captured_vars_expr(spawn_expr.body);
                let id = self.spawn_counter;
                self.spawn_counter += 1;
                let func_name = format!("__spawn_{}", id);

                // Store raw pointer to body for deferred trampoline compilation
                // Safety: Only used within the same compile() call
                // Note: spawn_expr.body is already a &BlockExpr, so we cast it directly
                #[allow(clippy::unnecessary_cast)]
                let body_ptr = spawn_expr.body as *const crate::ast::BlockExpr<'_>
                    as *const crate::ast::BlockExpr<'static>;

                self.spawn_blocks.insert(
                    id,
                    SpawnBlockInfo {
                        id,
                        func_name,
                        captured_vars: captured,
                        body_ptr,
                    },
                );

                // Also scan inside spawn block for nested spawns
                self.scan_for_spawn_blocks_expr(spawn_expr.body)?;
            }
            Expression::Lambda(lambda_expr) => {
                // Found a lambda - collect captured variables
                let captured = self.collect_captured_vars_for_lambda(lambda_expr);
                let id = self.lambda_counter;
                self.lambda_counter += 1;
                let func_name = format!("__lambda_{}", id);

                // Collect parameter names
                let param_names: Vec<String> = lambda_expr
                    .params
                    .iter()
                    .map(|p| self.interner.resolve(&p.name.symbol).to_string())
                    .collect();

                // Store raw pointer to body for deferred lambda compilation
                #[allow(clippy::unnecessary_cast)]
                let body_ptr = lambda_expr.body as *const crate::ast::Expression<'_>
                    as *const crate::ast::Expression<'static>;

                self.lambda_blocks.insert(
                    id,
                    LambdaInfo {
                        id,
                        func_name,
                        captured_vars: captured,
                        param_names,
                        body_ptr,
                    },
                );

                // Scan lambda body for nested spawns/lambdas
                self.scan_expression_for_spawns(lambda_expr.body)?;
            }
            Expression::Binary(bin) => {
                self.scan_expression_for_spawns(bin.left)?;
                self.scan_expression_for_spawns(bin.right)?;
            }
            Expression::Unary(un) => {
                self.scan_expression_for_spawns(un.operand)?;
            }
            Expression::Call(call) => {
                self.scan_expression_for_spawns(call.callee)?;
                for arg in &call.args {
                    self.scan_expression_for_spawns(arg)?;
                }
            }
            Expression::MethodCall(method) => {
                self.scan_expression_for_spawns(method.receiver)?;
                for arg in &method.args {
                    self.scan_expression_for_spawns(arg)?;
                }
            }
            Expression::Index(idx) => {
                self.scan_expression_for_spawns(idx.base)?;
                self.scan_expression_for_spawns(idx.index)?;
            }
            Expression::Array(arr) => {
                for elem in &arr.elements {
                    self.scan_expression_for_spawns(elem)?;
                }
            }
            Expression::If(if_expr) => {
                self.scan_expression_for_spawns(if_expr.condition)?;
                self.scan_for_spawn_blocks_expr(if_expr.then_branch)?;
                self.scan_else_branch_for_spawns(&if_expr.else_branch)?;
            }
            Expression::Block(block) => {
                self.scan_for_spawn_blocks_expr(block)?;
            }
            Expression::Grouped(grouped) => {
                self.scan_expression_for_spawns(grouped.inner)?;
            }
            Expression::Ternary(ternary) => {
                self.scan_expression_for_spawns(ternary.condition)?;
                self.scan_expression_for_spawns(ternary.true_expr)?;
                self.scan_expression_for_spawns(ternary.false_expr)?;
            }
            Expression::Elvis(elvis) => {
                self.scan_expression_for_spawns(elvis.left)?;
                self.scan_expression_for_spawns(elvis.right)?;
            }
            Expression::FallibleCast(cast) => {
                self.scan_expression_for_spawns(cast.expr)?;
            }
            Expression::ForceUnwrap(unwrap) => {
                self.scan_expression_for_spawns(unwrap.expr)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn scan_for_spawn_blocks_expr(
        &mut self,
        block: &crate::ast::BlockExpr<'_>,
    ) -> Result<(), CodegenError> {
        for stmt in &block.statements {
            self.scan_statement_for_spawns(stmt)?;
        }
        if let Some(tail) = block.tail {
            self.scan_expression_for_spawns(tail)?;
        }
        Ok(())
    }

    fn scan_else_branch_for_spawns(
        &mut self,
        else_branch: &Option<crate::ast::ElseExpr<'_>>,
    ) -> Result<(), CodegenError> {
        if let Some(branch) = else_branch {
            match branch {
                crate::ast::ElseExpr::ElseIf(elif) => {
                    self.scan_expression_for_spawns(elif.condition)?;
                    self.scan_for_spawn_blocks_expr(elif.then_branch)?;
                    self.scan_else_branch_for_spawns(&elif.else_branch)?;
                }
                crate::ast::ElseExpr::Else(block) => {
                    self.scan_for_spawn_blocks_expr(block)?;
                }
            }
        }
        Ok(())
    }

    fn collect_captured_vars_expr(&self, block: &crate::ast::BlockExpr<'_>) -> Vec<String> {
        let mut captured = Vec::new();
        let mut defined = std::collections::HashSet::new();
        self.collect_vars_in_block_expr(block, &mut captured, &mut defined);
        captured
    }

    fn collect_captured_vars_for_lambda(&self, lambda: &crate::ast::LambdaExpr<'_>) -> Vec<String> {
        let mut captured = Vec::new();
        let mut defined = std::collections::HashSet::new();

        // Lambda parameters are defined within the lambda scope
        for param in &lambda.params {
            let name = self.interner.resolve(&param.name.symbol).to_string();
            defined.insert(name);
        }

        // Collect from body (which is an Expression - typically a Block)
        self.collect_vars_in_expression(lambda.body, &mut captured, &defined);

        captured
    }

    fn collect_vars_in_block(
        &self,
        block: &crate::ast::BlockStmt<'_>,
        captured: &mut Vec<String>,
        defined: &mut std::collections::HashSet<String>,
    ) {
        for stmt in &block.statements {
            self.collect_vars_in_statement(stmt, captured, defined);
        }
    }

    fn collect_vars_in_block_expr(
        &self,
        block: &crate::ast::BlockExpr<'_>,
        captured: &mut Vec<String>,
        defined: &mut std::collections::HashSet<String>,
    ) {
        for stmt in &block.statements {
            self.collect_vars_in_statement(stmt, captured, defined);
        }
        if let Some(tail) = block.tail {
            self.collect_vars_in_expression(tail, captured, defined);
        }
    }

    fn collect_vars_in_statement(
        &self,
        stmt: &Statement<'_>,
        captured: &mut Vec<String>,
        defined: &mut std::collections::HashSet<String>,
    ) {
        match stmt {
            Statement::Var(var_stmt) => {
                if let Some(ref init) = var_stmt.init {
                    self.collect_vars_in_expression(init, captured, defined);
                }
                let name = self.interner.resolve(&var_stmt.name.symbol).to_string();
                defined.insert(name);
            }
            Statement::Expression(expr_stmt) => {
                self.collect_vars_in_expression(&expr_stmt.expr, captured, defined);
            }
            Statement::Assign(assign) => {
                self.collect_vars_in_expression(&assign.target, captured, defined);
                self.collect_vars_in_expression(&assign.value, captured, defined);
            }
            Statement::If(if_stmt) => {
                self.collect_vars_in_expression(&if_stmt.condition, captured, defined);
                self.collect_vars_in_block(&if_stmt.then_branch, captured, defined);
            }
            Statement::While(while_stmt) => {
                self.collect_vars_in_expression(&while_stmt.condition, captured, defined);
                self.collect_vars_in_block(&while_stmt.body, captured, defined);
            }
            Statement::For(for_stmt) => {
                self.collect_vars_in_expression(&for_stmt.iterable, captured, defined);
                let val_name = self.interner.resolve(&for_stmt.value.symbol).to_string();
                defined.insert(val_name);
                if let Some(ref idx) = for_stmt.index {
                    let idx_name = self.interner.resolve(&idx.symbol).to_string();
                    defined.insert(idx_name);
                }
                self.collect_vars_in_block(&for_stmt.body, captured, defined);
            }
            Statement::Return(ret) => {
                if let Some(ref value) = ret.value {
                    self.collect_vars_in_expression(value, captured, defined);
                }
            }
            _ => {}
        }
    }

    fn collect_vars_in_expression(
        &self,
        expr: &Expression<'_>,
        captured: &mut Vec<String>,
        defined: &std::collections::HashSet<String>,
    ) {
        match expr {
            Expression::Identifier(ident) => {
                let name = self.interner.resolve(&ident.ident.symbol).to_string();
                if !defined.contains(&name) && !captured.contains(&name) {
                    captured.push(name);
                }
            }
            Expression::Binary(bin) => {
                self.collect_vars_in_expression(bin.left, captured, defined);
                self.collect_vars_in_expression(bin.right, captured, defined);
            }
            Expression::Unary(un) => {
                self.collect_vars_in_expression(un.operand, captured, defined);
            }
            Expression::Call(call) => {
                self.collect_vars_in_expression(call.callee, captured, defined);
                for arg in &call.args {
                    self.collect_vars_in_expression(arg, captured, defined);
                }
            }
            Expression::MethodCall(method) => {
                self.collect_vars_in_expression(method.receiver, captured, defined);
                for arg in &method.args {
                    self.collect_vars_in_expression(arg, captured, defined);
                }
            }
            Expression::Index(idx) => {
                self.collect_vars_in_expression(idx.base, captured, defined);
                self.collect_vars_in_expression(idx.index, captured, defined);
            }
            Expression::Array(arr) => {
                for elem in &arr.elements {
                    self.collect_vars_in_expression(elem, captured, defined);
                }
            }
            Expression::Grouped(grouped) => {
                self.collect_vars_in_expression(grouped.inner, captured, defined);
            }
            Expression::Block(block) => {
                // Create a new defined set for block scope
                let mut block_defined = defined.clone();
                for stmt in &block.statements {
                    self.collect_vars_in_statement(stmt, captured, &mut block_defined);
                }
                if let Some(tail) = block.tail {
                    self.collect_vars_in_expression(tail, captured, &block_defined);
                }
            }
            Expression::Lambda(lambda) => {
                // Lambda creates its own scope - capture variables from outer scope
                let mut lambda_defined = defined.clone();
                for param in &lambda.params {
                    let name = self.interner.resolve(&param.name.symbol).to_string();
                    lambda_defined.insert(name);
                }
                self.collect_vars_in_expression(lambda.body, captured, &lambda_defined);
            }
            Expression::Ternary(ternary) => {
                self.collect_vars_in_expression(ternary.condition, captured, defined);
                self.collect_vars_in_expression(ternary.true_expr, captured, defined);
                self.collect_vars_in_expression(ternary.false_expr, captured, defined);
            }
            Expression::Elvis(elvis) => {
                self.collect_vars_in_expression(elvis.left, captured, defined);
                self.collect_vars_in_expression(elvis.right, captured, defined);
            }
            Expression::FallibleCast(cast) => {
                self.collect_vars_in_expression(cast.expr, captured, defined);
            }
            Expression::ForceUnwrap(unwrap) => {
                self.collect_vars_in_expression(unwrap.expr, captured, defined);
            }
            _ => {}
        }
    }

    fn declare_spawn_trampoline(
        &mut self,
        _id: u32,
        info: &SpawnBlockInfo,
    ) -> Result<FuncId, CodegenError> {
        let mut sig = self.module.make_signature();
        sig.params
            .push(AbiParam::new(cranelift::prelude::types::I64)); // *mut u8 as i64

        let func_id = self
            .module
            .declare_function(&info.func_name, Linkage::Local, &sig)
            .map_err(|e| {
                CodegenError::JitCompile(format!(
                    "Failed to declare spawn trampoline '{}': {}",
                    info.func_name, e
                ))
            })?;

        self.functions.insert(info.func_name.clone(), func_id);

        Ok(func_id)
    }

    fn compile_spawn_trampoline(&mut self, info: &SpawnBlockInfo) -> Result<(), CodegenError> {
        let func_id = *self.functions.get(&info.func_name).ok_or_else(|| {
            CodegenError::JitCompile(format!("Trampoline '{}' not declared", info.func_name))
        })?;

        self.ctx.func.signature = self
            .module
            .declarations()
            .get_function_decl(func_id)
            .signature
            .clone();
        self.ctx.func.name = cranelift_codegen::ir::UserFuncName::user(0, func_id.as_u32());

        let mut builder_ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);

        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        // Get the closure data pointer (first and only parameter)
        let data_ptr = builder.block_params(entry_block)[0];

        let mut ctx = CompileContext {
            interner: self.interner,
            module: &mut self.module,
            functions: &self.functions,
            runtime_funcs: &self.runtime_funcs,
            struct_defs: &self.struct_defs,
            enum_defs: &self.enum_defs,
            exception_names: &self.exception_names,
            extern_fns: &self.extern_fns,
            variables: HashMap::new(),
            var_heap_types: HashMap::new(),
            var_counter: 0,
            block_terminated: false,
            loop_exit_block: None,
            loop_header_block: None,
            spawn_blocks: &self.spawn_blocks,
            current_spawn_id: 0, // Not used in trampolines
            lambda_blocks: &self.lambda_blocks,
            current_lambda_id: 0,
            annotations: self.annotations,
            type_substitutions: HashMap::new(),
            func_return_type: None,
        };

        // Load captured variables from closure data
        for (i, var_name) in info.captured_vars.iter().enumerate() {
            let var = Variable::new(ctx.var_counter);
            ctx.var_counter += 1;
            builder.declare_var(var, cranelift::prelude::types::I64);

            // Load value from closure data: data_ptr + (i * 8)
            let offset = builder
                .ins()
                .iconst(cranelift::prelude::types::I64, (i * 8) as i64);
            let addr = builder.ins().iadd(data_ptr, offset);
            let val = builder
                .ins()
                .load(cranelift::prelude::types::I64, MemFlags::new(), addr, 0);
            builder.def_var(var, val);
            ctx.variables.insert(var_name.clone(), var);
        }
        let body = unsafe { &*info.body_ptr };
        for stmt in &body.statements {
            compile_statement(&mut ctx, &mut builder, stmt)?;
            if ctx.block_terminated {
                break;
            }
        }

        if !ctx.block_terminated {
            builder.ins().return_(&[]);
        }

        builder.finalize();

        let trampoline_name = info.func_name.clone();
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            self.module.define_function(func_id, &mut self.ctx)
        }));

        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                return Err(CodegenError::JitCompile(format!(
                    "Failed to define trampoline '{}': {}",
                    trampoline_name, e
                )));
            }
            Err(panic_info) => {
                let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown internal error".to_string()
                };
                return Err(convert_cranelift_error(&panic_msg, &trampoline_name));
            }
        }

        self.module.clear_context(&mut self.ctx);

        Ok(())
    }

    fn declare_lambda_function(&mut self, info: &LambdaInfo) -> Result<FuncId, CodegenError> {
        let mut sig = self.module.make_signature();

        // First parameter: closure data pointer
        sig.params
            .push(AbiParam::new(cranelift::prelude::types::I64));

        // Lambda parameters (all as i64 for now)
        for _ in &info.param_names {
            sig.params
                .push(AbiParam::new(cranelift::prelude::types::I64));
        }

        // Return type (i64 for now)
        sig.returns
            .push(AbiParam::new(cranelift::prelude::types::I64));

        let func_id = self
            .module
            .declare_function(&info.func_name, Linkage::Local, &sig)
            .map_err(|e| {
                CodegenError::JitCompile(format!(
                    "Failed to declare lambda '{}': {}",
                    info.func_name, e
                ))
            })?;

        self.functions.insert(info.func_name.clone(), func_id);

        Ok(func_id)
    }

    fn compile_lambda_function(&mut self, info: &LambdaInfo) -> Result<(), CodegenError> {
        let func_id = *self.functions.get(&info.func_name).ok_or_else(|| {
            CodegenError::JitCompile(format!("Lambda '{}' not declared", info.func_name))
        })?;

        self.ctx.func.signature = self
            .module
            .declarations()
            .get_function_decl(func_id)
            .signature
            .clone();
        self.ctx.func.name = cranelift_codegen::ir::UserFuncName::user(0, func_id.as_u32());

        let mut builder_ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);

        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        let block_params = builder.block_params(entry_block).to_vec();
        // First param is closure data pointer
        let data_ptr = block_params[0];

        let mut ctx = CompileContext {
            interner: self.interner,
            module: &mut self.module,
            functions: &self.functions,
            runtime_funcs: &self.runtime_funcs,
            struct_defs: &self.struct_defs,
            enum_defs: &self.enum_defs,
            exception_names: &self.exception_names,
            extern_fns: &self.extern_fns,
            variables: HashMap::new(),
            var_heap_types: HashMap::new(),
            var_counter: 0,
            block_terminated: false,
            loop_exit_block: None,
            loop_header_block: None,
            spawn_blocks: &self.spawn_blocks,
            current_spawn_id: 0,
            lambda_blocks: &self.lambda_blocks,
            current_lambda_id: 0,
            annotations: self.annotations,
            type_substitutions: HashMap::new(),
            func_return_type: Some(cranelift::prelude::types::I64), // Lambdas always return i64
        };

        // Load captured variables from closure data
        for (i, var_name) in info.captured_vars.iter().enumerate() {
            let var = Variable::new(ctx.var_counter);
            ctx.var_counter += 1;
            builder.declare_var(var, cranelift::prelude::types::I64);

            // Load value from closure data: data_ptr + (i * 8)
            let offset = builder
                .ins()
                .iconst(cranelift::prelude::types::I64, (i * 8) as i64);
            let addr = builder.ins().iadd(data_ptr, offset);
            let val = builder
                .ins()
                .load(cranelift::prelude::types::I64, MemFlags::new(), addr, 0);
            builder.def_var(var, val);
            ctx.variables.insert(var_name.clone(), var);
        }

        // Define lambda parameters (starting from param 1, since param 0 is closure data)
        for (i, param_name) in info.param_names.iter().enumerate() {
            let var = Variable::new(ctx.var_counter);
            ctx.var_counter += 1;
            builder.declare_var(var, cranelift::prelude::types::I64);
            // Parameter i+1 because param 0 is the closure data
            builder.def_var(var, block_params[i + 1]);
            ctx.variables.insert(param_name.clone(), var);
        }

        // Compile the lambda body (which is an Expression)
        let body = unsafe { &*info.body_ptr };
        let result = compile_expression(&mut ctx, &mut builder, body)?;

        if !ctx.block_terminated {
            let result_type = builder.func.dfg.value_type(result);
            let result_i64 = if result_type == cranelift::prelude::types::I8 {
                builder
                    .ins()
                    .uextend(cranelift::prelude::types::I64, result)
            } else if result_type != cranelift::prelude::types::I64
                && result_type != cranelift::prelude::types::F64
            {
                builder
                    .ins()
                    .uextend(cranelift::prelude::types::I64, result)
            } else {
                result
            };
            builder.ins().return_(&[result_i64]);
        }

        builder.finalize();

        let lambda_name = info.func_name.clone();
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            self.module.define_function(func_id, &mut self.ctx)
        }));

        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                return Err(CodegenError::JitCompile(format!(
                    "Failed to define lambda '{}': {}",
                    lambda_name, e
                )));
            }
            Err(panic_info) => {
                let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown internal error".to_string()
                };
                return Err(convert_cranelift_error(&panic_msg, &lambda_name));
            }
        }

        self.module.clear_context(&mut self.ctx);

        Ok(())
    }

    fn declare_function(&mut self, func: &FunctionItem<'_>) -> Result<FuncId, CodegenError> {
        let name = self.interner.resolve(&func.name.symbol);

        let mut sig = self.module.make_signature();

        sig.params
            .push(AbiParam::new(cranelift::prelude::types::I64));

        for param in &func.params {
            let ty = types::naml_to_cranelift(&param.ty);
            sig.params.push(AbiParam::new(ty));
        }

        if let Some(ref return_ty) = func.return_ty {
            let ty = types::naml_to_cranelift(return_ty);
            sig.returns.push(AbiParam::new(ty));
        }

        let func_id = self
            .module
            .declare_function(name, Linkage::Export, &sig)
            .map_err(|e| {
                CodegenError::JitCompile(format!("Failed to declare function '{}': {}", name, e))
            })?;

        self.functions.insert(name.to_string(), func_id);

        Ok(func_id)
    }

    fn compile_function(&mut self, func: &FunctionItem<'_>) -> Result<(), CodegenError> {
        let name = self.interner.resolve(&func.name.symbol);
        let func_id = *self
            .functions
            .get(name)
            .ok_or_else(|| CodegenError::JitCompile(format!("Function '{}' not declared", name)))?;

        self.ctx.func.signature = self
            .module
            .declarations()
            .get_function_decl(func_id)
            .signature
            .clone();
        self.ctx.func.name = cranelift_codegen::ir::UserFuncName::user(0, func_id.as_u32());

        let mut builder_ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);

        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        let func_return_type = func
            .return_ty
            .as_ref()
            .map(|ty| types::naml_to_cranelift(ty));

        let mut ctx = CompileContext {
            interner: self.interner,
            module: &mut self.module,
            functions: &self.functions,
            runtime_funcs: &self.runtime_funcs,
            struct_defs: &self.struct_defs,
            enum_defs: &self.enum_defs,
            exception_names: &self.exception_names,
            extern_fns: &self.extern_fns,
            variables: HashMap::new(),
            var_heap_types: HashMap::new(),
            var_counter: 0,
            block_terminated: false,
            loop_exit_block: None,
            loop_header_block: None,
            spawn_blocks: &self.spawn_blocks,
            current_spawn_id: 0,
            lambda_blocks: &self.lambda_blocks,
            current_lambda_id: 0,
            annotations: self.annotations,
            type_substitutions: HashMap::new(),
            func_return_type,
        };

        for (i, param) in func.params.iter().enumerate() {
            let param_name = self.interner.resolve(&param.name.symbol).to_string();
            let val = builder.block_params(entry_block)[i + 1];
            let var = Variable::new(ctx.var_counter);
            ctx.var_counter += 1;
            let ty = types::naml_to_cranelift(&param.ty);
            builder.declare_var(var, ty);
            builder.def_var(var, val);
            ctx.variables.insert(param_name, var);
        }

        // Push function onto shadow stack for stack traces
        let func_name_str = self.interner.resolve(&func.name.symbol);
        let (line, _) = self.source_info.line_col(func.span.start);
        let file_name = &*self.source_info.name;
        emit_stack_push(&mut ctx, &mut builder, func_name_str, file_name, line as u32)?;

        if let Some(ref body) = func.body {
            for stmt in &body.statements {
                compile_statement(&mut ctx, &mut builder, stmt)?;
                if ctx.block_terminated {
                    break;
                }
            }
        }

        // Pop from shadow stack before implicit return
        if !ctx.block_terminated && func.return_ty.is_none() {
            emit_stack_pop(&mut ctx, &mut builder)?;
            emit_cleanup_all_vars(&mut ctx, &mut builder, None)?;
            builder.ins().return_(&[]);
        }

        builder.finalize();

        let name_clone = name.to_string();
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            self.module.define_function(func_id, &mut self.ctx)
        }));

        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                return Err(CodegenError::JitCompile(format!(
                    "Failed to define function '{}': {:?}",
                    name, e
                )));
            }
            Err(panic_info) => {
                let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown internal error".to_string()
                };
                return Err(convert_cranelift_error(&panic_msg, &name_clone));
            }
        }

        self.module.clear_context(&mut self.ctx);

        Ok(())
    }

    fn declare_method(&mut self, func: &FunctionItem<'_>) -> Result<FuncId, CodegenError> {
        let receiver = func
            .receiver
            .as_ref()
            .ok_or_else(|| CodegenError::JitCompile("Method must have receiver".to_string()))?;

        // Get receiver type name (handles both Named and Generic types)
        let receiver_type_name = match &receiver.ty {
            crate::ast::NamlType::Named(ident) => self.interner.resolve(&ident.symbol).to_string(),
            crate::ast::NamlType::Generic(ident, _) => {
                self.interner.resolve(&ident.symbol).to_string()
            }
            _ => {
                return Err(CodegenError::JitCompile(
                    "Method receiver must be a named or generic type".to_string(),
                ));
            }
        };

        let method_name = self.interner.resolve(&func.name.symbol);
        let full_name = format!("{}_{}", receiver_type_name, method_name);

        let ptr_type = self.module.target_config().pointer_type();
        let mut sig = self.module.make_signature();

        // Receiver is the first parameter (pointer to struct)
        sig.params.push(AbiParam::new(ptr_type));

        for param in &func.params {
            let ty = types::naml_to_cranelift(&param.ty);
            sig.params.push(AbiParam::new(ty));
        }

        if let Some(ref return_ty) = func.return_ty {
            let ty = types::naml_to_cranelift(return_ty);
            sig.returns.push(AbiParam::new(ty));
        }

        let func_id = self
            .module
            .declare_function(&full_name, Linkage::Local, &sig)
            .map_err(|e| {
                CodegenError::JitCompile(format!("Failed to declare method '{}': {}", full_name, e))
            })?;

        self.functions.insert(full_name, func_id);

        Ok(func_id)
    }

    fn compile_method(&mut self, func: &FunctionItem<'_>) -> Result<(), CodegenError> {
        let receiver = func
            .receiver
            .as_ref()
            .ok_or_else(|| CodegenError::JitCompile("Method must have receiver".to_string()))?;

        let receiver_type_name = match &receiver.ty {
            crate::ast::NamlType::Named(ident) => self.interner.resolve(&ident.symbol).to_string(),
            crate::ast::NamlType::Generic(ident, _) => {
                self.interner.resolve(&ident.symbol).to_string()
            }
            _ => {
                return Err(CodegenError::JitCompile(
                    "Method receiver must be a named or generic type".to_string(),
                ));
            }
        };

        let method_name = self.interner.resolve(&func.name.symbol);
        let full_name = format!("{}_{}", receiver_type_name, method_name);

        let func_id = *self.functions.get(&full_name).ok_or_else(|| {
            CodegenError::JitCompile(format!("Method '{}' not declared", full_name))
        })?;

        self.ctx.func.signature = self
            .module
            .declarations()
            .get_function_decl(func_id)
            .signature
            .clone();
        self.ctx.func.name = cranelift_codegen::ir::UserFuncName::user(0, func_id.as_u32());

        // Get pointer type before borrowing module
        let ptr_type = self.module.target_config().pointer_type();

        let mut builder_ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);

        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        let func_return_type = func
            .return_ty
            .as_ref()
            .map(|ty| types::naml_to_cranelift(ty));

        let mut ctx = CompileContext {
            interner: self.interner,
            module: &mut self.module,
            functions: &self.functions,
            runtime_funcs: &self.runtime_funcs,
            struct_defs: &self.struct_defs,
            enum_defs: &self.enum_defs,
            exception_names: &self.exception_names,
            extern_fns: &self.extern_fns,
            variables: HashMap::new(),
            var_heap_types: HashMap::new(),
            var_counter: 0,
            block_terminated: false,
            loop_exit_block: None,
            loop_header_block: None,
            spawn_blocks: &self.spawn_blocks,
            current_spawn_id: 0,
            lambda_blocks: &self.lambda_blocks,
            current_lambda_id: 0,
            annotations: self.annotations,
            type_substitutions: HashMap::new(),
            func_return_type,
        };

        // Set up receiver variable (self)
        let receiver_name = self.interner.resolve(&receiver.name.symbol).to_string();
        let recv_val = builder.block_params(entry_block)[0];
        let recv_var = Variable::new(ctx.var_counter);
        ctx.var_counter += 1;
        builder.declare_var(recv_var, ptr_type);
        builder.def_var(recv_var, recv_val);
        ctx.variables.insert(receiver_name, recv_var);

        // Set up regular parameters (offset by 1 due to receiver)
        for (i, param) in func.params.iter().enumerate() {
            let param_name = self.interner.resolve(&param.name.symbol).to_string();
            let val = builder.block_params(entry_block)[i + 1]; // +1 for receiver
            let var = Variable::new(ctx.var_counter);
            ctx.var_counter += 1;
            let ty = types::naml_to_cranelift(&param.ty);
            builder.declare_var(var, ty);
            builder.def_var(var, val);
            ctx.variables.insert(param_name, var);
        }

        // Push method onto shadow stack for stack traces
        let (line, _) = self.source_info.line_col(func.span.start);
        let file_name = &*self.source_info.name;
        emit_stack_push(&mut ctx, &mut builder, &full_name, file_name, line as u32)?;

        if let Some(ref body) = func.body {
            for stmt in &body.statements {
                compile_statement(&mut ctx, &mut builder, stmt)?;
                if ctx.block_terminated {
                    break;
                }
            }
        }

        // Pop from shadow stack before implicit return
        if !ctx.block_terminated && func.return_ty.is_none() {
            emit_stack_pop(&mut ctx, &mut builder)?;
            emit_cleanup_all_vars(&mut ctx, &mut builder, None)?;
            builder.ins().return_(&[]);
        }

        builder.finalize();

        let full_name_clone = full_name.clone();
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            self.module.define_function(func_id, &mut self.ctx)
        }));

        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                return Err(CodegenError::JitCompile(format!(
                    "Failed to define method '{}': {}",
                    full_name, e
                )));
            }
            Err(panic_info) => {
                let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown internal error".to_string()
                };
                return Err(convert_cranelift_error(&panic_msg, &full_name_clone));
            }
        }

        self.module.clear_context(&mut self.ctx);

        Ok(())
    }

    pub fn run_main(&mut self) -> Result<(), CodegenError> {
        self.module
            .finalize_definitions()
            .map_err(|e| CodegenError::JitCompile(format!("Failed to finalize: {}", e)))?;

        let main_id = self
            .functions
            .get("main")
            .ok_or_else(|| CodegenError::Execution("No main function found".to_string()))?;

        let main_ptr = self.module.get_finalized_function(*main_id);

        let main_fn: fn(i64) = unsafe { std::mem::transmute(main_ptr) };
        main_fn(0);

        Ok(())
    }
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

extern "C" fn naml_print_str(ptr: *const i8) {
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
