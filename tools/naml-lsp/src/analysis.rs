///
/// Document Analysis Module
///
/// This module integrates with namlc to provide semantic analysis
/// of naml source files. It reuses the lexer, parser, and type
/// checker from the compiler library.
///
/// The SymbolTable from type checking is snapshotted into a thread-safe
/// LspSymbols struct to power completions, hover, go-to-def, and
/// document symbols across async boundaries.
///

use std::path::PathBuf;
use std::sync::Arc;

use tower_lsp::lsp_types::*;

use namlc::source::Span;
use namlc::typechecker::TypeError;
use namlc::{parse, tokenize, check_with_types, AstArena, ImportedModule};

use crate::lsp_symbols::{LspSymbols, LspModule, snapshot_symbols};

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
    pub symbols: Option<LspSymbols>,
    pub imported_modules: Vec<ImportedModule>,
}

pub struct AnalysisContext {
    pub line_starts: Vec<u32>,
}

impl AnalysisContext {
    pub fn new(source: &str) -> Self {
        Self {
            line_starts: Self::compute_line_starts(source),
        }
    }

    pub fn compute_line_starts(source: &str) -> Vec<u32> {
        let mut starts = vec![0];
        for (i, c) in source.char_indices() {
            if c == '\n' {
                starts.push((i + 1) as u32);
            }
        }
        starts
    }

    pub fn offset_to_position(&self, offset: u32) -> Position {
        let line_idx = self.line_starts
            .binary_search(&offset)
            .unwrap_or_else(|i| i.saturating_sub(1));
        let line = line_idx as u32;
        let character = offset.saturating_sub(self.line_starts[line_idx]);
        Position { line, character }
    }

    pub fn span_to_range(&self, span: Span) -> Range {
        Range {
            start: self.offset_to_position(span.start),
            end: self.offset_to_position(span.end),
        }
    }

    pub fn position_to_offset(&self, pos: Position) -> u32 {
        let line = pos.line as usize;
        if line < self.line_starts.len() {
            self.line_starts[line] + pos.character
        } else {
            *self.line_starts.last().unwrap_or(&0) + pos.character
        }
    }
}

impl DocumentAnalysis {
    pub fn analyze(content: &str, source_dir: Option<PathBuf>) -> Self {
        let ctx = AnalysisContext::new(content);
        let mut diagnostics = Vec::new();
        let mut undefined_symbols = Vec::new();
        #[allow(unused_assignments)]
        let mut symbols = None;
        let mut imported_modules = Vec::new();

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
            let pkg_manager = source_dir
                .as_ref()
                .and_then(|dir| naml_pkg::find_project_root(dir))
                .and_then(|root| {
                    let manifest_path = root.join("naml.toml");
                    match naml_pkg::PackageManager::from_manifest_path(&manifest_path) {
                        Ok(mut pm) => {
                            if pm.has_dependencies() {
                                if let Err(_e) = pm.ensure_all_downloaded() {}
                            }
                            Some(pm)
                        }
                        Err(_) => None,
                    }
                });

            let type_result = check_with_types(&parse_result.ast, &mut interner, source_dir, pkg_manager.as_ref());

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

            imported_modules = type_result.imported_modules;
            symbols = Some(snapshot_symbols(&type_result.symbols, &interner));
        } else {
            let empty_ast = namlc::ast::SourceFile::empty();
            let type_result = check_with_types(&empty_ast, &mut interner, None, None);
            symbols = Some(snapshot_symbols(&type_result.symbols, &interner));
        }

        Self {
            diagnostics,
            undefined_symbols,
            source: content.into(),
            line_starts: ctx.line_starts,
            symbols,
            imported_modules,
        }
    }

    pub fn get_import_suggestions(&self, position: Position) -> Vec<(String, String)> {
        let mut suggestions = Vec::new();

        let symbols = match &self.symbols {
            Some(s) => s,
            None => return suggestions,
        };

        for sym in &self.undefined_symbols {
            if Self::position_in_range(position, sym.range) {
                collect_module_functions(&symbols.root, &[], &sym.name, &mut suggestions);
            }
        }
        suggestions
    }

    pub fn position_in_range(pos: Position, range: Range) -> bool {
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

    pub fn ctx(&self) -> AnalysisContext {
        AnalysisContext {
            line_starts: self.line_starts.clone(),
        }
    }
}

fn collect_module_functions(
    module: &LspModule,
    path: &[String],
    target_name: &str,
    suggestions: &mut Vec<(String, String)>,
) {
    for sig in &module.functions {
        if sig.name == target_name {
            let full_path = if path.is_empty() {
                sig.name.clone()
            } else {
                path.join("::")
            };
            suggestions.push((target_name.to_string(), full_path));
        }
    }

    for (sub_name, submod) in &module.submodules {
        let mut new_path = path.to_vec();
        new_path.push(sub_name.clone());
        collect_module_functions(submod, &new_path, target_name, suggestions);
    }
}
