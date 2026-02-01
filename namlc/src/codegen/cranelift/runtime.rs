use cranelift::prelude::*;
use cranelift_codegen::ir::{FuncRef, Value};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_frontend::{FunctionBuilder, Variable};
use cranelift_module::Module;
use lasso::Rodeo;
use crate::ast::Expression;
use crate::codegen::CodegenError;
use crate::codegen::cranelift::CompileContext;
use crate::codegen::cranelift::heap::HeapType;
use crate::codegen::cranelift::structs::struct_has_heap_fields;
use crate::codegen::cranelift::literal::compile_string_literal;
use crate::codegen::cranelift::strings::call_string_from_cstr;

pub fn rt_func_ref(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    name: &str,
) -> Result<FuncRef, CodegenError> {
    let func_id = *ctx
        .runtime_funcs
        .get(name)
        .ok_or_else(|| CodegenError::JitCompile(format!("Unknown runtime function: {}", name)))?;
    Ok(ctx.module.declare_func_in_func(func_id, builder.func))
}

pub fn emit_incref(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    val: Value,
    heap_type: &HeapType,
) -> Result<(), CodegenError> {
    let func_name = match heap_type {
        HeapType::String => "naml_string_incref",
        HeapType::Array(_) => "naml_array_incref",
        HeapType::Map(_) => "naml_map_incref",
        HeapType::Struct(_) => "naml_struct_incref",
    };

    let func_ref = rt_func_ref(ctx, builder, func_name)?;
    let zero = builder
        .ins()
        .iconst(ctx.module.target_config().pointer_type(), 0);
    let is_null = builder.ins().icmp(IntCC::Equal, val, zero);

    let call_block = builder.create_block();
    let merge_block = builder.create_block();

    builder
        .ins()
        .brif(is_null, merge_block, &[], call_block, &[]);

    builder.switch_to_block(call_block);
    builder.seal_block(call_block);
    builder.ins().call(func_ref, &[val]);
    builder.ins().jump(merge_block, &[]);

    builder.switch_to_block(merge_block);
    builder.seal_block(merge_block);

    Ok(())
}

pub fn emit_decref(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    val: Value,
    heap_type: &HeapType,
) -> Result<(), CodegenError> {
    // Select the appropriate decref function based on element type for nested cleanup
    let func_name: String = match heap_type {
        HeapType::String => "naml_string_decref".to_string(),
        HeapType::Array(None) => "naml_array_decref".to_string(),
        HeapType::Array(Some(elem_type)) => match elem_type.as_ref() {
            HeapType::String => "naml_array_decref_strings".to_string(),
            HeapType::Array(_) => "naml_array_decref_arrays".to_string(),
            HeapType::Map(_) => "naml_array_decref_maps".to_string(),
            HeapType::Struct(_) => "naml_array_decref_structs".to_string(),
        },
        HeapType::Map(None) => "naml_map_decref".to_string(),
        HeapType::Map(Some(val_type)) => match val_type.as_ref() {
            HeapType::String => "naml_map_decref_strings".to_string(),
            HeapType::Array(_) => "naml_map_decref_arrays".to_string(),
            HeapType::Map(_) => "naml_map_decref_maps".to_string(),
            HeapType::Struct(_) => "naml_map_decref_structs".to_string(),
        },
        HeapType::Struct(None) => "naml_struct_decref".to_string(),
        HeapType::Struct(Some(struct_name)) => {
            if struct_has_heap_fields(ctx.struct_defs, struct_name) {
                format!("naml_struct_decref_{}", struct_name)
            } else {
                "naml_struct_decref".to_string()
            }
        }
    };

    let func_id = ctx
        .runtime_funcs
        .get(func_name.as_str())
        .or_else(|| ctx.functions.get(func_name.as_str()))
        .copied()
        .ok_or_else(|| {
            CodegenError::JitCompile(format!("Unknown decref function: {}", func_name))
        })?;
    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
    let zero = builder
        .ins()
        .iconst(ctx.module.target_config().pointer_type(), 0);
    let is_null = builder.ins().icmp(IntCC::Equal, val, zero);

    let call_block = builder.create_block();
    let merge_block = builder.create_block();

    builder
        .ins()
        .brif(is_null, merge_block, &[], call_block, &[]);

    builder.switch_to_block(call_block);
    builder.seal_block(call_block);
    builder.ins().call(func_ref, &[val]);
    builder.ins().jump(merge_block, &[]);

    builder.switch_to_block(merge_block);
    builder.seal_block(merge_block);

    Ok(())
}

pub fn emit_cleanup_all_vars(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    exclude_var: Option<&str>,
) -> Result<(), CodegenError> {
    let vars_to_cleanup: Vec<(String, Variable, HeapType)> = ctx
        .var_heap_types
        .iter()
        .filter_map(|(name, heap_type)| {
            if let Some(excl) = exclude_var
                && name == excl
            {
                return None;
            }
            ctx.variables
                .get(name)
                .map(|var| (name.clone(), *var, heap_type.clone()))
        })
        .collect();

    for (_, var, ref heap_type) in vars_to_cleanup {
        let val = builder.use_var(var);
        emit_decref(ctx, builder, val, heap_type)?;
    }

    Ok(())
}

pub fn get_returned_var_name(expr: &Expression, interner: &Rodeo) -> Option<String> {
    match expr {
        Expression::Identifier(ident) => Some(interner.resolve(&ident.ident.symbol).to_string()),
        _ => None,
    }
}

pub fn call_alloc_closure_data(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    size: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_alloc_closure_data")?;
    let call = builder.ins().call(func_ref, &[size]);
    Ok(builder.inst_results(call)[0])
}

pub fn emit_stack_pop(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), CodegenError> {
    let stack_pop = rt_func_ref(ctx, builder, "naml_stack_pop")?;
    builder.ins().call(stack_pop, &[]);
    Ok(())
}

pub fn emit_stack_push(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    func_name: &str,
    file_name: &str,
    line: u32,
) -> Result<(), CodegenError> {
    let stack_push = rt_func_ref(ctx, builder, "naml_stack_push")?;

    // Create function name string
    let func_name_cstr = compile_string_literal(ctx, builder, func_name)?;
    let func_name_ptr = call_string_from_cstr(ctx, builder, func_name_cstr)?;

    // Create file name string
    let file_name_cstr = compile_string_literal(ctx, builder, file_name)?;
    let file_name_ptr = call_string_from_cstr(ctx, builder, file_name_cstr)?;

    // Line number
    let line_val = builder.ins().iconst(types::I64, line as i64);

    builder.ins().call(stack_push, &[func_name_ptr, file_name_ptr, line_val]);
    Ok(())
}
