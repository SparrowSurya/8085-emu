pub mod code_actions;
pub mod completion;
pub mod definition;
pub mod diagnostics;
pub mod document;
pub mod hints;
pub mod hover;
pub mod rename;
pub mod server;

use server::E8085LanguageServer;
use tower_lsp::{LspService, Server};

/// Starts the e8085 Language Server listening on standard input and output.
pub async fn start_lsp_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| E8085LanguageServer::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}
