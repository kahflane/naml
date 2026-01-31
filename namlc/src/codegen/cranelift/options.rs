use cranelift::prelude::*;
use cranelift_codegen::ir::{MemFlags, StackSlotData, StackSlotKind, Value};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_frontend::FunctionBuilder;
use crate::codegen::CodegenError;
use crate::codegen::cranelift::{CompileContext, ARRAY_LEN_OFFSET};
use crate::codegen::cranelift::runtime::rt_func_ref;

pub fn compile_option_from_array_access(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
    runtime_fn: &str,
) -> Result<Value, CodegenError> {
    let option_slot =
        builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 16, 0));
    let option_ptr = builder
        .ins()
        .stack_addr(cranelift::prelude::types::I64, option_slot, 0);

    let len = builder.ins().load(
        cranelift::prelude::types::I64,
        MemFlags::trusted(),
        arr,
        ARRAY_LEN_OFFSET,
    );

    let empty_block = builder.create_block();
    let nonempty_block = builder.create_block();
    let merge_block = builder.create_block();

    let zero = builder.ins().iconst(cranelift::prelude::types::I64, 0);
    let is_empty = builder.ins().icmp(IntCC::Equal, len, zero);
    builder
        .ins()
        .brif(is_empty, empty_block, &[], nonempty_block, &[]);

    builder.switch_to_block(empty_block);
    builder.seal_block(empty_block);
    let none_tag = builder.ins().iconst(cranelift::prelude::types::I32, 0);
    builder
        .ins()
        .store(MemFlags::new(), none_tag, option_ptr, 0);
    builder.ins().jump(merge_block, &[]);

    builder.switch_to_block(nonempty_block);
    builder.seal_block(nonempty_block);
    let func_ref = rt_func_ref(ctx, builder, runtime_fn)?;
    let call = builder.ins().call(func_ref, &[arr]);
    let value = builder.inst_results(call)[0];
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

pub fn compile_option_from_minmax(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
    runtime_fn: &str,
    _is_min: bool,
) -> Result<Value, CodegenError> {
    compile_option_from_array_access(ctx, builder, arr, runtime_fn)
}

pub fn compile_option_from_array_get(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
    index: Value,
) -> Result<Value, CodegenError> {
    let option_slot =
        builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 16, 0));
    let option_ptr = builder
        .ins()
        .stack_addr(cranelift::prelude::types::I64, option_slot, 0);

    let len = builder.ins().load(
        cranelift::prelude::types::I64,
        MemFlags::trusted(),
        arr,
        ARRAY_LEN_OFFSET,
    );

    let valid_block = builder.create_block();
    let invalid_block = builder.create_block();
    let merge_block = builder.create_block();

    // Unsigned comparison: catches both negative and >= len in one check
    let in_bounds = builder.ins().icmp(IntCC::UnsignedLessThan, index, len);
    builder
        .ins()
        .brif(in_bounds, valid_block, &[], invalid_block, &[]);

    builder.switch_to_block(invalid_block);
    builder.seal_block(invalid_block);
    let none_tag = builder.ins().iconst(cranelift::prelude::types::I32, 0);
    builder
        .ins()
        .store(MemFlags::new(), none_tag, option_ptr, 0);
    builder.ins().jump(merge_block, &[]);

    builder.switch_to_block(valid_block);
    builder.seal_block(valid_block);
    let func_ref = rt_func_ref(ctx, builder, "naml_array_get")?;
    let call = builder.ins().call(func_ref, &[arr, index]);
    let value = builder.inst_results(call)[0];
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

pub fn compile_option_from_map_get(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    map: Value,
    key: Value,
) -> Result<Value, CodegenError> {
    let option_slot =
        builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 16, 0));
    let option_ptr = builder
        .ins()
        .stack_addr(cranelift::prelude::types::I64, option_slot, 0);

    let found_block = builder.create_block();
    let not_found_block = builder.create_block();
    let merge_block = builder.create_block();

    let contains_ref = rt_func_ref(ctx, builder, "naml_map_contains")?;
    let contains_call = builder.ins().call(contains_ref, &[map, key]);
    let contains = builder.inst_results(contains_call)[0];

    let zero = builder.ins().iconst(cranelift::prelude::types::I64, 0);
    let key_exists = builder.ins().icmp(IntCC::NotEqual, contains, zero);
    builder
        .ins()
        .brif(key_exists, found_block, &[], not_found_block, &[]);

    builder.switch_to_block(not_found_block);
    builder.seal_block(not_found_block);
    let none_tag = builder.ins().iconst(cranelift::prelude::types::I32, 0);
    builder
        .ins()
        .store(MemFlags::new(), none_tag, option_ptr, 0);
    builder.ins().jump(merge_block, &[]);

    builder.switch_to_block(found_block);
    builder.seal_block(found_block);
    let get_ref = rt_func_ref(ctx, builder, "naml_map_get")?;
    let get_call = builder.ins().call(get_ref, &[map, key]);
    let value = builder.inst_results(get_call)[0];
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

pub fn compile_option_from_index_of(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
    val: Value,
) -> Result<Value, CodegenError> {
    let option_slot =
        builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 16, 0));
    let option_ptr = builder
        .ins()
        .stack_addr(cranelift::prelude::types::I64, option_slot, 0);

    let func_ref = rt_func_ref(ctx, builder, "naml_array_index_of")?;
    let call = builder.ins().call(func_ref, &[arr, val]);
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

pub fn compile_option_from_last_index_of(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
    val: Value,
) -> Result<Value, CodegenError> {
    let option_slot =
        builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 16, 0));
    let option_ptr = builder
        .ins()
        .stack_addr(cranelift::prelude::types::I64, option_slot, 0);

    let func_ref = rt_func_ref(ctx, builder, "naml_array_last_index_of")?;
    let call = builder.ins().call(func_ref, &[arr, val]);
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

pub fn compile_option_from_remove_at(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
    index: Value,
    runtime_fn: &str,
) -> Result<Value, CodegenError> {
    let option_slot =
        builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 16, 0));
    let option_ptr = builder
        .ins()
        .stack_addr(cranelift::prelude::types::I64, option_slot, 0);

    let len = builder.ins().load(
        cranelift::prelude::types::I64,
        MemFlags::trusted(),
        arr,
        ARRAY_LEN_OFFSET,
    );

    let valid_block = builder.create_block();
    let invalid_block = builder.create_block();
    let merge_block = builder.create_block();

    let zero = builder.ins().iconst(cranelift::prelude::types::I64, 0);
    let index_negative = builder.ins().icmp(IntCC::SignedLessThan, index, zero);
    let index_ge_len = builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, index, len);
    let out_of_bounds = builder.ins().bor(index_negative, index_ge_len);
    builder
        .ins()
        .brif(out_of_bounds, invalid_block, &[], valid_block, &[]);

    builder.switch_to_block(invalid_block);
    builder.seal_block(invalid_block);
    let none_tag = builder.ins().iconst(cranelift::prelude::types::I32, 0);
    builder
        .ins()
        .store(MemFlags::new(), none_tag, option_ptr, 0);
    builder.ins().jump(merge_block, &[]);

    builder.switch_to_block(valid_block);
    builder.seal_block(valid_block);
    let func_ref = rt_func_ref(ctx, builder, runtime_fn)?;
    let call = builder.ins().call(func_ref, &[arr, index]);
    let value = builder.inst_results(call)[0];
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

pub fn compile_option_from_map_remove(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    map: Value,
    key: Value,
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

    let func_ref = rt_func_ref(ctx, builder, "naml_map_remove")?;
    let call = builder.ins().call(func_ref, &[map, key, found_ptr]);
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
    let was_found = builder.ins().icmp(IntCC::NotEqual, found_flag, zero);
    builder
        .ins()
        .brif(was_found, found_block, &[], not_found_block, &[]);

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

pub fn compile_option_from_map_first(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    map: Value,
    runtime_fn: &str,
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

    let func_ref = rt_func_ref(ctx, builder, runtime_fn)?;
    let call = builder.ins().call(func_ref, &[map, found_ptr]);
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
    let was_found = builder.ins().icmp(IntCC::NotEqual, found_flag, zero);
    builder
        .ins()
        .brif(was_found, found_block, &[], not_found_block, &[]);

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