//! Lifecycle request handlers
//!
//! Wraps LSP lifecycle requests (initialize, shutdown, exit).

use super::super::*;

const TRACE_LEVEL_OFF: &str = "off";
const TRACE_LEVEL_MESSAGES: &str = "messages";
const TRACE_LEVEL_VERBOSE: &str = "verbose";

impl LspServer {
    fn normalize_trace_level(value: Option<&str>) -> &'static str {
        match value {
            Some(TRACE_LEVEL_OFF) => TRACE_LEVEL_OFF,
            Some(TRACE_LEVEL_MESSAGES) => TRACE_LEVEL_MESSAGES,
            Some(TRACE_LEVEL_VERBOSE) => TRACE_LEVEL_VERBOSE,
            _ => TRACE_LEVEL_OFF,
        }
    }

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
    /// Updates the server trace level. Valid values per LSP 3.18 TraceValue: "off", "messages",
    /// "verbose". Invalid string values default to "off". If the "value" key is absent or not a
    /// string the trace level is left unchanged (malformed notification, defensive ignore).
    pub(super) fn handle_set_trace_dispatch(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            if let Some(value) = params.get("value").and_then(|v| v.as_str()) {
                let level = Self::normalize_trace_level(Some(value));
                tracing::debug!(level, "Trace level set");
                *self.trace_level.lock() = level.to_string();
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
        if current_level == TRACE_LEVEL_OFF {
            return;
        }
        let mut params = json!({
            "message": message
        });
        if current_level == TRACE_LEVEL_VERBOSE {
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
    fn initialized_requires_initialize_request_first() {
        let server = LspServer::new();

        let result = server.handle_initialized_dispatch();

        assert!(result.is_err(), "initialized before initialize must error");
        assert!(!server.is_initialized(), "server must remain uninitialized");
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

    #[test]
    fn set_trace_invalid_value_defaults_to_off() -> Result<(), JsonRpcError> {
        let server = LspServer::new();
        server.handle_set_trace_dispatch(Some(json!({ "value": "verbose" })))?;
        assert_eq!(server.trace_level.lock().as_str(), TRACE_LEVEL_VERBOSE);

        server.handle_set_trace_dispatch(Some(json!({ "value": "invalid-value" })))?;
        assert_eq!(server.trace_level.lock().as_str(), TRACE_LEVEL_OFF);
        Ok(())
    }

    #[test]
    fn set_trace_missing_value_key_preserves_current_level() -> Result<(), JsonRpcError> {
        // LSP spec: "value" is required in $/setTrace params. A malformed notification that
        // omits the key should be silently ignored — server must not reset to "off".
        let server = LspServer::new();
        server.handle_set_trace_dispatch(Some(json!({ "value": "messages" })))?;
        assert_eq!(server.trace_level.lock().as_str(), TRACE_LEVEL_MESSAGES);

        // Malformed: params present but "value" key absent — level must be preserved
        server.handle_set_trace_dispatch(Some(json!({})))?;
        assert_eq!(
            server.trace_level.lock().as_str(),
            TRACE_LEVEL_MESSAGES,
            "missing value key must not reset trace level"
        );
        Ok(())
    }

    #[test]
    fn set_trace_null_params_preserves_current_level() -> Result<(), JsonRpcError> {
        // params=None (notification with no params) must not mutate trace level.
        let server = LspServer::new();
        server.handle_set_trace_dispatch(Some(json!({ "value": "verbose" })))?;
        assert_eq!(server.trace_level.lock().as_str(), TRACE_LEVEL_VERBOSE);

        server.handle_set_trace_dispatch(None)?;
        assert_eq!(
            server.trace_level.lock().as_str(),
            TRACE_LEVEL_VERBOSE,
            "null params must not reset trace level"
        );
        Ok(())
    }

    #[test]
    fn set_trace_all_valid_values_roundtrip() -> Result<(), JsonRpcError> {
        // Verify each spec-defined TraceValue is accepted and stored exactly.
        let server = LspServer::new();

        server.handle_set_trace_dispatch(Some(json!({ "value": "off" })))?;
        assert_eq!(server.trace_level.lock().as_str(), TRACE_LEVEL_OFF);

        server.handle_set_trace_dispatch(Some(json!({ "value": "messages" })))?;
        assert_eq!(server.trace_level.lock().as_str(), TRACE_LEVEL_MESSAGES);

        server.handle_set_trace_dispatch(Some(json!({ "value": "verbose" })))?;
        assert_eq!(server.trace_level.lock().as_str(), TRACE_LEVEL_VERBOSE);

        Ok(())
    }

    #[derive(Clone, Copy, Debug)]
    enum LifecycleAction {
        Initialize,
        InitializedNotification,
        AutoInitializeCompat,
    }

    #[derive(Debug, Default)]
    struct LifecycleModel {
        initialize_requested: bool,
        initialized: bool,
    }

    impl LifecycleModel {
        fn initialize(&mut self) -> Result<(), i32> {
            if self.initialize_requested {
                return Err(-32600);
            }
            self.initialize_requested = true;
            Ok(())
        }

        fn initialized_notification(&mut self) -> Result<(), i32> {
            if !self.initialize_requested {
                return Err(-32002);
            }
            if self.initialized {
                return Err(-32600);
            }
            self.initialized = true;
            Ok(())
        }

        fn auto_initialize_compat(&mut self) {
            if self.initialize_requested {
                self.initialized = true;
            }
        }
    }

    fn action_strategy() -> impl Strategy<Value = LifecycleAction> {
        prop_oneof![
            Just(LifecycleAction::Initialize),
            Just(LifecycleAction::InitializedNotification),
            Just(LifecycleAction::AutoInitializeCompat),
        ]
    }

    proptest! {
        #[test]
        fn proptest_lifecycle_state_machine(actions in prop::collection::vec(action_strategy(), 0..32)) {
            let server = LspServer::new();
            let mut model = LifecycleModel::default();

            for action in actions {
                match action {
                    LifecycleAction::Initialize => {
                        let actual = server.handle_initialize(None).map(|_| ());
                        let expected = model.initialize();
                        // Use prop_assert_eq! (not assert_eq!) throughout so proptest can
                        // shrink failing sequences rather than panicking on first mismatch.
                        prop_assert_eq!(
                            actual.is_ok(),
                            expected.is_ok(),
                            "initialize result should match model"
                        );
                        if let (Err(actual_error), Err(expected_code)) = (&actual, &expected) {
                            prop_assert_eq!(actual_error.code, *expected_code);
                        }
                    }
                    LifecycleAction::InitializedNotification => {
                        let actual = server.handle_initialized_dispatch().map(|_| ());
                        let expected = model.initialized_notification();
                        prop_assert_eq!(
                            actual.is_ok(),
                            expected.is_ok(),
                            "initialized notification result should match model"
                        );
                        if let (Err(actual_error), Err(expected_code)) = (&actual, &expected) {
                            prop_assert_eq!(actual_error.code, *expected_code);
                        }
                    }
                    LifecycleAction::AutoInitializeCompat => {
                        server.auto_initialize_for_compat("textDocument/completion");
                        model.auto_initialize_compat();
                    }
                }

                // Assert both observable state fields against the model after every action.
                prop_assert_eq!(
                    server.initialize_requested.load(Ordering::Acquire),
                    model.initialize_requested,
                    "initialize_requested flag must track model"
                );
                prop_assert_eq!(
                    server.is_initialized(),
                    model.initialized,
                    "initialized flag must track model"
                );
            }
        }
    }
}
