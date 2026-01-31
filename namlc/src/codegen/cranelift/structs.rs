use std::collections::HashMap;
use cranelift::prelude::*;
use cranelift_codegen::ir::Value;
use cranelift_frontend::FunctionBuilder;
use crate::codegen::CodegenError;
use crate::codegen::cranelift::{CompileContext, StructDef};
use crate::codegen::cranelift::misc::ensure_i64;
use crate::codegen::cranelift::runtime::rt_func_ref;

pub fn call_struct_new(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    type_id: Value,
    field_count: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_struct_new")?;
    let call = builder.ins().call(func_ref, &[type_id, field_count]);
    Ok(builder.inst_results(call)[0])
}

pub fn call_struct_get_field(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    struct_ptr: Value,
    field_index: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_struct_get_field")?;
    let call = builder.ins().call(func_ref, &[struct_ptr, field_index]);
    Ok(builder.inst_results(call)[0])
}

pub fn call_struct_set_field(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    struct_ptr: Value,
    field_index: Value,
    value: Value,
) -> Result<(), CodegenError> {
    let value = ensure_i64(builder, value);
    let func_ref = rt_func_ref(ctx, builder, "naml_struct_set_field")?;
    builder
        .ins()
        .call(func_ref, &[struct_ptr, field_index, value]);
    Ok(())
}

pub fn struct_has_heap_fields(struct_defs: &HashMap<String, StructDef>, struct_name: &str) -> bool {
    if let Some(def) = struct_defs.get(struct_name) {
        def.field_heap_types.iter().any(|ht| ht.is_some())
    } else {
        false
    }
}