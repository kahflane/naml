use std::path::Path;

use cranelift_module::Linkage;
use lasso::Rodeo;

use crate::ast::{Expression, Item, SourceFile, Statement};
use crate::codegen::CodegenError;
use crate::codegen::cranelift::heap::{self, get_heap_type_resolved};
use crate::codegen::cranelift::{
    types, EnumDef, EnumVariantDef, ExternFn, GlobalVarDef, JitCompiler, StructDef,
};
use crate::typechecker::TypeAnnotations;

impl<'a> JitCompiler<'a> {
    pub fn compile(&mut self, ast: &'a SourceFile<'a>) -> Result<(), CodegenError> {
        for item in &ast.items {
            if let crate::ast::Item::Struct(struct_item) = item {
                let name_spur = struct_item.name.symbol;
                let mut fields = Vec::new();
                let mut field_heap_types = Vec::new();

                for f in &struct_item.fields {
                    fields.push(f.name.symbol);
                    field_heap_types.push(get_heap_type_resolved(&f.ty, self.interner));
                }

                let type_id = self.next_type_id;
                self.next_type_id += 1;

                self.struct_defs.insert(
                    name_spur,
                    StructDef {
                        type_id,
                        fields,
                        field_heap_types,
                    },
                );
            }
        }

        // Collect exception definitions (treated like structs for codegen)
        for item in &ast.items {
            if let crate::ast::Item::Exception(exception_item) = item {
                let name_spur = exception_item.name.symbol;
                let mut fields = Vec::new();
                let mut field_heap_types = Vec::new();

                for f in &exception_item.fields {
                    fields.push(f.name.symbol);
                    field_heap_types.push(get_heap_type_resolved(&f.ty, self.interner));
                }

                let type_id = self.next_type_id;
                self.next_type_id += 1;

                // Exception treated as a struct with its fields
                self.exception_names.insert(name_spur);
                self.struct_defs.insert(
                    name_spur,
                    StructDef {
                        type_id,
                        fields,
                        field_heap_types,
                    },
                );
            }
        }

        // Collect enum definitions
        for item in &ast.items {
            if let crate::ast::Item::Enum(enum_item) = item {
                let name = self.interner.resolve(&enum_item.name.symbol).to_string();
                let mut variants = Vec::new();
                let mut max_data_size: usize = 0;

                for (tag, variant) in enum_item.variants.iter().enumerate() {
                    let variant_name = self.interner.resolve(&variant.name.symbol).to_string();
                    let field_types = variant.fields.clone().unwrap_or_default();
                    let data_size = field_types.len() * 8; // Each field is 8 bytes
                    max_data_size = max_data_size.max(data_size);

                    variants.push(EnumVariantDef {
                        name: variant_name,
                        tag: tag as u32,
                        field_types,
                        data_offset: 8, // After tag + padding
                    });
                }

                // Align to 8 bytes
                let size = 8 + max_data_size.div_ceil(8) * 8;

                self.enum_defs.insert(
                    name.clone(),
                    EnumDef {
                        name,
                        variants,
                        size,
                    },
                );
            }
        }

        // Collect extern function declarations
        for item in &ast.items {
            if let crate::ast::Item::Extern(extern_item) = item {
                let name = self.interner.resolve(&extern_item.name.symbol).to_string();
                let link_name = if let Some(ref ln) = extern_item.link_name {
                    self.interner.resolve(&ln.symbol).to_string()
                } else {
                    name.clone()
                };

                let param_types: Vec<_> = extern_item.params.iter().map(|p| p.ty.clone()).collect();

                self.extern_fns.insert(
                    name,
                    ExternFn {
                        link_name,
                        param_types,
                        return_type: extern_item.return_ty.clone(),
                    },
                );
            }
        }

        // Collect global variable declarations from top-level statements
        for item in &ast.items {
            if let Item::TopLevelStmt(stmt_item) = item {
                if let Statement::Var(var_stmt) = &stmt_item.stmt {
                    let name = self.interner.resolve(&var_stmt.name.symbol).to_string();

                    // Determine the Cranelift type from the type annotation or infer from init
                    let cl_type = if let Some(ty) = &var_stmt.ty {
                        types::naml_to_cranelift(ty)
                    } else {
                        // Default to I64 for most types
                        cranelift::prelude::types::I64
                    };

                    // Create a data section for this global variable (8 bytes)
                    use cranelift_module::DataDescription;
                    let data_id = self
                        .module
                        .declare_data(&format!("__global_{}", name), Linkage::Local, true, false)
                        .map_err(|e| {
                            CodegenError::JitCompile(format!(
                                "Failed to declare global variable '{}': {}",
                                name, e
                            ))
                        })?;

                    let mut data_desc = DataDescription::new();
                    data_desc.define_zeroinit(8); // 8 bytes for any value
                    self.module.define_data(data_id, &data_desc).map_err(|e| {
                        CodegenError::JitCompile(format!(
                            "Failed to define global variable '{}': {}",
                            name, e
                        ))
                    })?;

                    // Store the initializer expression pointer for later compilation
                    let init_expr = var_stmt
                        .init
                        .as_ref()
                        .map(|e| e as *const Expression as *const Expression<'static>)
                        .unwrap_or(std::ptr::null());

                    self.global_vars.insert(
                        name,
                        GlobalVarDef {
                            data_id,
                            init_expr,
                            cl_type,
                        },
                    );
                }
            }
        }

        // Generate per-struct decref functions for structs with heap fields
        self.generate_struct_decref_functions()?;

        // Scan for spawn blocks and collect captured variable info
        for item in &ast.items {
            if let Item::Function(f) = item
                && let Some(ref body) = f.body
            {
                self.scan_for_spawn_blocks(body)?;
            }
        }

        // Declare spawn trampolines
        for (id, info) in &self.spawn_blocks.clone() {
            self.declare_spawn_trampoline(*id, info)?;
        }

        // Declare lambda functions
        for (id, info) in &self.lambda_blocks.clone() {
            self.declare_lambda_function(info)?;
            let _ = id; // suppress unused warning
        }

        // Declare all functions first (standalone and methods)
        // Skip generic functions - they will be monomorphized
        for item in &ast.items {
            if let Item::Function(f) = item {
                let is_generic = !f.generics.is_empty();
                if is_generic && f.receiver.is_none() {
                    // Store generic function for later monomorphization
                    let name = self.interner.resolve(&f.name.symbol).to_string();
                    self.generic_functions.insert(name, f as *const _);
                } else if f.receiver.is_none() {
                    self.declare_function(f)?;
                } else {
                    self.declare_method(f)?;
                }
            }
        }

        // Identify inline candidates (small non-generic functions)
        for item in &ast.items {
            if let Item::Function(f) = item {
                if f.receiver.is_none() && f.generics.is_empty() {
                    self.maybe_add_inline_candidate(f);
                }
            }
        }

        // Process monomorphizations - declare and compile specialized versions
        self.process_monomorphizations()?;

        // Compile spawn trampolines (after all functions are declared)
        for info in self.spawn_blocks.clone().values() {
            self.compile_spawn_trampoline(info)?;
        }

        // Compile lambda functions (after all functions are declared)
        for info in self.lambda_blocks.clone().values() {
            self.compile_lambda_function(info)?;
        }

        // Compile standalone functions (skip generic functions)
        for item in &ast.items {
            if let Item::Function(f) = item
                && f.receiver.is_none()
                && f.body.is_some()
                && f.generics.is_empty()
            {
                self.compile_function(f)?;
            }
        }

        // Compile methods
        for item in &ast.items {
            if let Item::Function(f) = item
                && f.receiver.is_some()
                && f.body.is_some()
            {
                self.compile_method(f)?;
            }
        }

        Ok(())
    }

    pub fn compile_module_source(&mut self, source: &str) -> Result<(), CodegenError> {
        let (tokens, mut module_interner) = crate::lexer::tokenize(source);
        let arena = crate::ast::AstArena::new();
        let parse_result = crate::parser::parse(&tokens, source, &arena);
        if !parse_result.errors.is_empty() {
            return Err(CodegenError::JitCompile(
                "parse errors in imported module".into(),
            ));
        }

        let type_result =
            crate::typechecker::check_with_types(&parse_result.ast, &mut module_interner, None, None);

        for item in &parse_result.ast.items {
            if let Item::Struct(struct_item) = item {
                if !struct_item.is_public {
                    continue;
                }
                let name_str = module_interner.resolve(&struct_item.name.symbol);
                let name_spur = match self.interner.get(name_str) {
                    Some(s) => s,
                    None => continue,
                };
                let mut fields = Vec::new();
                let mut field_heap_types = Vec::new();
                for f in &struct_item.fields {
                    let field_str = module_interner.resolve(&f.name.symbol);
                    let field_spur = match self.interner.get(field_str) {
                        Some(s) => s,
                        None => continue,
                    };
                    fields.push(field_spur);
                    let ht = heap::get_heap_type_resolved(&f.ty, &module_interner);
                    field_heap_types.push(
                        ht.map(|h| heap::remap_heap_type(h, &module_interner, self.interner)),
                    );
                }
                let type_id = self.next_type_id;
                self.next_type_id += 1;
                self.struct_defs.insert(
                    name_spur,
                    StructDef {
                        type_id,
                        fields,
                        field_heap_types,
                    },
                );
            }
        }

        let saved_interner = self.interner;
        let saved_annotations = self.annotations;
        self.interner = unsafe { std::mem::transmute::<&Rodeo, &Rodeo>(&module_interner) };
        self.annotations = unsafe {
            std::mem::transmute::<&TypeAnnotations, &TypeAnnotations>(&type_result.annotations)
        };

        for item in &parse_result.ast.items {
            if let Item::Function(f) = item {
                if f.is_public && f.receiver.is_none() && f.body.is_some() && f.generics.is_empty()
                {
                    self.declare_function(f)?;
                }
            }
        }
        for item in &parse_result.ast.items {
            if let Item::Function(f) = item {
                if f.is_public && f.receiver.is_none() && f.body.is_some() && f.generics.is_empty()
                {
                    self.compile_function(f)?;
                }
            }
        }

        self.interner = saved_interner;
        self.annotations = saved_annotations;
        Ok(())
    }

    pub fn run_main(&mut self) -> Result<(), CodegenError> {
        let jit = self.module.as_jit_mut().ok_or_else(|| {
            CodegenError::JitCompile("run_main requires JIT backend".to_string())
        })?;

        jit.finalize_definitions()
            .map_err(|e| CodegenError::JitCompile(format!("Failed to finalize: {}", e)))?;

        let main_id = self
            .functions
            .get("main")
            .ok_or_else(|| CodegenError::Execution("No main function found".to_string()))?;

        let main_ptr = jit.get_finalized_function(*main_id);

        let main_fn: fn(i64) = unsafe { std::mem::transmute(main_ptr) };
        main_fn(0);

        Ok(())
    }

    pub fn emit_object(self, output: &Path) -> Result<(), CodegenError> {
        let obj_module = self.module.as_object().ok_or_else(|| {
            CodegenError::JitCompile("emit_object requires Object backend".to_string())
        })?;
        let product = obj_module.finish();
        let bytes = product.emit().map_err(|e| {
            CodegenError::JitCompile(format!("Failed to emit object file: {}", e))
        })?;
        std::fs::write(output, bytes).map_err(|e| {
            CodegenError::JitCompile(format!("Failed to write object file: {}", e))
        })
    }
}
