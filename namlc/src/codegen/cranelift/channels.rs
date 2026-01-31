use cranelift::prelude::*;
use cranelift_codegen::ir::{MemFlags, StackSlotData, StackSlotKind, Value};
use cranelift_frontend::FunctionBuilder;
use crate::codegen::CodegenError;
use crate::codegen::cranelift::{CompileContext};
use crate::codegen::cranelift::misc::ensure_i64;
use crate::codegen::cranelift::runtime::rt_func_ref;

// Channel helper functions
pub fn call_channel_new(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    capacity: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_channel_new")?;
    let call = builder.ins().call(func_ref, &[capacity]);
    Ok(builder.inst_results(call)[0])
}

pub fn call_channel_send(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    ch: Value,
    value: Value,
) -> Result<Value, CodegenError> {
    let value = ensure_i64(builder, value);
    let func_ref = rt_func_ref(ctx, builder, "naml_channel_send")?;
    let call = builder.ins().call(func_ref, &[ch, value]);
    Ok(builder.inst_results(call)[0])
}

pub fn call_channel_receive(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    ch: Value,
) -> Result<Value, CodegenError> {
    // Allocate stack slot for option<T> (16 bytes: tag at 0, value at 8)
    let option_slot =
        builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 16, 0));
    let option_ptr = builder
        .ins()
        .stack_addr(cranelift::prelude::types::I64, option_slot, 0);
    let value_ptr = builder
        .ins()
        .stack_addr(cranelift::prelude::types::I64, option_slot, 8);

    // Call runtime: naml_channel_receive(ch, &out_value) -> tag
    let func_ref = rt_func_ref(ctx, builder, "naml_channel_receive")?;
    let call = builder.ins().call(func_ref, &[ch, value_ptr]);
    let tag = builder.inst_results(call)[0];

    // Store the tag (truncate i64 to i32 for option tag)
    let tag_i32 = builder.ins().ireduce(cranelift::prelude::types::I32, tag);
    builder.ins().store(MemFlags::new(), tag_i32, option_ptr, 0);

    Ok(option_ptr)
}

pub fn call_channel_close(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    ch: Value,
) -> Result<(), CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_channel_close")?;
    builder.ins().call(func_ref, &[ch]);
    Ok(())
}