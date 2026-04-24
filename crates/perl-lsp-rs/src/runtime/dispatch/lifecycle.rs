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
    fn bdd_given_no_initialize_request_when_initialized_notification_then_server_not_initialized_error()
     {
        // Given a fresh server with no initialize request
        let server = LspServer::new();

        // When the client sends initialized
        let result = server.handle_initialized_dispatch();

        // Then the server rejects it as not initialized yet
        assert!(result.is_err(), "initialized before initialize must error");
        let error_code = result.err().map(|error| error.code);
        assert_eq!(error_code, Some(-32002), "must return ServerNotInitialized error code");
        assert!(!server.is_initialized(), "server must remain uninitialized");
    }

    #[test]
    fn bdd_given_initialized_server_when_initialized_notification_repeated_then_invalid_request_error()
    -> Result<(), JsonRpcError> {
        // Given a server that has completed initialize + initialized once
        let server = LspServer::new();
        let initialize_result = server.handle_initialize(None);
        assert!(initialize_result.is_ok(), "initialize request should succeed");

        // When initialized is sent for the first and second time
        let first = server.handle_initialized_dispatch();
        let second = server.handle_initialized_dispatch();

        // Then first succeeds, second fails with InvalidRequest
        assert!(first.is_ok(), "first initialized must succeed");
        assert!(second.is_err(), "second initialized must error");
        let error_code = second.err().map(|error| error.code);
        assert_eq!(error_code, Some(-32600), "must return InvalidRequest error code");
        Ok(())
    }

    #[test]
    fn bdd_given_initialize_requested_when_compat_request_arrives_then_server_auto_initializes()
    -> Result<(), JsonRpcError> {
        // Given initialize was requested, but initialized notification was skipped
        let server = LspServer::new();
        let initialize_result = server.handle_initialize(None);
        assert!(initialize_result.is_ok(), "initialize request should succeed");
        assert!(!server.is_initialized(), "server must not be initialized yet");

        // When a normal request is dispatched through compatibility path
        server.auto_initialize_for_compat("textDocument/hover");

        // Then initialized state is promoted automatically
        assert!(server.is_initialized(), "compatibility path should mark server initialized");
        Ok(())
    }

    #[test]
    fn bdd_given_no_initialize_request_when_compat_request_arrives_then_server_stays_uninitialized()
    {
        // Given initialize was never requested
        let server = LspServer::new();
        assert!(!server.initialize_requested.load(Ordering::Acquire));

        // When compatibility path is called
        server.auto_initialize_for_compat("textDocument/hover");

        // Then server should remain uninitialized
        assert!(!server.is_initialized());
    }

    #[test]
    fn bdd_given_cancelled_requests_when_shutdown_dispatch_then_state_is_cleared_and_shutdown_set()
    {
        // Given pending cancelled request ids
        let server = LspServer::new();
        server.cancelled.lock().insert(json!(1));
        server.cancelled.lock().insert(json!("abc"));
        assert_eq!(server.cancelled.lock().len(), 2);

        // When shutdown is handled
        let result = server.handle_shutdown_dispatch();

        // Then cancelled state is cleared and shutdown is marked
        assert!(result.is_ok(), "shutdown dispatch should succeed");
        assert_eq!(result.ok(), Some(Some(json!(null))));
        assert!(server.cancelled.lock().is_empty(), "shutdown must clear cancelled ids");
        assert!(server.shutdown_received.load(Ordering::Acquire), "shutdown flag must be set");
    }

    #[test]
    fn bdd_given_trace_level_messages_when_set_trace_verbose_then_level_updates() {
        // Given trace level is messages
        let server = LspServer::new();
        *server.trace_level.lock() = "messages".to_string();

        // When client sets trace to verbose
        let result = server.handle_set_trace_dispatch(Some(json!({ "value": "verbose" })));

        // Then level is updated to verbose and call succeeds
        assert!(result.is_ok(), "setTrace should succeed");
        assert_eq!(result.ok(), Some(None));
        assert_eq!(server.trace_level.lock().as_str(), "verbose");
    }

    #[test]
    fn bdd_given_trace_level_verbose_when_set_trace_invalid_then_level_defaults_to_off() {
        // Given trace level is verbose
        let server = LspServer::new();
        *server.trace_level.lock() = "verbose".to_string();

        // When client sends invalid trace value
        let result = server.handle_set_trace_dispatch(Some(json!({ "value": "invalid-level" })));

        // Then level falls back to off per spec
        assert!(result.is_ok(), "invalid setTrace should still succeed");
        assert_eq!(result.ok(), Some(None));
        assert_eq!(server.trace_level.lock().as_str(), "off");
    }
}
