///
/// Completions Module
///
/// Provides context-aware code completions for the naml LSP.
/// Detects the cursor context (module path, dot access, bare identifier,
/// type position, use statement) and returns appropriate suggestions
/// from the stored LspSymbols.
///

use tower_lsp::lsp_types::*;

use crate::analysis::DocumentAnalysis;
use crate::lsp_symbols::{LspModule, LspSymbols, LspTypeDef};

const KEYWORDS: &[&str] = &[
    "fn", "var", "const", "pub", "struct", "enum", "interface", "exception",
    "if", "else", "while", "for", "loop", "break", "continue", "return",
    "switch", "case", "default", "spawn", "throw", "throws", "try", "catch",
    "use", "mod", "extern", "true", "false", "some", "none",
    "int", "uint", "float", "bool", "string", "bytes", "option", "map", "channel",
    "mutex", "rwlock", "atomic", "locked", "rlocked", "wlocked", "implements", "in",
    "and", "or", "not", "as", "is", "self", "super",
];

const BUILTINS: &[(&str, &str)] = &[
    ("print", "fn print(args: ...any)"),
    ("println", "fn println(args: ...any)"),
    ("fmt", "fn fmt(format: string, args: ...any) -> string"),
    ("warn", "fn warn(args: ...any)"),
    ("error", "fn error(args: ...any)"),
    ("panic", "fn panic(args: ...any)"),
];

enum CompletionContext {
    ModulePath(Vec<String>),
    UseModulePath(Vec<String>),
    UseBraceItems { segments: Vec<String>, partial: String },
    DotAccess(String),
    TypePosition,
    Bare(String),
}

impl DocumentAnalysis {
    pub fn completions_at(&self, content: &str, position: Position) -> Option<CompletionResponse> {
        let line_idx = position.line as usize;
        let lines: Vec<&str> = content.lines().collect();
        if line_idx >= lines.len() {
            return Some(self.bare_completions(""));
        }

        let line = lines[line_idx];
        let col = position.character as usize;
        let prefix = if col <= line.len() { &line[..col] } else { line };

        let ctx = detect_context(prefix);
        match ctx {
            CompletionContext::ModulePath(segments) => {
                self.module_path_completions(&segments)
            }
            CompletionContext::UseModulePath(segments) => {
                self.use_path_completions(&segments)
            }
            CompletionContext::UseBraceItems { segments, partial } => {
                self.use_brace_completions(&segments, &partial)
            }
            CompletionContext::DotAccess(var_name) => {
                self.dot_completions(&var_name)
            }
            CompletionContext::TypePosition => {
                Some(self.type_completions())
            }
            CompletionContext::Bare(partial) => {
                Some(self.bare_completions(&partial))
            }
        }
    }

    fn module_path_completions(&self, segments: &[String]) -> Option<CompletionResponse> {
        let symbols = self.symbols.as_ref()?;
        let module = navigate_to_module(&symbols.root, segments)?;
        let mut items = Vec::new();

        for sig in &module.functions {
            items.push(CompletionItem {
                label: sig.name.clone(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(sig.detail.clone()),
                ..Default::default()
            });
        }

        for (sub_name, _) in &module.submodules {
            items.push(CompletionItem {
                label: sub_name.clone(),
                kind: Some(CompletionItemKind::MODULE),
                ..Default::default()
            });
        }

        for typedef in &module.types {
            let kind = match typedef {
                LspTypeDef::Struct { .. } => CompletionItemKind::STRUCT,
                LspTypeDef::Enum { .. } => CompletionItemKind::ENUM,
                LspTypeDef::Interface { .. } => CompletionItemKind::INTERFACE,
                LspTypeDef::Exception { .. } => CompletionItemKind::STRUCT,
                LspTypeDef::TypeAlias { .. } => CompletionItemKind::TYPE_PARAMETER,
            };
            items.push(CompletionItem {
                label: typedef.name().to_string(),
                kind: Some(kind),
                ..Default::default()
            });
        }

        Some(CompletionResponse::Array(items))
    }

    fn use_path_completions(&self, segments: &[String]) -> Option<CompletionResponse> {
        let symbols = self.symbols.as_ref()?;
        let module = navigate_to_module(&symbols.root, segments)?;
        let mut items = Vec::new();

        for (sub_name, _) in &module.submodules {
            items.push(CompletionItem {
                label: sub_name.clone(),
                kind: Some(CompletionItemKind::MODULE),
                ..Default::default()
            });
        }

        if !module.functions.is_empty() || !module.types.is_empty() {
            items.push(CompletionItem {
                label: "*".to_string(),
                kind: Some(CompletionItemKind::MODULE),
                detail: Some("Import all".to_string()),
                ..Default::default()
            });
        }

        for sig in &module.functions {
            items.push(CompletionItem {
                label: sig.name.clone(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(sig.detail.clone()),
                ..Default::default()
            });
        }

        Some(CompletionResponse::Array(items))
    }

    fn use_brace_completions(&self, segments: &[String], partial: &str) -> Option<CompletionResponse> {
        let symbols = self.symbols.as_ref()?;
        let module = navigate_to_module(&symbols.root, segments)?;
        let mut items = Vec::new();

        for sig in &module.functions {
            if partial.is_empty() || sig.name.starts_with(partial) {
                items.push(CompletionItem {
                    label: sig.name.clone(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    detail: Some(sig.detail.clone()),
                    ..Default::default()
                });
            }
        }

        for typedef in &module.types {
            let name = typedef.name();
            if partial.is_empty() || name.starts_with(partial) {
                let kind = match typedef {
                    LspTypeDef::Struct { .. } => CompletionItemKind::STRUCT,
                    LspTypeDef::Enum { .. } => CompletionItemKind::ENUM,
                    LspTypeDef::Interface { .. } => CompletionItemKind::INTERFACE,
                    LspTypeDef::Exception { .. } => CompletionItemKind::STRUCT,
                    LspTypeDef::TypeAlias { .. } => CompletionItemKind::TYPE_PARAMETER,
                };
                items.push(CompletionItem {
                    label: name.to_string(),
                    kind: Some(kind),
                    ..Default::default()
                });
            }
        }

        Some(CompletionResponse::Array(items))
    }

    fn dot_completions(&self, var_name: &str) -> Option<CompletionResponse> {
        let symbols = self.symbols.as_ref()?;
        let mut items = Vec::new();

        for typedef in &symbols.types {
            if typedef.name() == var_name {
                if let LspTypeDef::Struct { fields, .. } = typedef {
                    for field in fields {
                        items.push(CompletionItem {
                            label: field.name.clone(),
                            kind: Some(CompletionItemKind::FIELD),
                            detail: Some(field.type_str.clone()),
                            ..Default::default()
                        });
                    }
                }
            }
        }

        if let Some(methods) = symbols.methods.get(var_name) {
            for method in methods {
                items.push(CompletionItem {
                    label: method.name.clone(),
                    kind: Some(CompletionItemKind::METHOD),
                    detail: Some(method.detail.clone()),
                    ..Default::default()
                });
            }
        }

        if !items.is_empty() {
            Some(CompletionResponse::Array(items))
        } else {
            None
        }
    }

    fn type_completions(&self) -> CompletionResponse {
        let primitive_types = [
            "int", "uint", "float", "bool", "string", "bytes",
            "option", "map", "channel", "mutex", "rwlock", "atomic",
        ];

        let mut items: Vec<CompletionItem> = primitive_types.iter().map(|t| {
            CompletionItem {
                label: t.to_string(),
                kind: Some(CompletionItemKind::TYPE_PARAMETER),
                ..Default::default()
            }
        }).collect();

        if let Some(symbols) = &self.symbols {
            for typedef in &symbols.types {
                let kind = match typedef {
                    LspTypeDef::Struct { .. } => CompletionItemKind::STRUCT,
                    LspTypeDef::Enum { .. } => CompletionItemKind::ENUM,
                    LspTypeDef::Interface { .. } => CompletionItemKind::INTERFACE,
                    LspTypeDef::Exception { .. } => CompletionItemKind::STRUCT,
                    LspTypeDef::TypeAlias { .. } => CompletionItemKind::TYPE_PARAMETER,
                };
                items.push(CompletionItem {
                    label: typedef.name().to_string(),
                    kind: Some(kind),
                    ..Default::default()
                });
            }
        }

        CompletionResponse::Array(items)
    }

    fn bare_completions(&self, partial: &str) -> CompletionResponse {
        let mut items = Vec::new();

        for kw in KEYWORDS {
            if partial.is_empty() || kw.starts_with(partial) {
                items.push(CompletionItem {
                    label: kw.to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    ..Default::default()
                });
            }
        }

        for (name, sig) in BUILTINS {
            if partial.is_empty() || name.starts_with(partial) {
                items.push(CompletionItem {
                    label: name.to_string(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    detail: Some(sig.to_string()),
                    ..Default::default()
                });
            }
        }

        if let Some(symbols) = &self.symbols {
            self.add_imported_completions(symbols, partial, &mut items);
            self.add_local_completions(symbols, partial, &mut items);
        }

        CompletionResponse::Array(items)
    }

    fn add_imported_completions(
        &self,
        symbols: &LspSymbols,
        partial: &str,
        items: &mut Vec<CompletionItem>,
    ) {
        for use_stmt in parse_use_statements(&self.source) {
            let module = match navigate_to_module(&symbols.root, &use_stmt.path) {
                Some(m) => m,
                None => continue,
            };

            match &use_stmt.items {
                ImportedItems::All => {
                    for sig in &module.functions {
                        if partial.is_empty() || sig.name.starts_with(partial) {
                            items.push(CompletionItem {
                                label: sig.name.clone(),
                                kind: Some(CompletionItemKind::FUNCTION),
                                detail: Some(sig.detail.clone()),
                                ..Default::default()
                            });
                        }
                    }
                }
                ImportedItems::Named(names) => {
                    for import_name in names {
                        if partial.is_empty() || import_name.starts_with(partial) {
                            if let Some(sig) = module.functions.iter().find(|f| f.name == *import_name) {
                                items.push(CompletionItem {
                                    label: import_name.clone(),
                                    kind: Some(CompletionItemKind::FUNCTION),
                                    detail: Some(sig.detail.clone()),
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    fn add_local_completions(
        &self,
        symbols: &LspSymbols,
        partial: &str,
        items: &mut Vec<CompletionItem>,
    ) {
        for sig in &symbols.functions {
            if !sig.is_std && (partial.is_empty() || sig.name.starts_with(partial)) {
                let already = items.iter().any(|i| i.label == sig.name);
                if !already {
                    items.push(CompletionItem {
                        label: sig.name.clone(),
                        kind: Some(CompletionItemKind::FUNCTION),
                        detail: Some(sig.detail.clone()),
                        ..Default::default()
                    });
                }
            }
        }

        for typedef in &symbols.types {
            let name = typedef.name();
            if partial.is_empty() || name.starts_with(partial) {
                let already = items.iter().any(|i| i.label == name);
                if !already {
                    let kind = match typedef {
                        LspTypeDef::Struct { .. } => CompletionItemKind::STRUCT,
                        LspTypeDef::Enum { .. } => CompletionItemKind::ENUM,
                        LspTypeDef::Interface { .. } => CompletionItemKind::INTERFACE,
                        LspTypeDef::Exception { .. } => CompletionItemKind::STRUCT,
                        LspTypeDef::TypeAlias { .. } => CompletionItemKind::TYPE_PARAMETER,
                    };
                    items.push(CompletionItem {
                        label: name.to_string(),
                        kind: Some(kind),
                        ..Default::default()
                    });
                }
            }
        }
    }
}

fn detect_context(prefix: &str) -> CompletionContext {
    let trimmed = prefix.trim_start();

    if trimmed.starts_with("use ") {
        let raw_path = &trimmed[4..];
        let before_brace = raw_path.split('{').next().unwrap_or(raw_path);
        let clean = before_brace.trim_end_matches(':').trim_end_matches(';');

        if raw_path.contains('{') {
            let segments: Vec<String> = clean.split("::")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let brace_pos = raw_path.find('{').unwrap();
            let inside_braces = &raw_path[brace_pos + 1..];
            let partial = inside_braces.rsplit(',')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            return CompletionContext::UseBraceItems { segments, partial };
        }

        if raw_path.contains("::") || before_brace.ends_with(':') {
            let segments: Vec<String> = clean.split("::")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            return CompletionContext::UseModulePath(segments);
        }
    }

    if let Some(colon_pos) = prefix.rfind("::") {
        let before = &prefix[..colon_pos];
        let path_start = before.rfind(|c: char| !c.is_alphanumeric() && c != '_' && c != ':')
            .map(|p| p + 1)
            .unwrap_or(0);
        let path_str = &before[path_start..];
        let segments: Vec<String> = path_str.split("::").map(|s| s.trim().to_string()).collect();
        return CompletionContext::ModulePath(segments);
    }

    if let Some(dot_pos) = prefix.rfind('.') {
        let before = &prefix[..dot_pos];
        let var_start = before.rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|p| p + 1)
            .unwrap_or(0);
        let var_name = &before[var_start..];
        if !var_name.is_empty() {
            return CompletionContext::DotAccess(var_name.to_string());
        }
    }

    if is_type_position(prefix) {
        return CompletionContext::TypePosition;
    }

    let ident_start = prefix.rfind(|c: char| !c.is_alphanumeric() && c != '_')
        .map(|p| p + 1)
        .unwrap_or(0);
    let partial = &prefix[ident_start..];
    CompletionContext::Bare(partial.to_string())
}

fn is_type_position(prefix: &str) -> bool {
    let trimmed = prefix.trim_end();
    if trimmed.ends_with(':') {
        let before = &trimmed[..trimmed.len() - 1].trim_end();
        if !before.ends_with(':') {
            return true;
        }
    }
    if trimmed.ends_with("->") {
        return true;
    }
    false
}

pub(crate) struct ParsedUse {
    path: Vec<String>,
    items: ImportedItems,
}

enum ImportedItems {
    All,
    Named(Vec<String>),
}

pub(crate) fn parse_use_statements(source: &str) -> Vec<ParsedUse> {
    let mut result = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("use ") {
            continue;
        }
        let rest = trimmed[4..].trim_end_matches(';').trim();

        if let Some(brace_start) = rest.find('{') {
            let path_part = rest[..brace_start].trim_end_matches("::").trim();
            let path: Vec<String> = path_part.split("::").map(|s| s.trim().to_string()).collect();

            let brace_end = rest.rfind('}').unwrap_or(rest.len());
            let items_str = &rest[brace_start + 1..brace_end];
            let names: Vec<String> = items_str.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            result.push(ParsedUse { path, items: ImportedItems::Named(names) });
        } else if rest.ends_with("::*") {
            let path_part = &rest[..rest.len() - 3];
            let path: Vec<String> = path_part.split("::").map(|s| s.trim().to_string()).collect();
            result.push(ParsedUse { path, items: ImportedItems::All });
        } else if rest.contains("::") {
            let parts: Vec<&str> = rest.rsplitn(2, "::").collect();
            if parts.len() == 2 {
                let path: Vec<String> = parts[1].split("::").map(|s| s.trim().to_string()).collect();
                let name = parts[0].trim().to_string();
                result.push(ParsedUse { path, items: ImportedItems::Named(vec![name]) });
            }
        }
    }
    result
}

pub fn navigate_to_module<'a>(
    root: &'a LspModule,
    segments: &[String],
) -> Option<&'a LspModule> {
    let mut current = root;
    for seg in segments {
        current = current.submodules.get(seg)?;
    }
    Some(current)
}
