use std::collections::{HashMap, HashSet};
use std::panic;

use cranelift::prelude::*;
use cranelift_module::{FuncId, Linkage};

use crate::ast::FunctionItem;
use crate::codegen::CodegenError;
use crate::codegen::cranelift::{JitCompiler, CompileContext};
use crate::codegen::cranelift::errors::convert_cranelift_error;
use crate::codegen::cranelift::runtime::emit_cleanup_all_vars;
use crate::codegen::cranelift::stmt::compile_statement;
use crate::codegen::cranelift::types;
use crate::typechecker::Type;

impl<'a> JitCompiler<'a> {
    pub fn process_monomorphizations(&mut self) -> Result<(), CodegenError> {
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
            type_substitutions,
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
}
