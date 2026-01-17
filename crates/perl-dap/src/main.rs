//! DAP adapter entry point
//!
//! This binary provides the Debug Adapter Protocol server for Perl debugging.
//! It follows the TDD approach with comprehensive test scaffolding for 19 acceptance criteria.

use clap::Parser;
use perl_dap::{DapConfig, DapServer};
use tracing_subscriber::EnvFilter;

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

fn main() -> anyhow::Result<()> {
    // Parse command-line arguments (AC5)
    let args = Args::parse();

    // Initialize logging (AC5)
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(&args.log_level)),
        )
        .with_writer(std::io::stderr)
        .init();

    // Determine mode
    let use_socket = args.socket;
    // Default to stdio if not socket

    let config = DapConfig {
        log_level: args.log_level.clone(),
    };

    let _server = DapServer::new(config)?;

    if use_socket {
        tracing::info!("Starting DAP server on port {}", args.port);
        // TODO: Implement socket transport (AC5)
        println!("perl-dap: Debug Adapter Protocol server listening on port {} (placeholder)", args.port);
    } else {
        tracing::info!("Starting DAP server on stdio");
        // TODO: Run stdio transport (AC5)
        println!("perl-dap: Debug Adapter Protocol server on stdio (placeholder)");
    }

    println!("Run tests with: cargo test -p perl-dap");

    Ok(())
}
