///
/// naml Language Server - Main Entry Point
///
/// This binary provides LSP support for the naml programming language.
/// It uses tower-lsp for the protocol implementation and reuses
/// namlc's lexer, parser, and type checker for analysis.
///

mod backend;
mod analysis;
mod capabilities;
mod completions;
mod hover;
mod lsp_symbols;
mod symbols;

use tower_lsp::{LspService, Server};
use backend::NamlBackend;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(NamlBackend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
