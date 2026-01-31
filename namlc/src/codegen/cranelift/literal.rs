use cranelift::prelude::*;
use cranelift_module::{DataDescription, Module};

use crate::ast::Literal;
use crate::codegen::cranelift::CompileContext;
use crate::codegen::CodegenError;

pub fn compile_literal(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    lit: &Literal,
) -> Result<Value, CodegenError> {
    match lit {
        Literal::Int(n) => Ok(builder.ins().iconst(cranelift::prelude::types::I64, *n)),
        Literal::UInt(n) => Ok(builder
            .ins()
            .iconst(cranelift::prelude::types::I64, *n as i64)),
        Literal::Float(f) => Ok(builder.ins().f64const(*f)),
        Literal::Bool(b) => {
            let val = if *b { 1i64 } else { 0i64 };
            Ok(builder.ins().iconst(cranelift::prelude::types::I8, val))
        }
        Literal::String(spur) => {
            let s = ctx.interner.resolve(spur);
            compile_string_literal(ctx, builder, s)
        }
        Literal::None => {
            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                16,
                0,
            ));
            let slot_addr = builder
                .ins()
                .stack_addr(cranelift::prelude::types::I64, slot, 0);

            let tag = builder.ins().iconst(cranelift::prelude::types::I32, 0);
            builder.ins().store(MemFlags::new(), tag, slot_addr, 0);

            Ok(slot_addr)
        }
        _ => Err(CodegenError::Unsupported(format!(
            "Literal type not yet implemented: {:?}",
            std::mem::discriminant(lit)
        ))),
    }
}

pub fn compile_string_literal(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    s: &str,
) -> Result<Value, CodegenError> {
    let mut bytes = s.as_bytes().to_vec();
    bytes.push(0);

    let data_id = ctx
        .module
        .declare_anonymous_data(false, false)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to declare string data: {}", e)))?;

    let mut data_description = DataDescription::new();
    data_description.define(bytes.into_boxed_slice());

    ctx.module
        .define_data(data_id, &data_description)
        .map_err(|e| CodegenError::JitCompile(format!("Failed to define string data: {}", e)))?;

    let global_value = ctx.module.declare_data_in_func(data_id, builder.func);
    let ptr = builder
        .ins()
        .global_value(ctx.module.target_config().pointer_type(), global_value);

    Ok(ptr)
}