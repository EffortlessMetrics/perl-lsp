//! Tool detection and management
//!
//! Handles detection of external tools like perltidy and perlcritic.

use super::super::*;

impl LspServer {
    /// Detect if a tool is available on the system
    ///
    /// Uses which/where command which is much faster than spawning the actual tools.
    /// This is used during initialization to determine which features to advertise.
    pub(crate) fn detect_tool(&self, tool_name: &str) -> bool {
        if cfg!(target_os = "windows") {
            std::process::Command::new("where")
                .arg(tool_name)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        } else {
            std::process::Command::new("which")
                .arg(tool_name)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        }
    }
}
