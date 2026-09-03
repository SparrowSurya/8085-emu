//! Standalone binary entrypoint for the e8085 Language Server.

#[tokio::main]
async fn main() {
    emu8085::lsp::start_lsp_server().await;
}
