use cranelift::prelude::*;
use cranelift_codegen::ir::Value;
use cranelift_frontend::FunctionBuilder;
use crate::ast::{Expression, Literal, LiteralExpr};
use crate::codegen::CodegenError;
use crate::codegen::cranelift::{CompileContext};
use crate::codegen::cranelift::expr::compile_expression;
use crate::codegen::cranelift::literal::compile_string_literal;
use crate::codegen::cranelift::misc::ensure_i64;
use crate::codegen::cranelift::runtime::rt_func_ref;
use crate::source::Spanned;

pub fn call_string_equals(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    a: Value,
    b: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_string_eq")?;
    let call = builder.ins().call(func_ref, &[a, b]);
    Ok(builder.inst_results(call)[0])
}

pub fn call_int_to_string(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    value: Value,
) -> Result<Value, CodegenError> {
    let value = ensure_i64(builder, value);
    let func_ref = rt_func_ref(ctx, builder, "naml_int_to_string")?;
    let call = builder.ins().call(func_ref, &[value]);
    Ok(builder.inst_results(call)[0])
}

pub fn call_float_to_string(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    value: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_float_to_string")?;
    let call = builder.ins().call(func_ref, &[value]);
    Ok(builder.inst_results(call)[0])
}

pub fn call_string_to_int(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    value: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_string_to_int")?;
    let call = builder.ins().call(func_ref, &[value]);
    Ok(builder.inst_results(call)[0])
}

pub fn call_string_to_float(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    value: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_string_to_float")?;
    let call = builder.ins().call(func_ref, &[value]);
    Ok(builder.inst_results(call)[0])
}

pub fn call_string_char_len(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    str_ptr: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_string_char_len")?;
    let call = builder.ins().call(func_ref, &[str_ptr]);
    Ok(builder.inst_results(call)[0])
}

pub fn call_string_char_at(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    str_ptr: Value,
    index: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_string_char_at")?;
    let call = builder.ins().call(func_ref, &[str_ptr, index]);
    Ok(builder.inst_results(call)[0])
}

pub fn call_string_to_bytes(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    str_ptr: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_string_to_bytes")?;
    let call = builder.ins().call(func_ref, &[str_ptr]);
    Ok(builder.inst_results(call)[0])
}

pub fn call_bytes_to_string(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    bytes_ptr: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_bytes_to_string")?;
    let call = builder.ins().call(func_ref, &[bytes_ptr]);
    Ok(builder.inst_results(call)[0])
}


pub fn call_string_from_cstr(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    cstr_ptr: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_string_from_cstr")?;
    let call = builder.ins().call(func_ref, &[cstr_ptr]);
    Ok(builder.inst_results(call)[0])
}

/// Ensure a value is a NamlString* (convert string literals if needed)
pub fn ensure_naml_string(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    value: Value,
    expr: &Expression,
) -> Result<Value, CodegenError> {
    if matches!(
        expr,
        Expression::Literal(LiteralExpr {
            value: Literal::String(_),
            ..
        })
    ) {
        call_string_from_cstr(ctx, builder, value)
    } else {
        Ok(value)
    }
}

pub fn arg_to_naml_string(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arg: &Expression<'_>,
) -> Result<Value, CodegenError> {
    match arg {
        Expression::Literal(LiteralExpr {
                                value: Literal::String(spur),
                                ..
                            }) => {
            let s = ctx.interner.resolve(spur);
            let ptr = compile_string_literal(ctx, builder, s)?;
            call_string_from_cstr(ctx, builder, ptr)
        }
        Expression::Literal(LiteralExpr {
                                value: Literal::Int(n),
                                ..
                            }) => {
            let val = builder.ins().iconst(cranelift::prelude::types::I64, *n);
            call_int_to_string(ctx, builder, val)
        }
        Expression::Literal(LiteralExpr {
                                value: Literal::Float(f),
                                ..
                            }) => {
            let val = builder.ins().f64const(*f);
            call_float_to_string(ctx, builder, val)
        }
        _ => {
            let val = compile_expression(ctx, builder, arg)?;
            let expr_type = ctx.annotations.get_type(arg.span());
            match expr_type {
                Some(crate::typechecker::Type::String) => Ok(val),
                Some(crate::typechecker::Type::Float) => call_float_to_string(ctx, builder, val),
                _ => {
                    let val_type = builder.func.dfg.value_type(val);
                    if val_type == cranelift::prelude::types::F64 {
                        call_float_to_string(ctx, builder, val)
                    } else {
                        call_int_to_string(ctx, builder, val)
                    }
                }
            }
        }
    }
}

pub fn call_string_concat(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    a: Value,
    b: Value,
) -> Result<Value, CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_string_concat")?;
    let call = builder.ins().call(func_ref, &[a, b]);
    Ok(builder.inst_results(call)[0])
}

pub fn build_message_string(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    args: &[Expression<'_>],
) -> Result<Value, CodegenError> {
    if args.is_empty() {
        let ptr = compile_string_literal(ctx, builder, "")?;
        return call_string_from_cstr(ctx, builder, ptr);
    }

    if let Expression::Literal(LiteralExpr {
                                   value: Literal::String(spur),
                                   ..
                               }) = &args[0]
    {
        let format_str = ctx.interner.resolve(spur).to_string();
        if format_str.contains("{}") {
            let mut result: Option<Value> = None;
            let mut arg_idx = 1;
            let mut last_end = 0;

            for (start, _) in format_str.match_indices("{}") {
                if start > last_end {
                    let literal_part = &format_str[last_end..start];
                    let ptr = compile_string_literal(ctx, builder, literal_part)?;
                    let part = call_string_from_cstr(ctx, builder, ptr)?;
                    result = Some(match result {
                        Some(acc) => call_string_concat(ctx, builder, acc, part)?,
                        None => part,
                    });
                }

                if arg_idx < args.len() {
                    let part = arg_to_naml_string(ctx, builder, &args[arg_idx])?;
                    arg_idx += 1;
                    result = Some(match result {
                        Some(acc) => call_string_concat(ctx, builder, acc, part)?,
                        None => part,
                    });
                }

                last_end = start + 2;
            }

            if last_end < format_str.len() {
                let remaining = &format_str[last_end..];
                let ptr = compile_string_literal(ctx, builder, remaining)?;
                let part = call_string_from_cstr(ctx, builder, ptr)?;
                result = Some(match result {
                    Some(acc) => call_string_concat(ctx, builder, acc, part)?,
                    None => part,
                });
            }

            return Ok(result.unwrap_or_else(|| {
                let ptr = compile_string_literal(ctx, builder, "").unwrap();
                call_string_from_cstr(ctx, builder, ptr).unwrap()
            }));
        }
    }

    let mut result: Option<Value> = None;
    for (i, arg) in args.iter().enumerate() {
        let part = arg_to_naml_string(ctx, builder, arg)?;
        if i > 0 {
            let space_ptr = compile_string_literal(ctx, builder, " ")?;
            let space = call_string_from_cstr(ctx, builder, space_ptr)?;
            result = Some(call_string_concat(ctx, builder, result.unwrap(), space)?);
        }
        result = Some(match result {
            Some(acc) => call_string_concat(ctx, builder, acc, part)?,
            None => part,
        });
    }

    Ok(result.unwrap())
}