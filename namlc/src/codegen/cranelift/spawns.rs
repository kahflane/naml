use cranelift::prelude::*;
use cranelift_codegen::ir::Value;
use cranelift_frontend::FunctionBuilder;
use crate::codegen::CodegenError;
use crate::codegen::cranelift::{CompileContext};
use crate::codegen::cranelift::runtime::rt_func_ref;

pub fn call_spawn_closure(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    func_addr: Value,
    data: Value,
    data_size: Value,
) -> Result<(), CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_spawn_closure")?;
    builder.ins().call(func_ref, &[func_addr, data, data_size]);
    Ok(())
}