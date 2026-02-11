use std::collections::{HashMap, HashSet};
use std::panic;

use cranelift::prelude::*;
use cranelift_module::{FuncId, Linkage};

use crate::codegen::CodegenError;
use crate::codegen::cranelift::errors::convert_cranelift_error;
use crate::codegen::cranelift::expr::compile_expression;
use crate::codegen::cranelift::{CompileContext, JitCompiler, LambdaInfo};

impl<'a> JitCompiler<'a> {
    pub(crate) fn declare_lambda_function(&mut self, info: &LambdaInfo) -> Result<FuncId, CodegenError> {
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

    pub(crate) fn compile_lambda_function(&mut self, info: &LambdaInfo) -> Result<(), CodegenError> {
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
            func_return_type: Some(cranelift::prelude::types::I64), // Lambdas always return i64
            release_mode: self.release_mode,
            unsafe_mode: self.unsafe_mode,
            inline_functions: &self.inline_functions,
            inline_depth: 0,
            inline_exit_block: None,
            inline_result_var: None,
            borrowed_vars: HashSet::new(),
            reassigned_vars: HashSet::new(),
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
}
