///
/// Statement Code Generation
///
/// Converts naml statements to Rust statements:
/// - Variable declarations (var, const)
/// - Assignments
/// - If/else
/// - While loops
/// - For loops
/// - Return statements
/// - Expression statements
///

use crate::ast::{AssignOp, BlockStmt, Expression, Statement};
use crate::codegen::CodegenError;

use super::expressions::emit_expression;
use super::types::naml_to_rust;
use super::RustGenerator;

pub fn emit_block(g: &mut RustGenerator, block: &BlockStmt<'_>) -> Result<(), CodegenError> {
    for stmt in &block.statements {
        emit_statement(g, stmt)?;
    }
    Ok(())
}

fn emit_switch_pattern(g: &mut RustGenerator, pattern: &Expression<'_>) -> Result<(), CodegenError> {
    if let Expression::Identifier(ident) = pattern {
        let name = g.interner().resolve(&ident.ident.symbol).to_string();
        if let Some(enum_name) = g.get_enum_for_variant(&name) {
            g.write(&format!("{}::{}", enum_name, name));
            return Ok(());
        }
    }
    emit_expression(g, pattern)
}

pub fn emit_statement(g: &mut RustGenerator, stmt: &Statement<'_>) -> Result<(), CodegenError> {
    match stmt {
        Statement::Var(var_stmt) => {
            let name = g.interner().resolve(&var_stmt.name.symbol).to_string();

            g.write_indent();
            g.write("let ");
            if var_stmt.mutable {
                g.write("mut ");
            }
            g.write(&name);

            if let Some(ref ty) = var_stmt.ty {
                let rust_ty = naml_to_rust(ty, g.interner());
                g.write(&format!(": {}", rust_ty));
            }

            if let Some(ref init) = var_stmt.init {
                g.write(" = ");
                emit_expression(g, init)?;
            }

            g.write(";\n");
            Ok(())
        }

        Statement::Const(const_stmt) => {
            let name = g.interner().resolve(&const_stmt.name.symbol).to_string();

            g.write_indent();
            g.write("let ");
            g.write(&name);

            if let Some(ref ty) = const_stmt.ty {
                let rust_ty = naml_to_rust(ty, g.interner());
                g.write(&format!(": {}", rust_ty));
            }

            g.write(" = ");
            emit_expression(g, &const_stmt.init)?;
            g.write(";\n");
            Ok(())
        }

        Statement::Assign(assign_stmt) => {
            g.write_indent();
            emit_expression(g, &assign_stmt.target)?;

            match assign_stmt.op {
                AssignOp::Assign => g.write(" = "),
                AssignOp::AddAssign => g.write(" += "),
                AssignOp::SubAssign => g.write(" -= "),
                AssignOp::MulAssign => g.write(" *= "),
                AssignOp::DivAssign => g.write(" /= "),
                AssignOp::ModAssign => g.write(" %= "),
                AssignOp::BitAndAssign => g.write(" &= "),
                AssignOp::BitOrAssign => g.write(" |= "),
                AssignOp::BitXorAssign => g.write(" ^= "),
            }

            emit_expression(g, &assign_stmt.value)?;
            g.write(";\n");
            Ok(())
        }

        Statement::Expression(expr_stmt) => {
            g.write_indent();
            emit_expression(g, &expr_stmt.expr)?;
            g.write(";\n");
            Ok(())
        }

        Statement::Return(return_stmt) => {
            g.write_indent();
            g.write("return");
            if let Some(ref value) = return_stmt.value {
                g.write(" ");
                emit_expression(g, value)?;
            }
            g.write(";\n");
            Ok(())
        }

        Statement::If(if_stmt) => {
            g.write_indent();
            g.write("if ");
            emit_expression(g, &if_stmt.condition)?;
            g.write(" {\n");

            g.indent_inc();
            emit_block(g, &if_stmt.then_branch)?;
            g.indent_dec();

            g.write_indent();
            g.write("}");

            if let Some(ref else_branch) = if_stmt.else_branch {
                g.write(" else ");
                match else_branch {
                    crate::ast::ElseBranch::Else(block) => {
                        g.write("{\n");
                        g.indent_inc();
                        emit_block(g, block)?;
                        g.indent_dec();
                        g.write_indent();
                        g.write("}");
                    }
                    crate::ast::ElseBranch::ElseIf(elif) => {
                        emit_statement(g, &Statement::If(*elif.clone()))?;
                        return Ok(());
                    }
                }
            }

            g.write("\n");
            Ok(())
        }

        Statement::While(while_stmt) => {
            g.write_indent();
            g.write("while ");
            emit_expression(g, &while_stmt.condition)?;
            g.write(" {\n");

            g.indent_inc();
            emit_block(g, &while_stmt.body)?;
            g.indent_dec();

            g.writeln("}");
            Ok(())
        }

        Statement::For(for_stmt) => {
            g.write_indent();

            let var_name = g.interner().resolve(&for_stmt.value.symbol);

            if let Some(ref index_var) = for_stmt.index {
                let index_name = g.interner().resolve(&index_var.symbol);
                g.write(&format!("for ({}, {}) in ", index_name, var_name));
                emit_expression(g, &for_stmt.iterable)?;
                g.write(".iter().enumerate()");
            } else {
                g.write(&format!("for {} in ", var_name));
                emit_expression(g, &for_stmt.iterable)?;
            }

            g.write(" {\n");

            g.indent_inc();
            emit_block(g, &for_stmt.body)?;
            g.indent_dec();

            g.writeln("}");
            Ok(())
        }

        Statement::Loop(loop_stmt) => {
            g.write_indent();
            g.write("loop {\n");

            g.indent_inc();
            emit_block(g, &loop_stmt.body)?;
            g.indent_dec();

            g.writeln("}");
            Ok(())
        }

        Statement::Break(_) => {
            g.writeln("break;");
            Ok(())
        }

        Statement::Continue(_) => {
            g.writeln("continue;");
            Ok(())
        }

        Statement::Block(block) => {
            g.write_indent();
            g.write("{\n");
            g.indent_inc();
            emit_block(g, block)?;
            g.indent_dec();
            g.writeln("}");
            Ok(())
        }

        Statement::Switch(switch_stmt) => {
            g.write_indent();
            g.write("match &");
            emit_expression(g, &switch_stmt.scrutinee)?;
            g.write(" {\n");

            g.indent_inc();

            for case in &switch_stmt.cases {
                g.write_indent();
                emit_switch_pattern(g, &case.pattern)?;
                g.write(" => {\n");
                g.indent_inc();
                emit_block(g, &case.body)?;
                g.indent_dec();
                g.writeln("},");
            }

            if let Some(ref default) = switch_stmt.default {
                g.writeln("_ => {");
                g.indent_inc();
                emit_block(g, default)?;
                g.indent_dec();
                g.writeln("},");
            }

            g.indent_dec();
            g.writeln("}");
            Ok(())
        }

        Statement::Throw(throw_stmt) => {
            g.write_indent();
            g.write("return Err(");
            emit_expression(g, &throw_stmt.value)?;
            g.write(");\n");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_exists() {
        assert!(true);
    }
}
