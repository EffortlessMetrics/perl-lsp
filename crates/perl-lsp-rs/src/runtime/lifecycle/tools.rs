//! Tool detection and management
//!
//! Handles detection of external tools like perltidy and perlcritic.

use super::super::*;

impl LspServer {
    /// Detect if a tool is available on the system
    ///
    /// Uses the shared command resolution helper so capability advertisement
    /// matches the command execution path on each platform.
    pub(crate) fn detect_tool(&self, tool_name: &str) -> bool {
        crate::execute_command::command_exists(tool_name)
    }
}
