//!
//! Cranelift JIT Compiler
//!
//! Compiles naml AST directly to machine code using Cranelift.
//! This eliminates the Rust transpilation step and gives full control
//! over memory management and runtime semantics.
//!

mod types;

use std::collections::HashMap;

use cranelift::prelude::*;
use cranelift_codegen::ir::AtomicRmwOp;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{DataDescription, FuncId, Linkage, Module};
use lasso::Rodeo;

use crate::ast::{
    BinaryOp, Expression, FunctionItem, Item, Literal, NamlType, SourceFile, Statement, UnaryOp,
    LiteralExpr,
};
use crate::codegen::CodegenError;
use crate::source::Spanned;
use crate::typechecker::{SymbolTable, Type, TypeAnnotations};

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

unsafe impl Send for LambdaInfo {}

pub struct JitCompiler<'a> {
    interner: &'a Rodeo,
    #[allow(dead_code)]
    annotations: &'a TypeAnnotations,
    #[allow(dead_code)]
    symbols: &'a SymbolTable,
    module: JITModule,
    ctx: codegen::Context,
    functions: HashMap<String, FuncId>,
    struct_defs: HashMap<String, StructDef>,
    enum_defs: HashMap<String, EnumDef>,
    extern_fns: HashMap<String, ExternFn>,
    next_type_id: u32,
    spawn_counter: u32,
    spawn_blocks: HashMap<u32, SpawnBlockInfo>,
    lambda_counter: u32,
    lambda_blocks: HashMap<u32, LambdaInfo>,
}

impl<'a> JitCompiler<'a> {
    pub fn new(
        interner: &'a Rodeo,
        annotations: &'a TypeAnnotations,
        symbols: &'a SymbolTable,
    ) -> Result<Self, CodegenError> {
        let mut flag_builder = settings::builder();
        flag_builder.set("use_colocated_libcalls", "false").unwrap();
        flag_builder.set("is_pic", "false").unwrap();

        let isa_builder = cranelift_native::builder()
            .map_err(|e| CodegenError::JitCompile(format!("Failed to create ISA builder: {}", e)))?;

        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .map_err(|e| CodegenError::JitCompile(format!("Failed to create ISA: {}", e)))?;

        let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());

        // Print builtins
        builder.symbol("naml_print_int", naml_print_int as *const u8);
        builder.symbol("naml_print_float", naml_print_float as *const u8);
        builder.symbol("naml_print_str", naml_print_str as *const u8);
        builder.symbol("naml_print_newline", naml_print_newline as *const u8);

        // Array runtime functions
        builder.symbol("naml_array_new", crate::runtime::naml_array_new as *const u8);
        builder.symbol("naml_array_from", crate::runtime::naml_array_from as *const u8);
        builder.symbol("naml_array_push", crate::runtime::naml_array_push as *const u8);
        builder.symbol("naml_array_get", crate::runtime::naml_array_get as *const u8);
        builder.symbol("naml_array_set", crate::runtime::naml_array_set as *const u8);
        builder.symbol("naml_array_len", crate::runtime::naml_array_len as *const u8);
        builder.symbol("naml_array_pop", crate::runtime::naml_array_pop as *const u8);
        builder.symbol("naml_array_print", crate::runtime::naml_array_print as *const u8);
        builder.symbol("naml_array_incref", crate::runtime::naml_array_incref as *const u8);
        builder.symbol("naml_array_decref", crate::runtime::naml_array_decref as *const u8);
        builder.symbol("naml_array_decref_strings", crate::runtime::naml_array_decref_strings as *const u8);
        builder.symbol("naml_array_decref_arrays", crate::runtime::naml_array_decref_arrays as *const u8);
        builder.symbol("naml_array_decref_maps", crate::runtime::naml_array_decref_maps as *const u8);
        builder.symbol("naml_array_decref_structs", crate::runtime::naml_array_decref_structs as *const u8);

        // Struct operations
        builder.symbol("naml_struct_new", crate::runtime::naml_struct_new as *const u8);
        builder.symbol("naml_struct_incref", crate::runtime::naml_struct_incref as *const u8);
        builder.symbol("naml_struct_decref", crate::runtime::naml_struct_decref as *const u8);
        builder.symbol("naml_struct_free", crate::runtime::naml_struct_free as *const u8);
        builder.symbol("naml_struct_get_field", crate::runtime::naml_struct_get_field as *const u8);
        builder.symbol("naml_struct_set_field", crate::runtime::naml_struct_set_field as *const u8);

        // Scheduler operations
        builder.symbol("naml_spawn", crate::runtime::naml_spawn as *const u8);
        builder.symbol("naml_spawn_closure", crate::runtime::naml_spawn_closure as *const u8);
        builder.symbol("naml_alloc_closure_data", crate::runtime::naml_alloc_closure_data as *const u8);
        builder.symbol("naml_wait_all", crate::runtime::naml_wait_all as *const u8);
        builder.symbol("naml_sleep", crate::runtime::naml_sleep as *const u8);

        // Channel operations
        builder.symbol("naml_channel_new", crate::runtime::naml_channel_new as *const u8);
        builder.symbol("naml_channel_send", crate::runtime::naml_channel_send as *const u8);
        builder.symbol("naml_channel_receive", crate::runtime::naml_channel_receive as *const u8);
        builder.symbol("naml_channel_close", crate::runtime::naml_channel_close as *const u8);
        builder.symbol("naml_channel_len", crate::runtime::naml_channel_len as *const u8);
        builder.symbol("naml_channel_incref", crate::runtime::naml_channel_incref as *const u8);
        builder.symbol("naml_channel_decref", crate::runtime::naml_channel_decref as *const u8);

        // Map operations
        builder.symbol("naml_map_new", crate::runtime::naml_map_new as *const u8);
        builder.symbol("naml_map_set", crate::runtime::naml_map_set as *const u8);
        builder.symbol("naml_map_set_string", crate::runtime::naml_map_set_string as *const u8);
        builder.symbol("naml_map_set_array", crate::runtime::naml_map_set_array as *const u8);
        builder.symbol("naml_map_set_map", crate::runtime::naml_map_set_map as *const u8);
        builder.symbol("naml_map_set_struct", crate::runtime::naml_map_set_struct as *const u8);
        builder.symbol("naml_map_get", crate::runtime::naml_map_get as *const u8);
        builder.symbol("naml_map_contains", crate::runtime::naml_map_contains as *const u8);
        builder.symbol("naml_map_len", crate::runtime::naml_map_len as *const u8);
        builder.symbol("naml_map_incref", crate::runtime::naml_map_incref as *const u8);
        builder.symbol("naml_map_decref", crate::runtime::naml_map_decref as *const u8);
        builder.symbol("naml_map_decref_strings", crate::runtime::naml_map_decref_strings as *const u8);
        builder.symbol("naml_map_decref_arrays", crate::runtime::naml_map_decref_arrays as *const u8);
        builder.symbol("naml_map_decref_maps", crate::runtime::naml_map_decref_maps as *const u8);
        builder.symbol("naml_map_decref_structs", crate::runtime::naml_map_decref_structs as *const u8);

        // Exception handling
        builder.symbol("naml_exception_set", crate::runtime::naml_exception_set as *const u8);
        builder.symbol("naml_exception_get", crate::runtime::naml_exception_get as *const u8);
        builder.symbol("naml_exception_clear", crate::runtime::naml_exception_clear as *const u8);
        builder.symbol("naml_exception_check", crate::runtime::naml_exception_check as *const u8);

        // String operations
        builder.symbol("naml_string_from_cstr", crate::runtime::naml_string_from_cstr as *const u8);
        builder.symbol("naml_string_print", crate::runtime::naml_string_print as *const u8);
        builder.symbol("naml_string_eq", crate::runtime::naml_string_eq as *const u8);
        builder.symbol("naml_string_incref", crate::runtime::naml_string_incref as *const u8);
        builder.symbol("naml_string_decref", crate::runtime::naml_string_decref as *const u8);
        builder.symbol("naml_string_char_at", crate::runtime::naml_string_char_at as *const u8);
        builder.symbol("naml_string_char_len", crate::runtime::naml_string_char_len as *const u8);

        // Type conversion operations
        builder.symbol("naml_int_to_string", crate::runtime::naml_int_to_string as *const u8);
        builder.symbol("naml_float_to_string", crate::runtime::naml_float_to_string as *const u8);
        builder.symbol("naml_string_to_int", crate::runtime::naml_string_to_int as *const u8);
        builder.symbol("naml_string_to_float", crate::runtime::naml_string_to_float as *const u8);

        // Bytes operations
        builder.symbol("naml_bytes_new", crate::runtime::naml_bytes_new as *const u8);
        builder.symbol("naml_bytes_from", crate::runtime::naml_bytes_from as *const u8);
        builder.symbol("naml_bytes_len", crate::runtime::naml_bytes_len as *const u8);
        builder.symbol("naml_bytes_get", crate::runtime::naml_bytes_get as *const u8);
        builder.symbol("naml_bytes_set", crate::runtime::naml_bytes_set as *const u8);
        builder.symbol("naml_bytes_incref", crate::runtime::naml_bytes_incref as *const u8);
        builder.symbol("naml_bytes_decref", crate::runtime::naml_bytes_decref as *const u8);
        builder.symbol("naml_bytes_to_string", crate::runtime::naml_bytes_to_string as *const u8);
        builder.symbol("naml_string_to_bytes", crate::runtime::naml_string_to_bytes as *const u8);

        let module = JITModule::new(builder);
        let ctx = module.make_context();

        // Built-in option type (polymorphic, treat as Option<i64> for now)
        let mut enum_defs = HashMap::new();
        enum_defs.insert("option".to_string(), EnumDef {
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
        });

        Ok(Self {
            interner,
            annotations,
            symbols,
            module,
            ctx,
            functions: HashMap::new(),
            struct_defs: HashMap::new(),
            enum_defs,
            extern_fns: HashMap::new(),
            next_type_id: 0,
            spawn_counter: 0,
            spawn_blocks: HashMap::new(),
            lambda_counter: 0,
            lambda_blocks: HashMap::new(),
        })
    }

    pub fn compile(&mut self, ast: &SourceFile<'_>) -> Result<(), CodegenError> {
        // First pass: collect struct definitions with field heap types
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

                self.struct_defs.insert(name, StructDef { type_id, fields, field_heap_types });
            }
        }

        // Collect exception definitions (treated like structs for codegen)
        for item in &ast.items {
            if let crate::ast::Item::Exception(exception_item) = item {
                let name = self.interner.resolve(&exception_item.name.symbol).to_string();
                let mut fields = Vec::new();
                let mut field_heap_types = Vec::new();

                for f in &exception_item.fields {
                    fields.push(self.interner.resolve(&f.name.symbol).to_string());
                    field_heap_types.push(get_heap_type(&f.ty));
                }

                let type_id = self.next_type_id;
                self.next_type_id += 1;

                // Exception treated as a struct with its fields
                self.struct_defs.insert(name, StructDef { type_id, fields, field_heap_types });
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
                let size = 8 + ((max_data_size + 7) / 8) * 8;

                self.enum_defs.insert(name.clone(), EnumDef {
                    name,
                    variants,
                    size,
                });
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

                let param_types: Vec<_> = extern_item.params.iter()
                    .map(|p| p.ty.clone())
                    .collect();

                self.extern_fns.insert(name, ExternFn {
                    link_name,
                    param_types,
                    return_type: extern_item.return_ty.clone(),
                });
            }
        }

        // Generate per-struct decref functions for structs with heap fields
        self.generate_struct_decref_functions()?;

        // Scan for spawn blocks and collect captured variable info
        for item in &ast.items {
            if let Item::Function(f) = item {
                if let Some(ref body) = f.body {
                    self.scan_for_spawn_blocks(body)?;
                }
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
        for item in &ast.items {
            if let Item::Function(f) = item {
                if f.receiver.is_none() {
                    self.declare_function(f)?;
                } else {
                    self.declare_method(f)?;
                }
            }
        }

        // Compile standalone functions
        for item in &ast.items {
            if let Item::Function(f) = item {
                if f.receiver.is_none() && f.body.is_some() {
                    self.compile_function(f)?;
                }
            }
        }

        // Compile methods
        for item in &ast.items {
            if let Item::Function(f) = item {
                if f.receiver.is_some() && f.body.is_some() {
                    self.compile_method(f)?;
                }
            }
        }

        Ok(())
    }

    fn generate_struct_decref_functions(&mut self) -> Result<(), CodegenError> {
        // Collect structs that need specialized decref functions
        let structs_with_heap_fields: Vec<(String, StructDef)> = self.struct_defs.iter()
            .filter(|(_, def)| def.field_heap_types.iter().any(|ht| ht.is_some()))
            .map(|(name, def)| (name.clone(), def.clone()))
            .collect();

        for (struct_name, struct_def) in structs_with_heap_fields {
            self.generate_struct_decref(&struct_name, &struct_def)?;
        }

        Ok(())
    }

    fn generate_struct_decref(&mut self, struct_name: &str, struct_def: &StructDef) -> Result<(), CodegenError> {
        let ptr_type = self.module.target_config().pointer_type();
        let func_name = format!("naml_struct_decref_{}", struct_name);

        // Function signature: fn(struct_ptr: *mut NamlStruct)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(ptr_type));

        let func_id = self.module
            .declare_function(&func_name, Linkage::Local, &sig)
            .map_err(|e| CodegenError::JitCompile(format!("Failed to declare {}: {}", func_name, e)))?;

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

        builder.ins().brif(is_null, null_block, &[], decref_block, &[]);

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

        builder.ins().brif(should_free, free_block, &[], done_block, &[]);
        builder.switch_to_block(free_block);
        builder.seal_block(free_block);
        builder.ins().fence();

        // Struct memory layout after header:
        // - type_id: u32 (offset 16)
        // - field_count: u32 (offset 20)
        // - fields[]: i64 (offset 24+)
        let base_field_offset: i32 = 24; // sizeof(HeapHeader) + type_id + field_count

        let mut decref_sig = self.module.make_signature();
        decref_sig.params.push(AbiParam::new(ptr_type));

        for (field_idx, heap_type) in struct_def.field_heap_types.iter().enumerate() {
            if let Some(ht) = heap_type {
                let field_offset = base_field_offset + (field_idx as i32 * 8);
                let field_val = builder.ins().load(cranelift::prelude::types::I64, MemFlags::new(), struct_ptr, field_offset);

                let field_is_null = builder.ins().icmp(IntCC::Equal, field_val, zero);
                let decref_field_block = builder.create_block();
                let next_field_block = builder.create_block();

                builder.ins().brif(field_is_null, next_field_block, &[], decref_field_block, &[]);
                builder.switch_to_block(decref_field_block);
                builder.seal_block(decref_field_block);

                let decref_func_name = match ht {
                    HeapType::String => "naml_string_decref",
                    HeapType::Array(None) => "naml_array_decref",
                    HeapType::Array(Some(elem_type)) => {
                        match elem_type.as_ref() {
                            HeapType::String => "naml_array_decref_strings",
                            HeapType::Array(_) => "naml_array_decref_arrays",
                            HeapType::Map(_) => "naml_array_decref_maps",
                            HeapType::Struct(_) => "naml_array_decref_structs",
                        }
                    }
                    HeapType::Map(None) => "naml_map_decref",
                    HeapType::Map(Some(val_type)) => {
                        match val_type.as_ref() {
                            HeapType::String => "naml_map_decref_strings",
                            HeapType::Array(_) => "naml_map_decref_arrays",
                            HeapType::Map(_) => "naml_map_decref_maps",
                            HeapType::Struct(_) => "naml_map_decref_structs",
                        }
                    }
                    HeapType::Struct(None) => "naml_struct_decref",
                    HeapType::Struct(Some(nested_struct_name)) => {
                        if self.struct_defs.get(nested_struct_name)
                            .map(|def| def.field_heap_types.iter().any(|h| h.is_some()))
                            .unwrap_or(false)
                        {
                            "naml_struct_decref" // Fallback for now
                        } else {
                            "naml_struct_decref"
                        }
                    }
                };

                let decref_func_id = self.module
                    .declare_function(decref_func_name, Linkage::Import, &decref_sig)
                    .map_err(|e| CodegenError::JitCompile(format!("Failed to declare {}: {}", decref_func_name, e)))?;

                let decref_func_ref = self.module.declare_func_in_func(decref_func_id, builder.func);
                builder.ins().call(decref_func_ref, &[field_val]);
                builder.ins().jump(next_field_block, &[]);

                builder.switch_to_block(next_field_block);
                builder.seal_block(next_field_block);
            }
        }

        // Call naml_struct_free to deallocate the struct memory
        let free_func_id = self.module
            .declare_function("naml_struct_free", Linkage::Import, &decref_sig)
            .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_struct_free: {}", e)))?;
        let free_func_ref = self.module.declare_func_in_func(free_func_id, builder.func);
        builder.ins().call(free_func_ref, &[struct_ptr]);
        builder.ins().jump(done_block, &[]);

        // Done block: return
        builder.switch_to_block(done_block);
        builder.seal_block(done_block);
        builder.ins().return_(&[]);

        builder.finalize();

        self.module
            .define_function(func_id, &mut self.ctx)
            .map_err(|e| CodegenError::JitCompile(format!("Failed to define {}: {}", func_name, e)))?;

        self.ctx.clear();

        Ok(())
    }

    fn scan_for_spawn_blocks(&mut self, block: &crate::ast::BlockStmt<'_>) -> Result<(), CodegenError> {
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
                let captured = self.collect_captured_vars_expr(&spawn_expr.body);
                let id = self.spawn_counter;
                self.spawn_counter += 1;
                let func_name = format!("__spawn_{}", id);

                // Store raw pointer to body for deferred trampoline compilation
                // Safety: Only used within the same compile() call
                // Note: spawn_expr.body is already a &BlockExpr, so we cast it directly
                let body_ptr = spawn_expr.body as *const crate::ast::BlockExpr<'_> as *const crate::ast::BlockExpr<'static>;

                self.spawn_blocks.insert(id, SpawnBlockInfo {
                    id,
                    func_name,
                    captured_vars: captured,
                    body_ptr,
                });

                // Also scan inside spawn block for nested spawns
                self.scan_for_spawn_blocks_expr(&spawn_expr.body)?;
            }
            Expression::Lambda(lambda_expr) => {
                // Found a lambda - collect captured variables
                let captured = self.collect_captured_vars_for_lambda(lambda_expr);
                let id = self.lambda_counter;
                self.lambda_counter += 1;
                let func_name = format!("__lambda_{}", id);

                // Collect parameter names
                let param_names: Vec<String> = lambda_expr.params.iter()
                    .map(|p| self.interner.resolve(&p.name.symbol).to_string())
                    .collect();

                // Store raw pointer to body for deferred lambda compilation
                let body_ptr = lambda_expr.body as *const crate::ast::Expression<'_> as *const crate::ast::Expression<'static>;

                self.lambda_blocks.insert(id, LambdaInfo {
                    id,
                    func_name,
                    captured_vars: captured,
                    param_names,
                    body_ptr,
                });

                // Scan lambda body for nested spawns/lambdas
                self.scan_expression_for_spawns(&lambda_expr.body)?;
            }
            Expression::Binary(bin) => {
                self.scan_expression_for_spawns(&bin.left)?;
                self.scan_expression_for_spawns(&bin.right)?;
            }
            Expression::Unary(un) => {
                self.scan_expression_for_spawns(&un.operand)?;
            }
            Expression::Call(call) => {
                self.scan_expression_for_spawns(&call.callee)?;
                for arg in &call.args {
                    self.scan_expression_for_spawns(arg)?;
                }
            }
            Expression::MethodCall(method) => {
                self.scan_expression_for_spawns(&method.receiver)?;
                for arg in &method.args {
                    self.scan_expression_for_spawns(arg)?;
                }
            }
            Expression::Index(idx) => {
                self.scan_expression_for_spawns(&idx.base)?;
                self.scan_expression_for_spawns(&idx.index)?;
            }
            Expression::Array(arr) => {
                for elem in &arr.elements {
                    self.scan_expression_for_spawns(elem)?;
                }
            }
            Expression::If(if_expr) => {
                self.scan_expression_for_spawns(&if_expr.condition)?;
                self.scan_for_spawn_blocks_expr(&if_expr.then_branch)?;
                self.scan_else_branch_for_spawns(&if_expr.else_branch)?;
            }
            Expression::Block(block) => {
                self.scan_for_spawn_blocks_expr(block)?;
            }
            Expression::Grouped(grouped) => {
                self.scan_expression_for_spawns(&grouped.inner)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn scan_for_spawn_blocks_expr(&mut self, block: &crate::ast::BlockExpr<'_>) -> Result<(), CodegenError> {
        for stmt in &block.statements {
            self.scan_statement_for_spawns(stmt)?;
        }
        if let Some(tail) = block.tail {
            self.scan_expression_for_spawns(tail)?;
        }
        Ok(())
    }

    fn scan_else_branch_for_spawns(&mut self, else_branch: &Option<crate::ast::ElseExpr<'_>>) -> Result<(), CodegenError> {
        if let Some(branch) = else_branch {
            match branch {
                crate::ast::ElseExpr::ElseIf(elif) => {
                    self.scan_expression_for_spawns(&elif.condition)?;
                    self.scan_for_spawn_blocks_expr(&elif.then_branch)?;
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
        self.collect_vars_in_expression(&lambda.body, &mut captured, &defined);

        captured
    }

    fn collect_vars_in_block(&self,
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
                self.collect_vars_in_expression(&bin.left, captured, defined);
                self.collect_vars_in_expression(&bin.right, captured, defined);
            }
            Expression::Unary(un) => {
                self.collect_vars_in_expression(&un.operand, captured, defined);
            }
            Expression::Call(call) => {
                self.collect_vars_in_expression(&call.callee, captured, defined);
                for arg in &call.args {
                    self.collect_vars_in_expression(arg, captured, defined);
                }
            }
            Expression::MethodCall(method) => {
                self.collect_vars_in_expression(&method.receiver, captured, defined);
                for arg in &method.args {
                    self.collect_vars_in_expression(arg, captured, defined);
                }
            }
            Expression::Index(idx) => {
                self.collect_vars_in_expression(&idx.base, captured, defined);
                self.collect_vars_in_expression(&idx.index, captured, defined);
            }
            Expression::Array(arr) => {
                for elem in &arr.elements {
                    self.collect_vars_in_expression(elem, captured, defined);
                }
            }
            Expression::Grouped(grouped) => {
                self.collect_vars_in_expression(&grouped.inner, captured, defined);
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
                self.collect_vars_in_expression(&lambda.body, captured, &lambda_defined);
            }
            _ => {}
        }
    }

    fn declare_spawn_trampoline(&mut self, _id: u32, info: &SpawnBlockInfo) -> Result<FuncId, CodegenError> {
        // Spawn trampolines take a single pointer parameter (closure data)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(cranelift::prelude::types::I64)); // *mut u8 as i64

        let func_id = self.module
            .declare_function(&info.func_name, Linkage::Local, &sig)
            .map_err(|e| CodegenError::JitCompile(format!("Failed to declare spawn trampoline '{}': {}", info.func_name, e)))?;

        self.functions.insert(info.func_name.clone(), func_id);

        Ok(func_id)
    }

    fn compile_spawn_trampoline(&mut self, info: &SpawnBlockInfo) -> Result<(), CodegenError> {
        let func_id = *self.functions.get(&info.func_name)
            .ok_or_else(|| CodegenError::JitCompile(format!("Trampoline '{}' not declared", info.func_name)))?;

        self.ctx.func.signature = self.module.declarations().get_function_decl(func_id).signature.clone();
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
            struct_defs: &self.struct_defs,
            enum_defs: &self.enum_defs,
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
        };

        // Load captured variables from closure data
        for (i, var_name) in info.captured_vars.iter().enumerate() {
            let var = Variable::new(ctx.var_counter);
            ctx.var_counter += 1;
            builder.declare_var(var, cranelift::prelude::types::I64);

            // Load value from closure data: data_ptr + (i * 8)
            let offset = builder.ins().iconst(cranelift::prelude::types::I64, (i * 8) as i64);
            let addr = builder.ins().iadd(data_ptr, offset);
            let val = builder.ins().load(
                cranelift::prelude::types::I64,
                MemFlags::new(),
                addr,
                0,
            );
            builder.def_var(var, val);
            ctx.variables.insert(var_name.clone(), var);
        }

        // Compile the spawn block body
        // Safety: body_ptr is valid within the same compile() call
        let body = unsafe { &*info.body_ptr };
        for stmt in &body.statements {
            compile_statement(&mut ctx, &mut builder, stmt)?;
            if ctx.block_terminated {
                break;
            }
        }

        // Return (trampolines return void)
        if !ctx.block_terminated {
            builder.ins().return_(&[]);
        }

        builder.finalize();

        self.module
            .define_function(func_id, &mut self.ctx)
            .map_err(|e| CodegenError::JitCompile(format!("Failed to define trampoline '{}': {}", info.func_name, e)))?;

        self.module.clear_context(&mut self.ctx);

        Ok(())
    }

    fn declare_lambda_function(&mut self, info: &LambdaInfo) -> Result<FuncId, CodegenError> {
        let mut sig = self.module.make_signature();

        // First parameter: closure data pointer
        sig.params.push(AbiParam::new(cranelift::prelude::types::I64));

        // Lambda parameters (all as i64 for now)
        for _ in &info.param_names {
            sig.params.push(AbiParam::new(cranelift::prelude::types::I64));
        }

        // Return type (i64 for now)
        sig.returns.push(AbiParam::new(cranelift::prelude::types::I64));

        let func_id = self.module
            .declare_function(&info.func_name, Linkage::Local, &sig)
            .map_err(|e| CodegenError::JitCompile(format!("Failed to declare lambda '{}': {}", info.func_name, e)))?;

        self.functions.insert(info.func_name.clone(), func_id);

        Ok(func_id)
    }

    fn compile_lambda_function(&mut self, info: &LambdaInfo) -> Result<(), CodegenError> {
        let func_id = *self.functions.get(&info.func_name)
            .ok_or_else(|| CodegenError::JitCompile(format!("Lambda '{}' not declared", info.func_name)))?;

        self.ctx.func.signature = self.module.declarations().get_function_decl(func_id).signature.clone();
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
            struct_defs: &self.struct_defs,
            enum_defs: &self.enum_defs,
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
        };

        // Load captured variables from closure data
        for (i, var_name) in info.captured_vars.iter().enumerate() {
            let var = Variable::new(ctx.var_counter);
            ctx.var_counter += 1;
            builder.declare_var(var, cranelift::prelude::types::I64);

            // Load value from closure data: data_ptr + (i * 8)
            let offset = builder.ins().iconst(cranelift::prelude::types::I64, (i * 8) as i64);
            let addr = builder.ins().iadd(data_ptr, offset);
            let val = builder.ins().load(
                cranelift::prelude::types::I64,
                MemFlags::new(),
                addr,
                0,
            );
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

        // Return the result
        if !ctx.block_terminated {
            builder.ins().return_(&[result]);
        }

        builder.finalize();

        self.module
            .define_function(func_id, &mut self.ctx)
            .map_err(|e| CodegenError::JitCompile(format!("Failed to define lambda '{}': {}", info.func_name, e)))?;

        self.module.clear_context(&mut self.ctx);

        Ok(())
    }

    fn declare_function(&mut self, func: &FunctionItem<'_>) -> Result<FuncId, CodegenError> {
        let name = self.interner.resolve(&func.name.symbol);

        let mut sig = self.module.make_signature();

        for param in &func.params {
            let ty = types::naml_to_cranelift(&param.ty);
            sig.params.push(AbiParam::new(ty));
        }

        if let Some(ref return_ty) = func.return_ty {
            let ty = types::naml_to_cranelift(return_ty);
            sig.returns.push(AbiParam::new(ty));
        }

        let func_id = self.module
            .declare_function(name, Linkage::Export, &sig)
            .map_err(|e| CodegenError::JitCompile(format!("Failed to declare function '{}': {}", name, e)))?;

        self.functions.insert(name.to_string(), func_id);

        Ok(func_id)
    }

    fn compile_function(&mut self, func: &FunctionItem<'_>) -> Result<(), CodegenError> {
        let name = self.interner.resolve(&func.name.symbol);
        let func_id = *self.functions.get(name)
            .ok_or_else(|| CodegenError::JitCompile(format!("Function '{}' not declared", name)))?;

        self.ctx.func.signature = self.module.declarations().get_function_decl(func_id).signature.clone();
        self.ctx.func.name = cranelift_codegen::ir::UserFuncName::user(0, func_id.as_u32());

        let mut builder_ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);

        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        let mut ctx = CompileContext {
            interner: self.interner,
            module: &mut self.module,
            functions: &self.functions,
            struct_defs: &self.struct_defs,
            enum_defs: &self.enum_defs,
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
        };

        for (i, param) in func.params.iter().enumerate() {
            let param_name = self.interner.resolve(&param.name.symbol).to_string();
            let val = builder.block_params(entry_block)[i];
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
            // Cleanup all heap variables before implicit void return
            emit_cleanup_all_vars(&mut ctx, &mut builder, None)?;
            builder.ins().return_(&[]);
        }

        builder.finalize();

        self.module
            .define_function(func_id, &mut self.ctx)
            .map_err(|e| CodegenError::JitCompile(format!("Failed to define function '{}': {}", name, e)))?;

        self.module.clear_context(&mut self.ctx);

        Ok(())
    }

    fn declare_method(&mut self, func: &FunctionItem<'_>) -> Result<FuncId, CodegenError> {
        let receiver = func.receiver.as_ref()
            .ok_or_else(|| CodegenError::JitCompile("Method must have receiver".to_string()))?;

        // Get receiver type name (handles both Named and Generic types)
        let receiver_type_name = match &receiver.ty {
            crate::ast::NamlType::Named(ident) => self.interner.resolve(&ident.symbol).to_string(),
            crate::ast::NamlType::Generic(ident, _) => self.interner.resolve(&ident.symbol).to_string(),
            _ => return Err(CodegenError::JitCompile("Method receiver must be a named or generic type".to_string())),
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

        let func_id = self.module
            .declare_function(&full_name, Linkage::Local, &sig)
            .map_err(|e| CodegenError::JitCompile(format!("Failed to declare method '{}': {}", full_name, e)))?;

        self.functions.insert(full_name, func_id);

        Ok(func_id)
    }

    fn compile_method(&mut self, func: &FunctionItem<'_>) -> Result<(), CodegenError> {
        let receiver = func.receiver.as_ref()
            .ok_or_else(|| CodegenError::JitCompile("Method must have receiver".to_string()))?;

        let receiver_type_name = match &receiver.ty {
            crate::ast::NamlType::Named(ident) => self.interner.resolve(&ident.symbol).to_string(),
            crate::ast::NamlType::Generic(ident, _) => self.interner.resolve(&ident.symbol).to_string(),
            _ => return Err(CodegenError::JitCompile("Method receiver must be a named or generic type".to_string())),
        };

        let method_name = self.interner.resolve(&func.name.symbol);
        let full_name = format!("{}_{}", receiver_type_name, method_name);

        let func_id = *self.functions.get(&full_name)
            .ok_or_else(|| CodegenError::JitCompile(format!("Method '{}' not declared", full_name)))?;

        self.ctx.func.signature = self.module.declarations().get_function_decl(func_id).signature.clone();
        self.ctx.func.name = cranelift_codegen::ir::UserFuncName::user(0, func_id.as_u32());

        // Get pointer type before borrowing module
        let ptr_type = self.module.target_config().pointer_type();

        let mut builder_ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);

        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        let mut ctx = CompileContext {
            interner: self.interner,
            module: &mut self.module,
            functions: &self.functions,
            struct_defs: &self.struct_defs,
            enum_defs: &self.enum_defs,
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

        self.module
            .define_function(func_id, &mut self.ctx)
            .map_err(|e| CodegenError::JitCompile(format!("Failed to define method '{}': {}", full_name, e)))?;

        self.module.clear_context(&mut self.ctx);

        Ok(())
    }

    pub fn run_main(&mut self) -> Result<(), CodegenError> {
        self.module.finalize_definitions()
            .map_err(|e| CodegenError::JitCompile(format!("Failed to finalize: {}", e)))?;

        let main_id = self.functions.get("main")
            .ok_or_else(|| CodegenError::Execution("No main function found".to_string()))?;

        let main_ptr = self.module.get_finalized_function(*main_id);

        let main_fn: fn() = unsafe { std::mem::transmute(main_ptr) };
        main_fn();

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HeapType {
    String,
    Array(Option<Box<HeapType>>),
    Map(Option<Box<HeapType>>),
    Struct(Option<String>),
}

fn get_heap_type(naml_ty: &crate::ast::NamlType) -> Option<HeapType> {
    use crate::ast::NamlType;
    match naml_ty {
        NamlType::String => Some(HeapType::String),
        NamlType::Array(elem_ty) => {
            let elem_heap_type = get_heap_type(elem_ty).map(Box::new);
            Some(HeapType::Array(elem_heap_type))
        }
        NamlType::FixedArray(elem_ty, _) => {
            let elem_heap_type = get_heap_type(elem_ty).map(Box::new);
            Some(HeapType::Array(elem_heap_type))
        }
        NamlType::Map(_, val_ty) => {
            let val_heap_type = get_heap_type(val_ty).map(Box::new);
            Some(HeapType::Map(val_heap_type))
        }
        NamlType::Named(_) => Some(HeapType::Struct(None)),
        NamlType::Generic(_, _) => Some(HeapType::Struct(None)),
        _ => None,
    }
}

#[allow(dead_code)]
fn get_heap_type_from_type(ty: &Type) -> Option<HeapType> {
    match ty {
        Type::String => Some(HeapType::String),
        Type::Array(elem_ty) => {
            let elem_heap_type = get_heap_type_from_type(elem_ty).map(Box::new);
            Some(HeapType::Array(elem_heap_type))
        }
        Type::FixedArray(elem_ty, _) => {
            let elem_heap_type = get_heap_type_from_type(elem_ty).map(Box::new);
            Some(HeapType::Array(elem_heap_type))
        }
        Type::Map(_, val_ty) => {
            let val_heap_type = get_heap_type_from_type(val_ty).map(Box::new);
            Some(HeapType::Map(val_heap_type))
        }
        Type::Struct(_) => Some(HeapType::Struct(None)),
        Type::Generic(_, _) => Some(HeapType::Struct(None)),
        _ => None,
    }
}

struct CompileContext<'a> {
    interner: &'a Rodeo,
    module: &'a mut JITModule,
    functions: &'a HashMap<String, FuncId>,
    struct_defs: &'a HashMap<String, StructDef>,
    enum_defs: &'a HashMap<String, EnumDef>,
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
}

fn compile_statement(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    stmt: &Statement<'_>,
) -> Result<(), CodegenError> {
    match stmt {
        Statement::Var(var_stmt) => {
            let var_name = ctx.interner.resolve(&var_stmt.name.symbol).to_string();
            let ty = if let Some(ref naml_ty) = var_stmt.ty {
                types::naml_to_cranelift(naml_ty)
            } else {
                cranelift::prelude::types::I64
            };

            // Check if this is a string variable
            let is_string_var = matches!(var_stmt.ty.as_ref(), Some(crate::ast::NamlType::String));

            // Track heap type for cleanup
            if let Some(ref naml_ty) = var_stmt.ty {
                if let Some(heap_type) = get_heap_type(naml_ty) {
                    ctx.var_heap_types.insert(var_name.clone(), heap_type);
                }
            }

            let var = Variable::new(ctx.var_counter);
            ctx.var_counter += 1;
            builder.declare_var(var, ty);

            if let Some(ref init) = var_stmt.init {
                let mut val = compile_expression(ctx, builder, init)?;

                // Box string literals as NamlString* for consistent memory management
                if is_string_var {
                    if matches!(init, Expression::Literal(LiteralExpr { value: Literal::String(_), .. })) {
                        val = call_string_from_cstr(ctx, builder, val)?;
                    }
                }

                builder.def_var(var, val);
                // Incref the value since we're storing a reference
                let heap_type_clone = ctx.var_heap_types.get(&var_name).cloned();
                if let Some(ref heap_type) = heap_type_clone {
                    emit_incref(ctx, builder, val, heap_type)?;
                }
            } else {
                let zero = builder.ins().iconst(ty, 0);
                builder.def_var(var, zero);
            }

            ctx.variables.insert(var_name, var);
        }

        Statement::Assign(assign) => {
            match &assign.target {
                Expression::Identifier(ident) => {
                    let var_name = ctx.interner.resolve(&ident.ident.symbol).to_string();

                    if let Some(&var) = ctx.variables.get(&var_name) {
                        // Clone heap type before mutable operations
                        let heap_type_clone = ctx.var_heap_types.get(&var_name).cloned();

                        // For heap variables: decref old value before assigning new one
                        if let Some(ref heap_type) = heap_type_clone {
                            let old_val = builder.use_var(var);
                            emit_decref(ctx, builder, old_val, heap_type)?;
                        }

                        let mut val = compile_expression(ctx, builder, &assign.value)?;

                        // Box string literals as NamlString* when assigning to string variables
                        if matches!(&heap_type_clone, Some(HeapType::String)) {
                            if matches!(&assign.value, Expression::Literal(LiteralExpr { value: Literal::String(_), .. })) {
                                val = call_string_from_cstr(ctx, builder, val)?;
                            }
                        }

                        builder.def_var(var, val);

                        // Incref the new value since we're storing a new reference
                        if let Some(ref heap_type) = heap_type_clone {
                            emit_incref(ctx, builder, val, heap_type)?;
                        }
                    } else {
                        return Err(CodegenError::JitCompile(format!("Undefined variable: {}", var_name)));
                    }
                }
                Expression::Index(index_expr) => {
                    let base = compile_expression(ctx, builder, &index_expr.base)?;
                    let value = compile_expression(ctx, builder, &assign.value)?;

                    // Check if index is a string literal - if so, use map_set with NamlString conversion
                    if let Expression::Literal(LiteralExpr { value: Literal::String(_), .. }) = &*index_expr.index {
                        let cstr_ptr = compile_expression(ctx, builder, &index_expr.index)?;
                        let naml_str = call_string_from_cstr(ctx, builder, cstr_ptr)?;
                        call_map_set(ctx, builder, base, naml_str, value)?;
                    } else {
                        // Default to array set for integer indices
                        let index = compile_expression(ctx, builder, &index_expr.index)?;
                        call_array_set(ctx, builder, base, index, value)?;
                    }
                }
                Expression::Field(field_expr) => {
                    // Field assignment: base.field = value
                    // Get the base pointer (struct/exception)
                    let base_ptr = compile_expression(ctx, builder, &field_expr.base)?;
                    let value = compile_expression(ctx, builder, &assign.value)?;
                    let field_name = ctx.interner.resolve(&field_expr.field.symbol).to_string();

                    // Determine field offset based on struct type
                    // For exceptions: offset 0 is message, then fields at 8, 16, etc.
                    // For structs: fields at 0, 8, 16, etc.
                    if let Expression::Identifier(ident) = &*field_expr.base {
                        let _var_name = ctx.interner.resolve(&ident.ident.symbol).to_string();
                        // Get the type annotation to determine struct/exception type
                        // Note: use ident.span (IdentExpr span), not ident.ident.span (Ident span)
                        if let Some(type_ann) = ctx.annotations.get_type(ident.span) {
                            if let crate::typechecker::Type::Exception(exc_name) = type_ann {
                                let exc_name_str = ctx.interner.resolve(&exc_name).to_string();
                                if let Some(struct_def) = ctx.struct_defs.get(&exc_name_str) {
                                    // Find field offset (message at 0, then fields at 8, 16, ...)
                                    let offset = if field_name == "message" {
                                        0
                                    } else if let Some(idx) = struct_def.fields.iter().position(|f| f == &field_name) {
                                        8 + (idx * 8) as i32
                                    } else {
                                        return Err(CodegenError::JitCompile(format!("Unknown field: {}", field_name)));
                                    };
                                    builder.ins().store(MemFlags::new(), value, base_ptr, offset);
                                    return Ok(());
                                }
                            } else if let crate::typechecker::Type::Struct(struct_type) = type_ann {
                                let struct_name = ctx.interner.resolve(&struct_type.name).to_string();
                                if let Some(struct_def) = ctx.struct_defs.get(&struct_name) {
                                    if let Some(idx) = struct_def.fields.iter().position(|f| f == &field_name) {
                                        let offset = (idx * 8) as i32;
                                        builder.ins().store(MemFlags::new(), value, base_ptr, offset);
                                        return Ok(());
                                    }
                                }
                            }
                        }
                    }

                    return Err(CodegenError::JitCompile(format!("Cannot assign to field: {}", field_name)));
                }
                _ => {
                    return Err(CodegenError::Unsupported(
                        format!("Assignment target not supported: {:?}", std::mem::discriminant(&assign.target))
                    ));
                }
            }
        }

        Statement::Return(ret) => {
            if let Some(ref expr) = ret.value {
                let mut val = compile_expression(ctx, builder, expr)?;

                // Convert string literals to NamlString when returning
                let return_type = ctx.annotations.get_type(expr.span());
                if matches!(return_type, Some(Type::String)) {
                    if matches!(expr, Expression::Literal(LiteralExpr { value: Literal::String(_), .. })) {
                        val = call_string_from_cstr(ctx, builder, val)?;
                    }
                }

                // Determine if we're returning a local heap variable (ownership transfer)
                let returned_var = get_returned_var_name(expr, ctx.interner);
                let exclude_var = returned_var.as_ref().and_then(|name| {
                    if ctx.var_heap_types.contains_key(name) {
                        Some(name.as_str())
                    } else {
                        None
                    }
                });

                // Cleanup all local heap variables except the returned one
                emit_cleanup_all_vars(ctx, builder, exclude_var)?;
                builder.ins().return_(&[val]);
            } else {
                // Void return - cleanup all heap variables
                emit_cleanup_all_vars(ctx, builder, None)?;
                builder.ins().return_(&[]);
            }
            ctx.block_terminated = true;
        }

        Statement::Expression(expr_stmt) => {
            compile_expression(ctx, builder, &expr_stmt.expr)?;
        }

        Statement::If(if_stmt) => {
            let condition = compile_expression(ctx, builder, &if_stmt.condition)?;

            let then_block = builder.create_block();
            let else_block = builder.create_block();
            let merge_block = builder.create_block();

            builder.ins().brif(condition, then_block, &[], else_block, &[]);

            builder.switch_to_block(then_block);
            builder.seal_block(then_block);
            ctx.block_terminated = false;
            for stmt in &if_stmt.then_branch.statements {
                compile_statement(ctx, builder, stmt)?;
                if ctx.block_terminated {
                    break;
                }
            }
            if !ctx.block_terminated {
                builder.ins().jump(merge_block, &[]);
            }

            builder.switch_to_block(else_block);
            builder.seal_block(else_block);
            ctx.block_terminated = false;
            if let Some(ref else_branch) = if_stmt.else_branch {
                match else_branch {
                    crate::ast::ElseBranch::Else(else_block_stmt) => {
                        for stmt in &else_block_stmt.statements {
                            compile_statement(ctx, builder, stmt)?;
                            if ctx.block_terminated {
                                break;
                            }
                        }
                    }
                    crate::ast::ElseBranch::ElseIf(else_if) => {
                        let nested_if = Statement::If(crate::ast::IfStmt {
                            condition: else_if.condition.clone(),
                            then_branch: else_if.then_branch.clone(),
                            else_branch: else_if.else_branch.clone(),
                            span: else_if.span,
                        });
                        compile_statement(ctx, builder, &nested_if)?;
                    }
                }
            }
            if !ctx.block_terminated {
                builder.ins().jump(merge_block, &[]);
            }

            builder.switch_to_block(merge_block);
            builder.seal_block(merge_block);
            ctx.block_terminated = false;
        }

        Statement::While(while_stmt) => {
            let header_block = builder.create_block();
            let body_block = builder.create_block();
            let exit_block = builder.create_block();

            builder.ins().jump(header_block, &[]);

            builder.switch_to_block(header_block);
            let condition = compile_expression(ctx, builder, &while_stmt.condition)?;
            builder.ins().brif(condition, body_block, &[], exit_block, &[]);

            builder.switch_to_block(body_block);
            builder.seal_block(body_block);
            ctx.block_terminated = false;
            for stmt in &while_stmt.body.statements {
                compile_statement(ctx, builder, stmt)?;
                if ctx.block_terminated {
                    break;
                }
            }
            if !ctx.block_terminated {
                builder.ins().jump(header_block, &[]);
            }

            builder.seal_block(header_block);
            builder.switch_to_block(exit_block);
            builder.seal_block(exit_block);
            ctx.block_terminated = false;
        }

        Statement::For(for_stmt) => {
            // Check if iterable is a range expression (binary op with Range or RangeIncl)
            let range_info = match &for_stmt.iterable {
                Expression::Binary(bin) if matches!(bin.op, BinaryOp::Range | BinaryOp::RangeIncl) => {
                    Some((bin.left, bin.right, matches!(bin.op, BinaryOp::RangeIncl)))
                }
                Expression::Range(range_expr) => {
                    // Handle Expression::Range if it exists
                    range_expr.start.zip(range_expr.end.as_ref()).map(|(s, e)| (s, *e, range_expr.inclusive))
                }
                _ => None
            };

            // Check if iterable is a string (via type annotation, string literal, or heap type)
            let is_string_literal = matches!(
                &for_stmt.iterable,
                Expression::Literal(LiteralExpr { value: Literal::String(_), .. })
            );

            // Also check if it's a string variable by looking at var_heap_types
            let is_string_var = if let Expression::Identifier(ident) = &for_stmt.iterable {
                let var_name = ctx.interner.resolve(&ident.ident.symbol).to_string();
                matches!(ctx.var_heap_types.get(&var_name), Some(HeapType::String))
            } else {
                false
            };

            let is_string = is_string_literal || is_string_var || matches!(
                ctx.annotations.get_type(for_stmt.iterable.span()),
                Some(Type::String)
            );

            if let Some((start_expr, end_expr, inclusive)) = range_info {
                // Handle range iteration directly without array allocation
                // Get start and end values
                let start = compile_expression(ctx, builder, start_expr)?;
                let end = compile_expression(ctx, builder, end_expr)?;

                // Create index variable (this is both the loop counter and the value)
                let idx_var = Variable::new(ctx.var_counter);
                ctx.var_counter += 1;
                builder.declare_var(idx_var, cranelift::prelude::types::I64);
                builder.def_var(idx_var, start);

                // Bind the value variable to the same as index
                let val_name = ctx.interner.resolve(&for_stmt.value.symbol).to_string();
                ctx.variables.insert(val_name, idx_var);

                // Optionally create separate index binding (for iteration count from 0)
                let iter_var = if for_stmt.index.is_some() {
                    let iter_var = Variable::new(ctx.var_counter);
                    ctx.var_counter += 1;
                    builder.declare_var(iter_var, cranelift::prelude::types::I64);
                    let zero = builder.ins().iconst(cranelift::prelude::types::I64, 0);
                    builder.def_var(iter_var, zero);
                    if let Some(ref idx_ident) = for_stmt.index {
                        let idx_name = ctx.interner.resolve(&idx_ident.symbol).to_string();
                        ctx.variables.insert(idx_name, iter_var);
                    }
                    Some(iter_var)
                } else {
                    None
                };

                let header_block = builder.create_block();
                let body_block = builder.create_block();
                let exit_block = builder.create_block();

                let prev_loop_exit = ctx.loop_exit_block.take();
                let prev_loop_header = ctx.loop_header_block.take();
                ctx.loop_exit_block = Some(exit_block);
                ctx.loop_header_block = Some(header_block);

                builder.ins().jump(header_block, &[]);

                // Header: check if idx < end (or <= for inclusive)
                builder.switch_to_block(header_block);
                let idx_val = builder.use_var(idx_var);
                let cond = if inclusive {
                    builder.ins().icmp(IntCC::SignedLessThanOrEqual, idx_val, end)
                } else {
                    builder.ins().icmp(IntCC::SignedLessThan, idx_val, end)
                };
                builder.ins().brif(cond, body_block, &[], exit_block, &[]);

                // Body
                builder.switch_to_block(body_block);
                builder.seal_block(body_block);
                ctx.block_terminated = false;

                for stmt in &for_stmt.body.statements {
                    compile_statement(ctx, builder, stmt)?;
                    if ctx.block_terminated {
                        break;
                    }
                }

                // Increment index
                if !ctx.block_terminated {
                    let idx_val = builder.use_var(idx_var);
                    let one = builder.ins().iconst(cranelift::prelude::types::I64, 1);
                    let next_idx = builder.ins().iadd(idx_val, one);
                    builder.def_var(idx_var, next_idx);

                    // Also increment iteration counter if present
                    if let Some(iter_v) = iter_var {
                        let iter_val = builder.use_var(iter_v);
                        let next_iter = builder.ins().iadd(iter_val, one);
                        builder.def_var(iter_v, next_iter);
                    }

                    builder.ins().jump(header_block, &[]);
                }

                builder.seal_block(header_block);
                builder.switch_to_block(exit_block);
                builder.seal_block(exit_block);
                ctx.block_terminated = false;

                ctx.loop_exit_block = prev_loop_exit;
                ctx.loop_header_block = prev_loop_header;
            } else if is_string {
                // Handle string character iteration
                let raw_str_ptr = compile_expression(ctx, builder, &for_stmt.iterable)?;

                // If the iterable is a string literal, convert it to NamlString*
                let str_ptr = if matches!(&for_stmt.iterable, Expression::Literal(LiteralExpr { value: Literal::String(_), .. })) {
                    // Convert C string to NamlString*
                    call_string_from_cstr(ctx, builder, raw_str_ptr)?
                } else {
                    raw_str_ptr
                };

                let len = call_string_char_len(ctx, builder, str_ptr)?;

                // Create index variable
                let idx_var = Variable::new(ctx.var_counter);
                ctx.var_counter += 1;
                builder.declare_var(idx_var, cranelift::prelude::types::I64);
                let zero = builder.ins().iconst(cranelift::prelude::types::I64, 0);
                builder.def_var(idx_var, zero);

                // Create character variable (holds codepoint as int)
                let char_var = Variable::new(ctx.var_counter);
                ctx.var_counter += 1;
                builder.declare_var(char_var, cranelift::prelude::types::I64);
                let val_name = ctx.interner.resolve(&for_stmt.value.symbol).to_string();
                ctx.variables.insert(val_name, char_var);

                // Bind index if requested
                if let Some(ref idx_ident) = for_stmt.index {
                    let idx_name = ctx.interner.resolve(&idx_ident.symbol).to_string();
                    ctx.variables.insert(idx_name, idx_var);
                }

                let header_block = builder.create_block();
                let body_block = builder.create_block();
                let exit_block = builder.create_block();

                let prev_loop_exit = ctx.loop_exit_block.take();
                let prev_loop_header = ctx.loop_header_block.take();
                ctx.loop_exit_block = Some(exit_block);
                ctx.loop_header_block = Some(header_block);

                builder.ins().jump(header_block, &[]);

                builder.switch_to_block(header_block);
                let idx_val = builder.use_var(idx_var);
                let cond = builder.ins().icmp(IntCC::SignedLessThan, idx_val, len);
                builder.ins().brif(cond, body_block, &[], exit_block, &[]);

                builder.switch_to_block(body_block);
                builder.seal_block(body_block);
                ctx.block_terminated = false;

                // Get character at current index
                let idx_val = builder.use_var(idx_var);
                let char_code = call_string_char_at(ctx, builder, str_ptr, idx_val)?;
                builder.def_var(char_var, char_code);

                for stmt in &for_stmt.body.statements {
                    compile_statement(ctx, builder, stmt)?;
                    if ctx.block_terminated {
                        break;
                    }
                }

                if !ctx.block_terminated {
                    let idx_val = builder.use_var(idx_var);
                    let one = builder.ins().iconst(cranelift::prelude::types::I64, 1);
                    let next_idx = builder.ins().iadd(idx_val, one);
                    builder.def_var(idx_var, next_idx);
                    builder.ins().jump(header_block, &[]);
                }

                builder.seal_block(header_block);
                builder.switch_to_block(exit_block);
                builder.seal_block(exit_block);
                ctx.block_terminated = false;

                ctx.loop_exit_block = prev_loop_exit;
                ctx.loop_header_block = prev_loop_header;
            } else {
                // Original array iteration code
                let arr_ptr = compile_expression(ctx, builder, &for_stmt.iterable)?;
                let len = call_array_len(ctx, builder, arr_ptr)?;

                let idx_var = Variable::new(ctx.var_counter);
                ctx.var_counter += 1;
                builder.declare_var(idx_var, cranelift::prelude::types::I64);
                let zero = builder.ins().iconst(cranelift::prelude::types::I64, 0);
                builder.def_var(idx_var, zero);

                let val_var = Variable::new(ctx.var_counter);
                ctx.var_counter += 1;
                builder.declare_var(val_var, cranelift::prelude::types::I64);
                let val_name = ctx.interner.resolve(&for_stmt.value.symbol).to_string();
                ctx.variables.insert(val_name, val_var);

                if let Some(ref idx_ident) = for_stmt.index {
                    let idx_name = ctx.interner.resolve(&idx_ident.symbol).to_string();
                    ctx.variables.insert(idx_name, idx_var);
                }

                let header_block = builder.create_block();
                let body_block = builder.create_block();
                let exit_block = builder.create_block();

                let prev_loop_exit = ctx.loop_exit_block.take();
                let prev_loop_header = ctx.loop_header_block.take();
                ctx.loop_exit_block = Some(exit_block);
                ctx.loop_header_block = Some(header_block);

                builder.ins().jump(header_block, &[]);

                builder.switch_to_block(header_block);
                let idx_val = builder.use_var(idx_var);
                let cond = builder.ins().icmp(IntCC::SignedLessThan, idx_val, len);
                builder.ins().brif(cond, body_block, &[], exit_block, &[]);

                builder.switch_to_block(body_block);
                builder.seal_block(body_block);
                ctx.block_terminated = false;

                let idx_val = builder.use_var(idx_var);
                let elem = call_array_get(ctx, builder, arr_ptr, idx_val)?;
                builder.def_var(val_var, elem);

                for stmt in &for_stmt.body.statements {
                    compile_statement(ctx, builder, stmt)?;
                    if ctx.block_terminated {
                        break;
                    }
                }

                if !ctx.block_terminated {
                    let idx_val = builder.use_var(idx_var);
                    let one = builder.ins().iconst(cranelift::prelude::types::I64, 1);
                    let next_idx = builder.ins().iadd(idx_val, one);
                    builder.def_var(idx_var, next_idx);
                    builder.ins().jump(header_block, &[]);
                }

                builder.seal_block(header_block);
                builder.switch_to_block(exit_block);
                builder.seal_block(exit_block);
                ctx.block_terminated = false;

                ctx.loop_exit_block = prev_loop_exit;
                ctx.loop_header_block = prev_loop_header;
            }
        }

        Statement::Loop(loop_stmt) => {
            let body_block = builder.create_block();
            let exit_block = builder.create_block();

            let prev_loop_exit = ctx.loop_exit_block.take();
            let prev_loop_header = ctx.loop_header_block.take();
            ctx.loop_exit_block = Some(exit_block);
            ctx.loop_header_block = Some(body_block);

            builder.ins().jump(body_block, &[]);

            builder.switch_to_block(body_block);
            builder.seal_block(body_block);
            ctx.block_terminated = false;

            for stmt in &loop_stmt.body.statements {
                compile_statement(ctx, builder, stmt)?;
                if ctx.block_terminated {
                    break;
                }
            }

            if !ctx.block_terminated {
                builder.ins().jump(body_block, &[]);
            }

            builder.switch_to_block(exit_block);
            builder.seal_block(exit_block);
            ctx.block_terminated = false;

            ctx.loop_exit_block = prev_loop_exit;
            ctx.loop_header_block = prev_loop_header;
        }

        Statement::Break(_) => {
            if let Some(exit_block) = ctx.loop_exit_block {
                builder.ins().jump(exit_block, &[]);
                ctx.block_terminated = true;
            } else {
                return Err(CodegenError::JitCompile("break outside of loop".to_string()));
            }
        }

        Statement::Continue(_) => {
            if let Some(header_block) = ctx.loop_header_block {
                builder.ins().jump(header_block, &[]);
                ctx.block_terminated = true;
            } else {
                return Err(CodegenError::JitCompile("continue outside of loop".to_string()));
            }
        }

        Statement::Switch(switch_stmt) => {
            let scrutinee = compile_expression(ctx, builder, &switch_stmt.scrutinee)?;
            let merge_block = builder.create_block();
            let default_block = builder.create_block();

            // Create case blocks and check blocks
            let mut case_blocks = Vec::new();
            let mut check_blocks = Vec::new();

            for _ in &switch_stmt.cases {
                case_blocks.push(builder.create_block());
                check_blocks.push(builder.create_block());
            }

            // Jump to first check (or default if no cases)
            if !check_blocks.is_empty() {
                builder.ins().jump(check_blocks[0], &[]);
            } else {
                builder.ins().jump(default_block, &[]);
            }

            // Build the chain of checks using pattern matching
            for (i, case) in switch_stmt.cases.iter().enumerate() {
                builder.switch_to_block(check_blocks[i]);
                builder.seal_block(check_blocks[i]);

                // Use compile_pattern_match instead of compile_expression
                let cond = compile_pattern_match(ctx, builder, &case.pattern, scrutinee)?;

                let next_check = if i + 1 < switch_stmt.cases.len() {
                    check_blocks[i + 1]
                } else {
                    default_block
                };

                builder.ins().brif(cond, case_blocks[i], &[], next_check, &[]);
            }

            // Compile each case body with pattern variable bindings
            for (i, case) in switch_stmt.cases.iter().enumerate() {
                builder.switch_to_block(case_blocks[i]);
                builder.seal_block(case_blocks[i]);
                ctx.block_terminated = false;

                // Bind pattern variables before executing the case body
                bind_pattern_vars(ctx, builder, &case.pattern, scrutinee)?;

                for stmt in &case.body.statements {
                    compile_statement(ctx, builder, stmt)?;
                    if ctx.block_terminated {
                        break;
                    }
                }

                if !ctx.block_terminated {
                    builder.ins().jump(merge_block, &[]);
                }
            }

            // Compile default
            builder.switch_to_block(default_block);
            builder.seal_block(default_block);
            ctx.block_terminated = false;

            if let Some(ref default_body) = switch_stmt.default {
                for stmt in &default_body.statements {
                    compile_statement(ctx, builder, stmt)?;
                    if ctx.block_terminated {
                        break;
                    }
                }
            }

            if !ctx.block_terminated {
                builder.ins().jump(merge_block, &[]);
            }

            builder.switch_to_block(merge_block);
            builder.seal_block(merge_block);
            ctx.block_terminated = false;
        }

        Statement::Throw(throw_stmt) => {
            // Compile the exception value
            let exception_ptr = compile_expression(ctx, builder, &throw_stmt.value)?;

            // Set the current exception in thread-local storage
            call_exception_set(ctx, builder, exception_ptr)?;

            // Return 0 (indicates exception) from the function
            let zero = builder.ins().iconst(cranelift::prelude::types::I64, 0);
            builder.ins().return_(&[zero]);
            ctx.block_terminated = true;
        }

        Statement::Const(const_stmt) => {
            // Constants are treated like immutable variables
            let var_name = ctx.interner.resolve(&const_stmt.name.symbol).to_string();
            let var = Variable::new(ctx.variables.len());
            ctx.variables.insert(var_name.clone(), var);
            builder.declare_var(var, cranelift::prelude::types::I64);

            let init_val = compile_expression(ctx, builder, &const_stmt.init)?;
            builder.def_var(var, init_val);
        }

        Statement::Block(block_stmt) => {
            for stmt in &block_stmt.statements {
                compile_statement(ctx, builder, stmt)?;
                if ctx.block_terminated {
                    break;
                }
            }
        }

        _ => {
            return Err(CodegenError::Unsupported(
                format!("Statement type not yet implemented: {:?}", std::mem::discriminant(stmt))
            ));
        }
    }

    Ok(())
}

fn compile_pattern_match(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    pattern: &crate::ast::Pattern<'_>,
    scrutinee: Value,
) -> Result<Value, CodegenError> {
    use crate::ast::Pattern;

    match pattern {
        Pattern::Literal(lit) => {
            let lit_val = compile_literal(ctx, builder, &lit.value)?;
            Ok(builder.ins().icmp(IntCC::Equal, scrutinee, lit_val))
        }

        Pattern::Identifier(ident) => {
            let name = ctx.interner.resolve(&ident.ident.symbol).to_string();
            for enum_def in ctx.enum_defs.values() {
                if let Some(variant) = enum_def.variants.iter().find(|v| v.name == name) {
                    let tag = builder.ins().load(cranelift::prelude::types::I32, MemFlags::new(), scrutinee, 0);
                    let expected_tag = builder.ins().iconst(cranelift::prelude::types::I32, variant.tag as i64);
                    return Ok(builder.ins().icmp(IntCC::Equal, tag, expected_tag));
                }
            }
            Ok(builder.ins().iconst(cranelift::prelude::types::I8, 1))
        }

        Pattern::Variant(variant) => {
            if variant.path.is_empty() {
                return Err(CodegenError::JitCompile("Empty variant path".to_string()));
            }
            let (enum_name, variant_name) = if variant.path.len() == 1 {
                let var_name = ctx.interner.resolve(&variant.path[0].symbol).to_string();
                let mut found = None;
                for (e_name, enum_def) in ctx.enum_defs.iter() {
                    if enum_def.variants.iter().any(|v| v.name == var_name) {
                        found = Some((e_name.clone(), var_name.clone()));
                        break;
                    }
                }

                match found {
                    Some(pair) => pair,
                    None => return Err(CodegenError::JitCompile(format!(
                        "Unknown variant: {}",
                        var_name
                    ))),
                }
            } else {
                // Qualified path
                let enum_name = ctx.interner.resolve(&variant.path[0].symbol).to_string();
                let variant_name = ctx.interner.resolve(&variant.path.last().unwrap().symbol).to_string();
                (enum_name, variant_name)
            };

            if let Some(enum_def) = ctx.enum_defs.get(&enum_name) {
                if let Some(var_def) = enum_def.variants.iter().find(|v| v.name == variant_name) {
                    let tag = builder.ins().load(cranelift::prelude::types::I32, MemFlags::new(), scrutinee, 0);
                    let expected_tag = builder.ins().iconst(cranelift::prelude::types::I32, var_def.tag as i64);
                    return Ok(builder.ins().icmp(IntCC::Equal, tag, expected_tag));
                }
            }

            Err(CodegenError::JitCompile(format!(
                "Unknown enum variant: {}::{}",
                enum_name, variant_name
            )))
        }

        Pattern::Wildcard(_) => {
            Ok(builder.ins().iconst(cranelift::prelude::types::I8, 1))
        }

        Pattern::_Phantom(_) => {
            Ok(builder.ins().iconst(cranelift::prelude::types::I8, 0))
        }
    }
}

fn bind_pattern_vars(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    pattern: &crate::ast::Pattern<'_>,
    scrutinee: Value,
) -> Result<(), CodegenError> {
    use crate::ast::Pattern;

    match pattern {
        Pattern::Variant(variant) if !variant.bindings.is_empty() => {
            // Get the enum and variant info
            let (enum_name, variant_name) = if variant.path.len() == 1 {
                let var_name = ctx.interner.resolve(&variant.path[0].symbol).to_string();

                // Search all enum definitions for this variant
                let mut found = None;
                for (e_name, enum_def) in ctx.enum_defs.iter() {
                    if enum_def.variants.iter().any(|v| v.name == var_name) {
                        found = Some((e_name.clone(), var_name.clone()));
                        break;
                    }
                }

                match found {
                    Some(pair) => pair,
                    None => return Ok(()), // Variant not found, nothing to bind
                }
            } else {
                let enum_name = ctx.interner.resolve(&variant.path[0].symbol).to_string();
                let variant_name = ctx.interner.resolve(&variant.path.last().unwrap().symbol).to_string();
                (enum_name, variant_name)
            };

            if let Some(enum_def) = ctx.enum_defs.get(&enum_name) {
                if let Some(var_def) = enum_def.variants.iter().find(|v| v.name == variant_name) {
                    for (i, binding) in variant.bindings.iter().enumerate() {
                        let binding_name = ctx.interner.resolve(&binding.symbol).to_string();
                        let offset = (var_def.data_offset + i * 8) as i32;

                        let field_val = builder.ins().load(
                            cranelift::prelude::types::I64,
                            MemFlags::new(),
                            scrutinee,
                            offset,
                        );

                        let var = Variable::new(ctx.var_counter);
                        ctx.var_counter += 1;
                        builder.declare_var(var, cranelift::prelude::types::I64);
                        builder.def_var(var, field_val);
                        ctx.variables.insert(binding_name, var);
                    }
                }
            }
        }

        Pattern::Identifier(ident) => {
            // Check if it's not a variant name (binding patterns)
            let name = ctx.interner.resolve(&ident.ident.symbol).to_string();

            // Check if it's a variant name - don't bind in that case
            let is_variant = ctx.enum_defs.values()
                .any(|def| def.variants.iter().any(|v| v.name == name));

            if !is_variant {
                let var = Variable::new(ctx.var_counter);
                ctx.var_counter += 1;
                builder.declare_var(var, cranelift::prelude::types::I64);
                builder.def_var(var, scrutinee);
                ctx.variables.insert(name, var);
            }
        }

        _ => {}
    }

    Ok(())
}

fn compile_expression(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    expr: &Expression<'_>,
) -> Result<Value, CodegenError> {
    match expr {
        Expression::Literal(lit_expr) => compile_literal(ctx, builder, &lit_expr.value),

        Expression::Identifier(ident) => {
            let name = ctx.interner.resolve(&ident.ident.symbol).to_string();
            if let Some(&var) = ctx.variables.get(&name) {
                Ok(builder.use_var(var))
            } else {
                Err(CodegenError::JitCompile(format!("Undefined variable: {}", name)))
            }
        }

        Expression::Path(path_expr) => {
            // Handle enum variant access: EnumType::Variant
            if path_expr.segments.len() == 2 {
                let enum_name = ctx.interner.resolve(&path_expr.segments[0].symbol).to_string();
                let variant_name = ctx.interner.resolve(&path_expr.segments[1].symbol).to_string();

                if let Some(enum_def) = ctx.enum_defs.get(&enum_name) {
                    if let Some(variant) = enum_def.variants.iter().find(|v| v.name == variant_name) {
                        // Unit variant - allocate stack slot and set tag
                        let slot = builder.create_sized_stack_slot(StackSlotData::new(
                            StackSlotKind::ExplicitSlot,
                            enum_def.size as u32,
                            0,
                        ));
                        let slot_addr = builder.ins().stack_addr(cranelift::prelude::types::I64, slot, 0);

                        // Store tag at offset 0
                        let tag_val = builder.ins().iconst(cranelift::prelude::types::I32, variant.tag as i64);
                        builder.ins().store(MemFlags::new(), tag_val, slot_addr, 0);

                        // Return pointer to stack slot
                        return Ok(slot_addr);
                    }
                }
            }

            Err(CodegenError::Unsupported(format!(
                "Path expression not supported: {:?}",
                path_expr.segments.iter()
                    .map(|s| ctx.interner.resolve(&s.symbol))
                    .collect::<Vec<_>>()
            )))
        }

        Expression::Binary(bin) => {
            // Handle null coalescing operator: lhs ?? rhs
            // Returns lhs if not null/none, otherwise rhs
            if bin.op == BinaryOp::NullCoalesce {
                let lhs = compile_expression(ctx, builder, &bin.left)?;

                // Create blocks for branching
                let then_block = builder.create_block();
                let else_block = builder.create_block();
                let merge_block = builder.create_block();

                // Add block parameter for the result
                builder.append_block_param(merge_block, cranelift::prelude::types::I64);

                // Check if lhs is null (0)
                let zero = builder.ins().iconst(cranelift::prelude::types::I64, 0);
                let is_null = builder.ins().icmp(IntCC::Equal, lhs, zero);
                builder.ins().brif(is_null, else_block, &[], then_block, &[]);

                // Then block: lhs is not null, use lhs
                builder.switch_to_block(then_block);
                builder.seal_block(then_block);
                builder.ins().jump(merge_block, &[lhs]);

                // Else block: lhs is null, evaluate and use rhs
                builder.switch_to_block(else_block);
                builder.seal_block(else_block);
                let rhs = compile_expression(ctx, builder, &bin.right)?;
                builder.ins().jump(merge_block, &[rhs]);

                // Merge block: result is block parameter
                builder.switch_to_block(merge_block);
                builder.seal_block(merge_block);
                let result = builder.block_params(merge_block)[0];
                return Ok(result);
            }

            // Check if this is a string comparison (Eq/NotEq)
            let lhs_type = ctx.annotations.get_type(bin.left.span());
            if matches!(lhs_type, Some(Type::String)) && matches!(bin.op, BinaryOp::Eq | BinaryOp::NotEq) {
                let lhs = compile_expression(ctx, builder, &bin.left)?;
                let rhs = compile_expression(ctx, builder, &bin.right)?;
                // Convert lhs to NamlString if it's a string literal
                let lhs_str = if matches!(&*bin.left, Expression::Literal(LiteralExpr { value: Literal::String(_), .. })) {
                    call_string_from_cstr(ctx, builder, lhs)?
                } else {
                    lhs
                };
                // Convert rhs to NamlString if it's a string literal
                let rhs_str = if matches!(&*bin.right, Expression::Literal(LiteralExpr { value: Literal::String(_), .. })) {
                    call_string_from_cstr(ctx, builder, rhs)?
                } else {
                    rhs
                };
                let result = call_string_equals(ctx, builder, lhs_str, rhs_str)?;
                if bin.op == BinaryOp::NotEq {
                    // Negate the result
                    let one = builder.ins().iconst(cranelift::prelude::types::I64, 1);
                    return Ok(builder.ins().bxor(result, one));
                }
                return Ok(result);
            }
            let lhs = compile_expression(ctx, builder, &bin.left)?;
            let rhs = compile_expression(ctx, builder, &bin.right)?;
            compile_binary_op(builder, &bin.op, lhs, rhs)
        }

        Expression::Unary(unary) => {
            let operand = compile_expression(ctx, builder, &unary.operand)?;
            compile_unary_op(builder, &unary.op, operand)
        }

        Expression::Call(call) => {
            if let Expression::Identifier(ident) = call.callee {
                let func_name = ctx.interner.resolve(&ident.ident.symbol);

                match func_name {
                    "printf" | "print" | "println" => {
                        return compile_print_call(ctx, builder, &call.args, func_name == "println");
                    }
                    "sleep" => {
                        if call.args.is_empty() {
                            return Err(CodegenError::JitCompile("sleep requires milliseconds argument".to_string()));
                        }
                        let ms = compile_expression(ctx, builder, &call.args[0])?;
                        return call_sleep(ctx, builder, ms);
                    }
                    "wait_all" => {
                        return call_wait_all(ctx, builder);
                    }
                    "make_channel" => {
                        let capacity = if call.args.is_empty() {
                            builder.ins().iconst(cranelift::prelude::types::I64, 1)
                        } else {
                            compile_expression(ctx, builder, &call.args[0])?
                        };
                        return call_channel_new(ctx, builder, capacity);
                    }
                    _ => {}
                }

                // Check for normal (naml) function
                if let Some(&func_id) = ctx.functions.get(func_name) {
                    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);

                    let mut args = Vec::new();
                    for arg in &call.args {
                        args.push(compile_expression(ctx, builder, arg)?);
                    }

                    let call_inst = builder.ins().call(func_ref, &args);
                    let results = builder.inst_results(call_inst);

                    if results.is_empty() {
                        Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
                    } else {
                        Ok(results[0])
                    }
                }
                // Check for extern function
                else if let Some(extern_fn) = ctx.extern_fns.get(func_name).cloned() {
                    compile_extern_call(ctx, builder, &extern_fn, &call.args)
                }
                // Check for closure (lambda) variable
                else if let Some(&var) = ctx.variables.get(func_name) {
                    // This is a closure call - load the closure struct
                    let closure_ptr = builder.use_var(var);

                    // Load function pointer from offset 0
                    let func_ptr = builder.ins().load(
                        cranelift::prelude::types::I64,
                        MemFlags::new(),
                        closure_ptr,
                        0,
                    );

                    // Load data pointer from offset 8
                    let data_ptr = builder.ins().load(
                        cranelift::prelude::types::I64,
                        MemFlags::new(),
                        closure_ptr,
                        8,
                    );

                    // Build signature for indirect call: (closure_data_ptr, ...args) -> i64
                    let mut sig = ctx.module.make_signature();
                    sig.params.push(AbiParam::new(cranelift::prelude::types::I64)); // closure data
                    for _ in &call.args {
                        sig.params.push(AbiParam::new(cranelift::prelude::types::I64));
                    }
                    sig.returns.push(AbiParam::new(cranelift::prelude::types::I64));

                    let sig_ref = builder.import_signature(sig);

                    // Build arguments: first is data_ptr, then actual args
                    let mut args = vec![data_ptr];
                    for arg in &call.args {
                        args.push(compile_expression(ctx, builder, arg)?);
                    }

                    // Indirect call through function pointer
                    let call_inst = builder.ins().call_indirect(sig_ref, func_ptr, &args);
                    let results = builder.inst_results(call_inst);

                    if results.is_empty() {
                        Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
                    } else {
                        Ok(results[0])
                    }
                }
                // Check for exception constructor: ExceptionType("message")
                else if ctx.struct_defs.contains_key(func_name) {
                    // Exception constructor - allocate on heap (exceptions outlive stack frames)
                    let struct_def = ctx.struct_defs.get(func_name).unwrap();
                    let num_fields = struct_def.fields.len();
                    // Exception has implicit message field + defined fields
                    // Total size: 8 bytes for message pointer + 8 bytes per field
                    let size = 8 + (num_fields * 8);

                    // Allocate on heap since exceptions can escape the current stack frame
                    let size_val = builder.ins().iconst(cranelift::prelude::types::I64, size as i64);
                    let exception_ptr = call_alloc_closure_data(ctx, builder, size_val)?;

                    // Store message string at offset 0
                    if !call.args.is_empty() {
                        let mut message = compile_expression(ctx, builder, &call.args[0])?;
                        // Convert string literal to NamlString
                        if matches!(&call.args[0], Expression::Literal(LiteralExpr { value: Literal::String(_), .. })) {
                            message = call_string_from_cstr(ctx, builder, message)?;
                        }
                        builder.ins().store(MemFlags::new(), message, exception_ptr, 0);
                    }

                    Ok(exception_ptr)
                }
                else {
                    Err(CodegenError::JitCompile(format!("Unknown function: {}", func_name)))
                }
            }
            // Check for enum variant constructor: EnumType::Variant(data)
            else if let Expression::Path(path_expr) = call.callee {
                if path_expr.segments.len() == 2 {
                    let enum_name = ctx.interner.resolve(&path_expr.segments[0].symbol).to_string();
                    let variant_name = ctx.interner.resolve(&path_expr.segments[1].symbol).to_string();

                    if let Some(enum_def) = ctx.enum_defs.get(&enum_name) {
                        if let Some(variant) = enum_def.variants.iter().find(|v| v.name == variant_name) {
                            // Allocate stack slot for enum
                            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                                StackSlotKind::ExplicitSlot,
                                enum_def.size as u32,
                                0,
                            ));
                            let slot_addr = builder.ins().stack_addr(cranelift::prelude::types::I64, slot, 0);

                            // Store tag
                            let tag_val = builder.ins().iconst(cranelift::prelude::types::I32, variant.tag as i64);
                            builder.ins().store(MemFlags::new(), tag_val, slot_addr, 0);

                            // Store data fields
                            for (i, arg) in call.args.iter().enumerate() {
                                let mut arg_val = compile_expression(ctx, builder, arg)?;
                                // Check if argument is a string type - if so, convert C string to NamlString
                                if let Some(Type::String) = ctx.annotations.get_type(arg.span()) {
                                    // For string literals, convert to NamlString
                                    if matches!(arg, Expression::Literal(LiteralExpr { value: Literal::String(_), .. })) {
                                        arg_val = call_string_from_cstr(ctx, builder, arg_val)?;
                                    }
                                }
                                let offset = (variant.data_offset + i * 8) as i32;
                                builder.ins().store(MemFlags::new(), arg_val, slot_addr, offset);
                            }

                            return Ok(slot_addr);
                        }
                    }
                }

                Err(CodegenError::Unsupported(format!(
                    "Unknown enum variant call: {:?}",
                    path_expr.segments.iter()
                        .map(|s| ctx.interner.resolve(&s.symbol))
                        .collect::<Vec<_>>()
                )))
            }
            else {
                Err(CodegenError::Unsupported("Indirect function calls not yet supported".to_string()))
            }
        }

        Expression::Grouped(grouped) => {
            compile_expression(ctx, builder, &grouped.inner)
        }

        Expression::Block(block) => {
            // Compile all statements in the block
            for stmt in &block.statements {
                compile_statement(ctx, builder, stmt)?;
                if ctx.block_terminated {
                    // Block already terminated (e.g., return statement)
                    // The block already has a terminator - create an unreachable block for any remaining code
                    let unreachable_block = builder.create_block();
                    builder.switch_to_block(unreachable_block);
                    builder.seal_block(unreachable_block);
                    // Create a dummy value FIRST (before the trap)
                    let dummy = builder.ins().iconst(cranelift::prelude::types::I64, 0);
                    // Then terminate with trap (using unwrap_user trap code for unreachable)
                    builder.ins().trap(cranelift::prelude::TrapCode::unwrap_user(1));
                    return Ok(dummy);
                }
            }
            // If there's a tail expression, compile and return it
            if let Some(tail) = &block.tail {
                compile_expression(ctx, builder, tail)
            } else {
                // Return unit/0 for blocks with no tail expression
                Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
            }
        }

        Expression::Array(arr_expr) => {
            compile_array_literal(ctx, builder, &arr_expr.elements)
        }

        Expression::Map(map_expr) => {
            compile_map_literal(ctx, builder, &map_expr.entries)
        }

        Expression::Index(index_expr) => {
            let base = compile_expression(ctx, builder, &index_expr.base)?;

            // Check if index is a string literal - if so, use map_get with NamlString conversion
            if let Expression::Literal(LiteralExpr { value: Literal::String(_), .. }) = &*index_expr.index {
                let cstr_ptr = compile_expression(ctx, builder, &index_expr.index)?;
                let naml_str = call_string_from_cstr(ctx, builder, cstr_ptr)?;
                call_map_get(ctx, builder, base, naml_str)
            } else {
                // Default to array access for integer indices
                let index = compile_expression(ctx, builder, &index_expr.index)?;
                call_array_get(ctx, builder, base, index)
            }
        }

        Expression::MethodCall(method_call) => {
            let method_name = ctx.interner.resolve(&method_call.method.symbol);
            compile_method_call(ctx, builder, &method_call.receiver, method_name, &method_call.args)
        }

        Expression::StructLiteral(struct_lit) => {
            let struct_name = ctx.interner.resolve(&struct_lit.name.symbol).to_string();

            let struct_def = ctx.struct_defs.get(&struct_name)
                .ok_or_else(|| CodegenError::JitCompile(format!("Unknown struct: {}", struct_name)))?
                .clone();

            let type_id = builder.ins().iconst(cranelift::prelude::types::I32, struct_def.type_id as i64);
            let field_count = builder.ins().iconst(cranelift::prelude::types::I32, struct_def.fields.len() as i64);

            // Call naml_struct_new(type_id, field_count)
            let struct_ptr = call_struct_new(ctx, builder, type_id, field_count)?;

            // Set each field value
            for field in struct_lit.fields.iter() {
                let field_name = ctx.interner.resolve(&field.name.symbol).to_string();
                // Find field index in struct definition
                let field_idx = struct_def.fields.iter()
                    .position(|f| *f == field_name)
                    .ok_or_else(|| CodegenError::JitCompile(format!("Unknown field: {}", field_name)))?;

                let mut value = compile_expression(ctx, builder, &field.value)?;
                // Convert string literals to NamlString
                if let Some(Type::String) = ctx.annotations.get_type(field.value.span()) {
                    if matches!(&field.value, Expression::Literal(LiteralExpr { value: Literal::String(_), .. })) {
                        value = call_string_from_cstr(ctx, builder, value)?;
                    }
                }
                let idx_val = builder.ins().iconst(cranelift::prelude::types::I32, field_idx as i64);
                call_struct_set_field(ctx, builder, struct_ptr, idx_val, value)?;
            }

            Ok(struct_ptr)
        }

        Expression::Field(field_expr) => {
            let struct_ptr = compile_expression(ctx, builder, &field_expr.base)?;
            let field_name = ctx.interner.resolve(&field_expr.field.symbol).to_string();

            // Use type annotation to determine correct field offset
            // Note: use ident.span (IdentExpr span), not ident.ident.span (Ident span)
            if let Expression::Identifier(ident) = &*field_expr.base {
                if let Some(type_ann) = ctx.annotations.get_type(ident.span) {
                    if let crate::typechecker::Type::Exception(exc_name) = type_ann {
                        let exc_name_str = ctx.interner.resolve(&exc_name).to_string();
                        if let Some(struct_def) = ctx.struct_defs.get(&exc_name_str) {
                            // Exception: message at offset 0, fields at 8, 16, ...
                            let offset = if field_name == "message" {
                                0
                            } else if let Some(idx) = struct_def.fields.iter().position(|f| f == &field_name) {
                                8 + (idx * 8) as i32
                            } else {
                                return Err(CodegenError::JitCompile(format!("Unknown field: {}", field_name)));
                            };
                            let value = builder.ins().load(
                                cranelift::prelude::types::I64,
                                MemFlags::new(),
                                struct_ptr,
                                offset,
                            );
                            return Ok(value);
                        }
                    } else if let crate::typechecker::Type::Struct(struct_type) = type_ann {
                        let struct_name = ctx.interner.resolve(&struct_type.name).to_string();
                        if let Some(struct_def) = ctx.struct_defs.get(&struct_name) {
                            if let Some(idx) = struct_def.fields.iter().position(|f| f == &field_name) {
                                let offset = (idx * 8) as i32;
                                let value = builder.ins().load(
                                    cranelift::prelude::types::I64,
                                    MemFlags::new(),
                                    struct_ptr,
                                    offset,
                                );
                                return Ok(value);
                            }
                        }
                    }
                }
            }

            // Fallback to runtime lookup for generic cases
            for (_, struct_def) in ctx.struct_defs.iter() {
                if let Some(field_idx) = struct_def.fields.iter().position(|f| *f == field_name) {
                    let idx_val = builder.ins().iconst(cranelift::prelude::types::I32, field_idx as i64);
                    return call_struct_get_field(ctx, builder, struct_ptr, idx_val);
                }
            }

            Err(CodegenError::JitCompile(format!("Unknown field: {}", field_name)))
        }

        Expression::Spawn(_spawn_expr) => {
            // True M:N spawn: schedule the spawn block on the thread pool
            let spawn_id = ctx.current_spawn_id;
            ctx.current_spawn_id += 1;

            let info = ctx.spawn_blocks.get(&spawn_id)
                .ok_or_else(|| CodegenError::JitCompile(format!("Spawn block {} not found", spawn_id)))?
                .clone();

            let ptr_type = ctx.module.target_config().pointer_type();

            // Calculate closure data size (8 bytes per captured variable)
            let data_size = info.captured_vars.len() * 8;
            let data_size_val = builder.ins().iconst(cranelift::prelude::types::I64, data_size as i64);

            // Allocate closure data
            let data_ptr = if data_size > 0 {
                call_alloc_closure_data(ctx, builder, data_size_val)?
            } else {
                builder.ins().iconst(ptr_type, 0)
            };

            // Store captured variables in closure data
            for (i, var_name) in info.captured_vars.iter().enumerate() {
                if let Some(&var) = ctx.variables.get(var_name) {
                    let val = builder.use_var(var);
                    let offset = builder.ins().iconst(ptr_type, (i * 8) as i64);
                    let addr = builder.ins().iadd(data_ptr, offset);
                    builder.ins().store(MemFlags::new(), val, addr, 0);
                }
            }

            // Get trampoline function address
            let trampoline_id = *ctx.functions.get(&info.func_name)
                .ok_or_else(|| CodegenError::JitCompile(format!("Trampoline '{}' not found", info.func_name)))?;
            let trampoline_ref = ctx.module.declare_func_in_func(trampoline_id, builder.func);
            let trampoline_addr = builder.ins().func_addr(ptr_type, trampoline_ref);

            // Call spawn_closure to schedule the task
            call_spawn_closure(ctx, builder, trampoline_addr, data_ptr, data_size_val)?;

            // Return unit (0) as spawn expressions don't have a meaningful return value
            Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
        }

        Expression::Some(some_expr) => {
            let inner_val = compile_expression(ctx, builder, &some_expr.value)?;

            // Allocate option on stack
            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                16, // option size
                0,
            ));
            let slot_addr = builder.ins().stack_addr(cranelift::prelude::types::I64, slot, 0);

            // Tag = 1 (some)
            let tag = builder.ins().iconst(cranelift::prelude::types::I32, 1);
            builder.ins().store(MemFlags::new(), tag, slot_addr, 0);

            // Store inner value at offset 8
            builder.ins().store(MemFlags::new(), inner_val, slot_addr, 8);

            Ok(slot_addr)
        }

        Expression::Lambda(_lambda_expr) => {
            // Get lambda info from the tracked lambdas
            let lambda_id = ctx.current_lambda_id;
            ctx.current_lambda_id += 1;

            let info = ctx.lambda_blocks.get(&lambda_id)
                .ok_or_else(|| CodegenError::JitCompile(format!("Lambda {} not found", lambda_id)))?
                .clone();

            let ptr_type = ctx.module.target_config().pointer_type();

            // Calculate closure data size (8 bytes per captured variable)
            let data_size = info.captured_vars.len() * 8;
            let data_size_val = builder.ins().iconst(cranelift::prelude::types::I64, data_size as i64);

            // Allocate closure data
            let data_ptr = if data_size > 0 {
                call_alloc_closure_data(ctx, builder, data_size_val)?
            } else {
                builder.ins().iconst(ptr_type, 0)
            };

            // Store captured variables in closure data (by value)
            for (i, var_name) in info.captured_vars.iter().enumerate() {
                if let Some(&var) = ctx.variables.get(var_name) {
                    let val = builder.use_var(var);
                    let offset = builder.ins().iconst(ptr_type, (i * 8) as i64);
                    let addr = builder.ins().iadd(data_ptr, offset);
                    builder.ins().store(MemFlags::new(), val, addr, 0);
                }
            }

            // Get function pointer
            let lambda_func_id = ctx.functions.get(&info.func_name)
                .ok_or_else(|| CodegenError::JitCompile(format!("Lambda function '{}' not found", info.func_name)))?;
            let func_ref = ctx.module.declare_func_in_func(*lambda_func_id, builder.func);
            let func_addr = builder.ins().func_addr(ptr_type, func_ref);

            // Allocate closure struct on stack: 24 bytes (func_ptr, data_ptr, data_size)
            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                24,
                0,
            ));
            let slot_addr = builder.ins().stack_addr(ptr_type, slot, 0);

            // Store function pointer at offset 0
            builder.ins().store(MemFlags::new(), func_addr, slot_addr, 0);

            // Store data pointer at offset 8
            builder.ins().store(MemFlags::new(), data_ptr, slot_addr, 8);

            // Store data size at offset 16
            builder.ins().store(MemFlags::new(), data_size_val, slot_addr, 16);

            Ok(slot_addr)
        }

        Expression::Try(try_expr) => {
            // For now, try just evaluates the expression
            // Full exception unwinding will be implemented later
            compile_expression(ctx, builder, &try_expr.expr)
        }

        Expression::Catch(catch_expr) => {
            // Compile the expression that might throw
            let result = compile_expression(ctx, builder, &catch_expr.expr)?;

            // Check if an exception occurred
            let has_exception = call_exception_check(ctx, builder)?;

            // Create blocks for branching
            let exception_block = builder.create_block();
            let no_exception_block = builder.create_block();
            let merge_block = builder.create_block();
            builder.append_block_param(merge_block, cranelift::prelude::types::I64);

            // Branch based on exception check
            let zero = builder.ins().iconst(cranelift::prelude::types::I64, 0);
            let has_ex = builder.ins().icmp(IntCC::NotEqual, has_exception, zero);
            builder.ins().brif(has_ex, exception_block, &[], no_exception_block, &[]);

            // Exception block: get exception, bind to variable, run handler
            builder.switch_to_block(exception_block);
            builder.seal_block(exception_block);

            // Get the exception pointer and bind to the error variable
            let exception_ptr = call_exception_get(ctx, builder)?;
            let error_var_name = ctx.interner.resolve(&catch_expr.error_binding.symbol).to_string();

            // Check if variable already exists (for multiple catch blocks with same binding name)
            let error_var = if let Some(&existing_var) = ctx.variables.get(&error_var_name) {
                existing_var
            } else {
                let new_var = Variable::new(ctx.var_counter);
                ctx.var_counter += 1;
                ctx.variables.insert(error_var_name, new_var);
                builder.declare_var(new_var, cranelift::prelude::types::I64);
                new_var
            };
            builder.def_var(error_var, exception_ptr);

            // Clear the exception so it doesn't propagate
            call_exception_clear(ctx, builder)?;

            // Compile the handler block
            for stmt in &catch_expr.handler.statements {
                compile_statement(ctx, builder, stmt)?;
                if ctx.block_terminated {
                    break;
                }
            }

            // If handler didn't return/throw, jump to merge with 0 (none)
            if !ctx.block_terminated {
                builder.ins().jump(merge_block, &[zero]);
            }
            ctx.block_terminated = false;

            // No exception block: jump to merge with the result
            builder.switch_to_block(no_exception_block);
            builder.seal_block(no_exception_block);
            builder.ins().jump(merge_block, &[result]);

            // Merge block
            builder.switch_to_block(merge_block);
            builder.seal_block(merge_block);
            let final_result = builder.block_params(merge_block)[0];
            Ok(final_result)
        }

        Expression::OrDefault(or_default_expr) => {
            // Compile the option expression (returns pointer to option struct)
            let option_ptr = compile_expression(ctx, builder, &or_default_expr.expr)?;

            // Load the tag from offset 0 (0 = none, 1 = some)
            let tag = builder.ins().load(
                cranelift::prelude::types::I32,
                MemFlags::new(),
                option_ptr,
                0,
            );

            // Create blocks for the conditional
            let some_block = builder.create_block();
            let none_block = builder.create_block();
            let merge_block = builder.create_block();

            // Add a block parameter for the result
            builder.append_block_param(merge_block, cranelift::prelude::types::I64);

            // Branch based on tag (tag != 0 means some)
            let zero = builder.ins().iconst(cranelift::prelude::types::I32, 0);
            let is_some = builder.ins().icmp(IntCC::NotEqual, tag, zero);
            builder.ins().brif(is_some, some_block, &[], none_block, &[]);

            // Some block: load and return the inner value
            builder.switch_to_block(some_block);
            builder.seal_block(some_block);
            let inner_val = builder.ins().load(
                cranelift::prelude::types::I64,
                MemFlags::new(),
                option_ptr,
                8,
            );
            builder.ins().jump(merge_block, &[inner_val]);

            // None block: compile and return the default value
            builder.switch_to_block(none_block);
            builder.seal_block(none_block);
            let default_val = compile_expression(ctx, builder, &or_default_expr.default)?;
            builder.ins().jump(merge_block, &[default_val]);

            // Merge block: return the result
            builder.switch_to_block(merge_block);
            builder.seal_block(merge_block);
            let result = builder.block_params(merge_block)[0];
            Ok(result)
        }

        Expression::Cast(cast_expr) => {
            // Evaluate the expression to cast
            let value = compile_expression(ctx, builder, &cast_expr.expr)?;

            // Get source and target types
            let source_type = ctx.annotations.get_type(cast_expr.expr.span());

            match &cast_expr.target_ty {
                NamlType::Int => {
                    match source_type {
                        Some(Type::Float) => {
                            Ok(builder.ins().fcvt_to_sint(cranelift::prelude::types::I64, value))
                        }
                        Some(Type::String) => {
                            call_string_to_int(ctx, builder, value)
                        }
                        Some(Type::Uint) | Some(Type::Int) => Ok(value),
                        _ => Ok(value)
                    }
                }
                NamlType::Uint => {
                    match source_type {
                        Some(Type::Float) => {
                            Ok(builder.ins().fcvt_to_uint(cranelift::prelude::types::I64, value))
                        }
                        Some(Type::Int) | Some(Type::Uint) => Ok(value),
                        _ => Ok(value)
                    }
                }
                NamlType::Float => {
                    match source_type {
                        Some(Type::Int) => {
                            Ok(builder.ins().fcvt_from_sint(cranelift::prelude::types::F64, value))
                        }
                        Some(Type::Uint) => {
                            Ok(builder.ins().fcvt_from_uint(cranelift::prelude::types::F64, value))
                        }
                        Some(Type::String) => {
                            call_string_to_float(ctx, builder, value)
                        }
                        Some(Type::Float) => Ok(value),
                        _ => Ok(value)
                    }
                }
                NamlType::String => {
                    match source_type {
                        Some(Type::Int) | Some(Type::Uint) => {
                            call_int_to_string(ctx, builder, value)
                        }
                        Some(Type::Float) => {
                            call_float_to_string(ctx, builder, value)
                        }
                        Some(Type::Bytes) => {
                            call_bytes_to_string(ctx, builder, value)
                        }
                        Some(Type::String) => Ok(value),
                        _ => Ok(value)
                    }
                }
                NamlType::Bytes => {
                    match source_type {
                        Some(Type::String) => {
                            call_string_to_bytes(ctx, builder, value)
                        }
                        Some(Type::Bytes) => Ok(value),
                        _ => Ok(value)
                    }
                }
                _ => {
                    // For other casts, just pass through the value
                    Ok(value)
                }
            }
        }

        _ => {
            Err(CodegenError::Unsupported(
                format!("Expression type not yet implemented: {:?}", std::mem::discriminant(expr))
            ))
        }
    }
}

fn compile_literal(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    lit: &Literal,
) -> Result<Value, CodegenError> {
    match lit {
        Literal::Int(n) => {
            Ok(builder.ins().iconst(cranelift::prelude::types::I64, *n))
        }
        Literal::UInt(n) => {
            Ok(builder.ins().iconst(cranelift::prelude::types::I64, *n as i64))
        }
        Literal::Float(f) => {
            Ok(builder.ins().f64const(*f))
        }
        Literal::Bool(b) => {
            let val = if *b { 1i64 } else { 0i64 };
            Ok(builder.ins().iconst(cranelift::prelude::types::I64, val))
        }
        Literal::String(spur) => {
            let s = ctx.interner.resolve(spur);
            compile_string_literal(ctx, builder, s)
        }
        Literal::None => {
            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                16,
                0,
            ));
            let slot_addr = builder.ins().stack_addr(cranelift::prelude::types::I64, slot, 0);

            let tag = builder.ins().iconst(cranelift::prelude::types::I32, 0);
            builder.ins().store(MemFlags::new(), tag, slot_addr, 0);

            Ok(slot_addr)
        }
        _ => {
            Err(CodegenError::Unsupported(
                format!("Literal type not yet implemented: {:?}", std::mem::discriminant(lit))
            ))
        }
    }
}

fn compile_string_literal(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    s: &str,
) -> Result<Value, CodegenError> {
    let mut bytes = s.as_bytes().to_vec();
    bytes.push(0);

    let data_id = ctx.module
        .declare_anonymous_data(false, false)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare string data: {}", e)))?;

    let mut data_description = DataDescription::new();
    data_description.define(bytes.into_boxed_slice());

    ctx.module
        .define_data(data_id, &data_description)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to define string data: {}", e)))?;

    let global_value = ctx.module.declare_data_in_func(data_id, builder.func);
    let ptr = builder.ins().global_value(ctx.module.target_config().pointer_type(), global_value);

    Ok(ptr)
}

fn compile_binary_op(
    builder: &mut FunctionBuilder<'_>,
    op: &BinaryOp,
    lhs: Value,
    rhs: Value,
) -> Result<Value, CodegenError> {
    let result = match op {
        BinaryOp::Add => builder.ins().iadd(lhs, rhs),
        BinaryOp::Sub => builder.ins().isub(lhs, rhs),
        BinaryOp::Mul => builder.ins().imul(lhs, rhs),
        BinaryOp::Div => builder.ins().sdiv(lhs, rhs),
        BinaryOp::Mod => builder.ins().srem(lhs, rhs),

        BinaryOp::Eq => {
            let cmp = builder.ins().icmp(IntCC::Equal, lhs, rhs);
            builder.ins().uextend(cranelift::prelude::types::I64, cmp)
        }
        BinaryOp::NotEq => {
            let cmp = builder.ins().icmp(IntCC::NotEqual, lhs, rhs);
            builder.ins().uextend(cranelift::prelude::types::I64, cmp)
        }
        BinaryOp::Lt => {
            let cmp = builder.ins().icmp(IntCC::SignedLessThan, lhs, rhs);
            builder.ins().uextend(cranelift::prelude::types::I64, cmp)
        }
        BinaryOp::LtEq => {
            let cmp = builder.ins().icmp(IntCC::SignedLessThanOrEqual, lhs, rhs);
            builder.ins().uextend(cranelift::prelude::types::I64, cmp)
        }
        BinaryOp::Gt => {
            let cmp = builder.ins().icmp(IntCC::SignedGreaterThan, lhs, rhs);
            builder.ins().uextend(cranelift::prelude::types::I64, cmp)
        }
        BinaryOp::GtEq => {
            let cmp = builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, lhs, rhs);
            builder.ins().uextend(cranelift::prelude::types::I64, cmp)
        }

        BinaryOp::And => builder.ins().band(lhs, rhs),
        BinaryOp::Or => builder.ins().bor(lhs, rhs),

        BinaryOp::BitAnd => builder.ins().band(lhs, rhs),
        BinaryOp::BitOr => builder.ins().bor(lhs, rhs),
        BinaryOp::BitXor => builder.ins().bxor(lhs, rhs),
        BinaryOp::Shl => builder.ins().ishl(lhs, rhs),
        BinaryOp::Shr => builder.ins().sshr(lhs, rhs),

        _ => {
            return Err(CodegenError::Unsupported(
                format!("Binary operator not yet implemented: {:?}", op)
            ));
        }
    };

    Ok(result)
}

fn compile_unary_op(
    builder: &mut FunctionBuilder<'_>,
    op: &UnaryOp,
    operand: Value,
) -> Result<Value, CodegenError> {
    let result = match op {
        UnaryOp::Neg => builder.ins().ineg(operand),
        UnaryOp::Not => {
            let one = builder.ins().iconst(cranelift::prelude::types::I64, 1);
            builder.ins().bxor(operand, one)
        }
        UnaryOp::BitNot => builder.ins().bnot(operand),
    };

    Ok(result)
}

fn compile_print_call(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    args: &[Expression<'_>],
    newline: bool,
) -> Result<Value, CodegenError> {
    if args.is_empty() {
        if newline {
            call_print_newline(ctx, builder)?;
        }
        return Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0));
    }

    // Check if first arg is a format string with {}
    if let Expression::Literal(LiteralExpr { value: Literal::String(spur), .. }) = &args[0] {
        let format_str = ctx.interner.resolve(spur);
        if format_str.contains("{}") {
            // Format string mode
            let mut arg_idx = 1;
            let mut last_end = 0;

            for (start, _) in format_str.match_indices("{}") {
                // Print literal part before placeholder
                if start > last_end {
                    let literal_part = &format_str[last_end..start];
                    let ptr = compile_string_literal(ctx, builder, literal_part)?;
                    call_print_str(ctx, builder, ptr)?;
                }

                // Print the argument
                if arg_idx < args.len() {
                    let arg = &args[arg_idx];
                    print_arg(ctx, builder, arg)?;
                    arg_idx += 1;
                }

                last_end = start + 2;
            }

            // Print remaining literal after last placeholder
            if last_end < format_str.len() {
                let remaining = &format_str[last_end..];
                let ptr = compile_string_literal(ctx, builder, remaining)?;
                call_print_str(ctx, builder, ptr)?;
            }

            if newline {
                call_print_newline(ctx, builder)?;
            }

            return Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0));
        }
    }

    // Original behavior for non-format strings
    for (i, arg) in args.iter().enumerate() {
        print_arg(ctx, builder, arg)?;

        if i < args.len() - 1 {
            let space = compile_string_literal(ctx, builder, " ")?;
            call_print_str(ctx, builder, space)?;
        }
    }

    if newline {
        call_print_newline(ctx, builder)?;
    }

    Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
}

fn print_arg(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arg: &Expression<'_>,
) -> Result<(), CodegenError> {
    match arg {
        Expression::Literal(LiteralExpr { value: Literal::String(spur), .. }) => {
            let s = ctx.interner.resolve(spur);
            let ptr = compile_string_literal(ctx, builder, s)?;
            call_print_str(ctx, builder, ptr)?;
        }
        Expression::Literal(LiteralExpr { value: Literal::Int(n), .. }) => {
            let val = builder.ins().iconst(cranelift::prelude::types::I64, *n);
            call_print_int(ctx, builder, val)?;
        }
        Expression::Literal(LiteralExpr { value: Literal::Float(f), .. }) => {
            let val = builder.ins().f64const(*f);
            call_print_float(ctx, builder, val)?;
        }
        _ => {
            let val = compile_expression(ctx, builder, arg)?;
            // Check type from annotations to call appropriate print function
            let expr_type = ctx.annotations.get_type(arg.span());
            match expr_type {
                Some(Type::String) => {
                    // String variables now hold NamlString* (boxed strings)
                    call_print_naml_string(ctx, builder, val)?;
                }
                Some(Type::Float) => {
                    call_print_float(ctx, builder, val)?;
                }
                _ => {
                    // Default: check Cranelift value type for F64, otherwise int
                    let val_type = builder.func.dfg.value_type(val);
                    if val_type == cranelift::prelude::types::F64 {
                        call_print_float(ctx, builder, val)?;
                    } else {
                        call_print_int(ctx, builder, val)?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn emit_incref(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    val: Value,
    heap_type: &HeapType,
) -> Result<(), CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();
    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ptr_type));

    let func_name = match heap_type {
        HeapType::String => "naml_string_incref",
        HeapType::Array(_) => "naml_array_incref",
        HeapType::Map(_) => "naml_map_incref",
        HeapType::Struct(_) => "naml_struct_incref",
    };

    let func_id = ctx.module
        .declare_function(func_name, Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare {}: {}", func_name, e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let zero = builder.ins().iconst(ptr_type, 0);
    let is_null = builder.ins().icmp(IntCC::Equal, val, zero);

    let call_block = builder.create_block();
    let merge_block = builder.create_block();

    builder.ins().brif(is_null, merge_block, &[], call_block, &[]);

    builder.switch_to_block(call_block);
    builder.seal_block(call_block);
    builder.ins().call(func_ref, &[val]);
    builder.ins().jump(merge_block, &[]);

    builder.switch_to_block(merge_block);
    builder.seal_block(merge_block);

    Ok(())
}

fn struct_has_heap_fields(struct_defs: &HashMap<String, StructDef>, struct_name: &str) -> bool {
    if let Some(def) = struct_defs.get(struct_name) {
        def.field_heap_types.iter().any(|ht| ht.is_some())
    } else {
        false
    }
}

fn emit_decref(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    val: Value,
    heap_type: &HeapType,
) -> Result<(), CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();
    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ptr_type));

    // Select the appropriate decref function based on element type for nested cleanup
    let func_name: String = match heap_type {
        HeapType::String => "naml_string_decref".to_string(),
        HeapType::Array(None) => "naml_array_decref".to_string(),
        HeapType::Array(Some(elem_type)) => {
            // Use specialized decref that also decrefs elements
            match elem_type.as_ref() {
                HeapType::String => "naml_array_decref_strings".to_string(),
                HeapType::Array(_) => "naml_array_decref_arrays".to_string(),
                HeapType::Map(_) => "naml_array_decref_maps".to_string(),
                HeapType::Struct(_) => "naml_array_decref_structs".to_string(),
            }
        }
        HeapType::Map(None) => "naml_map_decref".to_string(),
        HeapType::Map(Some(val_type)) => {
            // Use specialized decref that also decrefs values
            match val_type.as_ref() {
                HeapType::String => "naml_map_decref_strings".to_string(),
                HeapType::Array(_) => "naml_map_decref_arrays".to_string(),
                HeapType::Map(_) => "naml_map_decref_maps".to_string(),
                HeapType::Struct(_) => "naml_map_decref_structs".to_string(),
            }
        }
        HeapType::Struct(None) => "naml_struct_decref".to_string(),
        HeapType::Struct(Some(struct_name)) => {
            // Check if this struct has heap fields that need cleanup
            if struct_has_heap_fields(ctx.struct_defs, struct_name) {
                format!("naml_struct_decref_{}", struct_name)
            } else {
                "naml_struct_decref".to_string()
            }
        }
    };

    let func_id = ctx.module
        .declare_function(&func_name, Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare {}: {}", func_name, e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let zero = builder.ins().iconst(ptr_type, 0);
    let is_null = builder.ins().icmp(IntCC::Equal, val, zero);

    let call_block = builder.create_block();
    let merge_block = builder.create_block();

    builder.ins().brif(is_null, merge_block, &[], call_block, &[]);

    builder.switch_to_block(call_block);
    builder.seal_block(call_block);
    builder.ins().call(func_ref, &[val]);
    builder.ins().jump(merge_block, &[]);

    builder.switch_to_block(merge_block);
    builder.seal_block(merge_block);

    Ok(())
}

fn emit_cleanup_all_vars(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    exclude_var: Option<&str>,
) -> Result<(), CodegenError> {
    let vars_to_cleanup: Vec<(String, Variable, HeapType)> = ctx.var_heap_types
        .iter()
        .filter_map(|(name, heap_type)| {
            if let Some(excl) = exclude_var {
                if name == excl {
                    return None;
                }
            }
            ctx.variables.get(name).map(|var| (name.clone(), *var, heap_type.clone()))
        })
        .collect();

    for (_, var, ref heap_type) in vars_to_cleanup {
        let val = builder.use_var(var);
        emit_decref(ctx, builder, val, heap_type)?;
    }

    Ok(())
}

fn get_returned_var_name(expr: &Expression, interner: &Rodeo) -> Option<String> {
    match expr {
        Expression::Identifier(ident) => {
            Some(interner.resolve(&ident.ident.symbol).to_string())
        }
        _ => None,
    }
}

fn call_print_int(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    val: Value,
) -> Result<(), CodegenError> {
    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(cranelift::prelude::types::I64));

    let func_id = ctx.module
        .declare_function("naml_print_int", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_print_int: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    builder.ins().call(func_ref, &[val]);
    Ok(())
}

fn call_print_float(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    val: Value,
) -> Result<(), CodegenError> {
    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(cranelift::prelude::types::F64));

    let func_id = ctx.module
        .declare_function("naml_print_float", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_print_float: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    builder.ins().call(func_ref, &[val]);
    Ok(())
}

fn call_print_str(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    ptr: Value,
) -> Result<(), CodegenError> {
    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ctx.module.target_config().pointer_type()));

    let func_id = ctx.module
        .declare_function("naml_print_str", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_print_str: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    builder.ins().call(func_ref, &[ptr]);
    Ok(())
}

fn call_print_naml_string(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    ptr: Value,
) -> Result<(), CodegenError> {
    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ctx.module.target_config().pointer_type()));

    let func_id = ctx.module
        .declare_function("naml_string_print", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_string_print: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    builder.ins().call(func_ref, &[ptr]);
    Ok(())
}

fn call_string_equals(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    a: Value,
    b: Value,
) -> Result<Value, CodegenError> {
    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ctx.module.target_config().pointer_type()));
    sig.params.push(AbiParam::new(ctx.module.target_config().pointer_type()));
    sig.returns.push(AbiParam::new(cranelift::prelude::types::I64));

    let func_id = ctx.module
        .declare_function("naml_string_eq", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_string_eq: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, &[a, b]);
    Ok(builder.inst_results(call)[0])
}

fn call_int_to_string(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    value: Value,
) -> Result<Value, CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();
    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(cranelift::prelude::types::I64));
    sig.returns.push(AbiParam::new(ptr_type));

    let func_id = ctx.module
        .declare_function("naml_int_to_string", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_int_to_string: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, &[value]);
    Ok(builder.inst_results(call)[0])
}

fn call_float_to_string(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    value: Value,
) -> Result<Value, CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();
    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(cranelift::prelude::types::F64));
    sig.returns.push(AbiParam::new(ptr_type));

    let func_id = ctx.module
        .declare_function("naml_float_to_string", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_float_to_string: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, &[value]);
    Ok(builder.inst_results(call)[0])
}

fn call_string_to_int(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    value: Value,
) -> Result<Value, CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();
    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(cranelift::prelude::types::I64));

    let func_id = ctx.module
        .declare_function("naml_string_to_int", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_string_to_int: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, &[value]);
    Ok(builder.inst_results(call)[0])
}

fn call_string_to_float(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    value: Value,
) -> Result<Value, CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();
    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(cranelift::prelude::types::F64));

    let func_id = ctx.module
        .declare_function("naml_string_to_float", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_string_to_float: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, &[value]);
    Ok(builder.inst_results(call)[0])
}

fn call_string_char_len(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    str_ptr: Value,
) -> Result<Value, CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();
    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(cranelift::prelude::types::I64));

    let func_id = ctx.module
        .declare_function("naml_string_char_len", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_string_char_len: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, &[str_ptr]);
    Ok(builder.inst_results(call)[0])
}

fn call_string_char_at(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    str_ptr: Value,
    index: Value,
) -> Result<Value, CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();
    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(cranelift::prelude::types::I64));
    sig.returns.push(AbiParam::new(cranelift::prelude::types::I64));

    let func_id = ctx.module
        .declare_function("naml_string_char_at", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_string_char_at: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, &[str_ptr, index]);
    Ok(builder.inst_results(call)[0])
}

fn call_string_to_bytes(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    str_ptr: Value,
) -> Result<Value, CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();
    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(ptr_type));

    let func_id = ctx.module
        .declare_function("naml_string_to_bytes", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_string_to_bytes: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, &[str_ptr]);
    Ok(builder.inst_results(call)[0])
}

fn call_bytes_to_string(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    bytes_ptr: Value,
) -> Result<Value, CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();
    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(ptr_type));

    let func_id = ctx.module
        .declare_function("naml_bytes_to_string", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_bytes_to_string: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, &[bytes_ptr]);
    Ok(builder.inst_results(call)[0])
}

fn call_print_newline(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), CodegenError> {
    let sig = ctx.module.make_signature();

    let func_id = ctx.module
        .declare_function("naml_print_newline", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_print_newline: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    builder.ins().call(func_ref, &[]);
    Ok(())
}

fn compile_array_literal(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    elements: &[Expression<'_>],
) -> Result<Value, CodegenError> {
    // First, compile all elements and store on stack
    let mut element_values = Vec::new();
    for elem in elements {
        element_values.push(compile_expression(ctx, builder, elem)?);
    }

    // Create array with capacity
    let capacity = builder.ins().iconst(cranelift::prelude::types::I64, elements.len() as i64);
    let arr_ptr = call_array_new(ctx, builder, capacity)?;

    // Push each element
    for val in element_values {
        call_array_push(ctx, builder, arr_ptr, val)?;
    }

    Ok(arr_ptr)
}

fn call_array_new(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    capacity: Value,
) -> Result<Value, CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();

    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(cranelift::prelude::types::I64));
    sig.returns.push(AbiParam::new(ptr_type));

    let func_id = ctx.module
        .declare_function("naml_array_new", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_array_new: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, &[capacity]);
    Ok(builder.inst_results(call)[0])
}

fn call_array_push(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
    value: Value,
) -> Result<(), CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();

    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(cranelift::prelude::types::I64));

    let func_id = ctx.module
        .declare_function("naml_array_push", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_array_push: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    builder.ins().call(func_ref, &[arr, value]);
    Ok(())
}

fn call_array_get(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
    index: Value,
) -> Result<Value, CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();

    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(cranelift::prelude::types::I64));
    sig.returns.push(AbiParam::new(cranelift::prelude::types::I64));

    let func_id = ctx.module
        .declare_function("naml_array_get", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_array_get: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, &[arr, index]);
    Ok(builder.inst_results(call)[0])
}

fn call_array_len(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
) -> Result<Value, CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();

    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(cranelift::prelude::types::I64));

    let func_id = ctx.module
        .declare_function("naml_array_len", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_array_len: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, &[arr]);
    Ok(builder.inst_results(call)[0])
}

#[allow(dead_code)]
fn call_array_print(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
) -> Result<(), CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();

    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ptr_type));

    let func_id = ctx.module
        .declare_function("naml_array_print", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_array_print: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    builder.ins().call(func_ref, &[arr]);
    Ok(())
}

fn call_array_set(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
    index: Value,
    value: Value,
) -> Result<(), CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();

    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(cranelift::prelude::types::I64));
    sig.params.push(AbiParam::new(cranelift::prelude::types::I64));

    let func_id = ctx.module
        .declare_function("naml_array_set", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_array_set: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    builder.ins().call(func_ref, &[arr, index, value]);
    Ok(())
}

fn compile_map_literal(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    entries: &[crate::ast::MapEntry<'_>],
) -> Result<Value, CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();

    // Create naml_map_new signature
    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(cranelift::prelude::types::I64)); // capacity
    sig.returns.push(AbiParam::new(ptr_type)); // map ptr

    let func_id = ctx.module
        .declare_function("naml_map_new", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(e.to_string()))?;
    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);

    // Create map with capacity 16
    let capacity = builder.ins().iconst(cranelift::prelude::types::I64, 16);
    let call = builder.ins().call(func_ref, &[capacity]);
    let map_ptr = builder.inst_results(call)[0];

    // For each entry, call naml_map_set
    if !entries.is_empty() {
        let mut set_sig = ctx.module.make_signature();
        set_sig.params.push(AbiParam::new(ptr_type)); // map
        set_sig.params.push(AbiParam::new(cranelift::prelude::types::I64)); // key
        set_sig.params.push(AbiParam::new(cranelift::prelude::types::I64)); // value

        let set_func_id = ctx.module
            .declare_function("naml_map_set", Linkage::Import, &set_sig)
            .map_err(|e| CodegenError::JitCompile(e.to_string()))?;
        let set_func_ref = ctx.module.declare_func_in_func(set_func_id, builder.func);

        for entry in entries {
            // Convert string literals to NamlString pointers for map keys
            let key = if let Expression::Literal(LiteralExpr { value: Literal::String(_), .. }) = &entry.key {
                let cstr_ptr = compile_expression(ctx, builder, &entry.key)?;
                call_string_from_cstr(ctx, builder, cstr_ptr)?
            } else {
                compile_expression(ctx, builder, &entry.key)?
            };
            let value = compile_expression(ctx, builder, &entry.value)?;
            builder.ins().call(set_func_ref, &[map_ptr, key, value]);
        }
    }

    Ok(map_ptr)
}

fn call_string_from_cstr(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    cstr_ptr: Value,
) -> Result<Value, CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();

    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ptr_type)); // cstr: *const i8
    sig.returns.push(AbiParam::new(ptr_type)); // *mut NamlString

    let func_id = ctx.module
        .declare_function("naml_string_from_cstr", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_string_from_cstr: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, &[cstr_ptr]);
    Ok(builder.inst_results(call)[0])
}

#[allow(dead_code)]
fn call_map_new(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    capacity: Value,
) -> Result<Value, CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();

    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(cranelift::prelude::types::I64));
    sig.returns.push(AbiParam::new(ptr_type));

    let func_id = ctx.module
        .declare_function("naml_map_new", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_map_new: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, &[capacity]);
    Ok(builder.inst_results(call)[0])
}

fn call_map_set(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    map: Value,
    key: Value,
    value: Value,
) -> Result<(), CodegenError> {
    call_map_set_typed(ctx, builder, map, key, value, None)
}

fn call_map_set_typed(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    map: Value,
    key: Value,
    value: Value,
    value_type: Option<&HeapType>,
) -> Result<(), CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();

    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(cranelift::prelude::types::I64));
    sig.params.push(AbiParam::new(cranelift::prelude::types::I64));

    // Select the appropriate set function based on value type
    // This ensures proper decref of old values when updating map entries
    let func_name = match value_type {
        Some(HeapType::String) => "naml_map_set_string",
        Some(HeapType::Array(_)) => "naml_map_set_array",
        Some(HeapType::Map(_)) => "naml_map_set_map",
        Some(HeapType::Struct(_)) => "naml_map_set_struct",
        None => "naml_map_set",
    };

    let func_id = ctx.module
        .declare_function(func_name, Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare {}: {}", func_name, e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    builder.ins().call(func_ref, &[map, key, value]);
    Ok(())
}

fn call_map_get(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    map: Value,
    key: Value,
) -> Result<Value, CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();

    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(cranelift::prelude::types::I64));
    sig.returns.push(AbiParam::new(cranelift::prelude::types::I64));

    let func_id = ctx.module
        .declare_function("naml_map_get", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_map_get: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, &[map, key]);
    Ok(builder.inst_results(call)[0])
}

fn call_map_contains(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    map: Value,
    key: Value,
) -> Result<Value, CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();

    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(cranelift::prelude::types::I64));
    sig.returns.push(AbiParam::new(cranelift::prelude::types::I64));

    let func_id = ctx.module
        .declare_function("naml_map_contains", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_map_contains: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, &[map, key]);
    Ok(builder.inst_results(call)[0])
}

#[allow(dead_code)]
fn call_map_len(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    map: Value,
) -> Result<Value, CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();

    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(cranelift::prelude::types::I64));

    let func_id = ctx.module
        .declare_function("naml_map_len", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_map_len: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, &[map]);
    Ok(builder.inst_results(call)[0])
}

fn call_struct_new(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    type_id: Value,
    field_count: Value,
) -> Result<Value, CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();

    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(cranelift::prelude::types::I32));
    sig.params.push(AbiParam::new(cranelift::prelude::types::I32));
    sig.returns.push(AbiParam::new(ptr_type));

    let func_id = ctx.module
        .declare_function("naml_struct_new", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_struct_new: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, &[type_id, field_count]);
    Ok(builder.inst_results(call)[0])
}

fn call_struct_get_field(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    struct_ptr: Value,
    field_index: Value,
) -> Result<Value, CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();

    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(cranelift::prelude::types::I32));
    sig.returns.push(AbiParam::new(cranelift::prelude::types::I64));

    let func_id = ctx.module
        .declare_function("naml_struct_get_field", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_struct_get_field: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, &[struct_ptr, field_index]);
    Ok(builder.inst_results(call)[0])
}

fn call_struct_set_field(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    struct_ptr: Value,
    field_index: Value,
    value: Value,
) -> Result<(), CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();

    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(cranelift::prelude::types::I32));
    sig.params.push(AbiParam::new(cranelift::prelude::types::I64));

    let func_id = ctx.module
        .declare_function("naml_struct_set_field", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_struct_set_field: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    builder.ins().call(func_ref, &[struct_ptr, field_index, value]);
    Ok(())
}

fn compile_extern_call(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    extern_fn: &ExternFn,
    args: &[Expression<'_>],
) -> Result<Value, CodegenError> {
    // Build the signature
    let mut sig = ctx.module.make_signature();

    for param_ty in &extern_fn.param_types {
        let cl_type = types::naml_to_cranelift(param_ty);
        sig.params.push(AbiParam::new(cl_type));
    }

    if let Some(ref ret_ty) = extern_fn.return_type {
        let cl_type = types::naml_to_cranelift(ret_ty);
        sig.returns.push(AbiParam::new(cl_type));
    }

    // Declare the external function
    let func_id = ctx.module
        .declare_function(&extern_fn.link_name, Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare extern fn {}: {}", extern_fn.link_name, e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);

    // Compile arguments
    let mut compiled_args = Vec::new();
    for arg in args {
        compiled_args.push(compile_expression(ctx, builder, arg)?);
    }

    // Make the call
    let call_inst = builder.ins().call(func_ref, &compiled_args);
    let results = builder.inst_results(call_inst);

    if results.is_empty() {
        Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
    } else {
        Ok(results[0])
    }
}

fn compile_method_call(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    receiver: &Expression<'_>,
    method_name: &str,
    args: &[Expression<'_>],
) -> Result<Value, CodegenError> {
    // Handle option methods first (before compiling receiver for or_default)
    match method_name {
        "is_some" => {
            let opt_ptr = compile_expression(ctx, builder, receiver)?;
            let tag = builder.ins().load(cranelift::prelude::types::I32, MemFlags::new(), opt_ptr, 0);
            let one = builder.ins().iconst(cranelift::prelude::types::I32, 1);
            let result = builder.ins().icmp(IntCC::Equal, tag, one);
            return Ok(builder.ins().uextend(cranelift::prelude::types::I64, result));
        }
        "is_none" => {
            let opt_ptr = compile_expression(ctx, builder, receiver)?;
            let tag = builder.ins().load(cranelift::prelude::types::I32, MemFlags::new(), opt_ptr, 0);
            let zero = builder.ins().iconst(cranelift::prelude::types::I32, 0);
            let result = builder.ins().icmp(IntCC::Equal, tag, zero);
            return Ok(builder.ins().uextend(cranelift::prelude::types::I64, result));
        }
        "or_default" => {
            let opt_ptr = compile_expression(ctx, builder, receiver)?;
            if args.is_empty() {
                return Err(CodegenError::JitCompile("or_default requires a default value argument".to_string()));
            }
            let default_val = compile_expression(ctx, builder, &args[0])?;

            let tag = builder.ins().load(cranelift::prelude::types::I32, MemFlags::new(), opt_ptr, 0);
            let is_some = builder.ins().icmp_imm(IntCC::Equal, tag, 1);

            let some_val = builder.ins().load(cranelift::prelude::types::I64, MemFlags::new(), opt_ptr, 8);

            return Ok(builder.ins().select(is_some, some_val, default_val));
        }
        _ => {}
    }

    let recv = compile_expression(ctx, builder, receiver)?;

    match method_name {
        "len" => {
            // Check receiver type to dispatch to correct len function
            let receiver_type = ctx.annotations.get_type(receiver.span());
            if matches!(receiver_type, Some(Type::String)) {
                call_string_char_len(ctx, builder, recv)
            } else {
                call_array_len(ctx, builder, recv)
            }
        }
        "char_at" => {
            // String char_at method
            if args.is_empty() {
                return Err(CodegenError::JitCompile("char_at requires an index argument".to_string()));
            }
            let idx = compile_expression(ctx, builder, &args[0])?;
            call_string_char_at(ctx, builder, recv, idx)
        }
        "push" => {
            if args.is_empty() {
                return Err(CodegenError::JitCompile("push requires an argument".to_string()));
            }
            let val = compile_expression(ctx, builder, &args[0])?;
            call_array_push(ctx, builder, recv, val)?;
            Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
        }
        "pop" => {
            let ptr_type = ctx.module.target_config().pointer_type();

            let mut sig = ctx.module.make_signature();
            sig.params.push(AbiParam::new(ptr_type));
            sig.returns.push(AbiParam::new(cranelift::prelude::types::I64));

            let func_id = ctx.module
                .declare_function("naml_array_pop", Linkage::Import, &sig)
                .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_array_pop: {}", e)))?;

            let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
            let call = builder.ins().call(func_ref, &[recv]);
            Ok(builder.inst_results(call)[0])
        }
        "get" => {
            if args.is_empty() {
                return Err(CodegenError::JitCompile("get requires an index argument".to_string()));
            }
            let idx = compile_expression(ctx, builder, &args[0])?;
            call_array_get(ctx, builder, recv, idx)
        }
        // Channel methods
        "send" => {
            if args.is_empty() {
                return Err(CodegenError::JitCompile("send requires a value argument".to_string()));
            }
            let val = compile_expression(ctx, builder, &args[0])?;
            call_channel_send(ctx, builder, recv, val)
        }
        "receive" => {
            call_channel_receive(ctx, builder, recv)
        }
        "close" => {
            call_channel_close(ctx, builder, recv)?;
            Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
        }
        // Map methods
        "contains" => {
            if args.is_empty() {
                return Err(CodegenError::JitCompile("contains requires a key argument".to_string()));
            }
            let key = compile_expression(ctx, builder, &args[0])?;
            call_map_contains(ctx, builder, recv, key)
        }
        "set" => {
            if args.len() < 2 {
                return Err(CodegenError::JitCompile("set requires key and value arguments".to_string()));
            }
            let key = compile_expression(ctx, builder, &args[0])?;
            let value = compile_expression(ctx, builder, &args[1])?;
            call_map_set(ctx, builder, recv, key, value)?;
            Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
        }
        "message" => {
            // Exception message() method - load string from offset 0
            let receiver_type = ctx.annotations.get_type(receiver.span());
            if matches!(receiver_type, Some(Type::Exception(_))) {
                // Exception message is stored at offset 0
                let message_ptr = builder.ins().load(
                    cranelift::prelude::types::I64,
                    MemFlags::new(),
                    recv,
                    0,
                );
                Ok(message_ptr)
            } else {
                Err(CodegenError::JitCompile("message() is only available on exception types".to_string()))
            }
        }
        _ => {
            // Try to look up user-defined method
            let receiver_type = ctx.annotations.get_type(receiver.span());
            let type_name = match receiver_type {
                Some(Type::Struct(s)) => Some(ctx.interner.resolve(&s.name).to_string()),
                Some(Type::Generic(name, _)) => Some(ctx.interner.resolve(&name).to_string()),
                _ => None,
            };

            if let Some(type_name) = type_name {
                let full_name = format!("{}_{}", type_name, method_name);
                if let Some(&func_id) = ctx.functions.get(&full_name) {
                    let ptr_type = ctx.module.target_config().pointer_type();
                    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);

                    // Compile arguments
                    let mut call_args = vec![recv];
                    for arg in args {
                        call_args.push(compile_expression(ctx, builder, arg)?);
                    }

                    let call = builder.ins().call(func_ref, &call_args);
                    let results = builder.inst_results(call);
                    if results.is_empty() {
                        return Ok(builder.ins().iconst(ptr_type, 0));
                    } else {
                        return Ok(results[0]);
                    }
                }
            }

            Err(CodegenError::Unsupported(format!("Unknown method: {}", method_name)))
        }
    }
}

// Scheduler helper functions
fn call_sleep(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    ms: Value,
) -> Result<Value, CodegenError> {
    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(cranelift::prelude::types::I64));

    let func_id = ctx.module
        .declare_function("naml_sleep", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_sleep: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    builder.ins().call(func_ref, &[ms]);
    Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
}

fn call_wait_all(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<Value, CodegenError> {
    let sig = ctx.module.make_signature();

    let func_id = ctx.module
        .declare_function("naml_wait_all", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_wait_all: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    builder.ins().call(func_ref, &[]);
    Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
}

// Channel helper functions
fn call_channel_new(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    capacity: Value,
) -> Result<Value, CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();

    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(cranelift::prelude::types::I64));
    sig.returns.push(AbiParam::new(ptr_type));

    let func_id = ctx.module
        .declare_function("naml_channel_new", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_channel_new: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, &[capacity]);
    Ok(builder.inst_results(call)[0])
}

fn call_channel_send(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    ch: Value,
    value: Value,
) -> Result<Value, CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();

    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(cranelift::prelude::types::I64));
    sig.returns.push(AbiParam::new(cranelift::prelude::types::I64));

    let func_id = ctx.module
        .declare_function("naml_channel_send", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_channel_send: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, &[ch, value]);
    Ok(builder.inst_results(call)[0])
}

fn call_channel_receive(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    ch: Value,
) -> Result<Value, CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();

    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(cranelift::prelude::types::I64));

    let func_id = ctx.module
        .declare_function("naml_channel_receive", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_channel_receive: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, &[ch]);
    Ok(builder.inst_results(call)[0])
}

fn call_channel_close(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    ch: Value,
) -> Result<(), CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();

    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ptr_type));

    let func_id = ctx.module
        .declare_function("naml_channel_close", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_channel_close: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    builder.ins().call(func_ref, &[ch]);
    Ok(())
}

fn call_alloc_closure_data(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    size: Value,
) -> Result<Value, CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();

    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(cranelift::prelude::types::I64)); // size: usize
    sig.returns.push(AbiParam::new(ptr_type)); // *mut u8

    let func_id = ctx.module
        .declare_function("naml_alloc_closure_data", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_alloc_closure_data: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, &[size]);
    Ok(builder.inst_results(call)[0])
}

fn call_spawn_closure(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    func_addr: Value,
    data: Value,
    data_size: Value,
) -> Result<(), CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();

    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ptr_type)); // func: extern "C" fn(*mut u8)
    sig.params.push(AbiParam::new(ptr_type)); // data: *mut u8
    sig.params.push(AbiParam::new(cranelift::prelude::types::I64)); // data_size: usize

    let func_id = ctx.module
        .declare_function("naml_spawn_closure", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_spawn_closure: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    builder.ins().call(func_ref, &[func_addr, data, data_size]);
    Ok(())
}

extern "C" fn naml_print_int(val: i64) {
    print!("{}", val);
}

extern "C" fn naml_print_float(val: f64) {
    print!("{}", val);
}

// Exception handling helper functions
fn call_exception_set(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    exception_ptr: Value,
) -> Result<(), CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();

    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ptr_type));

    let func_id = ctx.module
        .declare_function("naml_exception_set", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_exception_set: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    builder.ins().call(func_ref, &[exception_ptr]);
    Ok(())
}

fn call_exception_get(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<Value, CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();

    let mut sig = ctx.module.make_signature();
    sig.returns.push(AbiParam::new(ptr_type));

    let func_id = ctx.module
        .declare_function("naml_exception_get", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_exception_get: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, &[]);
    Ok(builder.inst_results(call)[0])
}

fn call_exception_clear(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), CodegenError> {
    let sig = ctx.module.make_signature();

    let func_id = ctx.module
        .declare_function("naml_exception_clear", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_exception_clear: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    builder.ins().call(func_ref, &[]);
    Ok(())
}

fn call_exception_check(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<Value, CodegenError> {
    let mut sig = ctx.module.make_signature();
    sig.returns.push(AbiParam::new(cranelift::prelude::types::I64));

    let func_id = ctx.module
        .declare_function("naml_exception_check", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_exception_check: {}", e)))?;

    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let call = builder.ins().call(func_ref, &[]);
    Ok(builder.inst_results(call)[0])
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
