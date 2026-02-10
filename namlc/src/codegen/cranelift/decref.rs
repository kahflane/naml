use std::panic;

use cranelift::prelude::*;
use cranelift_codegen::ir::{AtomicRmwOp, StackSlotData, StackSlotKind};
use cranelift_module::{FuncId, Linkage, Module};
use lasso::Spur;

use crate::codegen::CodegenError;
use crate::codegen::cranelift::{JitCompiler, StructDef};
use crate::codegen::cranelift::errors::convert_cranelift_error;
use crate::codegen::cranelift::heap::HeapType;
use crate::codegen::cranelift::structs::{struct_has_heap_fields, emit_inline_arena_free};

impl<'a> JitCompiler<'a> {
    pub fn generate_struct_decref_functions(&mut self) -> Result<(), CodegenError> {
        let ptr_type = self.module.target_config().pointer_type();

        let structs_with_heap_fields: Vec<(Spur, StructDef)> = self
            .struct_defs
            .iter()
            .filter(|(_, def)| def.field_heap_types.iter().any(|ht| ht.is_some()))
            .map(|(name, def)| (*name, def.clone()))
            .collect();

        for (struct_name_spur, _) in &structs_with_heap_fields {
            let struct_name = self.interner.resolve(struct_name_spur);
            let func_name = format!("naml_struct_decref_{}", struct_name);
            let mut sig = self.module.make_signature();
            sig.params.push(AbiParam::new(ptr_type));
            let func_id = self
                .module
                .declare_function(&func_name, Linkage::Local, &sig)
                .map_err(|e| {
                    CodegenError::JitCompile(format!("Failed to declare {}: {}", func_name, e))
                })?;
            self.functions.insert(func_name, func_id);
        }

        for (struct_name_spur, struct_def) in structs_with_heap_fields {
            let struct_name = self.interner.resolve(&struct_name_spur).to_string();
            self.generate_struct_decref(&struct_name, &struct_def)?;
        }

        Ok(())
    }

    fn generate_struct_decref_iterative_wrapper(
        &mut self,
        func_name: &str,
        func_id: FuncId,
        heap_field_indices: &[usize],
    ) -> Result<(), CodegenError> {
        let ptr_type = self.module.target_config().pointer_type();

        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(ptr_type));
        self.ctx.func.signature = sig;

        let mut builder_ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);

        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        let struct_ptr = builder.block_params(entry_block)[0];

        let num_fields = heap_field_indices.len();
        let slot = builder.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            (num_fields * 4) as u32,
            4,
        ));

        for (i, &field_idx) in heap_field_indices.iter().enumerate() {
            let val = builder
                .ins()
                .iconst(cranelift::prelude::types::I32, field_idx as i64);
            builder.ins().stack_store(val, slot, (i * 4) as i32);
        }

        let indices_ptr = builder.ins().stack_addr(ptr_type, slot, 0);
        let num_fields_val = builder
            .ins()
            .iconst(cranelift::prelude::types::I32, num_fields as i64);

        let iter_func_id =
            *self
                .runtime_funcs
                .get("naml_struct_decref_iterative")
                .ok_or_else(|| {
                    CodegenError::JitCompile(
                        "naml_struct_decref_iterative not declared".to_string(),
                    )
                })?;
        let iter_func_ref = self
            .module
            .declare_func_in_func(iter_func_id, builder.func);
        builder
            .ins()
            .call(iter_func_ref, &[struct_ptr, indices_ptr, num_fields_val]);

        builder.ins().return_(&[]);
        builder.finalize();

        let func_name_clone = func_name.to_string();
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            self.module.define_function(func_id, &mut self.ctx)
        }));

        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                return Err(CodegenError::JitCompile(format!(
                    "Failed to define {}: {}",
                    func_name, e
                )));
            }
            Err(panic_info) => {
                let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown internal error".to_string()
                };
                return Err(convert_cranelift_error(&panic_msg, &func_name_clone));
            }
        }

        self.ctx.clear();
        Ok(())
    }

    fn generate_struct_decref_loop(
        &mut self,
        func_name: &str,
        func_id: FuncId,
        struct_def: &StructDef,
        self_ref_fields: &[usize],
    ) -> Result<(), CodegenError> {
        let ptr_type = self.module.target_config().pointer_type();

        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(ptr_type));
        self.ctx.func.signature = sig;

        let mut builder_ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);

        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);

        let struct_ptr = builder.block_params(entry_block)[0];

        let loop_header = builder.create_block();
        builder.append_block_param(loop_header, ptr_type);
        builder.ins().jump(loop_header, &[struct_ptr]);
        builder.seal_block(entry_block);

        builder.switch_to_block(loop_header);
        let current_ptr = builder.block_params(loop_header)[0];

        let zero = builder.ins().iconst(ptr_type, 0);
        let is_null = builder.ins().icmp(IntCC::Equal, current_ptr, zero);

        let done_block = builder.create_block();
        let check_rc_block = builder.create_block();

        builder
            .ins()
            .brif(is_null, done_block, &[], check_rc_block, &[]);

        builder.switch_to_block(check_rc_block);
        builder.seal_block(check_rc_block);

        let one = builder.ins().iconst(cranelift::prelude::types::I64, 1);
        let free_block = builder.create_block();

        if self.unsafe_mode {
            let current_rc = builder.ins().load(
                cranelift::prelude::types::I64,
                MemFlags::new(),
                current_ptr,
                0,
            );
            let should_free = builder.ins().icmp(IntCC::Equal, current_rc, one);
            let decrement_block = builder.create_block();
            builder
                .ins()
                .brif(should_free, free_block, &[], decrement_block, &[]);

            builder.switch_to_block(decrement_block);
            builder.seal_block(decrement_block);
            let decremented = builder.ins().iadd_imm(current_rc, -1);
            builder
                .ins()
                .store(MemFlags::new(), decremented, current_ptr, 0);
            builder.ins().jump(done_block, &[]);
        } else {
            let old_rc = builder.ins().atomic_rmw(
                cranelift::prelude::types::I64,
                MemFlags::new(),
                AtomicRmwOp::Sub,
                current_ptr,
                one,
            );
            let should_free = builder.ins().icmp(IntCC::Equal, old_rc, one);
            builder
                .ins()
                .brif(should_free, free_block, &[], done_block, &[]);
        }

        builder.switch_to_block(free_block);
        builder.seal_block(free_block);
        if !self.unsafe_mode {
            builder.ins().fence();
        }

        let base_field_offset: i32 = 24;
        let mut child_vals = Vec::new();
        for &field_idx in self_ref_fields {
            let field_offset = base_field_offset + (field_idx as i32 * 8);
            let field_val = builder.ins().load(
                cranelift::prelude::types::I64,
                MemFlags::new(),
                current_ptr,
                field_offset,
            );
            child_vals.push(field_val);
        }

        let free_func_id = *self.runtime_funcs.get("naml_struct_free").ok_or_else(|| {
            CodegenError::JitCompile("Unknown runtime function: naml_struct_free".to_string())
        })?;
        let free_func_ref = self.module.declare_func_in_func(free_func_id, builder.func);
        builder.ins().call(free_func_ref, &[current_ptr]);

        let self_func_id = *self.functions.get(func_name).ok_or_else(|| {
            CodegenError::JitCompile(format!("Function not found: {}", func_name))
        })?;
        let self_func_ref = self.module.declare_func_in_func(self_func_id, builder.func);

        for child in &child_vals[..child_vals.len() - 1] {
            builder.ins().call(self_func_ref, &[*child]);
        }

        let last_child = *child_vals.last().unwrap();
        builder.ins().jump(loop_header, &[last_child]);
        builder.seal_block(loop_header);

        builder.switch_to_block(done_block);
        builder.seal_block(done_block);
        builder.ins().return_(&[]);

        builder.finalize();

        let func_name_clone = func_name.to_string();
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            self.module.define_function(func_id, &mut self.ctx)
        }));

        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                return Err(CodegenError::JitCompile(format!(
                    "Failed to define {}: {}",
                    func_name, e
                )));
            }
            Err(panic_info) => {
                let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown internal error".to_string()
                };
                return Err(convert_cranelift_error(&panic_msg, &func_name_clone));
            }
        }

        self.ctx.clear();
        Ok(())
    }

    fn is_self_ref_heap_field(ht: &HeapType, struct_name: &str, interner: &lasso::Rodeo) -> bool {
        match ht {
            HeapType::Struct(Some(name)) => interner.resolve(name) == struct_name,
            HeapType::OptionOf(inner) => Self::is_self_ref_heap_field(inner, struct_name, interner),
            _ => false,
        }
    }

    fn generate_struct_decref(
        &mut self,
        struct_name: &str,
        struct_def: &StructDef,
    ) -> Result<(), CodegenError> {
        let ptr_type = self.module.target_config().pointer_type();
        let func_name = format!("naml_struct_decref_{}", struct_name);

        let func_id = *self.functions.get(&func_name).ok_or_else(|| {
            CodegenError::JitCompile(format!("Decref function not pre-declared: {}", func_name))
        })?;

        let self_ref_fields: Vec<usize> = struct_def
            .field_heap_types
            .iter()
            .enumerate()
            .filter_map(|(i, ht)| {
                ht.as_ref().and_then(|h| {
                    if Self::is_self_ref_heap_field(h, struct_name, self.interner) {
                        Some(i)
                    } else {
                        None
                    }
                })
            })
            .collect();

        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(ptr_type));
        self.ctx.func.signature = sig;

        let mut builder_ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);

        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        let struct_ptr = builder.block_params(entry_block)[0];

        // Check if ptr is null
        let zero = builder.ins().iconst(ptr_type, 0);
        let is_null = builder.ins().icmp(IntCC::Equal, struct_ptr, zero);

        let null_block = builder.create_block();
        let decref_block = builder.create_block();

        builder
            .ins()
            .brif(is_null, null_block, &[], decref_block, &[]);

        // Null case: just return
        builder.switch_to_block(null_block);
        builder.seal_block(null_block);
        builder.ins().return_(&[]);

        // Non-null case: decref the struct
        builder.switch_to_block(decref_block);
        builder.seal_block(decref_block);

        let one = builder.ins().iconst(cranelift::prelude::types::I64, 1);

        let free_block = builder.create_block();
        let decrement_block = builder.create_block();
        let done_block = builder.create_block();

        if self.unsafe_mode {
            let current = builder.ins().load(
                cranelift::prelude::types::I64,
                MemFlags::new(),
                struct_ptr,
                0,
            );
            let should_free = builder.ins().icmp(IntCC::Equal, current, one);
            builder
                .ins()
                .brif(should_free, free_block, &[], decrement_block, &[]);

            // Decrement-only path (rc > 1): store decremented value and return
            builder.switch_to_block(decrement_block);
            builder.seal_block(decrement_block);
            let decremented = builder.ins().iadd_imm(current, -1);
            builder
                .ins()
                .store(MemFlags::new(), decremented, struct_ptr, 0);
            builder.ins().jump(done_block, &[]);
        } else {
            let old_refcount = builder.ins().atomic_rmw(
                cranelift::prelude::types::I64,
                MemFlags::new(),
                AtomicRmwOp::Sub,
                struct_ptr,
                one,
            );
            let should_free = builder.ins().icmp(IntCC::Equal, old_refcount, one);
            builder
                .ins()
                .brif(should_free, free_block, &[], done_block, &[]);

            builder.switch_to_block(decrement_block);
            builder.seal_block(decrement_block);
            builder.ins().jump(done_block, &[]);
        }

        // Free path: skip refcount store (memory is about to be reused)
        builder.switch_to_block(free_block);
        builder.seal_block(free_block);
        if !self.unsafe_mode {
            builder.ins().fence();
        }

        // Struct memory layout after header:
        // - type_id: u32 (offset 16)
        // - field_count: u32 (offset 20)
        // - fields[]: i64 (offset 24+)
        let base_field_offset: i32 = 24; // sizeof(HeapHeader) + type_id + field_count

        for (field_idx, heap_type) in struct_def.field_heap_types.iter().enumerate() {
            if let Some(ht) = heap_type {
                let field_offset = base_field_offset + (field_idx as i32 * 8);
                let field_val = builder.ins().load(
                    cranelift::prelude::types::I64,
                    MemFlags::new(),
                    struct_ptr,
                    field_offset,
                );

                let field_is_null = builder.ins().icmp(IntCC::Equal, field_val, zero);
                let decref_field_block = builder.create_block();
                let next_field_block = builder.create_block();

                builder.ins().brif(
                    field_is_null,
                    next_field_block,
                    &[],
                    decref_field_block,
                    &[],
                );
                builder.switch_to_block(decref_field_block);
                builder.seal_block(decref_field_block);

                if let HeapType::OptionOf(inner) = ht {
                    let inner_func_name: String = match inner.as_ref() {
                        HeapType::String => "naml_string_decref".to_string(),
                        HeapType::Array(_) => "naml_array_decref".to_string(),
                        HeapType::Map(_) => "naml_map_decref".to_string(),
                        HeapType::Struct(None) => "naml_struct_decref".to_string(),
                        HeapType::Struct(Some(name)) => {
                            if struct_has_heap_fields(&self.struct_defs, name) {
                                format!("naml_struct_decref_{}", self.interner.resolve(name))
                            } else {
                                "naml_struct_decref".to_string()
                            }
                        }
                        HeapType::OptionOf(_) => "naml_struct_decref".to_string(),
                    };

                    let inner_func_id = self
                        .runtime_funcs
                        .get(inner_func_name.as_str())
                        .or_else(|| self.functions.get(inner_func_name.as_str()))
                        .copied()
                        .ok_or_else(|| {
                            CodegenError::JitCompile(format!(
                                "Unknown decref function: {}",
                                inner_func_name
                            ))
                        })?;

                    let inner_func_ref = self
                        .module
                        .declare_func_in_func(inner_func_id, builder.func);
                    builder.ins().call(inner_func_ref, &[field_val]);
                    builder.ins().jump(next_field_block, &[]);
                } else {
                    let decref_func_name: String = match ht {
                        HeapType::String => "naml_string_decref".to_string(),
                        HeapType::Array(None) => "naml_array_decref".to_string(),
                        HeapType::Array(Some(elem_type)) => match elem_type.as_ref() {
                            HeapType::String => "naml_array_decref_strings".to_string(),
                            HeapType::Array(_) => "naml_array_decref_arrays".to_string(),
                            HeapType::Map(_) => "naml_array_decref_maps".to_string(),
                            HeapType::Struct(_) => "naml_array_decref_structs".to_string(),
                            HeapType::OptionOf(_) => "naml_array_decref".to_string(),
                        },
                        HeapType::Map(None) => "naml_map_decref".to_string(),
                        HeapType::Map(Some(val_type)) => match val_type.as_ref() {
                            HeapType::String => "naml_map_decref_strings".to_string(),
                            HeapType::Array(_) => "naml_map_decref_arrays".to_string(),
                            HeapType::Map(_) => "naml_map_decref_maps".to_string(),
                            HeapType::Struct(_) => "naml_map_decref_structs".to_string(),
                            HeapType::OptionOf(_) => "naml_map_decref".to_string(),
                        },
                        HeapType::Struct(None) => "naml_struct_decref".to_string(),
                        HeapType::Struct(Some(field_struct_name)) => {
                            if struct_has_heap_fields(&self.struct_defs, field_struct_name) {
                                format!("naml_struct_decref_{}", self.interner.resolve(field_struct_name))
                            } else {
                                "naml_struct_decref".to_string()
                            }
                        }
                        HeapType::OptionOf(_) => unreachable!("OptionOf handled above"),
                    };

                    let decref_func_id = self
                        .runtime_funcs
                        .get(decref_func_name.as_str())
                        .or_else(|| self.functions.get(decref_func_name.as_str()))
                        .copied()
                        .ok_or_else(|| {
                            CodegenError::JitCompile(format!(
                                "Unknown decref function: {}",
                                decref_func_name
                            ))
                        })?;
                    let decref_func_ref = self
                        .module
                        .declare_func_in_func(decref_func_id, builder.func);
                    builder.ins().call(decref_func_ref, &[field_val]);
                    builder.ins().jump(next_field_block, &[]);
                }

                builder.switch_to_block(next_field_block);
                builder.seal_block(next_field_block);
            }
        }

        let alloc_size = 24 + struct_def.fields.len() * 8;
        emit_inline_arena_free(&mut self.module, &self.runtime_funcs, &mut builder, struct_ptr, alloc_size)?;
        builder.ins().jump(done_block, &[]);

        // Done block: return
        builder.switch_to_block(done_block);
        builder.seal_block(done_block);
        builder.ins().return_(&[]);

        builder.finalize();

        let func_name_clone = func_name.clone();
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            self.module.define_function(func_id, &mut self.ctx)
        }));

        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                return Err(CodegenError::JitCompile(format!(
                    "Failed to define {}: {}",
                    func_name, e
                )));
            }
            Err(panic_info) => {
                let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown internal error".to_string()
                };
                return Err(convert_cranelift_error(&panic_msg, &func_name_clone));
            }
        }

        self.ctx.clear();

        Ok(())
    }
}
