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

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn initialized_requires_initialize_request_first() {
        let server = LspServer::new();

        let result = server.handle_initialized_dispatch();

        assert!(result.is_err(), "initialized before initialize must error");
        if let Err(err) = result {
            assert_eq!(err.code, -32002, "must report ServerNotInitialized");
        }
        assert!(!server.is_initialized(), "server must remain uninitialized");
    }

    #[test]
    fn initialized_can_only_be_sent_once() {
        let server = LspServer::new();
        let init = server.handle_initialize(None);
        assert!(init.is_ok(), "initialize request should succeed");

        let first = server.handle_initialized_dispatch();
        let second = server.handle_initialized_dispatch();

        assert!(first.is_ok(), "first initialized must succeed");
        assert!(second.is_err(), "second initialized must error");
        if let Err(err) = second {
            assert_eq!(err.code, -32600, "must report InvalidRequest for duplicate initialized");
        }
    }

    #[test]
    fn auto_initialize_for_compat_promotes_initialized_state() {
        let server = LspServer::new();
        let init = server.handle_initialize(None);
        assert!(init.is_ok(), "initialize request should succeed");

        server.auto_initialize_for_compat("textDocument/hover");

        assert!(server.is_initialized(), "compatibility path should mark server initialized");
    }

    #[test]
    fn auto_initialize_for_compat_requires_initialize_request() -> TestResult {
        let server = LspServer::new();

        server.auto_initialize_for_compat("textDocument/hover");

        assert!(
            !server.is_initialized(),
            "compatibility path must not initialize server before initialize request"
        );
        Ok(())
    }

    #[test]
    fn auto_initialize_for_compat_noop_when_already_initialized() -> TestResult {
        let server = LspServer::new();
        let init = server.handle_initialize(None);
        assert!(init.is_ok(), "initialize request should succeed");
        let first_initialized = server.handle_initialized_dispatch();
        assert!(first_initialized.is_ok(), "first initialized notification should succeed");

        server.auto_initialize_for_compat("textDocument/hover");

        assert!(server.is_initialized(), "server should remain initialized");
        Ok(())
    }

    #[test]
    fn set_trace_accepts_known_values() -> TestResult {
        let server = LspServer::new();

        let result_messages =
            server.handle_set_trace_dispatch(Some(json!({ "value": "messages" })));
        assert!(result_messages.is_ok(), "setTrace(messages) should succeed");
        assert_eq!(
            server.trace_level.lock().as_str(),
            "messages",
            "setTrace(messages) should store messages level"
        );

        let result_verbose = server.handle_set_trace_dispatch(Some(json!({ "value": "verbose" })));
        assert!(result_verbose.is_ok(), "setTrace(verbose) should succeed");
        assert_eq!(
            server.trace_level.lock().as_str(),
            "verbose",
            "setTrace(verbose) should store verbose level"
        );
        Ok(())
    }

    #[test]
    fn set_trace_invalid_value_defaults_to_off() -> TestResult {
        let server = LspServer::new();
        let set_messages = server.handle_set_trace_dispatch(Some(json!({ "value": "messages" })));
        assert!(set_messages.is_ok(), "setTrace(messages) should succeed");
        assert_eq!(
            server.trace_level.lock().as_str(),
            "messages",
            "precondition: messages level should be set"
        );

        let invalid = server.handle_set_trace_dispatch(Some(json!({ "value": "invalid" })));
        assert!(invalid.is_ok(), "setTrace(invalid) should still be accepted as notification");
        assert_eq!(
            server.trace_level.lock().as_str(),
            "off",
            "invalid trace level must be coerced to off"
        );
        Ok(())
    }

    #[test]
    fn set_trace_explicit_off_stores_off() -> TestResult {
        let server = LspServer::new();
        // First set to a non-default level so we can verify the write.
        let r = server.handle_set_trace_dispatch(Some(json!({ "value": "verbose" })));
        assert!(r.is_ok(), "setTrace(verbose) should succeed");
        assert_eq!(server.trace_level.lock().as_str(), "verbose", "precondition");

        // Explicit "off" must store "off", not be treated as an invalid value.
        let r = server.handle_set_trace_dispatch(Some(json!({ "value": "off" })));
        assert!(r.is_ok(), "setTrace(off) should succeed");
        assert_eq!(
            server.trace_level.lock().as_str(),
            "off",
            "explicit off level must be stored as off"
        );
        Ok(())
    }
}
