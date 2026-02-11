use std::collections::{HashMap, HashSet};
use std::panic;

use cranelift::prelude::*;
use cranelift_module::{FuncId, Linkage, Module};

use crate::ast::{Expression, FunctionItem};
use crate::codegen::CodegenError;
use crate::codegen::cranelift::errors::convert_cranelift_error;
use crate::codegen::cranelift::expr::compile_expression;
use crate::codegen::cranelift::runtime::{emit_cleanup_all_vars, emit_stack_pop, emit_stack_push};
use crate::codegen::cranelift::stmt::compile_statement;
use crate::codegen::cranelift::{
    collect_reassigned_vars, types, CompileContext, InlineFuncInfo, JitCompiler,
};

impl<'a> JitCompiler<'a> {
    pub(crate) fn declare_function(&mut self, func: &FunctionItem<'_>) -> Result<FuncId, CodegenError> {
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
        } else if name == "main" && self.module.is_aot() {
            sig.returns
                .push(AbiParam::new(cranelift::prelude::types::I32));
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

    /// Check if a function is a good candidate for inlining and store it if so.
    /// Criteria: small body, no throws, no generics, not "main", not recursive.
    pub(crate) fn maybe_add_inline_candidate(&mut self, func: &FunctionItem<'_>) {
        let name = self.interner.resolve(&func.name.symbol);

        // Skip main function
        if name == "main" {
            return;
        }

        // Skip functions with throws
        if !func.throws.is_empty() {
            return;
        }

        // Skip generics (handled separately)
        if !func.generics.is_empty() {
            return;
        }

        // Skip functions without bodies
        let body = match &func.body {
            Some(b) => b,
            None => return,
        };

        // Count statements - inline only small functions (max 5 statements)
        let stmt_count = body.statements.len();
        if stmt_count > 5 {
            return;
        }

        // Simple recursion check: skip if function calls itself
        // (A more sophisticated check would walk the AST)
        // For now, we rely on inline_depth limiting in compile_expression

        // Collect parameter info
        let param_names: Vec<String> = func
            .params
            .iter()
            .map(|p| self.interner.resolve(&p.name.symbol).to_string())
            .collect();

        let param_types: Vec<crate::ast::NamlType> =
            func.params.iter().map(|p| p.ty.clone()).collect();

        let return_type = func.return_ty.clone();

        // Store as inline candidate
        let info = InlineFuncInfo {
            func_ptr: func as *const _ as *const FunctionItem<'static>,
            param_names,
            param_types,
            return_type,
        };

        self.inline_functions.insert(name.to_string(), info);
    }

    pub(crate) fn compile_function(&mut self, func: &FunctionItem<'_>) -> Result<(), CodegenError> {
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

        let func_return_type = if func.return_ty.is_some() {
            func.return_ty.as_ref().map(|ty| types::naml_to_cranelift(ty))
        } else if name == "main" && self.module.is_aot() {
            Some(cranelift::prelude::types::I32)
        } else {
            None
        };

        let mut ctx = CompileContext {
            interner: self.interner,
            module: &mut *self.module,
            functions: &self.functions,
            runtime_funcs: &self.runtime_funcs,
            struct_defs: &self.struct_defs,
            enum_defs: &self.enum_defs,
            exception_names: &self.exception_names,
            extern_fns: &self.extern_fns,
            global_vars: &self.global_vars,
            variables: HashMap::new(),
            var_heap_types: HashMap::new(),
            var_counter: 0,
            block_terminated: false,
            loop_exit_block: None,
            loop_header_block: None,
            spawn_blocks: &self.spawn_blocks,
            spawn_body_to_id: &self.spawn_body_to_id,
            lambda_blocks: &self.lambda_blocks,
            lambda_body_to_id: &self.lambda_body_to_id,
            annotations: self.annotations,
            type_substitutions: HashMap::new(),
            func_return_type,
            release_mode: self.release_mode,
            unsafe_mode: self.unsafe_mode,
            inline_functions: &self.inline_functions,
            inline_depth: 0,
            inline_exit_block: None,
            inline_result_var: None,
            borrowed_vars: HashSet::new(),
            reassigned_vars: HashSet::new(),
        };

        // Scan function body for variable reassignments to enable borrow optimization
        if let Some(ref body) = func.body {
            collect_reassigned_vars(&body.statements, self.interner, &mut ctx.reassigned_vars);
        }

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
        emit_stack_push(
            &mut ctx,
            &mut builder,
            func_name_str,
            file_name,
            line as u32,
        )?;

        // If this is main, initialize global variables first
        if name == "main" {
            // Collect global var info before borrowing ctx
            let global_init_info: Vec<_> = self
                .global_vars
                .iter()
                .filter(|(_, def)| !def.init_expr.is_null())
                .map(|(name, def)| {
                    (
                        name.clone(),
                        def.data_id,
                        def.cl_type,
                        def.init_expr,
                    )
                })
                .collect();

            for (var_name, data_id, _cl_type, init_expr_ptr) in global_init_info {
                // SAFETY: the expression pointer is valid for the lifetime of compilation
                let init_expr: &Expression<'_> = unsafe { &*init_expr_ptr };

                // Compile the initializer expression
                let value = compile_expression(&mut ctx, &mut builder, init_expr)?;

                // Get the global address and store the value
                let global_value = ctx.module.declare_data_in_func(data_id, builder.func);
                let ptr = builder
                    .ins()
                    .global_value(cranelift::prelude::types::I64, global_value);

                // Store floats natively as f64 (no bitcast needed)
                builder.ins().store(MemFlags::trusted(), value, ptr, 0);
                let _ = var_name; // suppress unused warning
            }
        }

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
            if let Some(ret_ty) = ctx.func_return_type {
                let zero = builder.ins().iconst(ret_ty, 0);
                builder.ins().return_(&[zero]);
            } else {
                builder.ins().return_(&[]);
            }
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
}
