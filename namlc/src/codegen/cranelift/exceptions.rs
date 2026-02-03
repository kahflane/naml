use cranelift::prelude::*;
use cranelift_codegen::ir::Value;
use cranelift_frontend::FunctionBuilder;
use crate::codegen::CodegenError;
use crate::codegen::cranelift::{CompileContext};
use crate::codegen::cranelift::runtime::rt_func_ref;
use crate::codegen::cranelift::literal::compile_string_literal;

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

/// Throw a DecodeError exception with the given error position
pub fn throw_decode_error(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    position: Value,
) -> Result<(), CodegenError> {
    let cstr_ptr = compile_string_literal(ctx, builder, "invalid encoding at position")?;

    let from_cstr = rt_func_ref(ctx, builder, "naml_string_from_cstr")?;
    let call = builder.ins().call(from_cstr, &[cstr_ptr]);
    let message = builder.inst_results(call)[0];

    let func_ref = rt_func_ref(ctx, builder, "naml_decode_error_new")?;
    let call = builder.ins().call(func_ref, &[message, position]);
    let exc_ptr = builder.inst_results(call)[0];

    call_exception_set(ctx, builder, exc_ptr)?;
    Ok(())
}

/// Throw a PathError exception with the given path string
pub fn throw_path_error(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    path: Value,
) -> Result<(), CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_path_error_new")?;
    let call = builder.ins().call(func_ref, &[path]);
    let exc_ptr = builder.inst_results(call)[0];

    call_exception_set(ctx, builder, exc_ptr)?;
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

/// Clear only the exception pointer, preserving type ID for 'is' checks in catch blocks
pub fn call_exception_clear_ptr(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_exception_clear_ptr")?;
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