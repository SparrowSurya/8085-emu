//! Standalone entry point for the 8085 Debug Adapter Protocol (DAP) server.

use emu8085::dap::DapServer;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    DapServer::run_stdio().await
}
