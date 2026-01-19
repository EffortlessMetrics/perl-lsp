//! DAP adapter entry point
//!
//! This binary provides the Debug Adapter Protocol server for Perl debugging.
//! It follows the TDD approach with comprehensive test scaffolding for 19 acceptance criteria.

use perl_dap::{DapConfig, DapServer};

fn main() -> anyhow::Result<()> {
    // TODO: Parse command-line arguments (AC5)

    let config = DapConfig { log_level: "info".to_string() };

    let mut server = DapServer::new(config)?;
    server.run()?;

    Ok(())
}
