use crate::ast::{Expression, Literal, LiteralExpr};
use crate::codegen::cranelift::expr::compile_expression;
use crate::codegen::cranelift::literal::compile_string_literal;
use crate::codegen::cranelift::{CompileContext};
use crate::codegen::CodegenError;
use crate::source::Spanned;
use crate::typechecker::Type;
use cranelift::prelude::*;
use cranelift_codegen::ir::Value;
use cranelift_frontend::FunctionBuilder;
use crate::codegen::cranelift::misc::ensure_i64;
use crate::codegen::cranelift::runtime::rt_func_ref;

pub fn compile_print_call(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    args: &[Expression<'_>],
    newline: bool,
) -> Result<Value, CodegenError> {
    if args.is_empty() {
        if newline {
            call_print_newline(ctx, builder)?;
        }
        return Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0));
    }

    // Check if first arg is a format string with {}
    if let Expression::Literal(LiteralExpr {
                                   value: Literal::String(spur),
                                   ..
                               }) = &args[0]
    {
        let format_str = ctx.interner.resolve(spur);
        if format_str.contains("{}") {
            // Format string mode
            let mut arg_idx = 1;
            let mut last_end = 0;

            for (start, _) in format_str.match_indices("{}") {
                // Print literal part before placeholder
                if start > last_end {
                    let literal_part = &format_str[last_end..start];
                    let ptr = compile_string_literal(ctx, builder, literal_part)?;
                    call_print_str(ctx, builder, ptr)?;
                }

                // Print the argument
                if arg_idx < args.len() {
                    let arg = &args[arg_idx];
                    print_arg(ctx, builder, arg)?;
                    arg_idx += 1;
                }

                last_end = start + 2;
            }

            // Print remaining literal after last placeholder
            if last_end < format_str.len() {
                let remaining = &format_str[last_end..];
                let ptr = compile_string_literal(ctx, builder, remaining)?;
                call_print_str(ctx, builder, ptr)?;
            }

            if newline {
                call_print_newline(ctx, builder)?;
            }

            return Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0));
        }
    }

    // Original behavior for non-format strings
    for (i, arg) in args.iter().enumerate() {
        print_arg(ctx, builder, arg)?;

        if i < args.len() - 1 {
            let space = compile_string_literal(ctx, builder, " ")?;
            call_print_str(ctx, builder, space)?;
        }
    }

    if newline {
        call_print_newline(ctx, builder)?;
    }

    Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
}

pub fn print_arg(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    arg: &Expression<'_>,
) -> Result<(), CodegenError> {
    match arg {
        Expression::Literal(LiteralExpr {
                                value: Literal::String(spur),
                                ..
                            }) => {
            let s = ctx.interner.resolve(spur);
            let ptr = compile_string_literal(ctx, builder, s)?;
            call_print_str(ctx, builder, ptr)?;
        }
        Expression::Literal(LiteralExpr {
                                value: Literal::Int(n),
                                ..
                            }) => {
            let val = builder.ins().iconst(cranelift::prelude::types::I64, *n);
            call_print_int(ctx, builder, val)?;
        }
        Expression::Literal(LiteralExpr {
                                value: Literal::Float(f),
                                ..
                            }) => {
            let val = builder.ins().f64const(*f);
            call_print_float(ctx, builder, val)?;
        }
        Expression::Literal(LiteralExpr {
                                value: Literal::Bool(b),
                                ..
                            }) => {
            let val = builder
                .ins()
                .iconst(cranelift::prelude::types::I64, if *b { 1 } else { 0 });
            call_print_bool(ctx, builder, val)?;
        }
        _ => {
            let val = compile_expression(ctx, builder, arg)?;
            // Check type from annotations to call appropriate print function
            let expr_type = ctx.annotations.get_type(arg.span());
            match expr_type {
                Some(Type::String) => {
                    // String variables now hold NamlString* (boxed strings)
                    call_print_naml_string(ctx, builder, val)?;
                }
                Some(Type::Float) => {
                    call_print_float(ctx, builder, val)?;
                }
                Some(Type::Bool) => {
                    call_print_bool(ctx, builder, val)?;
                }
                Some(Type::Array(elem_type)) => {
                    let print_fn = if matches!(elem_type.as_ref(), Type::String) {
                        "naml_array_print_strings"
                    } else {
                        "naml_array_print"
                    };
                    let func_ref = rt_func_ref(ctx, builder, print_fn)?;
                    builder.ins().call(func_ref, &[val]);
                }
                Some(Type::Map(_, val_type)) => {
                    let print_fn = match val_type.as_ref() {
                        Type::String => "naml_map_print_string_values",
                        Type::Float => "naml_map_print_float_values",
                        Type::Bool => "naml_map_print_bool_values",
                        _ => "naml_map_print",
                    };
                    let func_ref = rt_func_ref(ctx, builder, print_fn)?;
                    builder.ins().call(func_ref, &[val]);
                }
                _ => {
                    // Default: check Cranelift value type for F64, otherwise int
                    let val_type = builder.func.dfg.value_type(val);
                    if val_type == cranelift::prelude::types::F64 {
                        call_print_float(ctx, builder, val)?;
                    } else {
                        call_print_int(ctx, builder, val)?;
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn call_print_int(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    val: Value,
) -> Result<(), CodegenError> {
    let val = ensure_i64(builder, val);
    let func_ref = rt_func_ref(ctx, builder, "naml_print_int")?;
    builder.ins().call(func_ref, &[val]);
    Ok(())
}

pub fn call_print_float(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    val: Value,
) -> Result<(), CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_print_float")?;
    builder.ins().call(func_ref, &[val]);
    Ok(())
}

pub fn call_print_bool(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    val: Value,
) -> Result<(), CodegenError> {
    let val = ensure_i64(builder, val);
    let func_ref = rt_func_ref(ctx, builder, "naml_print_bool")?;
    builder.ins().call(func_ref, &[val]);
    Ok(())
}

pub fn call_print_str(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    ptr: Value,
) -> Result<(), CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_print_str")?;
    builder.ins().call(func_ref, &[ptr]);
    Ok(())
}

pub fn call_print_naml_string(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    ptr: Value,
) -> Result<(), CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_string_print")?;
    builder.ins().call(func_ref, &[ptr]);
    Ok(())
}

pub fn call_print_newline(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), CodegenError> {
    let func_ref = rt_func_ref(ctx, builder, "naml_print_newline")?;
    builder.ins().call(func_ref, &[]);
    Ok(())
}