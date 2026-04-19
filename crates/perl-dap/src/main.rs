//! DAP adapter entry point
//!
//! This binary provides the Debug Adapter Protocol server for Perl debugging.
//! It follows the TDD approach with comprehensive test scaffolding for 19 acceptance criteria.

use clap::Parser;
use perl_dap::{DapConfig, DapMode, DapServer};
use perl_lsp_launcher::{init_logging, log_server_startup};

const DEFAULT_DAP_PORT: u16 = 13_603;

fn resolve_socket_port(args: &perl_lsp_launcher::TransportArgs) -> Option<u16> {
    if args.socket || args.port.is_some() {
        Some(args.port.unwrap_or(DEFAULT_DAP_PORT))
    } else {
        None
    }
}

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

    init_logging(&args.log_level);
    log_server_startup(
        "perl-dap",
        env!("CARGO_PKG_VERSION"),
        args.transport.mode(),
        None,
        None,
    );

    let config = DapConfig {
        log_level: args.log_level,
        mode: if args.bridge {
            DapMode::Bridge
        } else {
            DapMode::Native
        },
        workspace_root: None,
    };

    let mut server = DapServer::new(config)?;

    if let Some(port) = resolve_socket_port(&args.transport) {
        tracing::info!("Starting DAP server on port {}", port);
        server.run_socket(port)?;
        return Ok(());
    }

    tracing::info!("Starting DAP server on stdio");
    server.run()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_DAP_PORT, resolve_socket_port};

    #[test]
    fn socket_mode_uses_dap_default_port() {
        let args = perl_lsp_launcher::TransportArgs {
            stdio: false,
            socket: true,
            port: None,
        };

        assert_eq!(resolve_socket_port(&args), Some(DEFAULT_DAP_PORT));
    }

    #[test]
    fn explicit_socket_port_is_preserved() {
        let args = perl_lsp_launcher::TransportArgs {
            stdio: false,
            socket: true,
            port: Some(9_999),
        };

        assert_eq!(resolve_socket_port(&args), Some(9_999));
    }

    #[test]
    fn stdio_mode_does_not_resolve_a_socket_port() {
        let args = perl_lsp_launcher::TransportArgs {
            stdio: true,
            socket: false,
            port: None,
        };

        assert_eq!(resolve_socket_port(&args), None);
    }
}
