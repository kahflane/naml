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
use crate::source::Spanned;
use crate::typechecker::Type;

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
            if bin.op == BinaryOp::Add {
                let left_ty = g.type_of(bin.left.span());
                let right_ty = g.type_of(bin.right.span());

                let is_string_concat = matches!(left_ty, Some(Type::String))
                    || matches!(right_ty, Some(Type::String));

                if is_string_concat {
                    g.write("format!(\"{}{}\", ");
                    emit_expression(g, bin.left)?;
                    g.write(", ");
                    emit_expression(g, bin.right)?;
                    g.write(")");
                    return Ok(());
                }
            }

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
                        // Use {:?} to support both Display and Debug types
                        g.write("print!(\"{:?}\"");
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
                            // Use {:?} to support both Display and Debug types (Vec, structs, etc.)
                            g.write("println!(\"{:?}\"");
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
                            // Use {:?} to support both Display and Debug types (Vec, structs, etc.)
                            let rust_fmt = fmt.replace("{}", "{:?}");
                            g.write(&format!("println!(\"{}\"", rust_fmt));
                            for arg in call.args.iter().skip(1) {
                                g.write(", ");
                                emit_expression(g, arg)?;
                            }
                            g.write(")");
                        } else {
                            g.write("println!(\"{:?}\"");
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
                            emit_function_arg(g, arg)?;
                        }
                        g.write(")");

                        // Add ? for throws functions when in throws context
                        // Skip if inside await (await handles the ?)
                        if g.function_throws(&name)
                            && g.is_in_throws_function()
                            && !g.is_in_await_expr()
                        {
                            g.write("?");
                        }
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
            let field_name = g.interner().resolve(&field.field.symbol).to_string();

            // Check if this is an enum variant access (EnumName.Variant)
            if let Expression::Identifier(ident) = field.base {
                let base_name = g.interner().resolve(&ident.ident.symbol).to_string();
                if g.is_enum(&base_name) {
                    g.write(&format!("{}::{}", base_name, field_name));
                    return Ok(());
                }
            }

            let base_ty = g.type_of(field.base.span()).cloned();

            match (&base_ty, field_name.as_str()) {
                (Some(Type::Array(_)), "length")
                | (Some(Type::FixedArray(_, _)), "length")
                | (Some(Type::String), "length") => {
                    emit_expression(g, field.base)?;
                    g.write(".len() as i64");
                    return Ok(());
                }
                (Some(Type::String), "chars") => {
                    emit_expression(g, field.base)?;
                    g.write(".chars()");
                    return Ok(());
                }
                (Some(Type::String), "bytes") => {
                    emit_expression(g, field.base)?;
                    g.write(".as_bytes()");
                    return Ok(());
                }
                _ => {}
            }

            emit_expression(g, field.base)?;
            g.write(&format!(".{}", field_name));

            // Add .clone() for non-Copy types accessed from &self in methods
            if g.is_in_ref_method() && g.needs_clone(field.span) {
                g.write(".clone()");
            }

            Ok(())
        }

        Expression::Index(index) => {
            let base_ty = g.type_of(index.base.span()).cloned();
            let idx_ty = g.type_of(index.index.span()).cloned();

            emit_expression(g, index.base)?;

            // For Map types, use .get() with reference instead of [] to avoid borrow issues
            // .cloned() converts Option<&V> to Option<V>
            if matches!(base_ty, Some(Type::Map(_, _))) {
                g.write(".get(&");
                emit_expression(g, index.index)?;
                g.write(").cloned()");
            } else {
                g.write("[");
                emit_expression(g, index.index)?;

                let needs_usize_cast = matches!(idx_ty, Some(Type::Int) | Some(Type::Uint));
                if needs_usize_cast {
                    g.write(" as usize");
                }

                g.write("]");

                // Clone array elements of non-Copy types to avoid move errors
                let element_is_non_copy = match &base_ty {
                    Some(Type::Array(inner)) | Some(Type::FixedArray(inner, _)) => {
                        !is_copy_type_ref(inner)
                    }
                    _ => false,
                };
                if element_is_non_copy {
                    g.write(".clone()");
                }
            }
            Ok(())
        }

        Expression::MethodCall(method) => {
            let receiver_ty = g.type_of(method.receiver.span()).cloned();
            let method_name = g.interner().resolve(&method.method.symbol).to_string();

            match (&receiver_ty, method_name.as_str()) {
                (Some(Type::Array(_)), "push") | (Some(Type::Array(_)), "append") => {
                    emit_expression(g, method.receiver)?;
                    g.write(".push(");
                    for (i, arg) in method.args.iter().enumerate() {
                        if i > 0 {
                            g.write(", ");
                        }
                        emit_expression(g, arg)?;
                    }
                    g.write(")");
                    return Ok(());
                }
                (Some(Type::Array(_)), "pop") => {
                    emit_expression(g, method.receiver)?;
                    g.write(".pop()");
                    return Ok(());
                }
                (Some(Type::Array(_)), "len") | (Some(Type::String), "len") => {
                    emit_expression(g, method.receiver)?;
                    g.write(".len() as i64");
                    return Ok(());
                }
                (Some(Type::Option(_)), "or_default") => {
                    emit_expression(g, method.receiver)?;
                    // Clone before unwrap_or if accessing self field in &self method
                    if g.is_in_ref_method() && is_self_field_access(g, method.receiver) {
                        g.write(".clone()");
                    }
                    g.write(".unwrap_or(");
                    if let Some(arg) = method.args.first() {
                        emit_expression(g, arg)?;
                    }
                    g.write(")");
                    return Ok(());
                }
                (Some(Type::Option(_)), "is_some") => {
                    emit_expression(g, method.receiver)?;
                    g.write(".is_some()");
                    return Ok(());
                }
                (Some(Type::Option(_)), "is_none") => {
                    emit_expression(g, method.receiver)?;
                    g.write(".is_none()");
                    return Ok(());
                }
                (Some(Type::Map(_, _)), "get") => {
                    emit_expression(g, method.receiver)?;
                    g.write(".get(&");
                    if let Some(arg) = method.args.first() {
                        emit_expression(g, arg)?;
                    }
                    g.write(").cloned()");
                    return Ok(());
                }
                (Some(Type::Map(_, _)), "insert") | (Some(Type::Map(_, _)), "set") => {
                    emit_expression(g, method.receiver)?;
                    g.write(".insert(");
                    for (i, arg) in method.args.iter().enumerate() {
                        if i > 0 {
                            g.write(", ");
                        }
                        emit_expression(g, arg)?;
                    }
                    g.write(")");
                    return Ok(());
                }
                (Some(Type::Map(_, _)), "contains") | (Some(Type::Map(_, _)), "contains_key") => {
                    emit_expression(g, method.receiver)?;
                    g.write(".contains_key(&");
                    if let Some(arg) = method.args.first() {
                        emit_expression(g, arg)?;
                    }
                    g.write(")");
                    return Ok(());
                }
                (Some(Type::Map(_, _)), "remove") => {
                    emit_expression(g, method.receiver)?;
                    g.write(".remove(&");
                    if let Some(arg) = method.args.first() {
                        emit_expression(g, arg)?;
                    }
                    g.write(")");
                    return Ok(());
                }
                (Some(Type::String), "contains") => {
                    emit_expression(g, method.receiver)?;
                    g.write(".contains(&");
                    if let Some(arg) = method.args.first() {
                        emit_expression(g, arg)?;
                    }
                    g.write(")");
                    return Ok(());
                }
                (Some(Type::String), "starts_with") => {
                    emit_expression(g, method.receiver)?;
                    g.write(".starts_with(&");
                    if let Some(arg) = method.args.first() {
                        emit_expression(g, arg)?;
                    }
                    g.write(")");
                    return Ok(());
                }
                (Some(Type::String), "ends_with") => {
                    emit_expression(g, method.receiver)?;
                    g.write(".ends_with(&");
                    if let Some(arg) = method.args.first() {
                        emit_expression(g, arg)?;
                    }
                    g.write(")");
                    return Ok(());
                }
                (Some(Type::String), "split") => {
                    emit_expression(g, method.receiver)?;
                    g.write(".split(&");
                    if let Some(arg) = method.args.first() {
                        emit_expression(g, arg)?;
                    }
                    g.write(").map(|s| s.to_string()).collect::<Vec<_>>()");
                    return Ok(());
                }
                (Some(Type::String), "trim") => {
                    emit_expression(g, method.receiver)?;
                    g.write(".trim().to_string()");
                    return Ok(());
                }
                (Some(Type::String), "to_uppercase") | (Some(Type::String), "upper") => {
                    emit_expression(g, method.receiver)?;
                    g.write(".to_uppercase()");
                    return Ok(());
                }
                (Some(Type::String), "to_lowercase") | (Some(Type::String), "lower") => {
                    emit_expression(g, method.receiver)?;
                    g.write(".to_lowercase()");
                    return Ok(());
                }
                (Some(Type::String), "replace") => {
                    emit_expression(g, method.receiver)?;
                    g.write(".replace(&");
                    if let Some(arg) = method.args.first() {
                        emit_expression(g, arg)?;
                    }
                    g.write(", &");
                    if let Some(arg) = method.args.get(1) {
                        emit_expression(g, arg)?;
                    }
                    g.write(")");
                    return Ok(());
                }
                (Some(Type::String), "substring") | (Some(Type::String), "substr") => {
                    emit_expression(g, method.receiver)?;
                    g.write(".chars().skip(");
                    if let Some(arg) = method.args.first() {
                        emit_expression(g, arg)?;
                    }
                    g.write(" as usize).take(");
                    if let Some(arg) = method.args.get(1) {
                        emit_expression(g, arg)?;
                    }
                    g.write(" as usize).collect::<String>()");
                    return Ok(());
                }
                // Handle compare() method - transform to partial_cmp for Rust compatibility
                (_, "compare") if method.args.len() == 1 => {
                    g.write("match ");
                    emit_expression(g, method.receiver)?;
                    g.write(".partial_cmp(&");
                    emit_expression(g, &method.args[0])?;
                    g.write(") { Some(std::cmp::Ordering::Greater) => 1, Some(std::cmp::Ordering::Less) => -1, _ => 0 }");
                    return Ok(());
                }
                _ => {}
            }

            emit_expression(g, method.receiver)?;
            g.write(&format!(".{}(", method_name));
            for (i, arg) in method.args.iter().enumerate() {
                if i > 0 {
                    g.write(", ");
                }
                // Clone arguments to avoid move errors when value is used again
                emit_function_arg(g, arg)?;
            }
            g.write(")");

            // Add ? for throws methods when in throws context
            // Skip if inside await (await handles the ?)
            if let Some(type_name) = resolve_receiver_type_name(g, method.receiver) {
                if g.method_throws(&type_name, &method_name)
                    && g.is_in_throws_function()
                    && !g.is_in_await_expr()
                {
                    g.write("?");
                }
            }

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
                // Clone elements to avoid partial moves
                emit_function_arg(g, elem)?;
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
                // Clone field values to avoid move errors when the value is used elsewhere
                emit_function_arg(g, &field.value)?;
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
            // Check if this is a recursive type that needs Box wrapping
            let needs_box = if let Some(current_struct) = g.current_struct() {
                // Check if the inner value is a struct literal of the same type
                match some_expr.value {
                    Expression::StructLiteral(lit) => {
                        let inner_struct = g.interner().resolve(&lit.name.symbol);
                        inner_struct == current_struct
                    }
                    _ => {
                        // Check if the inner type matches the current struct
                        if let Some(ty) = g.type_of(some_expr.value.span()) {
                            match ty {
                                Type::Struct(st) => {
                                    let type_name = g.interner().resolve(&st.name);
                                    type_name == current_struct
                                }
                                Type::Generic(name, _) => {
                                    let type_name = g.interner().resolve(name);
                                    type_name == current_struct
                                }
                                _ => false,
                            }
                        } else {
                            false
                        }
                    }
                }
            } else {
                false
            };

            if needs_box {
                g.write("Some(Box::new(");
                emit_expression(g, some_expr.value)?;
                g.write("))");
            } else {
                g.write("Some(");
                emit_expression(g, some_expr.value)?;
                g.write(")");
            }
            Ok(())
        }

        Expression::Cast(cast) => {
            let from_ty = g.type_of(cast.expr.span());
            let target = &cast.target_ty;

            match (from_ty, target) {
                (Some(Type::String), crate::ast::NamlType::Bytes) => {
                    emit_expression(g, cast.expr)?;
                    g.write(".into_bytes()");
                }
                (Some(Type::Int), crate::ast::NamlType::String)
                | (Some(Type::Uint), crate::ast::NamlType::String)
                | (Some(Type::Float), crate::ast::NamlType::String) => {
                    emit_expression(g, cast.expr)?;
                    g.write(".to_string()");
                }
                (Some(Type::Bool), crate::ast::NamlType::String) => {
                    emit_expression(g, cast.expr)?;
                    g.write(".to_string()");
                }
                (Some(Type::Bytes), crate::ast::NamlType::String) => {
                    g.write("String::from_utf8(");
                    emit_expression(g, cast.expr)?;
                    g.write(").unwrap_or_default()");
                }
                (Some(Type::String), crate::ast::NamlType::Int) => {
                    emit_expression(g, cast.expr)?;
                    g.write(".parse::<i64>().unwrap_or(0)");
                }
                (Some(Type::String), crate::ast::NamlType::Float) => {
                    emit_expression(g, cast.expr)?;
                    g.write(".parse::<f64>().unwrap_or(0.0)");
                }
                _ => {
                    emit_expression(g, cast.expr)?;
                    let target_ty = super::types::naml_to_rust(target, g.interner());
                    g.write(&format!(" as {}", target_ty));
                }
            }
            Ok(())
        }

        Expression::Lambda(lambda) => {
            g.write("|");
            for (i, param) in lambda.params.iter().enumerate() {
                if i > 0 {
                    g.write(", ");
                }
                let param_name = g.interner().resolve(&param.name.symbol).to_string();
                g.write(&param_name);
                if let Some(ref ty) = param.ty {
                    g.write(": ");
                    let param_ty = super::types::naml_to_rust(ty, g.interner());
                    g.write(&param_ty);
                }
            }
            g.write("| ");
            emit_expression(g, lambda.body)?;
            Ok(())
        }

        Expression::Map(map) => {
            g.write("std::collections::HashMap::from([");
            for (i, entry) in map.entries.iter().enumerate() {
                if i > 0 {
                    g.write(", ");
                }
                g.write("(");
                emit_expression(g, &entry.key)?;
                g.write(", ");
                emit_expression(g, &entry.value)?;
                g.write(")");
            }
            g.write("])");
            Ok(())
        }

        Expression::Block(block) => {
            g.write("{ ");
            for stmt in &block.statements {
                super::statements::emit_statement(g, stmt)?;
            }
            if let Some(tail) = block.tail {
                emit_expression(g, tail)?;
            }
            g.write(" }");
            Ok(())
        }

        Expression::Await(await_expr) => {
            // Set flag to prevent inner call from adding ? (we'll add it after .await)
            let was_in_await = g.is_in_await_expr();
            g.set_in_await_expr(true);
            emit_expression(g, await_expr.expr)?;
            g.set_in_await_expr(was_in_await);

            g.write(".await");

            // Check if inner expression throws
            let inner_throws = match await_expr.expr {
                Expression::Call(call) => {
                    if let Expression::Identifier(ident) = call.callee {
                        let name = g.interner().resolve(&ident.ident.symbol).to_string();
                        g.function_throws(&name)
                    } else {
                        false
                    }
                }
                Expression::MethodCall(method) => {
                    let method_name = g.interner().resolve(&method.method.symbol).to_string();
                    if let Some(type_name) = resolve_receiver_type_name(g, method.receiver) {
                        g.method_throws(&type_name, &method_name)
                    } else {
                        false
                    }
                }
                _ => false,
            };

            if inner_throws {
                if g.is_in_throws_function() {
                    // In throws context: propagate error with ?
                    g.write("?");
                } else {
                    // In non-throws context: unwrap the result
                    g.write(".unwrap()");
                }
            }
            Ok(())
        }

        Expression::Spawn(_) => Err(CodegenError::Unsupported(
            "Spawn expressions not yet supported in Rust codegen".to_string(),
        )),

        Expression::Try(_) => Err(CodegenError::Unsupported(
            "Try expressions not yet supported in Rust codegen".to_string(),
        )),

        Expression::Catch(catch) => {
            emit_expression(g, catch.expr)?;
            g.write(".unwrap_or_else(|");
            let error_name = g.interner().resolve(&catch.error_binding.symbol).to_string();
            g.write(&error_name);
            g.write("| {\n");
            g.indent += 1;
            for stmt in &catch.handler.statements {
                super::statements::emit_statement(g, stmt)?;
            }
            g.indent -= 1;
            g.write_indent();
            g.write("})");
            Ok(())
        }

        Expression::Path(path) => {
            for (i, segment) in path.segments.iter().enumerate() {
                if i > 0 {
                    g.write("::");
                }
                let name = g.interner().resolve(&segment.symbol).to_string();
                g.write(&name);
            }
            Ok(())
        }
    }
}

/// Emit a function argument, adding .clone() for identifiers with non-Copy types
/// to prevent move errors when the value is used again later
fn emit_function_arg(g: &mut RustGenerator, arg: &Expression<'_>) -> Result<(), CodegenError> {
    // For simple identifiers, check if the type needs cloning
    if let Expression::Identifier(_) = arg {
        let arg_ty = g.type_of(arg.span());
        let needs_clone = match arg_ty {
            Some(Type::String) | Some(Type::Array(_)) | Some(Type::Struct(_))
            | Some(Type::Generic(_, _)) | Some(Type::Map(_, _)) => true,
            _ => false,
        };
        emit_expression(g, arg)?;
        if needs_clone {
            g.write(".clone()");
        }
        return Ok(());
    }
    // For field access, also consider cloning
    if let Expression::Field(_) = arg {
        let arg_ty = g.type_of(arg.span());
        let needs_clone = match arg_ty {
            Some(Type::String) | Some(Type::Array(_)) | Some(Type::Struct(_))
            | Some(Type::Generic(_, _)) | Some(Type::Map(_, _)) => true,
            _ => false,
        };
        emit_expression(g, arg)?;
        if needs_clone {
            g.write(".clone()");
        }
        return Ok(());
    }
    // For other expressions, emit as-is
    emit_expression(g, arg)
}

fn emit_literal(g: &mut RustGenerator, lit: &LiteralExpr) -> Result<(), CodegenError> {
    match &lit.value {
        Literal::Int(n) => {
            // Don't suffix small integers - let Rust infer the type from context
            // This allows them to work with both i64 and u64
            g.write(&n.to_string());
        }
        Literal::UInt(n) => {
            // Explicit unsigned - always suffix
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

fn is_self_field_access(g: &RustGenerator, expr: &Expression) -> bool {
    match expr {
        Expression::Identifier(ident) => {
            let name = g.interner().resolve(&ident.ident.symbol);
            name == "self"
        }
        Expression::Field(field) => is_self_field_access(g, field.base),
        _ => false,
    }
}

fn is_copy_type(ty: &Option<&Type>) -> bool {
    matches!(
        ty,
        Some(Type::Int) | Some(Type::Uint) | Some(Type::Float) | Some(Type::Bool) | Some(Type::Unit)
    )
}

fn is_copy_type_val(ty: &Option<Type>) -> bool {
    matches!(
        ty,
        Some(Type::Int) | Some(Type::Uint) | Some(Type::Float) | Some(Type::Bool) | Some(Type::Unit)
    )
}

fn is_copy_type_ref(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Int | Type::Uint | Type::Float | Type::Bool | Type::Unit
    )
}

/// Resolves the type name for a receiver expression to look up method signatures.
/// Handles multiple type variants that can be receivers (Struct, Generic, etc.)
fn resolve_receiver_type_name(g: &RustGenerator, receiver: &Expression<'_>) -> Option<String> {
    // Try getting type from annotations first
    if let Some(ty) = g.type_of(receiver.span()) {
        return type_to_name(g, ty);
    }
    None
}

/// Converts a Type to its name string for method lookup
fn type_to_name(g: &RustGenerator, ty: &Type) -> Option<String> {
    match ty {
        Type::Struct(st) => Some(g.interner().resolve(&st.name).to_string()),
        Type::Generic(name, _) => Some(g.interner().resolve(name).to_string()),
        Type::Enum(e) => Some(g.interner().resolve(&e.name).to_string()),
        _ => None,
    }
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
