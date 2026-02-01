use crate::ast::{BinaryOp, Expression, Literal, LiteralExpr, NamlType, TemplateStringPart};
use crate::codegen::cranelift::array::{compile_array_literal, compile_direct_array_get_or_panic};
use crate::codegen::cranelift::literal::compile_literal;
use crate::codegen::cranelift::map::{compile_direct_map_get_or_panic, compile_map_literal};
use crate::codegen::cranelift::method::compile_method_call;
use crate::codegen::cranelift::stmt::compile_statement;
use crate::codegen::cranelift::CompileContext;
use crate::codegen::CodegenError;
use crate::source::Spanned;
use crate::typechecker::Type;
use cranelift::prelude::*;
use cranelift_module::Module;
use crate::codegen::cranelift::binop::{compile_binary_op, compile_unary_op};
use crate::codegen::cranelift::exceptions::{call_exception_check, call_exception_clear, call_exception_get};
use crate::codegen::cranelift::externs::compile_extern_call;
use crate::codegen::cranelift::options::{compile_option_from_array_get, compile_option_from_map_get};
use crate::codegen::cranelift::runtime::{call_alloc_closure_data, rt_func_ref};
use crate::codegen::cranelift::spawns::call_spawn_closure;
use crate::codegen::cranelift::strings::{call_bytes_to_string, call_float_to_string, call_int_to_string, call_string_concat, call_string_equals, call_string_from_cstr, call_string_to_bytes, call_string_to_float, call_string_to_int};
use crate::codegen::cranelift::literal::compile_string_literal;
use crate::codegen::cranelift::structs::{call_struct_get_field, call_struct_new, call_struct_set_field};
use crate::codegen::cranelift::types::tc_type_to_cranelift;

pub fn compile_expression(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    expr: &Expression<'_>,
) -> Result<Value, CodegenError> {
    match expr {
        Expression::Literal(lit_expr) => compile_literal(ctx, builder, &lit_expr.value),

        Expression::Identifier(ident) => {
            let name = ctx.interner.resolve(&ident.ident.symbol).to_string();
            if let Some(&var) = ctx.variables.get(&name) {
                Ok(builder.use_var(var))
            } else if let Some(&func_id) = ctx.functions.get(&name) {
                let ptr_type = cranelift::prelude::types::I64;
                let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);
                let func_addr = builder.ins().func_addr(ptr_type, func_ref);

                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    24,
                    0,
                ));
                let slot_addr = builder.ins().stack_addr(ptr_type, slot, 0);

                builder
                    .ins()
                    .store(MemFlags::new(), func_addr, slot_addr, 0);

                let null_ptr = builder.ins().iconst(ptr_type, 0);
                builder.ins().store(MemFlags::new(), null_ptr, slot_addr, 8);

                builder
                    .ins()
                    .store(MemFlags::new(), null_ptr, slot_addr, 16);

                Ok(slot_addr)
            } else {
                Err(CodegenError::JitCompile(format!(
                    "Undefined variable: {}",
                    name
                )))
            }
        }

        Expression::Path(path_expr) => {
            // Handle enum variant access: EnumType::Variant
            if path_expr.segments.len() == 2 {
                let enum_name = ctx
                    .interner
                    .resolve(&path_expr.segments[0].symbol)
                    .to_string();
                let variant_name = ctx
                    .interner
                    .resolve(&path_expr.segments[1].symbol)
                    .to_string();

                if let Some(enum_def) = ctx.enum_defs.get(&enum_name)
                    && let Some(variant) = enum_def.variants.iter().find(|v| v.name == variant_name)
                {
                    // Allocate stack slot and set tag
                    let slot = builder.create_sized_stack_slot(StackSlotData::new(
                        StackSlotKind::ExplicitSlot,
                        enum_def.size as u32,
                        3,
                    ));
                    let slot_addr =
                        builder
                            .ins()
                            .stack_addr(cranelift::prelude::types::I64, slot, 0);

                    let tag_val = builder
                        .ins()
                        .iconst(cranelift::prelude::types::I64, variant.tag as i64);
                    builder.ins().store(MemFlags::new(), tag_val, slot_addr, 0);

                    return Ok(slot_addr);
                }
            }

            Err(CodegenError::Unsupported(format!(
                "Path expression not supported: {:?}",
                path_expr
                    .segments
                    .iter()
                    .map(|s| ctx.interner.resolve(&s.symbol))
                    .collect::<Vec<_>>()
            )))
        }

        Expression::Binary(bin) => {
            if bin.op == BinaryOp::NullCoalesce {
                let lhs = compile_expression(ctx, builder, bin.left)?;

                // Create blocks for branching
                let some_block = builder.create_block();
                let none_block = builder.create_block();
                let merge_block = builder.create_block();

                // Add block parameter for the result
                builder.append_block_param(merge_block, cranelift::prelude::types::I64);

                // Load the tag from offset 0 of the option struct
                let tag =
                    builder
                        .ins()
                        .load(cranelift::prelude::types::I32, MemFlags::new(), lhs, 0);
                let zero_tag = builder.ins().iconst(cranelift::prelude::types::I32, 0);
                let is_none = builder.ins().icmp(IntCC::Equal, tag, zero_tag);
                builder
                    .ins()
                    .brif(is_none, none_block, &[], some_block, &[]);

                // Some block: extract the value from offset 8
                builder.switch_to_block(some_block);
                builder.seal_block(some_block);
                let inner_value =
                    builder
                        .ins()
                        .load(cranelift::prelude::types::I64, MemFlags::new(), lhs, 8);
                builder.ins().jump(merge_block, &[inner_value]);

                // None block: evaluate and use rhs
                builder.switch_to_block(none_block);
                builder.seal_block(none_block);
                let rhs = compile_expression(ctx, builder, bin.right)?;
                builder.ins().jump(merge_block, &[rhs]);

                // Merge block: result is block parameter
                builder.switch_to_block(merge_block);
                builder.seal_block(merge_block);
                let result = builder.block_params(merge_block)[0];
                return Ok(result);
            }

            // Check if this is a string comparison (Eq/NotEq)
            let lhs_type = ctx.annotations.get_type(bin.left.span());
            if matches!(lhs_type, Some(Type::String))
                && matches!(bin.op, BinaryOp::Eq | BinaryOp::NotEq)
            {
                let lhs = compile_expression(ctx, builder, bin.left)?;
                let rhs = compile_expression(ctx, builder, bin.right)?;
                // Convert lhs to NamlString if it's a string literal
                let lhs_str = if matches!(
                    bin.left,
                    Expression::Literal(LiteralExpr {
                        value: Literal::String(_),
                        ..
                    })
                ) {
                    call_string_from_cstr(ctx, builder, lhs)?
                } else {
                    lhs
                };
                // Convert rhs to NamlString if it's a string literal
                let rhs_str = if matches!(
                    bin.right,
                    Expression::Literal(LiteralExpr {
                        value: Literal::String(_),
                        ..
                    })
                ) {
                    call_string_from_cstr(ctx, builder, rhs)?
                } else {
                    rhs
                };
                let result = call_string_equals(ctx, builder, lhs_str, rhs_str)?;
                if bin.op == BinaryOp::NotEq {
                    // Negate the result
                    let one = builder.ins().iconst(cranelift::prelude::types::I64, 1);
                    return Ok(builder.ins().bxor(result, one));
                }
                return Ok(result);
            }
            let lhs = compile_expression(ctx, builder, bin.left)?;
            let rhs = compile_expression(ctx, builder, bin.right)?;
            compile_binary_op(builder, &bin.op, lhs, rhs)
        }

        Expression::Unary(unary) => {
            let operand = compile_expression(ctx, builder, unary.operand)?;
            compile_unary_op(builder, &unary.op, operand)
        }

        Expression::Call(call) => {
            if let Expression::Identifier(ident) = call.callee {
                let func_name = ctx.interner.resolve(&ident.ident.symbol);

                let actual_func_name =
                    if let Some(mangled_name) = ctx.annotations.get_call_instantiation(call.span) {
                        mangled_name.as_str()
                    } else {
                        func_name
                    };

                let is_user_defined = ctx.functions.contains_key(actual_func_name);

                if !is_user_defined {
                    let qualified_name = if let Some(module) = ctx.annotations.get_resolved_module(call.span) {
                        format!("{}::{}", module, func_name)
                    } else {
                        func_name.to_string()
                    };

                    if let Some(builtin) = super::builtins::lookup_builtin(&qualified_name)
                        .or_else(|| super::builtins::lookup_builtin(func_name)) {
                        return super::builtins::compile_builtin_call(ctx, builder, builtin, &call.args);
                    }
                }

                // Check for normal (naml) function
                if let Some(&func_id) = ctx.functions.get(actual_func_name) {
                    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);

                    let closure_data = builder.ins().iconst(cranelift::prelude::types::I64, 0);
                    let mut args = vec![closure_data];

                    for arg in &call.args {
                        let mut val = compile_expression(ctx, builder, arg)?;
                        if matches!(
                            arg,
                            Expression::Literal(LiteralExpr {
                                value: Literal::String(_),
                                ..
                            })
                        ) {
                            val = call_string_from_cstr(ctx, builder, val)?;
                        }
                        args.push(val);
                    }

                    let call_inst = builder.ins().call(func_ref, &args);
                    let results = builder.inst_results(call_inst);

                    if results.is_empty() {
                        Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
                    } else {
                        Ok(results[0])
                    }
                }
                // Check for extern function
                else if let Some(extern_fn) = ctx.extern_fns.get(func_name).cloned() {
                    compile_extern_call(ctx, builder, &extern_fn, &call.args)
                }
                // Check for closure (lambda) variable
                else if let Some(&var) = ctx.variables.get(func_name) {
                    // This is a closure call - load the closure struct
                    let closure_ptr = builder.use_var(var);

                    // Load function pointer from offset 0
                    let func_ptr = builder.ins().load(
                        cranelift::prelude::types::I64,
                        MemFlags::new(),
                        closure_ptr,
                        0,
                    );

                    // Load data pointer from offset 8
                    let data_ptr = builder.ins().load(
                        cranelift::prelude::types::I64,
                        MemFlags::new(),
                        closure_ptr,
                        8,
                    );

                    // Build signature for indirect call: (closure_data_ptr, ...args) -> i64
                    let mut sig = ctx.module.make_signature();
                    sig.params
                        .push(AbiParam::new(cranelift::prelude::types::I64)); // closure data
                    for _ in &call.args {
                        sig.params
                            .push(AbiParam::new(cranelift::prelude::types::I64));
                    }
                    sig.returns
                        .push(AbiParam::new(cranelift::prelude::types::I64));

                    let sig_ref = builder.import_signature(sig);

                    // Build arguments: first is data_ptr, then actual args
                    let mut args = vec![data_ptr];
                    for arg in &call.args {
                        args.push(compile_expression(ctx, builder, arg)?);
                    }

                    // Indirect call through function pointer
                    let call_inst = builder.ins().call_indirect(sig_ref, func_ptr, &args);
                    let results = builder.inst_results(call_inst);

                    if results.is_empty() {
                        Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
                    } else {
                        Ok(results[0])
                    }
                }
                // Check for exception constructor: ExceptionType("message")
                else if ctx.struct_defs.contains_key(func_name) {
                    // Exception constructor - allocate on heap (exceptions outlive stack frames)
                    let struct_def = ctx.struct_defs.get(func_name).unwrap();
                    let num_fields = struct_def.fields.len();
                    // Exception layout: message (8) + stack (8) + user fields (8 each)
                    // Total size: 16 bytes for message + stack pointers + 8 bytes per field
                    let size = 16 + (num_fields * 8);

                    // Allocate on heap since exceptions can escape the current stack frame
                    let size_val = builder
                        .ins()
                        .iconst(cranelift::prelude::types::I64, size as i64);
                    let exception_ptr = call_alloc_closure_data(ctx, builder, size_val)?;

                    // Store message string at offset 0
                    if !call.args.is_empty() {
                        let mut message = compile_expression(ctx, builder, &call.args[0])?;
                        // Convert string literal to NamlString
                        if matches!(
                            &call.args[0],
                            Expression::Literal(LiteralExpr {
                                value: Literal::String(_),
                                ..
                            })
                        ) {
                            message = call_string_from_cstr(ctx, builder, message)?;
                        }
                        builder
                            .ins()
                            .store(MemFlags::new(), message, exception_ptr, 0);
                    }

                    // Initialize stack to null (captured at throw time)
                    let null = builder.ins().iconst(cranelift::prelude::types::I64, 0);
                    builder.ins().store(MemFlags::new(), null, exception_ptr, 8);

                    Ok(exception_ptr)
                } else {
                    Err(CodegenError::JitCompile(format!(
                        "Unknown function: {}",
                        func_name
                    )))
                }
            }
            // Check for path-based function calls (module::function, std::module::function, etc.)
            else if let Expression::Path(path_expr) = call.callee {
                // Function name is always the last segment
                let func_name = ctx
                    .interner
                    .resolve(&path_expr.segments.last().unwrap().symbol)
                    .to_string();

                // Build qualified name for builtin lookup (e.g., "array::count", "map::count")
                // Skip "std" and "collections" prefixes for cleaner lookup names
                let qualified_name: String = {
                    let segments: Vec<&str> = path_expr.segments.iter()
                        .map(|s| ctx.interner.resolve(&s.symbol))
                        .filter(|&s| s != "std" && s != "collections")
                        .collect();
                    segments.join("::")
                };

                // 1. Check if this function exists in ctx.functions (user-defined or imported)
                if let Some(&func_id) = ctx.functions.get(&func_name) {
                    let func_ref = ctx.module.declare_func_in_func(func_id, builder.func);

                    // First arg is closure data (0 for regular functions)
                    let closure_data = builder.ins().iconst(cranelift::prelude::types::I64, 0);
                    let mut args = vec![closure_data];

                    for arg in &call.args {
                        let mut val = compile_expression(ctx, builder, arg)?;
                        // Check if argument is a string literal that needs conversion
                        if matches!(
                            arg,
                            Expression::Literal(LiteralExpr {
                                value: Literal::String(_),
                                ..
                            })
                        ) {
                            val = call_string_from_cstr(ctx, builder, val)?;
                        }
                        args.push(val);
                    }

                    let call_inst = builder.ins().call(func_ref, &args);
                    let results = builder.inst_results(call_inst);

                    return if results.is_empty() {
                        Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
                    } else {
                        Ok(results[0])
                    };
                }

                // 2. Check builtin registry - try qualified name first, then simple name
                if let Some(builtin) = super::builtins::lookup_builtin(&qualified_name)
                    .or_else(|| super::builtins::lookup_builtin(&func_name)) {
                    return super::builtins::compile_builtin_call(ctx, builder, builtin, &call.args);
                }

                // Check for enum variant constructor: EnumType::Variant(data)
                if path_expr.segments.len() == 2 {
                    let enum_name = ctx
                        .interner
                        .resolve(&path_expr.segments[0].symbol)
                        .to_string();
                    let variant_name = ctx
                        .interner
                        .resolve(&path_expr.segments[1].symbol)
                        .to_string();

                    if let Some(enum_def) = ctx.enum_defs.get(&enum_name)
                        && let Some(variant) =
                        enum_def.variants.iter().find(|v| v.name == variant_name)
                    {
                        // Allocate stack slot for enum
                        let slot = builder.create_sized_stack_slot(StackSlotData::new(
                            StackSlotKind::ExplicitSlot,
                            enum_def.size as u32,
                            0,
                        ));
                        let slot_addr =
                            builder
                                .ins()
                                .stack_addr(cranelift::prelude::types::I64, slot, 0);

                        // Store tag
                        let tag_val = builder
                            .ins()
                            .iconst(cranelift::prelude::types::I64, variant.tag as i64);
                        builder.ins().store(MemFlags::new(), tag_val, slot_addr, 0);

                        // Store data fields
                        for (i, arg) in call.args.iter().enumerate() {
                            let mut arg_val = compile_expression(ctx, builder, arg)?;
                            // Check if argument is a string type - if so, convert C string to NamlString
                            if let Some(Type::String) = ctx.annotations.get_type(arg.span()) {
                                // For string literals, convert to NamlString
                                if matches!(
                                    arg,
                                    Expression::Literal(LiteralExpr {
                                        value: Literal::String(_),
                                        ..
                                    })
                                ) {
                                    arg_val = call_string_from_cstr(ctx, builder, arg_val)?;
                                }
                            }
                            let offset = (variant.data_offset + i * 8) as i32;
                            builder
                                .ins()
                                .store(MemFlags::new(), arg_val, slot_addr, offset);
                        }

                        return Ok(slot_addr);
                    }
                }

                Err(CodegenError::Unsupported(format!(
                    "Unknown path call: {:?}",
                    path_expr
                        .segments
                        .iter()
                        .map(|s| ctx.interner.resolve(&s.symbol))
                        .collect::<Vec<_>>()
                )))
            } else {
                Err(CodegenError::Unsupported(
                    "Indirect function calls not yet supported".to_string(),
                ))
            }
        }

        Expression::Grouped(grouped) => compile_expression(ctx, builder, grouped.inner),

        Expression::Block(block) => {
            for stmt in &block.statements {
                compile_statement(ctx, builder, stmt)?;
                if ctx.block_terminated {
                    let unreachable_block = builder.create_block();
                    builder.switch_to_block(unreachable_block);
                    builder.seal_block(unreachable_block);
                    let dummy = builder.ins().iconst(cranelift::prelude::types::I64, 0);
                    builder
                        .ins()
                        .trap(cranelift::prelude::TrapCode::unwrap_user(1));
                    return Ok(dummy);
                }
            }
            if let Some(tail) = &block.tail {
                compile_expression(ctx, builder, tail)
            } else {
                Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
            }
        }

        Expression::Array(arr_expr) => compile_array_literal(ctx, builder, &arr_expr.elements),

        Expression::Map(map_expr) => compile_map_literal(ctx, builder, &map_expr.entries),

        Expression::Index(index_expr) => {
            let base = compile_expression(ctx, builder, index_expr.base)?;

            // Indexing returns option<T> for safety (none if out of bounds / key not found)
            if let Expression::Literal(LiteralExpr {
                                           value: Literal::String(_),
                                           ..
                                       }) = index_expr.index
            {
                let cstr_ptr = compile_expression(ctx, builder, index_expr.index)?;
                let naml_str = call_string_from_cstr(ctx, builder, cstr_ptr)?;
                compile_option_from_map_get(ctx, builder, base, naml_str)
            } else {
                let index = compile_expression(ctx, builder, index_expr.index)?;
                compile_option_from_array_get(ctx, builder, base, index)
            }
        }

        Expression::MethodCall(method_call) => {
            let method_name = ctx.interner.resolve(&method_call.method.symbol);
            compile_method_call(
                ctx,
                builder,
                method_call.receiver,
                method_name,
                &method_call.args,
            )
        }

        Expression::StructLiteral(struct_lit) => {
            let struct_name = ctx.interner.resolve(&struct_lit.name.symbol).to_string();

            let struct_def = ctx
                .struct_defs
                .get(&struct_name)
                .ok_or_else(|| {
                    CodegenError::JitCompile(format!("Unknown struct: {}", struct_name))
                })?
                .clone();

            let type_id = builder
                .ins()
                .iconst(cranelift::prelude::types::I32, struct_def.type_id as i64);
            let field_count = builder.ins().iconst(
                cranelift::prelude::types::I32,
                struct_def.fields.len() as i64,
            );

            // Call naml_struct_new(type_id, field_count)
            let struct_ptr = call_struct_new(ctx, builder, type_id, field_count)?;

            // Set each field value
            for field in struct_lit.fields.iter() {
                let field_name = ctx.interner.resolve(&field.name.symbol).to_string();
                // Find field index in struct definition
                let field_idx = struct_def
                    .fields
                    .iter()
                    .position(|f| *f == field_name)
                    .ok_or_else(|| {
                        CodegenError::JitCompile(format!("Unknown field: {}", field_name))
                    })?;

                let mut value = compile_expression(ctx, builder, &field.value)?;
                // Convert string literals to NamlString
                if let Some(Type::String) = ctx.annotations.get_type(field.value.span())
                    && matches!(
                        &field.value,
                        Expression::Literal(LiteralExpr {
                            value: Literal::String(_),
                            ..
                        })
                    )
                {
                    value = call_string_from_cstr(ctx, builder, value)?;
                }
                let idx_val = builder
                    .ins()
                    .iconst(cranelift::prelude::types::I32, field_idx as i64);
                call_struct_set_field(ctx, builder, struct_ptr, idx_val, value)?;
            }

            Ok(struct_ptr)
        }

        Expression::Field(field_expr) => {
            let struct_ptr = compile_expression(ctx, builder, field_expr.base)?;
            let field_name = ctx.interner.resolve(&field_expr.field.symbol).to_string();

            // Use type annotation to determine correct field offset
            // Note: use ident.span (IdentExpr span), not ident.ident.span (Ident span)
            if let Expression::Identifier(ident) = field_expr.base
                && let Some(type_ann) = ctx.annotations.get_type(ident.span)
            {
                if let crate::typechecker::Type::Exception(exc_name) = type_ann {
                    let exc_name_str = ctx.interner.resolve(exc_name).to_string();
                    if let Some(struct_def) = ctx.struct_defs.get(&exc_name_str) {
                        // Exception layout: message at 0, stack at 8, user fields at 16+
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
                        let value = builder.ins().load(
                            cranelift::prelude::types::I64,
                            MemFlags::new(),
                            struct_ptr,
                            offset,
                        );
                        return Ok(value);
                    }
                } else if let crate::typechecker::Type::Struct(struct_type) = type_ann {
                    let struct_name = ctx.interner.resolve(&struct_type.name).to_string();
                    if let Some(struct_def) = ctx.struct_defs.get(&struct_name)
                        && let Some(idx) = struct_def.fields.iter().position(|f| f == &field_name)
                    {
                        let offset = (24 + idx * 8) as i32;
                        let value = builder.ins().load(
                            cranelift::prelude::types::I64,
                            MemFlags::new(),
                            struct_ptr,
                            offset,
                        );
                        return Ok(value);
                    }
                } else if let crate::typechecker::Type::StackFrame = type_ann {
                    // stack_frame: function at 0, file at 8, line at 16
                    let offset = match field_name.as_str() {
                        "function" => 0,
                        "file" => 8,
                        "line" => 16,
                        _ => {
                            return Err(CodegenError::JitCompile(format!(
                                "Unknown stack_frame field: {}",
                                field_name
                            )));
                        }
                    };
                    let value = builder.ins().load(
                        cranelift::prelude::types::I64,
                        MemFlags::new(),
                        struct_ptr,
                        offset,
                    );
                    return Ok(value);
                }
            }

            for (_, struct_def) in ctx.struct_defs.iter() {
                if let Some(field_idx) = struct_def.fields.iter().position(|f| *f == field_name) {
                    let idx_val = builder
                        .ins()
                        .iconst(cranelift::prelude::types::I32, field_idx as i64);
                    return call_struct_get_field(ctx, builder, struct_ptr, idx_val);
                }
            }

            Err(CodegenError::JitCompile(format!(
                "Unknown field: {}",
                field_name
            )))
        }

        Expression::Spawn(_spawn_expr) => {
            // True M:N spawn: schedule the spawn block on the thread pool
            let spawn_id = ctx.current_spawn_id;
            ctx.current_spawn_id += 1;

            let info = ctx
                .spawn_blocks
                .get(&spawn_id)
                .ok_or_else(|| {
                    CodegenError::JitCompile(format!("Spawn block {} not found", spawn_id))
                })?
                .clone();

            let ptr_type = ctx.module.target_config().pointer_type();

            // Calculate closure data size (8 bytes per captured variable)
            let data_size = info.captured_vars.len() * 8;
            let data_size_val = builder
                .ins()
                .iconst(cranelift::prelude::types::I64, data_size as i64);

            // Allocate closure data
            let data_ptr = if data_size > 0 {
                call_alloc_closure_data(ctx, builder, data_size_val)?
            } else {
                builder.ins().iconst(ptr_type, 0)
            };

            // Store captured variables in closure data
            for (i, var_name) in info.captured_vars.iter().enumerate() {
                if let Some(&var) = ctx.variables.get(var_name) {
                    let val = builder.use_var(var);
                    let offset = builder.ins().iconst(ptr_type, (i * 8) as i64);
                    let addr = builder.ins().iadd(data_ptr, offset);
                    builder.ins().store(MemFlags::new(), val, addr, 0);
                }
            }

            // Get trampoline function address
            let trampoline_id = *ctx.functions.get(&info.func_name).ok_or_else(|| {
                CodegenError::JitCompile(format!("Trampoline '{}' not found", info.func_name))
            })?;
            let trampoline_ref = ctx.module.declare_func_in_func(trampoline_id, builder.func);
            let trampoline_addr = builder.ins().func_addr(ptr_type, trampoline_ref);

            // Call spawn_closure to schedule the task
            call_spawn_closure(ctx, builder, trampoline_addr, data_ptr, data_size_val)?;

            // Return unit (0) as spawn expressions don't have a meaningful return value
            Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
        }

        Expression::Some(some_expr) => {
            let inner_val = compile_expression(ctx, builder, some_expr.value)?;

            // Allocate option on stack
            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                16, // option size
                0,
            ));
            let slot_addr = builder
                .ins()
                .stack_addr(cranelift::prelude::types::I64, slot, 0);

            // Tag = 1 (some)
            let tag = builder.ins().iconst(cranelift::prelude::types::I32, 1);
            builder.ins().store(MemFlags::new(), tag, slot_addr, 0);

            // Store inner value at offset 8
            builder
                .ins()
                .store(MemFlags::new(), inner_val, slot_addr, 8);

            Ok(slot_addr)
        }

        Expression::Lambda(_lambda_expr) => {
            // Get lambda info from the tracked lambdas
            let lambda_id = ctx.current_lambda_id;
            ctx.current_lambda_id += 1;

            let info = ctx
                .lambda_blocks
                .get(&lambda_id)
                .ok_or_else(|| CodegenError::JitCompile(format!("Lambda {} not found", lambda_id)))?
                .clone();

            let ptr_type = ctx.module.target_config().pointer_type();

            // Calculate closure data size (8 bytes per captured variable)
            let data_size = info.captured_vars.len() * 8;
            let data_size_val = builder
                .ins()
                .iconst(cranelift::prelude::types::I64, data_size as i64);

            // Allocate closure data
            let data_ptr = if data_size > 0 {
                call_alloc_closure_data(ctx, builder, data_size_val)?
            } else {
                builder.ins().iconst(ptr_type, 0)
            };

            // Store captured variables in closure data (by value)
            for (i, var_name) in info.captured_vars.iter().enumerate() {
                if let Some(&var) = ctx.variables.get(var_name) {
                    let val = builder.use_var(var);
                    let offset = builder.ins().iconst(ptr_type, (i * 8) as i64);
                    let addr = builder.ins().iadd(data_ptr, offset);
                    builder.ins().store(MemFlags::new(), val, addr, 0);
                }
            }

            // Get function pointer
            let lambda_func_id = ctx.functions.get(&info.func_name).ok_or_else(|| {
                CodegenError::JitCompile(format!("Lambda function '{}' not found", info.func_name))
            })?;
            let func_ref = ctx
                .module
                .declare_func_in_func(*lambda_func_id, builder.func);
            let func_addr = builder.ins().func_addr(ptr_type, func_ref);

            // Allocate closure struct on stack: 24 bytes (func_ptr, data_ptr, data_size)
            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                24,
                0,
            ));
            let slot_addr = builder.ins().stack_addr(ptr_type, slot, 0);

            // Store function pointer at offset 0
            builder
                .ins()
                .store(MemFlags::new(), func_addr, slot_addr, 0);

            // Store data pointer at offset 8
            builder.ins().store(MemFlags::new(), data_ptr, slot_addr, 8);

            // Store data size at offset 16
            builder
                .ins()
                .store(MemFlags::new(), data_size_val, slot_addr, 16);

            Ok(slot_addr)
        }

        Expression::Try(try_expr) => {
            // try converts a throwing expression to option<T>
            // Returns some(result) on success, none on exception
            let result = compile_expression(ctx, builder, try_expr.expr)?;

            // Check if an exception occurred
            let has_exception = call_exception_check(ctx, builder)?;

            // Allocate option struct on stack (16 bytes: tag i32 at 0, value i64 at 8)
            let option_slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                16,
                0,
            ));
            let option_ptr =
                builder
                    .ins()
                    .stack_addr(cranelift::prelude::types::I64, option_slot, 0);

            // Create blocks for branching
            let exception_block = builder.create_block();
            let no_exception_block = builder.create_block();
            let merge_block = builder.create_block();

            // Branch based on exception check
            let zero = builder.ins().iconst(cranelift::prelude::types::I64, 0);
            let has_ex = builder.ins().icmp(IntCC::NotEqual, has_exception, zero);
            builder
                .ins()
                .brif(has_ex, exception_block, &[], no_exception_block, &[]);

            // Exception block: clear exception and create none
            builder.switch_to_block(exception_block);
            builder.seal_block(exception_block);
            call_exception_clear(ctx, builder)?;
            let none_tag = builder.ins().iconst(cranelift::prelude::types::I32, 0);
            builder
                .ins()
                .store(MemFlags::new(), none_tag, option_ptr, 0);
            builder.ins().jump(merge_block, &[]);

            // No exception block: create some(result)
            builder.switch_to_block(no_exception_block);
            builder.seal_block(no_exception_block);
            let some_tag = builder.ins().iconst(cranelift::prelude::types::I32, 1);
            builder
                .ins()
                .store(MemFlags::new(), some_tag, option_ptr, 0);
            builder.ins().store(MemFlags::new(), result, option_ptr, 8);
            builder.ins().jump(merge_block, &[]);

            builder.switch_to_block(merge_block);
            builder.seal_block(merge_block);

            Ok(option_ptr)
        }

        Expression::Catch(catch_expr) => {
            // Get the expression type to handle Bool correctly
            let expr_type = ctx.annotations.get_type(catch_expr.expr.span());
            let is_bool_type = matches!(expr_type, Some(Type::Bool));

            // Compile the expression that might throw
            let result = compile_expression(ctx, builder, catch_expr.expr)?;

            // Check if an exception occurred
            let has_exception = call_exception_check(ctx, builder)?;

            // Create blocks for branching
            let exception_block = builder.create_block();
            let no_exception_block = builder.create_block();
            let merge_block = builder.create_block();

            // Use the appropriate type for the merge block based on expression type
            let merge_type = expr_type
                .map(|t| tc_type_to_cranelift(&t))
                .unwrap_or(cranelift::prelude::types::I64);
            builder.append_block_param(merge_block, merge_type);

            // Branch based on exception check
            let zero = builder.ins().iconst(cranelift::prelude::types::I64, 0);
            let has_ex = builder.ins().icmp(IntCC::NotEqual, has_exception, zero);
            builder
                .ins()
                .brif(has_ex, exception_block, &[], no_exception_block, &[]);

            // Exception block: get exception, bind to variable, run handler
            builder.switch_to_block(exception_block);
            builder.seal_block(exception_block);

            // Get the exception pointer and bind to the error variable
            let exception_ptr = call_exception_get(ctx, builder)?;
            let error_var_name = ctx
                .interner
                .resolve(&catch_expr.error_binding.symbol)
                .to_string();

            // Check if variable already exists (for multiple catch blocks with same binding name)
            let error_var = if let Some(&existing_var) = ctx.variables.get(&error_var_name) {
                existing_var
            } else {
                let new_var = Variable::new(ctx.var_counter);
                ctx.var_counter += 1;
                ctx.variables.insert(error_var_name, new_var);
                builder.declare_var(new_var, cranelift::prelude::types::I64);
                new_var
            };
            builder.def_var(error_var, exception_ptr);

            // Clear the exception so it doesn't propagate
            call_exception_clear(ctx, builder)?;

            // Compile the handler block statements
            for stmt in &catch_expr.handler.statements {
                compile_statement(ctx, builder, stmt)?;
                if ctx.block_terminated {
                    break;
                }
            }

            // If handler didn't return/throw, check for tail expression or use default
            if !ctx.block_terminated {
                let handler_value = if let Some(tail) = catch_expr.handler.tail {
                    let val = compile_expression(ctx, builder, tail)?;
                    // Convert to correct type if needed
                    if is_bool_type {
                        builder.ins().ireduce(cranelift::prelude::types::I8, val)
                    } else {
                        val
                    }
                } else {
                    // No tail expression - use 0 as default value with correct type
                    builder.ins().iconst(merge_type, 0)
                };
                builder.ins().jump(merge_block, &[handler_value]);
            }
            ctx.block_terminated = false;

            // No exception block: jump to merge with the result
            builder.switch_to_block(no_exception_block);
            builder.seal_block(no_exception_block);
            // Convert result to correct type if Bool (runtime returns I64, but Bool needs I8)
            let result_converted = if is_bool_type {
                builder.ins().ireduce(cranelift::prelude::types::I8, result)
            } else {
                result
            };
            builder.ins().jump(merge_block, &[result_converted]);

            // Merge block - returns the value directly (not wrapped in option)
            builder.switch_to_block(merge_block);
            builder.seal_block(merge_block);
            let final_result = builder.block_params(merge_block)[0];
            Ok(final_result)
        }

        Expression::Cast(cast_expr) => {
            // Evaluate the expression to cast
            let value = compile_expression(ctx, builder, cast_expr.expr)?;

            // Get source and target types
            let source_type = ctx.annotations.get_type(cast_expr.expr.span());

            match &cast_expr.target_ty {
                NamlType::Int => match source_type {
                    Some(Type::Float) => Ok(builder
                        .ins()
                        .fcvt_to_sint(cranelift::prelude::types::I64, value)),
                    Some(Type::String) => call_string_to_int(ctx, builder, value),
                    Some(Type::Uint) | Some(Type::Int) => Ok(value),
                    _ => Ok(value),
                },
                NamlType::Uint => match source_type {
                    Some(Type::Float) => Ok(builder
                        .ins()
                        .fcvt_to_uint(cranelift::prelude::types::I64, value)),
                    Some(Type::Int) | Some(Type::Uint) => Ok(value),
                    _ => Ok(value),
                },
                NamlType::Float => match source_type {
                    Some(Type::Int) => Ok(builder
                        .ins()
                        .fcvt_from_sint(cranelift::prelude::types::F64, value)),
                    Some(Type::Uint) => Ok(builder
                        .ins()
                        .fcvt_from_uint(cranelift::prelude::types::F64, value)),
                    Some(Type::String) => call_string_to_float(ctx, builder, value),
                    Some(Type::Float) => Ok(value),
                    _ => Ok(value),
                },
                NamlType::String => match source_type {
                    Some(Type::Int) | Some(Type::Uint) => call_int_to_string(ctx, builder, value),
                    Some(Type::Float) => call_float_to_string(ctx, builder, value),
                    Some(Type::Bytes) => call_bytes_to_string(ctx, builder, value),
                    Some(Type::String) => Ok(value),
                    _ => Ok(value),
                },
                NamlType::Bytes => match source_type {
                    Some(Type::String) => {
                        // Convert string literal (C string) to NamlString first
                        let mut str_val = value;
                        if matches!(cast_expr.expr, Expression::Literal(LiteralExpr { value: Literal::String(_), .. })) {
                            str_val = call_string_from_cstr(ctx, builder, value)?;
                        }
                        call_string_to_bytes(ctx, builder, str_val)
                    }
                    Some(Type::Bytes) => Ok(value),
                    _ => Ok(value),
                },
                _ => {
                    // For other casts, just pass through the value
                    Ok(value)
                }
            }
        }

        Expression::FallibleCast(cast_expr) => {
            // Fallible cast: returns option<T> as tagged struct
            // Options are 16-byte structs: tag (i32) at offset 0, value (i64) at offset 8
            // Tag: 0 = none, 1 = some
            let mut value = compile_expression(ctx, builder, cast_expr.expr)?;
            let source_type = ctx.annotations.get_type(cast_expr.expr.span());

            // If source is a string literal, wrap it as NamlString*
            // String literals compile to raw C-string pointers, but runtime expects NamlString*
            if matches!(source_type, Some(Type::String)) {
                if let Expression::Literal(LiteralExpr {
                                               value: Literal::String(_),
                                               ..
                                           }) = cast_expr.expr
                {
                    value = call_string_from_cstr(ctx, builder, value)?;
                }
            }

            // Allocate option struct on stack (16 bytes)
            let option_slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                16,
                0,
            ));
            let option_ptr =
                builder
                    .ins()
                    .stack_addr(cranelift::prelude::types::I64, option_slot, 0);

            match (&cast_expr.target_ty, source_type) {
                (NamlType::Int, Some(Type::String)) => {
                    // String to int fallible conversion
                    let value_slot = builder.create_sized_stack_slot(StackSlotData::new(
                        StackSlotKind::ExplicitSlot,
                        8,
                        0,
                    ));
                    let value_ptr =
                        builder
                            .ins()
                            .stack_addr(cranelift::prelude::types::I64, value_slot, 0);

                    let func_ref = rt_func_ref(ctx, builder, "naml_string_try_to_int")?;
                    let call = builder.ins().call(func_ref, &[value, value_ptr]);
                    let success = builder.inst_results(call)[0];

                    // Create blocks for conditional handling
                    let success_block = builder.create_block();
                    let fail_block = builder.create_block();
                    let merge_block = builder.create_block();

                    let zero = builder.ins().iconst(cranelift::prelude::types::I64, 0);
                    let is_success = builder.ins().icmp(IntCC::NotEqual, success, zero);
                    builder
                        .ins()
                        .brif(is_success, success_block, &[], fail_block, &[]);

                    // Success: create some(parsed_value)
                    builder.switch_to_block(success_block);
                    builder.seal_block(success_block);
                    let some_tag = builder.ins().iconst(cranelift::prelude::types::I32, 1);
                    builder
                        .ins()
                        .store(MemFlags::new(), some_tag, option_ptr, 0);
                    let parsed_value = builder.ins().load(
                        cranelift::prelude::types::I64,
                        MemFlags::new(),
                        value_ptr,
                        0,
                    );
                    builder
                        .ins()
                        .store(MemFlags::new(), parsed_value, option_ptr, 8);
                    builder.ins().jump(merge_block, &[]);

                    // Failure: create none
                    builder.switch_to_block(fail_block);
                    builder.seal_block(fail_block);
                    let none_tag = builder.ins().iconst(cranelift::prelude::types::I32, 0);
                    builder
                        .ins()
                        .store(MemFlags::new(), none_tag, option_ptr, 0);
                    builder.ins().jump(merge_block, &[]);

                    builder.switch_to_block(merge_block);
                    builder.seal_block(merge_block);
                    Ok(option_ptr)
                }
                (NamlType::Float, Some(Type::String)) => {
                    // String to float fallible conversion
                    let value_slot = builder.create_sized_stack_slot(StackSlotData::new(
                        StackSlotKind::ExplicitSlot,
                        8,
                        0,
                    ));
                    let value_ptr =
                        builder
                            .ins()
                            .stack_addr(cranelift::prelude::types::I64, value_slot, 0);

                    let func_ref = rt_func_ref(ctx, builder, "naml_string_try_to_float")?;
                    let call = builder.ins().call(func_ref, &[value, value_ptr]);
                    let success = builder.inst_results(call)[0];

                    // Create blocks for conditional handling
                    let success_block = builder.create_block();
                    let fail_block = builder.create_block();
                    let merge_block = builder.create_block();

                    let zero = builder.ins().iconst(cranelift::prelude::types::I64, 0);
                    let is_success = builder.ins().icmp(IntCC::NotEqual, success, zero);
                    builder
                        .ins()
                        .brif(is_success, success_block, &[], fail_block, &[]);

                    // Success: create some(parsed_value)
                    builder.switch_to_block(success_block);
                    builder.seal_block(success_block);
                    let some_tag = builder.ins().iconst(cranelift::prelude::types::I32, 1);
                    builder
                        .ins()
                        .store(MemFlags::new(), some_tag, option_ptr, 0);
                    let parsed_value = builder.ins().load(
                        cranelift::prelude::types::I64,
                        MemFlags::new(),
                        value_ptr,
                        0,
                    );
                    builder
                        .ins()
                        .store(MemFlags::new(), parsed_value, option_ptr, 8);
                    builder.ins().jump(merge_block, &[]);

                    // Failure: create none
                    builder.switch_to_block(fail_block);
                    builder.seal_block(fail_block);
                    let none_tag = builder.ins().iconst(cranelift::prelude::types::I32, 0);
                    builder
                        .ins()
                        .store(MemFlags::new(), none_tag, option_ptr, 0);
                    builder.ins().jump(merge_block, &[]);

                    builder.switch_to_block(merge_block);
                    builder.seal_block(merge_block);
                    Ok(option_ptr)
                }
                _ => {
                    // For other conversions, wrap value in some()
                    let some_tag = builder.ins().iconst(cranelift::prelude::types::I32, 1);
                    builder
                        .ins()
                        .store(MemFlags::new(), some_tag, option_ptr, 0);
                    builder.ins().store(MemFlags::new(), value, option_ptr, 8);
                    Ok(option_ptr)
                }
            }
        }

        Expression::Ternary(ternary) => {
            // Compile: condition ? true_expr : false_expr
            let cond = compile_expression(ctx, builder, ternary.condition)?;

            // Create blocks for branching
            let then_block = builder.create_block();
            let else_block = builder.create_block();
            let merge_block = builder.create_block();

            builder.append_block_param(merge_block, cranelift::prelude::types::I64);

            // Branch on condition (condition is already a boolean value)
            builder.ins().brif(cond, then_block, &[], else_block, &[]);

            // Then block: evaluate true expression
            builder.switch_to_block(then_block);
            builder.seal_block(then_block);
            let true_val = compile_expression(ctx, builder, ternary.true_expr)?;
            builder.ins().jump(merge_block, &[true_val]);

            // Else block: evaluate false expression
            builder.switch_to_block(else_block);
            builder.seal_block(else_block);
            let false_val = compile_expression(ctx, builder, ternary.false_expr)?;
            builder.ins().jump(merge_block, &[false_val]);

            // Merge block: result is block parameter
            builder.switch_to_block(merge_block);
            builder.seal_block(merge_block);
            let result = builder.block_params(merge_block)[0];
            Ok(result)
        }

        Expression::Elvis(elvis) => {
            // Compile: left ?: right
            // Returns left if truthy, otherwise right
            let left = compile_expression(ctx, builder, elvis.left)?;

            // Create blocks for branching
            let then_block = builder.create_block();
            let else_block = builder.create_block();
            let merge_block = builder.create_block();

            builder.append_block_param(merge_block, cranelift::prelude::types::I64);

            // Check if left is falsy (zero/null)
            let zero = builder.ins().iconst(cranelift::prelude::types::I64, 0);
            let is_falsy = builder.ins().icmp(IntCC::Equal, left, zero);
            builder
                .ins()
                .brif(is_falsy, else_block, &[], then_block, &[]);

            // Then block: left is truthy, use left
            builder.switch_to_block(then_block);
            builder.seal_block(then_block);
            builder.ins().jump(merge_block, &[left]);

            // Else block: left is falsy, evaluate and use right
            builder.switch_to_block(else_block);
            builder.seal_block(else_block);
            let right = compile_expression(ctx, builder, elvis.right)?;
            builder.ins().jump(merge_block, &[right]);

            // Merge block: result is block parameter
            builder.switch_to_block(merge_block);
            builder.seal_block(merge_block);
            let result = builder.block_params(merge_block)[0];
            Ok(result)
        }

        Expression::ForceUnwrap(unwrap_expr) => {
            if let Expression::Index(index_expr) = unwrap_expr.expr {
                let base = compile_expression(ctx, builder, index_expr.base)?;

                // Check if this is a map access (string key) or array access (integer index)
                if let Expression::Literal(LiteralExpr {
                                               value: Literal::String(_),
                                               ..
                                           }) = index_expr.index
                {
                    let cstr_ptr = compile_expression(ctx, builder, index_expr.index)?;
                    let naml_str = call_string_from_cstr(ctx, builder, cstr_ptr)?;
                    return compile_direct_map_get_or_panic(ctx, builder, base, naml_str);
                } else {
                    // Array access: arr[index]!
                    let index = compile_expression(ctx, builder, index_expr.index)?;
                    return compile_direct_array_get_or_panic(ctx, builder, base, index);
                }
            }

            // General case: compile the option expression and unwrap
            let option_ptr = compile_expression(ctx, builder, unwrap_expr.expr)?;

            // Load the tag from offset 0 (0 = none, 1 = some)
            let tag = builder.ins().load(
                cranelift::prelude::types::I32,
                MemFlags::new(),
                option_ptr,
                0,
            );

            // Create blocks for conditional handling
            let some_block = builder.create_block();
            let none_block = builder.create_block();
            let merge_block = builder.create_block();
            builder.append_block_param(merge_block, cranelift::prelude::types::I64);

            // Check if tag == 0 (none)
            let is_none = builder.ins().icmp_imm(IntCC::Equal, tag, 0);
            builder
                .ins()
                .brif(is_none, none_block, &[], some_block, &[]);

            // None block: panic with error message
            builder.switch_to_block(none_block);
            builder.seal_block(none_block);
            let panic_func = rt_func_ref(ctx, builder, "naml_panic_unwrap")?;
            builder.ins().call(panic_func, &[]);
            // Panic doesn't return, but we need to provide a value for the block
            let zero = builder.ins().iconst(cranelift::prelude::types::I64, 0);
            builder.ins().jump(merge_block, &[zero]);

            // Some block: extract the value from offset 8
            builder.switch_to_block(some_block);
            builder.seal_block(some_block);
            let inner_value = builder.ins().load(
                cranelift::prelude::types::I64,
                MemFlags::new(),
                option_ptr,
                8,
            );
            builder.ins().jump(merge_block, &[inner_value]);

            // Merge block
            builder.switch_to_block(merge_block);
            builder.seal_block(merge_block);
            Ok(builder.block_params(merge_block)[0])
        }

        Expression::TemplateString(template) => {
            compile_template_string(ctx, builder, template)
        }

        _ => Err(CodegenError::Unsupported(format!(
            "Expression type not yet implemented: {:?}",
            std::mem::discriminant(expr)
        ))),
    }
}

fn compile_template_string(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    template: &crate::ast::TemplateStringExpr,
) -> Result<Value, CodegenError> {
    use crate::codegen::cranelift::heap::HeapType;
    use crate::lexer::tokenize;

    let mut result: Option<Value> = None;

    for part in &template.parts {
        let part_value = match part {
            TemplateStringPart::Literal(s) => {
                // Compile literal string part
                let ptr = compile_string_literal(ctx, builder, s)?;
                call_string_from_cstr(ctx, builder, ptr)?
            }
            TemplateStringPart::Expression(expr_str) => {
                // Tokenize the expression to get the identifier
                let (tokens, interner) = tokenize(expr_str);

                // Filter out whitespace, newline, and EOF tokens
                let tokens: Vec<_> = tokens.into_iter()
                    .filter(|t| !matches!(t.kind,
                        crate::lexer::TokenKind::Eof |
                        crate::lexer::TokenKind::Whitespace |
                        crate::lexer::TokenKind::Newline
                    ))
                    .collect();

                if tokens.is_empty() {
                    // Empty expression, return empty string
                    let ptr = compile_string_literal(ctx, builder, "")?;
                    call_string_from_cstr(ctx, builder, ptr)?
                } else if tokens.len() == 1 && tokens[0].kind == crate::lexer::TokenKind::Ident {
                    // Simple identifier case - look up and convert to string
                    let ident_spur = tokens[0].symbol.unwrap();
                    let ident_name = interner.resolve(&ident_spur);

                    // Look up the variable in the current context
                    if let Some(&var) = ctx.variables.get(ident_name) {
                        let val = builder.use_var(var);

                        // Check if this is a heap type (like string)
                        if let Some(heap_type) = ctx.var_heap_types.get(ident_name) {
                            match heap_type {
                                HeapType::String => val, // Already a string pointer
                                _ => call_int_to_string(ctx, builder, val)?, // Other heap types
                            }
                        } else {
                            // Not a heap type - check Cranelift type
                            let var_type = builder.func.dfg.value_type(val);
                            if var_type == cranelift::prelude::types::F64 {
                                call_float_to_string(ctx, builder, val)?
                            } else if var_type == cranelift::prelude::types::I8 {
                                // Bool type
                                let true_str = compile_string_literal(ctx, builder, "true")?;
                                let true_naml = call_string_from_cstr(ctx, builder, true_str)?;
                                let false_str = compile_string_literal(ctx, builder, "false")?;
                                let false_naml = call_string_from_cstr(ctx, builder, false_str)?;
                                builder.ins().select(val, true_naml, false_naml)
                            } else {
                                call_int_to_string(ctx, builder, val)?
                            }
                        }
                    } else {
                        // Variable not found, return as literal
                        let ptr = compile_string_literal(ctx, builder, &format!("{{{}}}", expr_str))?;
                        call_string_from_cstr(ctx, builder, ptr)?
                    }
                } else {
                    // Complex expression - not yet fully supported
                    // For now, output as literal with braces
                    let ptr = compile_string_literal(ctx, builder, &format!("{{{}}}", expr_str))?;
                    call_string_from_cstr(ctx, builder, ptr)?
                }
            }
        };

        result = Some(match result {
            Some(acc) => call_string_concat(ctx, builder, acc, part_value)?,
            None => part_value,
        });
    }

    // Handle empty template string
    match result {
        Some(value) => Ok(value),
        None => {
            let ptr = compile_string_literal(ctx, builder, "")?;
            call_string_from_cstr(ctx, builder, ptr)
        }
    }
}