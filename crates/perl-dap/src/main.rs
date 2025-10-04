//! DAP adapter entry point
//!
//! This binary provides the Debug Adapter Protocol server for Perl debugging.
//! It follows the TDD approach with comprehensive test scaffolding for 19 acceptance criteria.

use perl_dap::{DapConfig, DapServer};

fn main() -> anyhow::Result<()> {
    // TODO: Initialize logging (AC5)
    // TODO: Parse command-line arguments (AC5)
    // TODO: Create DAP server (AC5)
    // TODO: Run stdio transport (AC5)

    let config = DapConfig { log_level: "info".to_string() };

    let _server = DapServer::new(config)?;

    println!("perl-dap: Debug Adapter Protocol server (placeholder)");
    println!("Run tests with: cargo test -p perl-dap");

    Ok(())
}
