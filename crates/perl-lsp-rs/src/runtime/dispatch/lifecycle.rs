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

    type TestResult = Result<(), String>;

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
        assert_eq!(error.code, -32002, "must be ServerNotInitialized");
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
        assert_eq!(second_error.code, -32600, "must be InvalidRequest");
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
        assert!(!server.is_initialized(), "compat mode should no-op before initialize");
    }

    #[test]
    fn given_server_receives_shutdown_when_shutdown_dispatch_runs_then_shutdown_flag_and_null_response_are_set()
    -> TestResult {
        // Given
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
        assert_eq!(response, Some(json!(null)), "shutdown returns JSON null");
        assert!(server.shutdown_received.load(Ordering::Acquire), "shutdown flag should be set");
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
        assert_eq!(server.trace_level.lock().as_str(), "off");
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
        assert_eq!(server.trace_level.lock().as_str(), "verbose");
        Ok(())
    }

    // --- Property-based sequence fuzzing ---
    //
    // Drives arbitrary orderings of lifecycle calls and asserts structural
    // invariants that must hold regardless of sequencing.
    //
    // This is *constrained* proptest, not malformed-input fuzzing: each action
    // is a fixed valid call. The value under test is the ordering invariants,
    // not input parsing robustness (that is tested via dedicated fuzz targets
    // in the fuzz/ workspace member).

    #[derive(Clone, Debug)]
    enum LifecycleAction {
        InitializeNull,
        InitializeEmptyObject,
        /// Sends `initialized` notification (valid or premature)
        Initialized,
        AutoInitializeCompat,
        Shutdown,
        /// Sends `initialize` again after shutdown — tests double-init guard
        InitializeAfterShutdown,
        SetTraceOff,
        SetTraceMessages,
        SetTraceVerbose,
        /// Invalid trace value — must normalise to "off", never corrupt state
        SetTraceInvalid,
        /// Missing `value` key — must be a no-op, trace stays unchanged
        SetTraceMissingValue,
    }

    fn lifecycle_action_strategy() -> impl Strategy<Value = LifecycleAction> {
        prop_oneof![
            Just(LifecycleAction::InitializeNull),
            Just(LifecycleAction::InitializeEmptyObject),
            Just(LifecycleAction::Initialized),
            Just(LifecycleAction::AutoInitializeCompat),
            Just(LifecycleAction::Shutdown),
            Just(LifecycleAction::InitializeAfterShutdown),
            Just(LifecycleAction::SetTraceOff),
            Just(LifecycleAction::SetTraceMessages),
            Just(LifecycleAction::SetTraceVerbose),
            Just(LifecycleAction::SetTraceInvalid),
            Just(LifecycleAction::SetTraceMissingValue),
        ]
    }

    proptest! {
        #[test]
        fn lifecycle_dispatch_sequence_invariants(
            actions in prop::collection::vec(lifecycle_action_strategy(), 0..128)
        ) {
            let server = LspServer::new();

            for action in actions {
                // Snapshot state BEFORE the call so post-call assertions are
                // meaningful and can distinguish "this call changed state" from
                // "state was already like that."
                let was_initialized_before = server.is_initialized();
                let was_initialize_requested_before =
                    server.initialize_requested.load(Ordering::Acquire);
                let trace_before = server.trace_level.lock().clone();

                match action {
                    LifecycleAction::InitializeNull => {
                        let _ = server.handle_initialize_dispatch(None);
                    }
                    LifecycleAction::InitializeEmptyObject => {
                        let _ = server.handle_initialize_dispatch(Some(json!({})));
                    }
                    LifecycleAction::Initialized => {
                        let result = server.handle_initialized_dispatch();
                        if !was_initialize_requested_before {
                            // LSP spec: initialized before initialize MUST be
                            // ServerNotInitialized (-32002). This assertion is
                            // non-vacuous: it fires on sequences that omit
                            // InitializeNull/InitializeEmptyObject first.
                            prop_assert!(
                                matches!(
                                    result,
                                    Err(JsonRpcError { code: -32002, .. })
                                ),
                                "initialized before initialize request must return \
                                 ServerNotInitialized (-32002); got: {:?}",
                                result
                            );
                            // Server must stay un-initialized
                            prop_assert!(
                                !server.is_initialized(),
                                "premature initialized must not promote server state"
                            );
                        } else if was_initialized_before {
                            // LSP spec: second initialized MUST be InvalidRequest (-32600)
                            prop_assert!(
                                matches!(
                                    result,
                                    Err(JsonRpcError { code: -32600, .. })
                                ),
                                "duplicate initialized must return InvalidRequest (-32600); \
                                 got: {:?}",
                                result
                            );
                        }
                    }
                    LifecycleAction::AutoInitializeCompat => {
                        server.auto_initialize_for_compat("textDocument/hover");
                    }
                    LifecycleAction::Shutdown => {
                        let result = server.handle_shutdown_dispatch();
                        // shutdown_dispatch MUST succeed and return json!(null)
                        prop_assert!(
                            result.is_ok(),
                            "shutdown must always return Ok; got: {:?}",
                            result
                        );
                        prop_assert_eq!(
                            result.unwrap(),
                            Some(json!(null)),
                            "shutdown must return JSON null"
                        );
                        // After any shutdown call the flag MUST be set
                        prop_assert!(
                            server.shutdown_received.load(Ordering::Acquire),
                            "shutdown_received flag must be set after handle_shutdown_dispatch"
                        );
                    }
                    LifecycleAction::InitializeAfterShutdown => {
                        // Re-initialize after shutdown — the result is
                        // implementation-defined (may succeed or error), but
                        // the server must not panic and state must remain
                        // consistent with the invariants checked below.
                        let _ = server.handle_initialize_dispatch(None);
                    }
                    LifecycleAction::SetTraceOff => {
                        let _ = server.handle_set_trace_dispatch(Some(json!({ "value": "off" })));
                    }
                    LifecycleAction::SetTraceMessages => {
                        let _ = server.handle_set_trace_dispatch(
                            Some(json!({ "value": "messages" })),
                        );
                    }
                    LifecycleAction::SetTraceVerbose => {
                        let _ = server.handle_set_trace_dispatch(
                            Some(json!({ "value": "verbose" })),
                        );
                    }
                    LifecycleAction::SetTraceInvalid => {
                        let result = server.handle_set_trace_dispatch(
                            Some(json!({ "value": "unexpected" })),
                        );
                        prop_assert!(result.is_ok(), "setTrace must not error on invalid value");
                        // Invalid value MUST normalise to "off" — non-vacuous
                        // because trace_before may have been "messages" or "verbose".
                        let trace_after_invalid = server.trace_level.lock().clone();
                        prop_assert_eq!(
                            trace_after_invalid.as_str(),
                            "off",
                            "invalid trace value must normalise to 'off' (was '{}')",
                            trace_before
                        );
                    }
                    LifecycleAction::SetTraceMissingValue => {
                        let _ = server
                            .handle_set_trace_dispatch(Some(json!({ "not_value": true })));
                        // Missing `value` key is a no-op — trace must be unchanged
                        prop_assert_eq!(
                            server.trace_level.lock().clone(),
                            trace_before,
                            "missing 'value' key must leave trace_level unchanged"
                        );
                    }
                }

                // --- Post-action global invariants ---

                // Invariant 1 (non-vacuous when InitializeAfterShutdown or
                // AutoInitializeCompat fires without a prior Initialize):
                // is_initialized() == true implies initialize_requested == true.
                if server.is_initialized() {
                    prop_assert!(
                        server.initialize_requested.load(Ordering::Acquire),
                        "initialized state requires initialize_requested to be set"
                    );
                }

                // Invariant 2: trace_level must always be in the allowed set.
                // Non-vacuous because SetTraceInvalid tests the normalisation
                // path and SetTraceMissingValue tests the no-op path.
                let trace_level = server.trace_level.lock().clone();
                prop_assert!(
                    matches!(trace_level.as_str(), "off" | "messages" | "verbose"),
                    "trace_level '{}' is outside the allowed set {{off, messages, verbose}}",
                    trace_level
                );
            }
        }
    }
}
