///
/// Server Capabilities
///
/// Defines the LSP capabilities that the naml language server supports.
///

use tower_lsp::lsp_types::*;

pub fn server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::FULL
        )),

        hover_provider: Some(HoverProviderCapability::Simple(true)),

        definition_provider: Some(OneOf::Left(true)),

        references_provider: Some(OneOf::Left(true)),

        document_symbol_provider: Some(OneOf::Left(true)),

        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec![
                ".".to_string(),
                ":".to_string(),
            ]),
            ..Default::default()
        }),

        code_action_provider: Some(CodeActionProviderCapability::Options(
            CodeActionOptions {
                code_action_kinds: Some(vec![CodeActionKind::QUICKFIX]),
                ..Default::default()
            }
        )),

        ..Default::default()
    }
}
