use crate::server::mode::DapMode;

/// DAP server configuration.
///
/// Controls the operating mode, logging, and workspace context for the DAP server.
pub struct DapConfig {
    /// Logging level for DAP operations.
    pub log_level: String,
    /// Operating mode (native or bridge).
    pub mode: DapMode,
    /// Workspace root directory.
    pub workspace_root: Option<std::path::PathBuf>,
}
