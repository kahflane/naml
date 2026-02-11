use std::collections::{HashMap, HashSet};
use std::panic;

use cranelift::prelude::*;
use cranelift_module::{FuncId, Linkage};

use crate::codegen::CodegenError;
use crate::codegen::cranelift::errors::convert_cranelift_error;
use crate::codegen::cranelift::runtime::emit_cleanup_all_vars;
use crate::codegen::cranelift::stmt::compile_statement;
use crate::codegen::cranelift::{CompileContext, JitCompiler, SpawnBlockInfo};

impl<'a> JitCompiler<'a> {
    pub(crate) fn declare_spawn_trampoline(
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

    pub(crate) fn compile_spawn_trampoline(&mut self, info: &SpawnBlockInfo) -> Result<(), CodegenError> {
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
            func_return_type: None,
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
            if let Some(heap_type) = info.captured_heap_types.get(var_name) {
                ctx.var_heap_types.insert(var_name.clone(), heap_type.clone());
            }
        }
        let body = unsafe { &*info.body_ptr };
        for stmt in &body.statements {
            compile_statement(&mut ctx, &mut builder, stmt)?;
            if ctx.block_terminated {
                break;
            }
        }

        if !ctx.block_terminated {
            emit_cleanup_all_vars(&mut ctx, &mut builder, None)?;
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
}
