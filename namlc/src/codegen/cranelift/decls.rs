use std::collections::HashMap;

use cranelift::prelude::*;
use cranelift_module::{FuncId, Linkage, Module};

use crate::codegen::CodegenError;
use crate::codegen::cranelift::JitCompiler;

impl<'a> JitCompiler<'a> {
    pub(crate) fn declare_runtime_functions(&mut self) -> Result<(), CodegenError> {
        let ptr = self.module.target_config().pointer_type();
        let i64t = cranelift::prelude::types::I64;
        let f64t = cranelift::prelude::types::F64;
        let i32t = cranelift::prelude::types::I32;

        let declare = |module: &mut dyn Module,
                       cache: &mut HashMap<String, FuncId>,
                       name: &str,
                       params: &[cranelift::prelude::Type],
                       returns: &[cranelift::prelude::Type]|
         -> Result<(), CodegenError> {
            let mut sig = module.make_signature();
            for &p in params {
                sig.params.push(AbiParam::new(p));
            }
            for &r in returns {
                sig.returns.push(AbiParam::new(r));
            }
            let func_id = module
                .declare_function(name, Linkage::Import, &sig)
                .map_err(|e| {
                    CodegenError::JitCompile(format!("Failed to declare {}: {}", name, e))
                })?;
            cache.insert(name.to_string(), func_id);
            Ok(())
        };

        // Print functions
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_print_int",
            &[i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_print_float",
            &[f64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_print_bool",
            &[i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_print_str",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_print",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_print_newline",
            &[],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_option_print_int",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_option_print_float",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_option_print_bool",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_option_print_string",
            &[ptr],
            &[],
        )?;

        // String functions
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_concat",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_eq",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_from_cstr",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_char_len",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_char_at",
            &[ptr, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_is_empty",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_trim",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_upper",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_lower",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_contains",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_starts_with",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_ends_with",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_replace",
            &[ptr, ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_replace_all",
            &[ptr, ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_split",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_join",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_ltrim",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_rtrim",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_substr",
            &[ptr, i64t, i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_lpad",
            &[ptr, i64t, ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_rpad",
            &[ptr, i64t, ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_repeat",
            &[ptr, i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_lines",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_chars",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_incref",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_decref",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_to_bytes",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_bytes_to_string",
            &[ptr],
            &[ptr],
        )?;

        // Type conversion
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_int_to_string",
            &[i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_float_to_string",
            &[f64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_to_int",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_to_float",
            &[ptr],
            &[f64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_try_to_int",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_string_try_to_float",
            &[ptr, ptr],
            &[i64t],
        )?;

        // I/O
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_read_line",
            &[],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_read_key",
            &[],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_clear_screen",
            &[],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_set_cursor",
            &[i64t, i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_hide_cursor",
            &[],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_show_cursor",
            &[],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_terminal_width",
            &[],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_terminal_height",
            &[],
            &[i64t],
        )?;

        // Array functions
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_new",
            &[i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_from",
            &[ptr, i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_push",
            &[ptr, i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_get",
            &[ptr, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_set",
            &[ptr, i64t, i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_len",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_pop",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_is_empty",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_shift",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_fill",
            &[ptr, i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_clear",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_first",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_last",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_sum",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_min",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_max",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_reverse",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_reversed",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_take",
            &[ptr, i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_drop",
            &[ptr, i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_slice",
            &[ptr, i64t, i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_index_of",
            &[ptr, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_contains",
            &[ptr, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_any",
            &[ptr, i64t, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_all",
            &[ptr, i64t, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_count_if",
            &[ptr, i64t, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_map",
            &[ptr, i64t, i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_filter",
            &[ptr, i64t, i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_find",
            &[ptr, i64t, i64t, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_find_index",
            &[ptr, i64t, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_fold",
            &[ptr, i64t, i64t, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_flatten",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_sort",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_sort_by",
            &[ptr, i64t, i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_print",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_print_strings",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_print",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_print_string_values",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_print_float_values",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_print_bool_values",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_incref",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_decref",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_decref_strings",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_decref_arrays",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_decref_maps",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_decref_structs",
            &[ptr],
            &[],
        )?;
        // New array functions - Mutation
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_insert",
            &[ptr, i64t, i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_remove_at",
            &[ptr, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_remove",
            &[ptr, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_swap",
            &[ptr, i64t, i64t],
            &[],
        )?;
        // Deduplication
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_unique",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_compact",
            &[ptr],
            &[ptr],
        )?;
        // Backward search
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_last_index_of",
            &[ptr, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_find_last",
            &[ptr, i64t, i64t, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_find_last_index",
            &[ptr, i64t, i64t],
            &[i64t],
        )?;
        // Array combination
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_concat",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_zip",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_unzip",
            &[ptr],
            &[ptr],
        )?;
        // Splitting
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_chunk",
            &[ptr, i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_partition",
            &[ptr, i64t, i64t],
            &[ptr],
        )?;
        // Set operations
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_intersect",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_diff",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_union",
            &[ptr, ptr],
            &[ptr],
        )?;
        // Advanced iteration
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_take_while",
            &[ptr, i64t, i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_drop_while",
            &[ptr, i64t, i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_reject",
            &[ptr, i64t, i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_flat_apply",
            &[ptr, i64t, i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_scan",
            &[ptr, i64t, i64t, i64t],
            &[ptr],
        )?;
        // Random
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_shuffle",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_sample",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_array_sample_n",
            &[ptr, i64t],
            &[ptr],
        )?;

        // Map functions
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_new",
            &[i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_set",
            &[ptr, i64t, i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_set_string",
            &[ptr, i64t, i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_set_array",
            &[ptr, i64t, i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_set_map",
            &[ptr, i64t, i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_set_struct",
            &[ptr, i64t, i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_get",
            &[ptr, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_contains",
            &[ptr, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_len",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_incref",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_decref",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_decref_strings",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_decref_arrays",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_decref_maps",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_decref_structs",
            &[ptr],
            &[],
        )?;

        // Map collection functions (from naml-std-collections)
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_count",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_contains_key",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_remove",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_clear",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_keys",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_values",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_entries",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_first_key",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_first_value",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_any",
            &[ptr, ptr, ptr], // map, func_ptr, data_ptr
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_all",
            &[ptr, ptr, ptr], // map, func_ptr, data_ptr
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_count_if",
            &[ptr, ptr, ptr], // map, func_ptr, data_ptr
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_fold",
            &[ptr, i64t, ptr, ptr], // map, initial, func_ptr, data_ptr
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_transform",
            &[ptr, ptr, ptr], // map, func_ptr, data_ptr
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_where",
            &[ptr, ptr, ptr], // map, func_ptr, data_ptr
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_reject",
            &[ptr, ptr, ptr], // map, func_ptr, data_ptr
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_merge",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_defaults",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_intersect",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_diff",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_invert",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_from_arrays",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_map_from_entries",
            &[ptr],
            &[ptr],
        )?;

        // Arena allocator
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_arena_alloc",
            &[i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_arena_get_tls_ptr",
            &[],
            &[ptr],
        )?;

        // Struct functions
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_struct_new",
            &[i32t, i32t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_struct_get_field",
            &[ptr, i32t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_struct_set_field",
            &[ptr, i32t, i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_struct_incref",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_struct_decref",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_struct_free",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_arena_free_sized",
            &[ptr, i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_struct_decref_iterative",
            &[ptr, ptr, i32t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_struct_incref_fast",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_struct_decref_fast",
            &[ptr],
            &[],
        )?;

        // Channel functions
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_channel_new",
            &[i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_channel_send",
            &[ptr, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_channel_receive",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_channel_close",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_channel_len",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_channel_incref",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_channel_decref",
            &[ptr],
            &[],
        )?;

        // Mutex functions
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_mutex_new",
            &[i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_mutex_lock",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_mutex_unlock",
            &[ptr, i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_mutex_incref",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_mutex_decref",
            &[ptr],
            &[],
        )?;

        // RwLock functions
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_rwlock_new",
            &[i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_rwlock_read_lock",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_rwlock_read_unlock",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_rwlock_write_lock",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_rwlock_write_unlock",
            &[ptr, i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_rwlock_incref",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_rwlock_decref",
            &[ptr],
            &[],
        )?;

        // AtomicInt functions
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_int_new", &[i64t], &[ptr])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_int_load", &[ptr], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_int_store", &[ptr, i64t], &[])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_int_add", &[ptr, i64t], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_int_sub", &[ptr, i64t], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_int_inc", &[ptr], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_int_dec", &[ptr], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_int_cas", &[ptr, i64t, i64t], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_int_swap", &[ptr, i64t], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_int_and", &[ptr, i64t], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_int_or", &[ptr, i64t], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_int_xor", &[ptr, i64t], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_int_incref", &[ptr], &[])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_int_decref", &[ptr], &[])?;

        // AtomicUint functions
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_uint_new", &[i64t], &[ptr])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_uint_load", &[ptr], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_uint_store", &[ptr, i64t], &[])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_uint_add", &[ptr, i64t], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_uint_sub", &[ptr, i64t], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_uint_inc", &[ptr], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_uint_dec", &[ptr], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_uint_cas", &[ptr, i64t, i64t], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_uint_swap", &[ptr, i64t], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_uint_and", &[ptr, i64t], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_uint_or", &[ptr, i64t], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_uint_xor", &[ptr, i64t], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_uint_incref", &[ptr], &[])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_uint_decref", &[ptr], &[])?;

        // AtomicBool functions
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_bool_new", &[i64t], &[ptr])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_bool_load", &[ptr], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_bool_store", &[ptr, i64t], &[])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_bool_cas", &[ptr, i64t, i64t], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_bool_swap", &[ptr, i64t], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_bool_incref", &[ptr], &[])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_atomic_bool_decref", &[ptr], &[])?;

        // Scheduler/runtime
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_spawn",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_spawn_closure",
            &[ptr, ptr, i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_alloc_closure_data",
            &[i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_wait_all",
            &[],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_sleep",
            &[i64t],
            &[],
        )?;
        // Timer functions
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_timers_set_timeout",
            &[i64t, i64t, i64t, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_timers_cancel_timeout",
            &[i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_timers_set_interval",
            &[i64t, i64t, i64t, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_timers_cancel_interval",
            &[i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_timers_schedule",
            &[i64t, i64t, i64t, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_timers_cancel_schedule",
            &[i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_timers_next_run",
            &[i64t],
            &[i64t],
        )?;

        // Crypto operations - hash: (ptr) -> ptr
        for name in [
            "naml_crypto_md5", "naml_crypto_sha1",
            "naml_crypto_sha256", "naml_crypto_sha512",
        ] {
            declare(&mut *self.module, &mut self.runtime_funcs, name, &[ptr], &[ptr])?;
        }
        // Crypto operations - hash hex: (ptr) -> ptr
        for name in [
            "naml_crypto_md5_hex", "naml_crypto_sha1_hex",
            "naml_crypto_sha256_hex", "naml_crypto_sha512_hex",
        ] {
            declare(&mut *self.module, &mut self.runtime_funcs, name, &[ptr], &[ptr])?;
        }
        // Crypto operations - HMAC: (ptr, ptr) -> ptr
        for name in [
            "naml_crypto_hmac_sha256", "naml_crypto_hmac_sha256_hex",
            "naml_crypto_hmac_sha512", "naml_crypto_hmac_sha512_hex",
        ] {
            declare(&mut *self.module, &mut self.runtime_funcs, name, &[ptr, ptr], &[ptr])?;
        }
        // Crypto operations - HMAC verify: (ptr, ptr, ptr) -> i64 (bool)
        for name in [
            "naml_crypto_hmac_verify_sha256", "naml_crypto_hmac_verify_sha512",
        ] {
            declare(&mut *self.module, &mut self.runtime_funcs, name, &[ptr, ptr, ptr], &[i64t])?;
        }
        // Crypto operations - PBKDF2: (ptr, ptr, i64, i64) -> ptr
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_crypto_pbkdf2_sha256", &[ptr, ptr, i64t, i64t], &[ptr])?;
        // Crypto operations - random bytes: (i64) -> ptr
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_crypto_random_bytes", &[i64t], &[ptr])?;

        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_random",
            &[i64t, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_random_float",
            &[],
            &[f64t],
        )?;

        // Diagnostics
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_warn",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_error",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_panic",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_panic_unwrap",
            &[],
            &[],
        )?;

        // Exception handling
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_exception_set",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_exception_get",
            &[],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_exception_clear",
            &[],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_exception_check",
            &[],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_exception_set_typed",
            &[ptr, i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_exception_get_type_id",
            &[],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_exception_is_type",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_exception_clear_ptr",
            &[],
            &[],
        )?;

        // File system operations
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_read",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_read_bytes",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_write",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_append",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_write_bytes",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_append_bytes",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_exists",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_is_file",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_is_dir",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_list_dir",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_mkdir",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_mkdir_all",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_remove",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_remove_all",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_join",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_dirname",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_basename",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_extension",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_absolute",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_size",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_modified",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_copy",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_rename",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_getwd",
            &[],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_chdir",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_create_temp",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_mkdir_temp",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_chmod",
            &[ptr, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_truncate",
            &[ptr, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_stat",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_io_error_new",
            &[ptr, ptr, i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_permission_error_new",
            &[ptr, ptr, i64t],
            &[ptr],
        )?;

        // Memory-mapped file operations
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_mmap_open",
            &[ptr, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_mmap_len",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_mmap_read_byte",
            &[i64t, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_mmap_write_byte",
            &[i64t, i64t, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_mmap_read",
            &[i64t, i64t, i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_mmap_write",
            &[i64t, i64t, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_mmap_flush",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_mmap_close",
            &[i64t],
            &[i64t],
        )?;

        // File handle operations
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_file_open",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_file_close",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_file_read",
            &[i64t, i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_file_read_line",
            &[i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_file_read_all",
            &[i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_file_write",
            &[i64t, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_file_write_line",
            &[i64t, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_file_flush",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_file_seek",
            &[i64t, i64t, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_file_tell",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_file_eof",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_file_size",
            &[i64t],
            &[i64t],
        )?;

        // Link/symlink operations
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_symlink",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_readlink",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_lstat",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_link",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_chtimes",
            &[ptr, i64t, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_chown",
            &[ptr, i64t, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_lchown",
            &[ptr, i64t, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_same_file",
            &[ptr, ptr],
            &[i64t],
        )?;

        // Additional file handle operations
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_file_read_at",
            &[i64t, i64t, i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_file_write_at",
            &[i64t, ptr, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_file_name",
            &[i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_file_stat",
            &[i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_file_truncate",
            &[i64t, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_file_chmod",
            &[i64t, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_fs_file_chown",
            &[i64t, i64t, i64t],
            &[i64t],
        )?;

        // Path operations
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_path_join",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_path_normalize",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_path_is_absolute",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_path_is_relative",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_path_has_root",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_path_dirname",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_path_basename",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_path_extension",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_path_stem",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_path_with_extension",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_path_components",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_path_separator",
            &[],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_path_to_slash",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_path_from_slash",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_path_starts_with",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_path_ends_with",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_path_strip_prefix",
            &[ptr, ptr],
            &[ptr],
        )?;

        // Environment operations (from naml-std-env)
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_env_getenv",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_env_lookup_env",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_env_setenv",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_env_unsetenv",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_env_clearenv",
            &[],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_env_environ",
            &[],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_env_expand_env",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_env_error_new",
            &[ptr, ptr],
            &[ptr],
        )?;

        // OS operations (from naml-std-os)
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_os_hostname",
            &[],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_os_temp_dir",
            &[],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_os_home_dir",
            &[],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_os_cache_dir",
            &[],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_os_config_dir",
            &[],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_os_executable",
            &[],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_os_pagesize",
            &[],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_os_getuid",
            &[],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_os_geteuid",
            &[],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_os_getgid",
            &[],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_os_getegid",
            &[],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_os_getgroups",
            &[],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_os_error_new",
            &[ptr, i64t],
            &[ptr],
        )?;

        // Process operations (from naml-std-process)
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_process_getpid",
            &[],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_process_getppid",
            &[],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_process_exit",
            &[i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_process_pipe_read",
            &[],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_process_pipe_write",
            &[],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_process_start",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_process_find",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_process_wait",
            &[i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_process_signal",
            &[i64t, i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_process_kill",
            &[i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_process_release",
            &[i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_process_error_new",
            &[ptr, i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_process_sighup",
            &[],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_process_sigint",
            &[],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_process_sigquit",
            &[],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_process_sigkill",
            &[],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_process_sigterm",
            &[],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_process_sigstop",
            &[],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_process_sigcont",
            &[],
            &[i64t],
        )?;

        // Testing operations (from naml-std-testing)
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_testing_assert",
            &[i64t, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_testing_assert_eq",
            &[i64t, i64t, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_testing_assert_eq_float",
            &[f64t, f64t, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_testing_assert_eq_string",
            &[ptr, ptr, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_testing_assert_eq_bool",
            &[i64t, i64t, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_testing_assert_neq",
            &[i64t, i64t, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_testing_assert_neq_string",
            &[ptr, ptr, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_testing_assert_true",
            &[i64t, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_testing_assert_false",
            &[i64t, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_testing_assert_gt",
            &[i64t, i64t, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_testing_assert_gte",
            &[i64t, i64t, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_testing_assert_lt",
            &[i64t, i64t, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_testing_assert_lte",
            &[i64t, i64t, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_testing_fail",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_testing_assert_approx",
            &[f64t, f64t, f64t, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_testing_assert_contains",
            &[ptr, ptr, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_testing_assert_starts_with",
            &[ptr, ptr, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_testing_assert_ends_with",
            &[ptr, ptr, ptr],
            &[],
        )?;

        // Bytes operations
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_bytes_new",
            &[i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_bytes_from",
            &[ptr, i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_bytes_len",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_bytes_get",
            &[ptr, i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_bytes_set",
            &[ptr, i64t, i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_bytes_incref",
            &[ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_bytes_decref",
            &[ptr],
            &[],
        )?;

        // Encoding operations (from naml-std-encoding)
        // UTF-8
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_encoding_utf8_encode",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_encoding_utf8_decode",
            &[ptr, ptr, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_encoding_utf8_is_valid",
            &[ptr],
            &[i64t],
        )?;
        // Hex
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_encoding_hex_encode",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_encoding_hex_decode",
            &[ptr, ptr, ptr],
            &[],
        )?;
        // Base64
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_encoding_base64_encode",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_encoding_base64_decode",
            &[ptr, ptr, ptr],
            &[],
        )?;
        // URL
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_encoding_url_encode",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_encoding_url_decode",
            &[ptr, ptr, ptr],
            &[],
        )?;
        // DecodeError helper
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_decode_error_new",
            &[ptr, i64t],
            &[ptr],
        )?;

        // JSON encoding operations
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_json_decode",
            &[ptr, ptr, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_json_encode",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_json_encode_pretty",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_json_exists",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_json_path",
            &[ptr, ptr, ptr, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_json_keys",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_json_count",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_json_get_type",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_json_type_name",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_json_is_null",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_json_index_string",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_json_index_int",
            &[ptr, i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_json_as_int",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_json_as_float",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_json_as_bool",
            &[ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_json_as_string",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_json_null",
            &[],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_path_error_new",
            &[ptr],
            &[ptr],
        )?;

        // TOML encoding operations
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_encoding_toml_decode",
            &[ptr, ptr, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_encoding_toml_encode",
            &[ptr, ptr, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_encoding_toml_encode_pretty",
            &[ptr, ptr, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_encode_error_new",
            &[ptr],
            &[ptr],
        )?;

        // YAML encoding operations
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_encoding_yaml_decode",
            &[ptr, ptr, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_encoding_yaml_encode",
            &[ptr, ptr, ptr],
            &[],
        )?;

        // Binary encoding operations - integer reads: (ptr, i64) -> i64
        for name in [
            "naml_encoding_binary_read_u8", "naml_encoding_binary_read_i8",
            "naml_encoding_binary_read_u16_be", "naml_encoding_binary_read_u16_le",
            "naml_encoding_binary_read_i16_be", "naml_encoding_binary_read_i16_le",
            "naml_encoding_binary_read_u32_be", "naml_encoding_binary_read_u32_le",
            "naml_encoding_binary_read_i32_be", "naml_encoding_binary_read_i32_le",
            "naml_encoding_binary_read_u64_be", "naml_encoding_binary_read_u64_le",
            "naml_encoding_binary_read_i64_be", "naml_encoding_binary_read_i64_le",
        ] {
            declare(&mut *self.module, &mut self.runtime_funcs, name, &[ptr, i64t], &[i64t])?;
        }
        // Float reads: (ptr, i64) -> f64
        for name in [
            "naml_encoding_binary_read_f32_be", "naml_encoding_binary_read_f32_le",
            "naml_encoding_binary_read_f64_be", "naml_encoding_binary_read_f64_le",
        ] {
            declare(&mut *self.module, &mut self.runtime_funcs, name, &[ptr, i64t], &[f64t])?;
        }
        // Integer writes: (ptr, i64, i64) -> void
        for name in [
            "naml_encoding_binary_write_u8", "naml_encoding_binary_write_i8",
            "naml_encoding_binary_write_u16_be", "naml_encoding_binary_write_u16_le",
            "naml_encoding_binary_write_i16_be", "naml_encoding_binary_write_i16_le",
            "naml_encoding_binary_write_u32_be", "naml_encoding_binary_write_u32_le",
            "naml_encoding_binary_write_i32_be", "naml_encoding_binary_write_i32_le",
            "naml_encoding_binary_write_u64_be", "naml_encoding_binary_write_u64_le",
            "naml_encoding_binary_write_i64_be", "naml_encoding_binary_write_i64_le",
        ] {
            declare(&mut *self.module, &mut self.runtime_funcs, name, &[ptr, i64t, i64t], &[])?;
        }
        // Float writes: (ptr, i64, f64) -> void
        for name in [
            "naml_encoding_binary_write_f32_be", "naml_encoding_binary_write_f32_le",
            "naml_encoding_binary_write_f64_be", "naml_encoding_binary_write_f64_le",
        ] {
            declare(&mut *self.module, &mut self.runtime_funcs, name, &[ptr, i64t, f64t], &[])?;
        }
        // Buffer operations
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_encoding_binary_alloc", &[i64t], &[ptr])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_encoding_binary_from_string", &[ptr], &[ptr])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_encoding_binary_len", &[ptr], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_encoding_binary_capacity", &[ptr], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_encoding_binary_slice", &[ptr, i64t, i64t], &[ptr])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_encoding_binary_concat", &[ptr, ptr], &[ptr])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_encoding_binary_append", &[ptr, ptr], &[])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_encoding_binary_copy_within", &[ptr, i64t, i64t, i64t], &[])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_encoding_binary_clear", &[ptr], &[])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_encoding_binary_resize", &[ptr, i64t], &[])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_encoding_binary_fill", &[ptr, i64t], &[])?;
        // Search operations
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_encoding_binary_index_of", &[ptr, ptr], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_encoding_binary_contains", &[ptr, ptr], &[i32t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_encoding_binary_starts_with", &[ptr, ptr], &[i32t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_encoding_binary_ends_with", &[ptr, ptr], &[i32t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_encoding_binary_equals", &[ptr, ptr], &[i32t])?;

        // Datetime operations
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_datetime_now_ms",
            &[],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_datetime_now_s",
            &[],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_datetime_year",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_datetime_month",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_datetime_day",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_datetime_hour",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_datetime_minute",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_datetime_second",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_datetime_day_of_week",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_datetime_format",
            &[i64t, ptr],
            &[ptr],
        )?;

        // Metrics operations
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_metrics_perf_now",
            &[],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_metrics_elapsed_ms",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_metrics_elapsed_us",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_metrics_elapsed_ns",
            &[i64t],
            &[i64t],
        )?;

        // Stack trace functions
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_stack_push",
            &[ptr, ptr, i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_stack_pop",
            &[],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_stack_capture",
            &[],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_stack_clear",
            &[],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_stack_format",
            &[ptr],
            &[ptr],
        )?;

        // Networking operations (from naml-std-net)
        // Exception constructors
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_network_error_new",
            &[ptr, i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_timeout_error_new",
            &[ptr, i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_connection_refused_new",
            &[ptr],
            &[ptr],
        )?;

        // TCP Server
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_tcp_server_listen",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_tcp_server_accept",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_tcp_server_close",
            &[i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_tcp_server_local_addr",
            &[i64t],
            &[ptr],
        )?;

        // TCP Client
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_tcp_client_connect",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_tcp_client_read",
            &[i64t, i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_tcp_client_read_all",
            &[i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_tcp_client_write",
            &[i64t, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_tcp_client_close",
            &[i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_tcp_client_set_timeout",
            &[i64t, i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_tcp_socket_peer_addr",
            &[i64t],
            &[ptr],
        )?;

        // UDP
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_udp_bind",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_udp_send",
            &[i64t, ptr, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_udp_receive",
            &[i64t, i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_udp_receive_from",
            &[i64t, i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_udp_close",
            &[i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_udp_local_addr",
            &[i64t],
            &[ptr],
        )?;

        // HTTP Client (all methods accept optional headers: url, [body], headers)
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_client_get",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_client_post",
            &[ptr, ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_client_put",
            &[ptr, ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_client_patch",
            &[ptr, ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_client_delete",
            &[ptr, ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_client_set_timeout",
            &[i64t],
            &[],
        )?;
        // HTTP Response accessors
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_response_get_status",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_response_get_body_bytes",
            &[ptr],
            &[ptr],
        )?;

        // HTTP Server
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_server_open_router",
            &[],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_server_get",
            &[i64t, ptr, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_server_post",
            &[i64t, ptr, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_server_put",
            &[i64t, ptr, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_server_patch",
            &[i64t, ptr, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_server_delete",
            &[i64t, ptr, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_server_with",
            &[i64t, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_server_group",
            &[i64t, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_server_mount",
            &[i64t, ptr, i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_server_serve",
            &[ptr, i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_server_text_response",
            &[i64t, ptr],
            &[ptr],
        )?;

        // HTTP Middleware
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_middleware_logger",
            &[],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_middleware_timeout",
            &[i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_middleware_recover",
            &[],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_middleware_cors",
            &[ptr],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_middleware_rate_limit",
            &[i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_middleware_compress",
            &[],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_middleware_request_id",
            &[],
            &[ptr],
        )?;

        // TLS Client
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_tls_client_connect",
            &[ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_tls_client_read",
            &[i64t, i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_tls_client_read_all",
            &[i64t],
            &[ptr],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_tls_client_write",
            &[i64t, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_tls_client_close",
            &[i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_tls_client_set_timeout",
            &[i64t, i64t],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_tls_client_peer_addr",
            &[i64t],
            &[ptr],
        )?;

        // TLS Server
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_tls_server_wrap_listener",
            &[i64t, ptr, ptr],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_tls_server_accept",
            &[i64t],
            &[i64t],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_tls_server_close_listener",
            &[i64t],
            &[],
        )?;

        // HTTP over TLS
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_server_serve_tls",
            &[ptr, i64t, ptr, ptr],
            &[],
        )?;
        declare(
            &mut *self.module,
            &mut self.runtime_funcs,
            "naml_net_http_client_get_tls",
            &[ptr, ptr],
            &[ptr],
        )?;

        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_error_new", &[ptr, i64t], &[ptr])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_open", &[ptr], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_open_memory", &[], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_close", &[i64t], &[])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_exec", &[i64t, ptr], &[])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_query", &[i64t, ptr, i64t], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_row_count", &[i64t], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_row_at", &[i64t, i64t], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_get_string", &[i64t, ptr], &[ptr])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_get_int", &[i64t, ptr], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_get_float", &[i64t, ptr], &[f64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_get_bool", &[i64t, ptr], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_is_null", &[i64t, ptr], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_columns", &[i64t], &[ptr])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_column_count", &[i64t], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_begin", &[i64t], &[])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_commit", &[i64t], &[])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_rollback", &[i64t], &[])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_prepare", &[i64t, ptr], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_bind_string", &[i64t, i64t, ptr], &[])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_bind_int", &[i64t, i64t, i64t], &[])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_bind_float", &[i64t, i64t, f64t], &[])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_step", &[i64t], &[])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_step_query", &[i64t], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_reset", &[i64t], &[])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_finalize", &[i64t], &[])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_changes", &[i64t], &[i64t])?;
        declare(&mut *self.module, &mut self.runtime_funcs, "naml_db_sqlite_last_insert_id", &[i64t], &[i64t])?;

        Ok(())
    }
}
