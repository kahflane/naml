use std::collections::{HashMap, HashSet};
use indexmap::IndexMap;

use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_object::{ObjectBuilder, ObjectModule};
use lasso::Rodeo;

use crate::codegen::CodegenError;
use crate::codegen::cranelift::{BackendModule, EnumDef, EnumVariantDef, JitCompiler};
use crate::typechecker::TypeAnnotations;

fn create_isa(pic: bool, release: bool) -> Result<cranelift_codegen::isa::OwnedTargetIsa, CodegenError> {
    let mut flag_builder = settings::builder();
    flag_builder.set("use_colocated_libcalls", "false").unwrap();
    flag_builder
        .set("is_pic", if pic { "true" } else { "false" })
        .unwrap();
    flag_builder
        .set("opt_level", if release { "speed" } else { "none" })
        .unwrap();
    flag_builder
        .set("preserve_frame_pointers", if release { "false" } else { "true" })
        .unwrap();

    let isa_builder = cranelift_native::builder().map_err(|e| {
        CodegenError::JitCompile(format!("Failed to create ISA builder: {}", e))
    })?;

    isa_builder
        .finish(settings::Flags::new(flag_builder))
        .map_err(|e| CodegenError::JitCompile(format!("Failed to create ISA: {}", e)))
}

impl<'a> JitCompiler<'a> {
    fn build_compiler(
        interner: &'a Rodeo,
        annotations: &'a TypeAnnotations,
        source_info: &'a crate::source::SourceFile,
        module: BackendModule,
        release: bool,
        unsafe_mode: bool,
    ) -> Result<Self, CodegenError> {
        let ctx = module.make_context();

        let mut enum_defs = HashMap::new();
        enum_defs.insert(
            "option".to_string(),
            EnumDef {
                name: "option".to_string(),
                variants: vec![
                    EnumVariantDef {
                        name: "none".to_string(),
                        tag: 0,
                        field_types: vec![],
                        data_offset: 8,
                    },
                    EnumVariantDef {
                        name: "some".to_string(),
                        tag: 1,
                        field_types: vec![crate::ast::NamlType::Int],
                        data_offset: 8,
                    },
                ],
                size: 16,
            },
        );

        let mut compiler = Self {
            interner,
            annotations,
            source_info,
            module,
            ctx,
            functions: HashMap::new(),
            runtime_funcs: HashMap::new(),
            struct_defs: HashMap::new(),
            enum_defs,
            exception_names: HashSet::new(),
            extern_fns: HashMap::new(),
            global_vars: IndexMap::new(),
            next_type_id: 0,
            spawn_counter: 0,
            spawn_blocks: HashMap::new(),
            spawn_body_to_id: HashMap::new(),
            lambda_counter: 0,
            lambda_blocks: HashMap::new(),
            lambda_body_to_id: HashMap::new(),
            generic_functions: HashMap::new(),
            inline_functions: HashMap::new(),
            release_mode: release,
            unsafe_mode,
        };
        compiler.declare_runtime_functions()?;
        compiler.register_builtin_exceptions();
        Ok(compiler)
    }

    pub fn new(
        interner: &'a Rodeo,
        annotations: &'a TypeAnnotations,
        source_info: &'a crate::source::SourceFile,
        release: bool,
        unsafe_mode: bool,
    ) -> Result<Self, CodegenError> {
        let isa = create_isa(false, release)?;
        let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());

        // Print builtins
        builder.symbol("naml_print_int", super::naml_print_int as *const u8);
        builder.symbol("naml_print_float", super::naml_print_float as *const u8);
        builder.symbol("naml_print_bool", super::naml_print_bool as *const u8);
        builder.symbol("naml_print_str", super::naml_print_str as *const u8);
        builder.symbol("naml_print_newline", super::naml_print_newline as *const u8);
        builder.symbol("naml_option_print_int", super::naml_option_print_int as *const u8);
        builder.symbol("naml_option_print_float", super::naml_option_print_float as *const u8);
        builder.symbol("naml_option_print_bool", super::naml_option_print_bool as *const u8);
        builder.symbol("naml_option_print_string", super::naml_option_print_string as *const u8);

        // Array runtime functions
        builder.symbol(
            "naml_array_new",
            crate::runtime::naml_array_new as *const u8,
        );
        builder.symbol(
            "naml_array_from",
            crate::runtime::naml_array_from as *const u8,
        );
        builder.symbol(
            "naml_array_push",
            crate::runtime::naml_array_push as *const u8,
        );
        builder.symbol(
            "naml_array_get",
            crate::runtime::naml_array_get as *const u8,
        );
        builder.symbol(
            "naml_array_set",
            crate::runtime::naml_array_set as *const u8,
        );
        builder.symbol(
            "naml_array_len",
            crate::runtime::naml_array_len as *const u8,
        );
        builder.symbol(
            "naml_array_pop",
            crate::runtime::naml_array_pop as *const u8,
        );
        builder.symbol(
            "naml_array_is_empty",
            crate::runtime::naml_array_is_empty as *const u8,
        );
        builder.symbol(
            "naml_array_shift",
            crate::runtime::naml_array_shift as *const u8,
        );
        builder.symbol(
            "naml_array_fill",
            crate::runtime::naml_array_fill as *const u8,
        );
        builder.symbol(
            "naml_array_clear",
            crate::runtime::naml_array_clear as *const u8,
        );
        builder.symbol(
            "naml_array_first",
            crate::runtime::naml_array_first as *const u8,
        );
        builder.symbol(
            "naml_array_last",
            crate::runtime::naml_array_last as *const u8,
        );
        builder.symbol(
            "naml_array_sum",
            crate::runtime::naml_array_sum as *const u8,
        );
        builder.symbol(
            "naml_array_min",
            crate::runtime::naml_array_min as *const u8,
        );
        builder.symbol(
            "naml_array_max",
            crate::runtime::naml_array_max as *const u8,
        );
        builder.symbol(
            "naml_array_reverse",
            crate::runtime::naml_array_reverse as *const u8,
        );
        builder.symbol(
            "naml_array_reversed",
            crate::runtime::naml_array_reversed as *const u8,
        );
        builder.symbol(
            "naml_array_take",
            crate::runtime::naml_array_take as *const u8,
        );
        builder.symbol(
            "naml_array_drop",
            crate::runtime::naml_array_drop as *const u8,
        );
        builder.symbol(
            "naml_array_slice",
            crate::runtime::naml_array_slice as *const u8,
        );
        builder.symbol(
            "naml_array_index_of",
            crate::runtime::naml_array_index_of as *const u8,
        );
        builder.symbol(
            "naml_array_contains",
            crate::runtime::naml_array_contains as *const u8,
        );
        builder.symbol(
            "naml_array_any",
            crate::runtime::naml_array_any as *const u8,
        );
        builder.symbol(
            "naml_array_all",
            crate::runtime::naml_array_all as *const u8,
        );
        builder.symbol(
            "naml_array_count_if",
            crate::runtime::naml_array_count_if as *const u8,
        );
        builder.symbol(
            "naml_array_map",
            crate::runtime::naml_array_map as *const u8,
        );
        builder.symbol(
            "naml_array_filter",
            crate::runtime::naml_array_filter as *const u8,
        );
        builder.symbol(
            "naml_array_find",
            crate::runtime::naml_array_find as *const u8,
        );
        builder.symbol(
            "naml_array_find_index",
            crate::runtime::naml_array_find_index as *const u8,
        );
        builder.symbol(
            "naml_array_fold",
            crate::runtime::naml_array_fold as *const u8,
        );
        builder.symbol(
            "naml_array_flatten",
            crate::runtime::naml_array_flatten as *const u8,
        );
        builder.symbol(
            "naml_array_sort",
            crate::runtime::naml_array_sort as *const u8,
        );
        builder.symbol(
            "naml_array_sort_by",
            crate::runtime::naml_array_sort_by as *const u8,
        );
        builder.symbol(
            "naml_array_print",
            crate::runtime::naml_array_print as *const u8,
        );
        builder.symbol(
            "naml_array_print_strings",
            crate::runtime::naml_array_print_strings as *const u8,
        );
        builder.symbol(
            "naml_map_print",
            crate::runtime::naml_map_print as *const u8,
        );
        builder.symbol(
            "naml_map_print_string_values",
            crate::runtime::naml_map_print_string_values as *const u8,
        );
        builder.symbol(
            "naml_map_print_float_values",
            crate::runtime::naml_map_print_float_values as *const u8,
        );
        builder.symbol(
            "naml_map_print_bool_values",
            crate::runtime::naml_map_print_bool_values as *const u8,
        );
        builder.symbol(
            "naml_array_incref",
            crate::runtime::naml_array_incref as *const u8,
        );
        builder.symbol(
            "naml_array_decref",
            crate::runtime::naml_array_decref as *const u8,
        );
        builder.symbol(
            "naml_array_decref_strings",
            crate::runtime::naml_array_decref_strings as *const u8,
        );
        builder.symbol(
            "naml_array_decref_arrays",
            crate::runtime::naml_array_decref_arrays as *const u8,
        );
        builder.symbol(
            "naml_array_decref_maps",
            crate::runtime::naml_array_decref_maps as *const u8,
        );
        builder.symbol(
            "naml_array_decref_structs",
            crate::runtime::naml_array_decref_structs as *const u8,
        );
        // New array functions - Mutation
        builder.symbol(
            "naml_array_insert",
            crate::runtime::naml_array_insert as *const u8,
        );
        builder.symbol(
            "naml_array_remove_at",
            crate::runtime::naml_array_remove_at as *const u8,
        );
        builder.symbol(
            "naml_array_remove",
            crate::runtime::naml_array_remove as *const u8,
        );
        builder.symbol(
            "naml_array_swap",
            crate::runtime::naml_array_swap as *const u8,
        );
        // Deduplication
        builder.symbol(
            "naml_array_unique",
            crate::runtime::naml_array_unique as *const u8,
        );
        builder.symbol(
            "naml_array_compact",
            crate::runtime::naml_array_compact as *const u8,
        );
        // Backward search
        builder.symbol(
            "naml_array_last_index_of",
            crate::runtime::naml_array_last_index_of as *const u8,
        );
        builder.symbol(
            "naml_array_find_last",
            crate::runtime::naml_array_find_last as *const u8,
        );
        builder.symbol(
            "naml_array_find_last_index",
            crate::runtime::naml_array_find_last_index as *const u8,
        );
        // Array combination
        builder.symbol(
            "naml_array_concat",
            crate::runtime::naml_array_concat as *const u8,
        );
        builder.symbol(
            "naml_array_zip",
            crate::runtime::naml_array_zip as *const u8,
        );
        builder.symbol(
            "naml_array_unzip",
            crate::runtime::naml_array_unzip as *const u8,
        );
        // Splitting
        builder.symbol(
            "naml_array_chunk",
            crate::runtime::naml_array_chunk as *const u8,
        );
        builder.symbol(
            "naml_array_partition",
            crate::runtime::naml_array_partition as *const u8,
        );
        // Set operations
        builder.symbol(
            "naml_array_intersect",
            crate::runtime::naml_array_intersect as *const u8,
        );
        builder.symbol(
            "naml_array_diff",
            crate::runtime::naml_array_diff as *const u8,
        );
        builder.symbol(
            "naml_array_union",
            crate::runtime::naml_array_union as *const u8,
        );
        // Advanced iteration
        builder.symbol(
            "naml_array_take_while",
            crate::runtime::naml_array_take_while as *const u8,
        );
        builder.symbol(
            "naml_array_drop_while",
            crate::runtime::naml_array_drop_while as *const u8,
        );
        builder.symbol(
            "naml_array_reject",
            crate::runtime::naml_array_reject as *const u8,
        );
        builder.symbol(
            "naml_array_flat_apply",
            crate::runtime::naml_array_flat_apply as *const u8,
        );
        builder.symbol(
            "naml_array_scan",
            crate::runtime::naml_array_scan as *const u8,
        );
        // Random
        builder.symbol(
            "naml_array_shuffle",
            crate::runtime::naml_array_shuffle as *const u8,
        );
        builder.symbol(
            "naml_array_sample",
            crate::runtime::naml_array_sample as *const u8,
        );
        builder.symbol(
            "naml_array_sample_n",
            crate::runtime::naml_array_sample_n as *const u8,
        );

        // Arena allocation
        builder.symbol(
            "naml_arena_alloc",
            crate::runtime::naml_arena_alloc as *const u8,
        );
        builder.symbol(
            "naml_arena_free_sized",
            crate::runtime::naml_arena_free_sized as *const u8,
        );
        builder.symbol(
            "naml_arena_get_tls_ptr",
            crate::runtime::naml_arena_get_tls_ptr as *const u8,
        );

        // Struct operations
        builder.symbol(
            "naml_struct_new",
            crate::runtime::naml_struct_new as *const u8,
        );
        builder.symbol(
            "naml_struct_incref",
            crate::runtime::naml_struct_incref as *const u8,
        );
        builder.symbol(
            "naml_struct_decref",
            crate::runtime::naml_struct_decref as *const u8,
        );
        builder.symbol(
            "naml_struct_free",
            crate::runtime::naml_struct_free as *const u8,
        );
        builder.symbol(
            "naml_struct_decref_iterative",
            crate::runtime::naml_struct_decref_iterative as *const u8,
        );
        builder.symbol(
            "naml_struct_incref_fast",
            crate::runtime::naml_struct_incref_fast as *const u8,
        );
        builder.symbol(
            "naml_struct_decref_fast",
            crate::runtime::naml_struct_decref_fast as *const u8,
        );
        builder.symbol(
            "naml_struct_get_field",
            crate::runtime::naml_struct_get_field as *const u8,
        );
        builder.symbol(
            "naml_struct_set_field",
            crate::runtime::naml_struct_set_field as *const u8,
        );

        // Scheduler operations
        builder.symbol("naml_spawn", crate::runtime::naml_spawn as *const u8);
        builder.symbol(
            "naml_spawn_closure",
            crate::runtime::naml_spawn_closure as *const u8,
        );
        builder.symbol(
            "naml_alloc_closure_data",
            crate::runtime::naml_alloc_closure_data as *const u8,
        );
        builder.symbol("naml_wait_all", crate::runtime::naml_wait_all as *const u8);
        builder.symbol("naml_sleep", crate::runtime::naml_sleep as *const u8);
        builder.symbol("naml_random", crate::runtime::naml_random as *const u8);
        builder.symbol(
            "naml_random_float",
            crate::runtime::naml_random_float as *const u8,
        );

        // Timer operations
        builder.symbol(
            "naml_timers_set_timeout",
            crate::runtime::naml_timers_set_timeout as *const u8,
        );
        builder.symbol(
            "naml_timers_cancel_timeout",
            crate::runtime::naml_timers_cancel_timeout as *const u8,
        );
        builder.symbol(
            "naml_timers_set_interval",
            crate::runtime::naml_timers_set_interval as *const u8,
        );
        builder.symbol(
            "naml_timers_cancel_interval",
            crate::runtime::naml_timers_cancel_interval as *const u8,
        );
        builder.symbol(
            "naml_timers_schedule",
            crate::runtime::naml_timers_schedule as *const u8,
        );
        builder.symbol(
            "naml_timers_cancel_schedule",
            crate::runtime::naml_timers_cancel_schedule as *const u8,
        );
        builder.symbol(
            "naml_timers_next_run",
            crate::runtime::naml_timers_next_run as *const u8,
        );

        // Crypto operations (from naml-std-crypto)
        builder.symbol("naml_crypto_md5", crate::runtime::naml_crypto_md5 as *const u8);
        builder.symbol("naml_crypto_md5_hex", crate::runtime::naml_crypto_md5_hex as *const u8);
        builder.symbol("naml_crypto_sha1", crate::runtime::naml_crypto_sha1 as *const u8);
        builder.symbol("naml_crypto_sha1_hex", crate::runtime::naml_crypto_sha1_hex as *const u8);
        builder.symbol("naml_crypto_sha256", crate::runtime::naml_crypto_sha256 as *const u8);
        builder.symbol("naml_crypto_sha256_hex", crate::runtime::naml_crypto_sha256_hex as *const u8);
        builder.symbol("naml_crypto_sha512", crate::runtime::naml_crypto_sha512 as *const u8);
        builder.symbol("naml_crypto_sha512_hex", crate::runtime::naml_crypto_sha512_hex as *const u8);
        builder.symbol("naml_crypto_hmac_sha256", crate::runtime::naml_crypto_hmac_sha256 as *const u8);
        builder.symbol("naml_crypto_hmac_sha256_hex", crate::runtime::naml_crypto_hmac_sha256_hex as *const u8);
        builder.symbol("naml_crypto_hmac_sha512", crate::runtime::naml_crypto_hmac_sha512 as *const u8);
        builder.symbol("naml_crypto_hmac_sha512_hex", crate::runtime::naml_crypto_hmac_sha512_hex as *const u8);
        builder.symbol("naml_crypto_hmac_verify_sha256", crate::runtime::naml_crypto_hmac_verify_sha256 as *const u8);
        builder.symbol("naml_crypto_hmac_verify_sha512", crate::runtime::naml_crypto_hmac_verify_sha512 as *const u8);
        builder.symbol("naml_crypto_pbkdf2_sha256", crate::runtime::naml_crypto_pbkdf2_sha256 as *const u8);
        builder.symbol("naml_crypto_random_bytes", crate::runtime::naml_crypto_random_bytes as *const u8);

        // Diagnostic builtins
        builder.symbol("naml_warn", crate::runtime::naml_warn as *const u8);
        builder.symbol("naml_error", crate::runtime::naml_error as *const u8);
        builder.symbol("naml_panic", crate::runtime::naml_panic as *const u8);
        builder.symbol(
            "naml_panic_unwrap",
            crate::runtime::naml_panic_unwrap as *const u8,
        );
        builder.symbol(
            "naml_string_concat",
            crate::runtime::naml_string_concat as *const u8,
        );

        // I/O builtins
        builder.symbol(
            "naml_read_line",
            crate::runtime::naml_read_line as *const u8,
        );
        builder.symbol("naml_read_key", crate::runtime::naml_read_key as *const u8);
        builder.symbol(
            "naml_clear_screen",
            crate::runtime::naml_clear_screen as *const u8,
        );
        builder.symbol(
            "naml_set_cursor",
            crate::runtime::naml_set_cursor as *const u8,
        );
        builder.symbol(
            "naml_hide_cursor",
            crate::runtime::naml_hide_cursor as *const u8,
        );
        builder.symbol(
            "naml_show_cursor",
            crate::runtime::naml_show_cursor as *const u8,
        );
        builder.symbol(
            "naml_terminal_width",
            crate::runtime::naml_terminal_width as *const u8,
        );
        builder.symbol(
            "naml_terminal_height",
            crate::runtime::naml_terminal_height as *const u8,
        );

        // Datetime operations
        builder.symbol(
            "naml_datetime_now_ms",
            crate::runtime::naml_datetime_now_ms as *const u8,
        );
        builder.symbol(
            "naml_datetime_now_s",
            crate::runtime::naml_datetime_now_s as *const u8,
        );
        builder.symbol(
            "naml_datetime_year",
            crate::runtime::naml_datetime_year as *const u8,
        );
        builder.symbol(
            "naml_datetime_month",
            crate::runtime::naml_datetime_month as *const u8,
        );
        builder.symbol(
            "naml_datetime_day",
            crate::runtime::naml_datetime_day as *const u8,
        );
        builder.symbol(
            "naml_datetime_hour",
            crate::runtime::naml_datetime_hour as *const u8,
        );
        builder.symbol(
            "naml_datetime_minute",
            crate::runtime::naml_datetime_minute as *const u8,
        );
        builder.symbol(
            "naml_datetime_second",
            crate::runtime::naml_datetime_second as *const u8,
        );
        builder.symbol(
            "naml_datetime_day_of_week",
            crate::runtime::naml_datetime_day_of_week as *const u8,
        );
        builder.symbol(
            "naml_datetime_format",
            crate::runtime::naml_datetime_format as *const u8,
        );

        // Metrics operations
        builder.symbol(
            "naml_metrics_perf_now",
            crate::runtime::naml_metrics_perf_now as *const u8,
        );
        builder.symbol(
            "naml_metrics_elapsed_ms",
            crate::runtime::naml_metrics_elapsed_ms as *const u8,
        );
        builder.symbol(
            "naml_metrics_elapsed_us",
            crate::runtime::naml_metrics_elapsed_us as *const u8,
        );
        builder.symbol(
            "naml_metrics_elapsed_ns",
            crate::runtime::naml_metrics_elapsed_ns as *const u8,
        );

        // Channel operations
        builder.symbol(
            "naml_channel_new",
            crate::runtime::naml_channel_new as *const u8,
        );
        builder.symbol(
            "naml_channel_send",
            crate::runtime::naml_channel_send as *const u8,
        );
        builder.symbol(
            "naml_channel_receive",
            crate::runtime::naml_channel_receive as *const u8,
        );
        builder.symbol(
            "naml_channel_close",
            crate::runtime::naml_channel_close as *const u8,
        );
        builder.symbol(
            "naml_channel_len",
            crate::runtime::naml_channel_len as *const u8,
        );
        builder.symbol(
            "naml_channel_incref",
            crate::runtime::naml_channel_incref as *const u8,
        );
        builder.symbol(
            "naml_channel_decref",
            crate::runtime::naml_channel_decref as *const u8,
        );

        // Mutex operations
        builder.symbol(
            "naml_mutex_new",
            crate::runtime::naml_mutex_new as *const u8,
        );
        builder.symbol(
            "naml_mutex_lock",
            crate::runtime::naml_mutex_lock as *const u8,
        );
        builder.symbol(
            "naml_mutex_unlock",
            crate::runtime::naml_mutex_unlock as *const u8,
        );
        builder.symbol(
            "naml_mutex_incref",
            crate::runtime::naml_mutex_incref as *const u8,
        );
        builder.symbol(
            "naml_mutex_decref",
            crate::runtime::naml_mutex_decref as *const u8,
        );

        // RwLock operations
        builder.symbol(
            "naml_rwlock_new",
            crate::runtime::naml_rwlock_new as *const u8,
        );
        builder.symbol(
            "naml_rwlock_read_lock",
            crate::runtime::naml_rwlock_read_lock as *const u8,
        );
        builder.symbol(
            "naml_rwlock_read_unlock",
            crate::runtime::naml_rwlock_read_unlock as *const u8,
        );
        builder.symbol(
            "naml_rwlock_write_lock",
            crate::runtime::naml_rwlock_write_lock as *const u8,
        );
        builder.symbol(
            "naml_rwlock_write_unlock",
            crate::runtime::naml_rwlock_write_unlock as *const u8,
        );
        builder.symbol(
            "naml_rwlock_incref",
            crate::runtime::naml_rwlock_incref as *const u8,
        );
        builder.symbol(
            "naml_rwlock_decref",
            crate::runtime::naml_rwlock_decref as *const u8,
        );

        // AtomicInt operations
        builder.symbol("naml_atomic_int_new", crate::runtime::naml_atomic_int_new as *const u8);
        builder.symbol("naml_atomic_int_load", crate::runtime::naml_atomic_int_load as *const u8);
        builder.symbol("naml_atomic_int_store", crate::runtime::naml_atomic_int_store as *const u8);
        builder.symbol("naml_atomic_int_add", crate::runtime::naml_atomic_int_add as *const u8);
        builder.symbol("naml_atomic_int_sub", crate::runtime::naml_atomic_int_sub as *const u8);
        builder.symbol("naml_atomic_int_inc", crate::runtime::naml_atomic_int_inc as *const u8);
        builder.symbol("naml_atomic_int_dec", crate::runtime::naml_atomic_int_dec as *const u8);
        builder.symbol("naml_atomic_int_cas", crate::runtime::naml_atomic_int_cas as *const u8);
        builder.symbol("naml_atomic_int_swap", crate::runtime::naml_atomic_int_swap as *const u8);
        builder.symbol("naml_atomic_int_and", crate::runtime::naml_atomic_int_and as *const u8);
        builder.symbol("naml_atomic_int_or", crate::runtime::naml_atomic_int_or as *const u8);
        builder.symbol("naml_atomic_int_xor", crate::runtime::naml_atomic_int_xor as *const u8);
        builder.symbol("naml_atomic_int_incref", crate::runtime::naml_atomic_int_incref as *const u8);
        builder.symbol("naml_atomic_int_decref", crate::runtime::naml_atomic_int_decref as *const u8);

        // AtomicUint operations
        builder.symbol("naml_atomic_uint_new", crate::runtime::naml_atomic_uint_new as *const u8);
        builder.symbol("naml_atomic_uint_load", crate::runtime::naml_atomic_uint_load as *const u8);
        builder.symbol("naml_atomic_uint_store", crate::runtime::naml_atomic_uint_store as *const u8);
        builder.symbol("naml_atomic_uint_add", crate::runtime::naml_atomic_uint_add as *const u8);
        builder.symbol("naml_atomic_uint_sub", crate::runtime::naml_atomic_uint_sub as *const u8);
        builder.symbol("naml_atomic_uint_inc", crate::runtime::naml_atomic_uint_inc as *const u8);
        builder.symbol("naml_atomic_uint_dec", crate::runtime::naml_atomic_uint_dec as *const u8);
        builder.symbol("naml_atomic_uint_cas", crate::runtime::naml_atomic_uint_cas as *const u8);
        builder.symbol("naml_atomic_uint_swap", crate::runtime::naml_atomic_uint_swap as *const u8);
        builder.symbol("naml_atomic_uint_and", crate::runtime::naml_atomic_uint_and as *const u8);
        builder.symbol("naml_atomic_uint_or", crate::runtime::naml_atomic_uint_or as *const u8);
        builder.symbol("naml_atomic_uint_xor", crate::runtime::naml_atomic_uint_xor as *const u8);
        builder.symbol("naml_atomic_uint_incref", crate::runtime::naml_atomic_uint_incref as *const u8);
        builder.symbol("naml_atomic_uint_decref", crate::runtime::naml_atomic_uint_decref as *const u8);

        // AtomicBool operations
        builder.symbol("naml_atomic_bool_new", crate::runtime::naml_atomic_bool_new as *const u8);
        builder.symbol("naml_atomic_bool_load", crate::runtime::naml_atomic_bool_load as *const u8);
        builder.symbol("naml_atomic_bool_store", crate::runtime::naml_atomic_bool_store as *const u8);
        builder.symbol("naml_atomic_bool_cas", crate::runtime::naml_atomic_bool_cas as *const u8);
        builder.symbol("naml_atomic_bool_swap", crate::runtime::naml_atomic_bool_swap as *const u8);
        builder.symbol("naml_atomic_bool_incref", crate::runtime::naml_atomic_bool_incref as *const u8);
        builder.symbol("naml_atomic_bool_decref", crate::runtime::naml_atomic_bool_decref as *const u8);

        // Map operations
        builder.symbol("naml_map_new", crate::runtime::naml_map_new as *const u8);
        builder.symbol("naml_map_set", crate::runtime::naml_map_set as *const u8);
        builder.symbol(
            "naml_map_set_string",
            crate::runtime::naml_map_set_string as *const u8,
        );
        builder.symbol(
            "naml_map_set_array",
            crate::runtime::naml_map_set_array as *const u8,
        );
        builder.symbol(
            "naml_map_set_map",
            crate::runtime::naml_map_set_map as *const u8,
        );
        builder.symbol(
            "naml_map_set_struct",
            crate::runtime::naml_map_set_struct as *const u8,
        );
        builder.symbol("naml_map_get", crate::runtime::naml_map_get as *const u8);
        builder.symbol(
            "naml_map_contains",
            crate::runtime::naml_map_contains as *const u8,
        );
        builder.symbol("naml_map_len", crate::runtime::naml_map_len as *const u8);
        builder.symbol(
            "naml_map_incref",
            crate::runtime::naml_map_incref as *const u8,
        );
        builder.symbol(
            "naml_map_decref",
            crate::runtime::naml_map_decref as *const u8,
        );
        builder.symbol(
            "naml_map_decref_strings",
            crate::runtime::naml_map_decref_strings as *const u8,
        );
        builder.symbol(
            "naml_map_decref_arrays",
            crate::runtime::naml_map_decref_arrays as *const u8,
        );
        builder.symbol(
            "naml_map_decref_maps",
            crate::runtime::naml_map_decref_maps as *const u8,
        );
        builder.symbol(
            "naml_map_decref_structs",
            crate::runtime::naml_map_decref_structs as *const u8,
        );

        // Map collection operations (from naml-std-collections)
        builder.symbol(
            "naml_map_count",
            crate::runtime::naml_map_count as *const u8,
        );
        builder.symbol(
            "naml_map_contains_key",
            crate::runtime::naml_map_contains_key as *const u8,
        );
        builder.symbol(
            "naml_map_remove",
            crate::runtime::naml_map_remove as *const u8,
        );
        builder.symbol(
            "naml_map_clear",
            crate::runtime::naml_map_clear as *const u8,
        );
        builder.symbol("naml_map_keys", crate::runtime::naml_map_keys as *const u8);
        builder.symbol(
            "naml_map_values",
            crate::runtime::naml_map_values as *const u8,
        );
        builder.symbol(
            "naml_map_entries",
            crate::runtime::naml_map_entries as *const u8,
        );
        builder.symbol(
            "naml_map_first_key",
            crate::runtime::naml_map_first_key as *const u8,
        );
        builder.symbol(
            "naml_map_first_value",
            crate::runtime::naml_map_first_value as *const u8,
        );
        builder.symbol("naml_map_any", crate::runtime::naml_map_any as *const u8);
        builder.symbol("naml_map_all", crate::runtime::naml_map_all as *const u8);
        builder.symbol(
            "naml_map_count_if",
            crate::runtime::naml_map_count_if as *const u8,
        );
        builder.symbol("naml_map_fold", crate::runtime::naml_map_fold as *const u8);
        builder.symbol(
            "naml_map_transform",
            crate::runtime::naml_map_transform as *const u8,
        );
        builder.symbol(
            "naml_map_where",
            crate::runtime::naml_map_where as *const u8,
        );
        builder.symbol(
            "naml_map_reject",
            crate::runtime::naml_map_reject as *const u8,
        );
        builder.symbol(
            "naml_map_merge",
            crate::runtime::naml_map_merge as *const u8,
        );
        builder.symbol(
            "naml_map_defaults",
            crate::runtime::naml_map_defaults as *const u8,
        );
        builder.symbol(
            "naml_map_intersect",
            crate::runtime::naml_map_intersect as *const u8,
        );
        builder.symbol("naml_map_diff", crate::runtime::naml_map_diff as *const u8);
        builder.symbol(
            "naml_map_invert",
            crate::runtime::naml_map_invert as *const u8,
        );
        builder.symbol(
            "naml_map_from_arrays",
            crate::runtime::naml_map_from_arrays as *const u8,
        );
        builder.symbol(
            "naml_map_from_entries",
            crate::runtime::naml_map_from_entries as *const u8,
        );

        // File system operations (from naml-std-fs)
        builder.symbol("naml_fs_read", crate::runtime::naml_fs_read as *const u8);
        builder.symbol(
            "naml_fs_read_bytes",
            crate::runtime::naml_fs_read_bytes as *const u8,
        );
        builder.symbol("naml_fs_write", crate::runtime::naml_fs_write as *const u8);
        builder.symbol(
            "naml_fs_append",
            crate::runtime::naml_fs_append as *const u8,
        );
        builder.symbol(
            "naml_fs_write_bytes",
            crate::runtime::naml_fs_write_bytes as *const u8,
        );
        builder.symbol(
            "naml_fs_append_bytes",
            crate::runtime::naml_fs_append_bytes as *const u8,
        );
        builder.symbol(
            "naml_fs_exists",
            crate::runtime::naml_fs_exists as *const u8,
        );
        builder.symbol(
            "naml_fs_is_file",
            crate::runtime::naml_fs_is_file as *const u8,
        );
        builder.symbol(
            "naml_fs_is_dir",
            crate::runtime::naml_fs_is_dir as *const u8,
        );
        builder.symbol(
            "naml_fs_list_dir",
            crate::runtime::naml_fs_list_dir as *const u8,
        );
        builder.symbol("naml_fs_mkdir", crate::runtime::naml_fs_mkdir as *const u8);
        builder.symbol(
            "naml_fs_mkdir_all",
            crate::runtime::naml_fs_mkdir_all as *const u8,
        );
        builder.symbol(
            "naml_fs_remove",
            crate::runtime::naml_fs_remove as *const u8,
        );
        builder.symbol(
            "naml_fs_remove_all",
            crate::runtime::naml_fs_remove_all as *const u8,
        );
        builder.symbol("naml_fs_join", crate::runtime::naml_fs_join as *const u8);
        builder.symbol(
            "naml_fs_dirname",
            crate::runtime::naml_fs_dirname as *const u8,
        );
        builder.symbol(
            "naml_fs_basename",
            crate::runtime::naml_fs_basename as *const u8,
        );
        builder.symbol(
            "naml_fs_extension",
            crate::runtime::naml_fs_extension as *const u8,
        );
        builder.symbol(
            "naml_fs_absolute",
            crate::runtime::naml_fs_absolute as *const u8,
        );
        builder.symbol("naml_fs_size", crate::runtime::naml_fs_size as *const u8);
        builder.symbol(
            "naml_fs_modified",
            crate::runtime::naml_fs_modified as *const u8,
        );
        builder.symbol("naml_fs_copy", crate::runtime::naml_fs_copy as *const u8);
        builder.symbol(
            "naml_fs_rename",
            crate::runtime::naml_fs_rename as *const u8,
        );
        builder.symbol("naml_fs_getwd", crate::runtime::naml_fs_getwd as *const u8);
        builder.symbol("naml_fs_chdir", crate::runtime::naml_fs_chdir as *const u8);
        builder.symbol(
            "naml_fs_create_temp",
            crate::runtime::naml_fs_create_temp as *const u8,
        );
        builder.symbol(
            "naml_fs_mkdir_temp",
            crate::runtime::naml_fs_mkdir_temp as *const u8,
        );
        builder.symbol("naml_fs_chmod", crate::runtime::naml_fs_chmod as *const u8);
        builder.symbol(
            "naml_fs_truncate",
            crate::runtime::naml_fs_truncate as *const u8,
        );
        builder.symbol("naml_fs_stat", crate::runtime::naml_fs_stat as *const u8);
        builder.symbol(
            "naml_fs_symlink",
            crate::runtime::naml_fs_symlink as *const u8,
        );
        builder.symbol(
            "naml_fs_readlink",
            crate::runtime::naml_fs_readlink as *const u8,
        );
        builder.symbol(
            "naml_fs_lstat",
            crate::runtime::naml_fs_lstat as *const u8,
        );
        builder.symbol("naml_fs_link", crate::runtime::naml_fs_link as *const u8);
        builder.symbol(
            "naml_fs_chtimes",
            crate::runtime::naml_fs_chtimes as *const u8,
        );
        builder.symbol(
            "naml_fs_chown",
            crate::runtime::naml_fs_chown as *const u8,
        );
        builder.symbol(
            "naml_fs_lchown",
            crate::runtime::naml_fs_lchown as *const u8,
        );
        builder.symbol(
            "naml_fs_same_file",
            crate::runtime::naml_fs_same_file as *const u8,
        );
        builder.symbol(
            "naml_fs_file_read_at",
            crate::runtime::naml_fs_file_read_at as *const u8,
        );
        builder.symbol(
            "naml_fs_file_write_at",
            crate::runtime::naml_fs_file_write_at as *const u8,
        );
        builder.symbol(
            "naml_fs_file_name",
            crate::runtime::naml_fs_file_name as *const u8,
        );
        builder.symbol(
            "naml_fs_file_stat",
            crate::runtime::naml_fs_file_stat as *const u8,
        );
        builder.symbol(
            "naml_fs_file_truncate",
            crate::runtime::naml_fs_file_truncate as *const u8,
        );
        builder.symbol(
            "naml_fs_file_chmod",
            crate::runtime::naml_fs_file_chmod as *const u8,
        );
        builder.symbol(
            "naml_fs_file_chown",
            crate::runtime::naml_fs_file_chown as *const u8,
        );
        builder.symbol(
            "naml_io_error_new",
            crate::runtime::naml_io_error_new as *const u8,
        );
        builder.symbol(
            "naml_permission_error_new",
            crate::runtime::naml_permission_error_new as *const u8,
        );

        // Memory-mapped file operations
        builder.symbol(
            "naml_fs_mmap_open",
            crate::runtime::naml_fs_mmap_open as *const u8,
        );
        builder.symbol(
            "naml_fs_mmap_len",
            crate::runtime::naml_fs_mmap_len as *const u8,
        );
        builder.symbol(
            "naml_fs_mmap_read_byte",
            crate::runtime::naml_fs_mmap_read_byte as *const u8,
        );
        builder.symbol(
            "naml_fs_mmap_write_byte",
            crate::runtime::naml_fs_mmap_write_byte as *const u8,
        );
        builder.symbol(
            "naml_fs_mmap_read",
            crate::runtime::naml_fs_mmap_read as *const u8,
        );
        builder.symbol(
            "naml_fs_mmap_write",
            crate::runtime::naml_fs_mmap_write as *const u8,
        );
        builder.symbol(
            "naml_fs_mmap_flush",
            crate::runtime::naml_fs_mmap_flush as *const u8,
        );
        builder.symbol(
            "naml_fs_mmap_close",
            crate::runtime::naml_fs_mmap_close as *const u8,
        );

        // File handle operations
        builder.symbol(
            "naml_fs_file_open",
            crate::runtime::naml_fs_file_open as *const u8,
        );
        builder.symbol(
            "naml_fs_file_close",
            crate::runtime::naml_fs_file_close as *const u8,
        );
        builder.symbol(
            "naml_fs_file_read",
            crate::runtime::naml_fs_file_read as *const u8,
        );
        builder.symbol(
            "naml_fs_file_read_line",
            crate::runtime::naml_fs_file_read_line as *const u8,
        );
        builder.symbol(
            "naml_fs_file_read_all",
            crate::runtime::naml_fs_file_read_all as *const u8,
        );
        builder.symbol(
            "naml_fs_file_write",
            crate::runtime::naml_fs_file_write as *const u8,
        );
        builder.symbol(
            "naml_fs_file_write_line",
            crate::runtime::naml_fs_file_write_line as *const u8,
        );
        builder.symbol(
            "naml_fs_file_flush",
            crate::runtime::naml_fs_file_flush as *const u8,
        );
        builder.symbol(
            "naml_fs_file_seek",
            crate::runtime::naml_fs_file_seek as *const u8,
        );
        builder.symbol(
            "naml_fs_file_tell",
            crate::runtime::naml_fs_file_tell as *const u8,
        );
        builder.symbol(
            "naml_fs_file_eof",
            crate::runtime::naml_fs_file_eof as *const u8,
        );
        builder.symbol(
            "naml_fs_file_size",
            crate::runtime::naml_fs_file_size as *const u8,
        );

        // Path operations (from naml-std-path)
        builder.symbol(
            "naml_path_join",
            crate::runtime::naml_path_join as *const u8,
        );
        builder.symbol(
            "naml_path_normalize",
            crate::runtime::naml_path_normalize as *const u8,
        );
        builder.symbol(
            "naml_path_is_absolute",
            crate::runtime::naml_path_is_absolute as *const u8,
        );
        builder.symbol(
            "naml_path_is_relative",
            crate::runtime::naml_path_is_relative as *const u8,
        );
        builder.symbol(
            "naml_path_has_root",
            crate::runtime::naml_path_has_root as *const u8,
        );
        builder.symbol(
            "naml_path_dirname",
            crate::runtime::naml_path_dirname as *const u8,
        );
        builder.symbol(
            "naml_path_basename",
            crate::runtime::naml_path_basename as *const u8,
        );
        builder.symbol(
            "naml_path_extension",
            crate::runtime::naml_path_extension as *const u8,
        );
        builder.symbol(
            "naml_path_stem",
            crate::runtime::naml_path_stem as *const u8,
        );
        builder.symbol(
            "naml_path_with_extension",
            crate::runtime::naml_path_with_extension as *const u8,
        );
        builder.symbol(
            "naml_path_components",
            crate::runtime::naml_path_components as *const u8,
        );
        builder.symbol(
            "naml_path_separator",
            crate::runtime::naml_path_separator as *const u8,
        );
        builder.symbol(
            "naml_path_to_slash",
            crate::runtime::naml_path_to_slash as *const u8,
        );
        builder.symbol(
            "naml_path_from_slash",
            crate::runtime::naml_path_from_slash as *const u8,
        );
        builder.symbol(
            "naml_path_starts_with",
            crate::runtime::naml_path_starts_with as *const u8,
        );
        builder.symbol(
            "naml_path_ends_with",
            crate::runtime::naml_path_ends_with as *const u8,
        );
        builder.symbol(
            "naml_path_strip_prefix",
            crate::runtime::naml_path_strip_prefix as *const u8,
        );

        // Environment operations (from naml-std-env)
        builder.symbol(
            "naml_env_getenv",
            crate::runtime::naml_env_getenv as *const u8,
        );
        builder.symbol(
            "naml_env_lookup_env",
            crate::runtime::naml_env_lookup_env as *const u8,
        );
        builder.symbol(
            "naml_env_setenv",
            crate::runtime::naml_env_setenv as *const u8,
        );
        builder.symbol(
            "naml_env_unsetenv",
            crate::runtime::naml_env_unsetenv as *const u8,
        );
        builder.symbol(
            "naml_env_clearenv",
            crate::runtime::naml_env_clearenv as *const u8,
        );
        builder.symbol(
            "naml_env_environ",
            crate::runtime::naml_env_environ as *const u8,
        );
        builder.symbol(
            "naml_env_expand_env",
            crate::runtime::naml_env_expand_env as *const u8,
        );
        builder.symbol(
            "naml_env_error_new",
            crate::runtime::naml_env_error_new as *const u8,
        );

        // OS operations (from naml-std-os)
        builder.symbol(
            "naml_os_hostname",
            crate::runtime::naml_os_hostname as *const u8,
        );
        builder.symbol(
            "naml_os_temp_dir",
            crate::runtime::naml_os_temp_dir as *const u8,
        );
        builder.symbol(
            "naml_os_home_dir",
            crate::runtime::naml_os_home_dir as *const u8,
        );
        builder.symbol(
            "naml_os_cache_dir",
            crate::runtime::naml_os_cache_dir as *const u8,
        );
        builder.symbol(
            "naml_os_config_dir",
            crate::runtime::naml_os_config_dir as *const u8,
        );
        builder.symbol(
            "naml_os_executable",
            crate::runtime::naml_os_executable as *const u8,
        );
        builder.symbol(
            "naml_os_pagesize",
            crate::runtime::naml_os_pagesize as *const u8,
        );
        builder.symbol(
            "naml_os_getuid",
            crate::runtime::naml_os_getuid as *const u8,
        );
        builder.symbol(
            "naml_os_geteuid",
            crate::runtime::naml_os_geteuid as *const u8,
        );
        builder.symbol(
            "naml_os_getgid",
            crate::runtime::naml_os_getgid as *const u8,
        );
        builder.symbol(
            "naml_os_getegid",
            crate::runtime::naml_os_getegid as *const u8,
        );
        builder.symbol(
            "naml_os_getgroups",
            crate::runtime::naml_os_getgroups as *const u8,
        );
        builder.symbol(
            "naml_os_error_new",
            crate::runtime::naml_os_error_new as *const u8,
        );

        // Process operations (from naml-std-process)
        builder.symbol(
            "naml_process_getpid",
            crate::runtime::naml_process_getpid as *const u8,
        );
        builder.symbol(
            "naml_process_getppid",
            crate::runtime::naml_process_getppid as *const u8,
        );
        builder.symbol(
            "naml_process_exit",
            crate::runtime::naml_process_exit as *const u8,
        );
        builder.symbol(
            "naml_process_pipe_read",
            crate::runtime::naml_process_pipe_read as *const u8,
        );
        builder.symbol(
            "naml_process_pipe_write",
            crate::runtime::naml_process_pipe_write as *const u8,
        );
        builder.symbol(
            "naml_process_start",
            crate::runtime::naml_process_start as *const u8,
        );
        builder.symbol(
            "naml_process_find",
            crate::runtime::naml_process_find as *const u8,
        );
        builder.symbol(
            "naml_process_wait",
            crate::runtime::naml_process_wait as *const u8,
        );
        builder.symbol(
            "naml_process_signal",
            crate::runtime::naml_process_signal as *const u8,
        );
        builder.symbol(
            "naml_process_kill",
            crate::runtime::naml_process_kill as *const u8,
        );
        builder.symbol(
            "naml_process_release",
            crate::runtime::naml_process_release as *const u8,
        );
        builder.symbol(
            "naml_process_error_new",
            crate::runtime::naml_process_error_new as *const u8,
        );
        builder.symbol(
            "naml_process_sighup",
            crate::runtime::naml_process_sighup as *const u8,
        );
        builder.symbol(
            "naml_process_sigint",
            crate::runtime::naml_process_sigint as *const u8,
        );
        builder.symbol(
            "naml_process_sigquit",
            crate::runtime::naml_process_sigquit as *const u8,
        );
        builder.symbol(
            "naml_process_sigkill",
            crate::runtime::naml_process_sigkill as *const u8,
        );
        builder.symbol(
            "naml_process_sigterm",
            crate::runtime::naml_process_sigterm as *const u8,
        );
        builder.symbol(
            "naml_process_sigstop",
            crate::runtime::naml_process_sigstop as *const u8,
        );
        builder.symbol(
            "naml_process_sigcont",
            crate::runtime::naml_process_sigcont as *const u8,
        );

        // Testing operations (from naml-std-testing)
        builder.symbol(
            "naml_testing_assert",
            crate::runtime::naml_testing_assert as *const u8,
        );
        builder.symbol(
            "naml_testing_assert_eq",
            crate::runtime::naml_testing_assert_eq as *const u8,
        );
        builder.symbol(
            "naml_testing_assert_eq_float",
            crate::runtime::naml_testing_assert_eq_float as *const u8,
        );
        builder.symbol(
            "naml_testing_assert_eq_string",
            crate::runtime::naml_testing_assert_eq_string as *const u8,
        );
        builder.symbol(
            "naml_testing_assert_eq_bool",
            crate::runtime::naml_testing_assert_eq_bool as *const u8,
        );
        builder.symbol(
            "naml_testing_assert_neq",
            crate::runtime::naml_testing_assert_neq as *const u8,
        );
        builder.symbol(
            "naml_testing_assert_neq_string",
            crate::runtime::naml_testing_assert_neq_string as *const u8,
        );
        builder.symbol(
            "naml_testing_assert_true",
            crate::runtime::naml_testing_assert_true as *const u8,
        );
        builder.symbol(
            "naml_testing_assert_false",
            crate::runtime::naml_testing_assert_false as *const u8,
        );
        builder.symbol(
            "naml_testing_assert_gt",
            crate::runtime::naml_testing_assert_gt as *const u8,
        );
        builder.symbol(
            "naml_testing_assert_gte",
            crate::runtime::naml_testing_assert_gte as *const u8,
        );
        builder.symbol(
            "naml_testing_assert_lt",
            crate::runtime::naml_testing_assert_lt as *const u8,
        );
        builder.symbol(
            "naml_testing_assert_lte",
            crate::runtime::naml_testing_assert_lte as *const u8,
        );
        builder.symbol(
            "naml_testing_fail",
            crate::runtime::naml_testing_fail as *const u8,
        );
        builder.symbol(
            "naml_testing_assert_approx",
            crate::runtime::naml_testing_assert_approx as *const u8,
        );
        builder.symbol(
            "naml_testing_assert_contains",
            crate::runtime::naml_testing_assert_contains as *const u8,
        );
        builder.symbol(
            "naml_testing_assert_starts_with",
            crate::runtime::naml_testing_assert_starts_with as *const u8,
        );
        builder.symbol(
            "naml_testing_assert_ends_with",
            crate::runtime::naml_testing_assert_ends_with as *const u8,
        );

        // Exception handling
        builder.symbol(
            "naml_exception_set",
            crate::runtime::naml_exception_set as *const u8,
        );
        builder.symbol(
            "naml_exception_get",
            crate::runtime::naml_exception_get as *const u8,
        );
        builder.symbol(
            "naml_exception_clear",
            crate::runtime::naml_exception_clear as *const u8,
        );
        builder.symbol(
            "naml_exception_check",
            crate::runtime::naml_exception_check as *const u8,
        );
        builder.symbol(
            "naml_exception_set_typed",
            crate::runtime::naml_exception_set_typed as *const u8,
        );
        builder.symbol(
            "naml_exception_get_type_id",
            crate::runtime::naml_exception_get_type_id as *const u8,
        );
        builder.symbol(
            "naml_exception_is_type",
            crate::runtime::naml_exception_is_type as *const u8,
        );
        builder.symbol(
            "naml_exception_clear_ptr",
            crate::runtime::naml_exception_clear_ptr as *const u8,
        );

        // Stack trace functions
        builder.symbol(
            "naml_stack_push",
            crate::runtime::naml_stack_push as *const u8,
        );
        builder.symbol(
            "naml_stack_pop",
            crate::runtime::naml_stack_pop as *const u8,
        );
        builder.symbol(
            "NAML_SHADOW_STACK",
            std::ptr::addr_of!(crate::runtime::NAML_SHADOW_STACK) as *const u8,
        );
        builder.symbol(
            "naml_stack_capture",
            crate::runtime::naml_stack_capture as *const u8,
        );
        builder.symbol(
            "naml_stack_clear",
            crate::runtime::naml_stack_clear as *const u8,
        );
        builder.symbol(
            "naml_stack_format",
            crate::runtime::naml_stack_format as *const u8,
        );

        // String operations
        builder.symbol(
            "naml_string_from_cstr",
            crate::runtime::naml_string_from_cstr as *const u8,
        );
        builder.symbol(
            "naml_string_print",
            crate::runtime::naml_string_print as *const u8,
        );
        builder.symbol(
            "naml_string_eq",
            crate::runtime::naml_string_eq as *const u8,
        );
        builder.symbol(
            "naml_string_incref",
            crate::runtime::naml_string_incref as *const u8,
        );
        builder.symbol(
            "naml_string_decref",
            crate::runtime::naml_string_decref as *const u8,
        );
        builder.symbol(
            "naml_string_char_at",
            crate::runtime::naml_string_char_at as *const u8,
        );
        builder.symbol(
            "naml_string_char_len",
            crate::runtime::naml_string_char_len as *const u8,
        );
        builder.symbol(
            "naml_string_is_empty",
            crate::runtime::naml_string_is_empty as *const u8,
        );
        builder.symbol(
            "naml_string_trim",
            crate::runtime::naml_string_trim as *const u8,
        );
        builder.symbol(
            "naml_string_upper",
            crate::runtime::naml_string_upper as *const u8,
        );
        builder.symbol(
            "naml_string_lower",
            crate::runtime::naml_string_lower as *const u8,
        );
        builder.symbol(
            "naml_string_contains",
            crate::runtime::naml_string_contains as *const u8,
        );
        builder.symbol(
            "naml_string_starts_with",
            crate::runtime::naml_string_starts_with as *const u8,
        );
        builder.symbol(
            "naml_string_ends_with",
            crate::runtime::naml_string_ends_with as *const u8,
        );
        builder.symbol(
            "naml_string_replace",
            crate::runtime::naml_string_replace as *const u8,
        );
        builder.symbol(
            "naml_string_replace_all",
            crate::runtime::naml_string_replace_all as *const u8,
        );
        builder.symbol(
            "naml_string_split",
            crate::runtime::naml_string_split as *const u8,
        );
        builder.symbol(
            "naml_string_join",
            crate::runtime::naml_string_join as *const u8,
        );
        builder.symbol(
            "naml_string_ltrim",
            crate::runtime::naml_string_ltrim as *const u8,
        );
        builder.symbol(
            "naml_string_rtrim",
            crate::runtime::naml_string_rtrim as *const u8,
        );
        builder.symbol(
            "naml_string_substr",
            crate::runtime::naml_string_substr as *const u8,
        );
        builder.symbol(
            "naml_string_lpad",
            crate::runtime::naml_string_lpad as *const u8,
        );
        builder.symbol(
            "naml_string_rpad",
            crate::runtime::naml_string_rpad as *const u8,
        );
        builder.symbol(
            "naml_string_repeat",
            crate::runtime::naml_string_repeat as *const u8,
        );
        builder.symbol(
            "naml_string_lines",
            crate::runtime::naml_string_lines as *const u8,
        );
        builder.symbol(
            "naml_string_chars",
            crate::runtime::naml_string_chars as *const u8,
        );

        // Type conversion operations
        builder.symbol(
            "naml_int_to_string",
            crate::runtime::naml_int_to_string as *const u8,
        );
        builder.symbol(
            "naml_float_to_string",
            crate::runtime::naml_float_to_string as *const u8,
        );
        builder.symbol(
            "naml_string_to_int",
            crate::runtime::naml_string_to_int as *const u8,
        );
        builder.symbol(
            "naml_string_to_float",
            crate::runtime::naml_string_to_float as *const u8,
        );
        builder.symbol(
            "naml_string_try_to_int",
            crate::runtime::naml_string_try_to_int as *const u8,
        );
        builder.symbol(
            "naml_string_try_to_float",
            crate::runtime::naml_string_try_to_float as *const u8,
        );

        // Bytes operations
        builder.symbol(
            "naml_bytes_new",
            crate::runtime::naml_bytes_new as *const u8,
        );
        builder.symbol(
            "naml_bytes_from",
            crate::runtime::naml_bytes_from as *const u8,
        );
        builder.symbol(
            "naml_bytes_len",
            crate::runtime::naml_bytes_len as *const u8,
        );
        builder.symbol(
            "naml_bytes_get",
            crate::runtime::naml_bytes_get as *const u8,
        );
        builder.symbol(
            "naml_bytes_set",
            crate::runtime::naml_bytes_set as *const u8,
        );
        builder.symbol(
            "naml_bytes_incref",
            crate::runtime::naml_bytes_incref as *const u8,
        );
        builder.symbol(
            "naml_bytes_decref",
            crate::runtime::naml_bytes_decref as *const u8,
        );
        builder.symbol(
            "naml_bytes_to_string",
            crate::runtime::naml_bytes_to_string as *const u8,
        );
        builder.symbol(
            "naml_string_to_bytes",
            crate::runtime::naml_string_to_bytes as *const u8,
        );

        // Encoding operations (from naml-std-encoding)
        builder.symbol(
            "naml_encoding_utf8_encode",
            crate::runtime::naml_encoding_utf8_encode as *const u8,
        );
        builder.symbol(
            "naml_encoding_utf8_decode",
            crate::runtime::naml_encoding_utf8_decode as *const u8,
        );
        builder.symbol(
            "naml_encoding_utf8_is_valid",
            crate::runtime::naml_encoding_utf8_is_valid as *const u8,
        );
        builder.symbol(
            "naml_encoding_hex_encode",
            crate::runtime::naml_encoding_hex_encode as *const u8,
        );
        builder.symbol(
            "naml_encoding_hex_decode",
            crate::runtime::naml_encoding_hex_decode as *const u8,
        );
        builder.symbol(
            "naml_encoding_base64_encode",
            crate::runtime::naml_encoding_base64_encode as *const u8,
        );
        builder.symbol(
            "naml_encoding_base64_decode",
            crate::runtime::naml_encoding_base64_decode as *const u8,
        );
        builder.symbol(
            "naml_encoding_url_encode",
            crate::runtime::naml_encoding_url_encode as *const u8,
        );
        builder.symbol(
            "naml_encoding_url_decode",
            crate::runtime::naml_encoding_url_decode as *const u8,
        );
        builder.symbol(
            "naml_decode_error_new",
            crate::runtime::naml_decode_error_new as *const u8,
        );

        // Binary encoding operations
        builder.symbol("naml_encoding_binary_read_u8", crate::runtime::naml_encoding_binary_read_u8 as *const u8);
        builder.symbol("naml_encoding_binary_read_i8", crate::runtime::naml_encoding_binary_read_i8 as *const u8);
        builder.symbol("naml_encoding_binary_read_u16_be", crate::runtime::naml_encoding_binary_read_u16_be as *const u8);
        builder.symbol("naml_encoding_binary_read_u16_le", crate::runtime::naml_encoding_binary_read_u16_le as *const u8);
        builder.symbol("naml_encoding_binary_read_i16_be", crate::runtime::naml_encoding_binary_read_i16_be as *const u8);
        builder.symbol("naml_encoding_binary_read_i16_le", crate::runtime::naml_encoding_binary_read_i16_le as *const u8);
        builder.symbol("naml_encoding_binary_read_u32_be", crate::runtime::naml_encoding_binary_read_u32_be as *const u8);
        builder.symbol("naml_encoding_binary_read_u32_le", crate::runtime::naml_encoding_binary_read_u32_le as *const u8);
        builder.symbol("naml_encoding_binary_read_i32_be", crate::runtime::naml_encoding_binary_read_i32_be as *const u8);
        builder.symbol("naml_encoding_binary_read_i32_le", crate::runtime::naml_encoding_binary_read_i32_le as *const u8);
        builder.symbol("naml_encoding_binary_read_u64_be", crate::runtime::naml_encoding_binary_read_u64_be as *const u8);
        builder.symbol("naml_encoding_binary_read_u64_le", crate::runtime::naml_encoding_binary_read_u64_le as *const u8);
        builder.symbol("naml_encoding_binary_read_i64_be", crate::runtime::naml_encoding_binary_read_i64_be as *const u8);
        builder.symbol("naml_encoding_binary_read_i64_le", crate::runtime::naml_encoding_binary_read_i64_le as *const u8);
        builder.symbol("naml_encoding_binary_read_f32_be", crate::runtime::naml_encoding_binary_read_f32_be as *const u8);
        builder.symbol("naml_encoding_binary_read_f32_le", crate::runtime::naml_encoding_binary_read_f32_le as *const u8);
        builder.symbol("naml_encoding_binary_read_f64_be", crate::runtime::naml_encoding_binary_read_f64_be as *const u8);
        builder.symbol("naml_encoding_binary_read_f64_le", crate::runtime::naml_encoding_binary_read_f64_le as *const u8);
        builder.symbol("naml_encoding_binary_write_u8", crate::runtime::naml_encoding_binary_write_u8 as *const u8);
        builder.symbol("naml_encoding_binary_write_i8", crate::runtime::naml_encoding_binary_write_i8 as *const u8);
        builder.symbol("naml_encoding_binary_write_u16_be", crate::runtime::naml_encoding_binary_write_u16_be as *const u8);
        builder.symbol("naml_encoding_binary_write_u16_le", crate::runtime::naml_encoding_binary_write_u16_le as *const u8);
        builder.symbol("naml_encoding_binary_write_i16_be", crate::runtime::naml_encoding_binary_write_i16_be as *const u8);
        builder.symbol("naml_encoding_binary_write_i16_le", crate::runtime::naml_encoding_binary_write_i16_le as *const u8);
        builder.symbol("naml_encoding_binary_write_u32_be", crate::runtime::naml_encoding_binary_write_u32_be as *const u8);
        builder.symbol("naml_encoding_binary_write_u32_le", crate::runtime::naml_encoding_binary_write_u32_le as *const u8);
        builder.symbol("naml_encoding_binary_write_i32_be", crate::runtime::naml_encoding_binary_write_i32_be as *const u8);
        builder.symbol("naml_encoding_binary_write_i32_le", crate::runtime::naml_encoding_binary_write_i32_le as *const u8);
        builder.symbol("naml_encoding_binary_write_u64_be", crate::runtime::naml_encoding_binary_write_u64_be as *const u8);
        builder.symbol("naml_encoding_binary_write_u64_le", crate::runtime::naml_encoding_binary_write_u64_le as *const u8);
        builder.symbol("naml_encoding_binary_write_i64_be", crate::runtime::naml_encoding_binary_write_i64_be as *const u8);
        builder.symbol("naml_encoding_binary_write_i64_le", crate::runtime::naml_encoding_binary_write_i64_le as *const u8);
        builder.symbol("naml_encoding_binary_write_f32_be", crate::runtime::naml_encoding_binary_write_f32_be as *const u8);
        builder.symbol("naml_encoding_binary_write_f32_le", crate::runtime::naml_encoding_binary_write_f32_le as *const u8);
        builder.symbol("naml_encoding_binary_write_f64_be", crate::runtime::naml_encoding_binary_write_f64_be as *const u8);
        builder.symbol("naml_encoding_binary_write_f64_le", crate::runtime::naml_encoding_binary_write_f64_le as *const u8);
        builder.symbol("naml_encoding_binary_alloc", crate::runtime::naml_encoding_binary_alloc as *const u8);
        builder.symbol("naml_encoding_binary_from_string", crate::runtime::naml_encoding_binary_from_string as *const u8);
        builder.symbol("naml_encoding_binary_len", crate::runtime::naml_encoding_binary_len as *const u8);
        builder.symbol("naml_encoding_binary_capacity", crate::runtime::naml_encoding_binary_capacity as *const u8);
        builder.symbol("naml_encoding_binary_slice", crate::runtime::naml_encoding_binary_slice as *const u8);
        builder.symbol("naml_encoding_binary_concat", crate::runtime::naml_encoding_binary_concat as *const u8);
        builder.symbol("naml_encoding_binary_append", crate::runtime::naml_encoding_binary_append as *const u8);
        builder.symbol("naml_encoding_binary_copy_within", crate::runtime::naml_encoding_binary_copy_within as *const u8);
        builder.symbol("naml_encoding_binary_clear", crate::runtime::naml_encoding_binary_clear as *const u8);
        builder.symbol("naml_encoding_binary_resize", crate::runtime::naml_encoding_binary_resize as *const u8);
        builder.symbol("naml_encoding_binary_fill", crate::runtime::naml_encoding_binary_fill as *const u8);
        builder.symbol("naml_encoding_binary_index_of", crate::runtime::naml_encoding_binary_index_of as *const u8);
        builder.symbol("naml_encoding_binary_contains", crate::runtime::naml_encoding_binary_contains as *const u8);
        builder.symbol("naml_encoding_binary_starts_with", crate::runtime::naml_encoding_binary_starts_with as *const u8);
        builder.symbol("naml_encoding_binary_ends_with", crate::runtime::naml_encoding_binary_ends_with as *const u8);
        builder.symbol("naml_encoding_binary_equals", crate::runtime::naml_encoding_binary_equals as *const u8);

        // JSON encoding operations
        builder.symbol(
            "naml_json_decode",
            crate::runtime::naml_json_decode as *const u8,
        );
        builder.symbol(
            "naml_json_encode",
            crate::runtime::naml_json_encode as *const u8,
        );
        builder.symbol(
            "naml_json_encode_pretty",
            crate::runtime::naml_json_encode_pretty as *const u8,
        );
        builder.symbol(
            "naml_json_exists",
            crate::runtime::naml_json_exists as *const u8,
        );
        builder.symbol(
            "naml_json_path",
            crate::runtime::naml_json_path as *const u8,
        );
        builder.symbol(
            "naml_json_keys",
            crate::runtime::naml_json_keys as *const u8,
        );
        builder.symbol(
            "naml_json_count",
            crate::runtime::naml_json_count as *const u8,
        );
        builder.symbol(
            "naml_json_get_type",
            crate::runtime::naml_json_get_type as *const u8,
        );
        builder.symbol(
            "naml_json_type_name",
            crate::runtime::naml_json_type_name as *const u8,
        );
        builder.symbol(
            "naml_json_is_null",
            crate::runtime::naml_json_is_null as *const u8,
        );
        builder.symbol(
            "naml_json_index_string",
            crate::runtime::naml_json_index_string as *const u8,
        );
        builder.symbol(
            "naml_json_index_int",
            crate::runtime::naml_json_index_int as *const u8,
        );
        builder.symbol(
            "naml_json_as_int",
            crate::runtime::naml_json_as_int as *const u8,
        );
        builder.symbol(
            "naml_json_as_float",
            crate::runtime::naml_json_as_float as *const u8,
        );
        builder.symbol(
            "naml_json_as_bool",
            crate::runtime::naml_json_as_bool as *const u8,
        );
        builder.symbol(
            "naml_json_as_string",
            crate::runtime::naml_json_as_string as *const u8,
        );
        builder.symbol(
            "naml_json_null",
            crate::runtime::naml_json_null as *const u8,
        );
        builder.symbol(
            "naml_path_error_new",
            crate::runtime::naml_path_error_new as *const u8,
        );

        // TOML encoding operations (from naml-std-encoding)
        builder.symbol(
            "naml_encoding_toml_decode",
            crate::runtime::naml_encoding_toml_decode as *const u8,
        );
        builder.symbol(
            "naml_encoding_toml_encode",
            crate::runtime::naml_encoding_toml_encode as *const u8,
        );
        builder.symbol(
            "naml_encoding_toml_encode_pretty",
            crate::runtime::naml_encoding_toml_encode_pretty as *const u8,
        );
        builder.symbol(
            "naml_encode_error_new",
            crate::runtime::naml_encode_error_new as *const u8,
        );

        // YAML encoding operations (from naml-std-encoding)
        builder.symbol(
            "naml_encoding_yaml_decode",
            crate::runtime::naml_encoding_yaml_decode as *const u8,
        );
        builder.symbol(
            "naml_encoding_yaml_encode",
            crate::runtime::naml_encoding_yaml_encode as *const u8,
        );

        // Networking operations (from naml-std-net)
        // Exception constructors
        builder.symbol(
            "naml_network_error_new",
            crate::runtime::naml_network_error_new as *const u8,
        );
        builder.symbol(
            "naml_timeout_error_new",
            crate::runtime::naml_timeout_error_new as *const u8,
        );
        builder.symbol(
            "naml_connection_refused_new",
            crate::runtime::naml_connection_refused_new as *const u8,
        );

        // TCP Server
        builder.symbol(
            "naml_net_tcp_server_listen",
            crate::runtime::naml_net_tcp_server_listen as *const u8,
        );
        builder.symbol(
            "naml_net_tcp_server_accept",
            crate::runtime::naml_net_tcp_server_accept as *const u8,
        );
        builder.symbol(
            "naml_net_tcp_server_close",
            crate::runtime::naml_net_tcp_server_close as *const u8,
        );
        builder.symbol(
            "naml_net_tcp_server_local_addr",
            crate::runtime::naml_net_tcp_server_local_addr as *const u8,
        );

        // TCP Client
        builder.symbol(
            "naml_net_tcp_client_connect",
            crate::runtime::naml_net_tcp_client_connect as *const u8,
        );
        builder.symbol(
            "naml_net_tcp_client_read",
            crate::runtime::naml_net_tcp_client_read as *const u8,
        );
        builder.symbol(
            "naml_net_tcp_client_read_all",
            crate::runtime::naml_net_tcp_client_read_all as *const u8,
        );
        builder.symbol(
            "naml_net_tcp_client_write",
            crate::runtime::naml_net_tcp_client_write as *const u8,
        );
        builder.symbol(
            "naml_net_tcp_client_close",
            crate::runtime::naml_net_tcp_client_close as *const u8,
        );
        builder.symbol(
            "naml_net_tcp_client_set_timeout",
            crate::runtime::naml_net_tcp_client_set_timeout as *const u8,
        );
        builder.symbol(
            "naml_net_tcp_socket_peer_addr",
            crate::runtime::naml_net_tcp_socket_peer_addr as *const u8,
        );

        // UDP
        builder.symbol(
            "naml_net_udp_bind",
            crate::runtime::naml_net_udp_bind as *const u8,
        );
        builder.symbol(
            "naml_net_udp_send",
            crate::runtime::naml_net_udp_send as *const u8,
        );
        builder.symbol(
            "naml_net_udp_receive",
            crate::runtime::naml_net_udp_receive as *const u8,
        );
        builder.symbol(
            "naml_net_udp_receive_from",
            crate::runtime::naml_net_udp_receive_from as *const u8,
        );
        builder.symbol(
            "naml_net_udp_close",
            crate::runtime::naml_net_udp_close as *const u8,
        );
        builder.symbol(
            "naml_net_udp_local_addr",
            crate::runtime::naml_net_udp_local_addr as *const u8,
        );

        // HTTP Client
        builder.symbol(
            "naml_net_http_client_get",
            crate::runtime::naml_net_http_client_get as *const u8,
        );
        builder.symbol(
            "naml_net_http_client_post",
            crate::runtime::naml_net_http_client_post as *const u8,
        );
        builder.symbol(
            "naml_net_http_client_put",
            crate::runtime::naml_net_http_client_put as *const u8,
        );
        builder.symbol(
            "naml_net_http_client_patch",
            crate::runtime::naml_net_http_client_patch as *const u8,
        );
        builder.symbol(
            "naml_net_http_client_delete",
            crate::runtime::naml_net_http_client_delete as *const u8,
        );
        builder.symbol(
            "naml_net_http_client_set_timeout",
            crate::runtime::naml_net_http_client_set_timeout as *const u8,
        );
        // HTTP Response accessors
        builder.symbol(
            "naml_net_http_response_get_status",
            crate::runtime::naml_net_http_response_get_status as *const u8,
        );
        builder.symbol(
            "naml_net_http_response_get_body_bytes",
            crate::runtime::naml_net_http_response_get_body_bytes as *const u8,
        );

        // HTTP Server
        builder.symbol(
            "naml_net_http_server_open_router",
            crate::runtime::naml_net_http_server_open_router as *const u8,
        );
        builder.symbol(
            "naml_net_http_server_get",
            crate::runtime::naml_net_http_server_get as *const u8,
        );
        builder.symbol(
            "naml_net_http_server_post",
            crate::runtime::naml_net_http_server_post as *const u8,
        );
        builder.symbol(
            "naml_net_http_server_put",
            crate::runtime::naml_net_http_server_put as *const u8,
        );
        builder.symbol(
            "naml_net_http_server_patch",
            crate::runtime::naml_net_http_server_patch as *const u8,
        );
        builder.symbol(
            "naml_net_http_server_delete",
            crate::runtime::naml_net_http_server_delete as *const u8,
        );
        builder.symbol(
            "naml_net_http_server_with",
            crate::runtime::naml_net_http_server_with as *const u8,
        );
        builder.symbol(
            "naml_net_http_server_group",
            crate::runtime::naml_net_http_server_group as *const u8,
        );
        builder.symbol(
            "naml_net_http_server_mount",
            crate::runtime::naml_net_http_server_mount as *const u8,
        );
        builder.symbol(
            "naml_net_http_server_serve",
            crate::runtime::naml_net_http_server_serve as *const u8,
        );
        builder.symbol(
            "naml_net_http_server_text_response",
            crate::runtime::naml_net_http_server_text_response as *const u8,
        );

        // HTTP Middleware
        builder.symbol(
            "naml_net_http_middleware_logger",
            crate::runtime::naml_net_http_middleware_logger as *const u8,
        );
        builder.symbol(
            "naml_net_http_middleware_timeout",
            crate::runtime::naml_net_http_middleware_timeout as *const u8,
        );
        builder.symbol(
            "naml_net_http_middleware_recover",
            crate::runtime::naml_net_http_middleware_recover as *const u8,
        );
        builder.symbol(
            "naml_net_http_middleware_cors",
            crate::runtime::naml_net_http_middleware_cors as *const u8,
        );
        builder.symbol(
            "naml_net_http_middleware_rate_limit",
            crate::runtime::naml_net_http_middleware_rate_limit as *const u8,
        );
        builder.symbol(
            "naml_net_http_middleware_compress",
            crate::runtime::naml_net_http_middleware_compress as *const u8,
        );
        builder.symbol(
            "naml_net_http_middleware_request_id",
            crate::runtime::naml_net_http_middleware_request_id as *const u8,
        );

        // TLS Client
        builder.symbol(
            "naml_net_tls_client_connect",
            crate::runtime::naml_net_tls_client_connect as *const u8,
        );
        builder.symbol(
            "naml_net_tls_client_read",
            crate::runtime::naml_net_tls_client_read as *const u8,
        );
        builder.symbol(
            "naml_net_tls_client_read_all",
            crate::runtime::naml_net_tls_client_read_all as *const u8,
        );
        builder.symbol(
            "naml_net_tls_client_write",
            crate::runtime::naml_net_tls_client_write as *const u8,
        );
        builder.symbol(
            "naml_net_tls_client_close",
            crate::runtime::naml_net_tls_client_close as *const u8,
        );
        builder.symbol(
            "naml_net_tls_client_set_timeout",
            crate::runtime::naml_net_tls_client_set_timeout as *const u8,
        );
        builder.symbol(
            "naml_net_tls_client_peer_addr",
            crate::runtime::naml_net_tls_client_peer_addr as *const u8,
        );

        // TLS Server
        builder.symbol(
            "naml_net_tls_server_wrap_listener",
            crate::runtime::naml_net_tls_server_wrap_listener as *const u8,
        );
        builder.symbol(
            "naml_net_tls_server_accept",
            crate::runtime::naml_net_tls_server_accept as *const u8,
        );
        builder.symbol(
            "naml_net_tls_server_close_listener",
            crate::runtime::naml_net_tls_server_close_listener as *const u8,
        );

        // HTTP over TLS
        builder.symbol(
            "naml_net_http_server_serve_tls",
            crate::runtime::naml_net_http_server_serve_tls as *const u8,
        );
        builder.symbol(
            "naml_net_http_client_get_tls",
            crate::runtime::naml_net_http_client_get_tls as *const u8,
        );

        builder.symbol("naml_db_sqlite_error_new", crate::runtime::naml_db_sqlite_error_new as *const u8);
        builder.symbol("naml_db_sqlite_open", crate::runtime::naml_db_sqlite_open as *const u8);
        builder.symbol("naml_db_sqlite_open_memory", crate::runtime::naml_db_sqlite_open_memory as *const u8);
        builder.symbol("naml_db_sqlite_close", crate::runtime::naml_db_sqlite_close as *const u8);
        builder.symbol("naml_db_sqlite_exec", crate::runtime::naml_db_sqlite_exec as *const u8);
        builder.symbol("naml_db_sqlite_query", crate::runtime::naml_db_sqlite_query as *const u8);
        builder.symbol("naml_db_sqlite_row_count", crate::runtime::naml_db_sqlite_row_count as *const u8);
        builder.symbol("naml_db_sqlite_row_at", crate::runtime::naml_db_sqlite_row_at as *const u8);
        builder.symbol("naml_db_sqlite_get_string", crate::runtime::naml_db_sqlite_get_string as *const u8);
        builder.symbol("naml_db_sqlite_get_int", crate::runtime::naml_db_sqlite_get_int as *const u8);
        builder.symbol("naml_db_sqlite_get_float", crate::runtime::naml_db_sqlite_get_float as *const u8);
        builder.symbol("naml_db_sqlite_get_bool", crate::runtime::naml_db_sqlite_get_bool as *const u8);
        builder.symbol("naml_db_sqlite_is_null", crate::runtime::naml_db_sqlite_is_null as *const u8);
        builder.symbol("naml_db_sqlite_columns", crate::runtime::naml_db_sqlite_columns as *const u8);
        builder.symbol("naml_db_sqlite_column_count", crate::runtime::naml_db_sqlite_column_count as *const u8);
        builder.symbol("naml_db_sqlite_begin", crate::runtime::naml_db_sqlite_begin as *const u8);
        builder.symbol("naml_db_sqlite_commit", crate::runtime::naml_db_sqlite_commit as *const u8);
        builder.symbol("naml_db_sqlite_rollback", crate::runtime::naml_db_sqlite_rollback as *const u8);
        builder.symbol("naml_db_sqlite_prepare", crate::runtime::naml_db_sqlite_prepare as *const u8);
        builder.symbol("naml_db_sqlite_bind_string", crate::runtime::naml_db_sqlite_bind_string as *const u8);
        builder.symbol("naml_db_sqlite_bind_int", crate::runtime::naml_db_sqlite_bind_int as *const u8);
        builder.symbol("naml_db_sqlite_bind_float", crate::runtime::naml_db_sqlite_bind_float as *const u8);
        builder.symbol("naml_db_sqlite_step", crate::runtime::naml_db_sqlite_step as *const u8);
        builder.symbol("naml_db_sqlite_step_query", crate::runtime::naml_db_sqlite_step_query as *const u8);
        builder.symbol("naml_db_sqlite_reset", crate::runtime::naml_db_sqlite_reset as *const u8);
        builder.symbol("naml_db_sqlite_finalize", crate::runtime::naml_db_sqlite_finalize as *const u8);
        builder.symbol("naml_db_sqlite_changes", crate::runtime::naml_db_sqlite_changes as *const u8);
        builder.symbol("naml_db_sqlite_last_insert_id", crate::runtime::naml_db_sqlite_last_insert_id as *const u8);

        let module = BackendModule::Jit(JITModule::new(builder));
        Self::build_compiler(interner, annotations, source_info, module, release, unsafe_mode)
    }

    pub fn new_aot(
        interner: &'a Rodeo,
        annotations: &'a TypeAnnotations,
        source_info: &'a crate::source::SourceFile,
        release: bool,
        unsafe_mode: bool,
    ) -> Result<Self, CodegenError> {
        let isa = create_isa(true, release)?;
        let obj_builder = ObjectBuilder::new(
            isa,
            "naml_output",
            cranelift_module::default_libcall_names(),
        )
        .map_err(|e| CodegenError::JitCompile(format!("Failed to create ObjectBuilder: {}", e)))?;
        let module = BackendModule::Object(ObjectModule::new(obj_builder));
        Self::build_compiler(interner, annotations, source_info, module, release, unsafe_mode)
    }
}
