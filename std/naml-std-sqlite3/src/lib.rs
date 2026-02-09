///
/// naml SQLite3 Database Support
///
/// Provides full SQLite3 database access through a C FFI compatible with
/// naml's Cranelift JIT compilation pipeline. Uses rusqlite with bundled
/// SQLite for zero system dependency.
///
/// Architecture:
/// - Connection, rows, and statement handles are stored in thread-safe
///   registries behind LazyLock<Mutex<Registry>> (same pattern as file
///   handles in naml-std-fs).
/// - All handles are i64 IDs returned to naml code.
/// - Query results are eagerly materialized into Vec<HashMap<String, Value>>
///   to avoid lifetime issues with rusqlite's borrowed Rows.
/// - Errors use naml's exception system via naml_exception_set_typed().
///
/// Functions:
/// - Connection: open, open_memory, close
/// - Execute: exec
/// - Query: query, row_count, row_at, get_string, get_int, get_float,
///   get_bool, is_null, columns, column_count
/// - Transactions: begin, commit, rollback
/// - Prepared statements: prepare, bind_string, bind_int, bind_float,
///   step, reset, finalize
/// - Utility: changes, last_insert_id
///

pub mod sqlite;

pub use sqlite::*;
