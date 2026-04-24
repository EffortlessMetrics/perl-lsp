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
    use proptest::prelude::*;

    #[test]
    fn initialized_requires_initialize_request_first() -> Result<(), JsonRpcError> {
        let server = LspServer::new();

        let result = server.handle_initialized_dispatch();

        assert!(result.is_err(), "initialized before initialize must error");
        assert!(!server.is_initialized(), "server must remain uninitialized");
        Ok(())
    }

    #[test]
    fn initialized_can_only_be_sent_once() -> Result<(), JsonRpcError> {
        let server = LspServer::new();
        server.handle_initialize(None)?;

        let first = server.handle_initialized_dispatch();
        let second = server.handle_initialized_dispatch();

        assert!(first.is_ok(), "first initialized must succeed");
        assert!(second.is_err(), "second initialized must error");
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

    #[derive(Clone, Debug)]
    enum LifecycleAction {
        InitializeNull,
        InitializeEmptyObject,
        Initialized,
        AutoInitializeCompat,
        Shutdown,
        SetTraceOff,
        SetTraceMessages,
        SetTraceVerbose,
        SetTraceInvalid,
        SetTraceMissingValue,
    }

    fn lifecycle_action_strategy() -> impl Strategy<Value = LifecycleAction> {
        prop_oneof![
            Just(LifecycleAction::InitializeNull),
            Just(LifecycleAction::InitializeEmptyObject),
            Just(LifecycleAction::Initialized),
            Just(LifecycleAction::AutoInitializeCompat),
            Just(LifecycleAction::Shutdown),
            Just(LifecycleAction::SetTraceOff),
            Just(LifecycleAction::SetTraceMessages),
            Just(LifecycleAction::SetTraceVerbose),
            Just(LifecycleAction::SetTraceInvalid),
            Just(LifecycleAction::SetTraceMissingValue),
        ]
    }

    proptest! {
        #[test]
        fn lifecycle_dispatch_fuzz_does_not_violate_core_invariants(
            actions in prop::collection::vec(lifecycle_action_strategy(), 0..128)
        ) {
            let server = LspServer::new();

            for action in actions {
                match action {
                    LifecycleAction::InitializeNull => {
                        let _ = server.handle_initialize_dispatch(None);
                    }
                    LifecycleAction::InitializeEmptyObject => {
                        let _ = server.handle_initialize_dispatch(Some(json!({})));
                    }
                    LifecycleAction::Initialized => {
                        let initialized_result = server.handle_initialized_dispatch();
                        if !server.initialize_requested.load(Ordering::Acquire) {
                            prop_assert!(
                                matches!(
                                    initialized_result,
                                    Err(JsonRpcError {
                                        code: -32002,
                                        ..
                                    })
                                ),
                                "initialized before initialize request must return ServerNotInitialized"
                            );
                        }
                    }
                    LifecycleAction::AutoInitializeCompat => {
                        server.auto_initialize_for_compat("textDocument/hover");
                    }
                    LifecycleAction::Shutdown => {
                        let _ = server.handle_shutdown_dispatch();
                    }
                    LifecycleAction::SetTraceOff => {
                        let _ = server.handle_set_trace_dispatch(Some(json!({ "value": "off" })));
                    }
                    LifecycleAction::SetTraceMessages => {
                        let _ = server.handle_set_trace_dispatch(Some(json!({ "value": "messages" })));
                    }
                    LifecycleAction::SetTraceVerbose => {
                        let _ = server.handle_set_trace_dispatch(Some(json!({ "value": "verbose" })));
                    }
                    LifecycleAction::SetTraceInvalid => {
                        let _ = server.handle_set_trace_dispatch(Some(json!({ "value": "unexpected" })));
                    }
                    LifecycleAction::SetTraceMissingValue => {
                        let _ = server.handle_set_trace_dispatch(Some(json!({ "not_value": true })));
                    }
                }

                if server.is_initialized() {
                    prop_assert!(
                        server.initialize_requested.load(Ordering::Acquire),
                        "initialized state requires initialize request"
                    );
                }

                let trace_level = server.trace_level.lock().clone();
                prop_assert!(
                    matches!(trace_level.as_str(), "off" | "messages" | "verbose"),
                    "trace level must always remain in the allowed set"
                );
            }
        }
    }
}
