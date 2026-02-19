//! DAP adapter entry point
//!
//! This binary provides the Debug Adapter Protocol server for Perl debugging.
//! It follows the TDD approach with comprehensive test scaffolding for 19 acceptance criteria.

use clap::Parser;
use perl_dap::{DapConfig, DapServer};
use std::io;
use tracing_subscriber::{EnvFilter, fmt};

/// Perl Debug Adapter Protocol server
#[derive(Parser, Debug)]
#[command(name = "perl-dap", version, about, long_about = None)]
struct Args {
    /// Use stdio for communication (default)
    #[arg(long, default_value_t = true)]
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

    fmt().with_env_filter(filter).with_writer(io::stderr).init();
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    init_logging(&args.log_level);
    tracing::info!("perl-dap: Debug Adapter Protocol server starting");

    let config = DapConfig { log_level: args.log_level };

    let mut server = DapServer::new(config)?;

    if args.socket {
        tracing::info!("Starting DAP server on port {}", args.port);
        server.run_socket(args.port)?;
        return Ok(());
    }

    tracing::info!("Starting DAP server on stdio");
    server.run()?;

    Ok(())
}
