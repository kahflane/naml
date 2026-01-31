use crate::ast::Expression;
use crate::codegen::cranelift::expr::compile_expression;
use crate::codegen::cranelift::map::{call_map_contains, call_map_set};
use crate::codegen::cranelift::CompileContext;
use crate::codegen::CodegenError;
use crate::source::Spanned;
use crate::typechecker::Type;
use cranelift::prelude::*;
use cranelift_module::Module;

pub fn compile_method_call(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    receiver: &Expression<'_>,
    method_name: &str,
    args: &[Expression<'_>],
) -> Result<Value, CodegenError> {
    let recv = compile_expression(ctx, builder, receiver)?;

    // Check for user-defined struct methods FIRST
    let receiver_type = ctx.annotations.get_type(receiver.span());
    if let Some(Type::Struct(s)) = receiver_type {
        let type_name = ctx.interner.resolve(&s.name).to_string();
        let full_name = format!("{}_{}", type_name, method_name);
        if let Some(&func_id) = ctx.functions.get(&full_name) {
            let ptr_type = ctx.module.target_config().pointer_type();
            let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);

            // Compile arguments
            let mut call_args = vec![recv];
            for arg in args {
                call_args.push(compile_expression(ctx, builder, arg)?);
            }

            let call = builder.ins().call(func_ref, &call_args);
            let results = builder.inst_results(call);
            if results.is_empty() {
                return Ok(builder.ins().iconst(ptr_type, 0));
            } else {
                return Ok(results[0]);
            }
        }
    }

    match method_name {
        // Map methods (maps still use method syntax)
        "contains" => {
            if args.is_empty() {
                return Err(CodegenError::JitCompile(
                    "contains requires a key argument".to_string(),
                ));
            }
            let key = compile_expression(ctx, builder, &args[0])?;
            call_map_contains(ctx, builder, recv, key)
        }
        "set" => {
            if args.len() < 2 {
                return Err(CodegenError::JitCompile(
                    "set requires key and value arguments".to_string(),
                ));
            }
            let key = compile_expression(ctx, builder, &args[0])?;
            let value = compile_expression(ctx, builder, &args[1])?;
            call_map_set(ctx, builder, recv, key, value)?;
            Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
        }
        // Exception method (exceptions still use method syntax)
        "message" => {
            let receiver_type = ctx.annotations.get_type(receiver.span());
            if matches!(receiver_type, Some(Type::Exception(_))) {
                // Exception message is stored at offset 0
                let message_ptr =
                    builder
                        .ins()
                        .load(cranelift::prelude::types::I64, MemFlags::new(), recv, 0);
                Ok(message_ptr)
            } else {
                Err(CodegenError::JitCompile(
                    "message() is only available on exception types".to_string(),
                ))
            }
        }
        _ => {
            // Try to look up user-defined method
            let receiver_type = ctx.annotations.get_type(receiver.span());
            let type_name = match receiver_type {
                Some(Type::Struct(s)) => Some(ctx.interner.resolve(&s.name).to_string()),
                Some(Type::Generic(name, type_args)) => {
                    let name_str = ctx.interner.resolve(name).to_string();
                    // Check if this is a bare type parameter (no type args)
                    // If so, look up the concrete type from substitutions
                    if type_args.is_empty() {
                        if let Some(concrete_type) = ctx.type_substitutions.get(&name_str) {
                            Some(concrete_type.clone())
                        } else {
                            Some(name_str)
                        }
                    } else {
                        Some(name_str)
                    }
                }
                _ => None,
            };

            if let Some(type_name) = type_name {
                let full_name = format!("{}_{}", type_name, method_name);
                if let Some(&func_id) = ctx.functions.get(&full_name) {
                    let ptr_type = ctx.module.target_config().pointer_type();
                    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);

                    // Compile arguments
                    let mut call_args = vec![recv];
                    for arg in args {
                        call_args.push(compile_expression(ctx, builder, arg)?);
                    }

                    let call = builder.ins().call(func_ref, &call_args);
                    let results = builder.inst_results(call);
                    if results.is_empty() {
                        return Ok(builder.ins().iconst(ptr_type, 0));
                    } else {
                        return Ok(results[0]);
                    }
                }
            }

            Err(CodegenError::Unsupported(format!(
                "Unknown method: {}",
                method_name
            )))
        }
    }
}