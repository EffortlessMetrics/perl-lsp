//! LSP server lifecycle management
//!
//! This module manages the LSP server initialization, shutdown, and workspace configuration.
//! It implements the LSP lifecycle protocol and coordinates workspace-wide resources.
//!
//! # Architecture
//!
//! The lifecycle layer is organized into focused submodules:
//!
//! - **capabilities**: Server capability declaration and client capability negotiation
//! - **module_resolution**: Perl module path resolution and `@INC` configuration
//! - **tools**: External tool integration (perltidy, perlcritic)
//! - **watchers**: File system watchers for workspace file changes
//! - **workspace**: Workspace folder management and root path resolution
//!
//! # Initialization Flow
//!
//! 1. `initialize` request received with client capabilities
//! 2. Server capabilities computed based on client support
//! 3. Workspace folders registered and watchers configured
//! 4. Module resolution paths established from workspace structure
//! 5. `initialized` notification confirms server readiness
//! 6. Index coordinator transitions to Building or Ready state
//!
//! # Workspace Index Integration
//!
//! The lifecycle module coordinates with the workspace index coordinator:
//!
//! - Transitions index to Ready state when workspace scanning completes
//! - Sends `perl-lsp/index-ready` notification to clients
//! - Manages index state for single-file vs. workspace modes
//!
//! # Client Capability Handling
//!
//! Server adapts behavior based on client capabilities:
//!
//! - **textDocument/completion**: Snippet support, commit characters
//! - **textDocument/publishDiagnostics**: Related information, tags
//! - **workspace/workspaceFolders**: Multi-root workspace support
//! - **window/workDoneProgress**: Progress reporting for long operations
//!
//! # Shutdown Protocol
//!
//! 1. `shutdown` request received - server prepares for termination
//! 2. Ongoing operations cancelled gracefully
//! 3. `exit` notification triggers actual process termination
//! 4. Exit code 0 if shutdown was received, 1 otherwise

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
