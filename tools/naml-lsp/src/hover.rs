///
/// Hover Module
///
/// Provides hover information (type tooltips) for the naml LSP.
/// Resolves the word at cursor position against the LspSymbols
/// snapshot to show function signatures, type definitions, and
/// field/method information.
///

use tower_lsp::lsp_types::*;

use crate::analysis::DocumentAnalysis;
use crate::lsp_symbols::LspTypeDef;

impl DocumentAnalysis {
    pub fn hover_at(&self, position: Position) -> Option<Hover> {
        let symbols = self.symbols.as_ref()?;

        let actx = self.ctx();
        let offset = actx.position_to_offset(position);

        let word = extract_word_at(&self.source, offset);
        if word.is_empty() {
            return None;
        }

        if let Some(sig) = symbols.functions.iter().find(|f| f.name == word) {
            return Some(make_hover(&sig.detail));
        }

        if let Some(typedef) = symbols.types.iter().find(|t| t.name() == word) {
            let detail = format_type_hover(typedef);
            return Some(make_hover(&detail));
        }

        for typedef in &symbols.types {
            if let LspTypeDef::Struct { name, fields, .. } = typedef {
                for field in fields {
                    if field.name == word {
                        let detail = format!("(field) {}.{}: {}", name, field.name, field.type_str);
                        return Some(make_hover(&detail));
                    }
                }
            }
        }

        for (_type_name, methods) in &symbols.methods {
            for method in methods {
                if method.name == word {
                    return Some(make_hover(&method.detail));
                }
            }
        }

        let builtin_info = match word.as_str() {
            "print" => Some("fn print(args: ...any)"),
            "println" => Some("fn println(args: ...any)"),
            "fmt" => Some("fn fmt(format: string, args: ...any) -> string"),
            "warn" => Some("fn warn(args: ...any)"),
            "error" => Some("fn error(args: ...any)"),
            "panic" => Some("fn panic(args: ...any)"),
            _ => None,
        };

        if let Some(info) = builtin_info {
            return Some(make_hover(info));
        }

        None
    }
}

pub fn extract_word_at(source: &str, offset: u32) -> String {
    let bytes = source.as_bytes();
    let offset = offset as usize;
    if offset >= bytes.len() {
        return String::new();
    }

    let mut start = offset;
    while start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
        start -= 1;
    }

    let mut end = offset;
    while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        end += 1;
    }

    if start == end {
        return String::new();
    }

    source[start..end].to_string()
}

fn format_type_hover(typedef: &LspTypeDef) -> String {
    match typedef {
        LspTypeDef::Struct { name, fields, .. } => {
            let mut s = format!("struct {}", name);
            if !fields.is_empty() {
                s.push_str(" {\n");
                for field in fields {
                    let vis = if field.is_public { "pub " } else { "" };
                    s.push_str(&format!("    {}{}: {},\n", vis, field.name, field.type_str));
                }
                s.push('}');
            }
            s
        }
        LspTypeDef::Enum { name, variants, .. } => {
            let mut s = format!("enum {}", name);
            if !variants.is_empty() {
                s.push_str(" {\n");
                for variant in variants {
                    if let Some(fields) = &variant.fields {
                        s.push_str(&format!("    {}({}),\n", variant.name, fields.join(", ")));
                    } else {
                        s.push_str(&format!("    {},\n", variant.name));
                    }
                }
                s.push('}');
            }
            s
        }
        LspTypeDef::Interface { name, methods, .. } => {
            let mut s = format!("interface {}", name);
            if !methods.is_empty() {
                s.push_str(" {\n");
                for m in methods {
                    s.push_str(&format!("    {};\n", m.detail));
                }
                s.push('}');
            }
            s
        }
        LspTypeDef::Exception { name, fields, .. } => {
            let mut s = format!("exception {}", name);
            if !fields.is_empty() {
                s.push_str(" {\n");
                for field in fields {
                    s.push_str(&format!("    {}: {},\n", field.name, field.type_str));
                }
                s.push('}');
            }
            s
        }
        LspTypeDef::TypeAlias { name, aliased_type, .. } => {
            format!("type {} = {}", name, aliased_type)
        }
    }
}

fn make_hover(content: &str) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("```naml\n{}\n```", content),
        }),
        range: None,
    }
}
