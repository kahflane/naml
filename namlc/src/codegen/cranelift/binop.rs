use cranelift::prelude::*;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::Value;
use cranelift_frontend::FunctionBuilder;
use crate::ast::{BinaryOp, UnaryOp};
use crate::codegen::CodegenError;

pub fn compile_binary_op(
    builder: &mut FunctionBuilder<'_>,
    op: &BinaryOp,
    lhs: Value,
    rhs: Value,
) -> Result<Value, CodegenError> {
    // Check if operands are floats
    let lhs_type = builder.func.dfg.value_type(lhs);
    let is_float = lhs_type == cranelift::prelude::types::F64;

    let result = match op {
        BinaryOp::Add => {
            if is_float {
                builder.ins().fadd(lhs, rhs)
            } else {
                builder.ins().iadd(lhs, rhs)
            }
        }
        BinaryOp::Sub => {
            if is_float {
                builder.ins().fsub(lhs, rhs)
            } else {
                builder.ins().isub(lhs, rhs)
            }
        }
        BinaryOp::Mul => {
            if is_float {
                builder.ins().fmul(lhs, rhs)
            } else {
                builder.ins().imul(lhs, rhs)
            }
        }
        BinaryOp::Div => {
            if is_float {
                builder.ins().fdiv(lhs, rhs)
            } else {
                builder.ins().sdiv(lhs, rhs)
            }
        }
        BinaryOp::Mod => {
            if is_float {
                // Floating point remainder - fmod equivalent
                // a % b = a - (trunc(a / b) * b)
                let div = builder.ins().fdiv(lhs, rhs);
                let trunc = builder.ins().trunc(div);
                let prod = builder.ins().fmul(trunc, rhs);
                builder.ins().fsub(lhs, prod)
            } else {
                builder.ins().srem(lhs, rhs)
            }
        }

        BinaryOp::Eq => {
            if is_float {
                builder.ins().fcmp(FloatCC::Equal, lhs, rhs)
            } else {
                builder.ins().icmp(IntCC::Equal, lhs, rhs)
            }
        }
        BinaryOp::NotEq => {
            if is_float {
                builder.ins().fcmp(FloatCC::NotEqual, lhs, rhs)
            } else {
                builder.ins().icmp(IntCC::NotEqual, lhs, rhs)
            }
        }
        BinaryOp::Lt => {
            if is_float {
                builder.ins().fcmp(FloatCC::LessThan, lhs, rhs)
            } else {
                builder.ins().icmp(IntCC::SignedLessThan, lhs, rhs)
            }
        }
        BinaryOp::LtEq => {
            if is_float {
                builder.ins().fcmp(FloatCC::LessThanOrEqual, lhs, rhs)
            } else {
                builder.ins().icmp(IntCC::SignedLessThanOrEqual, lhs, rhs)
            }
        }
        BinaryOp::Gt => {
            if is_float {
                builder.ins().fcmp(FloatCC::GreaterThan, lhs, rhs)
            } else {
                builder.ins().icmp(IntCC::SignedGreaterThan, lhs, rhs)
            }
        }
        BinaryOp::GtEq => {
            if is_float {
                builder.ins().fcmp(FloatCC::GreaterThanOrEqual, lhs, rhs)
            } else {
                builder
                    .ins()
                    .icmp(IntCC::SignedGreaterThanOrEqual, lhs, rhs)
            }
        }

        BinaryOp::And => builder.ins().band(lhs, rhs),
        BinaryOp::Or => builder.ins().bor(lhs, rhs),

        BinaryOp::BitAnd => builder.ins().band(lhs, rhs),
        BinaryOp::BitOr => builder.ins().bor(lhs, rhs),
        BinaryOp::BitXor => builder.ins().bxor(lhs, rhs),
        BinaryOp::Shl => builder.ins().ishl(lhs, rhs),
        BinaryOp::Shr => builder.ins().sshr(lhs, rhs),

        _ => {
            return Err(CodegenError::Unsupported(format!(
                "Binary operator not yet implemented: {:?}",
                op
            )));
        }
    };

    Ok(result)
}

pub fn compile_unary_op(
    builder: &mut FunctionBuilder<'_>,
    op: &UnaryOp,
    operand: Value,
) -> Result<Value, CodegenError> {
    let result = match op {
        UnaryOp::Neg => {
            let ty = builder.func.dfg.value_type(operand);
            if ty == cranelift::prelude::types::F64 {
                builder.ins().fneg(operand)
            } else {
                builder.ins().ineg(operand)
            }
        }
        UnaryOp::Not => {
            let one = builder.ins().iconst(cranelift::prelude::types::I8, 1);
            builder.ins().bxor(operand, one)
        }
        UnaryOp::BitNot => builder.ins().bnot(operand),
    };

    Ok(result)
}
