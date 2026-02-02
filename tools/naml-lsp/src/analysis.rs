///
/// Document Analysis Module
///
/// This module integrates with namlc to provide semantic analysis
/// of naml source files. It reuses the lexer, parser, and type
/// checker from the compiler library.
///
/// Note: Due to thread-safety constraints (TypeVarRef uses Rc<RefCell>),
/// we extract diagnostic information at analysis time and discard the
/// full type information. Hover and completion features require future
/// work to make the compiler types Send+Sync safe.
///

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};

use tower_lsp::lsp_types::*;

use namlc::source::Span;
use namlc::typechecker::TypeError;
use namlc::{parse, tokenize, check_with_types, AstArena};

static STD_MODULE_MAP: LazyLock<HashMap<&'static str, Vec<&'static str>>> = LazyLock::new(|| {
    let mut m: HashMap<&'static str, Vec<&'static str>> = HashMap::new();
    let add = |m: &mut HashMap<&'static str, Vec<&'static str>>, name: &'static str, module: &'static str| {
        m.entry(name).or_default().push(module);
    };

    // std::random
    add(&mut m, "random", "std::random");
    add(&mut m, "random_float", "std::random");

    // std::io
    add(&mut m, "read_key", "std::io");
    add(&mut m, "clear_screen", "std::io");
    add(&mut m, "set_cursor", "std::io");
    add(&mut m, "hide_cursor", "std::io");
    add(&mut m, "show_cursor", "std::io");
    add(&mut m, "terminal_width", "std::io");
    add(&mut m, "terminal_height", "std::io");

    // std::threads
    add(&mut m, "join", "std::threads");
    add(&mut m, "open_channel", "std::threads");
    add(&mut m, "send", "std::threads");
    add(&mut m, "receive", "std::threads");
    add(&mut m, "close", "std::threads");
    add(&mut m, "with_mutex", "std::threads");
    add(&mut m, "with_rwlock", "std::threads");

    // std::datetime
    add(&mut m, "now_ms", "std::datetime");
    add(&mut m, "now_s", "std::datetime");
    add(&mut m, "year", "std::datetime");
    add(&mut m, "month", "std::datetime");
    add(&mut m, "day", "std::datetime");
    add(&mut m, "hour", "std::datetime");
    add(&mut m, "minute", "std::datetime");
    add(&mut m, "second", "std::datetime");
    add(&mut m, "day_of_week", "std::datetime");
    add(&mut m, "format_date", "std::datetime");

    // std::metrics
    add(&mut m, "perf_now", "std::metrics");
    add(&mut m, "elapsed_ms", "std::metrics");
    add(&mut m, "elapsed_us", "std::metrics");
    add(&mut m, "elapsed_ns", "std::metrics");

    // std::strings
    add(&mut m, "len", "std::strings");
    add(&mut m, "char_at", "std::strings");
    add(&mut m, "upper", "std::strings");
    add(&mut m, "lower", "std::strings");
    add(&mut m, "split", "std::strings");
    add(&mut m, "concat", "std::strings");
    add(&mut m, "has", "std::strings");
    add(&mut m, "starts_with", "std::strings");
    add(&mut m, "ends_with", "std::strings");
    add(&mut m, "replace", "std::strings");
    add(&mut m, "replace_all", "std::strings");
    add(&mut m, "ltrim", "std::strings");
    add(&mut m, "rtrim", "std::strings");
    add(&mut m, "substr", "std::strings");
    add(&mut m, "lpad", "std::strings");
    add(&mut m, "rpad", "std::strings");
    add(&mut m, "repeat", "std::strings");
    add(&mut m, "lines", "std::strings");
    add(&mut m, "chars", "std::strings");

    // std::path
    add(&mut m, "join", "std::path");
    add(&mut m, "normalize", "std::path");
    add(&mut m, "is_absolute", "std::path");
    add(&mut m, "is_relative", "std::path");
    add(&mut m, "has_root", "std::path");
    add(&mut m, "dirname", "std::path");
    add(&mut m, "basename", "std::path");
    add(&mut m, "extension", "std::path");
    add(&mut m, "stem", "std::path");
    add(&mut m, "with_extension", "std::path");
    add(&mut m, "components", "std::path");
    add(&mut m, "separator", "std::path");
    add(&mut m, "to_slash", "std::path");
    add(&mut m, "from_slash", "std::path");
    add(&mut m, "starts_with", "std::path");
    add(&mut m, "ends_with", "std::path");
    add(&mut m, "strip_prefix", "std::path");

    // std::fs
    add(&mut m, "exists", "std::fs");
    add(&mut m, "is_file", "std::fs");
    add(&mut m, "is_dir", "std::fs");
    add(&mut m, "list_dir", "std::fs");
    add(&mut m, "read_file", "std::fs");
    add(&mut m, "write_file", "std::fs");
    add(&mut m, "append_file", "std::fs");
    add(&mut m, "delete_file", "std::fs");
    add(&mut m, "create_dir", "std::fs");
    add(&mut m, "delete_dir", "std::fs");
    add(&mut m, "copy_file", "std::fs");
    add(&mut m, "move_file", "std::fs");
    add(&mut m, "file_size", "std::fs");
    add(&mut m, "dirname", "std::fs");
    add(&mut m, "basename", "std::fs");
    add(&mut m, "extension", "std::fs");
    add(&mut m, "join", "std::fs");

    // std::collections::arrays
    add(&mut m, "count", "std::collections::arrays");
    add(&mut m, "push", "std::collections::arrays");
    add(&mut m, "pop", "std::collections::arrays");
    add(&mut m, "shift", "std::collections::arrays");
    add(&mut m, "fill", "std::collections::arrays");
    add(&mut m, "clear", "std::collections::arrays");
    add(&mut m, "get", "std::collections::arrays");
    add(&mut m, "first", "std::collections::arrays");
    add(&mut m, "last", "std::collections::arrays");
    add(&mut m, "sum", "std::collections::arrays");
    add(&mut m, "min", "std::collections::arrays");
    add(&mut m, "max", "std::collections::arrays");
    add(&mut m, "reversed", "std::collections::arrays");
    add(&mut m, "take", "std::collections::arrays");
    add(&mut m, "drop", "std::collections::arrays");
    add(&mut m, "slice", "std::collections::arrays");
    add(&mut m, "index_of", "std::collections::arrays");
    add(&mut m, "contains", "std::collections::arrays");
    add(&mut m, "any", "std::collections::arrays");
    add(&mut m, "all", "std::collections::arrays");
    add(&mut m, "count_if", "std::collections::arrays");
    add(&mut m, "apply", "std::collections::arrays");
    add(&mut m, "where", "std::collections::arrays");
    add(&mut m, "find", "std::collections::arrays");
    add(&mut m, "find_index", "std::collections::arrays");
    add(&mut m, "fold", "std::collections::arrays");
    add(&mut m, "flatten", "std::collections::arrays");
    add(&mut m, "sort", "std::collections::arrays");
    add(&mut m, "sort_by", "std::collections::arrays");
    add(&mut m, "insert", "std::collections::arrays");
    add(&mut m, "remove_at", "std::collections::arrays");
    add(&mut m, "remove", "std::collections::arrays");
    add(&mut m, "swap", "std::collections::arrays");
    add(&mut m, "unique", "std::collections::arrays");
    add(&mut m, "compact", "std::collections::arrays");
    add(&mut m, "last_index_of", "std::collections::arrays");
    add(&mut m, "find_last", "std::collections::arrays");
    add(&mut m, "find_last_index", "std::collections::arrays");
    add(&mut m, "concat", "std::collections::arrays");
    add(&mut m, "zip", "std::collections::arrays");
    add(&mut m, "unzip", "std::collections::arrays");
    add(&mut m, "chunk", "std::collections::arrays");
    add(&mut m, "partition", "std::collections::arrays");
    add(&mut m, "intersect", "std::collections::arrays");
    add(&mut m, "diff", "std::collections::arrays");
    add(&mut m, "union", "std::collections::arrays");
    add(&mut m, "take_while", "std::collections::arrays");
    add(&mut m, "drop_while", "std::collections::arrays");
    add(&mut m, "reject", "std::collections::arrays");
    add(&mut m, "flat_apply", "std::collections::arrays");
    add(&mut m, "scan", "std::collections::arrays");
    add(&mut m, "shuffle", "std::collections::arrays");
    add(&mut m, "sample", "std::collections::arrays");
    add(&mut m, "sample_n", "std::collections::arrays");

    // std::collections::maps
    add(&mut m, "count", "std::collections::maps");
    add(&mut m, "contains_key", "std::collections::maps");
    add(&mut m, "remove", "std::collections::maps");
    add(&mut m, "clear", "std::collections::maps");
    add(&mut m, "keys", "std::collections::maps");
    add(&mut m, "values", "std::collections::maps");
    add(&mut m, "entries", "std::collections::maps");
    add(&mut m, "first_key", "std::collections::maps");
    add(&mut m, "first_value", "std::collections::maps");
    add(&mut m, "any", "std::collections::maps");
    add(&mut m, "all", "std::collections::maps");
    add(&mut m, "count_if", "std::collections::maps");
    add(&mut m, "fold", "std::collections::maps");
    add(&mut m, "transform", "std::collections::maps");
    add(&mut m, "where", "std::collections::maps");
    add(&mut m, "reject", "std::collections::maps");
    add(&mut m, "merge", "std::collections::maps");
    add(&mut m, "defaults", "std::collections::maps");
    add(&mut m, "intersect", "std::collections::maps");
    add(&mut m, "diff", "std::collections::maps");
    add(&mut m, "invert", "std::collections::maps");
    add(&mut m, "from_arrays", "std::collections::maps");
    add(&mut m, "from_entries", "std::collections::maps");

    // std::encoding::json
    add(&mut m, "decode", "std::encoding::json");
    add(&mut m, "encode", "std::encoding::json");
    add(&mut m, "encode_pretty", "std::encoding::json");
    add(&mut m, "get_type", "std::encoding::json");
    add(&mut m, "type_name", "std::encoding::json");
    add(&mut m, "is_null", "std::encoding::json");

    // std::encoding::base64
    add(&mut m, "encode", "std::encoding::base64");
    add(&mut m, "decode", "std::encoding::base64");

    // std::encoding::hex
    add(&mut m, "encode", "std::encoding::hex");
    add(&mut m, "decode", "std::encoding::hex");

    // std::encoding::url
    add(&mut m, "encode", "std::encoding::url");
    add(&mut m, "decode", "std::encoding::url");

    // std::net::tcp::server
    add(&mut m, "listen", "std::net::tcp::server");
    add(&mut m, "accept", "std::net::tcp::server");
    add(&mut m, "close", "std::net::tcp::server");
    add(&mut m, "local_addr", "std::net::tcp::server");

    // std::net::tcp::client
    add(&mut m, "connect", "std::net::tcp::client");
    add(&mut m, "write", "std::net::tcp::client");
    add(&mut m, "read", "std::net::tcp::client");
    add(&mut m, "close", "std::net::tcp::client");
    add(&mut m, "set_timeout", "std::net::tcp::client");
    add(&mut m, "peer_addr", "std::net::tcp::client");

    // std::net::udp
    add(&mut m, "bind", "std::net::udp");
    add(&mut m, "send_to", "std::net::udp");
    add(&mut m, "receive_from", "std::net::udp");
    add(&mut m, "close", "std::net::udp");
    add(&mut m, "local_addr", "std::net::udp");

    // std::net::http::client
    add(&mut m, "get", "std::net::http::client");
    add(&mut m, "post", "std::net::http::client");
    add(&mut m, "put", "std::net::http::client");
    add(&mut m, "patch", "std::net::http::client");
    add(&mut m, "delete", "std::net::http::client");
    add(&mut m, "set_timeout", "std::net::http::client");
    add(&mut m, "status", "std::net::http::client");
    add(&mut m, "body", "std::net::http::client");

    // std::net::http::router
    add(&mut m, "open_router", "std::net::http::router");
    add(&mut m, "get", "std::net::http::router");
    add(&mut m, "post", "std::net::http::router");
    add(&mut m, "put", "std::net::http::router");
    add(&mut m, "patch", "std::net::http::router");
    add(&mut m, "delete", "std::net::http::router");
    add(&mut m, "with", "std::net::http::router");
    add(&mut m, "group", "std::net::http::router");
    add(&mut m, "mount", "std::net::http::router");
    add(&mut m, "serve", "std::net::http::router");

    // std::net::http::middleware
    add(&mut m, "logger", "std::net::http::middleware");
    add(&mut m, "timeout", "std::net::http::middleware");
    add(&mut m, "recover", "std::net::http::middleware");
    add(&mut m, "cors", "std::net::http::middleware");
    add(&mut m, "rate_limit", "std::net::http::middleware");
    add(&mut m, "compress", "std::net::http::middleware");
    add(&mut m, "request_id", "std::net::http::middleware");

    m
});

#[derive(Clone, Debug)]
pub struct UndefinedSymbol {
    pub name: String,
    pub range: Range,
}

pub struct DocumentAnalysis {
    pub diagnostics: Vec<Diagnostic>,
    pub undefined_symbols: Vec<UndefinedSymbol>,
    pub source: Arc<str>,
    pub line_starts: Vec<u32>,
}

struct AnalysisContext {
    line_starts: Vec<u32>,
}

impl AnalysisContext {
    fn new(source: &str) -> Self {
        Self {
            line_starts: Self::compute_line_starts(source),
        }
    }

    fn compute_line_starts(source: &str) -> Vec<u32> {
        let mut starts = vec![0];
        for (i, c) in source.char_indices() {
            if c == '\n' {
                starts.push((i + 1) as u32);
            }
        }
        starts
    }

    fn offset_to_position(&self, offset: u32) -> Position {
        let line_idx = self.line_starts
            .binary_search(&offset)
            .unwrap_or_else(|i| i.saturating_sub(1));
        let line = line_idx as u32;
        let character = offset.saturating_sub(self.line_starts[line_idx]);
        Position { line, character }
    }

    fn span_to_range(&self, span: Span) -> Range {
        Range {
            start: self.offset_to_position(span.start),
            end: self.offset_to_position(span.end),
        }
    }
}

impl DocumentAnalysis {
    pub fn analyze(content: &str, source_dir: Option<PathBuf>) -> Self {
        let ctx = AnalysisContext::new(content);
        let mut diagnostics = Vec::new();
        let mut undefined_symbols = Vec::new();

        let (tokens, mut interner) = tokenize(content);
        let arena = AstArena::new();
        let parse_result = parse(&tokens, content, &arena);

        for err in &parse_result.errors {
            diagnostics.push(Diagnostic {
                range: ctx.span_to_range(err.span),
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("naml".to_string()),
                message: err.message.clone(),
                ..Default::default()
            });
        }

        if parse_result.errors.is_empty() {
            let type_result = check_with_types(&parse_result.ast, &mut interner, source_dir);

            for err in &type_result.errors {
                let range = ctx.span_to_range(err.span());
                diagnostics.push(Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::ERROR),
                    source: Some("naml".to_string()),
                    message: err.to_string(),
                    ..Default::default()
                });

                if let TypeError::UndefinedFunction { name, .. } = err {
                    undefined_symbols.push(UndefinedSymbol {
                        name: name.clone(),
                        range,
                    });
                }
            }
        }

        Self {
            diagnostics,
            undefined_symbols,
            source: content.into(),
            line_starts: ctx.line_starts,
        }
    }

    pub fn get_import_suggestions(&self, position: Position) -> Vec<(String, String)> {
        let mut suggestions = Vec::new();
        for sym in &self.undefined_symbols {
            if Self::position_in_range(position, sym.range) {
                if let Some(modules) = STD_MODULE_MAP.get(sym.name.as_str()) {
                    for module in modules {
                        suggestions.push((sym.name.clone(), module.to_string()));
                    }
                }
            }
        }
        suggestions
    }

    fn position_in_range(pos: Position, range: Range) -> bool {
        if pos.line < range.start.line || pos.line > range.end.line {
            return false;
        }
        if pos.line == range.start.line && pos.character < range.start.character {
            return false;
        }
        if pos.line == range.end.line && pos.character > range.end.character {
            return false;
        }
        true
    }

    pub fn diagnostics_to_lsp(&self) -> Vec<Diagnostic> {
        self.diagnostics.clone()
    }

    pub fn hover_at(&self, _position: Position) -> Option<Hover> {
        None
    }

    pub fn definition_at(&self, _uri: &Url, _position: Position) -> Option<GotoDefinitionResponse> {
        None
    }

    pub fn references_at(&self, _uri: &Url, _position: Position) -> Option<Vec<Location>> {
        None
    }

    pub fn document_symbols(&self) -> Option<DocumentSymbolResponse> {
        None
    }

    pub fn completions_at(&self, _content: &str, _position: Position) -> Option<CompletionResponse> {
        let keywords = [
            "fn", "var", "const", "pub", "struct", "enum", "interface", "exception",
            "if", "else", "while", "for", "loop", "break", "continue", "return",
            "switch", "case", "default", "spawn", "throw", "throws", "try", "catch",
            "use", "mod", "extern", "true", "false", "some", "none",
            "int", "uint", "float", "bool", "string", "bytes", "option", "map", "channel",
            "mutex", "rwlock", "locked", "rlocked", "wlocked", "implements", "in",
            "and", "or", "not", "as", "is", "self", "super",
        ];

        let builtins = [
            ("print", "Print to stdout (no newline). Supports {} placeholders."),
            ("println", "Print to stdout with newline. Supports {} placeholders."),
            ("fmt", "Format string with {} placeholders, returns result."),
            ("warn", "Print to stderr with warning: prefix."),
            ("error", "Print to stderr with error: prefix."),
            ("panic", "Print to stderr with panic: prefix, then abort."),
            ("read_line", "Read a line from stdin."),
            ("sleep", "Pause execution for ms milliseconds."),
        ];

        let mut items: Vec<CompletionItem> = keywords.iter().map(|kw| {
            CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                ..Default::default()
            }
        }).collect();

        for (name, doc) in builtins {
            items.push(CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(doc.to_string()),
                ..Default::default()
            });
        }

        Some(CompletionResponse::Array(items))
    }
}
