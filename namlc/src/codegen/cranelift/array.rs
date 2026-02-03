use crate::ast::{Expression, Literal, LiteralExpr};
use crate::codegen::CodegenError;
use crate::codegen::cranelift::misc::ensure_i64;
use crate::codegen::cranelift::runtime::rt_func_ref;
use crate::codegen::cranelift::strings::call_string_from_cstr;
use crate::codegen::cranelift::{
    ARRAY_CAPACITY_OFFSET, ARRAY_DATA_OFFSET, ARRAY_LEN_OFFSET, CompileContext, compile_expression,
};
use cranelift::prelude::*;
use cranelift_module::Module;

pub fn compile_array_literal(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    elements: &[Expression<'_>],
) -> Result<Value, CodegenError> {
    let mut element_values = Vec::new();
    for elem in elements {
        let mut val = compile_expression(ctx, builder, elem)?;
        if matches!(
            elem,
            Expression::Literal(LiteralExpr {
                value: Literal::String(_),
                ..
            })
        ) {
            val = call_string_from_cstr(ctx, builder, val)?;
        }
        element_values.push(val);
    }
    let capacity = builder
        .ins()
        .iconst(cranelift::prelude::types::I64, elements.len() as i64);
    let arr_ptr = call_array_new(ctx, builder, capacity)?;

    for val in element_values {
        call_array_push(ctx, builder, arr_ptr, val)?;
    }

    Ok(arr_ptr)
}
/// Direct array indexing: arr[index]
/// Returns the raw value (0 if out of bounds) - used for direct indexing expressions
pub fn call_array_index(
    _ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
    index: Value,
) -> Result<Value, CodegenError> {
    let len = builder.ins().load(
        cranelift::prelude::types::I64,
        MemFlags::trusted(),
        arr,
        ARRAY_LEN_OFFSET,
    );

    let in_bounds_block = builder.create_block();
    let out_of_bounds_block = builder.create_block();
    let merge_block = builder.create_block();
    builder.append_block_param(merge_block, cranelift::prelude::types::I64);

    let is_out_of_bounds = builder
        .ins()
        .icmp(IntCC::UnsignedGreaterThanOrEqual, index, len);
    builder.ins().brif(
        is_out_of_bounds,
        out_of_bounds_block,
        &[],
        in_bounds_block,
        &[],
    );

    // Out of bounds: return 0
    builder.switch_to_block(out_of_bounds_block);
    builder.seal_block(out_of_bounds_block);
    let zero = builder.ins().iconst(cranelift::prelude::types::I64, 0);
    builder.ins().jump(merge_block, &[zero]);

    // In bounds: return the actual value
    builder.switch_to_block(in_bounds_block);
    builder.seal_block(in_bounds_block);

    let data_ptr = builder.ins().load(
        cranelift::prelude::types::I64,
        MemFlags::trusted(),
        arr,
        ARRAY_DATA_OFFSET,
    );

    let offset = builder.ins().imul_imm(index, 8);
    let elem_addr = builder.ins().iadd(data_ptr, offset);

    let value = builder.ins().load(
        cranelift::prelude::types::I64,
        MemFlags::trusted(),
        elem_addr,
        0,
    );
    builder.ins().jump(merge_block, &[value]);

    builder.switch_to_block(merge_block);
    builder.seal_block(merge_block);
    let result = builder.block_params(merge_block)[0];

    Ok(result)
}

pub fn call_array_len(
    _ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
) -> Result<Value, CodegenError> {
    let len = builder.ins().load(
        cranelift::prelude::types::I64,
        MemFlags::trusted(),
        arr,
        ARRAY_LEN_OFFSET,
    );
    Ok(len)
}

pub fn call_array_set(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
    index: Value,
    value: Value,
) -> Result<(), CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();
    let value = ensure_i64(builder, value);
    // Load len field
    let len = builder
        .ins()
        .load(ptr_type, MemFlags::trusted(), arr, ARRAY_LEN_OFFSET);

    // Create blocks for bounds check
    let in_bounds_block = builder.create_block();
    let done_block = builder.create_block();

    // Bounds check: index >= len (unsigned) means out of bounds, skip the set
    let is_out_of_bounds = builder
        .ins()
        .icmp(IntCC::UnsignedGreaterThanOrEqual, index, len);
    builder
        .ins()
        .brif(is_out_of_bounds, done_block, &[], in_bounds_block, &[]);

    // In bounds: store value to data[index]
    builder.switch_to_block(in_bounds_block);
    builder.seal_block(in_bounds_block);

    // Load data pointer
    let data_ptr = builder
        .ins()
        .load(ptr_type, MemFlags::trusted(), arr, ARRAY_DATA_OFFSET as i32);

    // Optimized offset: index << 3
    let offset = builder.ins().ishl_imm(index, 3);
    let elem_addr = builder.ins().iadd(data_ptr, offset);

    // Store element value
    builder
        .ins()
        .store(MemFlags::trusted(), value, elem_addr, 0);
    builder.ins().jump(done_block, &[]);

    // Done block
    builder.switch_to_block(done_block);
    builder.seal_block(done_block);

    Ok(())
}

pub fn call_array_new(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    capacity: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_array_new")?;
    let call = builder.ins().call(func_ref, &[capacity]);
    Ok(builder.inst_results(call)[0])
}

pub fn call_array_push(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
    value: Value,
) -> Result<(), CodegenError> {
    let value = ensure_i64(builder, value);
    let len = builder.ins().load(
        cranelift::prelude::types::I64,
        MemFlags::trusted(),
        arr,
        ARRAY_LEN_OFFSET,
    );
    let capacity = builder.ins().load(
        cranelift::prelude::types::I64,
        MemFlags::trusted(),
        arr,
        ARRAY_CAPACITY_OFFSET,
    );

    let fast_path_block = builder.create_block();
    let slow_path_block = builder.create_block();
    let done_block = builder.create_block();

    let needs_grow = builder
        .ins()
        .icmp(IntCC::UnsignedGreaterThanOrEqual, len, capacity);
    builder
        .ins()
        .brif(needs_grow, slow_path_block, &[], fast_path_block, &[]);

    builder.switch_to_block(fast_path_block);
    builder.seal_block(fast_path_block);

    let data_ptr = builder.ins().load(
        cranelift::prelude::types::I64,
        MemFlags::trusted(),
        arr,
        ARRAY_DATA_OFFSET,
    );

    // Optimized offset: len << 3
    let offset = builder.ins().ishl_imm(len, 3);
    let elem_addr = builder.ins().iadd(data_ptr, offset);

    builder
        .ins()
        .store(MemFlags::trusted().with_notrap(), value, elem_addr, 0);

    let new_len = builder.ins().iadd_imm(len, 1);
    builder.ins().store(
        MemFlags::trusted().with_notrap(),
        new_len,
        arr,
        ARRAY_LEN_OFFSET,
    );

    builder.ins().jump(done_block, &[]);

    builder.switch_to_block(slow_path_block);
    builder.seal_block(slow_path_block);

    let func_ref = rt_func_ref(ctx, builder, "naml_array_push")?;
    builder.ins().call(func_ref, &[arr, value]);
    builder.ins().jump(done_block, &[]);

    builder.switch_to_block(done_block);
    builder.seal_block(done_block);

    Ok(())
}

pub fn call_array_fill_runtime(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
    val: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_array_fill")?;
    builder.ins().call(func_ref, &[arr, val]);
    Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
}

pub fn call_array_clear_runtime(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_array_clear")?;
    builder.ins().call(func_ref, &[arr]);
    Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
}

pub fn call_array_contains_bool(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
    val: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_array_contains")?;
    let call = builder.ins().call(func_ref, &[arr, val]);
    let result = builder.inst_results(call)[0];
    Ok(builder.ins().ireduce(cranelift::prelude::types::I8, result))
}

/// Optimized array access for arr[index]! pattern
/// Directly returns value or panics - no intermediate option struct
/// Fully inlined: no function call overhead
pub fn compile_direct_array_get_or_panic(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arr: Value,
    index: Value,
) -> Result<Value, CodegenError> {
    let ptr_type = ctx.module.target_config().pointer_type();
    let len = builder
        .ins()
        .load(ptr_type, MemFlags::trusted(), arr, ARRAY_LEN_OFFSET);

    // Bounds check using branching (CPU branch predictor handles this better than traps)
    let in_bounds = builder.ins().icmp(IntCC::UnsignedLessThan, index, len);

    let ok_block = builder.create_block();
    let panic_block = builder.create_block();
    builder
        .ins()
        .brif(in_bounds, ok_block, &[], panic_block, &[]);

    builder.switch_to_block(panic_block);
    builder.seal_block(panic_block);
    let panic_func = rt_func_ref(ctx, builder, "naml_panic_unwrap")?;
    builder.ins().call(panic_func, &[]);
    builder.ins().jump(ok_block, &[]); // Never reached but needed for block closure

    builder.switch_to_block(ok_block);
    builder.seal_block(ok_block);

    // Load data pointer with notrap since we checked bounds
    let data_ptr = builder.ins().load(
        ptr_type,
        MemFlags::trusted().with_notrap(),
        arr,
        ARRAY_DATA_OFFSET,
    );

    // Optimized offset: index << 3
    let offset = builder.ins().ishl_imm(index, 3);
    let elem_addr = builder.ins().iadd(data_ptr, offset);

    // Load result directly from element address
    let val = builder.ins().load(
        cranelift::prelude::types::I64,
        MemFlags::trusted().with_notrap(),
        elem_addr,
        0,
    );

    Ok(val)
}
