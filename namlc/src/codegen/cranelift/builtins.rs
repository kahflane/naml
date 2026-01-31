///
/// Built-in Function Registry
///
/// This module defines a registry for built-in standard library functions
/// and their compilation strategies. New built-in functions can be added
/// by extending the registry without modifying the core codegen logic.
///
/// Supported Modules:
/// - collections: Array operations (count, push, pop, first, last, etc.)
/// - io: Terminal I/O (read_key, clear_screen, set_cursor, etc.)
/// - random: Random number generation (random, random_float)
/// - datetime: Date/time utilities (now_ms, year, month, format_date, etc.)
/// - metrics: Performance measurement (perf_now, elapsed_ms/us/ns)
/// - strings: String operations (len, upper, lower, split, etc.)
/// - threads: Concurrency (open_channel, send, receive, close, join)
///

use cranelift::prelude::*;
use cranelift_codegen::ir::MemFlags;

use super::array::{
    call_array_clear_runtime, call_array_contains_bool, call_array_fill_runtime, call_array_push,
};
use super::misc::{
    call_int_runtime, call_one_arg_int_runtime, call_one_arg_ptr_runtime,
    call_three_arg_ptr_runtime, call_three_arg_void_runtime, call_two_arg_bool_runtime,
    call_two_arg_int_runtime, call_two_arg_ptr_runtime, call_two_arg_runtime, call_void_runtime,
};
use super::options::{
    compile_option_from_array_access, compile_option_from_array_get, compile_option_from_index_of,
    compile_option_from_last_index_of, compile_option_from_minmax, compile_option_from_remove_at,
};
use super::{CompileContext, ARRAY_LEN_OFFSET};
use crate::ast::Expression;
use crate::codegen::CodegenError;

/// Describes how to compile a built-in function
#[derive(Debug, Clone, Copy)]
pub enum BuiltinStrategy {
    /// Load array length from header offset
    ArrayLength,
    /// Call array push helper (arr, val) -> unit
    ArrayPush,
    /// One arg -> option via array_access pattern
    OneArgOptionAccess(&'static str),
    /// One arg -> int return
    OneArgInt(&'static str),
    /// One arg -> ptr return
    OneArgPtr(&'static str),
    /// Two args -> ptr return
    TwoArgPtr(&'static str),
    /// Three args -> ptr return
    ThreeArgPtr(&'static str),
    /// Array get (arr, index) -> option
    ArrayGet,
    /// Array fill (arr, val) -> unit
    ArrayFill,
    /// Array clear (arr) -> unit
    ArrayClear,
    /// Array min/max with is_min flag
    ArrayMinMax(&'static str, bool),
    /// Array index_of (arr, val) -> option<int>
    ArrayIndexOf,
    /// Array contains (arr, val) -> bool
    ArrayContains,
    /// Three args -> void (insert, swap)
    ThreeArgVoid(&'static str),
    /// Two args -> option<int> (remove_at with index check)
    TwoArgOptionInt(&'static str),
    /// Two args -> bool (remove returning success)
    TwoArgBool(&'static str),
    /// Two args -> option<int> using index_of pattern
    ArrayLastIndexOf,

    // === IO Module ===
    /// No args -> int return (read_key, terminal_width, etc.)
    NoArgInt(&'static str),
    /// No args -> void (clear_screen, hide_cursor, show_cursor)
    NoArgVoid(&'static str),
    /// Two args -> void (set_cursor)
    TwoArgVoid(&'static str),

    // === Random Module ===
    /// (min, max) -> int
    RandomInt,
    /// () -> float
    RandomFloat,

    // === Datetime Module ===
    /// One arg int -> int (year, month, day, etc.)
    DatetimeOneArgInt(&'static str),
    /// (timestamp, fmt) -> string
    DatetimeFormat,

    // === Strings Module ===
    /// One arg string -> int (len/char_len)
    StringOneArgInt(&'static str),
    /// One arg string -> ptr (upper, lower, ltrim, rtrim)
    StringOneArgPtr(&'static str),
    /// (string, string) -> bool (has/contains, starts_with, ends_with)
    StringTwoArgBool(&'static str),
    /// (string, int) -> int (char_at)
    StringArgIntInt(&'static str),
    /// (string, string) -> ptr (split returns array)
    StringTwoArgPtr(&'static str),
    /// (string, string, string) -> ptr (replace, replace_all)
    StringThreeArgPtr(&'static str),
    /// (string, int, int) -> ptr (substr)
    StringArgIntIntPtr(&'static str),
    /// (string, int, string) -> ptr (lpad, rpad)
    StringArgIntStrPtr(&'static str),
    /// (string, int) -> ptr (repeat)
    StringArgIntPtr(&'static str),
    /// (array<string>, string) -> string (concat/join)
    StringJoin,

    // === Threads/Channel Module ===
    /// No args -> void (join/wait_all)
    ThreadsJoin,
    /// (capacity) -> channel
    ChannelOpen,
    /// (channel, value) -> int
    ChannelSend,
    /// (channel) -> option<T>
    ChannelReceive,
    /// (channel) -> void
    ChannelClose,

    // ========================================
    // Lambda-based collection strategies
    // ========================================
    /// (arr, closure) -> bool (any, all)
    LambdaBool(&'static str),
    /// (arr, closure) -> int (count_if)
    LambdaInt(&'static str),
    /// (arr, closure) -> array (apply/map, where/filter, partition, take_while, drop_while, reject, flat_apply)
    LambdaArray(&'static str),
    /// (arr, closure) -> option<T> (find)
    LambdaFind,
    /// (arr, closure) -> option<int> (find_index)
    LambdaFindIndex,
    /// (arr, closure) -> option<T> (find_last)
    LambdaFindLast,
    /// (arr, closure) -> option<int> (find_last_index)
    LambdaFindLastIndex,
    /// (arr, initial, closure) -> T (fold)
    LambdaFold,
    /// (arr, initial, closure) -> array (scan)
    LambdaScan,
    /// (arr, closure) -> array (sort_by)
    LambdaSortBy,
    /// (arr) -> option<T> (sample - random element)
    Sample,

    // ========================================
    // Core I/O strategies (varargs/special handling)
    // ========================================
    /// Varargs print with newline flag
    Print(bool),
    /// Sleep with milliseconds validation
    Sleep,
    /// Stderr output (warn, error, panic)
    Stderr(&'static str),
    /// Format string with varargs
    Fmt,
    /// Read line from stdin
    ReadLine,
}

/// Registry entry for a built-in function
pub struct BuiltinFunction {
    pub name: &'static str,
    pub strategy: BuiltinStrategy,
}

/// Get the built-in function registry
/// Add new built-in functions here
pub fn get_builtin_registry() -> &'static [BuiltinFunction] {
    static REGISTRY: &[BuiltinFunction] = &[
        // ========================================
        // Collections module - array operations
        // ========================================
        BuiltinFunction { name: "count", strategy: BuiltinStrategy::ArrayLength },
        BuiltinFunction { name: "push", strategy: BuiltinStrategy::ArrayPush },
        BuiltinFunction { name: "pop", strategy: BuiltinStrategy::OneArgOptionAccess("naml_array_pop") },
        BuiltinFunction { name: "shift", strategy: BuiltinStrategy::OneArgOptionAccess("naml_array_shift") },
        BuiltinFunction { name: "first", strategy: BuiltinStrategy::OneArgOptionAccess("naml_array_first") },
        BuiltinFunction { name: "last", strategy: BuiltinStrategy::OneArgOptionAccess("naml_array_last") },
        BuiltinFunction { name: "fill", strategy: BuiltinStrategy::ArrayFill },
        BuiltinFunction { name: "clear", strategy: BuiltinStrategy::ArrayClear },
        BuiltinFunction { name: "get", strategy: BuiltinStrategy::ArrayGet },
        BuiltinFunction { name: "sum", strategy: BuiltinStrategy::OneArgInt("naml_array_sum") },
        BuiltinFunction { name: "min", strategy: BuiltinStrategy::ArrayMinMax("naml_array_min", true) },
        BuiltinFunction { name: "max", strategy: BuiltinStrategy::ArrayMinMax("naml_array_max", false) },
        BuiltinFunction { name: "reversed", strategy: BuiltinStrategy::OneArgPtr("naml_array_reversed") },
        BuiltinFunction { name: "sort", strategy: BuiltinStrategy::OneArgPtr("naml_array_sort") },
        BuiltinFunction { name: "flatten", strategy: BuiltinStrategy::OneArgPtr("naml_array_flatten") },
        BuiltinFunction { name: "take", strategy: BuiltinStrategy::TwoArgPtr("naml_array_take") },
        BuiltinFunction { name: "drop", strategy: BuiltinStrategy::TwoArgPtr("naml_array_drop") },
        BuiltinFunction { name: "slice", strategy: BuiltinStrategy::ThreeArgPtr("naml_array_slice") },
        BuiltinFunction { name: "index_of", strategy: BuiltinStrategy::ArrayIndexOf },
        BuiltinFunction { name: "contains", strategy: BuiltinStrategy::ArrayContains },
        // Mutation operations
        BuiltinFunction { name: "insert", strategy: BuiltinStrategy::ThreeArgVoid("naml_array_insert") },
        BuiltinFunction { name: "remove_at", strategy: BuiltinStrategy::TwoArgOptionInt("naml_array_remove_at") },
        BuiltinFunction { name: "remove", strategy: BuiltinStrategy::TwoArgBool("naml_array_remove") },
        BuiltinFunction { name: "swap", strategy: BuiltinStrategy::ThreeArgVoid("naml_array_swap") },
        // Deduplication
        BuiltinFunction { name: "unique", strategy: BuiltinStrategy::OneArgPtr("naml_array_unique") },
        BuiltinFunction { name: "compact", strategy: BuiltinStrategy::OneArgPtr("naml_array_compact") },
        // Backward search
        BuiltinFunction { name: "last_index_of", strategy: BuiltinStrategy::ArrayLastIndexOf },
        // Array combination
        BuiltinFunction { name: "zip", strategy: BuiltinStrategy::TwoArgPtr("naml_array_zip") },
        BuiltinFunction { name: "unzip", strategy: BuiltinStrategy::OneArgPtr("naml_array_unzip") },
        // Splitting
        BuiltinFunction { name: "chunk", strategy: BuiltinStrategy::TwoArgPtr("naml_array_chunk") },
        // Set operations
        BuiltinFunction { name: "intersect", strategy: BuiltinStrategy::TwoArgPtr("naml_array_intersect") },
        BuiltinFunction { name: "diff", strategy: BuiltinStrategy::TwoArgPtr("naml_array_diff") },
        BuiltinFunction { name: "union", strategy: BuiltinStrategy::TwoArgPtr("naml_array_union") },
        // Random
        BuiltinFunction { name: "shuffle", strategy: BuiltinStrategy::OneArgPtr("naml_array_shuffle") },
        BuiltinFunction { name: "sample_n", strategy: BuiltinStrategy::TwoArgPtr("naml_array_sample_n") },
        BuiltinFunction { name: "sample", strategy: BuiltinStrategy::Sample },
        // Lambda-based collection functions
        BuiltinFunction { name: "any", strategy: BuiltinStrategy::LambdaBool("naml_array_any") },
        BuiltinFunction { name: "all", strategy: BuiltinStrategy::LambdaBool("naml_array_all") },
        BuiltinFunction { name: "count_if", strategy: BuiltinStrategy::LambdaInt("naml_array_count_if") },
        BuiltinFunction { name: "apply", strategy: BuiltinStrategy::LambdaArray("naml_array_map") },
        BuiltinFunction { name: "where", strategy: BuiltinStrategy::LambdaArray("naml_array_filter") },
        BuiltinFunction { name: "partition", strategy: BuiltinStrategy::LambdaArray("naml_array_partition") },
        BuiltinFunction { name: "take_while", strategy: BuiltinStrategy::LambdaArray("naml_array_take_while") },
        BuiltinFunction { name: "drop_while", strategy: BuiltinStrategy::LambdaArray("naml_array_drop_while") },
        BuiltinFunction { name: "reject", strategy: BuiltinStrategy::LambdaArray("naml_array_reject") },
        BuiltinFunction { name: "flat_apply", strategy: BuiltinStrategy::LambdaArray("naml_array_flat_apply") },
        BuiltinFunction { name: "find", strategy: BuiltinStrategy::LambdaFind },
        BuiltinFunction { name: "find_index", strategy: BuiltinStrategy::LambdaFindIndex },
        BuiltinFunction { name: "find_last", strategy: BuiltinStrategy::LambdaFindLast },
        BuiltinFunction { name: "find_last_index", strategy: BuiltinStrategy::LambdaFindLastIndex },
        BuiltinFunction { name: "fold", strategy: BuiltinStrategy::LambdaFold },
        BuiltinFunction { name: "scan", strategy: BuiltinStrategy::LambdaScan },
        BuiltinFunction { name: "sort_by", strategy: BuiltinStrategy::LambdaSortBy },

        // ========================================
        // IO module - core I/O operations
        // ========================================
        BuiltinFunction { name: "print", strategy: BuiltinStrategy::Print(false) },
        BuiltinFunction { name: "println", strategy: BuiltinStrategy::Print(true) },
        BuiltinFunction { name: "read_line", strategy: BuiltinStrategy::ReadLine },
        BuiltinFunction { name: "fmt", strategy: BuiltinStrategy::Fmt },
        BuiltinFunction { name: "warn", strategy: BuiltinStrategy::Stderr("warn") },
        BuiltinFunction { name: "error", strategy: BuiltinStrategy::Stderr("error") },
        BuiltinFunction { name: "panic", strategy: BuiltinStrategy::Stderr("panic") },

        // ========================================
        // IO module - terminal operations
        // ========================================
        BuiltinFunction { name: "read_key", strategy: BuiltinStrategy::NoArgInt("naml_read_key") },
        BuiltinFunction { name: "clear_screen", strategy: BuiltinStrategy::NoArgVoid("naml_clear_screen") },
        BuiltinFunction { name: "set_cursor", strategy: BuiltinStrategy::TwoArgVoid("naml_set_cursor") },
        BuiltinFunction { name: "hide_cursor", strategy: BuiltinStrategy::NoArgVoid("naml_hide_cursor") },
        BuiltinFunction { name: "show_cursor", strategy: BuiltinStrategy::NoArgVoid("naml_show_cursor") },
        BuiltinFunction { name: "terminal_width", strategy: BuiltinStrategy::NoArgInt("naml_terminal_width") },
        BuiltinFunction { name: "terminal_height", strategy: BuiltinStrategy::NoArgInt("naml_terminal_height") },

        // ========================================
        // Random module
        // ========================================
        BuiltinFunction { name: "random", strategy: BuiltinStrategy::RandomInt },
        BuiltinFunction { name: "random_float", strategy: BuiltinStrategy::RandomFloat },

        // ========================================
        // Datetime module
        // ========================================
        BuiltinFunction { name: "now_ms", strategy: BuiltinStrategy::NoArgInt("naml_datetime_now_ms") },
        BuiltinFunction { name: "now_s", strategy: BuiltinStrategy::NoArgInt("naml_datetime_now_s") },
        BuiltinFunction { name: "year", strategy: BuiltinStrategy::DatetimeOneArgInt("naml_datetime_year") },
        BuiltinFunction { name: "month", strategy: BuiltinStrategy::DatetimeOneArgInt("naml_datetime_month") },
        BuiltinFunction { name: "day", strategy: BuiltinStrategy::DatetimeOneArgInt("naml_datetime_day") },
        BuiltinFunction { name: "hour", strategy: BuiltinStrategy::DatetimeOneArgInt("naml_datetime_hour") },
        BuiltinFunction { name: "minute", strategy: BuiltinStrategy::DatetimeOneArgInt("naml_datetime_minute") },
        BuiltinFunction { name: "second", strategy: BuiltinStrategy::DatetimeOneArgInt("naml_datetime_second") },
        BuiltinFunction { name: "day_of_week", strategy: BuiltinStrategy::DatetimeOneArgInt("naml_datetime_day_of_week") },
        BuiltinFunction { name: "format_date", strategy: BuiltinStrategy::DatetimeFormat },

        // ========================================
        // Metrics module
        // ========================================
        BuiltinFunction { name: "perf_now", strategy: BuiltinStrategy::NoArgInt("naml_metrics_perf_now") },
        BuiltinFunction { name: "elapsed_ms", strategy: BuiltinStrategy::OneArgInt("naml_metrics_elapsed_ms") },
        BuiltinFunction { name: "elapsed_us", strategy: BuiltinStrategy::OneArgInt("naml_metrics_elapsed_us") },
        BuiltinFunction { name: "elapsed_ns", strategy: BuiltinStrategy::OneArgInt("naml_metrics_elapsed_ns") },

        // ========================================
        // Strings module
        // ========================================
        BuiltinFunction { name: "len", strategy: BuiltinStrategy::StringOneArgInt("naml_string_char_len") },
        BuiltinFunction { name: "char_at", strategy: BuiltinStrategy::StringArgIntInt("naml_string_char_at") },
        BuiltinFunction { name: "upper", strategy: BuiltinStrategy::StringOneArgPtr("naml_string_upper") },
        BuiltinFunction { name: "lower", strategy: BuiltinStrategy::StringOneArgPtr("naml_string_lower") },
        BuiltinFunction { name: "split", strategy: BuiltinStrategy::StringTwoArgPtr("naml_string_split") },
        BuiltinFunction { name: "concat", strategy: BuiltinStrategy::StringJoin },
        BuiltinFunction { name: "has", strategy: BuiltinStrategy::StringTwoArgBool("naml_string_contains") },
        BuiltinFunction { name: "starts_with", strategy: BuiltinStrategy::StringTwoArgBool("naml_string_starts_with") },
        BuiltinFunction { name: "ends_with", strategy: BuiltinStrategy::StringTwoArgBool("naml_string_ends_with") },
        BuiltinFunction { name: "replace", strategy: BuiltinStrategy::StringThreeArgPtr("naml_string_replace") },
        BuiltinFunction { name: "replace_all", strategy: BuiltinStrategy::StringThreeArgPtr("naml_string_replace_all") },
        BuiltinFunction { name: "ltrim", strategy: BuiltinStrategy::StringOneArgPtr("naml_string_ltrim") },
        BuiltinFunction { name: "rtrim", strategy: BuiltinStrategy::StringOneArgPtr("naml_string_rtrim") },
        BuiltinFunction { name: "substr", strategy: BuiltinStrategy::StringArgIntIntPtr("naml_string_substr") },
        BuiltinFunction { name: "lpad", strategy: BuiltinStrategy::StringArgIntStrPtr("naml_string_lpad") },
        BuiltinFunction { name: "rpad", strategy: BuiltinStrategy::StringArgIntStrPtr("naml_string_rpad") },
        BuiltinFunction { name: "repeat", strategy: BuiltinStrategy::StringArgIntPtr("naml_string_repeat") },
        BuiltinFunction { name: "lines", strategy: BuiltinStrategy::StringOneArgPtr("naml_string_lines") },
        BuiltinFunction { name: "chars", strategy: BuiltinStrategy::StringOneArgPtr("naml_string_chars") },

        // ========================================
        // Threads/Channel module
        // ========================================
        BuiltinFunction { name: "sleep", strategy: BuiltinStrategy::Sleep },
        BuiltinFunction { name: "join", strategy: BuiltinStrategy::ThreadsJoin },
        BuiltinFunction { name: "open_channel", strategy: BuiltinStrategy::ChannelOpen },
        BuiltinFunction { name: "send", strategy: BuiltinStrategy::ChannelSend },
        BuiltinFunction { name: "receive", strategy: BuiltinStrategy::ChannelReceive },
        BuiltinFunction { name: "close", strategy: BuiltinStrategy::ChannelClose },
    ];
    REGISTRY
}

/// Look up a built-in function by name
pub fn lookup_builtin(name: &str) -> Option<&'static BuiltinFunction> {
    get_builtin_registry().iter().find(|f| f.name == name)
}

/// Compile a built-in function call using the registry
pub fn compile_builtin_call(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    builtin: &BuiltinFunction,
    args: &[Expression<'_>],
) -> Result<Value, CodegenError> {
    use super::expr::compile_expression;
    use super::misc::{call_random, call_random_float, call_datetime_format, call_sleep};
    use super::runtime::rt_func_ref;
    use super::channels::{
        call_channel_new, call_channel_send, call_channel_receive, call_channel_close,
    };
    use super::strings::ensure_naml_string;
    use super::lambda::{
        compile_lambda_bool_collection, compile_lambda_int_collection,
        compile_lambda_array_collection, compile_lambda_find, compile_lambda_find_index,
        compile_lambda_find_last, compile_lambda_find_last_index, compile_lambda_fold,
        compile_lambda_scan, compile_lambda_sort_by, compile_sample,
    };
    use super::print::compile_print_call;
    use super::io::{call_read_line, compile_fmt_call, compile_stderr_call};

    match builtin.strategy {
        // ========================================
        // Collections strategies
        // ========================================
        BuiltinStrategy::ArrayLength => {
            let arr = compile_expression(ctx, builder, &args[0])?;
            let len = builder.ins().load(
                types::I64,
                MemFlags::trusted(),
                arr,
                ARRAY_LEN_OFFSET,
            );
            Ok(len)
        }

        BuiltinStrategy::ArrayPush => {
            let arr = compile_expression(ctx, builder, &args[0])?;
            let val = compile_expression(ctx, builder, &args[1])?;
            call_array_push(ctx, builder, arr, val)?;
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::OneArgOptionAccess(runtime_fn) => {
            let arr = compile_expression(ctx, builder, &args[0])?;
            compile_option_from_array_access(ctx, builder, arr, runtime_fn)
        }

        BuiltinStrategy::OneArgInt(runtime_fn) => {
            let arr = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, runtime_fn, arr)
        }

        BuiltinStrategy::OneArgPtr(runtime_fn) => {
            let arr = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, runtime_fn, arr)
        }

        BuiltinStrategy::TwoArgPtr(runtime_fn) => {
            let arg0 = compile_expression(ctx, builder, &args[0])?;
            let arg1 = compile_expression(ctx, builder, &args[1])?;
            call_two_arg_ptr_runtime(ctx, builder, runtime_fn, arg0, arg1)
        }

        BuiltinStrategy::ThreeArgPtr(runtime_fn) => {
            let arg0 = compile_expression(ctx, builder, &args[0])?;
            let arg1 = compile_expression(ctx, builder, &args[1])?;
            let arg2 = compile_expression(ctx, builder, &args[2])?;
            call_three_arg_ptr_runtime(ctx, builder, runtime_fn, arg0, arg1, arg2)
        }

        BuiltinStrategy::ArrayGet => {
            let arr = compile_expression(ctx, builder, &args[0])?;
            let index = compile_expression(ctx, builder, &args[1])?;
            compile_option_from_array_get(ctx, builder, arr, index)
        }

        BuiltinStrategy::ArrayFill => {
            let arr = compile_expression(ctx, builder, &args[0])?;
            let val = compile_expression(ctx, builder, &args[1])?;
            call_array_fill_runtime(ctx, builder, arr, val)
        }

        BuiltinStrategy::ArrayClear => {
            let arr = compile_expression(ctx, builder, &args[0])?;
            call_array_clear_runtime(ctx, builder, arr)
        }

        BuiltinStrategy::ArrayMinMax(runtime_fn, is_min) => {
            let arr = compile_expression(ctx, builder, &args[0])?;
            compile_option_from_minmax(ctx, builder, arr, runtime_fn, is_min)
        }

        BuiltinStrategy::ArrayIndexOf => {
            let arr = compile_expression(ctx, builder, &args[0])?;
            let val = compile_expression(ctx, builder, &args[1])?;
            compile_option_from_index_of(ctx, builder, arr, val)
        }

        BuiltinStrategy::ArrayContains => {
            let arr = compile_expression(ctx, builder, &args[0])?;
            let val = compile_expression(ctx, builder, &args[1])?;
            call_array_contains_bool(ctx, builder, arr, val)
        }

        BuiltinStrategy::ThreeArgVoid(runtime_fn) => {
            let arg0 = compile_expression(ctx, builder, &args[0])?;
            let arg1 = compile_expression(ctx, builder, &args[1])?;
            let arg2 = compile_expression(ctx, builder, &args[2])?;
            call_three_arg_void_runtime(ctx, builder, runtime_fn, arg0, arg1, arg2)
        }

        BuiltinStrategy::TwoArgOptionInt(runtime_fn) => {
            let arr = compile_expression(ctx, builder, &args[0])?;
            let index = compile_expression(ctx, builder, &args[1])?;
            compile_option_from_remove_at(ctx, builder, arr, index, runtime_fn)
        }

        BuiltinStrategy::TwoArgBool(runtime_fn) => {
            let arr = compile_expression(ctx, builder, &args[0])?;
            let val = compile_expression(ctx, builder, &args[1])?;
            call_two_arg_bool_runtime(ctx, builder, runtime_fn, arr, val)
        }

        BuiltinStrategy::ArrayLastIndexOf => {
            let arr = compile_expression(ctx, builder, &args[0])?;
            let val = compile_expression(ctx, builder, &args[1])?;
            compile_option_from_last_index_of(ctx, builder, arr, val)
        }

        // ========================================
        // IO strategies
        // ========================================
        BuiltinStrategy::NoArgInt(runtime_fn) => {
            call_int_runtime(ctx, builder, runtime_fn)
        }

        BuiltinStrategy::NoArgVoid(runtime_fn) => {
            call_void_runtime(ctx, builder, runtime_fn)
        }

        BuiltinStrategy::TwoArgVoid(runtime_fn) => {
            let arg0 = compile_expression(ctx, builder, &args[0])?;
            let arg1 = compile_expression(ctx, builder, &args[1])?;
            call_two_arg_runtime(ctx, builder, runtime_fn, arg0, arg1)
        }

        // ========================================
        // Random strategies
        // ========================================
        BuiltinStrategy::RandomInt => {
            let min = compile_expression(ctx, builder, &args[0])?;
            let max = compile_expression(ctx, builder, &args[1])?;
            call_random(ctx, builder, min, max)
        }

        BuiltinStrategy::RandomFloat => {
            call_random_float(ctx, builder)
        }

        // ========================================
        // Datetime strategies
        // ========================================
        BuiltinStrategy::DatetimeOneArgInt(runtime_fn) => {
            let timestamp = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, runtime_fn, timestamp)
        }

        BuiltinStrategy::DatetimeFormat => {
            let timestamp = compile_expression(ctx, builder, &args[0])?;
            let fmt = compile_expression(ctx, builder, &args[1])?;
            call_datetime_format(ctx, builder, timestamp, fmt)
        }

        // ========================================
        // Strings strategies
        // ========================================
        BuiltinStrategy::StringOneArgInt(runtime_fn) => {
            let s = compile_expression(ctx, builder, &args[0])?;
            let s = ensure_naml_string(ctx, builder, s, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, runtime_fn, s)
        }

        BuiltinStrategy::StringOneArgPtr(runtime_fn) => {
            let s = compile_expression(ctx, builder, &args[0])?;
            let s = ensure_naml_string(ctx, builder, s, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, runtime_fn, s)
        }

        BuiltinStrategy::StringTwoArgBool(runtime_fn) => {
            let s = compile_expression(ctx, builder, &args[0])?;
            let s = ensure_naml_string(ctx, builder, s, &args[0])?;
            let sub = compile_expression(ctx, builder, &args[1])?;
            let sub = ensure_naml_string(ctx, builder, sub, &args[1])?;
            call_two_arg_bool_runtime(ctx, builder, runtime_fn, s, sub)
        }

        BuiltinStrategy::StringArgIntInt(runtime_fn) => {
            let s = compile_expression(ctx, builder, &args[0])?;
            let s = ensure_naml_string(ctx, builder, s, &args[0])?;
            let idx = compile_expression(ctx, builder, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, runtime_fn, s, idx)
        }

        BuiltinStrategy::StringTwoArgPtr(runtime_fn) => {
            let s = compile_expression(ctx, builder, &args[0])?;
            let s = ensure_naml_string(ctx, builder, s, &args[0])?;
            let delim = compile_expression(ctx, builder, &args[1])?;
            let delim = ensure_naml_string(ctx, builder, delim, &args[1])?;
            call_two_arg_ptr_runtime(ctx, builder, runtime_fn, s, delim)
        }

        BuiltinStrategy::StringThreeArgPtr(runtime_fn) => {
            let s = compile_expression(ctx, builder, &args[0])?;
            let s = ensure_naml_string(ctx, builder, s, &args[0])?;
            let old = compile_expression(ctx, builder, &args[1])?;
            let old = ensure_naml_string(ctx, builder, old, &args[1])?;
            let new = compile_expression(ctx, builder, &args[2])?;
            let new = ensure_naml_string(ctx, builder, new, &args[2])?;
            call_three_arg_ptr_runtime(ctx, builder, runtime_fn, s, old, new)
        }

        BuiltinStrategy::StringArgIntIntPtr(runtime_fn) => {
            let s = compile_expression(ctx, builder, &args[0])?;
            let s = ensure_naml_string(ctx, builder, s, &args[0])?;
            let start = compile_expression(ctx, builder, &args[1])?;
            let end = compile_expression(ctx, builder, &args[2])?;
            call_three_arg_ptr_runtime(ctx, builder, runtime_fn, s, start, end)
        }

        BuiltinStrategy::StringArgIntStrPtr(runtime_fn) => {
            let s = compile_expression(ctx, builder, &args[0])?;
            let s = ensure_naml_string(ctx, builder, s, &args[0])?;
            let len = compile_expression(ctx, builder, &args[1])?;
            let ch = compile_expression(ctx, builder, &args[2])?;
            let ch = ensure_naml_string(ctx, builder, ch, &args[2])?;
            call_three_arg_ptr_runtime(ctx, builder, runtime_fn, s, len, ch)
        }

        BuiltinStrategy::StringArgIntPtr(runtime_fn) => {
            let s = compile_expression(ctx, builder, &args[0])?;
            let s = ensure_naml_string(ctx, builder, s, &args[0])?;
            let n = compile_expression(ctx, builder, &args[1])?;
            call_two_arg_ptr_runtime(ctx, builder, runtime_fn, s, n)
        }

        BuiltinStrategy::StringJoin => {
            let arr = compile_expression(ctx, builder, &args[0])?;
            let delim = compile_expression(ctx, builder, &args[1])?;
            let delim = ensure_naml_string(ctx, builder, delim, &args[1])?;
            call_two_arg_ptr_runtime(ctx, builder, "naml_string_join", arr, delim)
        }

        // ========================================
        // Threads/Channel strategies
        // ========================================
        BuiltinStrategy::ThreadsJoin => {
            let func_ref = rt_func_ref(ctx, builder, "naml_wait_all")?;
            builder.ins().call(func_ref, &[]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::ChannelOpen => {
            let capacity = if args.is_empty() {
                builder.ins().iconst(types::I64, 1)
            } else {
                compile_expression(ctx, builder, &args[0])?
            };
            call_channel_new(ctx, builder, capacity)
        }

        BuiltinStrategy::ChannelSend => {
            let channel = compile_expression(ctx, builder, &args[0])?;
            let value = compile_expression(ctx, builder, &args[1])?;
            call_channel_send(ctx, builder, channel, value)
        }

        BuiltinStrategy::ChannelReceive => {
            let channel = compile_expression(ctx, builder, &args[0])?;
            call_channel_receive(ctx, builder, channel)
        }

        BuiltinStrategy::ChannelClose => {
            let channel = compile_expression(ctx, builder, &args[0])?;
            call_channel_close(ctx, builder, channel)?;
            Ok(builder.ins().iconst(types::I64, 0))
        }

        // ========================================
        // Lambda-based collection strategies
        // ========================================
        BuiltinStrategy::LambdaBool(runtime_fn) => {
            let arr = compile_expression(ctx, builder, &args[0])?;
            let closure = compile_expression(ctx, builder, &args[1])?;
            compile_lambda_bool_collection(ctx, builder, arr, closure, runtime_fn)
        }

        BuiltinStrategy::LambdaInt(runtime_fn) => {
            let arr = compile_expression(ctx, builder, &args[0])?;
            let closure = compile_expression(ctx, builder, &args[1])?;
            compile_lambda_int_collection(ctx, builder, arr, closure, runtime_fn)
        }

        BuiltinStrategy::LambdaArray(runtime_fn) => {
            let arr = compile_expression(ctx, builder, &args[0])?;
            let closure = compile_expression(ctx, builder, &args[1])?;
            compile_lambda_array_collection(ctx, builder, arr, closure, runtime_fn)
        }

        BuiltinStrategy::LambdaFind => {
            let arr = compile_expression(ctx, builder, &args[0])?;
            let closure = compile_expression(ctx, builder, &args[1])?;
            compile_lambda_find(ctx, builder, arr, closure)
        }

        BuiltinStrategy::LambdaFindIndex => {
            let arr = compile_expression(ctx, builder, &args[0])?;
            let closure = compile_expression(ctx, builder, &args[1])?;
            compile_lambda_find_index(ctx, builder, arr, closure)
        }

        BuiltinStrategy::LambdaFindLast => {
            let arr = compile_expression(ctx, builder, &args[0])?;
            let closure = compile_expression(ctx, builder, &args[1])?;
            compile_lambda_find_last(ctx, builder, arr, closure)
        }

        BuiltinStrategy::LambdaFindLastIndex => {
            let arr = compile_expression(ctx, builder, &args[0])?;
            let closure = compile_expression(ctx, builder, &args[1])?;
            compile_lambda_find_last_index(ctx, builder, arr, closure)
        }

        BuiltinStrategy::LambdaFold => {
            let arr = compile_expression(ctx, builder, &args[0])?;
            let initial = compile_expression(ctx, builder, &args[1])?;
            let closure = compile_expression(ctx, builder, &args[2])?;
            compile_lambda_fold(ctx, builder, arr, initial, closure)
        }

        BuiltinStrategy::LambdaScan => {
            let arr = compile_expression(ctx, builder, &args[0])?;
            let initial = compile_expression(ctx, builder, &args[1])?;
            let closure = compile_expression(ctx, builder, &args[2])?;
            compile_lambda_scan(ctx, builder, arr, initial, closure)
        }

        BuiltinStrategy::LambdaSortBy => {
            let arr = compile_expression(ctx, builder, &args[0])?;
            let closure = compile_expression(ctx, builder, &args[1])?;
            compile_lambda_sort_by(ctx, builder, arr, closure)
        }

        BuiltinStrategy::Sample => {
            let arr = compile_expression(ctx, builder, &args[0])?;
            compile_sample(ctx, builder, arr)
        }

        // ========================================
        // Core I/O strategies
        // ========================================
        BuiltinStrategy::Print(newline) => {
            compile_print_call(ctx, builder, args, newline)
        }

        BuiltinStrategy::Sleep => {
            if args.is_empty() {
                return Err(CodegenError::JitCompile(
                    "sleep requires milliseconds argument".to_string(),
                ));
            }
            let ms = compile_expression(ctx, builder, &args[0])?;
            call_sleep(ctx, builder, ms)
        }

        BuiltinStrategy::Stderr(func_name) => {
            compile_stderr_call(ctx, builder, args, func_name)
        }

        BuiltinStrategy::Fmt => {
            compile_fmt_call(ctx, builder, args)
        }

        BuiltinStrategy::ReadLine => {
            call_read_line(ctx, builder)
        }
    }
}
