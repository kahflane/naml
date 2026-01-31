use cranelift::prelude::*;
use cranelift_codegen::ir::Value;
use cranelift_frontend::FunctionBuilder;
use crate::codegen::CodegenError;
use crate::codegen::cranelift::{CompileContext};
use crate::codegen::cranelift::runtime::rt_func_ref;

// Exception handling helper functions
pub fn call_exception_set(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    exception_ptr: Value,
) -> Result<(), CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_exception_set")?;
    builder.ins().call(func_ref, &[exception_ptr]);
    Ok(())
}

pub fn call_exception_get(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_exception_get")?;
    let call = builder.ins().call(func_ref, &[]);
    Ok(builder.inst_results(call)[0])
}

pub fn call_exception_clear(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_exception_clear")?;
    builder.ins().call(func_ref, &[]);
    Ok(())
}

pub fn call_exception_check(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_exception_check")?;
    let call = builder.ins().call(func_ref, &[]);
    Ok(builder.inst_results(call)[0])
}