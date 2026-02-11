use std::collections::HashMap;
use cranelift::prelude::*;
use cranelift_codegen::ir::{FuncRef, Value};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::{FuncId, Module};
use crate::codegen::CodegenError;
use crate::codegen::cranelift::{CompileContext, StructDef};
use crate::codegen::cranelift::runtime::rt_func_ref;

fn get_tls_func_ref(
    module: &mut dyn Module,
    runtime_funcs: &HashMap<String, FuncId>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<FuncRef, CodegenError> {
    let func_id = *runtime_funcs
        .get("naml_arena_get_tls_ptr")
        .ok_or_else(|| CodegenError::JitCompile("Unknown runtime function: naml_arena_get_tls_ptr".to_string()))?;
    Ok(module.declare_func_in_func(func_id, builder.func))
}

pub fn call_struct_new(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    type_id: Value,
    field_count: Value,
) -> Result<Value, CodegenError> {
    // Inline allocation: call arena_alloc + write header fields directly.
    // Avoids function call overhead of naml_struct_new wrapper.
    // Alloc size: sizeof(NamlStruct) + field_count * 8 = 24 + field_count * 8
    let fc_i64 = builder.ins().uextend(cranelift::prelude::types::I64, field_count);
    let field_bytes = builder.ins().imul_imm(fc_i64, 8);
    let size = builder.ins().iadd_imm(field_bytes, 24); // 24 = HeapHeader(16) + type_id(4) + field_count(4)

    let alloc_ref = rt_func_ref(ctx, builder, "naml_arena_alloc")?;
    let call = builder.ins().call(alloc_ref, &[size]);
    let ptr = builder.inst_results(call)[0];

    // Write refcount = 1 at offset 0
    let one = builder.ins().iconst(cranelift::prelude::types::I64, 1);
    builder.ins().store(MemFlags::new(), one, ptr, 0);

    // Write tag = 2 (HeapTag::Struct) at offset 8
    let tag = builder.ins().iconst(cranelift::prelude::types::I8, 2);
    builder.ins().store(MemFlags::new(), tag, ptr, 8);

    // Write type_id at offset 16
    builder.ins().store(MemFlags::new(), type_id, ptr, 16);

    // Write field_count at offset 20
    builder.ins().store(MemFlags::new(), field_count, ptr, 20);

    Ok(ptr)
}

pub fn struct_has_heap_fields(struct_defs: &HashMap<lasso::Spur, StructDef>, struct_name: &lasso::Spur) -> bool {
    if let Some(def) = struct_defs.get(struct_name) {
        def.field_heap_types.iter().any(|ht| ht.is_some())
    } else {
        false
    }
}

fn arena_size_class(alloc_size: usize) -> (usize, i32) {
    let idx = if alloc_size <= 32 { 0 }
        else if alloc_size <= 48 { 1 }
        else if alloc_size <= 64 { 2 }
        else if alloc_size <= 80 { 3 }
        else if alloc_size <= 96 { 4 }
        else if alloc_size <= 128 { 5 }
        else if alloc_size <= 192 { 6 }
        else if alloc_size <= 256 { 7 }
        else { 8 };
    let class_size = [32, 48, 64, 80, 96, 128, 192, 256, 512][idx];
    let aligned = (class_size + 7) & !7;
    let fl_offset = (24 + idx * 8) as i32;
    (aligned, fl_offset)
}

pub fn emit_inline_arena_alloc(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    alloc_size: usize,
) -> Result<Value, CodegenError> {
    if alloc_size > 512 {
        let size_val = builder.ins().iconst(cranelift::prelude::types::I64, alloc_size as i64);
        let alloc_ref = rt_func_ref(ctx, builder, "naml_arena_alloc")?;
        let call = builder.ins().call(alloc_ref, &[size_val]);
        return Ok(builder.inst_results(call)[0]);
    }

    let (aligned, fl_offset) = arena_size_class(alloc_size);
    let ptr_ty = cranelift::prelude::types::I64;
    let zero = builder.ins().iconst(ptr_ty, 0);

    let tls_ref = rt_func_ref(ctx, builder, "naml_arena_get_tls_ptr")?;
    let tls_call = builder.ins().call(tls_ref, &[]);
    let arena_ptr = builder.inst_results(tls_call)[0];

    let done_block = builder.create_block();
    builder.append_block_param(done_block, ptr_ty);

    let free_head = builder.ins().load(ptr_ty, MemFlags::new(), arena_ptr, fl_offset);
    let has_free = builder.ins().icmp(IntCC::NotEqual, free_head, zero);

    let freelist_block = builder.create_block();
    let bump_block = builder.create_block();
    builder.ins().brif(has_free, freelist_block, &[], bump_block, &[]);

    builder.switch_to_block(freelist_block);
    builder.seal_block(freelist_block);
    let next = builder.ins().load(ptr_ty, MemFlags::new(), free_head, 0);
    builder.ins().store(MemFlags::new(), next, arena_ptr, fl_offset);
    builder.ins().jump(done_block, &[free_head]);

    builder.switch_to_block(bump_block);
    builder.seal_block(bump_block);
    let bump_ptr = builder.ins().load(ptr_ty, MemFlags::new(), arena_ptr, 0);
    let new_ptr = builder.ins().iadd_imm(bump_ptr, aligned as i64);
    let bump_end = builder.ins().load(ptr_ty, MemFlags::new(), arena_ptr, 8);
    let fits = builder.ins().icmp(IntCC::UnsignedLessThanOrEqual, new_ptr, bump_end);

    let bump_ok_block = builder.create_block();
    let slow_block = builder.create_block();
    builder.ins().brif(fits, bump_ok_block, &[], slow_block, &[]);

    builder.switch_to_block(bump_ok_block);
    builder.seal_block(bump_ok_block);
    builder.ins().store(MemFlags::new(), new_ptr, arena_ptr, 0);
    builder.ins().jump(done_block, &[bump_ptr]);

    builder.switch_to_block(slow_block);
    builder.seal_block(slow_block);
    let size_val = builder.ins().iconst(ptr_ty, alloc_size as i64);
    let alloc_ref = rt_func_ref(ctx, builder, "naml_arena_alloc")?;
    let call = builder.ins().call(alloc_ref, &[size_val]);
    let slow_result = builder.inst_results(call)[0];
    builder.ins().jump(done_block, &[slow_result]);

    builder.switch_to_block(done_block);
    builder.seal_block(done_block);
    Ok(builder.block_params(done_block)[0])
}

pub fn emit_inline_arena_free(
    module: &mut dyn Module,
    runtime_funcs: &HashMap<String, FuncId>,
    builder: &mut FunctionBuilder<'_>,
    ptr: Value,
    alloc_size: usize,
) -> Result<(), CodegenError> {
    if alloc_size > 512 {
        return Ok(());
    }

    let (_, fl_offset) = arena_size_class(alloc_size);
    let ptr_ty = cranelift::prelude::types::I64;

    let tls_ref = get_tls_func_ref(module, runtime_funcs, builder)?;
    let tls_call = builder.ins().call(tls_ref, &[]);
    let arena_ptr = builder.inst_results(tls_call)[0];

    let old_head = builder.ins().load(ptr_ty, MemFlags::new(), arena_ptr, fl_offset);
    builder.ins().store(MemFlags::new(), old_head, ptr, 0);
    builder.ins().store(MemFlags::new(), ptr, arena_ptr, fl_offset);
    Ok(())
}