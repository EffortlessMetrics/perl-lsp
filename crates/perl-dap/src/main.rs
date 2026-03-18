//! DAP adapter entry point
//!
//! This binary provides the Debug Adapter Protocol server for Perl debugging.
//! It follows the TDD approach with comprehensive test scaffolding for 19 acceptance criteria.

use clap::Parser;
use perl_dap::{DapConfig, DapMode, DapServer};
use perl_lsp_launcher::{init_stderr_logging, log_server_startup};

/// Perl Debug Adapter Protocol server
#[derive(Parser, Debug)]
#[command(name = "perl-dap", version, about, long_about = None)]
struct Args {
    #[command(flatten)]
    transport: perl_lsp_launcher::TransportArgs,

    /// Use bridge mode (proxy to Perl::LanguageServer)
    #[arg(long)]
    bridge: bool,

    /// Logging level (error, warn, info, debug, trace)
    #[arg(long, default_value = "info")]
    log_level: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let _logging_initialized = init_stderr_logging(&args.log_level);
    log_server_startup("perl-dap", args.transport.mode(), None, Some(&args.log_level));

    let config = DapConfig {
        log_level: args.log_level,
        mode: if args.bridge { DapMode::Bridge } else { DapMode::Native },
        workspace_root: None,
    };

    let mut server = DapServer::new(config)?;

    if args.transport.socket || args.transport.port.is_some() {
        let port = args.transport.port.unwrap_or(perl_lsp_launcher::DEFAULT_LSP_PORT);
        tracing::info!("Starting DAP server on port {}", port);
        server.run_socket(port)?;
        return Ok(());
    }

    tracing::info!("Starting DAP server on stdio");
    server.run()?;

    Ok(())
}
