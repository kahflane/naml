///
/// SQLite3 runtime implementation for naml.
///
/// Uses three handle registries:
/// - CONN_REGISTRY: maps i64 handle → rusqlite::Connection
/// - ROWS_REGISTRY: maps i64 handle → materialized query result set
/// - STMT_REGISTRY: maps i64 handle → prepared statement (leaked connection ref)
///
/// Row handles encode (rows_handle << 32 | row_index) to avoid a separate registry.
///
/// Error handling follows naml's exception pattern:
/// - On success: return value normally
/// - On failure: call throw_db_error(), return sentinel (0, -1, or null)
///

use std::collections::HashMap;
use std::sync::Mutex;

use naml_std_core::{
    naml_exception_set_typed, naml_stack_capture, naml_string_new, NamlString,
    EXCEPTION_TYPE_DB_ERROR,
};
use rusqlite::{params_from_iter, Connection, Statement, types::Value as SqlValue};

fn sqlite_error_code(e: &rusqlite::Error) -> i64 {
    match e {
        rusqlite::Error::SqliteFailure(err, _) => err.extended_code as i64,
        _ => -1,
    }
}

fn throw_db_error(message: &str, code: i64) {
    unsafe {
        let message_ptr = naml_string_new(message.as_ptr(), message.len());
        let layout = std::alloc::Layout::from_size_align(24, 8).unwrap();
        let ptr = std::alloc::alloc(layout);
        if ptr.is_null() {
            panic!("Failed to allocate DBError");
        }
        *(ptr as *mut i64) = message_ptr as i64;
        let stack = naml_stack_capture();
        *(ptr.add(8) as *mut *mut u8) = stack;
        *(ptr.add(16) as *mut i64) = code;

        naml_exception_set_typed(ptr, EXCEPTION_TYPE_DB_ERROR);
    }
}

fn string_from_naml(s: *const NamlString) -> String {
    if s.is_null() {
        return String::new();
    }
    unsafe {
        let slice = std::slice::from_raw_parts((*s).data.as_ptr(), (*s).len);
        String::from_utf8_lossy(slice).into_owned()
    }
}

struct ConnRegistry {
    connections: HashMap<i64, Connection>,
    next_id: i64,
}

impl ConnRegistry {
    fn new() -> Self {
        Self {
            connections: HashMap::new(),
            next_id: 1,
        }
    }

    fn insert(&mut self, conn: Connection) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        self.connections.insert(id, conn);
        id
    }
}

struct MaterializedRow {
    columns: Vec<String>,
    values: Vec<SqlValue>,
}

struct MaterializedRows {
    columns: Vec<String>,
    rows: Vec<MaterializedRow>,
}

struct RowsRegistry {
    results: HashMap<i64, MaterializedRows>,
    next_id: i64,
}

impl RowsRegistry {
    fn new() -> Self {
        Self {
            results: HashMap::new(),
            next_id: 1,
        }
    }

    fn insert(&mut self, rows: MaterializedRows) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        self.results.insert(id, rows);
        id
    }
}

struct StmtEntry {
    stmt: *mut Statement<'static>,
    _conn_id: i64,
}

unsafe impl Send for StmtEntry {}

struct StmtRegistry {
    stmts: HashMap<i64, StmtEntry>,
    next_id: i64,
}

impl StmtRegistry {
    fn new() -> Self {
        Self {
            stmts: HashMap::new(),
            next_id: 1,
        }
    }

    fn insert(&mut self, entry: StmtEntry) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        self.stmts.insert(id, entry);
        id
    }
}

static CONN_REGISTRY: std::sync::LazyLock<Mutex<ConnRegistry>> =
    std::sync::LazyLock::new(|| Mutex::new(ConnRegistry::new()));

static ROWS_REGISTRY: std::sync::LazyLock<Mutex<RowsRegistry>> =
    std::sync::LazyLock::new(|| Mutex::new(RowsRegistry::new()));

static STMT_REGISTRY: std::sync::LazyLock<Mutex<StmtRegistry>> =
    std::sync::LazyLock::new(|| Mutex::new(StmtRegistry::new()));

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_db_sqlite_open(path: *const NamlString) -> i64 {
    let path_str = string_from_naml(path);
    match Connection::open(&path_str) {
        Ok(conn) => {
            let mut reg = CONN_REGISTRY.lock().unwrap();
            reg.insert(conn)
        }
        Err(e) => {
            throw_db_error(&e.to_string(), sqlite_error_code(&e));
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_db_sqlite_open_memory() -> i64 {
    match Connection::open_in_memory() {
        Ok(conn) => {
            let mut reg = CONN_REGISTRY.lock().unwrap();
            reg.insert(conn)
        }
        Err(e) => {
            throw_db_error(&e.to_string(), sqlite_error_code(&e));
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_db_sqlite_close(handle: i64) {
    let mut reg = CONN_REGISTRY.lock().unwrap();
    reg.connections.remove(&handle);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_db_sqlite_exec(
    handle: i64,
    sql: *const NamlString,
) {
    let sql_str = string_from_naml(sql);
    let reg = CONN_REGISTRY.lock().unwrap();
    if let Some(conn) = reg.connections.get(&handle) {
        if let Err(e) = conn.execute_batch(&sql_str) {
            throw_db_error(&e.to_string(), sqlite_error_code(&e));
        }
    } else {
        throw_db_error("Invalid database handle", -1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_db_sqlite_query(
    handle: i64,
    sql: *const NamlString,
    params: i64,
) -> i64 {
    let sql_str = string_from_naml(sql);

    let param_strings = if params != 0 {
        let arr = params as *const naml_std_core::NamlArray;
        let len = unsafe { (*arr).len };
        let data = unsafe { (*arr).data };
        let mut strings = Vec::with_capacity(len);
        for i in 0..len {
            let val = unsafe { *data.add(i) };
            let s = val as *const NamlString;
            strings.push(string_from_naml(s));
        }
        strings
    } else {
        Vec::new()
    };

    let reg = CONN_REGISTRY.lock().unwrap();
    if let Some(conn) = reg.connections.get(&handle) {
        let result = conn.prepare(&sql_str);
        match result {
            Ok(mut stmt) => {
                let col_count = stmt.column_count();
                let columns: Vec<String> = (0..col_count)
                    .map(|i| stmt.column_name(i).unwrap_or("").to_string())
                    .collect();

                let param_refs: Vec<&dyn rusqlite::types::ToSql> = param_strings
                    .iter()
                    .map(|s| s as &dyn rusqlite::types::ToSql)
                    .collect();

                let row_result = stmt.query_map(params_from_iter(param_refs.iter().copied()), |row| {
                    let mut values = Vec::with_capacity(col_count);
                    for i in 0..col_count {
                        let val: SqlValue = row.get_unwrap(i);
                        values.push(val);
                    }
                    Ok(MaterializedRow {
                        columns: columns.clone(),
                        values,
                    })
                });

                match row_result {
                    Ok(mapped_rows) => {
                        let mut rows_vec = Vec::new();
                        for row in mapped_rows {
                            match row {
                                Ok(r) => rows_vec.push(r),
                                Err(e) => {
                                    throw_db_error(&e.to_string(), -1);
                                    return -1;
                                }
                            }
                        }
                        let materialized = MaterializedRows {
                            columns,
                            rows: rows_vec,
                        };
                        let mut rows_reg = ROWS_REGISTRY.lock().unwrap();
                        rows_reg.insert(materialized)
                    }
                    Err(e) => {
                        throw_db_error(&e.to_string(), sqlite_error_code(&e));
                        -1
                    }
                }
            }
            Err(e) => {
                throw_db_error(&e.to_string(), sqlite_error_code(&e));
                -1
            }
        }
    } else {
        throw_db_error("Invalid database handle", -1);
        -1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_db_sqlite_row_count(rows_handle: i64) -> i64 {
    let reg = ROWS_REGISTRY.lock().unwrap();
    if let Some(rows) = reg.results.get(&rows_handle) {
        rows.rows.len() as i64
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_db_sqlite_row_at(
    rows_handle: i64,
    index: i64,
) -> i64 {
    (rows_handle << 32) | (index & 0xFFFFFFFF)
}

fn decode_row_handle(row_handle: i64) -> (i64, usize) {
    let rows_id = row_handle >> 32;
    let index = (row_handle & 0xFFFFFFFF) as usize;
    (rows_id, index)
}

fn get_column_value<'a>(
    reg: &'a std::sync::MutexGuard<'_, RowsRegistry>,
    row_handle: i64,
    col: *const NamlString,
) -> Option<&'a SqlValue> {
    let (rows_id, index) = decode_row_handle(row_handle);
    let col_name = string_from_naml(col);
    let rows = reg.results.get(&rows_id)?;
    let row = rows.rows.get(index)?;
    let col_idx = row.columns.iter().position(|c| c == &col_name)?;
    row.values.get(col_idx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_db_sqlite_get_string(
    row_handle: i64,
    col: *const NamlString,
) -> *mut NamlString {
    let reg = ROWS_REGISTRY.lock().unwrap();
    if let Some(val) = get_column_value(&reg, row_handle, col) {
        match val {
            SqlValue::Text(s) => unsafe { naml_string_new(s.as_ptr(), s.len()) },
            SqlValue::Integer(i) => {
                let s = i.to_string();
                unsafe { naml_string_new(s.as_ptr(), s.len()) }
            }
            SqlValue::Real(f) => {
                let s = f.to_string();
                unsafe { naml_string_new(s.as_ptr(), s.len()) }
            }
            SqlValue::Null => {
                let s = "";
                unsafe { naml_string_new(s.as_ptr(), s.len()) }
            }
            SqlValue::Blob(b) => {
                let s = format!("<blob {} bytes>", b.len());
                unsafe { naml_string_new(s.as_ptr(), s.len()) }
            }
        }
    } else {
        let s = "";
        unsafe { naml_string_new(s.as_ptr(), s.len()) }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_db_sqlite_get_int(
    row_handle: i64,
    col: *const NamlString,
) -> i64 {
    let reg = ROWS_REGISTRY.lock().unwrap();
    if let Some(val) = get_column_value(&reg, row_handle, col) {
        match val {
            SqlValue::Integer(i) => *i,
            SqlValue::Real(f) => *f as i64,
            SqlValue::Text(s) => s.parse::<i64>().unwrap_or(0),
            _ => 0,
        }
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_db_sqlite_get_float(
    row_handle: i64,
    col: *const NamlString,
) -> f64 {
    let reg = ROWS_REGISTRY.lock().unwrap();
    if let Some(val) = get_column_value(&reg, row_handle, col) {
        match val {
            SqlValue::Real(f) => *f,
            SqlValue::Integer(i) => *i as f64,
            SqlValue::Text(s) => s.parse::<f64>().unwrap_or(0.0),
            _ => 0.0,
        }
    } else {
        0.0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_db_sqlite_get_bool(
    row_handle: i64,
    col: *const NamlString,
) -> i64 {
    let reg = ROWS_REGISTRY.lock().unwrap();
    if let Some(val) = get_column_value(&reg, row_handle, col) {
        match val {
            SqlValue::Integer(i) => if *i != 0 { 1 } else { 0 },
            SqlValue::Real(f) => if *f != 0.0 { 1 } else { 0 },
            SqlValue::Text(s) => {
                if s == "true" || s == "1" { 1 } else { 0 }
            }
            SqlValue::Null => 0,
            _ => 0,
        }
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_db_sqlite_is_null(
    row_handle: i64,
    col: *const NamlString,
) -> i64 {
    let reg = ROWS_REGISTRY.lock().unwrap();
    if let Some(val) = get_column_value(&reg, row_handle, col) {
        if matches!(val, SqlValue::Null) { 1 } else { 0 }
    } else {
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_db_sqlite_columns(rows_handle: i64) -> *mut NamlString {
    let reg = ROWS_REGISTRY.lock().unwrap();
    if let Some(rows) = reg.results.get(&rows_handle) {
        let joined = rows.columns.join(",");
        unsafe { naml_string_new(joined.as_ptr(), joined.len()) }
    } else {
        let s = "";
        unsafe { naml_string_new(s.as_ptr(), s.len()) }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_db_sqlite_column_count(rows_handle: i64) -> i64 {
    let reg = ROWS_REGISTRY.lock().unwrap();
    if let Some(rows) = reg.results.get(&rows_handle) {
        rows.columns.len() as i64
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_db_sqlite_begin(handle: i64) {
    let reg = CONN_REGISTRY.lock().unwrap();
    if let Some(conn) = reg.connections.get(&handle) {
        if let Err(e) = conn.execute_batch("BEGIN") {
            throw_db_error(&e.to_string(), sqlite_error_code(&e));
        }
    } else {
        throw_db_error("Invalid database handle", -1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_db_sqlite_commit(handle: i64) {
    let reg = CONN_REGISTRY.lock().unwrap();
    if let Some(conn) = reg.connections.get(&handle) {
        if let Err(e) = conn.execute_batch("COMMIT") {
            throw_db_error(&e.to_string(), sqlite_error_code(&e));
        }
    } else {
        throw_db_error("Invalid database handle", -1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_db_sqlite_rollback(handle: i64) {
    let reg = CONN_REGISTRY.lock().unwrap();
    if let Some(conn) = reg.connections.get(&handle) {
        if let Err(e) = conn.execute_batch("ROLLBACK") {
            throw_db_error(&e.to_string(), sqlite_error_code(&e));
        }
    } else {
        throw_db_error("Invalid database handle", -1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_db_sqlite_prepare(
    handle: i64,
    sql: *const NamlString,
) -> i64 {
    let sql_str = string_from_naml(sql);
    let reg = CONN_REGISTRY.lock().unwrap();
    if let Some(conn) = reg.connections.get(&handle) {
        let conn_ptr = conn as *const Connection;
        let conn_ref: &'static Connection = unsafe { &*conn_ptr };
        match conn_ref.prepare(&sql_str) {
            Ok(stmt) => {
                let boxed = Box::new(stmt);
                let raw = Box::into_raw(boxed) as *mut Statement<'static>;
                let entry = StmtEntry {
                    stmt: raw,
                    _conn_id: handle,
                };
                let mut stmt_reg = STMT_REGISTRY.lock().unwrap();
                stmt_reg.insert(entry)
            }
            Err(e) => {
                throw_db_error(&e.to_string(), sqlite_error_code(&e));
                -1
            }
        }
    } else {
        throw_db_error("Invalid database handle", -1);
        -1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_db_sqlite_bind_string(
    stmt_handle: i64,
    index: i64,
    val: *const NamlString,
) {
    let val_str = string_from_naml(val);
    let reg = STMT_REGISTRY.lock().unwrap();
    if let Some(entry) = reg.stmts.get(&stmt_handle) {
        let stmt = unsafe { &mut *entry.stmt };
        if let Err(e) = stmt.raw_bind_parameter(index as usize, val_str) {
            throw_db_error(&e.to_string(), -1);
        }
    } else {
        throw_db_error("Invalid statement handle", -1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_db_sqlite_bind_int(
    stmt_handle: i64,
    index: i64,
    val: i64,
) {
    let reg = STMT_REGISTRY.lock().unwrap();
    if let Some(entry) = reg.stmts.get(&stmt_handle) {
        let stmt = unsafe { &mut *entry.stmt };
        if let Err(e) = stmt.raw_bind_parameter(index as usize, val) {
            throw_db_error(&e.to_string(), -1);
        }
    } else {
        throw_db_error("Invalid statement handle", -1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_db_sqlite_bind_float(
    stmt_handle: i64,
    index: i64,
    val: f64,
) {
    let reg = STMT_REGISTRY.lock().unwrap();
    if let Some(entry) = reg.stmts.get(&stmt_handle) {
        let stmt = unsafe { &mut *entry.stmt };
        if let Err(e) = stmt.raw_bind_parameter(index as usize, val) {
            throw_db_error(&e.to_string(), -1);
        }
    } else {
        throw_db_error("Invalid statement handle", -1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_db_sqlite_step(stmt_handle: i64) {
    let reg = STMT_REGISTRY.lock().unwrap();
    if let Some(entry) = reg.stmts.get(&stmt_handle) {
        let stmt = unsafe { &mut *entry.stmt };
        if let Err(e) = stmt.raw_execute() {
            throw_db_error(&e.to_string(), -1);
        }
    } else {
        throw_db_error("Invalid statement handle", -1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_db_sqlite_reset(stmt_handle: i64) {
    let mut reg = STMT_REGISTRY.lock().unwrap();
    if let Some(entry) = reg.stmts.get_mut(&stmt_handle) {
        let stmt = unsafe { &mut *entry.stmt };
        stmt.clear_bindings();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_db_sqlite_finalize(stmt_handle: i64) {
    let mut reg = STMT_REGISTRY.lock().unwrap();
    if let Some(entry) = reg.stmts.remove(&stmt_handle) {
        unsafe {
            let _ = Box::from_raw(entry.stmt);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_db_sqlite_changes(handle: i64) -> i64 {
    let reg = CONN_REGISTRY.lock().unwrap();
    if let Some(conn) = reg.connections.get(&handle) {
        conn.changes() as i64
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_db_sqlite_last_insert_id(handle: i64) -> i64 {
    let reg = CONN_REGISTRY.lock().unwrap();
    if let Some(conn) = reg.connections.get(&handle) {
        conn.last_insert_rowid()
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn naml_db_sqlite_error_new(
    message: *const NamlString,
    code: i64,
) -> *mut u8 {
    unsafe {
        let layout = std::alloc::Layout::from_size_align(24, 8).unwrap();
        let ptr = std::alloc::alloc(layout);
        if ptr.is_null() {
            panic!("Failed to allocate DBError");
        }
        *(ptr as *mut i64) = message as i64;
        *(ptr.add(8) as *mut i64) = 0;
        *(ptr.add(16) as *mut i64) = code;
        ptr
    }
}
