///
/// Expression Code Generation
///
/// Converts naml expressions to Rust expressions:
/// - Literals (int, float, string, bool)
/// - Binary operations (+, -, *, /, ==, etc.)
/// - Unary operations (-, !)
/// - Function calls
/// - Identifiers
/// - Field access
/// - Index access
///

use crate::ast::{BinaryOp, Expression, Literal, LiteralExpr, UnaryOp};
use crate::codegen::CodegenError;

use super::RustGenerator;

pub fn emit_expression(g: &mut RustGenerator, expr: &Expression<'_>) -> Result<(), CodegenError> {
    match expr {
        Expression::Literal(lit) => emit_literal(g, lit),

        Expression::Identifier(ident) => {
            let name = g.interner().resolve(&ident.ident.symbol).to_string();
            g.write(&name);
            Ok(())
        }

        Expression::Binary(bin) => {
            g.write("(");
            emit_expression(g, bin.left)?;
            g.write(" ");
            g.write(binary_op_to_rust(&bin.op));
            g.write(" ");
            emit_expression(g, bin.right)?;
            g.write(")");
            Ok(())
        }

        Expression::Unary(un) => {
            g.write(unary_op_to_rust(&un.op));
            emit_expression(g, un.operand)?;
            Ok(())
        }

        Expression::Call(call) => {
            if let Expression::Identifier(ident) = call.callee {
                let name = g.interner().resolve(&ident.ident.symbol).to_string();

                match name.as_str() {
                    "print" => {
                        g.write("print!(\"{}\"");
                        for arg in &call.args {
                            g.write(", ");
                            emit_expression(g, arg)?;
                        }
                        g.write(")");
                    }
                    "println" => {
                        if call.args.is_empty() {
                            g.write("println!()");
                        } else {
                            g.write("println!(\"{}\"");
                            for arg in &call.args {
                                g.write(", ");
                                emit_expression(g, arg)?;
                            }
                            g.write(")");
                        }
                    }
                    "printf" => {
                        if let Some(Expression::Literal(LiteralExpr {
                            value: Literal::String(fmt_spur),
                            ..
                        })) = call.args.first()
                        {
                            let fmt = g.interner().resolve(fmt_spur);
                            let rust_fmt = fmt.replace("{}", "{}");
                            g.write(&format!("println!(\"{}\"", rust_fmt));
                            for arg in call.args.iter().skip(1) {
                                g.write(", ");
                                emit_expression(g, arg)?;
                            }
                            g.write(")");
                        } else {
                            g.write("println!(\"{}\"");
                            for arg in &call.args {
                                g.write(", ");
                                emit_expression(g, arg)?;
                            }
                            g.write(")");
                        }
                    }
                    _ => {
                        g.write(&name);
                        g.write("(");
                        for (i, arg) in call.args.iter().enumerate() {
                            if i > 0 {
                                g.write(", ");
                            }
                            emit_expression(g, arg)?;
                        }
                        g.write(")");
                    }
                }
            } else {
                return Err(CodegenError::Unsupported(
                    "Complex call targets not yet supported".to_string(),
                ));
            }
            Ok(())
        }

        Expression::Grouped(grouped) => {
            g.write("(");
            emit_expression(g, grouped.inner)?;
            g.write(")");
            Ok(())
        }

        Expression::Field(field) => {
            emit_expression(g, field.base)?;
            let field_name = g.interner().resolve(&field.field.symbol);
            g.write(&format!(".{}", field_name));
            Ok(())
        }

        Expression::Index(index) => {
            emit_expression(g, index.base)?;
            g.write("[");
            emit_expression(g, index.index)?;
            g.write("]");
            Ok(())
        }

        Expression::MethodCall(method) => {
            emit_expression(g, method.receiver)?;
            let method_name = g.interner().resolve(&method.method.symbol);
            g.write(&format!(".{}(", method_name));
            for (i, arg) in method.args.iter().enumerate() {
                if i > 0 {
                    g.write(", ");
                }
                emit_expression(g, arg)?;
            }
            g.write(")");
            Ok(())
        }

        Expression::If(if_expr) => {
            g.write("if ");
            emit_expression(g, if_expr.condition)?;
            g.write(" { ");
            for stmt in &if_expr.then_branch.statements {
                super::statements::emit_statement(g, stmt)?;
            }
            if let Some(tail) = if_expr.then_branch.tail {
                emit_expression(g, tail)?;
            }
            g.write(" }");

            if let Some(ref else_branch) = if_expr.else_branch {
                g.write(" else ");
                match else_branch {
                    crate::ast::ElseExpr::Else(block) => {
                        g.write("{ ");
                        for stmt in &block.statements {
                            super::statements::emit_statement(g, stmt)?;
                        }
                        if let Some(tail) = block.tail {
                            emit_expression(g, tail)?;
                        }
                        g.write(" }");
                    }
                    crate::ast::ElseExpr::ElseIf(elif) => {
                        emit_expression(g, &Expression::If((*elif).clone()))?;
                    }
                }
            }
            Ok(())
        }

        Expression::Array(arr) => {
            g.write("vec![");
            for (i, elem) in arr.elements.iter().enumerate() {
                if i > 0 {
                    g.write(", ");
                }
                emit_expression(g, elem)?;
            }
            g.write("]");
            Ok(())
        }

        Expression::StructLiteral(lit) => {
            let struct_name = g.interner().resolve(&lit.name.symbol);
            g.write(&format!("{} {{ ", struct_name));
            for (i, field) in lit.fields.iter().enumerate() {
                if i > 0 {
                    g.write(", ");
                }
                let name = g.interner().resolve(&field.name.symbol);
                g.write(&format!("{}: ", name));
                emit_expression(g, &field.value)?;
            }
            g.write(" }");
            Ok(())
        }

        Expression::Range(range) => {
            if let Some(start) = range.start {
                emit_expression(g, start)?;
            }
            if range.inclusive {
                g.write("..=");
            } else {
                g.write("..");
            }
            if let Some(end) = range.end {
                emit_expression(g, end)?;
            }
            Ok(())
        }

        Expression::Some(some_expr) => {
            g.write("Some(");
            emit_expression(g, some_expr.value)?;
            g.write(")");
            Ok(())
        }

        Expression::Cast(cast) => {
            emit_expression(g, cast.expr)?;
            let target_ty = super::types::naml_to_rust(&cast.target_ty, g.interner());
            g.write(&format!(" as {}", target_ty));
            Ok(())
        }

        _ => Err(CodegenError::Unsupported(format!(
            "Expression type not yet supported: {:?}",
            std::mem::discriminant(expr)
        ))),
    }
}

fn emit_literal(g: &mut RustGenerator, lit: &LiteralExpr) -> Result<(), CodegenError> {
    match &lit.value {
        Literal::Int(n) => {
            g.write(&n.to_string());
            g.write("_i64");
        }
        Literal::UInt(n) => {
            g.write(&n.to_string());
            g.write("_u64");
        }
        Literal::Float(f) => {
            g.write(&f.to_string());
            if !f.to_string().contains('.') {
                g.write(".0");
            }
            g.write("_f64");
        }
        Literal::Bool(b) => {
            g.write(if *b { "true" } else { "false" });
        }
        Literal::String(spur) => {
            let s = g.interner().resolve(spur);
            g.write(&format!("\"{}\".to_string()", escape_string(s)));
        }
        Literal::Bytes(bytes) => {
            g.write("vec![");
            for (i, b) in bytes.iter().enumerate() {
                if i > 0 {
                    g.write(", ");
                }
                g.write(&format!("{}u8", b));
            }
            g.write("]");
        }
        Literal::None => {
            g.write("None");
        }
    }
    Ok(())
}

fn binary_op_to_rust(op: &BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Mod => "%",
        BinaryOp::Eq => "==",
        BinaryOp::NotEq => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::LtEq => "<=",
        BinaryOp::Gt => ">",
        BinaryOp::GtEq => ">=",
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
        BinaryOp::BitAnd => "&",
        BinaryOp::BitOr => "|",
        BinaryOp::BitXor => "^",
        BinaryOp::Shl => "<<",
        BinaryOp::Shr => ">>",
        BinaryOp::Range => "..",
        BinaryOp::RangeIncl => "..=",
        BinaryOp::Is => "/* is */",
    }
}

fn unary_op_to_rust(op: &UnaryOp) -> &'static str {
    match op {
        UnaryOp::Neg => "-",
        UnaryOp::Not => "!",
        UnaryOp::BitNot => "!",
    }
}

fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_ops() {
        assert_eq!(binary_op_to_rust(&BinaryOp::Add), "+");
        assert_eq!(binary_op_to_rust(&BinaryOp::Eq), "==");
        assert_eq!(binary_op_to_rust(&BinaryOp::And), "&&");
    }

    #[test]
    fn test_escape_string() {
        assert_eq!(escape_string("hello"), "hello");
        assert_eq!(escape_string("hello\nworld"), "hello\\nworld");
        assert_eq!(escape_string("say \"hi\""), "say \\\"hi\\\"");
    }
}
