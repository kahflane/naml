use crate::codegen::cranelift::{CompileContext};
use crate::codegen::CodegenError;
use cranelift::prelude::*;
use cranelift_codegen::ir::Value;
use cranelift_frontend::FunctionBuilder;
use crate::codegen::cranelift::runtime::rt_func_ref;
use crate::codegen::cranelift::strings::call_string_from_cstr;

// Scheduler helper functions
pub fn call_sleep(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    ms: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_sleep")?;
    builder.ins().call(func_ref, &[ms]);
    Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
}

pub fn call_wait_all(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_wait_all")?;
    builder.ins().call(func_ref, &[]);
    Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
}

pub fn call_random(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    min: Value,
    max: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_random")?;
    let call_inst = builder.ins().call(func_ref, &[min, max]);
    Ok(builder.inst_results(call_inst)[0])
}

pub fn call_random_float(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_random_float")?;
    let call_inst = builder.ins().call(func_ref, &[]);
    Ok(builder.inst_results(call_inst)[0])
}

pub fn call_void_runtime(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    name: &str,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, name)?;
    builder.ins().call(func_ref, &[]);
    Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
}

pub fn call_int_runtime(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    name: &str,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, name)?;
    let call = builder.ins().call(func_ref, &[]);
    Ok(builder.inst_results(call)[0])
}

pub fn call_two_arg_runtime(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    name: &str,
    a: Value,
    b: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, name)?;
    builder.ins().call(func_ref, &[a, b]);
    Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
}

pub fn call_one_arg_int_runtime(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    name: &str,
    arg: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, name)?;
    let call = builder.ins().call(func_ref, &[arg]);
    Ok(builder.inst_results(call)[0])
}

pub fn call_one_arg_ptr_runtime(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    name: &str,
    arg: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, name)?;
    let call = builder.ins().call(func_ref, &[arg]);
    Ok(builder.inst_results(call)[0])
}

pub fn call_two_arg_ptr_runtime(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    name: &str,
    a: Value,
    b: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, name)?;
    let call = builder.ins().call(func_ref, &[a, b]);
    Ok(builder.inst_results(call)[0])
}

pub fn call_two_arg_int_runtime(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    name: &str,
    a: Value,
    b: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, name)?;
    let call = builder.ins().call(func_ref, &[a, b]);
    Ok(builder.inst_results(call)[0])
}

pub fn call_two_arg_bool_runtime(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    name: &str,
    a: Value,
    b: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, name)?;
    let call = builder.ins().call(func_ref, &[a, b]);
    let result = builder.inst_results(call)[0];
    // Truncate i64 to i8 for bool type
    Ok(builder.ins().ireduce(cranelift::prelude::types::I8, result))
}

pub fn call_three_arg_ptr_runtime(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    name: &str,
    a: Value,
    b: Value,
    c: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, name)?;
    let call = builder.ins().call(func_ref, &[a, b, c]);
    Ok(builder.inst_results(call)[0])
}

pub fn call_datetime_format(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    timestamp: Value,
    fmt: Value,
) -> Result<Value, CodegenError> {
    let fmt_str = call_string_from_cstr(ctx, builder, fmt)?;
    let func_ref = rt_func_ref(ctx, builder, "naml_datetime_format")?;
    let call = builder.ins().call(func_ref, &[timestamp, fmt_str]);
    Ok(builder.inst_results(call)[0])
}

pub fn ensure_i64(builder: &mut FunctionBuilder<'_>, val: Value) -> Value {
    let ty = builder.func.dfg.value_type(val);
    if ty.is_int() && ty.bits() < 64 {
        builder.ins().uextend(cranelift::prelude::types::I64, val)
    } else {
        val
    }
}