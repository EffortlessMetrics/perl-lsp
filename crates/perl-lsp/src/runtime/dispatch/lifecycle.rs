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

    /// Handle $/setTrace notification
    ///
    /// Updates the server trace level. Valid values: "off", "messages", "verbose".
    /// Invalid values default to "off" per LSP spec.
    pub(super) fn handle_set_trace_dispatch(
        &mut self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            if let Some(value) = params.get("value").and_then(|v| v.as_str()) {
                let level = match value {
                    "off" | "messages" | "verbose" => value.to_string(),
                    _ => "off".to_string(),
                };
                eprintln!("Trace level set to: {}", level);
                *self.trace_level.lock() = level;
            }
        }
        Ok(None) // Notification, no response
    }

    /// Send $/logTrace notification to client
    ///
    /// Only sends if trace level is "messages" or "verbose".
    /// The verbose field is only included when trace level is "verbose".
    #[allow(dead_code)]
    pub(crate) fn send_log_trace(&self, message: &str, verbose: Option<&str>) {
        let current_level = self.trace_level.lock().clone();
        if current_level == "off" {
            return;
        }
        let mut params = json!({
            "message": message
        });
        if current_level == "verbose" {
            if let Some(v) = verbose {
                params["verbose"] = json!(v);
            }
        }
        let _ = self.notify("$/logTrace", params);
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
