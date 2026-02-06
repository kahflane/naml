///
/// Symbols Module
///
/// Provides document symbols (file outline), go-to-definition, and
/// find-references for the naml LSP. Uses the stored LspSymbols
/// snapshot and source spans.
///

use tower_lsp::lsp_types::*;

use namlc::source::Span;

use crate::analysis::{AnalysisContext, DocumentAnalysis};
use crate::hover::extract_word_at;
use crate::lsp_symbols::LspTypeDef;

impl DocumentAnalysis {
    pub fn document_symbols(&self) -> Option<DocumentSymbolResponse> {
        let symbols = self.symbols.as_ref()?;
        let actx = self.ctx();
        let mut doc_symbols = Vec::new();

        for sig in &symbols.functions {
            if sig.is_std { continue; }
            let range = actx.span_to_range(sig.span);

            #[allow(deprecated)]
            doc_symbols.push(DocumentSymbol {
                name: sig.name.clone(),
                detail: Some(sig.detail.clone()),
                kind: SymbolKind::FUNCTION,
                tags: None,
                deprecated: None,
                range,
                selection_range: range,
                children: None,
            });
        }

        for typedef in &symbols.types {
            match typedef {
                LspTypeDef::Struct { name, fields, span } => {
                    let range = actx.span_to_range(*span);
                    let mut children: Vec<DocumentSymbol> = fields.iter().map(|field| {
                        #[allow(deprecated)]
                        DocumentSymbol {
                            name: field.name.clone(),
                            detail: Some(field.type_str.clone()),
                            kind: SymbolKind::FIELD,
                            tags: None,
                            deprecated: None,
                            range,
                            selection_range: range,
                            children: None,
                        }
                    }).collect();

                    if let Some(methods) = symbols.methods.get(name.as_str()) {
                        for method in methods {
                            let mrange = actx.span_to_range(method.span);
                            #[allow(deprecated)]
                            children.push(DocumentSymbol {
                                name: method.name.clone(),
                                detail: Some(method.detail.clone()),
                                kind: SymbolKind::METHOD,
                                tags: None,
                                deprecated: None,
                                range: mrange,
                                selection_range: mrange,
                                children: None,
                            });
                        }
                    }

                    #[allow(deprecated)]
                    doc_symbols.push(DocumentSymbol {
                        name: name.clone(),
                        detail: None,
                        kind: SymbolKind::STRUCT,
                        tags: None,
                        deprecated: None,
                        range,
                        selection_range: range,
                        children: if children.is_empty() { None } else { Some(children) },
                    });
                }
                LspTypeDef::Enum { name, variants, span } => {
                    let range = actx.span_to_range(*span);
                    let children: Vec<DocumentSymbol> = variants.iter().map(|variant| {
                        #[allow(deprecated)]
                        DocumentSymbol {
                            name: variant.name.clone(),
                            detail: None,
                            kind: SymbolKind::ENUM_MEMBER,
                            tags: None,
                            deprecated: None,
                            range,
                            selection_range: range,
                            children: None,
                        }
                    }).collect();

                    #[allow(deprecated)]
                    doc_symbols.push(DocumentSymbol {
                        name: name.clone(),
                        detail: None,
                        kind: SymbolKind::ENUM,
                        tags: None,
                        deprecated: None,
                        range,
                        selection_range: range,
                        children: if children.is_empty() { None } else { Some(children) },
                    });
                }
                LspTypeDef::Interface { name, methods, span } => {
                    let range = actx.span_to_range(*span);
                    let children: Vec<DocumentSymbol> = methods.iter().map(|m| {
                        #[allow(deprecated)]
                        DocumentSymbol {
                            name: m.name.clone(),
                            detail: Some(m.detail.clone()),
                            kind: SymbolKind::METHOD,
                            tags: None,
                            deprecated: None,
                            range,
                            selection_range: range,
                            children: None,
                        }
                    }).collect();

                    #[allow(deprecated)]
                    doc_symbols.push(DocumentSymbol {
                        name: name.clone(),
                        detail: None,
                        kind: SymbolKind::INTERFACE,
                        tags: None,
                        deprecated: None,
                        range,
                        selection_range: range,
                        children: if children.is_empty() { None } else { Some(children) },
                    });
                }
                LspTypeDef::Exception { name, span, .. } => {
                    let range = actx.span_to_range(*span);
                    #[allow(deprecated)]
                    doc_symbols.push(DocumentSymbol {
                        name: name.clone(),
                        detail: None,
                        kind: SymbolKind::STRUCT,
                        tags: None,
                        deprecated: None,
                        range,
                        selection_range: range,
                        children: None,
                    });
                }
                LspTypeDef::TypeAlias { name, aliased_type, span } => {
                    let range = actx.span_to_range(*span);
                    #[allow(deprecated)]
                    doc_symbols.push(DocumentSymbol {
                        name: name.clone(),
                        detail: Some(format!("= {}", aliased_type)),
                        kind: SymbolKind::TYPE_PARAMETER,
                        tags: None,
                        deprecated: None,
                        range,
                        selection_range: range,
                        children: None,
                    });
                }
            }
        }

        if doc_symbols.is_empty() {
            None
        } else {
            Some(DocumentSymbolResponse::Nested(doc_symbols))
        }
    }

    pub fn definition_at(&self, uri: &Url, position: Position) -> Option<GotoDefinitionResponse> {
        let symbols = self.symbols.as_ref()?;
        let actx = self.ctx();
        let offset = actx.position_to_offset(position);
        let word = extract_word_at(&self.source, offset);
        if word.is_empty() {
            return None;
        }

        if let Some(sig) = symbols.functions.iter().find(|f| f.name == word) {
            if !sig.is_std {
                let range = actx.span_to_range(sig.span);
                return Some(GotoDefinitionResponse::Scalar(Location {
                    uri: uri.clone(),
                    range,
                }));
            }

            for imported in &self.imported_modules {
                let imp_uri = Url::from_file_path(&imported.file_path).ok()?;
                let imp_ctx = AnalysisContext::new(&imported.source_text);
                let (tokens, mut imp_interner) = namlc::tokenize(&imported.source_text);
                let arena = namlc::AstArena::new();
                let parse_result = namlc::parse(&tokens, &imported.source_text, &arena);

                if parse_result.errors.is_empty() {
                    let type_result = namlc::check_with_types(
                        &parse_result.ast,
                        &mut imp_interner,
                        imported.file_path.parent().map(|p| p.to_path_buf()),
                    );

                    if let Some(imp_spur) = imp_interner.get(&word) {
                        if let Some(imp_sig) = type_result.symbols.get_function(imp_spur) {
                            let range = imp_ctx.span_to_range(imp_sig.span);
                            return Some(GotoDefinitionResponse::Scalar(Location {
                                uri: imp_uri,
                                range,
                            }));
                        }
                    }
                }
            }
        }

        if let Some(typedef) = symbols.types.iter().find(|t| t.name() == word) {
            let range = actx.span_to_range(typedef.span());
            return Some(GotoDefinitionResponse::Scalar(Location {
                uri: uri.clone(),
                range,
            }));
        }

        for (_type_name, methods) in &symbols.methods {
            for method in methods {
                if method.name == word {
                    let range = actx.span_to_range(method.span);
                    return Some(GotoDefinitionResponse::Scalar(Location {
                        uri: uri.clone(),
                        range,
                    }));
                }
            }
        }

        None
    }

    pub fn references_at(&self, uri: &Url, position: Position) -> Option<Vec<Location>> {
        let actx = self.ctx();
        let offset = actx.position_to_offset(position);
        let word = extract_word_at(&self.source, offset);
        if word.is_empty() {
            return None;
        }

        let mut locations = Vec::new();
        let source_bytes = self.source.as_bytes();
        let word_bytes = word.as_bytes();
        let mut pos = 0;

        while pos + word_bytes.len() <= source_bytes.len() {
            if let Some(found) = self.source[pos..].find(&word) {
                let abs_pos = pos + found;
                let end = abs_pos + word.len();

                let before_ok = abs_pos == 0
                    || (!source_bytes[abs_pos - 1].is_ascii_alphanumeric()
                        && source_bytes[abs_pos - 1] != b'_');
                let after_ok = end >= source_bytes.len()
                    || (!source_bytes[end].is_ascii_alphanumeric()
                        && source_bytes[end] != b'_');

                if before_ok && after_ok {
                    let span = Span::new(abs_pos as u32, end as u32, 0);
                    let range = actx.span_to_range(span);
                    locations.push(Location {
                        uri: uri.clone(),
                        range,
                    });
                }
                pos = abs_pos + 1;
            } else {
                break;
            }
        }

        if locations.is_empty() {
            None
        } else {
            Some(locations)
        }
    }
}
