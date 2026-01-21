///
/// Cranelift JIT Compiler
///
/// Compiles naml AST directly to machine code using Cranelift.
/// This eliminates the Rust transpilation step and gives full control
/// over memory management and runtime semantics.
///

mod types;

use std::collections::HashMap;

use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{DataDescription, FuncId, Linkage, Module};
use lasso::Rodeo;

use crate::ast::{
    BinaryOp, Expression, FunctionItem, Item, Literal, SourceFile, Statement, UnaryOp,
    LiteralExpr,
};
use crate::codegen::CodegenError;
use crate::typechecker::{SymbolTable, TypeAnnotations};

/// Struct definition with type_id and ordered field names
#[derive(Clone)]
pub struct StructDef {
    pub type_id: u32,
    pub fields: Vec<String>,
}

/// Enum definition with variant info for codegen
#[derive(Clone)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<EnumVariantDef>,
    /// Size in bytes (8 + max_data_size, aligned)
    pub size: usize,
}

#[derive(Clone)]
pub struct EnumVariantDef {
    pub name: String,
    pub tag: u32,
    /// Types of data fields (empty for unit variants)
    pub field_types: Vec<crate::ast::NamlType>,
    /// Offset of data within enum (always 8 for now)
    pub data_offset: usize,
}

/// External function declaration info
#[derive(Clone)]
pub struct ExternFn {
    pub link_name: String,
    pub param_types: Vec<crate::ast::NamlType>,
    pub return_type: Option<crate::ast::NamlType>,
}

/// Information about a spawn block for closure compilation
#[derive(Clone)]
pub struct SpawnBlockInfo {
    pub id: u32,
    pub func_name: String,
    pub captured_vars: Vec<String>,
    /// Raw pointer to the spawn block body for deferred compilation
    /// Safety: Only valid during the same compile() call
    pub body_ptr: *const crate::ast::BlockExpr<'static>,
}

/// Explicitly implement Send since body_ptr is only used within compile()
unsafe impl Send for SpawnBlockInfo {}

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

        // Struct operations
        builder.symbol("naml_struct_new", crate::runtime::naml_struct_new as *const u8);
        builder.symbol("naml_struct_incref", crate::runtime::naml_struct_incref as *const u8);
        builder.symbol("naml_struct_decref", crate::runtime::naml_struct_decref as *const u8);
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
        builder.symbol("naml_map_get", crate::runtime::naml_map_get as *const u8);
        builder.symbol("naml_map_contains", crate::runtime::naml_map_contains as *const u8);
        builder.symbol("naml_map_len", crate::runtime::naml_map_len as *const u8);
        builder.symbol("naml_map_incref", crate::runtime::naml_map_incref as *const u8);
        builder.symbol("naml_map_decref", crate::runtime::naml_map_decref as *const u8);

        // String operations
        builder.symbol("naml_string_from_cstr", crate::runtime::naml_string_from_cstr as *const u8);

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
        })
    }

    pub fn compile(&mut self, ast: &SourceFile<'_>) -> Result<(), CodegenError> {
        // First pass: collect struct definitions
        for item in &ast.items {
            if let crate::ast::Item::Struct(struct_item) = item {
                let name = self.interner.resolve(&struct_item.name.symbol).to_string();
                let fields: Vec<String> = struct_item.fields.iter()
                    .map(|f| self.interner.resolve(&f.name.symbol).to_string())
                    .collect();

                let type_id = self.next_type_id;
                self.next_type_id += 1;

                self.struct_defs.insert(name, StructDef { type_id, fields });
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

        for item in &ast.items {
            if let Item::Function(f) = item {
                if f.receiver.is_none() {
                    self.declare_function(f)?;
                }
            }
        }

        for item in &ast.items {
            if let Item::Function(f) = item {
                if f.receiver.is_none() && f.body.is_some() {
                    self.compile_function(f)?;
                }
            }
        }

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
            var_counter: 0,
            block_terminated: false,
            loop_exit_block: None,
            loop_header_block: None,
            spawn_blocks: &self.spawn_blocks,
            current_spawn_id: 0, // Not used in trampolines
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
            var_counter: 0,
            block_terminated: false,
            loop_exit_block: None,
            loop_header_block: None,
            spawn_blocks: &self.spawn_blocks,
            current_spawn_id: 0,
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
            builder.ins().return_(&[]);
        }

        builder.finalize();

        self.module
            .define_function(func_id, &mut self.ctx)
            .map_err(|e| CodegenError::JitCompile(format!("Failed to define function '{}': {}", name, e)))?;

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

struct CompileContext<'a> {
    interner: &'a Rodeo,
    module: &'a mut JITModule,
    functions: &'a HashMap<String, FuncId>,
    struct_defs: &'a HashMap<String, StructDef>,
    enum_defs: &'a HashMap<String, EnumDef>,
    extern_fns: &'a HashMap<String, ExternFn>,
    variables: HashMap<String, Variable>,
    var_counter: usize,
    block_terminated: bool,
    loop_exit_block: Option<Block>,
    loop_header_block: Option<Block>,
    spawn_blocks: &'a HashMap<u32, SpawnBlockInfo>,
    /// Counter for tracking which spawn block we're currently at
    current_spawn_id: u32,
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

            let var = Variable::new(ctx.var_counter);
            ctx.var_counter += 1;
            builder.declare_var(var, ty);

            if let Some(ref init) = var_stmt.init {
                let val = compile_expression(ctx, builder, init)?;
                builder.def_var(var, val);
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
                    let val = compile_expression(ctx, builder, &assign.value)?;

                    if let Some(&var) = ctx.variables.get(&var_name) {
                        builder.def_var(var, val);
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
                _ => {
                    return Err(CodegenError::Unsupported(
                        format!("Assignment target not supported: {:?}", std::mem::discriminant(&assign.target))
                    ));
                }
            }
        }

        Statement::Return(ret) => {
            if let Some(ref expr) = ret.value {
                let val = compile_expression(ctx, builder, expr)?;
                builder.ins().return_(&[val]);
            } else {
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
            // Compile the iterable (should be an array)
            let arr_ptr = compile_expression(ctx, builder, &for_stmt.iterable)?;

            // Get array length
            let len = call_array_len(ctx, builder, arr_ptr)?;

            // Create index variable
            let idx_var = Variable::new(ctx.var_counter);
            ctx.var_counter += 1;
            builder.declare_var(idx_var, cranelift::prelude::types::I64);
            let zero = builder.ins().iconst(cranelift::prelude::types::I64, 0);
            builder.def_var(idx_var, zero);

            // Create value variable
            let val_var = Variable::new(ctx.var_counter);
            ctx.var_counter += 1;
            builder.declare_var(val_var, cranelift::prelude::types::I64);
            let val_name = ctx.interner.resolve(&for_stmt.value.symbol).to_string();
            ctx.variables.insert(val_name, val_var);

            // Optionally create index binding
            if let Some(ref idx_ident) = for_stmt.index {
                let idx_name = ctx.interner.resolve(&idx_ident.symbol).to_string();
                ctx.variables.insert(idx_name, idx_var);
            }

            // Create blocks
            let header_block = builder.create_block();
            let body_block = builder.create_block();
            let exit_block = builder.create_block();

            // Store exit block for break statements
            let prev_loop_exit = ctx.loop_exit_block.take();
            let prev_loop_header = ctx.loop_header_block.take();
            ctx.loop_exit_block = Some(exit_block);
            ctx.loop_header_block = Some(header_block);

            builder.ins().jump(header_block, &[]);

            // Header: check if idx < len
            builder.switch_to_block(header_block);
            let idx_val = builder.use_var(idx_var);
            let cond = builder.ins().icmp(IntCC::SignedLessThan, idx_val, len);
            builder.ins().brif(cond, body_block, &[], exit_block, &[]);

            // Body
            builder.switch_to_block(body_block);
            builder.seal_block(body_block);
            ctx.block_terminated = false;

            // Get current element
            let idx_val = builder.use_var(idx_var);
            let elem = call_array_get(ctx, builder, arr_ptr, idx_val)?;
            builder.def_var(val_var, elem);

            // Compile body
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
                builder.ins().jump(header_block, &[]);
            }

            builder.seal_block(header_block);
            builder.switch_to_block(exit_block);
            builder.seal_block(exit_block);
            ctx.block_terminated = false;

            // Restore previous loop context
            ctx.loop_exit_block = prev_loop_exit;
            ctx.loop_header_block = prev_loop_header;
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

        _ => {
            return Err(CodegenError::Unsupported(
                format!("Statement type not yet implemented: {:?}", std::mem::discriminant(stmt))
            ));
        }
    }

    Ok(())
}

/// Compile a pattern match, returning a boolean value indicating if the pattern matches.
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
            // Check if this is an enum variant name by checking switch_scrutinee context
            // For now, identifier patterns always match (they bind the value)
            // If it's a bare variant name, we need to compare tags
            let name = ctx.interner.resolve(&ident.ident.symbol).to_string();

            // Check all enum definitions for a matching variant
            for enum_def in ctx.enum_defs.values() {
                if let Some(variant) = enum_def.variants.iter().find(|v| v.name == name) {
                    // This is a variant name - compare tags
                    let tag = builder.ins().load(cranelift::prelude::types::I32, MemFlags::new(), scrutinee, 0);
                    let expected_tag = builder.ins().iconst(cranelift::prelude::types::I32, variant.tag as i64);
                    return Ok(builder.ins().icmp(IntCC::Equal, tag, expected_tag));
                }
            }

            // Not a variant name - identifier pattern always matches (it's a binding)
            Ok(builder.ins().iconst(cranelift::prelude::types::I8, 1))
        }

        Pattern::Variant(variant) => {
            if variant.path.is_empty() {
                return Err(CodegenError::JitCompile("Empty variant path".to_string()));
            }

            // Handle both bare variant (Active) and qualified (Status::Active)
            let (enum_name, variant_name) = if variant.path.len() == 1 {
                // Bare variant - need to find the enum from context
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
                    // Get enum tag from scrutinee (scrutinee is pointer to enum)
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
            // Wildcard always matches
            Ok(builder.ins().iconst(cranelift::prelude::types::I8, 1))
        }

        Pattern::_Phantom(_) => {
            Ok(builder.ins().iconst(cranelift::prelude::types::I8, 0))
        }
    }
}

/// Bind pattern variables to the matched value.
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
                                let arg_val = compile_expression(ctx, builder, arg)?;
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

                let value = compile_expression(ctx, builder, &field.value)?;
                let idx_val = builder.ins().iconst(cranelift::prelude::types::I32, field_idx as i64);
                call_struct_set_field(ctx, builder, struct_ptr, idx_val, value)?;
            }

            Ok(struct_ptr)
        }

        Expression::Field(field_expr) => {
            let struct_ptr = compile_expression(ctx, builder, &field_expr.base)?;
            let field_name = ctx.interner.resolve(&field_expr.field.symbol).to_string();

            // Try to determine the struct type from the base expression
            // For now, we'll need to look up the field in all structs
            // In a real implementation, we'd use type information from the typechecker
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

    for (i, arg) in args.iter().enumerate() {
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
                // Check value type to call appropriate print function
                let val_type = builder.func.dfg.value_type(val);
                if val_type == cranelift::prelude::types::F64 {
                    call_print_float(ctx, builder, val)?;
                } else {
                    call_print_int(ctx, builder, val)?;
                }
            }
        }

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
    let ptr_type = ctx.module.target_config().pointer_type();

    let mut sig = ctx.module.make_signature();
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(cranelift::prelude::types::I64));
    sig.params.push(AbiParam::new(cranelift::prelude::types::I64));

    let func_id = ctx.module
        .declare_function("naml_map_set", Linkage::Import, &sig)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare naml_map_set: {}", e)))?;

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
            call_array_len(ctx, builder, recv)
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
        _ => {
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
