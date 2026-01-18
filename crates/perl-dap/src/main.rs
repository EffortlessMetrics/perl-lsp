//! DAP adapter entry point
//!
//! This binary provides the Debug Adapter Protocol server for Perl debugging.
//! It follows the TDD approach with comprehensive test scaffolding for 19 acceptance criteria.

use clap::Parser;
use perl_dap::{DapConfig, DapServer};
use tracing_subscriber::{EnvFilter, fmt};
use std::io;

#[derive(Parser, Debug)]
#[command(name = "perl-dap", author, version, about, long_about = None)]
struct Args {
    /// Use stdio for communication (default)
    #[arg(long)]
    stdio: bool,

    /// Use TCP socket for communication
    #[arg(long, conflicts_with = "stdio")]
    socket: bool,

    /// Port to listen on (for socket mode)
    #[arg(long, default_value_t = 13603)]
    port: u16,

    /// Logging level (error, warn, info, debug, trace)
    #[arg(long, default_value = "info")]
    log_level: String,
}

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
    // Parse command-line arguments (AC5)
    let args = Args::parse();

    // Initialize logging (AC5)
    init_logging(&args.log_level);
    tracing::info!("perl-dap: Debug Adapter Protocol server initialized");

    // Determine mode
    let use_socket = args.socket;
    // Default to stdio if not socket

    let config = DapConfig {
        log_level: args.log_level.clone(),
    };

    let _server = DapServer::new(config)?;
    // TODO: implement server.run() (AC5)

    if use_socket {
        tracing::info!("Starting DAP server on port {}", args.port);
        // Socket transport implementation is planned for Phase 2 (AC5)
        anyhow::bail!("Socket transport mode is not yet implemented. Please use stdio mode.");
    } else {
        tracing::info!("Starting DAP server on stdio");
        // TODO: Run stdio transport (AC5)
        tracing::info!("perl-dap: Debug Adapter Protocol server on stdio (placeholder)");
    }

    tracing::info!("Run tests with: cargo test -p perl-dap");

    Ok(())
}
