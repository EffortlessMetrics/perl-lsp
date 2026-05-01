use crate::{BridgeAdapter, DebugAdapter};
use crate::server::{config::DapConfig, mode::DapMode};

/// DAP server
///
/// Supports two operating modes:
/// - **Native** (default): Uses the built-in [`DebugAdapter`] with `perl -d`
/// - **Bridge**: Proxies DAP messages to Perl::LanguageServer via [`BridgeAdapter`]
pub struct DapServer {
    /// Server configuration
    pub config: DapConfig,
    /// The underlying debug adapter (used in Native mode)
    adapter: DebugAdapter,
}

impl DapServer {
    /// Create a new DAP server instance
    ///
    /// # Arguments
    ///
    /// * `config` - Server configuration including operating mode
    ///
    /// # Errors
    ///
    /// Currently always succeeds. Phase 2 will add validation and initialization errors.
    pub fn new(config: DapConfig) -> anyhow::Result<Self> {
        Ok(Self { config, adapter: DebugAdapter::new() })
    }

    /// Run the DAP server
    ///
    /// Dispatches to the appropriate transport based on the configured [`DapMode`]:
    /// - [`DapMode::Native`]: Starts the stdio transport loop via `DebugAdapter::run`
    /// - [`DapMode::Bridge`]: Spawns Perl::LanguageServer and proxies DAP messages
    ///   via [`BridgeAdapter`] using a tokio async runtime
    pub fn run(&mut self) -> anyhow::Result<()> {
        match self.config.mode {
            DapMode::Native => self.adapter.run().map_err(Into::into),
            DapMode::Bridge => {
                tracing::info!("Starting DAP server in bridge mode");
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(async {
                    let mut bridge = BridgeAdapter::new();
                    bridge.spawn_pls_dap().await?;
                    bridge.proxy_messages().await?;
                    bridge.shutdown().await?;
                    Ok(())
                })
            }
        }
    }

    /// Run the DAP server over TCP socket transport.
    ///
    /// This binds to `127.0.0.1:<port>` and serves one DAP client session.
    ///
    /// # Errors
    ///
    /// Returns an error if bridge mode is selected, since socket transport
    /// is only supported for native mode.
    pub fn run_socket(&mut self, port: u16) -> anyhow::Result<()> {
        if self.config.mode == DapMode::Bridge {
            anyhow::bail!("Socket transport is not supported in bridge mode");
        }
        self.adapter.run_socket(port).map_err(Into::into)
    }
}
