use std::collections::{HashMap, HashSet};
use std::panic;

use cranelift::prelude::*;
use cranelift_module::{FuncId, Linkage, Module};

use crate::ast::FunctionItem;
use crate::codegen::CodegenError;
use crate::codegen::cranelift::errors::convert_cranelift_error;
use crate::codegen::cranelift::runtime::{emit_cleanup_all_vars, emit_stack_pop, emit_stack_push};
use crate::codegen::cranelift::stmt::compile_statement;
use crate::codegen::cranelift::{collect_reassigned_vars, types, CompileContext, JitCompiler};

impl<'a> JitCompiler<'a> {
    pub(crate) fn declare_method(&mut self, func: &FunctionItem<'_>) -> Result<FuncId, CodegenError> {
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

    pub(crate) fn compile_method(&mut self, func: &FunctionItem<'_>) -> Result<(), CodegenError> {
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

        // Scan method body for variable reassignments to enable borrow optimization
        if let Some(ref body) = func.body {
            collect_reassigned_vars(&body.statements, self.interner, &mut ctx.reassigned_vars);
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
}
