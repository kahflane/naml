use cranelift::prelude::*;
use cranelift_codegen::ir::{AbiParam, Value};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::{Linkage, Module};
use crate::ast::Expression;
use crate::codegen::CodegenError;
use crate::codegen::cranelift::{types, CompileContext, ExternFn};
use crate::codegen::cranelift::expr::compile_expression;

pub fn compile_extern_call(
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
    let func_id = ctx
        .module
        .declare_function(&extern_fn.link_name, Linkage::Import, &sig)
        .map_err(|e| {
            CodegenError::JitCompile(format!(
                "Failed to declare extern fn {}: {}",
                extern_fn.link_name, e
            ))
        })?;

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
