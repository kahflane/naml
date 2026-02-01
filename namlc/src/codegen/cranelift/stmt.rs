use crate::ast::{BinaryOp, Expression, Literal, LiteralExpr, Statement};
use crate::codegen::cranelift::array::{
    call_array_index, call_array_len, call_array_new, call_array_set,
};
use crate::codegen::cranelift::pattern::compile_pattern_match;
use crate::codegen::cranelift::{
    call_map_set, compile_expression, get_heap_type, 
    types, CompileContext, HeapType,
};
use crate::codegen::CodegenError;
use crate::source::Spanned;
use crate::typechecker::Type;
use cranelift::prelude::*;
use crate::codegen::cranelift::exceptions::call_exception_set;
use crate::codegen::cranelift::runtime::{emit_cleanup_all_vars, emit_decref, emit_incref, emit_stack_pop, get_returned_var_name, rt_func_ref};
use crate::codegen::cranelift::strings::{call_string_char_at, call_string_char_len, call_string_from_cstr};

pub fn compile_statement(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    stmt: &Statement<'_>,
) -> Result<(), CodegenError> {
    match stmt {
        Statement::Var(var_stmt) => {
            let var_name = ctx.interner.resolve(&var_stmt.name.symbol).to_string();
            let ty = if let Some(ref naml_ty) = var_stmt.ty {
                types::naml_to_cranelift(naml_ty)
            } else if let Some(ref init) = var_stmt.init {
                // Try to get the inferred type from type annotations
                if let Some(tc_type) = ctx.annotations.get_type(init.span()) {
                    types::tc_type_to_cranelift(tc_type)
                } else {
                    cranelift::prelude::types::I64
                }
            } else {
                cranelift::prelude::types::I64
            };

            // Check if this is a string variable
            let is_string_var = matches!(var_stmt.ty.as_ref(), Some(crate::ast::NamlType::String));

            // Track heap type for cleanup (skip enum types - they are stack-allocated,
            // and exception types - they use raw allocation, not NamlStruct)
            let skip_heap_tracking = matches!(var_stmt.ty.as_ref(), Some(crate::ast::NamlType::Named(ident)) if {
                let type_name = ctx.interner.resolve(&ident.symbol).to_string();
                ctx.enum_defs.contains_key(&type_name) || ctx.exception_names.contains(&type_name)
            });
            if !skip_heap_tracking {
                if let Some(ref naml_ty) = var_stmt.ty
                    && let Some(heap_type) = get_heap_type(naml_ty)
                {
                    ctx.var_heap_types.insert(var_name.clone(), heap_type);
                }
            }

            let var = Variable::new(ctx.var_counter);
            ctx.var_counter += 1;
            builder.declare_var(var, ty);

            // Handle else block for option unwrap pattern: var x = opt else { ... }
            if let (Some(init), Some(else_block)) = (&var_stmt.init, &var_stmt.else_block) {
                // This is an option unwrap with else block
                // Compile the option expression
                let option_ptr = compile_expression(ctx, builder, init)?;

                // Load the tag from offset 0 (0 = none, 1 = some)
                let tag = builder.ins().load(
                    cranelift::prelude::types::I32,
                    MemFlags::new(),
                    option_ptr,
                    0,
                );

                // Create blocks
                let some_block = builder.create_block();
                let none_block = builder.create_block();
                let merge_block = builder.create_block();

                // Branch based on tag (tag == 0 means none)
                let is_none = builder.ins().icmp_imm(IntCC::Equal, tag, 0);
                builder
                    .ins()
                    .brif(is_none, none_block, &[], some_block, &[]);

                // None block: execute else block
                builder.switch_to_block(none_block);
                builder.seal_block(none_block);

                // Initialize variable with zero before else block (in case else doesn't exit)
                let zero = builder.ins().iconst(ty, 0);
                builder.def_var(var, zero);

                for else_stmt in &else_block.statements {
                    compile_statement(ctx, builder, else_stmt)?;
                    if ctx.block_terminated {
                        break;
                    }
                }

                // If else block didn't terminate (return/break), jump to merge
                if !ctx.block_terminated {
                    builder.ins().jump(merge_block, &[]);
                }
                ctx.block_terminated = false;

                // Some block: extract value and assign
                builder.switch_to_block(some_block);
                builder.seal_block(some_block);

                // Load value from offset 8
                let val = builder.ins().load(
                    cranelift::prelude::types::I64,
                    MemFlags::new(),
                    option_ptr,
                    8,
                );
                builder.def_var(var, val);

                // Incref the value
                let heap_type_clone = ctx.var_heap_types.get(&var_name).cloned();
                if let Some(ref heap_type) = heap_type_clone {
                    emit_incref(ctx, builder, val, heap_type)?;
                }

                builder.ins().jump(merge_block, &[]);

                // Merge block
                builder.switch_to_block(merge_block);
                builder.seal_block(merge_block);
            } else if let Some(ref init) = var_stmt.init {
                let mut val = compile_expression(ctx, builder, init)?;

                // Box string literals as NamlString* for consistent memory management
                if is_string_var
                    && matches!(
                        init,
                        Expression::Literal(LiteralExpr {
                            value: Literal::String(_),
                            ..
                        })
                    )
                {
                    val = call_string_from_cstr(ctx, builder, val)?;
                }

                builder.def_var(var, val);
                // Incref the value since we're storing a reference
                let heap_type_clone = ctx.var_heap_types.get(&var_name).cloned();
                if let Some(ref heap_type) = heap_type_clone {
                    emit_incref(ctx, builder, val, heap_type)?;
                }
            } else {
                // No initializer - create default values for collection types
                let val = match var_stmt.ty.as_ref() {
                    Some(crate::ast::NamlType::Map(_, _)) => {
                        // Create empty map with default capacity
                        let capacity = builder.ins().iconst(cranelift::prelude::types::I64, 16);
                        let func_ref = rt_func_ref(ctx, builder, "naml_map_new")?;
                        let call = builder.ins().call(func_ref, &[capacity]);
                        builder.inst_results(call)[0]
                    }
                    Some(crate::ast::NamlType::Array(_)) => {
                        // Create empty array with default capacity
                        let capacity = builder.ins().iconst(cranelift::prelude::types::I64, 8);
                        call_array_new(ctx, builder, capacity)?
                    }
                    _ => {
                        // Default to zero for other types
                        builder.ins().iconst(ty, 0)
                    }
                };
                builder.def_var(var, val);
            }

            ctx.variables.insert(var_name, var);
        }

        Statement::Assign(assign) => {
            match &assign.target {
                Expression::Identifier(ident) => {
                    let var_name = ctx.interner.resolve(&ident.ident.symbol).to_string();

                    if let Some(&var) = ctx.variables.get(&var_name) {
                        // Clone heap type before mutable operations
                        let heap_type_clone = ctx.var_heap_types.get(&var_name).cloned();

                        // For heap variables: decref old value before assigning new one
                        if let Some(ref heap_type) = heap_type_clone {
                            let old_val = builder.use_var(var);
                            emit_decref(ctx, builder, old_val, heap_type)?;
                        }

                        let mut val = compile_expression(ctx, builder, &assign.value)?;

                        // Box string literals as NamlString* when assigning to string variables
                        if matches!(&heap_type_clone, Some(HeapType::String))
                            && matches!(
                                &assign.value,
                                Expression::Literal(LiteralExpr {
                                    value: Literal::String(_),
                                    ..
                                })
                            )
                        {
                            val = call_string_from_cstr(ctx, builder, val)?;
                        }

                        builder.def_var(var, val);

                        // Incref the new value since we're storing a new reference
                        if let Some(ref heap_type) = heap_type_clone {
                            emit_incref(ctx, builder, val, heap_type)?;
                        }
                    } else {
                        return Err(CodegenError::JitCompile(format!(
                            "Undefined variable: {}",
                            var_name
                        )));
                    }
                }
                Expression::Index(index_expr) => {
                    let base = compile_expression(ctx, builder, index_expr.base)?;
                    let value = compile_expression(ctx, builder, &assign.value)?;

                    // Check if index is a string literal - if so, use map_set with NamlString conversion
                    if let Expression::Literal(LiteralExpr {
                        value: Literal::String(_),
                        ..
                    }) = index_expr.index
                    {
                        let cstr_ptr = compile_expression(ctx, builder, index_expr.index)?;
                        let naml_str = call_string_from_cstr(ctx, builder, cstr_ptr)?;
                        call_map_set(ctx, builder, base, naml_str, value)?;
                    } else {
                        // Default to array set for integer indices
                        let index = compile_expression(ctx, builder, index_expr.index)?;
                        call_array_set(ctx, builder, base, index, value)?;
                    }
                }
                Expression::Field(field_expr) => {
                    // Field assignment: base.field = value
                    // Get the base pointer (struct/exception)
                    let base_ptr = compile_expression(ctx, builder, field_expr.base)?;
                    let value = compile_expression(ctx, builder, &assign.value)?;
                    let field_name = ctx.interner.resolve(&field_expr.field.symbol).to_string();

                    // Determine field offset based on struct type
                    // For exceptions: message at 0, stack at 8, user fields at 16, 24, etc.
                    // For structs: fields at 0, 8, 16, etc.
                    if let Expression::Identifier(ident) = field_expr.base {
                        let _var_name = ctx.interner.resolve(&ident.ident.symbol).to_string();
                        // Get the type annotation to determine struct/exception type
                        // Note: use ident.span (IdentExpr span), not ident.ident.span (Ident span)
                        if let Some(type_ann) = ctx.annotations.get_type(ident.span) {
                            if let crate::typechecker::Type::Exception(exc_name) = type_ann {
                                let exc_name_str = ctx.interner.resolve(exc_name).to_string();
                                if let Some(struct_def) = ctx.struct_defs.get(&exc_name_str) {
                                    // Find field offset (message at 0, stack at 8, user fields at 16+)
                                    let offset = if field_name == "message" {
                                        0
                                    } else if field_name == "stack" {
                                        8
                                    } else if let Some(idx) =
                                        struct_def.fields.iter().position(|f| f == &field_name)
                                    {
                                        16 + (idx * 8) as i32
                                    } else {
                                        return Err(CodegenError::JitCompile(format!(
                                            "Unknown field: {}",
                                            field_name
                                        )));
                                    };
                                    builder
                                        .ins()
                                        .store(MemFlags::new(), value, base_ptr, offset);
                                    return Ok(());
                                }
                            } else if let crate::typechecker::Type::Struct(struct_type) = type_ann {
                                let struct_name =
                                    ctx.interner.resolve(&struct_type.name).to_string();
                                if let Some(struct_def) = ctx.struct_defs.get(&struct_name)
                                    && let Some(idx) =
                                        struct_def.fields.iter().position(|f| f == &field_name)
                                {
                                    let offset = (24 + idx * 8) as i32;
                                    builder
                                        .ins()
                                        .store(MemFlags::new(), value, base_ptr, offset);
                                    return Ok(());
                                }
                            } else if let crate::typechecker::Type::Generic(name, _) = type_ann {
                                // Handle generic struct types like LinkedList<T>
                                let struct_name = ctx.interner.resolve(name).to_string();
                                if let Some(struct_def) = ctx.struct_defs.get(&struct_name)
                                    && let Some(idx) =
                                        struct_def.fields.iter().position(|f| f == &field_name)
                                {
                                    let offset = (24 + idx * 8) as i32;
                                    builder
                                        .ins()
                                        .store(MemFlags::new(), value, base_ptr, offset);
                                    return Ok(());
                                }
                            }
                        }
                    }

                    return Err(CodegenError::JitCompile(format!(
                        "Cannot assign to field: {}",
                        field_name
                    )));
                }
                _ => {
                    return Err(CodegenError::Unsupported(format!(
                        "Assignment target not supported: {:?}",
                        std::mem::discriminant(&assign.target)
                    )));
                }
            }
        }

        Statement::Return(ret) => {
            // Pop from shadow stack before returning
            emit_stack_pop(ctx, builder)?;

            if let Some(ref expr) = ret.value {
                let mut val = compile_expression(ctx, builder, expr)?;

                // Convert string literals to NamlString when returning
                let return_type = ctx.annotations.get_type(expr.span());
                if matches!(return_type, Some(Type::String))
                    && matches!(
                        expr,
                        Expression::Literal(LiteralExpr {
                            value: Literal::String(_),
                            ..
                        })
                    )
                {
                    val = call_string_from_cstr(ctx, builder, val)?;
                }

                // Determine if we're returning a local heap variable (ownership transfer)
                let returned_var =
                    get_returned_var_name(expr, ctx.interner);
                let exclude_var = returned_var.as_ref().and_then(|name| {
                    if ctx.var_heap_types.contains_key(name) {
                        Some(name.as_str())
                    } else {
                        None
                    }
                });

                // Cleanup all local heap variables except the returned one
                emit_cleanup_all_vars(ctx, builder, exclude_var)?;

                // Only extend i8 to i64 if the function signature expects i64 (lambdas)
                // Regular bool-returning functions should return i8 directly
                let val_type = builder.func.dfg.value_type(val);
                let val = if val_type == cranelift::prelude::types::I8
                    && ctx.func_return_type == Some(cranelift::prelude::types::I64)
                {
                    builder.ins().uextend(cranelift::prelude::types::I64, val)
                } else {
                    val
                };
                builder.ins().return_(&[val]);
            } else {
                // Void return - cleanup all heap variables
                emit_cleanup_all_vars(ctx, builder, None)?;
                builder.ins().return_(&[]);
            }
            ctx.block_terminated = true;
        }

        Statement::Expression(expr_stmt) => {
            compile_expression(ctx, builder, &expr_stmt.expr)?;
        }

        Statement::If(if_stmt) => {
            let condition = compile_expression(ctx, builder, &if_stmt.condition)?;

            let then_block = builder.create_block();
            let else_block = builder.create_block();
            let merge_block = builder.create_block();

            builder
                .ins()
                .brif(condition, then_block, &[], else_block, &[]);

            builder.switch_to_block(then_block);
            builder.seal_block(then_block);
            ctx.block_terminated = false;
            for stmt in &if_stmt.then_branch.statements {
                compile_statement(ctx, builder, stmt)?;
                if ctx.block_terminated {
                    break;
                }
            }
            if !ctx.block_terminated {
                builder.ins().jump(merge_block, &[]);
            }

            builder.switch_to_block(else_block);
            builder.seal_block(else_block);
            ctx.block_terminated = false;
            if let Some(ref else_branch) = if_stmt.else_branch {
                match else_branch {
                    crate::ast::ElseBranch::Else(else_block_stmt) => {
                        for stmt in &else_block_stmt.statements {
                            compile_statement(ctx, builder, stmt)?;
                            if ctx.block_terminated {
                                break;
                            }
                        }
                    }
                    crate::ast::ElseBranch::ElseIf(else_if) => {
                        let nested_if = Statement::If(crate::ast::IfStmt {
                            condition: else_if.condition.clone(),
                            then_branch: else_if.then_branch.clone(),
                            else_branch: else_if.else_branch.clone(),
                            span: else_if.span,
                        });
                        compile_statement(ctx, builder, &nested_if)?;
                    }
                }
            }
            if !ctx.block_terminated {
                builder.ins().jump(merge_block, &[]);
            }

            builder.switch_to_block(merge_block);
            builder.seal_block(merge_block);
            ctx.block_terminated = false;
        }

        Statement::While(while_stmt) => {
            let header_block = builder.create_block();
            let body_block = builder.create_block();
            let exit_block = builder.create_block();

            // Save and set loop context for break/continue
            let prev_loop_exit = ctx.loop_exit_block.take();
            let prev_loop_header = ctx.loop_header_block.take();
            ctx.loop_exit_block = Some(exit_block);
            ctx.loop_header_block = Some(header_block);

            builder.ins().jump(header_block, &[]);

            builder.switch_to_block(header_block);
            let condition = compile_expression(ctx, builder, &while_stmt.condition)?;
            builder
                .ins()
                .brif(condition, body_block, &[], exit_block, &[]);

            builder.switch_to_block(body_block);
            builder.seal_block(body_block);
            ctx.block_terminated = false;
            for stmt in &while_stmt.body.statements {
                compile_statement(ctx, builder, stmt)?;
                if ctx.block_terminated {
                    break;
                }
            }
            if !ctx.block_terminated {
                builder.ins().jump(header_block, &[]);
            }

            builder.seal_block(header_block);
            builder.switch_to_block(exit_block);
            builder.seal_block(exit_block);
            ctx.block_terminated = false;

            // Restore previous loop context
            ctx.loop_exit_block = prev_loop_exit;
            ctx.loop_header_block = prev_loop_header;
        }

        Statement::For(for_stmt) => {
            // Check if iterable is a range expression (binary op with Range or RangeIncl)
            let range_info = match &for_stmt.iterable {
                Expression::Binary(bin)
                    if matches!(bin.op, BinaryOp::Range | BinaryOp::RangeIncl) =>
                {
                    Some((bin.left, bin.right, matches!(bin.op, BinaryOp::RangeIncl)))
                }
                Expression::Range(range_expr) => {
                    // Handle Expression::Range if it exists
                    range_expr
                        .start
                        .zip(range_expr.end.as_ref())
                        .map(|(s, e)| (s, *e, range_expr.inclusive))
                }
                _ => None,
            };

            // Check if iterable is a string (via type annotation, string literal, or heap type)
            let is_string_literal = matches!(
                &for_stmt.iterable,
                Expression::Literal(LiteralExpr {
                    value: Literal::String(_),
                    ..
                })
            );

            // Also check if it's a string variable by looking at var_heap_types
            let is_string_var = if let Expression::Identifier(ident) = &for_stmt.iterable {
                let var_name = ctx.interner.resolve(&ident.ident.symbol).to_string();
                matches!(ctx.var_heap_types.get(&var_name), Some(HeapType::String))
            } else {
                false
            };

            let is_string = is_string_literal
                || is_string_var
                || matches!(
                    ctx.annotations.get_type(for_stmt.iterable.span()),
                    Some(Type::String)
                );

            if let Some((start_expr, end_expr, inclusive)) = range_info {
                // Handle range iteration directly without array allocation
                // Get start and end values
                let start = compile_expression(ctx, builder, start_expr)?;
                let end = compile_expression(ctx, builder, end_expr)?;

                // Create index variable (this is both the loop counter and the value)
                let idx_var = Variable::new(ctx.var_counter);
                ctx.var_counter += 1;
                builder.declare_var(idx_var, cranelift::prelude::types::I64);
                builder.def_var(idx_var, start);

                // Bind the value variable to the same as index
                let val_name = ctx.interner.resolve(&for_stmt.value.symbol).to_string();
                ctx.variables.insert(val_name, idx_var);

                // Optionally create separate index binding (for iteration count from 0)
                let iter_var = if for_stmt.index.is_some() {
                    let iter_var = Variable::new(ctx.var_counter);
                    ctx.var_counter += 1;
                    builder.declare_var(iter_var, cranelift::prelude::types::I64);
                    let zero = builder.ins().iconst(cranelift::prelude::types::I64, 0);
                    builder.def_var(iter_var, zero);
                    if let Some(ref idx_ident) = for_stmt.index {
                        let idx_name = ctx.interner.resolve(&idx_ident.symbol).to_string();
                        ctx.variables.insert(idx_name, iter_var);
                    }
                    Some(iter_var)
                } else {
                    None
                };

                let header_block = builder.create_block();
                let body_block = builder.create_block();
                let exit_block = builder.create_block();

                let prev_loop_exit = ctx.loop_exit_block.take();
                let prev_loop_header = ctx.loop_header_block.take();
                ctx.loop_exit_block = Some(exit_block);
                ctx.loop_header_block = Some(header_block);

                builder.ins().jump(header_block, &[]);

                // Header: check if idx < end (or <= for inclusive)
                builder.switch_to_block(header_block);
                let idx_val = builder.use_var(idx_var);
                let cond = if inclusive {
                    builder
                        .ins()
                        .icmp(IntCC::SignedLessThanOrEqual, idx_val, end)
                } else {
                    builder.ins().icmp(IntCC::SignedLessThan, idx_val, end)
                };
                builder.ins().brif(cond, body_block, &[], exit_block, &[]);

                // Body
                builder.switch_to_block(body_block);
                builder.seal_block(body_block);
                ctx.block_terminated = false;

                for stmt in &for_stmt.body.statements {
                    compile_statement(ctx, builder, stmt)?;
                    if ctx.block_terminated {
                        break;
                    }
                }

                // Increment index
                if !ctx.block_terminated {
                    let idx_val = builder.use_var(idx_var);
                    let one = builder.ins().iconst(cranelift::prelude::types::I64, 1);
                    let next_idx = builder.ins().iadd(idx_val, one);
                    builder.def_var(idx_var, next_idx);

                    // Also increment iteration counter if present
                    if let Some(iter_v) = iter_var {
                        let iter_val = builder.use_var(iter_v);
                        let next_iter = builder.ins().iadd(iter_val, one);
                        builder.def_var(iter_v, next_iter);
                    }

                    builder.ins().jump(header_block, &[]);
                }

                builder.seal_block(header_block);
                builder.switch_to_block(exit_block);
                builder.seal_block(exit_block);
                ctx.block_terminated = false;

                ctx.loop_exit_block = prev_loop_exit;
                ctx.loop_header_block = prev_loop_header;
            } else if is_string {
                // Handle string character iteration
                let raw_str_ptr = compile_expression(ctx, builder, &for_stmt.iterable)?;

                // If the iterable is a string literal, convert it to NamlString*
                let str_ptr = if matches!(
                    &for_stmt.iterable,
                    Expression::Literal(LiteralExpr {
                        value: Literal::String(_),
                        ..
                    })
                ) {
                    call_string_from_cstr(ctx, builder, raw_str_ptr)?
                } else {
                    raw_str_ptr
                };

                let len = call_string_char_len(ctx, builder, str_ptr)?;

                // Create index variable
                let idx_var = Variable::new(ctx.var_counter);
                ctx.var_counter += 1;
                builder.declare_var(idx_var, cranelift::prelude::types::I64);
                let zero = builder.ins().iconst(cranelift::prelude::types::I64, 0);
                builder.def_var(idx_var, zero);

                // Create character variable (holds codepoint as int)
                let char_var = Variable::new(ctx.var_counter);
                ctx.var_counter += 1;
                builder.declare_var(char_var, cranelift::prelude::types::I64);
                let val_name = ctx.interner.resolve(&for_stmt.value.symbol).to_string();
                ctx.variables.insert(val_name, char_var);

                // Bind index if requested
                if let Some(ref idx_ident) = for_stmt.index {
                    let idx_name = ctx.interner.resolve(&idx_ident.symbol).to_string();
                    ctx.variables.insert(idx_name, idx_var);
                }

                let header_block = builder.create_block();
                let body_block = builder.create_block();
                let exit_block = builder.create_block();

                let prev_loop_exit = ctx.loop_exit_block.take();
                let prev_loop_header = ctx.loop_header_block.take();
                ctx.loop_exit_block = Some(exit_block);
                ctx.loop_header_block = Some(header_block);

                builder.ins().jump(header_block, &[]);

                builder.switch_to_block(header_block);
                let idx_val = builder.use_var(idx_var);
                let cond = builder.ins().icmp(IntCC::SignedLessThan, idx_val, len);
                builder.ins().brif(cond, body_block, &[], exit_block, &[]);

                builder.switch_to_block(body_block);
                builder.seal_block(body_block);
                ctx.block_terminated = false;

                // Get character at current index
                let idx_val = builder.use_var(idx_var);
                let char_code = call_string_char_at(ctx, builder, str_ptr, idx_val)?;
                builder.def_var(char_var, char_code);

                for stmt in &for_stmt.body.statements {
                    compile_statement(ctx, builder, stmt)?;
                    if ctx.block_terminated {
                        break;
                    }
                }

                if !ctx.block_terminated {
                    let idx_val = builder.use_var(idx_var);
                    let one = builder.ins().iconst(cranelift::prelude::types::I64, 1);
                    let next_idx = builder.ins().iadd(idx_val, one);
                    builder.def_var(idx_var, next_idx);
                    builder.ins().jump(header_block, &[]);
                }

                builder.seal_block(header_block);
                builder.switch_to_block(exit_block);
                builder.seal_block(exit_block);
                ctx.block_terminated = false;

                ctx.loop_exit_block = prev_loop_exit;
                ctx.loop_header_block = prev_loop_header;
            } else {
                // Original array iteration code
                let arr_ptr = compile_expression(ctx, builder, &for_stmt.iterable)?;
                let len = call_array_len(ctx, builder, arr_ptr)?;

                let idx_var = Variable::new(ctx.var_counter);
                ctx.var_counter += 1;
                builder.declare_var(idx_var, cranelift::prelude::types::I64);
                let zero = builder.ins().iconst(cranelift::prelude::types::I64, 0);
                builder.def_var(idx_var, zero);

                let val_var = Variable::new(ctx.var_counter);
                ctx.var_counter += 1;
                builder.declare_var(val_var, cranelift::prelude::types::I64);
                let val_name = ctx.interner.resolve(&for_stmt.value.symbol).to_string();
                ctx.variables.insert(val_name, val_var);

                if let Some(ref idx_ident) = for_stmt.index {
                    let idx_name = ctx.interner.resolve(&idx_ident.symbol).to_string();
                    ctx.variables.insert(idx_name, idx_var);
                }

                let header_block = builder.create_block();
                let body_block = builder.create_block();
                let exit_block = builder.create_block();

                let prev_loop_exit = ctx.loop_exit_block.take();
                let prev_loop_header = ctx.loop_header_block.take();
                ctx.loop_exit_block = Some(exit_block);
                ctx.loop_header_block = Some(header_block);

                builder.ins().jump(header_block, &[]);

                builder.switch_to_block(header_block);
                let idx_val = builder.use_var(idx_var);
                let cond = builder.ins().icmp(IntCC::SignedLessThan, idx_val, len);
                builder.ins().brif(cond, body_block, &[], exit_block, &[]);

                builder.switch_to_block(body_block);
                builder.seal_block(body_block);
                ctx.block_terminated = false;

                let idx_val = builder.use_var(idx_var);
                // Use call_array_index for direct element access (returns raw value)
                let elem = call_array_index(ctx, builder, arr_ptr, idx_val)?;
                builder.def_var(val_var, elem);

                for stmt in &for_stmt.body.statements {
                    compile_statement(ctx, builder, stmt)?;
                    if ctx.block_terminated {
                        break;
                    }
                }

                if !ctx.block_terminated {
                    let idx_val = builder.use_var(idx_var);
                    let one = builder.ins().iconst(cranelift::prelude::types::I64, 1);
                    let next_idx = builder.ins().iadd(idx_val, one);
                    builder.def_var(idx_var, next_idx);
                    builder.ins().jump(header_block, &[]);
                }

                builder.seal_block(header_block);
                builder.switch_to_block(exit_block);
                builder.seal_block(exit_block);
                ctx.block_terminated = false;

                ctx.loop_exit_block = prev_loop_exit;
                ctx.loop_header_block = prev_loop_header;
            }
        }

        Statement::Loop(loop_stmt) => {
            let body_block = builder.create_block();
            let exit_block = builder.create_block();

            let prev_loop_exit = ctx.loop_exit_block.take();
            let prev_loop_header = ctx.loop_header_block.take();
            ctx.loop_exit_block = Some(exit_block);
            ctx.loop_header_block = Some(body_block);

            builder.ins().jump(body_block, &[]);

            builder.switch_to_block(body_block);
            builder.seal_block(body_block);
            ctx.block_terminated = false;

            for stmt in &loop_stmt.body.statements {
                compile_statement(ctx, builder, stmt)?;
                if ctx.block_terminated {
                    break;
                }
            }

            if !ctx.block_terminated {
                builder.ins().jump(body_block, &[]);
            }

            builder.switch_to_block(exit_block);
            builder.seal_block(exit_block);
            ctx.block_terminated = false;

            ctx.loop_exit_block = prev_loop_exit;
            ctx.loop_header_block = prev_loop_header;
        }

        Statement::Break(_) => {
            if let Some(exit_block) = ctx.loop_exit_block {
                builder.ins().jump(exit_block, &[]);
                ctx.block_terminated = true;
            } else {
                return Err(CodegenError::JitCompile(
                    "break outside of loop".to_string(),
                ));
            }
        }

        Statement::Continue(_) => {
            if let Some(header_block) = ctx.loop_header_block {
                builder.ins().jump(header_block, &[]);
                ctx.block_terminated = true;
            } else {
                return Err(CodegenError::JitCompile(
                    "continue outside of loop".to_string(),
                ));
            }
        }

        Statement::Switch(switch_stmt) => {
            let scrutinee = compile_expression(ctx, builder, &switch_stmt.scrutinee)?;
            let merge_block = builder.create_block();
            let default_block = builder.create_block();

            // Create case blocks and check blocks
            let mut case_blocks = Vec::new();
            let mut check_blocks = Vec::new();

            for _ in &switch_stmt.cases {
                case_blocks.push(builder.create_block());
                check_blocks.push(builder.create_block());
            }

            // Jump to first check (or default if no cases)
            if !check_blocks.is_empty() {
                builder.ins().jump(check_blocks[0], &[]);
            } else {
                builder.ins().jump(default_block, &[]);
            }

            // Build the chain of checks using pattern matching
            for (i, case) in switch_stmt.cases.iter().enumerate() {
                builder.switch_to_block(check_blocks[i]);
                builder.seal_block(check_blocks[i]);

                // Use compile_pattern_match instead of compile_expression
                let cond = compile_pattern_match(ctx, builder, &case.pattern, scrutinee)?;

                let next_check = if i + 1 < switch_stmt.cases.len() {
                    check_blocks[i + 1]
                } else {
                    default_block
                };

                builder
                    .ins()
                    .brif(cond, case_blocks[i], &[], next_check, &[]);
            }

            // Compile each case body with pattern variable bindings
            for (i, case) in switch_stmt.cases.iter().enumerate() {
                builder.switch_to_block(case_blocks[i]);
                builder.seal_block(case_blocks[i]);
                ctx.block_terminated = false;

                // Bind pattern variables before executing the case body
                bind_pattern_vars(ctx, builder, &case.pattern, scrutinee)?;

                for stmt in &case.body.statements {
                    compile_statement(ctx, builder, stmt)?;
                    if ctx.block_terminated {
                        break;
                    }
                }

                if !ctx.block_terminated {
                    builder.ins().jump(merge_block, &[]);
                }
            }

            // Compile default
            builder.switch_to_block(default_block);
            builder.seal_block(default_block);
            ctx.block_terminated = false;

            if let Some(ref default_body) = switch_stmt.default {
                for stmt in &default_body.statements {
                    compile_statement(ctx, builder, stmt)?;
                    if ctx.block_terminated {
                        break;
                    }
                }
            }

            if !ctx.block_terminated {
                builder.ins().jump(merge_block, &[]);
            }

            builder.switch_to_block(merge_block);
            builder.seal_block(merge_block);
            ctx.block_terminated = false;
        }

        Statement::Throw(throw_stmt) => {
            // Compile the exception value
            let exception_ptr = compile_expression(ctx, builder, &throw_stmt.value)?;

            // Capture the current stack trace and store at offset 8
            let stack_capture_func = rt_func_ref(ctx, builder, "naml_stack_capture")?;
            let stack_call = builder.ins().call(stack_capture_func, &[]);
            let stack_ptr = builder.inst_results(stack_call)[0];
            builder.ins().store(MemFlags::new(), stack_ptr, exception_ptr, 8);

            // Set the current exception in thread-local storage
            call_exception_set(ctx, builder, exception_ptr)?;

            // Return 0 (indicates exception) from the function
            let zero = builder.ins().iconst(cranelift::prelude::types::I64, 0);
            builder.ins().return_(&[zero]);
            ctx.block_terminated = true;
        }

        Statement::Const(const_stmt) => {
            // Constants are treated like immutable variables
            let var_name = ctx.interner.resolve(&const_stmt.name.symbol).to_string();
            let var = Variable::new(ctx.var_counter);
            ctx.var_counter += 1;
            ctx.variables.insert(var_name.clone(), var);
            builder.declare_var(var, cranelift::prelude::types::I64);

            let init_val = compile_expression(ctx, builder, &const_stmt.init)?;
            builder.def_var(var, init_val);
        }

        Statement::Block(block_stmt) => {
            for stmt in &block_stmt.statements {
                compile_statement(ctx, builder, stmt)?;
                if ctx.block_terminated {
                    break;
                }
            }
        }

        Statement::Locked(locked_stmt) => {
            use crate::ast::LockKind;

            // Compile the mutex/rwlock expression to get the pointer
            let mutex_ptr = compile_expression(ctx, builder, &locked_stmt.mutex)?;

            // Determine which lock/unlock functions to use based on lock kind
            let (lock_func, unlock_func) = match locked_stmt.kind {
                LockKind::Exclusive => ("naml_mutex_lock", "naml_mutex_unlock"),
                LockKind::Read => ("naml_rwlock_read_lock", "naml_rwlock_read_unlock"),
                LockKind::Write => ("naml_rwlock_write_lock", "naml_rwlock_write_unlock"),
            };

            // Call lock function to acquire the lock and get initial value
            let lock_fn = rt_func_ref(ctx, builder, lock_func)?;
            let locked_value = builder.ins().call(lock_fn, &[mutex_ptr]);
            let locked_value = builder.inst_results(locked_value)[0];

            // Create a variable for the binding
            let var = Variable::new(ctx.var_counter);
            ctx.var_counter += 1;
            builder.declare_var(var, cranelift::prelude::types::I64);
            builder.def_var(var, locked_value);

            // Store the binding in the variables map
            let binding_name = ctx.interner.resolve(&locked_stmt.binding.symbol).to_string();
            let old_binding = ctx.variables.insert(binding_name.clone(), var);

            // Compile the body statements
            for stmt in &locked_stmt.body.statements {
                compile_statement(ctx, builder, stmt)?;
                if ctx.block_terminated {
                    break;
                }
            }

            // Read the final value from the binding (may have been modified)
            let final_value = builder.use_var(var);

            // Call unlock function to release the lock
            // Note: For read locks, we don't update the value (read-only)
            let unlock_fn = rt_func_ref(ctx, builder, unlock_func)?;
            match locked_stmt.kind {
                LockKind::Read => {
                    // Read unlock doesn't take a value parameter
                    builder.ins().call(unlock_fn, &[mutex_ptr]);
                }
                LockKind::Exclusive | LockKind::Write => {
                    // Exclusive and write unlock take the new value
                    builder.ins().call(unlock_fn, &[mutex_ptr, final_value]);
                }
            }

            // Restore or remove the binding
            if let Some(old) = old_binding {
                ctx.variables.insert(binding_name, old);
            } else {
                ctx.variables.remove(&binding_name);
            }
        }
    }

    Ok(())
}

fn bind_pattern_vars(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    pattern: &crate::ast::Pattern<'_>,
    scrutinee: Value,
) -> Result<(), CodegenError> {
    use crate::ast::Pattern;

    match pattern {
        Pattern::Variant(variant) if !variant.bindings.is_empty() => {
            // Get the enum and variant info
            let (enum_name, variant_name) = if variant.path.len() == 1 {
                let var_name = ctx.interner.resolve(&variant.path[0].symbol).to_string();

                // Search all enum definitions for this variant
                let mut found = None;
                for (e_name, enum_def) in ctx.enum_defs.iter() {
                    if enum_def.variants.iter().any(|v| v.name == var_name) {
                        found = Some((e_name.clone(), var_name.clone()));
                        break;
                    }
                }

                match found {
                    Some(pair) => pair,
                    None => return Ok(()),
                }
            } else {
                let enum_name = ctx.interner.resolve(&variant.path[0].symbol).to_string();
                let variant_name = ctx
                    .interner
                    .resolve(&variant.path.last().unwrap().symbol)
                    .to_string();
                (enum_name, variant_name)
            };

            if let Some(enum_def) = ctx.enum_defs.get(&enum_name)
                && let Some(var_def) = enum_def.variants.iter().find(|v| v.name == variant_name)
            {
                for (i, binding) in variant.bindings.iter().enumerate() {
                    let binding_name = ctx.interner.resolve(&binding.symbol).to_string();
                    let offset = (var_def.data_offset + i * 8) as i32;

                    let field_val = builder.ins().load(
                        cranelift::prelude::types::I64,
                        MemFlags::new(),
                        scrutinee,
                        offset,
                    );

                    let var = Variable::new(ctx.var_counter);
                    ctx.var_counter += 1;
                    builder.declare_var(var, cranelift::prelude::types::I64);
                    builder.def_var(var, field_val);
                    ctx.variables.insert(binding_name, var);
                }
            }
        }

        Pattern::Identifier(ident) => {
            // Check if it's not a variant name (binding patterns)
            let name = ctx.interner.resolve(&ident.ident.symbol).to_string();

            // Check if it's a variant name - don't bind in that case
            let is_variant = ctx
                .enum_defs
                .values()
                .any(|def| def.variants.iter().any(|v| v.name == name));

            if !is_variant {
                let var = Variable::new(ctx.var_counter);
                ctx.var_counter += 1;
                builder.declare_var(var, cranelift::prelude::types::I64);
                builder.def_var(var, scrutinee);
                ctx.variables.insert(name, var);
            }
        }

        _ => {}
    }

    Ok(())
}
