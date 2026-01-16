//! DAP adapter entry point
//!
//! This binary provides the Debug Adapter Protocol server for Perl debugging.
//! It follows the TDD approach with comprehensive test scaffolding for 19 acceptance criteria.

use perl_dap::{DapConfig, DapServer};
use std::io;
use tracing_subscriber::{EnvFilter, fmt};

fn init_logging(log_level: &str) {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(log_level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .with_writer(io::stderr)
        .init();
}

fn main() -> anyhow::Result<()> {
    // TODO: Parse command-line arguments (AC5)
    // TODO: Create DAP server (AC5)
    // TODO: Run stdio transport (AC5)

    let config = DapConfig { log_level: "info".to_string() };

    init_logging(&config.log_level);
    tracing::info!("perl-dap: Debug Adapter Protocol server initialized");

    let _server = DapServer::new(config)?;

    println!("perl-dap: Debug Adapter Protocol server (placeholder)");
    println!("Run tests with: cargo test -p perl-dap");

    Ok(())
}
