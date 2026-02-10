use crate::ast::{Expression, Literal, LiteralExpr};
use crate::codegen::cranelift::expr::compile_expression;
use crate::codegen::cranelift::{CompileContext, HeapType};
use crate::codegen::CodegenError;
use cranelift::prelude::*;
use crate::codegen::cranelift::misc::ensure_i64;
use crate::codegen::cranelift::runtime::rt_func_ref;
use crate::codegen::cranelift::strings::call_string_from_cstr;

pub fn compile_map_literal(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    entries: &[crate::ast::MapEntry<'_>],
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_map_new")?;

    // Create map with capacity 16
    let capacity = builder.ins().iconst(cranelift::prelude::types::I64, 16);
    let call = builder.ins().call(func_ref, &[capacity]);
    let map_ptr = builder.inst_results(call)[0];

    // For each entry, call naml_map_set
    if !entries.is_empty() {
        let set_func_ref = rt_func_ref(ctx, builder, "naml_map_set")?;

        for entry in entries {
            // Convert string literals to NamlString pointers for map keys
            let key = if let Expression::Literal(LiteralExpr {
                value: Literal::String(_),
                ..
            }) = &entry.key
            {
                let cstr_ptr = compile_expression(ctx, builder, &entry.key)?;
                call_string_from_cstr(ctx, builder, cstr_ptr)?
            } else {
                compile_expression(ctx, builder, &entry.key)?
            };
            let value = compile_expression(ctx, builder, &entry.value)?;
            let key = ensure_i64(builder, key);
            let value = ensure_i64(builder, value);
            builder.ins().call(set_func_ref, &[map_ptr, key, value]);
        }
    }

    Ok(map_ptr)
}

pub fn call_map_set(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    map: Value,
    key: Value,
    value: Value,
) -> Result<(), CodegenError> {
    call_map_set_typed(ctx, builder, map, key, value, None)
}

pub fn call_map_set_typed(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    map: Value,
    key: Value,
    value: Value,
    value_type: Option<&HeapType>,
) -> Result<(), CodegenError> {
    // Select the appropriate set function based on value type
    // This ensures proper decref of old values when updating map entries
    let func_name = match value_type {
        Some(HeapType::String) => "naml_map_set_string",
        Some(HeapType::Array(_)) => "naml_map_set_array",
        Some(HeapType::Map(_)) => "naml_map_set_map",
        Some(HeapType::Struct(_)) => "naml_map_set_struct",
        Some(HeapType::OptionOf(_)) => "naml_map_set",
        None => "naml_map_set",
    };

    let key = ensure_i64(builder, key);
    let value = ensure_i64(builder, value);
    let func_ref = rt_func_ref(ctx, builder, func_name)?;
    builder.ins().call(func_ref, &[map, key, value]);
    Ok(())
}

pub fn call_map_contains(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    map: Value,
    key: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_map_contains")?;
    let call = builder.ins().call(func_ref, &[map, key]);
    Ok(builder.inst_results(call)[0])
}

/// Optimized map access for map[key]! pattern
/// Directly returns value or panics - no intermediate option struct
pub fn compile_direct_map_get_or_panic(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    map: Value,
    key: Value,
) -> Result<Value, CodegenError> {
    let found_block = builder.create_block();
    let panic_block = builder.create_block();
    let merge_block = builder.create_block();
    builder.append_block_param(merge_block, cranelift::prelude::types::I64);

    // Check if key exists
    let contains_ref = rt_func_ref(ctx, builder, "naml_map_contains")?;
    let contains_call = builder.ins().call(contains_ref, &[map, key]);
    let contains = builder.inst_results(contains_call)[0];

    let zero = builder.ins().iconst(cranelift::prelude::types::I64, 0);
    let key_exists = builder.ins().icmp(IntCC::NotEqual, contains, zero);
    builder
        .ins()
        .brif(key_exists, found_block, &[], panic_block, &[]);

    // Panic block: key not found
    builder.switch_to_block(panic_block);
    builder.seal_block(panic_block);
    let panic_func = rt_func_ref(ctx, builder, "naml_panic_unwrap")?;
    builder.ins().call(panic_func, &[]);
    let zero_val = builder.ins().iconst(cranelift::prelude::types::I64, 0);
    builder.ins().jump(merge_block, &[zero_val]);

    // Found block: get value directly
    builder.switch_to_block(found_block);
    builder.seal_block(found_block);
    let get_ref = rt_func_ref(ctx, builder, "naml_map_get")?;
    let get_call = builder.ins().call(get_ref, &[map, key]);
    let value = builder.inst_results(get_call)[0];
    builder.ins().jump(merge_block, &[value]);

    builder.switch_to_block(merge_block);
    builder.seal_block(merge_block);

    Ok(builder.block_params(merge_block)[0])
}