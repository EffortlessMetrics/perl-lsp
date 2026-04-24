//! Lifecycle request handlers
//!
//! Wraps LSP lifecycle requests (initialize, shutdown, exit).

use super::super::*;

impl LspServer {
    fn complete_initialization(&self) {
        if self
            .initialized
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }

        tracing::info!("Server initialized");

        // Register file watchers for Perl files only if client supports it
        if self.client_capabilities.lock().dynamic_registration_support {
            self.register_file_watchers_async();
        }

        // Start workspace indexing in the background (if workspace folders exist)
        #[cfg(feature = "workspace")]
        self.start_workspace_indexing();

        // Send index-ready notification
        if let Err(e) = self.send_index_ready_notification() {
            tracing::warn!(error = %e, "Failed to send index-ready notification");
        }

        if std::env::var("PERL_LSP_QUIET").is_err() {
            let folder_count = self.workspace_folders.lock().len();
            if folder_count == 0 {
                tracing::info!("perl-lsp ready (single-file mode)");
            } else {
                tracing::info!(folder_count, "perl-lsp ready");
            }
        }
    }

    pub(super) fn auto_initialize_for_compat(&self, method: &str) {
        if self.initialize_requested.load(Ordering::Acquire)
            && !self.initialized.load(Ordering::Acquire)
        {
            tracing::warn!(
                method,
                "Client skipped initialized notification; auto-initializing for compatibility"
            );
            self.complete_initialization();
        }
    }

    /// Handle initialize request
    pub(super) fn handle_initialize_dispatch(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_initialize(params)
    }

    /// Handle shutdown request
    pub(super) fn handle_shutdown_dispatch(&self) -> Result<Option<Value>, JsonRpcError> {
        // Clear any pending cancelled requests on shutdown
        self.cancelled.lock().clear();
        self.shutdown_received.store(true, Ordering::Release);
        Ok(Some(json!(null)))
    }

    /// Handle exit request
    pub(super) fn handle_exit_dispatch(&self) -> Result<Option<Value>, JsonRpcError> {
        // LSP spec: exit with 0 if shutdown was called, 1 otherwise
        let exit_code = if self.shutdown_received.load(Ordering::Acquire) { 0 } else { 1 };
        tracing::info!(exit_code, "LSP server exiting");
        std::process::exit(exit_code);
    }

    /// Handle $/setTrace notification
    ///
    /// Updates the server trace level. Valid values: "off", "messages", "verbose".
    /// Invalid values default to "off" per LSP spec.
    pub(super) fn handle_set_trace_dispatch(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            if let Some(value) = params.get("value").and_then(|v| v.as_str()) {
                let level = match value {
                    "off" | "messages" | "verbose" => value.to_string(),
                    _ => "off".to_string(),
                };
                tracing::debug!(level, "Trace level set");
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
        if let Err(e) = self.notify("$/logTrace", params) {
            tracing::warn!(error = %e, "Failed to send logTrace notification");
        }
    }

    /// Handle initialized notification
    pub(super) fn handle_initialized_dispatch(&self) -> Result<Option<Value>, JsonRpcError> {
        if !self.initialize_requested.load(Ordering::Acquire) {
            return Err(JsonRpcError {
                code: -32002, // ServerNotInitialized per LSP spec
                message: "Server not initialized".to_string(),
                data: None,
            });
        }

        if self.initialized.load(Ordering::Acquire) {
            return Err(JsonRpcError {
                code: -32600, // InvalidRequest per LSP spec
                message: "initialized notification may only be sent once".to_string(),
                data: None,
            });
        }

        self.complete_initialization();

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialized_requires_initialize_request_first() {
        let server = LspServer::new();

        let result = server.handle_initialized_dispatch();

        assert!(result.is_err(), "initialized before initialize must error");
        let Err(error) = result else {
            return;
        };
        assert_eq!(
            error.code, -32002,
            "initialized before initialize must return ServerNotInitialized",
        );
        assert_eq!(error.message, "Server not initialized");
        assert!(!server.is_initialized(), "server must remain uninitialized");
    }

    #[test]
    fn initialized_can_only_be_sent_once() -> Result<(), JsonRpcError> {
        let server = LspServer::new();
        server.handle_initialize(None)?;

        let first = server.handle_initialized_dispatch()?;
        let second = server.handle_initialized_dispatch();

        assert!(first.is_none(), "initialized notification should not return a payload");
        assert!(second.is_err(), "second initialized must error");
        let Err(error) = second else {
            return Ok(());
        };
        assert_eq!(error.code, -32600, "second initialized must be InvalidRequest");
        assert_eq!(
            error.message, "initialized notification may only be sent once",
            "second initialized message must explain one-time semantics",
        );
        Ok(())
    }

    #[test]
    fn auto_initialize_for_compat_promotes_initialized_state() -> Result<(), JsonRpcError> {
        let server = LspServer::new();
        server.handle_initialize(None)?;

        server.auto_initialize_for_compat("textDocument/hover");

        assert!(server.is_initialized(), "compatibility path should mark server initialized");
        Ok(())
    }

    #[test]
    fn auto_initialize_for_compat_requires_initialize_request() {
        let server = LspServer::new();

        server.auto_initialize_for_compat("textDocument/hover");

        assert!(
            !server.is_initialized(),
            "compatibility path must not initialize server without initialize request",
        );
    }

    #[test]
    fn shutdown_dispatch_clears_cancellation_state() -> Result<(), JsonRpcError> {
        let server = LspServer::new();
        let cancelled_id = json!(9);
        server.cancel_mark(&cancelled_id);
        assert!(server.is_cancelled(&cancelled_id), "sanity check: cancel marker must be set");

        let response = server.handle_shutdown_dispatch()?;

        assert_eq!(response, Some(json!(null)), "shutdown should reply with null");
        assert!(
            !server.is_cancelled(&cancelled_id),
            "shutdown must clear pending cancellation markers",
        );
        Ok(())
    }
}
