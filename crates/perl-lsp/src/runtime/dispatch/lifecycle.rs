//! Lifecycle request handlers
//!
//! Wraps LSP lifecycle requests (initialize, shutdown, exit).

use super::super::*;

impl LspServer {
    /// Handle initialize request
    pub(super) fn handle_initialize_dispatch(
        &mut self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_initialize(params)
    }

    /// Handle shutdown request
    pub(super) fn handle_shutdown_dispatch(&mut self) -> Result<Option<Value>, JsonRpcError> {
        // Clear any pending cancelled requests on shutdown
        self.cancelled.lock().clear();
        self.shutdown_received = true;
        Ok(Some(json!(null)))
    }

    /// Handle exit request
    pub(super) fn handle_exit_dispatch(&mut self) -> Result<Option<Value>, JsonRpcError> {
        // LSP spec: exit with 0 if shutdown was called, 1 otherwise
        let exit_code = if self.shutdown_received { 0 } else { 1 };
        eprintln!("LSP server exiting with code {}", exit_code);
        std::process::exit(exit_code);
    }

    /// Handle initialized notification
    pub(super) fn handle_initialized_dispatch(&mut self) -> Result<Option<Value>, JsonRpcError> {
        self.initialized = true;
        eprintln!("Server initialized");

        // Register file watchers for Perl files only if client supports it
        if self.client_capabilities.dynamic_registration_support {
            self.register_file_watchers_async();
        }

        // Start workspace indexing in the background (if workspace folders exist)
        #[cfg(feature = "workspace")]
        self.start_workspace_indexing();

        // Send index-ready notification
        let _ = self.send_index_ready_notification();

        Ok(None)
    }
}
