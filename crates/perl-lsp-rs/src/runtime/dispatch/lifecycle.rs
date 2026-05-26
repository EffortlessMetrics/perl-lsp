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

        // File watcher dynamic registration is intentionally separate from
        // feature-specific dynamic registrations such as inline completion.
        self.register_file_watchers_if_needed();
        self.register_inline_completion_if_needed();

        // Start workspace indexing in the background (if workspace folders
        // exist and the eager-indexing gate allows it). The gate defaults to
        // true for normal editor sessions; e2e harness mode flips it off so
        // latency tests do not pay for indexing they will not consult.
        #[cfg(feature = "workspace")]
        if self.should_start_workspace_indexing() {
            self.start_workspace_indexing();
        } else {
            tracing::debug!(
                runtime_mode = ?self.runtime_tuning().runtime_mode,
                "Skipping eager workspace indexing on `initialized` (gate disabled)"
            );
        }

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
        // Enforce single-shutdown idempotence via atomic swap.
        // Note: The LSP router permits shutdown before initialize_requested
        // (see dispatch/mod.rs), so we do not check initialize_requested here.
        if self.shutdown_received.swap(true, Ordering::AcqRel) {
            return Err(JsonRpcError {
                code: -32600, // InvalidRequest per LSP spec
                message: "shutdown request may only be sent once".to_string(),
                data: None,
            });
        }

        // Clear any pending cancelled requests on shutdown
        self.cancelled.lock().clear();
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
    pub(crate) fn handle_initialized_dispatch(&self) -> Result<Option<Value>, JsonRpcError> {
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

    type TestResult = Result<(), String>;

    // ── BDD lifecycle dispatch scenarios ────────────────────────────────────

    #[test]
    fn given_fresh_server_when_initialized_notification_arrives_then_server_not_initialized_error_returned()
    -> TestResult {
        // Given
        let server = LspServer::new();

        // When
        let result = server.handle_initialized_dispatch();

        // Then
        let error =
            result.err().ok_or("expected initialized to fail, but it succeeded".to_string())?;
        assert_eq!(error.code, -32002, "must be ServerNotInitialized (-32002) per LSP spec");
        assert!(!server.is_initialized(), "server must remain uninitialized");
        Ok(())
    }

    #[test]
    fn given_initialized_server_when_initialized_notification_sent_twice_then_second_request_is_invalid()
    -> TestResult {
        // Given
        let server = LspServer::new();
        server
            .handle_initialize(None)
            .map_err(|e| format!("initialize request should succeed: {e}"))?;

        // When
        let first = server.handle_initialized_dispatch();
        let second = server.handle_initialized_dispatch();

        // Then
        assert!(first.is_ok(), "first initialized must succeed");
        let second_error = second
            .err()
            .ok_or("expected second initialized to fail, but it succeeded".to_string())?;
        assert_eq!(second_error.code, -32600, "must be InvalidRequest (-32600) per LSP spec");
        Ok(())
    }

    #[test]
    fn given_initialize_request_without_initialized_when_compat_mode_runs_then_server_becomes_initialized()
    -> TestResult {
        // Given
        let server = LspServer::new();
        server
            .handle_initialize(None)
            .map_err(|e| format!("initialize request should succeed: {e}"))?;

        // When
        server.auto_initialize_for_compat("textDocument/hover");

        // Then
        assert!(server.is_initialized(), "compatibility path should mark server initialized");
        Ok(())
    }

    #[test]
    fn given_server_not_in_initialize_phase_when_compat_mode_runs_then_server_stays_uninitialized()
    {
        // Given
        let server = LspServer::new();

        // When
        server.auto_initialize_for_compat("textDocument/hover");

        // Then
        assert!(!server.is_initialized(), "compat mode must no-op before initialize request");
    }

    #[test]
    fn given_server_receives_shutdown_when_shutdown_dispatch_runs_then_shutdown_flag_and_null_response_are_set()
    -> TestResult {
        // Given — fully initialize so shutdown is valid per LSP spec
        let server = LspServer::new();
        server
            .handle_initialize(None)
            .map_err(|e| format!("initialize request should succeed: {e}"))?;
        server
            .handle_initialized_dispatch()
            .map_err(|e| format!("initialized notification should succeed: {e}"))?;

        // When
        let response = server
            .handle_shutdown_dispatch()
            .map_err(|e| format!("shutdown should succeed: {e}"))?;

        // Then
        assert_eq!(response, Some(json!(null)), "shutdown returns JSON null per LSP spec");
        assert!(
            server.shutdown_received.load(Ordering::Acquire),
            "shutdown_received must be set (exit will use code 0)"
        );
        Ok(())
    }

    #[test]
    fn given_shutdown_already_received_when_shutdown_sent_again_then_invalid_request_error_returned()
    -> TestResult {
        // Given
        let server = LspServer::new();
        server
            .handle_initialize(None)
            .map_err(|e| format!("initialize request should succeed: {e}"))?;

        // When
        let first = server.handle_shutdown_dispatch();
        let second = server.handle_shutdown_dispatch();

        // Then
        assert!(first.is_ok(), "first shutdown must succeed");
        let second_error =
            second.err().ok_or("expected second shutdown to fail, but it succeeded".to_string())?;
        assert_eq!(
            second_error.code, -32600,
            "second shutdown must return InvalidRequest (-32600) per LSP spec"
        );
        Ok(())
    }

    #[test]
    fn given_trace_notification_with_unknown_level_when_set_trace_dispatch_runs_then_trace_defaults_to_off()
    -> TestResult {
        // Given
        let server = LspServer::new();

        // When
        server
            .handle_set_trace_dispatch(Some(json!({"value": "unexpected"})))
            .map_err(|e| format!("setTrace should succeed: {e}"))?;

        // Then
        assert_eq!(
            server.trace_level.lock().as_str(),
            TRACE_LEVEL_OFF,
            "unknown trace value must default to 'off'"
        );
        Ok(())
    }

    #[test]
    fn given_trace_notification_with_verbose_level_when_set_trace_dispatch_runs_then_trace_level_is_updated()
    -> TestResult {
        // Given
        let server = LspServer::new();

        // When
        server
            .handle_set_trace_dispatch(Some(json!({"value": "verbose"})))
            .map_err(|e| format!("setTrace should succeed: {e}"))?;

        // Then
        assert_eq!(
            server.trace_level.lock().as_str(),
            TRACE_LEVEL_VERBOSE,
            "verbose is a valid LSP TraceValue and must be stored exactly"
        );
        Ok(())
    }

    #[test]
    fn given_trace_notification_with_messages_level_when_set_trace_dispatch_runs_then_trace_level_is_messages()
    -> TestResult {
        // Given
        let server = LspServer::new();

        // When
        server
            .handle_set_trace_dispatch(Some(json!({"value": "messages"})))
            .map_err(|e| format!("setTrace should succeed: {e}"))?;

        // Then
        assert_eq!(
            server.trace_level.lock().as_str(),
            TRACE_LEVEL_MESSAGES,
            "messages is a valid LSP TraceValue and must be stored exactly"
        );
        Ok(())
    }

    #[test]
    fn given_set_trace_with_no_params_when_dispatch_runs_then_trace_level_is_unchanged()
    -> TestResult {
        // Given — establish a non-default level first
        let server = LspServer::new();
        server
            .handle_set_trace_dispatch(Some(json!({"value": "verbose"})))
            .map_err(|e| format!("setTrace should succeed: {e}"))?;

        // When — params=None (malformed/missing notification body)
        server
            .handle_set_trace_dispatch(None)
            .map_err(|e| format!("setTrace with no params should not error: {e}"))?;

        // Then — level must be preserved; None params must not reset to "off"
        assert_eq!(
            server.trace_level.lock().as_str(),
            TRACE_LEVEL_VERBOSE,
            "missing params must not reset trace level"
        );
        Ok(())
    }

    #[test]
    fn given_set_trace_with_missing_value_key_when_dispatch_runs_then_trace_level_is_unchanged()
    -> TestResult {
        // Given — LSP spec: "value" is required in $/setTrace params; absent key must be ignored.
        let server = LspServer::new();
        server
            .handle_set_trace_dispatch(Some(json!({"value": "messages"})))
            .map_err(|e| format!("setTrace should succeed: {e}"))?;

        // When — params present but "value" key absent
        server
            .handle_set_trace_dispatch(Some(json!({})))
            .map_err(|e| format!("setTrace with empty params should not error: {e}"))?;

        // Then — level must be preserved
        assert_eq!(
            server.trace_level.lock().as_str(),
            TRACE_LEVEL_MESSAGES,
            "missing value key must not reset trace level"
        );
        Ok(())
    }

    #[test]
    fn given_all_valid_trace_values_when_set_trace_runs_then_each_is_roundtripped() -> TestResult {
        // Given
        let server = LspServer::new();

        // When / Then for each spec-defined TraceValue
        server
            .handle_set_trace_dispatch(Some(json!({"value": "off"})))
            .map_err(|e| format!("setTrace off should succeed: {e}"))?;
        assert_eq!(server.trace_level.lock().as_str(), TRACE_LEVEL_OFF, "'off' roundtrip");

        server
            .handle_set_trace_dispatch(Some(json!({"value": "messages"})))
            .map_err(|e| format!("setTrace messages should succeed: {e}"))?;
        assert_eq!(
            server.trace_level.lock().as_str(),
            TRACE_LEVEL_MESSAGES,
            "'messages' roundtrip"
        );

        server
            .handle_set_trace_dispatch(Some(json!({"value": "verbose"})))
            .map_err(|e| format!("setTrace verbose should succeed: {e}"))?;
        assert_eq!(server.trace_level.lock().as_str(), TRACE_LEVEL_VERBOSE, "'verbose' roundtrip");

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
