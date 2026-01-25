//! LSP server lifecycle management
//!
//! Handles initialize/shutdown and workspace configuration.

mod capabilities;
mod module_resolution;
mod tools;
mod watchers;
mod workspace;

use super::*;
use serde_json::json;

impl LspServer {
    /// Send a notification when the workspace index is ready
    ///
    /// Also transitions the index coordinator to Ready state if there are no
    /// workspace folders to scan (i.e., ready for single-file operation).
    pub(super) fn send_index_ready_notification(&self) -> io::Result<()> {
        #[cfg(feature = "workspace")]
        let (has_symbols, file_count, symbol_count) = {
            // Use coordinator.index() for consistency with coordinator-first approach
            if let Some(coordinator) = self.coordinator() {
                let idx = coordinator.index();
                let has = idx.has_symbols();
                let files = idx.all_symbols().len();
                (has, if has { 1 } else { 0 }, files)
            } else {
                (false, 0, 0)
            }
        };

        #[cfg(not(feature = "workspace"))]
        let (has_symbols, _file_count, _symbol_count): (bool, usize, usize) = (false, 0, 0);

        // Transition coordinator to Ready state
        // This marks the index as available for full queries
        #[cfg(feature = "workspace")]
        if let Some(coordinator) = self.coordinator() {
            // Check if workspace folders exist - if not, we're ready immediately
            let folders = self.workspace_folders.lock();
            if folders.is_empty() || has_symbols {
                coordinator.transition_to_ready(file_count, symbol_count);
                eprintln!(
                    "Index coordinator transitioned to Ready (files: {}, symbols: {})",
                    file_count, symbol_count
                );
            } else {
                // Workspace folders exist but no symbols indexed yet
                // Stay in Building state - files will be indexed as they're opened
                // or when file watcher events are received
                eprintln!(
                    "Index coordinator remaining in Building state ({} workspace folders, awaiting files)",
                    folders.len()
                );
            }
        }

        self.notify(
            "perl-lsp/index-ready",
            json!({
                "ready": has_symbols
            }),
        )
    }
}
