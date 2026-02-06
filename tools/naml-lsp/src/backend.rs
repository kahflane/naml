///
/// LSP Backend Implementation
///
/// This module implements the LanguageServer trait from tower-lsp,
/// handling all LSP requests and notifications. It maintains document
/// state and coordinates with the analysis module.
///

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::analysis::DocumentAnalysis;

pub struct NamlBackend {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, DocumentState>>>,
}

pub struct DocumentState {
    pub content: String,
    pub version: i32,
    pub analysis: Option<DocumentAnalysis>,
}

impl NamlBackend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn analyze_document(&self, uri: &Url) {
        let content = {
            let documents = self.documents.read().await;
            documents.get(uri).map(|doc| doc.content.clone())
        };

        if let Some(content) = content {
            let source_dir = uri.to_file_path().ok().and_then(|p| p.parent().map(|p| p.to_path_buf()));
            let analysis = DocumentAnalysis::analyze(&content, source_dir);
            let diagnostics = analysis.diagnostics_to_lsp();

            {
                let mut documents = self.documents.write().await;
                if let Some(doc) = documents.get_mut(uri) {
                    doc.analysis = Some(analysis);
                }
            }

            self.client
                .publish_diagnostics(uri.clone(), diagnostics, None)
                .await;
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for NamlBackend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: crate::capabilities::server_capabilities(),
            server_info: Some(ServerInfo {
                name: "naml-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "naml language server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let content = params.text_document.text;
        let version = params.text_document.version;

        {
            let mut docs = self.documents.write().await;
            docs.insert(uri.clone(), DocumentState {
                content,
                version,
                analysis: None,
            });
        }

        self.analyze_document(&uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;

        {
            let mut docs = self.documents.write().await;
            if let Some(doc) = docs.get_mut(&uri) {
                for change in params.content_changes {
                    doc.content = change.text;
                }
                doc.version = version;
            }
        }

        self.analyze_document(&uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        let mut docs = self.documents.write().await;
        docs.remove(&uri);

        self.client.publish_diagnostics(uri, vec![], None).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let docs = self.documents.read().await;
        if let Some(doc) = docs.get(&uri) {
            if let Some(ref analysis) = doc.analysis {
                return Ok(analysis.hover_at(position));
            }
        }
        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let docs = self.documents.read().await;
        if let Some(doc) = docs.get(&uri) {
            if let Some(ref analysis) = doc.analysis {
                return Ok(analysis.definition_at(&uri, position));
            }
        }
        Ok(None)
    }

    async fn references(
        &self,
        params: ReferenceParams,
    ) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let docs = self.documents.read().await;
        if let Some(doc) = docs.get(&uri) {
            if let Some(ref analysis) = doc.analysis {
                return Ok(analysis.references_at(&uri, position));
            }
        }
        Ok(None)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;

        let docs = self.documents.read().await;
        if let Some(doc) = docs.get(&uri) {
            if let Some(ref analysis) = doc.analysis {
                return Ok(analysis.document_symbols());
            }
        }
        Ok(None)
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let docs = self.documents.read().await;
        if let Some(doc) = docs.get(&uri) {
            if let Some(ref analysis) = doc.analysis {
                return Ok(analysis.completions_at(&doc.content, position));
            }
        }
        Ok(None)
    }

    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let range = params.range;

        let docs = self.documents.read().await;
        if let Some(doc) = docs.get(&uri) {
            if let Some(ref analysis) = doc.analysis {
                let suggestions = analysis.get_import_suggestions(range.start);
                if !suggestions.is_empty() {
                    let insert_position = Self::find_import_insert_position(&doc.content);
                    let mut actions = Vec::new();

                    for (i, (func_name, module_path)) in suggestions.iter().enumerate() {
                        let use_statement = format!("use {}::{};\n", module_path, func_name);

                        let edit = TextEdit {
                            range: Range {
                                start: insert_position,
                                end: insert_position,
                            },
                            new_text: use_statement,
                        };

                        let mut changes = std::collections::HashMap::new();
                        changes.insert(uri.clone(), vec![edit]);

                        let workspace_edit = WorkspaceEdit {
                            changes: Some(changes),
                            ..Default::default()
                        };

                        let action = CodeAction {
                            title: format!("Import {} from {}", func_name, module_path),
                            kind: Some(CodeActionKind::QUICKFIX),
                            diagnostics: None,
                            edit: Some(workspace_edit),
                            command: None,
                            is_preferred: Some(i == 0),
                            disabled: None,
                            data: None,
                        };

                        actions.push(CodeActionOrCommand::CodeAction(action));
                    }

                    return Ok(Some(actions));
                }
            }
        }
        Ok(None)
    }
}

impl NamlBackend {
    fn find_import_insert_position(content: &str) -> Position {
        let mut last_use_line: Option<u32> = None;
        let mut line_num = 0u32;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("use ") {
                last_use_line = Some(line_num);
            } else if !trimmed.is_empty()
                && !trimmed.starts_with("//")
                && !trimmed.starts_with("mod ")
                && last_use_line.is_none()
            {
                return Position { line: 0, character: 0 };
            }
            line_num += 1;
        }

        if let Some(line) = last_use_line {
            Position { line: line + 1, character: 0 }
        } else {
            Position { line: 0, character: 0 }
        }
    }
}
