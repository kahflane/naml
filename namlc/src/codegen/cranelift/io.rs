use cranelift::prelude::*;
use cranelift_codegen::ir::Value;
use cranelift_frontend::FunctionBuilder;
use crate::ast::Expression;
use crate::codegen::CodegenError;
use crate::codegen::cranelift::{CompileContext};
use crate::codegen::cranelift::runtime::rt_func_ref;
use crate::codegen::cranelift::strings::build_message_string;

pub fn compile_stderr_call(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    args: &[Expression<'_>],
    func_name: &str,
) -> Result<Value, CodegenError> {
    let msg = build_message_string(ctx, builder, args)?;

    let runtime_name = format!("naml_{}", func_name);
    let func_ref = rt_func_ref(ctx, builder, &runtime_name)?;
    builder.ins().call(func_ref, &[msg]);

    Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
}

pub fn compile_fmt_call(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    args: &[Expression<'_>],
) -> Result<Value, CodegenError> {
    build_message_string(ctx, builder, args)
}

pub fn call_read_line(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_read_line")?;
    let call = builder.ins().call(func_ref, &[]);
    Ok(builder.inst_results(call)[0])
}

pub fn call_read_key(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_read_key")?;
    let call = builder.ins().call(func_ref, &[]);
    Ok(builder.inst_results(call)[0])
}