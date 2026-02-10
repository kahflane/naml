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
use cranelift_module::Module;

use super::array::{
    call_array_clear_runtime, call_array_contains_bool, call_array_fill_runtime, call_array_push,
};
use super::misc::{
    call_int_runtime, call_one_arg_int_runtime, call_one_arg_ptr_runtime,
    call_three_arg_int_runtime, call_three_arg_ptr_runtime, call_three_arg_void_runtime,
    call_two_arg_bool_runtime, call_two_arg_int_runtime, call_two_arg_ptr_runtime,
    call_two_arg_runtime, call_void_runtime, ensure_i64,
};
use super::options::{
    compile_option_from_array_access, compile_option_from_array_get, compile_option_from_index_of,
    compile_option_from_last_index_of, compile_option_from_map_first,
    compile_option_from_map_remove, compile_option_from_minmax, compile_option_from_nullable_ptr,
    compile_option_from_remove_at,
};
use super::heap::heap_type_from_type;
use super::runtime::emit_incref;
use super::strings::call_string_from_cstr;
use super::{ARRAY_LEN_OFFSET, CompileContext};
use crate::ast::{Expression, Literal, LiteralExpr};
use crate::codegen::CodegenError;

/// Describes how to compile a built-in function
#[derive(Debug, Clone, Copy)]
pub enum BuiltinStrategy {
    /// Load array length from header offset
    ArrayLength,
    /// Create array with pre-allocated capacity
    ArrayWithCapacity,
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
    /// (value) -> mutex<T>
    MutexNew,
    /// (value) -> rwlock<T>
    RwlockNew,
    /// (value) -> atomic<T>
    AtomicNew,
    /// (atomic<T>) -> T
    AtomicLoad,
    /// (atomic<T>, T) -> void
    AtomicStore,
    /// (atomic<T>, T) -> T
    AtomicAdd,
    /// (atomic<T>, T) -> T
    AtomicSub,
    /// (atomic<T>) -> T
    AtomicInc,
    /// (atomic<T>) -> T
    AtomicDec,
    /// (atomic<T>, T, T) -> bool
    AtomicCas,
    /// (atomic<T>, T) -> T
    AtomicSwap,
    /// (atomic<T>, T) -> T
    AtomicAnd,
    /// (atomic<T>, T) -> T
    AtomicOr,
    /// (atomic<T>, T) -> T
    AtomicXor,

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
    // Map collection strategies
    // ========================================
    /// Get map length (count)
    MapLength,
    /// (map, key) -> bool (contains_key)
    MapContainsKey,
    /// (map, key) -> option<V> (remove)
    MapRemove,
    /// (map) -> unit (clear)
    MapClear,
    /// (map) -> array (keys, values)
    MapExtract(&'static str),
    /// (map) -> array of pairs (entries)
    MapEntries,
    /// (map) -> option<K> or option<V> (first_key, first_value)
    MapFirstOption(&'static str),
    /// (map, closure) -> bool (any, all)
    MapLambdaBool(&'static str),
    /// (map, closure) -> int (count_if)
    MapLambdaInt(&'static str),
    /// (map, initial, closure) -> T (fold)
    MapLambdaFold,
    /// (map, closure) -> map (transform, where, reject)
    MapLambdaMap(&'static str),
    /// (map, map) -> map (merge, defaults, intersect, diff)
    MapCombine(&'static str),
    /// (map) -> map (invert)
    MapInvert,
    /// (keys_array, values_array) -> map (from_arrays)
    MapFromArrays,
    /// (pairs_array) -> map (from_entries)
    MapFromEntries,

    // ========================================
    // File system module strategies
    // ========================================
    /// (path) -> string throws IOError
    FsRead,
    /// (path) -> bytes throws IOError
    FsReadBytes,
    /// (path, content) -> unit throws IOError
    FsWrite,
    /// (path, content) -> unit throws IOError
    FsAppend,
    /// (path, bytes) -> unit throws IOError
    FsWriteBytes,
    /// (path, bytes) -> unit throws IOError
    FsAppendBytes,
    /// (path) -> bool
    FsExists,
    /// (path) -> bool
    FsIsFile,
    /// (path) -> bool
    FsIsDir,
    /// (path) -> [string] throws IOError
    FsListDir,
    /// (path) -> unit throws IOError
    FsMkdir,
    /// (path) -> unit throws IOError
    FsMkdirAll,
    /// (path) -> unit throws IOError
    FsRemove,
    /// (path) -> unit throws IOError
    FsRemoveAll,
    /// ([string]) -> string
    FsJoin,
    /// (path) -> string
    FsDirname,
    /// (path) -> string
    FsBasename,
    /// (path) -> string
    FsExtension,
    /// (path) -> string throws IOError
    FsAbsolute,
    /// (path) -> int throws IOError
    FsSize,
    /// (path) -> int throws IOError
    FsModified,
    /// (src, dst) -> unit throws IOError
    FsCopy,
    /// (src, dst) -> unit throws IOError
    FsRename,
    /// () -> string throws IOError
    FsGetwd,
    /// (path) -> unit throws IOError
    FsChdir,
    /// (prefix) -> string throws IOError
    FsCreateTemp,
    /// (prefix) -> string throws IOError
    FsMkdirTemp,
    /// (path, mode) -> unit throws IOError
    FsChmod,
    /// (path, size) -> unit throws IOError
    FsTruncate,
    /// (path) -> [int] throws IOError
    FsStat,

    // ========================================
    // Memory-mapped file strategies
    // ========================================
    /// (path, writable) -> int throws IOError
    FsMmapOpen,
    /// (handle) -> int throws IOError
    FsMmapLen,
    /// (handle, offset) -> int throws IOError
    FsMmapReadByte,
    /// (handle, offset, value) -> unit throws IOError
    FsMmapWriteByte,
    /// (handle, offset, len) -> bytes throws IOError
    FsMmapRead,
    /// (handle, offset, data) -> unit throws IOError
    FsMmapWrite,
    /// (handle) -> unit throws IOError
    FsMmapFlush,
    /// (handle) -> unit throws IOError
    FsMmapClose,

    // ========================================
    // File handle strategies
    // ========================================
    /// (path, mode) -> int throws IOError
    FsFileOpen,
    /// (handle) -> unit throws IOError
    FsFileClose,
    /// (handle, count) -> string throws IOError
    FsFileRead,
    /// (handle) -> string throws IOError
    FsFileReadLine,
    /// (handle) -> string throws IOError
    FsFileReadAll,
    /// (handle, content) -> int throws IOError
    FsFileWrite,
    /// (handle, content) -> int throws IOError
    FsFileWriteLine,
    /// (handle) -> unit throws IOError
    FsFileFlush,
    /// (handle, offset, whence) -> int throws IOError
    FsFileSeek,
    /// (handle) -> int throws IOError
    FsFileTell,
    /// (handle) -> bool throws IOError
    FsFileEof,
    /// (handle) -> int throws IOError
    FsFileSize,

    // ========================================
    // Link/symlink strategies
    // ========================================
    /// (target, link_path) -> unit throws IOError
    FsSymlink,
    /// (path) -> string throws IOError
    FsReadlink,
    /// (path) -> [int] throws IOError
    FsLstat,
    /// (src, dst) -> unit throws IOError
    FsLink,
    /// (path, atime_ms, mtime_ms) -> unit throws IOError
    FsChtimes,
    /// (path, uid, gid) -> unit throws IOError
    FsChown,
    /// (path, uid, gid) -> unit throws IOError
    FsLchown,
    /// (path1, path2) -> bool throws IOError
    FsSameFile,

    // ========================================
    // Additional file handle strategies
    // ========================================
    /// (handle, buf_size, offset) -> string throws IOError
    FsFileReadAt,
    /// (handle, content, offset) -> int throws IOError
    FsFileWriteAt,
    /// (handle) -> string throws IOError
    FsFileName,
    /// (handle) -> [int] throws IOError
    FsFileStat,
    /// (handle, size) -> unit throws IOError
    FsFileTruncate,
    /// (handle, mode) -> unit throws IOError
    FsFileChmod,
    /// (handle, uid, gid) -> unit throws IOError
    FsFileChown,

    // ========================================
    // Path module strategies
    // ========================================
    /// ([string]) -> string (join)
    PathJoin,
    /// (path) -> string (normalize, dirname, basename, extension, stem)
    PathOneArgStr(&'static str),
    /// (path) -> bool (is_absolute, is_relative, has_root)
    PathOneArgBool(&'static str),
    /// (path, other) -> string (with_extension, strip_prefix)
    PathTwoArgStr(&'static str),
    /// (path, other) -> bool (starts_with, ends_with)
    PathTwoArgBool(&'static str),
    /// (path) -> [string] (components)
    PathComponents,
    /// () -> string (separator)
    PathSeparator,

    // ========================================
    // Env module strategies
    // ========================================
    /// (key) -> string (getenv)
    EnvGetenv,
    /// (key) -> option<string> (lookup_env)
    EnvLookupEnv,
    /// (key, value) -> unit throws EnvError (setenv)
    EnvSetenv,
    /// (key) -> unit throws EnvError (unsetenv)
    EnvUnsetenv,
    /// () -> unit throws EnvError (clearenv)
    EnvClearenv,
    /// () -> [string] (environ)
    EnvEnviron,
    /// (s) -> string (expand_env)
    EnvExpandEnv,

    // ========================================
    // OS module strategies
    // ========================================
    /// () -> string throws OSError (hostname)
    OsHostname,
    /// () -> string (temp_dir)
    OsTempDir,
    /// () -> string throws OSError (home_dir)
    OsHomeDir,
    /// () -> string throws OSError (cache_dir)
    OsCacheDir,
    /// () -> string throws OSError (config_dir)
    OsConfigDir,
    /// () -> string throws OSError (executable)
    OsExecutable,
    /// () -> int (pagesize)
    OsPagesize,
    /// () -> int (getuid)
    OsGetuid,
    /// () -> int (geteuid)
    OsGeteuid,
    /// () -> int (getgid)
    OsGetgid,
    /// () -> int (getegid)
    OsGetegid,
    /// () -> [int] throws OSError (getgroups)
    OsGetgroups,

    // ========================================
    // Process module strategies
    // ========================================
    /// () -> int (getpid)
    ProcessGetpid,
    /// () -> int (getppid)
    ProcessGetppid,
    /// (code: int) -> unit (exit)
    ProcessExit,
    /// () -> int throws ProcessError (pipe read fd)
    ProcessPipeRead,
    /// () -> int (pipe write fd)
    ProcessPipeWrite,
    /// (name: string, args: [string]) -> int throws ProcessError
    ProcessStart,
    /// (pid: int) -> int throws ProcessError
    ProcessFind,
    /// (handle: int) -> [int] throws ProcessError
    ProcessWait,
    /// (handle: int, sig: int) throws ProcessError
    ProcessSignal,
    /// (handle: int) throws ProcessError
    ProcessKill,
    /// (handle: int) -> unit
    ProcessRelease,
    /// Signal constants
    ProcessSighup,
    ProcessSigint,
    ProcessSigquit,
    ProcessSigkill,
    ProcessSigterm,
    ProcessSigstop,
    ProcessSigcont,

    // ========================================
    // Testing module strategies
    // ========================================
    /// (condition: bool, message: string) -> unit
    TestingAssert,
    /// (actual: int, expected: int, message: string) -> unit
    TestingAssertEq,
    /// (actual: float, expected: float, message: string) -> unit
    TestingAssertEqFloat,
    /// (actual: string, expected: string, message: string) -> unit
    TestingAssertEqString,
    /// (actual: bool, expected: bool, message: string) -> unit
    TestingAssertEqBool,
    /// (actual: int, expected: int, message: string) -> unit
    TestingAssertNeq,
    /// (actual: string, expected: string, message: string) -> unit
    TestingAssertNeqString,
    /// (condition: bool, message: string) -> unit
    TestingAssertTrue,
    /// (condition: bool, message: string) -> unit
    TestingAssertFalse,
    /// (actual: int, expected: int, message: string) -> unit
    TestingAssertGt,
    /// (actual: int, expected: int, message: string) -> unit
    TestingAssertGte,
    /// (actual: int, expected: int, message: string) -> unit
    TestingAssertLt,
    /// (actual: int, expected: int, message: string) -> unit
    TestingAssertLte,
    /// (message: string) -> unit
    TestingFail,
    /// (actual: float, expected: float, epsilon: float, message: string) -> unit
    TestingAssertApprox,
    /// (haystack: string, needle: string, message: string) -> unit
    TestingAssertContains,
    /// (value: string, prefix: string, message: string) -> unit
    TestingAssertStartsWith,
    /// (value: string, suffix: string, message: string) -> unit
    TestingAssertEndsWith,

    // ========================================
    // Crypto module strategies
    // ========================================
    /// (bytes) -> bytes (hash digest)
    CryptoHashBytes(&'static str),
    /// (bytes) -> string (hash hex)
    CryptoHashHex(&'static str),
    /// (bytes, bytes) -> bytes (HMAC digest)
    CryptoHmacBytes(&'static str),
    /// (bytes, bytes) -> string (HMAC hex)
    CryptoHmacHex(&'static str),
    /// (bytes, bytes, bytes) -> bool (HMAC verify)
    CryptoHmacVerify(&'static str),
    /// (bytes, bytes, int, int) -> bytes (PBKDF2)
    CryptoPbkdf2(&'static str),
    /// (int) -> bytes (random bytes)
    CryptoRandomBytes(&'static str),

    // ========================================
    // Encoding module strategies
    // ========================================
    /// (bytes) -> string (encode bytes to string)
    EncodingBytesToString(&'static str),
    /// (string) -> bytes (encode string to bytes)
    EncodingStringToBytes(&'static str),
    /// (bytes) -> bool (validation)
    EncodingValidate(&'static str),
    /// (bytes, out_tag, out_value) -> throwing decode to string
    EncodingDecodeToString(&'static str),
    /// (string, out_tag, out_value) -> throwing decode to bytes
    EncodingDecodeToBytes(&'static str),

    // ========================================
    // JSON encoding strategies
    // ========================================
    /// (string) -> json throws DecodeError
    JsonDecode,
    /// (json) -> string
    JsonEncode(&'static str),
    /// (json, string) -> bool
    JsonExists,
    /// (json, string) -> json throws PathError
    JsonPath,
    /// (json) -> [string]
    JsonKeys,
    /// (json) -> int
    JsonCount,
    /// (json) -> int
    JsonGetType,
    /// (json) -> string
    JsonTypeName,
    /// (json) -> bool
    JsonIsNull,

    // ========================================
    // TOML encoding strategies
    // ========================================
    /// (string) -> json throws DecodeError
    TomlDecode,
    /// (json) -> string throws EncodeError
    TomlEncode(&'static str),

    // ========================================
    // YAML encoding strategies
    // ========================================
    /// (string) -> json throws DecodeError
    YamlDecode,
    /// (json) -> string throws EncodeError
    YamlEncode,

    // ========================================
    // Binary encoding strategies
    // ========================================
    /// (arg0) -> result: alloc, from_string, len, capacity
    BinaryOneArgCall(&'static str),
    /// (arg0, arg1) -> result: int reads, index_of, concat
    BinaryTwoArgCall(&'static str),
    /// (arg0, arg1, arg2) -> result: slice
    BinaryThreeArgCall(&'static str),
    /// (bytes, offset) -> f64: float reads
    BinaryReadFloat(&'static str),
    /// (arg0) -> void: clear
    BinaryOneArgVoid(&'static str),
    /// (arg0, arg1) -> void: append, resize, fill
    BinaryTwoArgVoid(&'static str),
    /// (arg0, arg1, arg2) -> void: int writes
    BinaryThreeArgVoid(&'static str),
    /// (bytes, offset, f64) -> void: float writes
    BinaryWriteFloat(&'static str),
    /// (arg0, arg1, arg2, arg3) -> void: copy_within
    BinaryFourArgVoid(&'static str),
    /// (arg0, arg1) -> bool: contains, starts_with, ends_with, equals
    BinaryTwoArgBool(&'static str),

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

    // ========================================
    // Networking module strategies
    // ========================================
    // TCP Server
    /// (address: string) -> int throws NetworkError
    NetTcpListen,
    /// (listener: int) -> int throws NetworkError
    NetTcpAccept,
    /// (listener: int) -> unit
    NetTcpServerClose,
    /// (listener: int) -> string
    NetTcpServerLocalAddr,

    // TCP Client
    /// (address: string) -> int throws NetworkError, TimeoutError
    NetTcpConnect,
    /// (socket: int, size: int) -> bytes throws NetworkError
    NetTcpRead,
    /// (socket: int) -> bytes throws NetworkError
    NetTcpReadAll,
    /// (socket: int, data: bytes) -> unit throws NetworkError
    NetTcpWrite,
    /// (socket: int) -> unit
    NetTcpClientClose,
    /// (socket: int, ms: int) -> unit
    NetTcpSetTimeout,
    /// (socket: int) -> string
    NetTcpPeerAddr,

    // UDP
    /// (address: string) -> int throws NetworkError
    NetUdpBind,
    /// (socket: int, data: bytes, address: string) -> unit throws NetworkError
    NetUdpSend,
    /// (socket: int, size: int) -> bytes throws NetworkError
    NetUdpReceive,
    /// (socket: int) -> unit
    NetUdpClose,
    /// (socket: int) -> string
    NetUdpLocalAddr,

    // HTTP Client
    /// (url: string) -> int throws NetworkError, TimeoutError
    NetHttpGet,
    /// (url: string, body: bytes) -> int throws NetworkError, TimeoutError
    NetHttpPost,
    /// (url: string, body: bytes) -> int throws NetworkError, TimeoutError
    NetHttpPut,
    /// (url: string, body: bytes) -> int throws NetworkError, TimeoutError
    NetHttpPatch,
    /// (url: string) -> int throws NetworkError, TimeoutError
    NetHttpDelete,
    /// (ms: int) -> unit
    NetHttpSetTimeout,
    /// (response: int) -> int
    NetHttpStatus,
    /// (response: int) -> bytes
    NetHttpBody,

    // ========================================
    // HTTP Server strategies
    // ========================================
    /// () -> int (router handle)
    NetHttpServerOpenRouter,
    /// (router: int, pattern: string, handler: fn_ptr) -> unit
    NetHttpServerGet,
    /// (router: int, pattern: string, handler: fn_ptr) -> unit
    NetHttpServerPost,
    /// (router: int, pattern: string, handler: fn_ptr) -> unit
    NetHttpServerPut,
    /// (router: int, pattern: string, handler: fn_ptr) -> unit
    NetHttpServerPatch,
    /// (router: int, pattern: string, handler: fn_ptr) -> unit
    NetHttpServerDelete,
    /// (router: int, middleware: int) -> unit
    NetHttpServerWith,
    /// (router: int, prefix: string) -> int (group handle)
    NetHttpServerGroup,
    /// (router: int, prefix: string, sub_router: int) -> unit
    NetHttpServerMount,
    /// (address: string, router: int) -> unit throws NetworkError
    NetHttpServerServe,
    /// (status: int, body: string) -> int (response handle)
    NetHttpServerTextResponse,

    // ========================================
    // SQLite database strategies
    // ========================================
    /// (path: string) -> int throws DBError
    SqliteOpen,
    /// () -> int throws DBError
    SqliteOpenMemory,
    /// (handle: int) -> unit
    SqliteClose,
    /// (handle: int, sql: string) -> unit throws DBError
    SqliteExec,
    /// (handle: int, sql: string, params: [string]) -> int throws DBError
    SqliteQuery,
    /// (rows: int) -> int
    SqliteRowCount,
    /// (rows: int, index: int) -> int
    SqliteRowAt,
    /// (row: int, col: string) -> string
    SqliteGetString,
    /// (row: int, col: string) -> int
    SqliteGetInt,
    /// (row: int, col: string) -> float
    SqliteGetFloat,
    /// (row: int, col: string) -> bool
    SqliteGetBool,
    /// (row: int, col: string) -> bool
    SqliteIsNull,
    /// (rows: int) -> string
    SqliteColumns,
    /// (rows: int) -> int
    SqliteColumnCount,
    /// (handle: int) -> unit throws DBError
    SqliteBegin,
    /// (handle: int) -> unit throws DBError
    SqliteCommit,
    /// (handle: int) -> unit throws DBError
    SqliteRollback,
    /// (handle: int, sql: string) -> int throws DBError
    SqlitePrepare,
    /// (stmt: int, index: int, val: string) -> unit throws DBError
    SqliteBindString,
    /// (stmt: int, index: int, val: int) -> unit throws DBError
    SqliteBindInt,
    /// (stmt: int, index: int, val: float) -> unit throws DBError
    SqliteBindFloat,
    /// (stmt: int) -> unit throws DBError
    SqliteStep,
    /// (stmt: int) -> unit
    SqliteReset,
    /// (stmt: int) -> unit
    SqliteFinalize,
    /// (handle: int) -> int
    SqliteChanges,
    /// (handle: int) -> int
    SqliteLastInsertId,

    // ========================================
    // Timers module strategies
    // ========================================
    /// (callback, ms) -> int handle — unpack 24-byte closure
    TimerSetTimeout,
    /// (handle) -> void
    TimerCancelTimeout,
    /// (callback, ms) -> int handle — unpack 24-byte closure
    TimerSetInterval,
    /// (handle) -> void
    TimerCancelInterval,
    /// (callback, cron_expr) -> int handle throws ScheduleError
    TimerSchedule,
    /// (handle) -> void
    TimerCancelSchedule,
    /// (handle) -> int (epoch ms)
    TimerNextRun,
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
        BuiltinFunction {
            name: "collections::arrays::count",
            strategy: BuiltinStrategy::ArrayLength,
        },
        BuiltinFunction {
            name: "collections::arrays::push",
            strategy: BuiltinStrategy::ArrayPush,
        },
        BuiltinFunction {
            name: "collections::arrays::reserved",
            strategy: BuiltinStrategy::ArrayWithCapacity,
        },
        BuiltinFunction {
            name: "collections::arrays::pop",
            strategy: BuiltinStrategy::OneArgOptionAccess("naml_array_pop"),
        },
        BuiltinFunction {
            name: "collections::arrays::shift",
            strategy: BuiltinStrategy::OneArgOptionAccess("naml_array_shift"),
        },
        BuiltinFunction {
            name: "collections::arrays::first",
            strategy: BuiltinStrategy::OneArgOptionAccess("naml_array_first"),
        },
        BuiltinFunction {
            name: "collections::arrays::last",
            strategy: BuiltinStrategy::OneArgOptionAccess("naml_array_last"),
        },
        BuiltinFunction {
            name: "collections::arrays::fill",
            strategy: BuiltinStrategy::ArrayFill,
        },
        BuiltinFunction {
            name: "collections::arrays::clear",
            strategy: BuiltinStrategy::ArrayClear,
        },
        BuiltinFunction {
            name: "collections::arrays::get",
            strategy: BuiltinStrategy::ArrayGet,
        },
        BuiltinFunction {
            name: "collections::arrays::sum",
            strategy: BuiltinStrategy::OneArgInt("naml_array_sum"),
        },
        BuiltinFunction {
            name: "collections::arrays::min",
            strategy: BuiltinStrategy::ArrayMinMax("naml_array_min", true),
        },
        BuiltinFunction {
            name: "collections::arrays::max",
            strategy: BuiltinStrategy::ArrayMinMax("naml_array_max", false),
        },
        BuiltinFunction {
            name: "collections::arrays::reversed",
            strategy: BuiltinStrategy::OneArgPtr("naml_array_reversed"),
        },
        BuiltinFunction {
            name: "collections::arrays::sort",
            strategy: BuiltinStrategy::OneArgPtr("naml_array_sort"),
        },
        BuiltinFunction {
            name: "collections::arrays::flatten",
            strategy: BuiltinStrategy::OneArgPtr("naml_array_flatten"),
        },
        BuiltinFunction {
            name: "collections::arrays::take",
            strategy: BuiltinStrategy::TwoArgPtr("naml_array_take"),
        },
        BuiltinFunction {
            name: "collections::arrays::drop",
            strategy: BuiltinStrategy::TwoArgPtr("naml_array_drop"),
        },
        BuiltinFunction {
            name: "collections::arrays::slice",
            strategy: BuiltinStrategy::ThreeArgPtr("naml_array_slice"),
        },
        BuiltinFunction {
            name: "collections::arrays::index_of",
            strategy: BuiltinStrategy::ArrayIndexOf,
        },
        BuiltinFunction {
            name: "collections::arrays::contains",
            strategy: BuiltinStrategy::ArrayContains,
        },
        // Mutation operations
        BuiltinFunction {
            name: "collections::arrays::insert",
            strategy: BuiltinStrategy::ThreeArgVoid("naml_array_insert"),
        },
        BuiltinFunction {
            name: "collections::arrays::remove_at",
            strategy: BuiltinStrategy::TwoArgOptionInt("naml_array_remove_at"),
        },
        BuiltinFunction {
            name: "collections::arrays::remove",
            strategy: BuiltinStrategy::TwoArgBool("naml_array_remove"),
        },
        BuiltinFunction {
            name: "collections::arrays::swap",
            strategy: BuiltinStrategy::ThreeArgVoid("naml_array_swap"),
        },
        // Deduplication
        BuiltinFunction {
            name: "collections::arrays::unique",
            strategy: BuiltinStrategy::OneArgPtr("naml_array_unique"),
        },
        BuiltinFunction {
            name: "collections::arrays::compact",
            strategy: BuiltinStrategy::OneArgPtr("naml_array_compact"),
        },
        // Backward search
        BuiltinFunction {
            name: "collections::arrays::last_index_of",
            strategy: BuiltinStrategy::ArrayLastIndexOf,
        },
        // Array combination
        BuiltinFunction {
            name: "collections::arrays::zip",
            strategy: BuiltinStrategy::TwoArgPtr("naml_array_zip"),
        },
        BuiltinFunction {
            name: "collections::arrays::unzip",
            strategy: BuiltinStrategy::OneArgPtr("naml_array_unzip"),
        },
        // Splitting
        BuiltinFunction {
            name: "collections::arrays::chunk",
            strategy: BuiltinStrategy::TwoArgPtr("naml_array_chunk"),
        },
        // Set operations
        BuiltinFunction {
            name: "collections::arrays::intersect",
            strategy: BuiltinStrategy::TwoArgPtr("naml_array_intersect"),
        },
        BuiltinFunction {
            name: "collections::arrays::diff",
            strategy: BuiltinStrategy::TwoArgPtr("naml_array_diff"),
        },
        BuiltinFunction {
            name: "collections::arrays::union",
            strategy: BuiltinStrategy::TwoArgPtr("naml_array_union"),
        },
        // Random
        BuiltinFunction {
            name: "collections::arrays::shuffle",
            strategy: BuiltinStrategy::OneArgPtr("naml_array_shuffle"),
        },
        BuiltinFunction {
            name: "collections::arrays::sample_n",
            strategy: BuiltinStrategy::TwoArgPtr("naml_array_sample_n"),
        },
        BuiltinFunction {
            name: "collections::arrays::sample",
            strategy: BuiltinStrategy::Sample,
        },
        // Lambda-based collection functions
        BuiltinFunction {
            name: "collections::arrays::any",
            strategy: BuiltinStrategy::LambdaBool("naml_array_any"),
        },
        BuiltinFunction {
            name: "collections::arrays::all",
            strategy: BuiltinStrategy::LambdaBool("naml_array_all"),
        },
        BuiltinFunction {
            name: "collections::arrays::count_if",
            strategy: BuiltinStrategy::LambdaInt("naml_array_count_if"),
        },
        BuiltinFunction {
            name: "collections::arrays::apply",
            strategy: BuiltinStrategy::LambdaArray("naml_array_map"),
        },
        BuiltinFunction {
            name: "collections::arrays::where",
            strategy: BuiltinStrategy::LambdaArray("naml_array_filter"),
        },
        BuiltinFunction {
            name: "collections::arrays::partition",
            strategy: BuiltinStrategy::LambdaArray("naml_array_partition"),
        },
        BuiltinFunction {
            name: "collections::arrays::take_while",
            strategy: BuiltinStrategy::LambdaArray("naml_array_take_while"),
        },
        BuiltinFunction {
            name: "collections::arrays::drop_while",
            strategy: BuiltinStrategy::LambdaArray("naml_array_drop_while"),
        },
        BuiltinFunction {
            name: "collections::arrays::reject",
            strategy: BuiltinStrategy::LambdaArray("naml_array_reject"),
        },
        BuiltinFunction {
            name: "collections::arrays::flat_apply",
            strategy: BuiltinStrategy::LambdaArray("naml_array_flat_apply"),
        },
        BuiltinFunction {
            name: "collections::arrays::find",
            strategy: BuiltinStrategy::LambdaFind,
        },
        BuiltinFunction {
            name: "collections::arrays::find_index",
            strategy: BuiltinStrategy::LambdaFindIndex,
        },
        BuiltinFunction {
            name: "collections::arrays::find_last",
            strategy: BuiltinStrategy::LambdaFindLast,
        },
        BuiltinFunction {
            name: "collections::arrays::find_last_index",
            strategy: BuiltinStrategy::LambdaFindLastIndex,
        },
        BuiltinFunction {
            name: "collections::arrays::fold",
            strategy: BuiltinStrategy::LambdaFold,
        },
        BuiltinFunction {
            name: "collections::arrays::scan",
            strategy: BuiltinStrategy::LambdaScan,
        },
        BuiltinFunction {
            name: "collections::arrays::sort_by",
            strategy: BuiltinStrategy::LambdaSortBy,
        },
        // ========================================
        // Collections module - map operations
        // ========================================
        // Basic operations
        BuiltinFunction {
            name: "collections::maps::count",
            strategy: BuiltinStrategy::MapLength,
        },
        BuiltinFunction {
            name: "collections::maps::contains_key",
            strategy: BuiltinStrategy::MapContainsKey,
        },
        BuiltinFunction {
            name: "collections::maps::remove",
            strategy: BuiltinStrategy::MapRemove,
        },
        BuiltinFunction {
            name: "collections::maps::clear",
            strategy: BuiltinStrategy::MapClear,
        },
        // Extraction
        BuiltinFunction {
            name: "collections::maps::keys",
            strategy: BuiltinStrategy::MapExtract("naml_map_keys"),
        },
        BuiltinFunction {
            name: "collections::maps::values",
            strategy: BuiltinStrategy::MapExtract("naml_map_values"),
        },
        BuiltinFunction {
            name: "collections::maps::entries",
            strategy: BuiltinStrategy::MapEntries,
        },
        // Lookup
        BuiltinFunction {
            name: "collections::maps::first_key",
            strategy: BuiltinStrategy::MapFirstOption("naml_map_first_key"),
        },
        BuiltinFunction {
            name: "collections::maps::first_value",
            strategy: BuiltinStrategy::MapFirstOption("naml_map_first_value"),
        },
        // Lambda-based functions
        BuiltinFunction {
            name: "collections::maps::any",
            strategy: BuiltinStrategy::MapLambdaBool("naml_map_any"),
        },
        BuiltinFunction {
            name: "collections::maps::all",
            strategy: BuiltinStrategy::MapLambdaBool("naml_map_all"),
        },
        BuiltinFunction {
            name: "collections::maps::count_if",
            strategy: BuiltinStrategy::MapLambdaInt("naml_map_count_if"),
        },
        BuiltinFunction {
            name: "collections::maps::fold",
            strategy: BuiltinStrategy::MapLambdaFold,
        },
        // Transformation
        BuiltinFunction {
            name: "collections::maps::transform",
            strategy: BuiltinStrategy::MapLambdaMap("naml_map_transform"),
        },
        BuiltinFunction {
            name: "collections::maps::where",
            strategy: BuiltinStrategy::MapLambdaMap("naml_map_where"),
        },
        BuiltinFunction {
            name: "collections::maps::reject",
            strategy: BuiltinStrategy::MapLambdaMap("naml_map_reject"),
        },
        // Combining
        BuiltinFunction {
            name: "collections::maps::merge",
            strategy: BuiltinStrategy::MapCombine("naml_map_merge"),
        },
        BuiltinFunction {
            name: "collections::maps::defaults",
            strategy: BuiltinStrategy::MapCombine("naml_map_defaults"),
        },
        BuiltinFunction {
            name: "collections::maps::intersect",
            strategy: BuiltinStrategy::MapCombine("naml_map_intersect"),
        },
        BuiltinFunction {
            name: "collections::maps::diff",
            strategy: BuiltinStrategy::MapCombine("naml_map_diff"),
        },
        // Conversion
        BuiltinFunction {
            name: "collections::maps::invert",
            strategy: BuiltinStrategy::MapInvert,
        },
        BuiltinFunction {
            name: "collections::maps::from_arrays",
            strategy: BuiltinStrategy::MapFromArrays,
        },
        BuiltinFunction {
            name: "collections::maps::from_entries",
            strategy: BuiltinStrategy::MapFromEntries,
        },
        // ========================================
        // IO module - core I/O operations
        // ========================================
        BuiltinFunction {
            name: "print",
            strategy: BuiltinStrategy::Print(false),
        },
        BuiltinFunction {
            name: "println",
            strategy: BuiltinStrategy::Print(true),
        },
        BuiltinFunction {
            name: "io::read_line",
            strategy: BuiltinStrategy::ReadLine,
        },
        BuiltinFunction {
            name: "fmt",
            strategy: BuiltinStrategy::Fmt,
        },
        BuiltinFunction {
            name: "warn",
            strategy: BuiltinStrategy::Stderr("warn"),
        },
        BuiltinFunction {
            name: "error",
            strategy: BuiltinStrategy::Stderr("error"),
        },
        BuiltinFunction {
            name: "panic",
            strategy: BuiltinStrategy::Stderr("panic"),
        },
        // ========================================
        // IO module - terminal operations
        // ========================================
        BuiltinFunction {
            name: "io::read_key",
            strategy: BuiltinStrategy::NoArgInt("naml_read_key"),
        },
        BuiltinFunction {
            name: "io::clear_screen",
            strategy: BuiltinStrategy::NoArgVoid("naml_clear_screen"),
        },
        BuiltinFunction {
            name: "io::set_cursor",
            strategy: BuiltinStrategy::TwoArgVoid("naml_set_cursor"),
        },
        BuiltinFunction {
            name: "io::hide_cursor",
            strategy: BuiltinStrategy::NoArgVoid("naml_hide_cursor"),
        },
        BuiltinFunction {
            name: "io::show_cursor",
            strategy: BuiltinStrategy::NoArgVoid("naml_show_cursor"),
        },
        BuiltinFunction {
            name: "io::terminal_width",
            strategy: BuiltinStrategy::NoArgInt("naml_terminal_width"),
        },
        BuiltinFunction {
            name: "io::terminal_height",
            strategy: BuiltinStrategy::NoArgInt("naml_terminal_height"),
        },
        // ========================================
        // Random module
        // ========================================
        BuiltinFunction {
            name: "random::random",
            strategy: BuiltinStrategy::RandomInt,
        },
        BuiltinFunction {
            name: "random::random_float",
            strategy: BuiltinStrategy::RandomFloat,
        },
        // ========================================
        // Datetime module
        // ========================================
        BuiltinFunction {
            name: "datetime::now_ms",
            strategy: BuiltinStrategy::NoArgInt("naml_datetime_now_ms"),
        },
        BuiltinFunction {
            name: "datetime::now_s",
            strategy: BuiltinStrategy::NoArgInt("naml_datetime_now_s"),
        },
        BuiltinFunction {
            name: "datetime::year",
            strategy: BuiltinStrategy::DatetimeOneArgInt("naml_datetime_year"),
        },
        BuiltinFunction {
            name: "datetime::month",
            strategy: BuiltinStrategy::DatetimeOneArgInt("naml_datetime_month"),
        },
        BuiltinFunction {
            name: "datetime::day",
            strategy: BuiltinStrategy::DatetimeOneArgInt("naml_datetime_day"),
        },
        BuiltinFunction {
            name: "datetime::hour",
            strategy: BuiltinStrategy::DatetimeOneArgInt("naml_datetime_hour"),
        },
        BuiltinFunction {
            name: "datetime::minute",
            strategy: BuiltinStrategy::DatetimeOneArgInt("naml_datetime_minute"),
        },
        BuiltinFunction {
            name: "datetime::second",
            strategy: BuiltinStrategy::DatetimeOneArgInt("naml_datetime_second"),
        },
        BuiltinFunction {
            name: "datetime::day_of_week",
            strategy: BuiltinStrategy::DatetimeOneArgInt("naml_datetime_day_of_week"),
        },
        BuiltinFunction {
            name: "datetime::format_date",
            strategy: BuiltinStrategy::DatetimeFormat,
        },
        // ========================================
        // Metrics module
        // ========================================
        BuiltinFunction {
            name: "metrics::perf_now",
            strategy: BuiltinStrategy::NoArgInt("naml_metrics_perf_now"),
        },
        BuiltinFunction {
            name: "metrics::elapsed_ms",
            strategy: BuiltinStrategy::OneArgInt("naml_metrics_elapsed_ms"),
        },
        BuiltinFunction {
            name: "metrics::elapsed_us",
            strategy: BuiltinStrategy::OneArgInt("naml_metrics_elapsed_us"),
        },
        BuiltinFunction {
            name: "metrics::elapsed_ns",
            strategy: BuiltinStrategy::OneArgInt("naml_metrics_elapsed_ns"),
        },
        // ========================================
        // Strings module
        // ========================================
        BuiltinFunction {
            name: "strings::len",
            strategy: BuiltinStrategy::StringOneArgInt("naml_string_char_len"),
        },
        BuiltinFunction {
            name: "strings::char_at",
            strategy: BuiltinStrategy::StringArgIntInt("naml_string_char_at"),
        },
        BuiltinFunction {
            name: "strings::upper",
            strategy: BuiltinStrategy::StringOneArgPtr("naml_string_upper"),
        },
        BuiltinFunction {
            name: "strings::lower",
            strategy: BuiltinStrategy::StringOneArgPtr("naml_string_lower"),
        },
        BuiltinFunction {
            name: "strings::split",
            strategy: BuiltinStrategy::StringTwoArgPtr("naml_string_split"),
        },
        BuiltinFunction {
            name: "strings::concat",
            strategy: BuiltinStrategy::StringJoin,
        },
        BuiltinFunction {
            name: "strings::has",
            strategy: BuiltinStrategy::StringTwoArgBool("naml_string_contains"),
        },
        BuiltinFunction {
            name: "strings::starts_with",
            strategy: BuiltinStrategy::StringTwoArgBool("naml_string_starts_with"),
        },
        BuiltinFunction {
            name: "strings::ends_with",
            strategy: BuiltinStrategy::StringTwoArgBool("naml_string_ends_with"),
        },
        BuiltinFunction {
            name: "strings::replace",
            strategy: BuiltinStrategy::StringThreeArgPtr("naml_string_replace"),
        },
        BuiltinFunction {
            name: "strings::replace_all",
            strategy: BuiltinStrategy::StringThreeArgPtr("naml_string_replace_all"),
        },
        BuiltinFunction {
            name: "strings::ltrim",
            strategy: BuiltinStrategy::StringOneArgPtr("naml_string_ltrim"),
        },
        BuiltinFunction {
            name: "strings::rtrim",
            strategy: BuiltinStrategy::StringOneArgPtr("naml_string_rtrim"),
        },
        BuiltinFunction {
            name: "strings::substr",
            strategy: BuiltinStrategy::StringArgIntIntPtr("naml_string_substr"),
        },
        BuiltinFunction {
            name: "strings::lpad",
            strategy: BuiltinStrategy::StringArgIntStrPtr("naml_string_lpad"),
        },
        BuiltinFunction {
            name: "strings::rpad",
            strategy: BuiltinStrategy::StringArgIntStrPtr("naml_string_rpad"),
        },
        BuiltinFunction {
            name: "strings::repeat",
            strategy: BuiltinStrategy::StringArgIntPtr("naml_string_repeat"),
        },
        BuiltinFunction {
            name: "strings::lines",
            strategy: BuiltinStrategy::StringOneArgPtr("naml_string_lines"),
        },
        BuiltinFunction {
            name: "strings::chars",
            strategy: BuiltinStrategy::StringOneArgPtr("naml_string_chars"),
        },
        // ========================================
        // Threads/Channel module
        // ========================================
        BuiltinFunction {
            name: "threads::sleep",
            strategy: BuiltinStrategy::Sleep,
        },
        BuiltinFunction {
            name: "threads::join",
            strategy: BuiltinStrategy::ThreadsJoin,
        },
        BuiltinFunction {
            name: "threads::open_channel",
            strategy: BuiltinStrategy::ChannelOpen,
        },
        BuiltinFunction {
            name: "threads::send",
            strategy: BuiltinStrategy::ChannelSend,
        },
        BuiltinFunction {
            name: "threads::receive",
            strategy: BuiltinStrategy::ChannelReceive,
        },
        BuiltinFunction {
            name: "threads::close",
            strategy: BuiltinStrategy::ChannelClose,
        },
        BuiltinFunction {
            name: "threads::with_mutex",
            strategy: BuiltinStrategy::MutexNew,
        },
        BuiltinFunction {
            name: "threads::with_rwlock",
            strategy: BuiltinStrategy::RwlockNew,
        },
        BuiltinFunction {
            name: "threads::with_atomic",
            strategy: BuiltinStrategy::AtomicNew,
        },
        BuiltinFunction {
            name: "threads::atomic_load",
            strategy: BuiltinStrategy::AtomicLoad,
        },
        BuiltinFunction {
            name: "threads::atomic_store",
            strategy: BuiltinStrategy::AtomicStore,
        },
        BuiltinFunction {
            name: "threads::atomic_add",
            strategy: BuiltinStrategy::AtomicAdd,
        },
        BuiltinFunction {
            name: "threads::atomic_sub",
            strategy: BuiltinStrategy::AtomicSub,
        },
        BuiltinFunction {
            name: "threads::atomic_inc",
            strategy: BuiltinStrategy::AtomicInc,
        },
        BuiltinFunction {
            name: "threads::atomic_dec",
            strategy: BuiltinStrategy::AtomicDec,
        },
        BuiltinFunction {
            name: "threads::atomic_cas",
            strategy: BuiltinStrategy::AtomicCas,
        },
        BuiltinFunction {
            name: "threads::atomic_swap",
            strategy: BuiltinStrategy::AtomicSwap,
        },
        BuiltinFunction {
            name: "threads::atomic_and",
            strategy: BuiltinStrategy::AtomicAnd,
        },
        BuiltinFunction {
            name: "threads::atomic_or",
            strategy: BuiltinStrategy::AtomicOr,
        },
        BuiltinFunction {
            name: "threads::atomic_xor",
            strategy: BuiltinStrategy::AtomicXor,
        },
        // ========================================
        // File system module
        // ========================================
        BuiltinFunction {
            name: "fs::read",
            strategy: BuiltinStrategy::FsRead,
        },
        BuiltinFunction {
            name: "fs::read_bytes",
            strategy: BuiltinStrategy::FsReadBytes,
        },
        BuiltinFunction {
            name: "fs::write",
            strategy: BuiltinStrategy::FsWrite,
        },
        BuiltinFunction {
            name: "fs::append",
            strategy: BuiltinStrategy::FsAppend,
        },
        BuiltinFunction {
            name: "fs::write_bytes",
            strategy: BuiltinStrategy::FsWriteBytes,
        },
        BuiltinFunction {
            name: "fs::append_bytes",
            strategy: BuiltinStrategy::FsAppendBytes,
        },
        BuiltinFunction {
            name: "fs::exists",
            strategy: BuiltinStrategy::FsExists,
        },
        BuiltinFunction {
            name: "fs::is_file",
            strategy: BuiltinStrategy::FsIsFile,
        },
        BuiltinFunction {
            name: "fs::is_dir",
            strategy: BuiltinStrategy::FsIsDir,
        },
        BuiltinFunction {
            name: "fs::list_dir",
            strategy: BuiltinStrategy::FsListDir,
        },
        BuiltinFunction {
            name: "fs::mkdir",
            strategy: BuiltinStrategy::FsMkdir,
        },
        BuiltinFunction {
            name: "fs::mkdir_all",
            strategy: BuiltinStrategy::FsMkdirAll,
        },
        BuiltinFunction {
            name: "fs::remove",
            strategy: BuiltinStrategy::FsRemove,
        },
        BuiltinFunction {
            name: "fs::remove_all",
            strategy: BuiltinStrategy::FsRemoveAll,
        },
        BuiltinFunction {
            name: "fs::join",
            strategy: BuiltinStrategy::FsJoin,
        },
        BuiltinFunction {
            name: "fs::dirname",
            strategy: BuiltinStrategy::FsDirname,
        },
        BuiltinFunction {
            name: "fs::basename",
            strategy: BuiltinStrategy::FsBasename,
        },
        BuiltinFunction {
            name: "fs::extension",
            strategy: BuiltinStrategy::FsExtension,
        },
        BuiltinFunction {
            name: "fs::absolute",
            strategy: BuiltinStrategy::FsAbsolute,
        },
        BuiltinFunction {
            name: "fs::size",
            strategy: BuiltinStrategy::FsSize,
        },
        BuiltinFunction {
            name: "fs::modified",
            strategy: BuiltinStrategy::FsModified,
        },
        BuiltinFunction {
            name: "fs::copy",
            strategy: BuiltinStrategy::FsCopy,
        },
        BuiltinFunction {
            name: "fs::rename",
            strategy: BuiltinStrategy::FsRename,
        },
        BuiltinFunction {
            name: "fs::getwd",
            strategy: BuiltinStrategy::FsGetwd,
        },
        BuiltinFunction {
            name: "fs::chdir",
            strategy: BuiltinStrategy::FsChdir,
        },
        BuiltinFunction {
            name: "fs::create_temp",
            strategy: BuiltinStrategy::FsCreateTemp,
        },
        BuiltinFunction {
            name: "fs::mkdir_temp",
            strategy: BuiltinStrategy::FsMkdirTemp,
        },
        BuiltinFunction {
            name: "fs::chmod",
            strategy: BuiltinStrategy::FsChmod,
        },
        BuiltinFunction {
            name: "fs::truncate",
            strategy: BuiltinStrategy::FsTruncate,
        },
        BuiltinFunction {
            name: "fs::stat",
            strategy: BuiltinStrategy::FsStat,
        },
        // ========================================
        // Memory-mapped file operations
        // ========================================
        BuiltinFunction {
            name: "fs::mmap_open",
            strategy: BuiltinStrategy::FsMmapOpen,
        },
        BuiltinFunction {
            name: "fs::mmap_len",
            strategy: BuiltinStrategy::FsMmapLen,
        },
        BuiltinFunction {
            name: "fs::mmap_read_byte",
            strategy: BuiltinStrategy::FsMmapReadByte,
        },
        BuiltinFunction {
            name: "fs::mmap_write_byte",
            strategy: BuiltinStrategy::FsMmapWriteByte,
        },
        BuiltinFunction {
            name: "fs::mmap_read",
            strategy: BuiltinStrategy::FsMmapRead,
        },
        BuiltinFunction {
            name: "fs::mmap_write",
            strategy: BuiltinStrategy::FsMmapWrite,
        },
        BuiltinFunction {
            name: "fs::mmap_flush",
            strategy: BuiltinStrategy::FsMmapFlush,
        },
        BuiltinFunction {
            name: "fs::mmap_close",
            strategy: BuiltinStrategy::FsMmapClose,
        },
        // ========================================
        // File handle operations
        // ========================================
        BuiltinFunction {
            name: "fs::file_open",
            strategy: BuiltinStrategy::FsFileOpen,
        },
        BuiltinFunction {
            name: "fs::file_close",
            strategy: BuiltinStrategy::FsFileClose,
        },
        BuiltinFunction {
            name: "fs::file_read",
            strategy: BuiltinStrategy::FsFileRead,
        },
        BuiltinFunction {
            name: "fs::file_read_line",
            strategy: BuiltinStrategy::FsFileReadLine,
        },
        BuiltinFunction {
            name: "fs::file_read_all",
            strategy: BuiltinStrategy::FsFileReadAll,
        },
        BuiltinFunction {
            name: "fs::file_write",
            strategy: BuiltinStrategy::FsFileWrite,
        },
        BuiltinFunction {
            name: "fs::file_write_line",
            strategy: BuiltinStrategy::FsFileWriteLine,
        },
        BuiltinFunction {
            name: "fs::file_flush",
            strategy: BuiltinStrategy::FsFileFlush,
        },
        BuiltinFunction {
            name: "fs::file_seek",
            strategy: BuiltinStrategy::FsFileSeek,
        },
        BuiltinFunction {
            name: "fs::file_tell",
            strategy: BuiltinStrategy::FsFileTell,
        },
        BuiltinFunction {
            name: "fs::file_eof",
            strategy: BuiltinStrategy::FsFileEof,
        },
        BuiltinFunction {
            name: "fs::file_size",
            strategy: BuiltinStrategy::FsFileSize,
        },
        // ========================================
        // Link/symlink operations
        // ========================================
        BuiltinFunction {
            name: "fs::symlink",
            strategy: BuiltinStrategy::FsSymlink,
        },
        BuiltinFunction {
            name: "fs::readlink",
            strategy: BuiltinStrategy::FsReadlink,
        },
        BuiltinFunction {
            name: "fs::lstat",
            strategy: BuiltinStrategy::FsLstat,
        },
        BuiltinFunction {
            name: "fs::link",
            strategy: BuiltinStrategy::FsLink,
        },
        BuiltinFunction {
            name: "fs::chtimes",
            strategy: BuiltinStrategy::FsChtimes,
        },
        BuiltinFunction {
            name: "fs::chown",
            strategy: BuiltinStrategy::FsChown,
        },
        BuiltinFunction {
            name: "fs::lchown",
            strategy: BuiltinStrategy::FsLchown,
        },
        BuiltinFunction {
            name: "fs::same_file",
            strategy: BuiltinStrategy::FsSameFile,
        },
        // ========================================
        // Additional file handle operations
        // ========================================
        BuiltinFunction {
            name: "fs::file_read_at",
            strategy: BuiltinStrategy::FsFileReadAt,
        },
        BuiltinFunction {
            name: "fs::file_write_at",
            strategy: BuiltinStrategy::FsFileWriteAt,
        },
        BuiltinFunction {
            name: "fs::file_name",
            strategy: BuiltinStrategy::FsFileName,
        },
        BuiltinFunction {
            name: "fs::file_stat",
            strategy: BuiltinStrategy::FsFileStat,
        },
        BuiltinFunction {
            name: "fs::file_truncate",
            strategy: BuiltinStrategy::FsFileTruncate,
        },
        BuiltinFunction {
            name: "fs::file_chmod",
            strategy: BuiltinStrategy::FsFileChmod,
        },
        BuiltinFunction {
            name: "fs::file_chown",
            strategy: BuiltinStrategy::FsFileChown,
        },
        // ========================================
        // Path module
        // ========================================
        // Note: path::join conflicts with threads::join, so needs qualified call
        BuiltinFunction {
            name: "path::join",
            strategy: BuiltinStrategy::PathJoin,
        },
        BuiltinFunction {
            name: "path::normalize",
            strategy: BuiltinStrategy::PathOneArgStr("naml_path_normalize"),
        },
        BuiltinFunction {
            name: "path::dirname",
            strategy: BuiltinStrategy::PathOneArgStr("naml_path_dirname"),
        },
        BuiltinFunction {
            name: "path::basename",
            strategy: BuiltinStrategy::PathOneArgStr("naml_path_basename"),
        },
        BuiltinFunction {
            name: "path::extension",
            strategy: BuiltinStrategy::PathOneArgStr("naml_path_extension"),
        },
        BuiltinFunction {
            name: "path::stem",
            strategy: BuiltinStrategy::PathOneArgStr("naml_path_stem"),
        },
        BuiltinFunction {
            name: "path::to_slash",
            strategy: BuiltinStrategy::PathOneArgStr("naml_path_to_slash"),
        },
        BuiltinFunction {
            name: "path::from_slash",
            strategy: BuiltinStrategy::PathOneArgStr("naml_path_from_slash"),
        },
        BuiltinFunction {
            name: "path::is_absolute",
            strategy: BuiltinStrategy::PathOneArgBool("naml_path_is_absolute"),
        },
        BuiltinFunction {
            name: "path::is_relative",
            strategy: BuiltinStrategy::PathOneArgBool("naml_path_is_relative"),
        },
        BuiltinFunction {
            name: "path::has_root",
            strategy: BuiltinStrategy::PathOneArgBool("naml_path_has_root"),
        },
        BuiltinFunction {
            name: "path::with_extension",
            strategy: BuiltinStrategy::PathTwoArgStr("naml_path_with_extension"),
        },
        BuiltinFunction {
            name: "path::strip_prefix",
            strategy: BuiltinStrategy::PathTwoArgStr("naml_path_strip_prefix"),
        },
        BuiltinFunction {
            name: "path::starts_with",
            strategy: BuiltinStrategy::PathTwoArgBool("naml_path_starts_with"),
        },
        BuiltinFunction {
            name: "path::ends_with",
            strategy: BuiltinStrategy::PathTwoArgBool("naml_path_ends_with"),
        },
        BuiltinFunction {
            name: "path::components",
            strategy: BuiltinStrategy::PathComponents,
        },
        BuiltinFunction {
            name: "path::separator",
            strategy: BuiltinStrategy::PathSeparator,
        },
        // ========================================
        // Env module
        // ========================================
        BuiltinFunction {
            name: "env::getenv",
            strategy: BuiltinStrategy::EnvGetenv,
        },
        BuiltinFunction {
            name: "env::lookup_env",
            strategy: BuiltinStrategy::EnvLookupEnv,
        },
        BuiltinFunction {
            name: "env::setenv",
            strategy: BuiltinStrategy::EnvSetenv,
        },
        BuiltinFunction {
            name: "env::unsetenv",
            strategy: BuiltinStrategy::EnvUnsetenv,
        },
        BuiltinFunction {
            name: "env::clearenv",
            strategy: BuiltinStrategy::EnvClearenv,
        },
        BuiltinFunction {
            name: "env::environ",
            strategy: BuiltinStrategy::EnvEnviron,
        },
        BuiltinFunction {
            name: "env::expand_env",
            strategy: BuiltinStrategy::EnvExpandEnv,
        },
        // ========================================
        // OS module
        // ========================================
        BuiltinFunction {
            name: "os::hostname",
            strategy: BuiltinStrategy::OsHostname,
        },
        BuiltinFunction {
            name: "os::temp_dir",
            strategy: BuiltinStrategy::OsTempDir,
        },
        BuiltinFunction {
            name: "os::home_dir",
            strategy: BuiltinStrategy::OsHomeDir,
        },
        BuiltinFunction {
            name: "os::cache_dir",
            strategy: BuiltinStrategy::OsCacheDir,
        },
        BuiltinFunction {
            name: "os::config_dir",
            strategy: BuiltinStrategy::OsConfigDir,
        },
        BuiltinFunction {
            name: "os::executable",
            strategy: BuiltinStrategy::OsExecutable,
        },
        BuiltinFunction {
            name: "os::pagesize",
            strategy: BuiltinStrategy::OsPagesize,
        },
        BuiltinFunction {
            name: "os::getuid",
            strategy: BuiltinStrategy::OsGetuid,
        },
        BuiltinFunction {
            name: "os::geteuid",
            strategy: BuiltinStrategy::OsGeteuid,
        },
        BuiltinFunction {
            name: "os::getgid",
            strategy: BuiltinStrategy::OsGetgid,
        },
        BuiltinFunction {
            name: "os::getegid",
            strategy: BuiltinStrategy::OsGetegid,
        },
        BuiltinFunction {
            name: "os::getgroups",
            strategy: BuiltinStrategy::OsGetgroups,
        },
        // ========================================
        // Process module
        // ========================================
        BuiltinFunction {
            name: "process::getpid",
            strategy: BuiltinStrategy::ProcessGetpid,
        },
        BuiltinFunction {
            name: "process::getppid",
            strategy: BuiltinStrategy::ProcessGetppid,
        },
        BuiltinFunction {
            name: "process::exit",
            strategy: BuiltinStrategy::ProcessExit,
        },
        BuiltinFunction {
            name: "process::pipe_read",
            strategy: BuiltinStrategy::ProcessPipeRead,
        },
        BuiltinFunction {
            name: "process::pipe_write",
            strategy: BuiltinStrategy::ProcessPipeWrite,
        },
        BuiltinFunction {
            name: "process::start_process",
            strategy: BuiltinStrategy::ProcessStart,
        },
        BuiltinFunction {
            name: "process::find_process",
            strategy: BuiltinStrategy::ProcessFind,
        },
        BuiltinFunction {
            name: "process::wait",
            strategy: BuiltinStrategy::ProcessWait,
        },
        BuiltinFunction {
            name: "process::signal",
            strategy: BuiltinStrategy::ProcessSignal,
        },
        BuiltinFunction {
            name: "process::kill",
            strategy: BuiltinStrategy::ProcessKill,
        },
        BuiltinFunction {
            name: "process::release",
            strategy: BuiltinStrategy::ProcessRelease,
        },
        BuiltinFunction {
            name: "process::SIGHUP",
            strategy: BuiltinStrategy::ProcessSighup,
        },
        BuiltinFunction {
            name: "process::SIGINT",
            strategy: BuiltinStrategy::ProcessSigint,
        },
        BuiltinFunction {
            name: "process::SIGQUIT",
            strategy: BuiltinStrategy::ProcessSigquit,
        },
        BuiltinFunction {
            name: "process::SIGKILL",
            strategy: BuiltinStrategy::ProcessSigkill,
        },
        BuiltinFunction {
            name: "process::SIGTERM",
            strategy: BuiltinStrategy::ProcessSigterm,
        },
        BuiltinFunction {
            name: "process::SIGSTOP",
            strategy: BuiltinStrategy::ProcessSigstop,
        },
        BuiltinFunction {
            name: "process::SIGCONT",
            strategy: BuiltinStrategy::ProcessSigcont,
        },
        // ========================================
        // Testing module
        // ========================================
        BuiltinFunction {
            name: "testing::assert",
            strategy: BuiltinStrategy::TestingAssert,
        },
        BuiltinFunction {
            name: "testing::assert_eq",
            strategy: BuiltinStrategy::TestingAssertEq,
        },
        BuiltinFunction {
            name: "testing::assert_eq_float",
            strategy: BuiltinStrategy::TestingAssertEqFloat,
        },
        BuiltinFunction {
            name: "testing::assert_eq_string",
            strategy: BuiltinStrategy::TestingAssertEqString,
        },
        BuiltinFunction {
            name: "testing::assert_eq_bool",
            strategy: BuiltinStrategy::TestingAssertEqBool,
        },
        BuiltinFunction {
            name: "testing::assert_neq",
            strategy: BuiltinStrategy::TestingAssertNeq,
        },
        BuiltinFunction {
            name: "testing::assert_neq_string",
            strategy: BuiltinStrategy::TestingAssertNeqString,
        },
        BuiltinFunction {
            name: "testing::assert_true",
            strategy: BuiltinStrategy::TestingAssertTrue,
        },
        BuiltinFunction {
            name: "testing::assert_false",
            strategy: BuiltinStrategy::TestingAssertFalse,
        },
        BuiltinFunction {
            name: "testing::assert_gt",
            strategy: BuiltinStrategy::TestingAssertGt,
        },
        BuiltinFunction {
            name: "testing::assert_gte",
            strategy: BuiltinStrategy::TestingAssertGte,
        },
        BuiltinFunction {
            name: "testing::assert_lt",
            strategy: BuiltinStrategy::TestingAssertLt,
        },
        BuiltinFunction {
            name: "testing::assert_lte",
            strategy: BuiltinStrategy::TestingAssertLte,
        },
        BuiltinFunction {
            name: "testing::fail",
            strategy: BuiltinStrategy::TestingFail,
        },
        BuiltinFunction {
            name: "testing::assert_approx",
            strategy: BuiltinStrategy::TestingAssertApprox,
        },
        BuiltinFunction {
            name: "testing::assert_contains",
            strategy: BuiltinStrategy::TestingAssertContains,
        },
        BuiltinFunction {
            name: "testing::assert_starts_with",
            strategy: BuiltinStrategy::TestingAssertStartsWith,
        },
        BuiltinFunction {
            name: "testing::assert_ends_with",
            strategy: BuiltinStrategy::TestingAssertEndsWith,
        },
        // ========================================
        // Encoding module
        // ========================================
        // UTF-8
        BuiltinFunction {
            name: "utf8::encode",
            strategy: BuiltinStrategy::EncodingStringToBytes("naml_encoding_utf8_encode"),
        },
        BuiltinFunction {
            name: "utf8::decode",
            strategy: BuiltinStrategy::EncodingDecodeToString("naml_encoding_utf8_decode"),
        },
        BuiltinFunction {
            name: "utf8::is_valid",
            strategy: BuiltinStrategy::EncodingValidate("naml_encoding_utf8_is_valid"),
        },
        // Hex
        BuiltinFunction {
            name: "encoding::hex::encode",
            strategy: BuiltinStrategy::EncodingBytesToString("naml_encoding_hex_encode"),
        },
        BuiltinFunction {
            name: "encoding::hex::decode",
            strategy: BuiltinStrategy::EncodingDecodeToBytes("naml_encoding_hex_decode"),
        },
        // Base64
        BuiltinFunction {
            name: "base64::encode",
            strategy: BuiltinStrategy::EncodingBytesToString("naml_encoding_base64_encode"),
        },
        BuiltinFunction {
            name: "base64::decode",
            strategy: BuiltinStrategy::EncodingDecodeToBytes("naml_encoding_base64_decode"),
        },
        // URL
        BuiltinFunction {
            name: "encoding::url::encode",
            strategy: BuiltinStrategy::EncodingStringToBytes("naml_encoding_url_encode"),
        },
        BuiltinFunction {
            name: "encoding::url::decode",
            strategy: BuiltinStrategy::EncodingDecodeToString("naml_encoding_url_decode"),
        },
        // JSON
        BuiltinFunction {
            name: "encoding::json::decode",
            strategy: BuiltinStrategy::JsonDecode,
        },
        BuiltinFunction {
            name: "encoding::json::encode",
            strategy: BuiltinStrategy::JsonEncode("naml_json_encode"),
        },
        BuiltinFunction {
            name: "encoding::json::encode_pretty",
            strategy: BuiltinStrategy::JsonEncode("naml_json_encode_pretty"),
        },
        BuiltinFunction {
            name: "encoding::json::exists",
            strategy: BuiltinStrategy::JsonExists,
        },
        BuiltinFunction {
            name: "encoding::json::path",
            strategy: BuiltinStrategy::JsonPath,
        },
        BuiltinFunction {
            name: "encoding::json::keys",
            strategy: BuiltinStrategy::JsonKeys,
        },
        BuiltinFunction {
            name: "encoding::json::count",
            strategy: BuiltinStrategy::JsonCount,
        },
        BuiltinFunction {
            name: "encoding::json::get_type",
            strategy: BuiltinStrategy::JsonGetType,
        },
        BuiltinFunction {
            name: "encoding::json::type_name",
            strategy: BuiltinStrategy::JsonTypeName,
        },
        BuiltinFunction {
            name: "encoding::json::is_null",
            strategy: BuiltinStrategy::JsonIsNull,
        },
        // ========================================
        // TOML encoding module
        // ========================================
        BuiltinFunction {
            name: "encoding::toml::decode",
            strategy: BuiltinStrategy::TomlDecode,
        },
        BuiltinFunction {
            name: "encoding::toml::encode",
            strategy: BuiltinStrategy::TomlEncode("naml_encoding_toml_encode"),
        },
        BuiltinFunction {
            name: "encoding::toml::encode_pretty",
            strategy: BuiltinStrategy::TomlEncode("naml_encoding_toml_encode_pretty"),
        },
        // ========================================
        // YAML encoding module
        // ========================================
        BuiltinFunction {
            name: "encoding::yaml::decode",
            strategy: BuiltinStrategy::YamlDecode,
        },
        BuiltinFunction {
            name: "encoding::yaml::encode",
            strategy: BuiltinStrategy::YamlEncode,
        },
        // ========================================
        // Binary encoding module
        // ========================================
        BuiltinFunction { name: "encoding::binary::read_u8", strategy: BuiltinStrategy::BinaryTwoArgCall("naml_encoding_binary_read_u8") },
        BuiltinFunction { name: "encoding::binary::read_i8", strategy: BuiltinStrategy::BinaryTwoArgCall("naml_encoding_binary_read_i8") },
        BuiltinFunction { name: "encoding::binary::read_u16_be", strategy: BuiltinStrategy::BinaryTwoArgCall("naml_encoding_binary_read_u16_be") },
        BuiltinFunction { name: "encoding::binary::read_u16_le", strategy: BuiltinStrategy::BinaryTwoArgCall("naml_encoding_binary_read_u16_le") },
        BuiltinFunction { name: "encoding::binary::read_i16_be", strategy: BuiltinStrategy::BinaryTwoArgCall("naml_encoding_binary_read_i16_be") },
        BuiltinFunction { name: "encoding::binary::read_i16_le", strategy: BuiltinStrategy::BinaryTwoArgCall("naml_encoding_binary_read_i16_le") },
        BuiltinFunction { name: "encoding::binary::read_u32_be", strategy: BuiltinStrategy::BinaryTwoArgCall("naml_encoding_binary_read_u32_be") },
        BuiltinFunction { name: "encoding::binary::read_u32_le", strategy: BuiltinStrategy::BinaryTwoArgCall("naml_encoding_binary_read_u32_le") },
        BuiltinFunction { name: "encoding::binary::read_i32_be", strategy: BuiltinStrategy::BinaryTwoArgCall("naml_encoding_binary_read_i32_be") },
        BuiltinFunction { name: "encoding::binary::read_i32_le", strategy: BuiltinStrategy::BinaryTwoArgCall("naml_encoding_binary_read_i32_le") },
        BuiltinFunction { name: "encoding::binary::read_u64_be", strategy: BuiltinStrategy::BinaryTwoArgCall("naml_encoding_binary_read_u64_be") },
        BuiltinFunction { name: "encoding::binary::read_u64_le", strategy: BuiltinStrategy::BinaryTwoArgCall("naml_encoding_binary_read_u64_le") },
        BuiltinFunction { name: "encoding::binary::read_i64_be", strategy: BuiltinStrategy::BinaryTwoArgCall("naml_encoding_binary_read_i64_be") },
        BuiltinFunction { name: "encoding::binary::read_i64_le", strategy: BuiltinStrategy::BinaryTwoArgCall("naml_encoding_binary_read_i64_le") },
        BuiltinFunction { name: "encoding::binary::read_f32_be", strategy: BuiltinStrategy::BinaryReadFloat("naml_encoding_binary_read_f32_be") },
        BuiltinFunction { name: "encoding::binary::read_f32_le", strategy: BuiltinStrategy::BinaryReadFloat("naml_encoding_binary_read_f32_le") },
        BuiltinFunction { name: "encoding::binary::read_f64_be", strategy: BuiltinStrategy::BinaryReadFloat("naml_encoding_binary_read_f64_be") },
        BuiltinFunction { name: "encoding::binary::read_f64_le", strategy: BuiltinStrategy::BinaryReadFloat("naml_encoding_binary_read_f64_le") },
        BuiltinFunction { name: "encoding::binary::write_u8", strategy: BuiltinStrategy::BinaryThreeArgVoid("naml_encoding_binary_write_u8") },
        BuiltinFunction { name: "encoding::binary::write_i8", strategy: BuiltinStrategy::BinaryThreeArgVoid("naml_encoding_binary_write_i8") },
        BuiltinFunction { name: "encoding::binary::write_u16_be", strategy: BuiltinStrategy::BinaryThreeArgVoid("naml_encoding_binary_write_u16_be") },
        BuiltinFunction { name: "encoding::binary::write_u16_le", strategy: BuiltinStrategy::BinaryThreeArgVoid("naml_encoding_binary_write_u16_le") },
        BuiltinFunction { name: "encoding::binary::write_i16_be", strategy: BuiltinStrategy::BinaryThreeArgVoid("naml_encoding_binary_write_i16_be") },
        BuiltinFunction { name: "encoding::binary::write_i16_le", strategy: BuiltinStrategy::BinaryThreeArgVoid("naml_encoding_binary_write_i16_le") },
        BuiltinFunction { name: "encoding::binary::write_u32_be", strategy: BuiltinStrategy::BinaryThreeArgVoid("naml_encoding_binary_write_u32_be") },
        BuiltinFunction { name: "encoding::binary::write_u32_le", strategy: BuiltinStrategy::BinaryThreeArgVoid("naml_encoding_binary_write_u32_le") },
        BuiltinFunction { name: "encoding::binary::write_i32_be", strategy: BuiltinStrategy::BinaryThreeArgVoid("naml_encoding_binary_write_i32_be") },
        BuiltinFunction { name: "encoding::binary::write_i32_le", strategy: BuiltinStrategy::BinaryThreeArgVoid("naml_encoding_binary_write_i32_le") },
        BuiltinFunction { name: "encoding::binary::write_u64_be", strategy: BuiltinStrategy::BinaryThreeArgVoid("naml_encoding_binary_write_u64_be") },
        BuiltinFunction { name: "encoding::binary::write_u64_le", strategy: BuiltinStrategy::BinaryThreeArgVoid("naml_encoding_binary_write_u64_le") },
        BuiltinFunction { name: "encoding::binary::write_i64_be", strategy: BuiltinStrategy::BinaryThreeArgVoid("naml_encoding_binary_write_i64_be") },
        BuiltinFunction { name: "encoding::binary::write_i64_le", strategy: BuiltinStrategy::BinaryThreeArgVoid("naml_encoding_binary_write_i64_le") },
        BuiltinFunction { name: "encoding::binary::write_f32_be", strategy: BuiltinStrategy::BinaryWriteFloat("naml_encoding_binary_write_f32_be") },
        BuiltinFunction { name: "encoding::binary::write_f32_le", strategy: BuiltinStrategy::BinaryWriteFloat("naml_encoding_binary_write_f32_le") },
        BuiltinFunction { name: "encoding::binary::write_f64_be", strategy: BuiltinStrategy::BinaryWriteFloat("naml_encoding_binary_write_f64_be") },
        BuiltinFunction { name: "encoding::binary::write_f64_le", strategy: BuiltinStrategy::BinaryWriteFloat("naml_encoding_binary_write_f64_le") },
        BuiltinFunction { name: "encoding::binary::alloc", strategy: BuiltinStrategy::BinaryOneArgCall("naml_encoding_binary_alloc") },
        BuiltinFunction { name: "encoding::binary::from_string", strategy: BuiltinStrategy::EncodingStringToBytes("naml_encoding_binary_from_string") },
        BuiltinFunction { name: "encoding::binary::len", strategy: BuiltinStrategy::BinaryOneArgCall("naml_encoding_binary_len") },
        BuiltinFunction { name: "encoding::binary::capacity", strategy: BuiltinStrategy::BinaryOneArgCall("naml_encoding_binary_capacity") },
        BuiltinFunction { name: "encoding::binary::slice", strategy: BuiltinStrategy::BinaryThreeArgCall("naml_encoding_binary_slice") },
        BuiltinFunction { name: "encoding::binary::concat", strategy: BuiltinStrategy::BinaryTwoArgCall("naml_encoding_binary_concat") },
        BuiltinFunction { name: "encoding::binary::append", strategy: BuiltinStrategy::BinaryTwoArgVoid("naml_encoding_binary_append") },
        BuiltinFunction { name: "encoding::binary::copy_within", strategy: BuiltinStrategy::BinaryFourArgVoid("naml_encoding_binary_copy_within") },
        BuiltinFunction { name: "encoding::binary::clear", strategy: BuiltinStrategy::BinaryOneArgVoid("naml_encoding_binary_clear") },
        BuiltinFunction { name: "encoding::binary::resize", strategy: BuiltinStrategy::BinaryTwoArgVoid("naml_encoding_binary_resize") },
        BuiltinFunction { name: "encoding::binary::fill", strategy: BuiltinStrategy::BinaryTwoArgVoid("naml_encoding_binary_fill") },
        BuiltinFunction { name: "encoding::binary::index_of", strategy: BuiltinStrategy::BinaryTwoArgCall("naml_encoding_binary_index_of") },
        BuiltinFunction { name: "encoding::binary::contains", strategy: BuiltinStrategy::BinaryTwoArgBool("naml_encoding_binary_contains") },
        BuiltinFunction { name: "encoding::binary::starts_with", strategy: BuiltinStrategy::BinaryTwoArgBool("naml_encoding_binary_starts_with") },
        BuiltinFunction { name: "encoding::binary::ends_with", strategy: BuiltinStrategy::BinaryTwoArgBool("naml_encoding_binary_ends_with") },
        BuiltinFunction { name: "encoding::binary::equals", strategy: BuiltinStrategy::BinaryTwoArgBool("naml_encoding_binary_equals") },
        // ========================================
        // Crypto module
        // ========================================
        BuiltinFunction { name: "crypto::md5", strategy: BuiltinStrategy::CryptoHashBytes("naml_crypto_md5") },
        BuiltinFunction { name: "crypto::md5_hex", strategy: BuiltinStrategy::CryptoHashHex("naml_crypto_md5_hex") },
        BuiltinFunction { name: "crypto::sha1", strategy: BuiltinStrategy::CryptoHashBytes("naml_crypto_sha1") },
        BuiltinFunction { name: "crypto::sha1_hex", strategy: BuiltinStrategy::CryptoHashHex("naml_crypto_sha1_hex") },
        BuiltinFunction { name: "crypto::sha256", strategy: BuiltinStrategy::CryptoHashBytes("naml_crypto_sha256") },
        BuiltinFunction { name: "crypto::sha256_hex", strategy: BuiltinStrategy::CryptoHashHex("naml_crypto_sha256_hex") },
        BuiltinFunction { name: "crypto::sha512", strategy: BuiltinStrategy::CryptoHashBytes("naml_crypto_sha512") },
        BuiltinFunction { name: "crypto::sha512_hex", strategy: BuiltinStrategy::CryptoHashHex("naml_crypto_sha512_hex") },
        BuiltinFunction { name: "crypto::hmac_sha256", strategy: BuiltinStrategy::CryptoHmacBytes("naml_crypto_hmac_sha256") },
        BuiltinFunction { name: "crypto::hmac_sha256_hex", strategy: BuiltinStrategy::CryptoHmacHex("naml_crypto_hmac_sha256_hex") },
        BuiltinFunction { name: "crypto::hmac_sha512", strategy: BuiltinStrategy::CryptoHmacBytes("naml_crypto_hmac_sha512") },
        BuiltinFunction { name: "crypto::hmac_sha512_hex", strategy: BuiltinStrategy::CryptoHmacHex("naml_crypto_hmac_sha512_hex") },
        BuiltinFunction { name: "crypto::hmac_verify_sha256", strategy: BuiltinStrategy::CryptoHmacVerify("naml_crypto_hmac_verify_sha256") },
        BuiltinFunction { name: "crypto::hmac_verify_sha512", strategy: BuiltinStrategy::CryptoHmacVerify("naml_crypto_hmac_verify_sha512") },
        BuiltinFunction { name: "crypto::pbkdf2_sha256", strategy: BuiltinStrategy::CryptoPbkdf2("naml_crypto_pbkdf2_sha256") },
        BuiltinFunction { name: "crypto::random_bytes", strategy: BuiltinStrategy::CryptoRandomBytes("naml_crypto_random_bytes") },
        // ========================================
        // Networking module (strict hierarchy: net::tcp::server, net::tcp::client, etc.)
        // ========================================
        // TCP Server
        BuiltinFunction {
            name: "net::tcp::server::listen",
            strategy: BuiltinStrategy::NetTcpListen,
        },
        BuiltinFunction {
            name: "net::tcp::server::accept",
            strategy: BuiltinStrategy::NetTcpAccept,
        },
        BuiltinFunction {
            name: "net::tcp::server::close",
            strategy: BuiltinStrategy::NetTcpServerClose,
        },
        BuiltinFunction {
            name: "net::tcp::server::local_addr",
            strategy: BuiltinStrategy::NetTcpServerLocalAddr,
        },
        // TCP Client
        BuiltinFunction {
            name: "net::tcp::client::connect",
            strategy: BuiltinStrategy::NetTcpConnect,
        },
        BuiltinFunction {
            name: "net::tcp::client::read",
            strategy: BuiltinStrategy::NetTcpRead,
        },
        BuiltinFunction {
            name: "net::tcp::client::read_all",
            strategy: BuiltinStrategy::NetTcpReadAll,
        },
        BuiltinFunction {
            name: "net::tcp::client::write",
            strategy: BuiltinStrategy::NetTcpWrite,
        },
        BuiltinFunction {
            name: "net::tcp::client::set_timeout",
            strategy: BuiltinStrategy::NetTcpSetTimeout,
        },
        BuiltinFunction {
            name: "net::tcp::client::peer_addr",
            strategy: BuiltinStrategy::NetTcpPeerAddr,
        },
        BuiltinFunction {
            name: "net::tcp::client::close",
            strategy: BuiltinStrategy::NetTcpClientClose,
        },
        // UDP
        BuiltinFunction {
            name: "net::udp::bind",
            strategy: BuiltinStrategy::NetUdpBind,
        },
        BuiltinFunction {
            name: "net::udp::send",
            strategy: BuiltinStrategy::NetUdpSend,
        },
        BuiltinFunction {
            name: "net::udp::receive",
            strategy: BuiltinStrategy::NetUdpReceive,
        },
        BuiltinFunction {
            name: "net::udp::close",
            strategy: BuiltinStrategy::NetUdpClose,
        },
        BuiltinFunction {
            name: "net::udp::local_addr",
            strategy: BuiltinStrategy::NetUdpLocalAddr,
        },
        // HTTP Client
        BuiltinFunction {
            name: "net::http::client::get",
            strategy: BuiltinStrategy::NetHttpGet,
        },
        BuiltinFunction {
            name: "net::http::client::post",
            strategy: BuiltinStrategy::NetHttpPost,
        },
        BuiltinFunction {
            name: "net::http::client::put",
            strategy: BuiltinStrategy::NetHttpPut,
        },
        BuiltinFunction {
            name: "net::http::client::patch",
            strategy: BuiltinStrategy::NetHttpPatch,
        },
        BuiltinFunction {
            name: "net::http::client::delete",
            strategy: BuiltinStrategy::NetHttpDelete,
        },
        BuiltinFunction {
            name: "net::http::client::set_timeout",
            strategy: BuiltinStrategy::NetHttpSetTimeout,
        },
        BuiltinFunction {
            name: "net::http::client::status",
            strategy: BuiltinStrategy::NetHttpStatus,
        },
        BuiltinFunction {
            name: "net::http::client::body",
            strategy: BuiltinStrategy::NetHttpBody,
        },
        // ========================================
        // HTTP Server module
        // ========================================
        BuiltinFunction { name: "net::http::server::open_router", strategy: BuiltinStrategy::NetHttpServerOpenRouter },
        BuiltinFunction { name: "net::http::server::get", strategy: BuiltinStrategy::NetHttpServerGet },
        BuiltinFunction { name: "net::http::server::post", strategy: BuiltinStrategy::NetHttpServerPost },
        BuiltinFunction { name: "net::http::server::put", strategy: BuiltinStrategy::NetHttpServerPut },
        BuiltinFunction { name: "net::http::server::patch", strategy: BuiltinStrategy::NetHttpServerPatch },
        BuiltinFunction { name: "net::http::server::delete", strategy: BuiltinStrategy::NetHttpServerDelete },
        BuiltinFunction { name: "net::http::server::with", strategy: BuiltinStrategy::NetHttpServerWith },
        BuiltinFunction { name: "net::http::server::group", strategy: BuiltinStrategy::NetHttpServerGroup },
        BuiltinFunction { name: "net::http::server::mount", strategy: BuiltinStrategy::NetHttpServerMount },
        BuiltinFunction { name: "net::http::server::serve", strategy: BuiltinStrategy::NetHttpServerServe },
        BuiltinFunction { name: "net::http::server::text_response", strategy: BuiltinStrategy::NetHttpServerTextResponse },
        // ========================================
        // SQLite database module
        // ========================================
        BuiltinFunction { name: "db::sqlite::open", strategy: BuiltinStrategy::SqliteOpen },
        BuiltinFunction { name: "db::sqlite::open_memory", strategy: BuiltinStrategy::SqliteOpenMemory },
        BuiltinFunction { name: "db::sqlite::close", strategy: BuiltinStrategy::SqliteClose },
        BuiltinFunction { name: "db::sqlite::exec", strategy: BuiltinStrategy::SqliteExec },
        BuiltinFunction { name: "db::sqlite::query", strategy: BuiltinStrategy::SqliteQuery },
        BuiltinFunction { name: "db::sqlite::row_count", strategy: BuiltinStrategy::SqliteRowCount },
        BuiltinFunction { name: "db::sqlite::row_at", strategy: BuiltinStrategy::SqliteRowAt },
        BuiltinFunction { name: "db::sqlite::get_string", strategy: BuiltinStrategy::SqliteGetString },
        BuiltinFunction { name: "db::sqlite::get_int", strategy: BuiltinStrategy::SqliteGetInt },
        BuiltinFunction { name: "db::sqlite::get_float", strategy: BuiltinStrategy::SqliteGetFloat },
        BuiltinFunction { name: "db::sqlite::get_bool", strategy: BuiltinStrategy::SqliteGetBool },
        BuiltinFunction { name: "db::sqlite::is_null", strategy: BuiltinStrategy::SqliteIsNull },
        BuiltinFunction { name: "db::sqlite::columns", strategy: BuiltinStrategy::SqliteColumns },
        BuiltinFunction { name: "db::sqlite::column_count", strategy: BuiltinStrategy::SqliteColumnCount },
        BuiltinFunction { name: "db::sqlite::begin", strategy: BuiltinStrategy::SqliteBegin },
        BuiltinFunction { name: "db::sqlite::commit", strategy: BuiltinStrategy::SqliteCommit },
        BuiltinFunction { name: "db::sqlite::rollback", strategy: BuiltinStrategy::SqliteRollback },
        BuiltinFunction { name: "db::sqlite::prepare", strategy: BuiltinStrategy::SqlitePrepare },
        BuiltinFunction { name: "db::sqlite::bind_string", strategy: BuiltinStrategy::SqliteBindString },
        BuiltinFunction { name: "db::sqlite::bind_int", strategy: BuiltinStrategy::SqliteBindInt },
        BuiltinFunction { name: "db::sqlite::bind_float", strategy: BuiltinStrategy::SqliteBindFloat },
        BuiltinFunction { name: "db::sqlite::step", strategy: BuiltinStrategy::SqliteStep },
        BuiltinFunction { name: "db::sqlite::reset", strategy: BuiltinStrategy::SqliteReset },
        BuiltinFunction { name: "db::sqlite::finalize", strategy: BuiltinStrategy::SqliteFinalize },
        BuiltinFunction { name: "db::sqlite::changes", strategy: BuiltinStrategy::SqliteChanges },
        BuiltinFunction { name: "db::sqlite::last_insert_id", strategy: BuiltinStrategy::SqliteLastInsertId },
        // ========================================
        // Timers module
        // ========================================
        BuiltinFunction { name: "timers::set_timeout", strategy: BuiltinStrategy::TimerSetTimeout },
        BuiltinFunction { name: "timers::cancel_timeout", strategy: BuiltinStrategy::TimerCancelTimeout },
        BuiltinFunction { name: "timers::set_interval", strategy: BuiltinStrategy::TimerSetInterval },
        BuiltinFunction { name: "timers::cancel_interval", strategy: BuiltinStrategy::TimerCancelInterval },
        BuiltinFunction { name: "timers::schedule", strategy: BuiltinStrategy::TimerSchedule },
        BuiltinFunction { name: "timers::cancel_schedule", strategy: BuiltinStrategy::TimerCancelSchedule },
        BuiltinFunction { name: "timers::next_run", strategy: BuiltinStrategy::TimerNextRun },
    ];
    REGISTRY
}

pub fn lookup_builtin(name: &str) -> Option<&'static BuiltinFunction> {
    let registry = get_builtin_registry();
    // 1. Exact match
    if let Some(f) = registry.iter().find(|f| f.name == name) {
        return Some(f);
    }
    // 2. Suffix match (e.g., "arrays::count" matches "collections::arrays::count")
    if name.contains("::") {
        let suffix = format!("::{}", name);
        if let Some(f) = registry.iter().find(|f| f.name.ends_with(&suffix)) {
            return Some(f);
        }
    }
    None
}

/// Compile a built-in function call using the registry
pub fn compile_builtin_call(
    ctx: &mut CompileContext<'_>,
    builder: &mut FunctionBuilder<'_>,
    builtin: &BuiltinFunction,
    args: &[Expression<'_>],
) -> Result<Value, CodegenError> {
    use super::channels::{
        call_channel_close, call_channel_new, call_channel_receive, call_channel_send,
        call_mutex_new, call_rwlock_new,
    };
    use super::expr::compile_expression;
    use super::io::{call_read_line, compile_fmt_call, compile_stderr_call};
    use super::lambda::{
        compile_lambda_array_collection, compile_lambda_bool_collection, compile_lambda_find,
        compile_lambda_find_index, compile_lambda_find_last, compile_lambda_find_last_index,
        compile_lambda_fold, compile_lambda_int_collection, compile_lambda_scan,
        compile_lambda_sort_by, compile_map_lambda_bool, compile_map_lambda_fold,
        compile_map_lambda_int, compile_map_lambda_map, compile_sample,
    };
    use super::misc::{call_datetime_format, call_random, call_random_float, call_sleep};
    use super::print::compile_print_call;
    use super::runtime::rt_func_ref;
    use super::strings::ensure_naml_string;

    match builtin.strategy {
        // ========================================
        // Collections strategies
        // ========================================
        BuiltinStrategy::ArrayWithCapacity => {
            let cap = compile_expression(ctx, builder, &args[0])?;
            super::array::call_array_new(ctx, builder, cap)
        }

        BuiltinStrategy::ArrayLength => {
            let arr = compile_expression(ctx, builder, &args[0])?;
            let len = builder
                .ins()
                .load(types::I64, MemFlags::trusted(), arr, ARRAY_LEN_OFFSET);
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
        BuiltinStrategy::NoArgInt(runtime_fn) => call_int_runtime(ctx, builder, runtime_fn),

        BuiltinStrategy::NoArgVoid(runtime_fn) => call_void_runtime(ctx, builder, runtime_fn),

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

        BuiltinStrategy::RandomFloat => call_random_float(ctx, builder),

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
            let mut value = compile_expression(ctx, builder, &args[1])?;

            let is_string_literal = matches!(
                &args[1],
                Expression::Literal(LiteralExpr { value: Literal::String(_), .. })
            );
            if is_string_literal {
                value = call_string_from_cstr(ctx, builder, value)?;
            }

            if !is_string_literal {
                let is_fresh = matches!(
                    &args[1],
                    Expression::Call(_) | Expression::StructLiteral(_)
                );
                if !is_fresh {
                    use crate::source::Spanned;
                    if let Some(ch_ty) = ctx.annotations.get_type(args[0].span()) {
                        let resolved = ch_ty.resolve();
                        if let crate::typechecker::types::Type::Channel(inner) = &resolved {
                            if let Some(heap_type) = heap_type_from_type(inner, ctx.interner) {
                                emit_incref(ctx, builder, value, &heap_type)?;
                            }
                        }
                    }
                }
            }

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

        BuiltinStrategy::MutexNew => {
            let value = compile_expression(ctx, builder, &args[0])?;
            call_mutex_new(ctx, builder, value)
        }

        BuiltinStrategy::RwlockNew => {
            let value = compile_expression(ctx, builder, &args[0])?;
            call_rwlock_new(ctx, builder, value)
        }

        BuiltinStrategy::AtomicNew => {
            let value = compile_expression(ctx, builder, &args[0])?;
            let value = ensure_i64(builder, value);
            let suffix = get_atomic_type_suffix_from_arg(ctx, &args[0]);
            let func_name = format!("naml_atomic_{}_new", suffix);
            let func_ref = rt_func_ref(ctx, builder, &func_name)?;
            let call = builder.ins().call(func_ref, &[value]);
            Ok(builder.inst_results(call)[0])
        }

        BuiltinStrategy::AtomicLoad => {
            let ptr = compile_expression(ctx, builder, &args[0])?;
            let suffix = get_atomic_type_suffix_from_atomic_arg(ctx, &args[0]);
            let func_name = format!("naml_atomic_{}_load", suffix);
            let func_ref = rt_func_ref(ctx, builder, &func_name)?;
            let call = builder.ins().call(func_ref, &[ptr]);
            let result = builder.inst_results(call)[0];
            if suffix == "bool" {
                Ok(builder.ins().ireduce(types::I8, result))
            } else {
                Ok(result)
            }
        }

        BuiltinStrategy::AtomicStore => {
            let ptr = compile_expression(ctx, builder, &args[0])?;
            let value = compile_expression(ctx, builder, &args[1])?;
            let value = ensure_i64(builder, value);
            let suffix = get_atomic_type_suffix_from_atomic_arg(ctx, &args[0]);
            let func_name = format!("naml_atomic_{}_store", suffix);
            let func_ref = rt_func_ref(ctx, builder, &func_name)?;
            builder.ins().call(func_ref, &[ptr, value]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::AtomicAdd => {
            let ptr = compile_expression(ctx, builder, &args[0])?;
            let value = compile_expression(ctx, builder, &args[1])?;
            let value = ensure_i64(builder, value);
            let suffix = get_atomic_type_suffix_from_atomic_arg(ctx, &args[0]);
            let func_name = format!("naml_atomic_{}_add", suffix);
            let func_ref = rt_func_ref(ctx, builder, &func_name)?;
            let call = builder.ins().call(func_ref, &[ptr, value]);
            Ok(builder.inst_results(call)[0])
        }

        BuiltinStrategy::AtomicSub => {
            let ptr = compile_expression(ctx, builder, &args[0])?;
            let value = compile_expression(ctx, builder, &args[1])?;
            let value = ensure_i64(builder, value);
            let suffix = get_atomic_type_suffix_from_atomic_arg(ctx, &args[0]);
            let func_name = format!("naml_atomic_{}_sub", suffix);
            let func_ref = rt_func_ref(ctx, builder, &func_name)?;
            let call = builder.ins().call(func_ref, &[ptr, value]);
            Ok(builder.inst_results(call)[0])
        }

        BuiltinStrategy::AtomicInc => {
            let ptr = compile_expression(ctx, builder, &args[0])?;
            let suffix = get_atomic_type_suffix_from_atomic_arg(ctx, &args[0]);
            let func_name = format!("naml_atomic_{}_inc", suffix);
            let func_ref = rt_func_ref(ctx, builder, &func_name)?;
            let call = builder.ins().call(func_ref, &[ptr]);
            Ok(builder.inst_results(call)[0])
        }

        BuiltinStrategy::AtomicDec => {
            let ptr = compile_expression(ctx, builder, &args[0])?;
            let suffix = get_atomic_type_suffix_from_atomic_arg(ctx, &args[0]);
            let func_name = format!("naml_atomic_{}_dec", suffix);
            let func_ref = rt_func_ref(ctx, builder, &func_name)?;
            let call = builder.ins().call(func_ref, &[ptr]);
            Ok(builder.inst_results(call)[0])
        }

        BuiltinStrategy::AtomicCas => {
            let ptr = compile_expression(ctx, builder, &args[0])?;
            let expected = compile_expression(ctx, builder, &args[1])?;
            let expected = ensure_i64(builder, expected);
            let new = compile_expression(ctx, builder, &args[2])?;
            let new = ensure_i64(builder, new);
            let suffix = get_atomic_type_suffix_from_atomic_arg(ctx, &args[0]);
            let func_name = format!("naml_atomic_{}_cas", suffix);
            let func_ref = rt_func_ref(ctx, builder, &func_name)?;
            let call = builder.ins().call(func_ref, &[ptr, expected, new]);
            let result = builder.inst_results(call)[0];
            Ok(builder.ins().ireduce(types::I8, result))
        }

        BuiltinStrategy::AtomicSwap => {
            let ptr = compile_expression(ctx, builder, &args[0])?;
            let value = compile_expression(ctx, builder, &args[1])?;
            let value = ensure_i64(builder, value);
            let suffix = get_atomic_type_suffix_from_atomic_arg(ctx, &args[0]);
            let func_name = format!("naml_atomic_{}_swap", suffix);
            let func_ref = rt_func_ref(ctx, builder, &func_name)?;
            let call = builder.ins().call(func_ref, &[ptr, value]);
            let result = builder.inst_results(call)[0];
            if suffix == "bool" {
                Ok(builder.ins().ireduce(types::I8, result))
            } else {
                Ok(result)
            }
        }

        BuiltinStrategy::AtomicAnd => {
            let ptr = compile_expression(ctx, builder, &args[0])?;
            let value = compile_expression(ctx, builder, &args[1])?;
            let value = ensure_i64(builder, value);
            let suffix = get_atomic_type_suffix_from_atomic_arg(ctx, &args[0]);
            let func_name = format!("naml_atomic_{}_and", suffix);
            let func_ref = rt_func_ref(ctx, builder, &func_name)?;
            let call = builder.ins().call(func_ref, &[ptr, value]);
            Ok(builder.inst_results(call)[0])
        }

        BuiltinStrategy::AtomicOr => {
            let ptr = compile_expression(ctx, builder, &args[0])?;
            let value = compile_expression(ctx, builder, &args[1])?;
            let value = ensure_i64(builder, value);
            let suffix = get_atomic_type_suffix_from_atomic_arg(ctx, &args[0]);
            let func_name = format!("naml_atomic_{}_or", suffix);
            let func_ref = rt_func_ref(ctx, builder, &func_name)?;
            let call = builder.ins().call(func_ref, &[ptr, value]);
            Ok(builder.inst_results(call)[0])
        }

        BuiltinStrategy::AtomicXor => {
            let ptr = compile_expression(ctx, builder, &args[0])?;
            let value = compile_expression(ctx, builder, &args[1])?;
            let value = ensure_i64(builder, value);
            let suffix = get_atomic_type_suffix_from_atomic_arg(ctx, &args[0]);
            let func_name = format!("naml_atomic_{}_xor", suffix);
            let func_ref = rt_func_ref(ctx, builder, &func_name)?;
            let call = builder.ins().call(func_ref, &[ptr, value]);
            Ok(builder.inst_results(call)[0])
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
        BuiltinStrategy::Print(newline) => compile_print_call(ctx, builder, args, newline),

        BuiltinStrategy::Sleep => {
            if args.is_empty() {
                return Err(CodegenError::JitCompile(
                    "sleep requires milliseconds argument".to_string(),
                ));
            }
            let ms = compile_expression(ctx, builder, &args[0])?;
            call_sleep(ctx, builder, ms)
        }

        BuiltinStrategy::Stderr(func_name) => compile_stderr_call(ctx, builder, args, func_name),

        BuiltinStrategy::Fmt => compile_fmt_call(ctx, builder, args),

        BuiltinStrategy::ReadLine => call_read_line(ctx, builder),

        // ========================================
        // Map collection strategies
        // ========================================
        BuiltinStrategy::MapLength => {
            let map = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_map_count", map)
        }

        BuiltinStrategy::MapContainsKey => {
            let map = compile_expression(ctx, builder, &args[0])?;
            let key = compile_expression(ctx, builder, &args[1])?;
            let key = ensure_naml_string(ctx, builder, key, &args[1])?;
            call_two_arg_bool_runtime(ctx, builder, "naml_map_contains_key", map, key)
        }

        BuiltinStrategy::MapRemove => {
            let map = compile_expression(ctx, builder, &args[0])?;
            let key = compile_expression(ctx, builder, &args[1])?;
            let key = ensure_naml_string(ctx, builder, key, &args[1])?;
            compile_option_from_map_remove(ctx, builder, map, key)
        }

        BuiltinStrategy::MapClear => {
            let map = compile_expression(ctx, builder, &args[0])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_map_clear")?;
            builder.ins().call(func_ref, &[map]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::MapExtract(runtime_fn) => {
            let map = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, runtime_fn, map)
        }

        BuiltinStrategy::MapEntries => {
            let map = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_map_entries", map)
        }

        BuiltinStrategy::MapFirstOption(runtime_fn) => {
            let map = compile_expression(ctx, builder, &args[0])?;
            compile_option_from_map_first(ctx, builder, map, runtime_fn)
        }

        BuiltinStrategy::MapLambdaBool(runtime_fn) => {
            let map = compile_expression(ctx, builder, &args[0])?;
            let closure = compile_expression(ctx, builder, &args[1])?;
            compile_map_lambda_bool(ctx, builder, map, closure, runtime_fn)
        }

        BuiltinStrategy::MapLambdaInt(runtime_fn) => {
            let map = compile_expression(ctx, builder, &args[0])?;
            let closure = compile_expression(ctx, builder, &args[1])?;
            compile_map_lambda_int(ctx, builder, map, closure, runtime_fn)
        }

        BuiltinStrategy::MapLambdaFold => {
            let map = compile_expression(ctx, builder, &args[0])?;
            let initial = compile_expression(ctx, builder, &args[1])?;
            let closure = compile_expression(ctx, builder, &args[2])?;
            compile_map_lambda_fold(ctx, builder, map, initial, closure)
        }

        BuiltinStrategy::MapLambdaMap(runtime_fn) => {
            let map = compile_expression(ctx, builder, &args[0])?;
            let closure = compile_expression(ctx, builder, &args[1])?;
            compile_map_lambda_map(ctx, builder, map, closure, runtime_fn)
        }

        BuiltinStrategy::MapCombine(runtime_fn) => {
            let map_a = compile_expression(ctx, builder, &args[0])?;
            let map_b = compile_expression(ctx, builder, &args[1])?;
            call_two_arg_ptr_runtime(ctx, builder, runtime_fn, map_a, map_b)
        }

        BuiltinStrategy::MapInvert => {
            let map = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_map_invert", map)
        }

        BuiltinStrategy::MapFromArrays => {
            let keys = compile_expression(ctx, builder, &args[0])?;
            let values = compile_expression(ctx, builder, &args[1])?;
            call_two_arg_ptr_runtime(ctx, builder, "naml_map_from_arrays", keys, values)
        }

        BuiltinStrategy::MapFromEntries => {
            let pairs = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_map_from_entries", pairs)
        }

        // ========================================
        // File system strategies
        // ========================================
        BuiltinStrategy::FsRead => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_fs_read", path)
        }

        BuiltinStrategy::FsReadBytes => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_fs_read_bytes", path)
        }

        BuiltinStrategy::FsWrite => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            let content = compile_expression(ctx, builder, &args[1])?;
            let content = ensure_naml_string(ctx, builder, content, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, "naml_fs_write", path, content)
        }

        BuiltinStrategy::FsAppend => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            let content = compile_expression(ctx, builder, &args[1])?;
            let content = ensure_naml_string(ctx, builder, content, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, "naml_fs_append", path, content)
        }

        BuiltinStrategy::FsWriteBytes => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            let content = compile_expression(ctx, builder, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, "naml_fs_write_bytes", path, content)
        }

        BuiltinStrategy::FsAppendBytes => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            let content = compile_expression(ctx, builder, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, "naml_fs_append_bytes", path, content)
        }

        BuiltinStrategy::FsExists => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_fs_exists", path)
        }

        BuiltinStrategy::FsIsFile => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_fs_is_file", path)
        }

        BuiltinStrategy::FsIsDir => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_fs_is_dir", path)
        }

        BuiltinStrategy::FsListDir => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_fs_list_dir", path)
        }

        BuiltinStrategy::FsMkdir => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_fs_mkdir", path)
        }

        BuiltinStrategy::FsMkdirAll => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_fs_mkdir_all", path)
        }

        BuiltinStrategy::FsRemove => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_fs_remove", path)
        }

        BuiltinStrategy::FsRemoveAll => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_fs_remove_all", path)
        }

        BuiltinStrategy::FsJoin => {
            let parts = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_fs_join", parts)
        }

        BuiltinStrategy::FsDirname => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_fs_dirname", path)
        }

        BuiltinStrategy::FsBasename => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_fs_basename", path)
        }

        BuiltinStrategy::FsExtension => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_fs_extension", path)
        }

        BuiltinStrategy::FsAbsolute => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_fs_absolute", path)
        }

        BuiltinStrategy::FsSize => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_fs_size", path)
        }

        BuiltinStrategy::FsModified => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_fs_modified", path)
        }

        BuiltinStrategy::FsCopy => {
            let src = compile_expression(ctx, builder, &args[0])?;
            let src = ensure_naml_string(ctx, builder, src, &args[0])?;
            let dst = compile_expression(ctx, builder, &args[1])?;
            let dst = ensure_naml_string(ctx, builder, dst, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, "naml_fs_copy", src, dst)
        }

        BuiltinStrategy::FsRename => {
            let src = compile_expression(ctx, builder, &args[0])?;
            let src = ensure_naml_string(ctx, builder, src, &args[0])?;
            let dst = compile_expression(ctx, builder, &args[1])?;
            let dst = ensure_naml_string(ctx, builder, dst, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, "naml_fs_rename", src, dst)
        }

        BuiltinStrategy::FsGetwd => {
            // No arguments - returns pointer to string
            call_int_runtime(ctx, builder, "naml_fs_getwd")
        }

        BuiltinStrategy::FsChdir => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_fs_chdir", path)
        }

        BuiltinStrategy::FsCreateTemp => {
            let prefix = compile_expression(ctx, builder, &args[0])?;
            let prefix = ensure_naml_string(ctx, builder, prefix, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_fs_create_temp", prefix)
        }

        BuiltinStrategy::FsMkdirTemp => {
            let prefix = compile_expression(ctx, builder, &args[0])?;
            let prefix = ensure_naml_string(ctx, builder, prefix, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_fs_mkdir_temp", prefix)
        }

        BuiltinStrategy::FsChmod => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            let mode = compile_expression(ctx, builder, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, "naml_fs_chmod", path, mode)
        }

        BuiltinStrategy::FsTruncate => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            let size = compile_expression(ctx, builder, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, "naml_fs_truncate", path, size)
        }

        BuiltinStrategy::FsStat => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_fs_stat", path)
        }

        // ========================================
        // Memory-mapped file operations
        // ========================================
        BuiltinStrategy::FsMmapOpen => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            let writable = compile_expression(ctx, builder, &args[1])?;
            // Convert bool (i8) to i64 for runtime call
            let writable_i64 = builder
                .ins()
                .uextend(cranelift::prelude::types::I64, writable);
            call_two_arg_int_runtime(ctx, builder, "naml_fs_mmap_open", path, writable_i64)
        }

        BuiltinStrategy::FsMmapLen => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_fs_mmap_len", handle)
        }

        BuiltinStrategy::FsMmapReadByte => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            let offset = compile_expression(ctx, builder, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, "naml_fs_mmap_read_byte", handle, offset)
        }

        BuiltinStrategy::FsMmapWriteByte => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            let offset = compile_expression(ctx, builder, &args[1])?;
            let value = compile_expression(ctx, builder, &args[2])?;
            call_three_arg_int_runtime(
                ctx,
                builder,
                "naml_fs_mmap_write_byte",
                handle,
                offset,
                value,
            )
        }

        BuiltinStrategy::FsMmapRead => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            let offset = compile_expression(ctx, builder, &args[1])?;
            let len = compile_expression(ctx, builder, &args[2])?;
            call_three_arg_ptr_runtime(ctx, builder, "naml_fs_mmap_read", handle, offset, len)
        }

        BuiltinStrategy::FsMmapWrite => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            let offset = compile_expression(ctx, builder, &args[1])?;
            let data = compile_expression(ctx, builder, &args[2])?;
            call_three_arg_int_runtime(ctx, builder, "naml_fs_mmap_write", handle, offset, data)
        }

        BuiltinStrategy::FsMmapFlush => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_fs_mmap_flush", handle)
        }

        BuiltinStrategy::FsMmapClose => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_fs_mmap_close", handle)
        }

        // ========================================
        // File handle operations
        // ========================================
        BuiltinStrategy::FsFileOpen => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            let mode = compile_expression(ctx, builder, &args[1])?;
            let mode = ensure_naml_string(ctx, builder, mode, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, "naml_fs_file_open", path, mode)
        }

        BuiltinStrategy::FsFileClose => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_fs_file_close", handle)
        }

        BuiltinStrategy::FsFileRead => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            let count = compile_expression(ctx, builder, &args[1])?;
            call_two_arg_ptr_runtime(ctx, builder, "naml_fs_file_read", handle, count)
        }

        BuiltinStrategy::FsFileReadLine => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_fs_file_read_line", handle)
        }

        BuiltinStrategy::FsFileReadAll => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_fs_file_read_all", handle)
        }

        BuiltinStrategy::FsFileWrite => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            let content = compile_expression(ctx, builder, &args[1])?;
            let content = ensure_naml_string(ctx, builder, content, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, "naml_fs_file_write", handle, content)
        }

        BuiltinStrategy::FsFileWriteLine => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            let content = compile_expression(ctx, builder, &args[1])?;
            let content = ensure_naml_string(ctx, builder, content, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, "naml_fs_file_write_line", handle, content)
        }

        BuiltinStrategy::FsFileFlush => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_fs_file_flush", handle)
        }

        BuiltinStrategy::FsFileSeek => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            let offset = compile_expression(ctx, builder, &args[1])?;
            let whence = compile_expression(ctx, builder, &args[2])?;
            call_three_arg_int_runtime(ctx, builder, "naml_fs_file_seek", handle, offset, whence)
        }

        BuiltinStrategy::FsFileTell => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_fs_file_tell", handle)
        }

        BuiltinStrategy::FsFileEof => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_fs_file_eof", handle)
        }

        BuiltinStrategy::FsFileSize => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_fs_file_size", handle)
        }

        // ========================================
        // Link/symlink operations
        // ========================================
        BuiltinStrategy::FsSymlink => {
            let target = compile_expression(ctx, builder, &args[0])?;
            let target = ensure_naml_string(ctx, builder, target, &args[0])?;
            let link_path = compile_expression(ctx, builder, &args[1])?;
            let link_path = ensure_naml_string(ctx, builder, link_path, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, "naml_fs_symlink", target, link_path)
        }

        BuiltinStrategy::FsReadlink => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_fs_readlink", path)
        }

        BuiltinStrategy::FsLstat => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_fs_lstat", path)
        }

        BuiltinStrategy::FsLink => {
            let src = compile_expression(ctx, builder, &args[0])?;
            let src = ensure_naml_string(ctx, builder, src, &args[0])?;
            let dst = compile_expression(ctx, builder, &args[1])?;
            let dst = ensure_naml_string(ctx, builder, dst, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, "naml_fs_link", src, dst)
        }

        BuiltinStrategy::FsChtimes => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            let atime = compile_expression(ctx, builder, &args[1])?;
            let mtime = compile_expression(ctx, builder, &args[2])?;
            call_three_arg_int_runtime(ctx, builder, "naml_fs_chtimes", path, atime, mtime)
        }

        BuiltinStrategy::FsChown => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            let uid = compile_expression(ctx, builder, &args[1])?;
            let gid = compile_expression(ctx, builder, &args[2])?;
            call_three_arg_int_runtime(ctx, builder, "naml_fs_chown", path, uid, gid)
        }

        BuiltinStrategy::FsLchown => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            let uid = compile_expression(ctx, builder, &args[1])?;
            let gid = compile_expression(ctx, builder, &args[2])?;
            call_three_arg_int_runtime(ctx, builder, "naml_fs_lchown", path, uid, gid)
        }

        BuiltinStrategy::FsSameFile => {
            let path1 = compile_expression(ctx, builder, &args[0])?;
            let path1 = ensure_naml_string(ctx, builder, path1, &args[0])?;
            let path2 = compile_expression(ctx, builder, &args[1])?;
            let path2 = ensure_naml_string(ctx, builder, path2, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, "naml_fs_same_file", path1, path2)
        }

        // ========================================
        // Additional file handle operations
        // ========================================
        BuiltinStrategy::FsFileReadAt => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            let buf_size = compile_expression(ctx, builder, &args[1])?;
            let offset = compile_expression(ctx, builder, &args[2])?;
            call_three_arg_ptr_runtime(ctx, builder, "naml_fs_file_read_at", handle, buf_size, offset)
        }

        BuiltinStrategy::FsFileWriteAt => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            let content = compile_expression(ctx, builder, &args[1])?;
            let content = ensure_naml_string(ctx, builder, content, &args[1])?;
            let offset = compile_expression(ctx, builder, &args[2])?;
            call_three_arg_int_runtime(ctx, builder, "naml_fs_file_write_at", handle, content, offset)
        }

        BuiltinStrategy::FsFileName => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_fs_file_name", handle)
        }

        BuiltinStrategy::FsFileStat => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_fs_file_stat", handle)
        }

        BuiltinStrategy::FsFileTruncate => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            let size = compile_expression(ctx, builder, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, "naml_fs_file_truncate", handle, size)
        }

        BuiltinStrategy::FsFileChmod => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            let mode = compile_expression(ctx, builder, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, "naml_fs_file_chmod", handle, mode)
        }

        BuiltinStrategy::FsFileChown => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            let uid = compile_expression(ctx, builder, &args[1])?;
            let gid = compile_expression(ctx, builder, &args[2])?;
            call_three_arg_int_runtime(ctx, builder, "naml_fs_file_chown", handle, uid, gid)
        }

        // ========================================
        // Path module operations
        // ========================================
        BuiltinStrategy::PathJoin => {
            let parts = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_path_join", parts)
        }

        BuiltinStrategy::PathOneArgStr(runtime_fn) => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, runtime_fn, path)
        }

        BuiltinStrategy::PathOneArgBool(runtime_fn) => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, runtime_fn, path)
        }

        BuiltinStrategy::PathTwoArgStr(runtime_fn) => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            let other = compile_expression(ctx, builder, &args[1])?;
            let other = ensure_naml_string(ctx, builder, other, &args[1])?;
            call_two_arg_ptr_runtime(ctx, builder, runtime_fn, path, other)
        }

        BuiltinStrategy::PathTwoArgBool(runtime_fn) => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            let other = compile_expression(ctx, builder, &args[1])?;
            let other = ensure_naml_string(ctx, builder, other, &args[1])?;
            call_two_arg_bool_runtime(ctx, builder, runtime_fn, path, other)
        }

        BuiltinStrategy::PathComponents => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_path_components", path)
        }

        BuiltinStrategy::PathSeparator => {
            use super::runtime::rt_func_ref;
            let func_ref = rt_func_ref(ctx, builder, "naml_path_separator")?;
            let inst = builder.ins().call(func_ref, &[]);
            let results = builder.inst_results(inst);
            Ok(results[0])
        }

        // ========================================
        // Env strategies
        // ========================================
        BuiltinStrategy::EnvGetenv => {
            let key = compile_expression(ctx, builder, &args[0])?;
            let key = ensure_naml_string(ctx, builder, key, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_env_getenv", key)
        }

        BuiltinStrategy::EnvLookupEnv => {
            let key = compile_expression(ctx, builder, &args[0])?;
            let key = ensure_naml_string(ctx, builder, key, &args[0])?;
            compile_option_from_nullable_ptr(ctx, builder, key, "naml_env_lookup_env")
        }

        BuiltinStrategy::EnvSetenv => {
            let key = compile_expression(ctx, builder, &args[0])?;
            let key = ensure_naml_string(ctx, builder, key, &args[0])?;
            let value = compile_expression(ctx, builder, &args[1])?;
            let value = ensure_naml_string(ctx, builder, value, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, "naml_env_setenv", key, value)
        }

        BuiltinStrategy::EnvUnsetenv => {
            let key = compile_expression(ctx, builder, &args[0])?;
            let key = ensure_naml_string(ctx, builder, key, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_env_unsetenv", key)
        }

        BuiltinStrategy::EnvClearenv => {
            use super::runtime::rt_func_ref;
            let func_ref = rt_func_ref(ctx, builder, "naml_env_clearenv")?;
            let inst = builder.ins().call(func_ref, &[]);
            let results = builder.inst_results(inst);
            Ok(results[0])
        }

        BuiltinStrategy::EnvEnviron => {
            use super::runtime::rt_func_ref;
            let func_ref = rt_func_ref(ctx, builder, "naml_env_environ")?;
            let inst = builder.ins().call(func_ref, &[]);
            let results = builder.inst_results(inst);
            Ok(results[0])
        }

        BuiltinStrategy::EnvExpandEnv => {
            let s = compile_expression(ctx, builder, &args[0])?;
            let s = ensure_naml_string(ctx, builder, s, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_env_expand_env", s)
        }

        // ========================================
        // OS strategies
        // ========================================
        BuiltinStrategy::OsHostname => {
            use super::runtime::rt_func_ref;
            let func_ref = rt_func_ref(ctx, builder, "naml_os_hostname")?;
            let inst = builder.ins().call(func_ref, &[]);
            let results = builder.inst_results(inst);
            Ok(results[0])
        }

        BuiltinStrategy::OsTempDir => {
            use super::runtime::rt_func_ref;
            let func_ref = rt_func_ref(ctx, builder, "naml_os_temp_dir")?;
            let inst = builder.ins().call(func_ref, &[]);
            let results = builder.inst_results(inst);
            Ok(results[0])
        }

        BuiltinStrategy::OsHomeDir => {
            use super::runtime::rt_func_ref;
            let func_ref = rt_func_ref(ctx, builder, "naml_os_home_dir")?;
            let inst = builder.ins().call(func_ref, &[]);
            let results = builder.inst_results(inst);
            Ok(results[0])
        }

        BuiltinStrategy::OsCacheDir => {
            use super::runtime::rt_func_ref;
            let func_ref = rt_func_ref(ctx, builder, "naml_os_cache_dir")?;
            let inst = builder.ins().call(func_ref, &[]);
            let results = builder.inst_results(inst);
            Ok(results[0])
        }

        BuiltinStrategy::OsConfigDir => {
            use super::runtime::rt_func_ref;
            let func_ref = rt_func_ref(ctx, builder, "naml_os_config_dir")?;
            let inst = builder.ins().call(func_ref, &[]);
            let results = builder.inst_results(inst);
            Ok(results[0])
        }

        BuiltinStrategy::OsExecutable => {
            use super::runtime::rt_func_ref;
            let func_ref = rt_func_ref(ctx, builder, "naml_os_executable")?;
            let inst = builder.ins().call(func_ref, &[]);
            let results = builder.inst_results(inst);
            Ok(results[0])
        }

        BuiltinStrategy::OsPagesize => {
            use super::runtime::rt_func_ref;
            let func_ref = rt_func_ref(ctx, builder, "naml_os_pagesize")?;
            let inst = builder.ins().call(func_ref, &[]);
            let results = builder.inst_results(inst);
            Ok(results[0])
        }

        BuiltinStrategy::OsGetuid => {
            use super::runtime::rt_func_ref;
            let func_ref = rt_func_ref(ctx, builder, "naml_os_getuid")?;
            let inst = builder.ins().call(func_ref, &[]);
            let results = builder.inst_results(inst);
            Ok(results[0])
        }

        BuiltinStrategy::OsGeteuid => {
            use super::runtime::rt_func_ref;
            let func_ref = rt_func_ref(ctx, builder, "naml_os_geteuid")?;
            let inst = builder.ins().call(func_ref, &[]);
            let results = builder.inst_results(inst);
            Ok(results[0])
        }

        BuiltinStrategy::OsGetgid => {
            use super::runtime::rt_func_ref;
            let func_ref = rt_func_ref(ctx, builder, "naml_os_getgid")?;
            let inst = builder.ins().call(func_ref, &[]);
            let results = builder.inst_results(inst);
            Ok(results[0])
        }

        BuiltinStrategy::OsGetegid => {
            use super::runtime::rt_func_ref;
            let func_ref = rt_func_ref(ctx, builder, "naml_os_getegid")?;
            let inst = builder.ins().call(func_ref, &[]);
            let results = builder.inst_results(inst);
            Ok(results[0])
        }

        BuiltinStrategy::OsGetgroups => {
            use super::runtime::rt_func_ref;
            let func_ref = rt_func_ref(ctx, builder, "naml_os_getgroups")?;
            let inst = builder.ins().call(func_ref, &[]);
            let results = builder.inst_results(inst);
            Ok(results[0])
        }

        // ========================================
        // Process strategies
        // ========================================
        BuiltinStrategy::ProcessGetpid => {
            call_int_runtime(ctx, builder, "naml_process_getpid")
        }

        BuiltinStrategy::ProcessGetppid => {
            call_int_runtime(ctx, builder, "naml_process_getppid")
        }

        BuiltinStrategy::ProcessExit => {
            let code = compile_expression(ctx, builder, &args[0])?;
            use super::runtime::rt_func_ref;
            let func_ref = rt_func_ref(ctx, builder, "naml_process_exit")?;
            builder.ins().call(func_ref, &[code]);
            Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
        }

        BuiltinStrategy::ProcessPipeRead => {
            call_int_runtime(ctx, builder, "naml_process_pipe_read")
        }

        BuiltinStrategy::ProcessPipeWrite => {
            call_int_runtime(ctx, builder, "naml_process_pipe_write")
        }

        BuiltinStrategy::ProcessStart => {
            let name = compile_expression(ctx, builder, &args[0])?;
            let name = ensure_naml_string(ctx, builder, name, &args[0])?;
            let arr = compile_expression(ctx, builder, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, "naml_process_start", name, arr)
        }

        BuiltinStrategy::ProcessFind => {
            let pid = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_process_find", pid)
        }

        BuiltinStrategy::ProcessWait => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_process_wait", handle)
        }

        BuiltinStrategy::ProcessSignal => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            let sig = compile_expression(ctx, builder, &args[1])?;
            use super::runtime::rt_func_ref;
            let func_ref = rt_func_ref(ctx, builder, "naml_process_signal")?;
            builder.ins().call(func_ref, &[handle, sig]);
            Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
        }

        BuiltinStrategy::ProcessKill => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            use super::runtime::rt_func_ref;
            let func_ref = rt_func_ref(ctx, builder, "naml_process_kill")?;
            builder.ins().call(func_ref, &[handle]);
            Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
        }

        BuiltinStrategy::ProcessRelease => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            use super::runtime::rt_func_ref;
            let func_ref = rt_func_ref(ctx, builder, "naml_process_release")?;
            builder.ins().call(func_ref, &[handle]);
            Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
        }

        BuiltinStrategy::ProcessSighup => {
            call_int_runtime(ctx, builder, "naml_process_sighup")
        }
        BuiltinStrategy::ProcessSigint => {
            call_int_runtime(ctx, builder, "naml_process_sigint")
        }
        BuiltinStrategy::ProcessSigquit => {
            call_int_runtime(ctx, builder, "naml_process_sigquit")
        }
        BuiltinStrategy::ProcessSigkill => {
            call_int_runtime(ctx, builder, "naml_process_sigkill")
        }
        BuiltinStrategy::ProcessSigterm => {
            call_int_runtime(ctx, builder, "naml_process_sigterm")
        }
        BuiltinStrategy::ProcessSigstop => {
            call_int_runtime(ctx, builder, "naml_process_sigstop")
        }
        BuiltinStrategy::ProcessSigcont => {
            call_int_runtime(ctx, builder, "naml_process_sigcont")
        }

        // ========================================
        // Testing strategies
        // ========================================
        BuiltinStrategy::TestingAssert => {
            let cond = compile_expression(ctx, builder, &args[0])?;
            let cond = ensure_i64(builder, cond);
            let msg = compile_expression(ctx, builder, &args[1])?;
            let msg = ensure_naml_string(ctx, builder, msg, &args[1])?;
            call_two_arg_runtime(ctx, builder, "naml_testing_assert", cond, msg)
        }

        BuiltinStrategy::TestingAssertEq => {
            let actual = compile_expression(ctx, builder, &args[0])?;
            let expected = compile_expression(ctx, builder, &args[1])?;
            let msg = compile_expression(ctx, builder, &args[2])?;
            let msg = ensure_naml_string(ctx, builder, msg, &args[2])?;
            call_three_arg_void_runtime(ctx, builder, "naml_testing_assert_eq", actual, expected, msg)
        }

        BuiltinStrategy::TestingAssertEqFloat => {
            let actual = compile_expression(ctx, builder, &args[0])?;
            let expected = compile_expression(ctx, builder, &args[1])?;
            let msg = compile_expression(ctx, builder, &args[2])?;
            let msg = ensure_naml_string(ctx, builder, msg, &args[2])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_testing_assert_eq_float")?;
            builder.ins().call(func_ref, &[actual, expected, msg]);
            Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
        }

        BuiltinStrategy::TestingAssertEqString => {
            let actual = compile_expression(ctx, builder, &args[0])?;
            let actual = ensure_naml_string(ctx, builder, actual, &args[0])?;
            let expected = compile_expression(ctx, builder, &args[1])?;
            let expected = ensure_naml_string(ctx, builder, expected, &args[1])?;
            let msg = compile_expression(ctx, builder, &args[2])?;
            let msg = ensure_naml_string(ctx, builder, msg, &args[2])?;
            call_three_arg_void_runtime(ctx, builder, "naml_testing_assert_eq_string", actual, expected, msg)
        }

        BuiltinStrategy::TestingAssertEqBool => {
            let actual = compile_expression(ctx, builder, &args[0])?;
            let actual = ensure_i64(builder, actual);
            let expected = compile_expression(ctx, builder, &args[1])?;
            let expected = ensure_i64(builder, expected);
            let msg = compile_expression(ctx, builder, &args[2])?;
            let msg = ensure_naml_string(ctx, builder, msg, &args[2])?;
            call_three_arg_void_runtime(ctx, builder, "naml_testing_assert_eq_bool", actual, expected, msg)
        }

        BuiltinStrategy::TestingAssertNeq => {
            let actual = compile_expression(ctx, builder, &args[0])?;
            let expected = compile_expression(ctx, builder, &args[1])?;
            let msg = compile_expression(ctx, builder, &args[2])?;
            let msg = ensure_naml_string(ctx, builder, msg, &args[2])?;
            call_three_arg_void_runtime(ctx, builder, "naml_testing_assert_neq", actual, expected, msg)
        }

        BuiltinStrategy::TestingAssertNeqString => {
            let actual = compile_expression(ctx, builder, &args[0])?;
            let actual = ensure_naml_string(ctx, builder, actual, &args[0])?;
            let expected = compile_expression(ctx, builder, &args[1])?;
            let expected = ensure_naml_string(ctx, builder, expected, &args[1])?;
            let msg = compile_expression(ctx, builder, &args[2])?;
            let msg = ensure_naml_string(ctx, builder, msg, &args[2])?;
            call_three_arg_void_runtime(ctx, builder, "naml_testing_assert_neq_string", actual, expected, msg)
        }

        BuiltinStrategy::TestingAssertTrue => {
            let cond = compile_expression(ctx, builder, &args[0])?;
            let cond = ensure_i64(builder, cond);
            let msg = compile_expression(ctx, builder, &args[1])?;
            let msg = ensure_naml_string(ctx, builder, msg, &args[1])?;
            call_two_arg_runtime(ctx, builder, "naml_testing_assert_true", cond, msg)
        }

        BuiltinStrategy::TestingAssertFalse => {
            let cond = compile_expression(ctx, builder, &args[0])?;
            let cond = ensure_i64(builder, cond);
            let msg = compile_expression(ctx, builder, &args[1])?;
            let msg = ensure_naml_string(ctx, builder, msg, &args[1])?;
            call_two_arg_runtime(ctx, builder, "naml_testing_assert_false", cond, msg)
        }

        BuiltinStrategy::TestingAssertGt => {
            let actual = compile_expression(ctx, builder, &args[0])?;
            let expected = compile_expression(ctx, builder, &args[1])?;
            let msg = compile_expression(ctx, builder, &args[2])?;
            let msg = ensure_naml_string(ctx, builder, msg, &args[2])?;
            call_three_arg_void_runtime(ctx, builder, "naml_testing_assert_gt", actual, expected, msg)
        }

        BuiltinStrategy::TestingAssertGte => {
            let actual = compile_expression(ctx, builder, &args[0])?;
            let expected = compile_expression(ctx, builder, &args[1])?;
            let msg = compile_expression(ctx, builder, &args[2])?;
            let msg = ensure_naml_string(ctx, builder, msg, &args[2])?;
            call_three_arg_void_runtime(ctx, builder, "naml_testing_assert_gte", actual, expected, msg)
        }

        BuiltinStrategy::TestingAssertLt => {
            let actual = compile_expression(ctx, builder, &args[0])?;
            let expected = compile_expression(ctx, builder, &args[1])?;
            let msg = compile_expression(ctx, builder, &args[2])?;
            let msg = ensure_naml_string(ctx, builder, msg, &args[2])?;
            call_three_arg_void_runtime(ctx, builder, "naml_testing_assert_lt", actual, expected, msg)
        }

        BuiltinStrategy::TestingAssertLte => {
            let actual = compile_expression(ctx, builder, &args[0])?;
            let expected = compile_expression(ctx, builder, &args[1])?;
            let msg = compile_expression(ctx, builder, &args[2])?;
            let msg = ensure_naml_string(ctx, builder, msg, &args[2])?;
            call_three_arg_void_runtime(ctx, builder, "naml_testing_assert_lte", actual, expected, msg)
        }

        BuiltinStrategy::TestingFail => {
            let msg = compile_expression(ctx, builder, &args[0])?;
            let msg = ensure_naml_string(ctx, builder, msg, &args[0])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_testing_fail")?;
            builder.ins().call(func_ref, &[msg]);
            Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
        }

        BuiltinStrategy::TestingAssertApprox => {
            let actual = compile_expression(ctx, builder, &args[0])?;
            let expected = compile_expression(ctx, builder, &args[1])?;
            let epsilon = compile_expression(ctx, builder, &args[2])?;
            let msg = compile_expression(ctx, builder, &args[3])?;
            let msg = ensure_naml_string(ctx, builder, msg, &args[3])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_testing_assert_approx")?;
            builder.ins().call(func_ref, &[actual, expected, epsilon, msg]);
            Ok(builder.ins().iconst(cranelift::prelude::types::I64, 0))
        }

        BuiltinStrategy::TestingAssertContains => {
            let haystack = compile_expression(ctx, builder, &args[0])?;
            let haystack = ensure_naml_string(ctx, builder, haystack, &args[0])?;
            let needle = compile_expression(ctx, builder, &args[1])?;
            let needle = ensure_naml_string(ctx, builder, needle, &args[1])?;
            let msg = compile_expression(ctx, builder, &args[2])?;
            let msg = ensure_naml_string(ctx, builder, msg, &args[2])?;
            call_three_arg_void_runtime(ctx, builder, "naml_testing_assert_contains", haystack, needle, msg)
        }

        BuiltinStrategy::TestingAssertStartsWith => {
            let value = compile_expression(ctx, builder, &args[0])?;
            let value = ensure_naml_string(ctx, builder, value, &args[0])?;
            let prefix = compile_expression(ctx, builder, &args[1])?;
            let prefix = ensure_naml_string(ctx, builder, prefix, &args[1])?;
            let msg = compile_expression(ctx, builder, &args[2])?;
            let msg = ensure_naml_string(ctx, builder, msg, &args[2])?;
            call_three_arg_void_runtime(ctx, builder, "naml_testing_assert_starts_with", value, prefix, msg)
        }

        BuiltinStrategy::TestingAssertEndsWith => {
            let value = compile_expression(ctx, builder, &args[0])?;
            let value = ensure_naml_string(ctx, builder, value, &args[0])?;
            let suffix = compile_expression(ctx, builder, &args[1])?;
            let suffix = ensure_naml_string(ctx, builder, suffix, &args[1])?;
            let msg = compile_expression(ctx, builder, &args[2])?;
            let msg = ensure_naml_string(ctx, builder, msg, &args[2])?;
            call_three_arg_void_runtime(ctx, builder, "naml_testing_assert_ends_with", value, suffix, msg)
        }

        // ========================================
        // Crypto strategies
        // ========================================
        BuiltinStrategy::CryptoHashBytes(runtime_fn) => {
            let data = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, runtime_fn, data)
        }

        BuiltinStrategy::CryptoHashHex(runtime_fn) => {
            let data = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, runtime_fn, data)
        }

        BuiltinStrategy::CryptoHmacBytes(runtime_fn) => {
            let key = compile_expression(ctx, builder, &args[0])?;
            let data = compile_expression(ctx, builder, &args[1])?;
            call_two_arg_ptr_runtime(ctx, builder, runtime_fn, key, data)
        }

        BuiltinStrategy::CryptoHmacHex(runtime_fn) => {
            let key = compile_expression(ctx, builder, &args[0])?;
            let data = compile_expression(ctx, builder, &args[1])?;
            call_two_arg_ptr_runtime(ctx, builder, runtime_fn, key, data)
        }

        BuiltinStrategy::CryptoHmacVerify(runtime_fn) => {
            use super::runtime::rt_func_ref;
            let key = compile_expression(ctx, builder, &args[0])?;
            let data = compile_expression(ctx, builder, &args[1])?;
            let mac = compile_expression(ctx, builder, &args[2])?;
            let func_ref = rt_func_ref(ctx, builder, runtime_fn)?;
            let call = builder.ins().call(func_ref, &[key, data, mac]);
            let result = builder.inst_results(call)[0];
            Ok(builder.ins().ireduce(cranelift::prelude::types::I8, result))
        }

        BuiltinStrategy::CryptoPbkdf2(runtime_fn) => {
            use super::runtime::rt_func_ref;
            let password = compile_expression(ctx, builder, &args[0])?;
            let salt = compile_expression(ctx, builder, &args[1])?;
            let iterations = compile_expression(ctx, builder, &args[2])?;
            let key_len = compile_expression(ctx, builder, &args[3])?;
            let func_ref = rt_func_ref(ctx, builder, runtime_fn)?;
            let call = builder.ins().call(func_ref, &[password, salt, iterations, key_len]);
            Ok(builder.inst_results(call)[0])
        }

        BuiltinStrategy::CryptoRandomBytes(runtime_fn) => {
            let n = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, runtime_fn, n)
        }

        // ========================================
        // Encoding strategies
        // ========================================
        BuiltinStrategy::EncodingBytesToString(runtime_fn) => {
            let bytes = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, runtime_fn, bytes)
        }

        BuiltinStrategy::EncodingStringToBytes(runtime_fn) => {
            let s = compile_expression(ctx, builder, &args[0])?;
            let s = ensure_naml_string(ctx, builder, s, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, runtime_fn, s)
        }

        BuiltinStrategy::EncodingValidate(runtime_fn) => {
            let bytes = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, runtime_fn, bytes)
        }

        BuiltinStrategy::EncodingDecodeToString(runtime_fn) => {
            use super::runtime::rt_func_ref;
            let ptr_type = ctx.module.target_config().pointer_type();

            let bytes = compile_expression(ctx, builder, &args[0])?;

            let slot_tag = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                4,
                4,
            ));
            let slot_value = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                8,
                8,
            ));

            let out_tag = builder.ins().stack_addr(ptr_type, slot_tag, 0);
            let out_value = builder.ins().stack_addr(ptr_type, slot_value, 0);

            let func_ref = rt_func_ref(ctx, builder, runtime_fn)?;
            builder.ins().call(func_ref, &[bytes, out_tag, out_value]);

            let tag = builder
                .ins()
                .load(types::I32, MemFlags::trusted(), out_tag, 0);
            let value = builder
                .ins()
                .load(types::I64, MemFlags::trusted(), out_value, 0);

            let success_block = builder.create_block();
            let error_block = builder.create_block();
            let merge_block = builder.create_block();
            builder.append_block_param(merge_block, types::I64);

            let tag_is_zero = builder.ins().icmp_imm(IntCC::Equal, tag, 0);
            builder
                .ins()
                .brif(tag_is_zero, success_block, &[], error_block, &[]);

            builder.switch_to_block(success_block);
            builder.seal_block(success_block);
            builder.ins().jump(merge_block, &[value]);

            builder.switch_to_block(error_block);
            builder.seal_block(error_block);
            use super::exceptions::throw_decode_error;
            throw_decode_error(ctx, builder, value)?;
            builder.ins().jump(merge_block, &[value]);

            builder.switch_to_block(merge_block);
            builder.seal_block(merge_block);

            let result = builder.block_params(merge_block)[0];
            Ok(result)
        }

        BuiltinStrategy::EncodingDecodeToBytes(runtime_fn) => {
            use super::runtime::rt_func_ref;
            let ptr_type = ctx.module.target_config().pointer_type();

            let s = compile_expression(ctx, builder, &args[0])?;
            let s = ensure_naml_string(ctx, builder, s, &args[0])?;

            let slot_tag = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                4,
                4,
            ));
            let slot_value = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                8,
                8,
            ));

            let out_tag = builder.ins().stack_addr(ptr_type, slot_tag, 0);
            let out_value = builder.ins().stack_addr(ptr_type, slot_value, 0);

            let func_ref = rt_func_ref(ctx, builder, runtime_fn)?;
            builder.ins().call(func_ref, &[s, out_tag, out_value]);

            let tag = builder
                .ins()
                .load(types::I32, MemFlags::trusted(), out_tag, 0);
            let value = builder
                .ins()
                .load(types::I64, MemFlags::trusted(), out_value, 0);

            let success_block = builder.create_block();
            let error_block = builder.create_block();
            let merge_block = builder.create_block();
            builder.append_block_param(merge_block, types::I64);

            let tag_is_zero = builder.ins().icmp_imm(IntCC::Equal, tag, 0);
            builder
                .ins()
                .brif(tag_is_zero, success_block, &[], error_block, &[]);

            builder.switch_to_block(success_block);
            builder.seal_block(success_block);
            builder.ins().jump(merge_block, &[value]);

            builder.switch_to_block(error_block);
            builder.seal_block(error_block);
            use super::exceptions::throw_decode_error;
            throw_decode_error(ctx, builder, value)?;
            builder.ins().jump(merge_block, &[value]);

            builder.switch_to_block(merge_block);
            builder.seal_block(merge_block);

            let result = builder.block_params(merge_block)[0];
            Ok(result)
        }

        // ========================================
        // Binary encoding strategies
        // ========================================
        BuiltinStrategy::BinaryOneArgCall(runtime_fn) => {
            let arg0 = compile_expression(ctx, builder, &args[0])?;
            let func_ref = rt_func_ref(ctx, builder, runtime_fn)?;
            let call = builder.ins().call(func_ref, &[arg0]);
            Ok(builder.inst_results(call)[0])
        }

        BuiltinStrategy::BinaryTwoArgCall(runtime_fn) => {
            let arg0 = compile_expression(ctx, builder, &args[0])?;
            let arg1 = compile_expression(ctx, builder, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, runtime_fn, arg0, arg1)
        }

        BuiltinStrategy::BinaryThreeArgCall(runtime_fn) => {
            let arg0 = compile_expression(ctx, builder, &args[0])?;
            let arg1 = compile_expression(ctx, builder, &args[1])?;
            let arg2 = compile_expression(ctx, builder, &args[2])?;
            call_three_arg_ptr_runtime(ctx, builder, runtime_fn, arg0, arg1, arg2)
        }

        BuiltinStrategy::BinaryReadFloat(runtime_fn) => {
            let arg0 = compile_expression(ctx, builder, &args[0])?;
            let arg1 = compile_expression(ctx, builder, &args[1])?;
            let func_ref = rt_func_ref(ctx, builder, runtime_fn)?;
            let call = builder.ins().call(func_ref, &[arg0, arg1]);
            Ok(builder.inst_results(call)[0])
        }

        BuiltinStrategy::BinaryOneArgVoid(runtime_fn) => {
            let arg0 = compile_expression(ctx, builder, &args[0])?;
            let func_ref = rt_func_ref(ctx, builder, runtime_fn)?;
            builder.ins().call(func_ref, &[arg0]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::BinaryTwoArgVoid(runtime_fn) => {
            let arg0 = compile_expression(ctx, builder, &args[0])?;
            let arg1 = compile_expression(ctx, builder, &args[1])?;
            let func_ref = rt_func_ref(ctx, builder, runtime_fn)?;
            builder.ins().call(func_ref, &[arg0, arg1]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::BinaryThreeArgVoid(runtime_fn) => {
            let arg0 = compile_expression(ctx, builder, &args[0])?;
            let arg1 = compile_expression(ctx, builder, &args[1])?;
            let arg2 = compile_expression(ctx, builder, &args[2])?;
            call_three_arg_void_runtime(ctx, builder, runtime_fn, arg0, arg1, arg2)
        }

        BuiltinStrategy::BinaryWriteFloat(runtime_fn) => {
            let arg0 = compile_expression(ctx, builder, &args[0])?;
            let arg1 = compile_expression(ctx, builder, &args[1])?;
            let arg2 = compile_expression(ctx, builder, &args[2])?;
            let func_ref = rt_func_ref(ctx, builder, runtime_fn)?;
            builder.ins().call(func_ref, &[arg0, arg1, arg2]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::BinaryFourArgVoid(runtime_fn) => {
            let arg0 = compile_expression(ctx, builder, &args[0])?;
            let arg1 = compile_expression(ctx, builder, &args[1])?;
            let arg2 = compile_expression(ctx, builder, &args[2])?;
            let arg3 = compile_expression(ctx, builder, &args[3])?;
            let func_ref = rt_func_ref(ctx, builder, runtime_fn)?;
            builder.ins().call(func_ref, &[arg0, arg1, arg2, arg3]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::BinaryTwoArgBool(runtime_fn) => {
            let arg0 = compile_expression(ctx, builder, &args[0])?;
            let arg1 = compile_expression(ctx, builder, &args[1])?;
            call_two_arg_bool_runtime(ctx, builder, runtime_fn, arg0, arg1)
        }

        // ========================================
        // JSON strategies
        // ========================================
        BuiltinStrategy::JsonDecode => {
            use super::runtime::rt_func_ref;
            let ptr_type = ctx.module.target_config().pointer_type();

            let s = compile_expression(ctx, builder, &args[0])?;
            let s = ensure_naml_string(ctx, builder, s, &args[0])?;

            let slot_tag = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                4,
                4,
            ));
            let slot_value = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                8,
                8,
            ));

            let out_tag = builder.ins().stack_addr(ptr_type, slot_tag, 0);
            let out_value = builder.ins().stack_addr(ptr_type, slot_value, 0);

            let func_ref = rt_func_ref(ctx, builder, "naml_json_decode")?;
            builder.ins().call(func_ref, &[s, out_tag, out_value]);

            let tag = builder
                .ins()
                .load(types::I32, MemFlags::trusted(), out_tag, 0);
            let value = builder
                .ins()
                .load(types::I64, MemFlags::trusted(), out_value, 0);

            let success_block = builder.create_block();
            let error_block = builder.create_block();
            let merge_block = builder.create_block();
            builder.append_block_param(merge_block, types::I64);

            let tag_is_zero = builder.ins().icmp_imm(IntCC::Equal, tag, 0);
            builder
                .ins()
                .brif(tag_is_zero, success_block, &[], error_block, &[]);

            builder.switch_to_block(success_block);
            builder.seal_block(success_block);
            builder.ins().jump(merge_block, &[value]);

            builder.switch_to_block(error_block);
            builder.seal_block(error_block);
            use super::exceptions::throw_decode_error;
            throw_decode_error(ctx, builder, value)?;
            builder.ins().jump(merge_block, &[value]);

            builder.switch_to_block(merge_block);
            builder.seal_block(merge_block);

            let result = builder.block_params(merge_block)[0];
            Ok(result)
        }

        BuiltinStrategy::JsonEncode(runtime_fn) => {
            let json = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, runtime_fn, json)
        }

        BuiltinStrategy::JsonExists => {
            use super::runtime::rt_func_ref;

            let json = compile_expression(ctx, builder, &args[0])?;
            let key = compile_expression(ctx, builder, &args[1])?;
            let key = ensure_naml_string(ctx, builder, key, &args[1])?;

            let func_ref = rt_func_ref(ctx, builder, "naml_json_exists")?;
            let inst = builder.ins().call(func_ref, &[json, key]);
            let result = builder.inst_results(inst)[0];
            // Truncate i64 to i8 for bool type
            Ok(builder.ins().ireduce(cranelift::prelude::types::I8, result))
        }

        BuiltinStrategy::JsonPath => {
            use super::runtime::rt_func_ref;
            let ptr_type = ctx.module.target_config().pointer_type();

            let json = compile_expression(ctx, builder, &args[0])?;
            let path = compile_expression(ctx, builder, &args[1])?;
            let path = ensure_naml_string(ctx, builder, path, &args[1])?;

            let slot_tag = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                4,
                4,
            ));
            let slot_value = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                8,
                8,
            ));

            let out_tag = builder.ins().stack_addr(ptr_type, slot_tag, 0);
            let out_value = builder.ins().stack_addr(ptr_type, slot_value, 0);

            let func_ref = rt_func_ref(ctx, builder, "naml_json_path")?;
            builder
                .ins()
                .call(func_ref, &[json, path, out_tag, out_value]);

            let tag = builder
                .ins()
                .load(types::I32, MemFlags::trusted(), out_tag, 0);
            let value = builder
                .ins()
                .load(types::I64, MemFlags::trusted(), out_value, 0);

            let success_block = builder.create_block();
            let error_block = builder.create_block();
            let merge_block = builder.create_block();
            builder.append_block_param(merge_block, types::I64);

            let tag_is_zero = builder.ins().icmp_imm(IntCC::Equal, tag, 0);
            builder
                .ins()
                .brif(tag_is_zero, success_block, &[], error_block, &[]);

            builder.switch_to_block(success_block);
            builder.seal_block(success_block);
            builder.ins().jump(merge_block, &[value]);

            builder.switch_to_block(error_block);
            builder.seal_block(error_block);
            use super::exceptions::throw_path_error;
            throw_path_error(ctx, builder, path)?;
            builder.ins().jump(merge_block, &[value]);

            builder.switch_to_block(merge_block);
            builder.seal_block(merge_block);

            let result = builder.block_params(merge_block)[0];
            Ok(result)
        }

        BuiltinStrategy::JsonKeys => {
            let json = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_json_keys", json)
        }

        BuiltinStrategy::JsonCount => {
            use super::runtime::rt_func_ref;

            let json = compile_expression(ctx, builder, &args[0])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_json_count")?;
            let inst = builder.ins().call(func_ref, &[json]);
            Ok(builder.inst_results(inst)[0])
        }

        BuiltinStrategy::JsonGetType => {
            use super::runtime::rt_func_ref;

            let json = compile_expression(ctx, builder, &args[0])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_json_get_type")?;
            let inst = builder.ins().call(func_ref, &[json]);
            Ok(builder.inst_results(inst)[0])
        }

        BuiltinStrategy::JsonTypeName => {
            use super::runtime::rt_func_ref;

            let json = compile_expression(ctx, builder, &args[0])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_json_type_name")?;
            let inst = builder.ins().call(func_ref, &[json]);
            Ok(builder.inst_results(inst)[0])
        }

        BuiltinStrategy::JsonIsNull => {
            use super::runtime::rt_func_ref;

            let json = compile_expression(ctx, builder, &args[0])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_json_is_null")?;
            let inst = builder.ins().call(func_ref, &[json]);
            let result = builder.inst_results(inst)[0];
            // Truncate i64 to i8 for bool type
            Ok(builder.ins().ireduce(cranelift::prelude::types::I8, result))
        }

        // ========================================
        // TOML strategies
        // ========================================
        BuiltinStrategy::TomlDecode => {
            use super::runtime::rt_func_ref;
            let ptr_type = ctx.module.target_config().pointer_type();

            let s = compile_expression(ctx, builder, &args[0])?;
            let s = ensure_naml_string(ctx, builder, s, &args[0])?;

            let slot_tag = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                4,
                4,
            ));
            let slot_value = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                8,
                8,
            ));

            let out_tag = builder.ins().stack_addr(ptr_type, slot_tag, 0);
            let out_value = builder.ins().stack_addr(ptr_type, slot_value, 0);

            let func_ref = rt_func_ref(ctx, builder, "naml_encoding_toml_decode")?;
            builder.ins().call(func_ref, &[s, out_tag, out_value]);

            let tag = builder
                .ins()
                .load(types::I32, MemFlags::trusted(), out_tag, 0);
            let value = builder
                .ins()
                .load(types::I64, MemFlags::trusted(), out_value, 0);

            let success_block = builder.create_block();
            let error_block = builder.create_block();
            let merge_block = builder.create_block();
            builder.append_block_param(merge_block, types::I64);

            let tag_is_zero = builder.ins().icmp_imm(IntCC::Equal, tag, 0);
            builder
                .ins()
                .brif(tag_is_zero, success_block, &[], error_block, &[]);

            builder.switch_to_block(success_block);
            builder.seal_block(success_block);
            builder.ins().jump(merge_block, &[value]);

            builder.switch_to_block(error_block);
            builder.seal_block(error_block);
            use super::exceptions::throw_decode_error;
            throw_decode_error(ctx, builder, value)?;
            builder.ins().jump(merge_block, &[value]);

            builder.switch_to_block(merge_block);
            builder.seal_block(merge_block);

            let result = builder.block_params(merge_block)[0];
            Ok(result)
        }

        BuiltinStrategy::TomlEncode(runtime_fn) => {
            use super::runtime::rt_func_ref;
            let ptr_type = ctx.module.target_config().pointer_type();

            let json = compile_expression(ctx, builder, &args[0])?;

            let slot_tag = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                4,
                4,
            ));
            let slot_value = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                8,
                8,
            ));

            let out_tag = builder.ins().stack_addr(ptr_type, slot_tag, 0);
            let out_value = builder.ins().stack_addr(ptr_type, slot_value, 0);

            let func_ref = rt_func_ref(ctx, builder, runtime_fn)?;
            builder.ins().call(func_ref, &[json, out_tag, out_value]);

            let tag = builder
                .ins()
                .load(types::I32, MemFlags::trusted(), out_tag, 0);
            let value = builder
                .ins()
                .load(types::I64, MemFlags::trusted(), out_value, 0);

            let success_block = builder.create_block();
            let error_block = builder.create_block();
            let merge_block = builder.create_block();
            builder.append_block_param(merge_block, types::I64);

            let tag_is_zero = builder.ins().icmp_imm(IntCC::Equal, tag, 0);
            builder
                .ins()
                .brif(tag_is_zero, success_block, &[], error_block, &[]);

            builder.switch_to_block(success_block);
            builder.seal_block(success_block);
            builder.ins().jump(merge_block, &[value]);

            builder.switch_to_block(error_block);
            builder.seal_block(error_block);
            use super::exceptions::throw_encode_error;
            throw_encode_error(ctx, builder)?;
            builder.ins().jump(merge_block, &[value]);

            builder.switch_to_block(merge_block);
            builder.seal_block(merge_block);

            let result = builder.block_params(merge_block)[0];
            Ok(result)
        }

        // ========================================
        // YAML strategies
        // ========================================
        BuiltinStrategy::YamlDecode => {
            use super::runtime::rt_func_ref;
            let ptr_type = ctx.module.target_config().pointer_type();

            let s = compile_expression(ctx, builder, &args[0])?;
            let s = ensure_naml_string(ctx, builder, s, &args[0])?;

            let slot_tag = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                4,
                4,
            ));
            let slot_value = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                8,
                8,
            ));

            let out_tag = builder.ins().stack_addr(ptr_type, slot_tag, 0);
            let out_value = builder.ins().stack_addr(ptr_type, slot_value, 0);

            let func_ref = rt_func_ref(ctx, builder, "naml_encoding_yaml_decode")?;
            builder.ins().call(func_ref, &[s, out_tag, out_value]);

            let tag = builder
                .ins()
                .load(types::I32, MemFlags::trusted(), out_tag, 0);
            let value = builder
                .ins()
                .load(types::I64, MemFlags::trusted(), out_value, 0);

            let success_block = builder.create_block();
            let error_block = builder.create_block();
            let merge_block = builder.create_block();
            builder.append_block_param(merge_block, types::I64);

            let tag_is_zero = builder.ins().icmp_imm(IntCC::Equal, tag, 0);
            builder
                .ins()
                .brif(tag_is_zero, success_block, &[], error_block, &[]);

            builder.switch_to_block(success_block);
            builder.seal_block(success_block);
            builder.ins().jump(merge_block, &[value]);

            builder.switch_to_block(error_block);
            builder.seal_block(error_block);
            use super::exceptions::throw_decode_error;
            throw_decode_error(ctx, builder, value)?;
            builder.ins().jump(merge_block, &[value]);

            builder.switch_to_block(merge_block);
            builder.seal_block(merge_block);

            let result = builder.block_params(merge_block)[0];
            Ok(result)
        }

        BuiltinStrategy::YamlEncode => {
            use super::runtime::rt_func_ref;
            let ptr_type = ctx.module.target_config().pointer_type();

            let json = compile_expression(ctx, builder, &args[0])?;

            let slot_tag = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                4,
                4,
            ));
            let slot_value = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                8,
                8,
            ));

            let out_tag = builder.ins().stack_addr(ptr_type, slot_tag, 0);
            let out_value = builder.ins().stack_addr(ptr_type, slot_value, 0);

            let func_ref = rt_func_ref(ctx, builder, "naml_encoding_yaml_encode")?;
            builder.ins().call(func_ref, &[json, out_tag, out_value]);

            let tag = builder
                .ins()
                .load(types::I32, MemFlags::trusted(), out_tag, 0);
            let value = builder
                .ins()
                .load(types::I64, MemFlags::trusted(), out_value, 0);

            let success_block = builder.create_block();
            let error_block = builder.create_block();
            let merge_block = builder.create_block();
            builder.append_block_param(merge_block, types::I64);

            let tag_is_zero = builder.ins().icmp_imm(IntCC::Equal, tag, 0);
            builder
                .ins()
                .brif(tag_is_zero, success_block, &[], error_block, &[]);

            builder.switch_to_block(success_block);
            builder.seal_block(success_block);
            builder.ins().jump(merge_block, &[value]);

            builder.switch_to_block(error_block);
            builder.seal_block(error_block);
            use super::exceptions::throw_encode_error;
            throw_encode_error(ctx, builder)?;
            builder.ins().jump(merge_block, &[value]);

            builder.switch_to_block(merge_block);
            builder.seal_block(merge_block);

            let result = builder.block_params(merge_block)[0];
            Ok(result)
        }

        // ========================================
        // Networking strategies
        // ========================================
        // TCP Server
        BuiltinStrategy::NetTcpListen => {
            let addr = compile_expression(ctx, builder, &args[0])?;
            let addr = ensure_naml_string(ctx, builder, addr, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_net_tcp_server_listen", addr)
        }

        BuiltinStrategy::NetTcpAccept => {
            let listener = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_net_tcp_server_accept", listener)
        }

        BuiltinStrategy::NetTcpServerClose => {
            use super::runtime::rt_func_ref;
            let listener = compile_expression(ctx, builder, &args[0])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_net_tcp_server_close")?;
            builder.ins().call(func_ref, &[listener]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::NetTcpServerLocalAddr => {
            let listener = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_net_tcp_server_local_addr", listener)
        }

        // TCP Client
        BuiltinStrategy::NetTcpConnect => {
            let addr = compile_expression(ctx, builder, &args[0])?;
            let addr = ensure_naml_string(ctx, builder, addr, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_net_tcp_client_connect", addr)
        }

        BuiltinStrategy::NetTcpRead => {
            let socket = compile_expression(ctx, builder, &args[0])?;
            let size = compile_expression(ctx, builder, &args[1])?;
            call_two_arg_ptr_runtime(ctx, builder, "naml_net_tcp_client_read", socket, size)
        }

        BuiltinStrategy::NetTcpReadAll => {
            let socket = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_net_tcp_client_read_all", socket)
        }

        BuiltinStrategy::NetTcpWrite => {
            use super::runtime::rt_func_ref;
            let socket = compile_expression(ctx, builder, &args[0])?;
            let data = compile_expression(ctx, builder, &args[1])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_net_tcp_client_write")?;
            builder.ins().call(func_ref, &[socket, data]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::NetTcpClientClose => {
            use super::runtime::rt_func_ref;
            let socket = compile_expression(ctx, builder, &args[0])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_net_tcp_client_close")?;
            builder.ins().call(func_ref, &[socket]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::NetTcpSetTimeout => {
            use super::runtime::rt_func_ref;
            let socket = compile_expression(ctx, builder, &args[0])?;
            let ms = compile_expression(ctx, builder, &args[1])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_net_tcp_client_set_timeout")?;
            builder.ins().call(func_ref, &[socket, ms]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::NetTcpPeerAddr => {
            let socket = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_net_tcp_socket_peer_addr", socket)
        }

        // UDP
        BuiltinStrategy::NetUdpBind => {
            let addr = compile_expression(ctx, builder, &args[0])?;
            let addr = ensure_naml_string(ctx, builder, addr, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_net_udp_bind", addr)
        }

        BuiltinStrategy::NetUdpSend => {
            use super::runtime::rt_func_ref;
            let socket = compile_expression(ctx, builder, &args[0])?;
            let data = compile_expression(ctx, builder, &args[1])?;
            let addr = compile_expression(ctx, builder, &args[2])?;
            let addr = ensure_naml_string(ctx, builder, addr, &args[2])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_net_udp_send")?;
            builder.ins().call(func_ref, &[socket, data, addr]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::NetUdpReceive => {
            let socket = compile_expression(ctx, builder, &args[0])?;
            let size = compile_expression(ctx, builder, &args[1])?;
            call_two_arg_ptr_runtime(ctx, builder, "naml_net_udp_receive", socket, size)
        }

        BuiltinStrategy::NetUdpClose => {
            use super::runtime::rt_func_ref;
            let socket = compile_expression(ctx, builder, &args[0])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_net_udp_close")?;
            builder.ins().call(func_ref, &[socket]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::NetUdpLocalAddr => {
            let socket = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_net_udp_local_addr", socket)
        }

        // HTTP Client (all methods accept optional headers)
        BuiltinStrategy::NetHttpGet => {
            let url = compile_expression(ctx, builder, &args[0])?;
            let url = ensure_naml_string(ctx, builder, url, &args[0])?;
            let headers = compile_expression(ctx, builder, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, "naml_net_http_client_get", url, headers)
        }

        BuiltinStrategy::NetHttpPost => {
            let url = compile_expression(ctx, builder, &args[0])?;
            let url = ensure_naml_string(ctx, builder, url, &args[0])?;
            let body = compile_expression(ctx, builder, &args[1])?;
            let headers = compile_expression(ctx, builder, &args[2])?;
            call_three_arg_int_runtime(
                ctx,
                builder,
                "naml_net_http_client_post",
                url,
                body,
                headers,
            )
        }

        BuiltinStrategy::NetHttpPut => {
            let url = compile_expression(ctx, builder, &args[0])?;
            let url = ensure_naml_string(ctx, builder, url, &args[0])?;
            let body = compile_expression(ctx, builder, &args[1])?;
            let headers = compile_expression(ctx, builder, &args[2])?;
            call_three_arg_int_runtime(ctx, builder, "naml_net_http_client_put", url, body, headers)
        }

        BuiltinStrategy::NetHttpPatch => {
            let url = compile_expression(ctx, builder, &args[0])?;
            let url = ensure_naml_string(ctx, builder, url, &args[0])?;
            let body = compile_expression(ctx, builder, &args[1])?;
            let headers = compile_expression(ctx, builder, &args[2])?;
            call_three_arg_int_runtime(
                ctx,
                builder,
                "naml_net_http_client_patch",
                url,
                body,
                headers,
            )
        }

        BuiltinStrategy::NetHttpDelete => {
            let url = compile_expression(ctx, builder, &args[0])?;
            let url = ensure_naml_string(ctx, builder, url, &args[0])?;
            let headers = compile_expression(ctx, builder, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, "naml_net_http_client_delete", url, headers)
        }

        BuiltinStrategy::NetHttpSetTimeout => {
            use super::runtime::rt_func_ref;
            let ms = compile_expression(ctx, builder, &args[0])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_net_http_client_set_timeout")?;
            builder.ins().call(func_ref, &[ms]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::NetHttpStatus => {
            let response = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_net_http_response_get_status", response)
        }

        BuiltinStrategy::NetHttpBody => {
            let response = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_ptr_runtime(
                ctx,
                builder,
                "naml_net_http_response_get_body_bytes",
                response,
            )
        }

        // ========================================
        // HTTP Server strategies
        // ========================================
        BuiltinStrategy::NetHttpServerOpenRouter => {
            call_int_runtime(ctx, builder, "naml_net_http_server_open_router")
        }

        BuiltinStrategy::NetHttpServerGet => {
            let router = compile_expression(ctx, builder, &args[0])?;
            let pattern = compile_expression(ctx, builder, &args[1])?;
            let pattern = ensure_naml_string(ctx, builder, pattern, &args[1])?;
            let handler_closure = compile_expression(ctx, builder, &args[2])?;
            let handler = builder.ins().load(cranelift::prelude::types::I64, MemFlags::new(), handler_closure, 0);
            call_three_arg_void_runtime(ctx, builder, "naml_net_http_server_get", router, pattern, handler)
        }

        BuiltinStrategy::NetHttpServerPost => {
            let router = compile_expression(ctx, builder, &args[0])?;
            let pattern = compile_expression(ctx, builder, &args[1])?;
            let pattern = ensure_naml_string(ctx, builder, pattern, &args[1])?;
            let handler_closure = compile_expression(ctx, builder, &args[2])?;
            let handler = builder.ins().load(cranelift::prelude::types::I64, MemFlags::new(), handler_closure, 0);
            call_three_arg_void_runtime(ctx, builder, "naml_net_http_server_post", router, pattern, handler)
        }

        BuiltinStrategy::NetHttpServerPut => {
            let router = compile_expression(ctx, builder, &args[0])?;
            let pattern = compile_expression(ctx, builder, &args[1])?;
            let pattern = ensure_naml_string(ctx, builder, pattern, &args[1])?;
            let handler_closure = compile_expression(ctx, builder, &args[2])?;
            let handler = builder.ins().load(cranelift::prelude::types::I64, MemFlags::new(), handler_closure, 0);
            call_three_arg_void_runtime(ctx, builder, "naml_net_http_server_put", router, pattern, handler)
        }

        BuiltinStrategy::NetHttpServerPatch => {
            let router = compile_expression(ctx, builder, &args[0])?;
            let pattern = compile_expression(ctx, builder, &args[1])?;
            let pattern = ensure_naml_string(ctx, builder, pattern, &args[1])?;
            let handler_closure = compile_expression(ctx, builder, &args[2])?;
            let handler = builder.ins().load(cranelift::prelude::types::I64, MemFlags::new(), handler_closure, 0);
            call_three_arg_void_runtime(ctx, builder, "naml_net_http_server_patch", router, pattern, handler)
        }

        BuiltinStrategy::NetHttpServerDelete => {
            let router = compile_expression(ctx, builder, &args[0])?;
            let pattern = compile_expression(ctx, builder, &args[1])?;
            let pattern = ensure_naml_string(ctx, builder, pattern, &args[1])?;
            let handler_closure = compile_expression(ctx, builder, &args[2])?;
            let handler = builder.ins().load(cranelift::prelude::types::I64, MemFlags::new(), handler_closure, 0);
            call_three_arg_void_runtime(ctx, builder, "naml_net_http_server_delete", router, pattern, handler)
        }

        BuiltinStrategy::NetHttpServerWith => {
            let router = compile_expression(ctx, builder, &args[0])?;
            let middleware = compile_expression(ctx, builder, &args[1])?;
            call_two_arg_runtime(ctx, builder, "naml_net_http_server_with", router, middleware)
        }

        BuiltinStrategy::NetHttpServerGroup => {
            let router = compile_expression(ctx, builder, &args[0])?;
            let prefix = compile_expression(ctx, builder, &args[1])?;
            let prefix = ensure_naml_string(ctx, builder, prefix, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, "naml_net_http_server_group", router, prefix)
        }

        BuiltinStrategy::NetHttpServerMount => {
            let router = compile_expression(ctx, builder, &args[0])?;
            let prefix = compile_expression(ctx, builder, &args[1])?;
            let prefix = ensure_naml_string(ctx, builder, prefix, &args[1])?;
            let sub_router = compile_expression(ctx, builder, &args[2])?;
            call_three_arg_void_runtime(ctx, builder, "naml_net_http_server_mount", router, prefix, sub_router)
        }

        BuiltinStrategy::NetHttpServerServe => {
            let addr = compile_expression(ctx, builder, &args[0])?;
            let addr = ensure_naml_string(ctx, builder, addr, &args[0])?;
            let router = compile_expression(ctx, builder, &args[1])?;
            call_two_arg_runtime(ctx, builder, "naml_net_http_server_serve", addr, router)
        }

        BuiltinStrategy::NetHttpServerTextResponse => {
            let status = compile_expression(ctx, builder, &args[0])?;
            let body = compile_expression(ctx, builder, &args[1])?;
            let body = ensure_naml_string(ctx, builder, body, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, "naml_net_http_server_text_response", status, body)
        }

        // ========================================
        // SQLite database strategies
        // ========================================
        BuiltinStrategy::SqliteOpen => {
            let path = compile_expression(ctx, builder, &args[0])?;
            let path = ensure_naml_string(ctx, builder, path, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_db_sqlite_open", path)
        }

        BuiltinStrategy::SqliteOpenMemory => {
            call_int_runtime(ctx, builder, "naml_db_sqlite_open_memory")
        }

        BuiltinStrategy::SqliteClose => {
            use super::runtime::rt_func_ref;
            let handle = compile_expression(ctx, builder, &args[0])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_db_sqlite_close")?;
            builder.ins().call(func_ref, &[handle]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::SqliteExec => {
            use super::runtime::rt_func_ref;
            let handle = compile_expression(ctx, builder, &args[0])?;
            let sql = compile_expression(ctx, builder, &args[1])?;
            let sql = ensure_naml_string(ctx, builder, sql, &args[1])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_db_sqlite_exec")?;
            builder.ins().call(func_ref, &[handle, sql]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::SqliteQuery => {
            use super::runtime::rt_func_ref;
            let handle = compile_expression(ctx, builder, &args[0])?;
            let sql = compile_expression(ctx, builder, &args[1])?;
            let sql = ensure_naml_string(ctx, builder, sql, &args[1])?;
            let params = compile_expression(ctx, builder, &args[2])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_db_sqlite_query")?;
            let call = builder.ins().call(func_ref, &[handle, sql, params]);
            Ok(builder.inst_results(call)[0])
        }

        BuiltinStrategy::SqliteRowCount => {
            let rows = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_db_sqlite_row_count", rows)
        }

        BuiltinStrategy::SqliteRowAt => {
            let rows = compile_expression(ctx, builder, &args[0])?;
            let index = compile_expression(ctx, builder, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, "naml_db_sqlite_row_at", rows, index)
        }

        BuiltinStrategy::SqliteGetString => {
            let row = compile_expression(ctx, builder, &args[0])?;
            let col = compile_expression(ctx, builder, &args[1])?;
            let col = ensure_naml_string(ctx, builder, col, &args[1])?;
            call_two_arg_ptr_runtime(ctx, builder, "naml_db_sqlite_get_string", row, col)
        }

        BuiltinStrategy::SqliteGetInt => {
            let row = compile_expression(ctx, builder, &args[0])?;
            let col = compile_expression(ctx, builder, &args[1])?;
            let col = ensure_naml_string(ctx, builder, col, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, "naml_db_sqlite_get_int", row, col)
        }

        BuiltinStrategy::SqliteGetFloat => {
            use super::runtime::rt_func_ref;
            let row = compile_expression(ctx, builder, &args[0])?;
            let col = compile_expression(ctx, builder, &args[1])?;
            let col = ensure_naml_string(ctx, builder, col, &args[1])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_db_sqlite_get_float")?;
            let call = builder.ins().call(func_ref, &[row, col]);
            Ok(builder.inst_results(call)[0])
        }

        BuiltinStrategy::SqliteGetBool => {
            let row = compile_expression(ctx, builder, &args[0])?;
            let col = compile_expression(ctx, builder, &args[1])?;
            let col = ensure_naml_string(ctx, builder, col, &args[1])?;
            let i64_val = call_two_arg_int_runtime(ctx, builder, "naml_db_sqlite_get_bool", row, col)?;
            Ok(builder.ins().ireduce(types::I8, i64_val))
        }

        BuiltinStrategy::SqliteIsNull => {
            let row = compile_expression(ctx, builder, &args[0])?;
            let col = compile_expression(ctx, builder, &args[1])?;
            let col = ensure_naml_string(ctx, builder, col, &args[1])?;
            let i64_val = call_two_arg_int_runtime(ctx, builder, "naml_db_sqlite_is_null", row, col)?;
            Ok(builder.ins().ireduce(types::I8, i64_val))
        }

        BuiltinStrategy::SqliteColumns => {
            let rows = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_ptr_runtime(ctx, builder, "naml_db_sqlite_columns", rows)
        }

        BuiltinStrategy::SqliteColumnCount => {
            let rows = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_db_sqlite_column_count", rows)
        }

        BuiltinStrategy::SqliteBegin => {
            use super::runtime::rt_func_ref;
            let handle = compile_expression(ctx, builder, &args[0])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_db_sqlite_begin")?;
            builder.ins().call(func_ref, &[handle]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::SqliteCommit => {
            use super::runtime::rt_func_ref;
            let handle = compile_expression(ctx, builder, &args[0])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_db_sqlite_commit")?;
            builder.ins().call(func_ref, &[handle]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::SqliteRollback => {
            use super::runtime::rt_func_ref;
            let handle = compile_expression(ctx, builder, &args[0])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_db_sqlite_rollback")?;
            builder.ins().call(func_ref, &[handle]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::SqlitePrepare => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            let sql = compile_expression(ctx, builder, &args[1])?;
            let sql = ensure_naml_string(ctx, builder, sql, &args[1])?;
            call_two_arg_int_runtime(ctx, builder, "naml_db_sqlite_prepare", handle, sql)
        }

        BuiltinStrategy::SqliteBindString => {
            use super::runtime::rt_func_ref;
            let stmt = compile_expression(ctx, builder, &args[0])?;
            let index = compile_expression(ctx, builder, &args[1])?;
            let val = compile_expression(ctx, builder, &args[2])?;
            let val = ensure_naml_string(ctx, builder, val, &args[2])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_db_sqlite_bind_string")?;
            builder.ins().call(func_ref, &[stmt, index, val]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::SqliteBindInt => {
            use super::runtime::rt_func_ref;
            let stmt = compile_expression(ctx, builder, &args[0])?;
            let index = compile_expression(ctx, builder, &args[1])?;
            let val = compile_expression(ctx, builder, &args[2])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_db_sqlite_bind_int")?;
            builder.ins().call(func_ref, &[stmt, index, val]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::SqliteBindFloat => {
            use super::runtime::rt_func_ref;
            let stmt = compile_expression(ctx, builder, &args[0])?;
            let index = compile_expression(ctx, builder, &args[1])?;
            let val = compile_expression(ctx, builder, &args[2])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_db_sqlite_bind_float")?;
            builder.ins().call(func_ref, &[stmt, index, val]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::SqliteStep => {
            use super::runtime::rt_func_ref;
            let stmt = compile_expression(ctx, builder, &args[0])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_db_sqlite_step")?;
            builder.ins().call(func_ref, &[stmt]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::SqliteReset => {
            use super::runtime::rt_func_ref;
            let stmt = compile_expression(ctx, builder, &args[0])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_db_sqlite_reset")?;
            builder.ins().call(func_ref, &[stmt]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::SqliteFinalize => {
            use super::runtime::rt_func_ref;
            let stmt = compile_expression(ctx, builder, &args[0])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_db_sqlite_finalize")?;
            builder.ins().call(func_ref, &[stmt]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::SqliteChanges => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_db_sqlite_changes", handle)
        }

        BuiltinStrategy::SqliteLastInsertId => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_db_sqlite_last_insert_id", handle)
        }

        // ========================================
        // Timers module
        // ========================================
        BuiltinStrategy::TimerSetTimeout => {
            let closure = compile_expression(ctx, builder, &args[0])?;
            let ms = compile_expression(ctx, builder, &args[1])?;
            let func_ptr = builder.ins().load(types::I64, MemFlags::new(), closure, 0);
            let data_ptr = builder.ins().load(types::I64, MemFlags::new(), closure, 8);
            let data_size = builder.ins().load(types::I64, MemFlags::new(), closure, 16);
            let func_ref = rt_func_ref(ctx, builder, "naml_timers_set_timeout")?;
            let call = builder.ins().call(func_ref, &[func_ptr, data_ptr, data_size, ms]);
            Ok(builder.inst_results(call)[0])
        }

        BuiltinStrategy::TimerCancelTimeout => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_timers_cancel_timeout")?;
            builder.ins().call(func_ref, &[handle]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::TimerSetInterval => {
            let closure = compile_expression(ctx, builder, &args[0])?;
            let ms = compile_expression(ctx, builder, &args[1])?;
            let func_ptr = builder.ins().load(types::I64, MemFlags::new(), closure, 0);
            let data_ptr = builder.ins().load(types::I64, MemFlags::new(), closure, 8);
            let data_size = builder.ins().load(types::I64, MemFlags::new(), closure, 16);
            let func_ref = rt_func_ref(ctx, builder, "naml_timers_set_interval")?;
            let call = builder.ins().call(func_ref, &[func_ptr, data_ptr, data_size, ms]);
            Ok(builder.inst_results(call)[0])
        }

        BuiltinStrategy::TimerCancelInterval => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_timers_cancel_interval")?;
            builder.ins().call(func_ref, &[handle]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::TimerSchedule => {
            let closure = compile_expression(ctx, builder, &args[0])?;
            let cron_expr = compile_expression(ctx, builder, &args[1])?;
            let cron_expr = ensure_naml_string(ctx, builder, cron_expr, &args[1])?;
            let func_ptr = builder.ins().load(types::I64, MemFlags::new(), closure, 0);
            let data_ptr = builder.ins().load(types::I64, MemFlags::new(), closure, 8);
            let data_size = builder.ins().load(types::I64, MemFlags::new(), closure, 16);
            let func_ref = rt_func_ref(ctx, builder, "naml_timers_schedule")?;
            let call = builder.ins().call(func_ref, &[func_ptr, data_ptr, data_size, cron_expr]);
            Ok(builder.inst_results(call)[0])
        }

        BuiltinStrategy::TimerCancelSchedule => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            let func_ref = rt_func_ref(ctx, builder, "naml_timers_cancel_schedule")?;
            builder.ins().call(func_ref, &[handle]);
            Ok(builder.ins().iconst(types::I64, 0))
        }

        BuiltinStrategy::TimerNextRun => {
            let handle = compile_expression(ctx, builder, &args[0])?;
            call_one_arg_int_runtime(ctx, builder, "naml_timers_next_run", handle)
        }
    }
}

fn get_atomic_type_suffix_from_arg(ctx: &CompileContext<'_>, arg: &Expression<'_>) -> &'static str {
    use crate::source::Spanned;
    if let Some(ty) = ctx.annotations.get_type(arg.span()) {
        let resolved = ty.resolve();
        match &resolved {
            crate::typechecker::types::Type::Uint => "uint",
            crate::typechecker::types::Type::Bool => "bool",
            _ => "int",
        }
    } else {
        "int"
    }
}

fn get_atomic_type_suffix_from_atomic_arg(ctx: &CompileContext<'_>, arg: &Expression<'_>) -> &'static str {
    use crate::source::Spanned;
    if let Some(ty) = ctx.annotations.get_type(arg.span()) {
        let resolved = ty.resolve();
        match &resolved {
            crate::typechecker::types::Type::Atomic(inner) => match inner.as_ref() {
                crate::typechecker::types::Type::Uint => "uint",
                crate::typechecker::types::Type::Bool => "bool",
                _ => "int",
            },
            _ => "int",
        }
    } else {
        "int"
    }
}
