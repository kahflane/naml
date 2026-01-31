use cranelift::prelude::*;
use cranelift_codegen::ir::{MemFlags, StackSlotData, StackSlotKind, Value};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_frontend::FunctionBuilder;
use crate::codegen::CodegenError;
use crate::codegen::cranelift::{CompileContext};
use crate::codegen::cranelift::runtime::rt_func_ref;

pub fn compile_lambda_bool_collection(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
    closure: Value,
    runtime_fn: &str,
) -> Result<Value, CodegenError> {
    let func_ptr = builder
        .ins()
        .load(cranelift::prelude::types::I64, MemFlags::new(), closure, 0);
    let data_ptr = builder
        .ins()
        .load(cranelift::prelude::types::I64, MemFlags::new(), closure, 8);
    let func_ref = rt_func_ref(ctx, builder, runtime_fn)?;
    let call = builder.ins().call(func_ref, &[arr, func_ptr, data_ptr]);
    let result = builder.inst_results(call)[0];
    Ok(builder.ins().ireduce(cranelift::prelude::types::I8, result))
}

pub fn compile_lambda_int_collection(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
    closure: Value,
    runtime_fn: &str,
) -> Result<Value, CodegenError> {
    let func_ptr = builder
        .ins()
        .load(cranelift::prelude::types::I64, MemFlags::new(), closure, 0);
    let data_ptr = builder
        .ins()
        .load(cranelift::prelude::types::I64, MemFlags::new(), closure, 8);
    let func_ref = rt_func_ref(ctx, builder, runtime_fn)?;
    let call = builder.ins().call(func_ref, &[arr, func_ptr, data_ptr]);
    Ok(builder.inst_results(call)[0])
}

pub fn compile_lambda_array_collection(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
    closure: Value,
    runtime_fn: &str,
) -> Result<Value, CodegenError> {
    let func_ptr = builder
        .ins()
        .load(cranelift::prelude::types::I64, MemFlags::new(), closure, 0);
    let data_ptr = builder
        .ins()
        .load(cranelift::prelude::types::I64, MemFlags::new(), closure, 8);
    let func_ref = rt_func_ref(ctx, builder, runtime_fn)?;
    let call = builder.ins().call(func_ref, &[arr, func_ptr, data_ptr]);
    Ok(builder.inst_results(call)[0])
}

pub fn compile_lambda_find(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
    closure: Value,
) -> Result<Value, CodegenError> {
    let option_slot =
        builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 16, 0));
    let option_ptr = builder
        .ins()
        .stack_addr(cranelift::prelude::types::I64, option_slot, 0);

    let found_slot =
        builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 0));
    let found_ptr = builder
        .ins()
        .stack_addr(cranelift::prelude::types::I64, found_slot, 0);

    let func_ptr = builder
        .ins()
        .load(cranelift::prelude::types::I64, MemFlags::new(), closure, 0);
    let data_ptr = builder
        .ins()
        .load(cranelift::prelude::types::I64, MemFlags::new(), closure, 8);

    let func_ref = rt_func_ref(ctx, builder, "naml_array_find")?;
    let call = builder
        .ins()
        .call(func_ref, &[arr, func_ptr, data_ptr, found_ptr]);
    let value = builder.inst_results(call)[0];

    let found_flag = builder.ins().load(
        cranelift::prelude::types::I64,
        MemFlags::new(),
        found_ptr,
        0,
    );

    let found_block = builder.create_block();
    let not_found_block = builder.create_block();
    let merge_block = builder.create_block();

    let zero = builder.ins().iconst(cranelift::prelude::types::I64, 0);
    let not_found = builder.ins().icmp(IntCC::Equal, found_flag, zero);
    builder
        .ins()
        .brif(not_found, not_found_block, &[], found_block, &[]);

    builder.switch_to_block(not_found_block);
    builder.seal_block(not_found_block);
    let none_tag = builder.ins().iconst(cranelift::prelude::types::I32, 0);
    builder
        .ins()
        .store(MemFlags::new(), none_tag, option_ptr, 0);
    builder.ins().jump(merge_block, &[]);

    builder.switch_to_block(found_block);
    builder.seal_block(found_block);
    let some_tag = builder.ins().iconst(cranelift::prelude::types::I32, 1);
    builder
        .ins()
        .store(MemFlags::new(), some_tag, option_ptr, 0);
    builder.ins().store(MemFlags::new(), value, option_ptr, 8);
    builder.ins().jump(merge_block, &[]);

    builder.switch_to_block(merge_block);
    builder.seal_block(merge_block);

    Ok(option_ptr)
}

pub fn compile_lambda_find_index(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
    closure: Value,
) -> Result<Value, CodegenError> {
    let option_slot =
        builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 16, 0));
    let option_ptr = builder
        .ins()
        .stack_addr(cranelift::prelude::types::I64, option_slot, 0);

    let func_ptr = builder
        .ins()
        .load(cranelift::prelude::types::I64, MemFlags::new(), closure, 0);
    let data_ptr = builder
        .ins()
        .load(cranelift::prelude::types::I64, MemFlags::new(), closure, 8);

    let func_ref = rt_func_ref(ctx, builder, "naml_array_find_index")?;
    let call = builder.ins().call(func_ref, &[arr, func_ptr, data_ptr]);
    let index = builder.inst_results(call)[0];

    let found_block = builder.create_block();
    let not_found_block = builder.create_block();
    let merge_block = builder.create_block();

    let neg_one = builder
        .ins()
        .iconst(cranelift::prelude::types::I64, -1i64 as i64);
    let not_found = builder.ins().icmp(IntCC::Equal, index, neg_one);
    builder
        .ins()
        .brif(not_found, not_found_block, &[], found_block, &[]);

    builder.switch_to_block(not_found_block);
    builder.seal_block(not_found_block);
    let none_tag = builder.ins().iconst(cranelift::prelude::types::I32, 0);
    builder
        .ins()
        .store(MemFlags::new(), none_tag, option_ptr, 0);
    builder.ins().jump(merge_block, &[]);

    builder.switch_to_block(found_block);
    builder.seal_block(found_block);
    let some_tag = builder.ins().iconst(cranelift::prelude::types::I32, 1);
    builder
        .ins()
        .store(MemFlags::new(), some_tag, option_ptr, 0);
    builder.ins().store(MemFlags::new(), index, option_ptr, 8);
    builder.ins().jump(merge_block, &[]);

    builder.switch_to_block(merge_block);
    builder.seal_block(merge_block);

    Ok(option_ptr)
}

pub fn compile_lambda_fold(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
    initial: Value,
    closure: Value,
) -> Result<Value, CodegenError> {
    let func_ptr = builder
        .ins()
        .load(cranelift::prelude::types::I64, MemFlags::new(), closure, 0);
    let data_ptr = builder
        .ins()
        .load(cranelift::prelude::types::I64, MemFlags::new(), closure, 8);

    let func_ref = rt_func_ref(ctx, builder, "naml_array_fold")?;
    let call = builder
        .ins()
        .call(func_ref, &[arr, initial, func_ptr, data_ptr]);
    Ok(builder.inst_results(call)[0])
}

pub fn compile_lambda_sort_by(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
    closure: Value,
) -> Result<Value, CodegenError> {
    let func_ptr = builder
        .ins()
        .load(cranelift::prelude::types::I64, MemFlags::new(), closure, 0);
    let data_ptr = builder
        .ins()
        .load(cranelift::prelude::types::I64, MemFlags::new(), closure, 8);

    let func_ref = rt_func_ref(ctx, builder, "naml_array_sort_by")?;
    let call = builder.ins().call(func_ref, &[arr, func_ptr, data_ptr]);
    Ok(builder.inst_results(call)[0])
}

pub fn compile_lambda_find_last(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
    closure: Value,
) -> Result<Value, CodegenError> {
    let option_slot =
        builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 16, 0));
    let option_ptr = builder
        .ins()
        .stack_addr(cranelift::prelude::types::I64, option_slot, 0);

    let found_slot =
        builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 0));
    let found_ptr = builder
        .ins()
        .stack_addr(cranelift::prelude::types::I64, found_slot, 0);

    let func_ptr = builder
        .ins()
        .load(cranelift::prelude::types::I64, MemFlags::new(), closure, 0);
    let data_ptr = builder
        .ins()
        .load(cranelift::prelude::types::I64, MemFlags::new(), closure, 8);

    let func_ref = rt_func_ref(ctx, builder, "naml_array_find_last")?;
    let call = builder
        .ins()
        .call(func_ref, &[arr, func_ptr, data_ptr, found_ptr]);
    let value = builder.inst_results(call)[0];

    let found_flag = builder.ins().load(
        cranelift::prelude::types::I64,
        MemFlags::new(),
        found_ptr,
        0,
    );

    let found_block = builder.create_block();
    let not_found_block = builder.create_block();
    let merge_block = builder.create_block();

    let zero = builder.ins().iconst(cranelift::prelude::types::I64, 0);
    let not_found = builder.ins().icmp(IntCC::Equal, found_flag, zero);
    builder
        .ins()
        .brif(not_found, not_found_block, &[], found_block, &[]);

    builder.switch_to_block(not_found_block);
    builder.seal_block(not_found_block);
    let none_tag = builder.ins().iconst(cranelift::prelude::types::I32, 0);
    builder
        .ins()
        .store(MemFlags::new(), none_tag, option_ptr, 0);
    builder.ins().jump(merge_block, &[]);

    builder.switch_to_block(found_block);
    builder.seal_block(found_block);
    let some_tag = builder.ins().iconst(cranelift::prelude::types::I32, 1);
    builder
        .ins()
        .store(MemFlags::new(), some_tag, option_ptr, 0);
    builder.ins().store(MemFlags::new(), value, option_ptr, 8);
    builder.ins().jump(merge_block, &[]);

    builder.switch_to_block(merge_block);
    builder.seal_block(merge_block);

    Ok(option_ptr)
}

pub fn compile_lambda_find_last_index(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
    closure: Value,
) -> Result<Value, CodegenError> {
    let option_slot =
        builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 16, 0));
    let option_ptr = builder
        .ins()
        .stack_addr(cranelift::prelude::types::I64, option_slot, 0);

    let func_ptr = builder
        .ins()
        .load(cranelift::prelude::types::I64, MemFlags::new(), closure, 0);
    let data_ptr = builder
        .ins()
        .load(cranelift::prelude::types::I64, MemFlags::new(), closure, 8);

    let func_ref = rt_func_ref(ctx, builder, "naml_array_find_last_index")?;
    let call = builder.ins().call(func_ref, &[arr, func_ptr, data_ptr]);
    let index = builder.inst_results(call)[0];

    let found_block = builder.create_block();
    let not_found_block = builder.create_block();
    let merge_block = builder.create_block();

    let neg_one = builder
        .ins()
        .iconst(cranelift::prelude::types::I64, -1i64 as i64);
    let not_found = builder.ins().icmp(IntCC::Equal, index, neg_one);
    builder
        .ins()
        .brif(not_found, not_found_block, &[], found_block, &[]);

    builder.switch_to_block(not_found_block);
    builder.seal_block(not_found_block);
    let none_tag = builder.ins().iconst(cranelift::prelude::types::I32, 0);
    builder
        .ins()
        .store(MemFlags::new(), none_tag, option_ptr, 0);
    builder.ins().jump(merge_block, &[]);

    builder.switch_to_block(found_block);
    builder.seal_block(found_block);
    let some_tag = builder.ins().iconst(cranelift::prelude::types::I32, 1);
    builder
        .ins()
        .store(MemFlags::new(), some_tag, option_ptr, 0);
    builder.ins().store(MemFlags::new(), index, option_ptr, 8);
    builder.ins().jump(merge_block, &[]);

    builder.switch_to_block(merge_block);
    builder.seal_block(merge_block);

    Ok(option_ptr)
}

pub fn compile_lambda_scan(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
    initial: Value,
    closure: Value,
) -> Result<Value, CodegenError> {
    let func_ptr = builder
        .ins()
        .load(cranelift::prelude::types::I64, MemFlags::new(), closure, 0);
    let data_ptr = builder
        .ins()
        .load(cranelift::prelude::types::I64, MemFlags::new(), closure, 8);

    let func_ref = rt_func_ref(ctx, builder, "naml_array_scan")?;
    let call = builder
        .ins()
        .call(func_ref, &[arr, initial, func_ptr, data_ptr]);
    Ok(builder.inst_results(call)[0])
}

pub fn compile_sample(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
) -> Result<Value, CodegenError> {
    let option_slot =
        builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 16, 0));
    let option_ptr = builder
        .ins()
        .stack_addr(cranelift::prelude::types::I64, option_slot, 0);

    let found_slot =
        builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 0));
    let found_ptr = builder
        .ins()
        .stack_addr(cranelift::prelude::types::I64, found_slot, 0);

    let func_ref = rt_func_ref(ctx, builder, "naml_array_sample")?;
    let call = builder.ins().call(func_ref, &[arr, found_ptr]);
    let value = builder.inst_results(call)[0];

    let found_flag = builder.ins().load(
        cranelift::prelude::types::I64,
        MemFlags::new(),
        found_ptr,
        0,
    );

    let found_block = builder.create_block();
    let not_found_block = builder.create_block();
    let merge_block = builder.create_block();

    let zero = builder.ins().iconst(cranelift::prelude::types::I64, 0);
    let not_found = builder.ins().icmp(IntCC::Equal, found_flag, zero);
    builder
        .ins()
        .brif(not_found, not_found_block, &[], found_block, &[]);

    builder.switch_to_block(not_found_block);
    builder.seal_block(not_found_block);
    let none_tag = builder.ins().iconst(cranelift::prelude::types::I32, 0);
    builder
        .ins()
        .store(MemFlags::new(), none_tag, option_ptr, 0);
    builder.ins().jump(merge_block, &[]);

    builder.switch_to_block(found_block);
    builder.seal_block(found_block);
    let some_tag = builder.ins().iconst(cranelift::prelude::types::I32, 1);
    builder
        .ins()
        .store(MemFlags::new(), some_tag, option_ptr, 0);
    builder.ins().store(MemFlags::new(), value, option_ptr, 8);
    builder.ins().jump(merge_block, &[]);

    builder.switch_to_block(merge_block);
    builder.seal_block(merge_block);

    Ok(option_ptr)
}