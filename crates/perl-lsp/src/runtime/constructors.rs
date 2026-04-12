//! LspServer constructors.
//!
//! All `LspServer::new*` and `LspServer::with_*` constructors live here
//! so that `mod.rs` is limited to the struct definition and core accessors.

use super::*;

impl LspServer {
    /// Create a new LSP server
    pub fn new() -> Self {
        Self::new_with_feature_profile(FeatureProfile::current())
    }

    /// Create a new LSP server using an explicit feature profile.
    pub fn new_with_feature_profile(feature_profile: FeatureProfile) -> Self {
        // Initialize workspace indexing with coordinator lifecycle management
        #[cfg(feature = "workspace")]
        let index_coordinator = Some(Arc::new(IndexCoordinator::new()));

        let default_features = feature_profile.advertised_features();
        let (outbound, outbound_writer_handle) = outbound::spawn_writer(Box::new(io::stdout()));

        Self {
            documents: Arc::new(Mutex::new(HashMap::new())),
            initialize_requested: AtomicBool::new(false),
            initialized: AtomicBool::new(false),
            shutdown_received: AtomicBool::new(false),
            #[cfg(feature = "workspace")]
            index_coordinator,
            // Cache up to 100 ASTs with 5 minute TTL
            ast_cache: Arc::new(AstCache::new(100, 300)),
            symbol_index: Arc::new(Mutex::new(SymbolIndex::new())),
            config: Arc::new(Mutex::new(ServerConfig::default())),
            reader: Arc::new(Mutex::new(Box::new(BufReader::new(io::stdin())))),
            outbound,
            outbound_writer_handle: Some(outbound_writer_handle),
            client_capabilities: Mutex::new(ClientCapabilities::default()),
            cancelled: Arc::new(Mutex::new(HashSet::new())),
            workspace_folders: Arc::new(Mutex::new(Vec::new())),
            root_path: Arc::new(Mutex::new(None)),
            advertised_features: Mutex::new(default_features),
            client_supports_pull_diags: Arc::new(AtomicBool::new(false)),
            workspace_config: Arc::new(Mutex::new(WorkspaceConfig::default())),
            next_request_id: Arc::new(AtomicI64::new(1)),
            pending_workspace_configuration_requests: Arc::new(Mutex::new(HashMap::new())),
            progress_tokens: Arc::new(Mutex::new(HashSet::new())),
            progress_token_to_request: Arc::new(Mutex::new(HashMap::new())),
            refresh_controller: refresh::RefreshController::new(),
            diagnostic_debouncer: Mutex::new(None),
            file_watcher_debouncer: Mutex::new(None),
            notebook_store: notebook::NotebookStore::new(),
            trace_level: Arc::new(Mutex::new("off".to_string())),
            stream_session_manager: super::stream_session::StreamSessionManager::new(),
            feature_profile,
            pod_cache: Arc::new(Mutex::new(HashMap::new())),
            pending_index_task_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            parse_cancel_flags: Arc::new(Mutex::new(HashMap::new())),
            pull_diagnostics_orchestrator: super::diagnostics::PullDiagnosticsOrchestrator::new(),
            semantic_analyzer_cache: Arc::new(Mutex::new(HashMap::new())),
            #[cfg(feature = "workspace")]
            indexing_in_progress: Arc::new(AtomicBool::new(false)),
            #[cfg(feature = "workspace")]
            permission_denied_shown: Arc::new(AtomicBool::new(false)),
            #[cfg(not(target_arch = "wasm32"))]
            critic_analyzer: Mutex::new(None),
            #[cfg(not(target_arch = "wasm32"))]
            critic_runtime_override: Mutex::new(None),
            #[cfg(not(target_arch = "wasm32"))]
            skip_perlcritic_command_check: AtomicBool::new(false),
            #[cfg(not(target_arch = "wasm32"))]
            critic_workspace_warnings_sent: Mutex::new(HashSet::new()),
            ai_inline_backend: Mutex::new(None),
        }
    }

    /// Create a new LSP server with custom I/O (for testing)
    ///
    /// This constructor allows you to provide custom Read and Write trait objects
    /// for testing purposes, enabling you to test LSP protocol edge cases without
    /// requiring actual stdin/stdout or process spawning.
    ///
    /// # Parameters
    ///
    /// - `reader`: A boxed reader implementing `Read + Send` for reading LSP messages
    /// - `writer`: A boxed writer implementing `Write + Send` for writing LSP responses
    ///
    /// # Thread Safety
    ///
    /// Both reader and writer are automatically wrapped in `Arc<Mutex<...>>` to ensure
    /// thread-safe access. The server can safely be used from multiple threads.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::io::Cursor;
    /// use perl_lsp::LspServer;
    ///
    /// let input = Cursor::new(Vec::new());
    /// let output = Vec::new();
    ///
    /// let server = LspServer::with_io(
    ///     Box::new(input),
    ///     Box::new(output)
    /// );
    /// ```
    #[allow(clippy::boxed_local)] // reader is intentionally unused for API compatibility
    pub fn with_io<R, W>(reader: Box<R>, writer: Box<W>) -> Self
    where
        R: Read + Send + 'static,
        W: Write + Send + 'static,
    {
        Self::with_io_and_feature_profile(reader, writer, FeatureProfile::current())
    }

    /// Create a new LSP server with custom I/O and explicit feature profile.
    pub fn with_io_and_feature_profile<R, W>(
        reader: Box<R>,
        writer: Box<W>,
        feature_profile: FeatureProfile,
    ) -> Self
    where
        R: Read + Send + 'static,
        W: Write + Send + 'static,
    {
        // Initialize workspace indexing with coordinator lifecycle management
        #[cfg(feature = "workspace")]
        let index_coordinator = Some(Arc::new(IndexCoordinator::new()));

        let default_features = feature_profile.advertised_features();
        let (outbound, outbound_writer_handle) =
            outbound::spawn_writer(writer as Box<dyn Write + Send>);

        Self {
            documents: Arc::new(Mutex::new(HashMap::new())),
            initialize_requested: AtomicBool::new(false),
            initialized: AtomicBool::new(false),
            shutdown_received: AtomicBool::new(false),
            #[cfg(feature = "workspace")]
            index_coordinator,
            ast_cache: Arc::new(AstCache::new(100, 300)),
            symbol_index: Arc::new(Mutex::new(SymbolIndex::new())),
            config: Arc::new(Mutex::new(ServerConfig::default())),
            reader: Arc::new(Mutex::new(Box::new(BufReader::new(reader)))),
            outbound,
            outbound_writer_handle: Some(outbound_writer_handle),
            client_capabilities: Mutex::new(ClientCapabilities::default()),
            cancelled: Arc::new(Mutex::new(HashSet::new())),
            workspace_folders: Arc::new(Mutex::new(Vec::new())),
            root_path: Arc::new(Mutex::new(None)),
            advertised_features: Mutex::new(default_features),
            client_supports_pull_diags: Arc::new(AtomicBool::new(false)),
            workspace_config: Arc::new(Mutex::new(WorkspaceConfig::default())),
            next_request_id: Arc::new(AtomicI64::new(1)),
            pending_workspace_configuration_requests: Arc::new(Mutex::new(HashMap::new())),
            progress_tokens: Arc::new(Mutex::new(HashSet::new())),
            progress_token_to_request: Arc::new(Mutex::new(HashMap::new())),
            refresh_controller: refresh::RefreshController::new(),
            diagnostic_debouncer: Mutex::new(None),
            file_watcher_debouncer: Mutex::new(None),
            notebook_store: notebook::NotebookStore::new(),
            trace_level: Arc::new(Mutex::new("off".to_string())),
            stream_session_manager: super::stream_session::StreamSessionManager::new(),
            feature_profile,
            pod_cache: Arc::new(Mutex::new(HashMap::new())),
            pending_index_task_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            parse_cancel_flags: Arc::new(Mutex::new(HashMap::new())),
            pull_diagnostics_orchestrator: super::diagnostics::PullDiagnosticsOrchestrator::new(),
            semantic_analyzer_cache: Arc::new(Mutex::new(HashMap::new())),
            #[cfg(feature = "workspace")]
            indexing_in_progress: Arc::new(AtomicBool::new(false)),
            #[cfg(feature = "workspace")]
            permission_denied_shown: Arc::new(AtomicBool::new(false)),
            #[cfg(not(target_arch = "wasm32"))]
            critic_analyzer: Mutex::new(None),
            #[cfg(not(target_arch = "wasm32"))]
            critic_runtime_override: Mutex::new(None),
            #[cfg(not(target_arch = "wasm32"))]
            skip_perlcritic_command_check: AtomicBool::new(false),
            #[cfg(not(target_arch = "wasm32"))]
            critic_workspace_warnings_sent: Mutex::new(HashSet::new()),
            ai_inline_backend: Mutex::new(None),
        }
    }

    /// Create a new LSP server with custom output (for testing)
    ///
    /// **Deprecated**: Use `with_io()` instead for full control over I/O.
    /// This method is maintained for backward compatibility.
    pub fn with_output(output: Arc<Mutex<Box<dyn Write + Send>>>) -> Self {
        Self::with_output_and_feature_profile(output, FeatureProfile::current())
    }

    /// Create a new LSP server with custom output and explicit feature profile.
    pub fn with_output_and_feature_profile(
        output: Arc<Mutex<Box<dyn Write + Send>>>,
        feature_profile: FeatureProfile,
    ) -> Self {
        // Initialize workspace indexing with coordinator lifecycle management
        #[cfg(feature = "workspace")]
        let index_coordinator = Some(Arc::new(IndexCoordinator::new()));

        let default_features = feature_profile.advertised_features();
        let (outbound, outbound_writer_handle) = outbound::spawn_writer_shared(output);

        Self {
            documents: Arc::new(Mutex::new(HashMap::new())),
            initialize_requested: AtomicBool::new(false),
            initialized: AtomicBool::new(false),
            shutdown_received: AtomicBool::new(false),
            #[cfg(feature = "workspace")]
            index_coordinator,
            ast_cache: Arc::new(AstCache::new(100, 300)),
            symbol_index: Arc::new(Mutex::new(SymbolIndex::new())),
            config: Arc::new(Mutex::new(ServerConfig::default())),
            reader: Arc::new(Mutex::new(Box::new(BufReader::new(io::stdin())))),
            outbound,
            outbound_writer_handle: Some(outbound_writer_handle),
            client_capabilities: Mutex::new(ClientCapabilities::default()),
            cancelled: Arc::new(Mutex::new(HashSet::new())),
            workspace_folders: Arc::new(Mutex::new(Vec::new())),
            root_path: Arc::new(Mutex::new(None)),
            advertised_features: Mutex::new(default_features),
            client_supports_pull_diags: Arc::new(AtomicBool::new(false)),
            workspace_config: Arc::new(Mutex::new(WorkspaceConfig::default())),
            next_request_id: Arc::new(AtomicI64::new(1)),
            pending_workspace_configuration_requests: Arc::new(Mutex::new(HashMap::new())),
            progress_tokens: Arc::new(Mutex::new(HashSet::new())),
            progress_token_to_request: Arc::new(Mutex::new(HashMap::new())),
            refresh_controller: refresh::RefreshController::new(),
            diagnostic_debouncer: Mutex::new(None),
            file_watcher_debouncer: Mutex::new(None),
            notebook_store: notebook::NotebookStore::new(),
            trace_level: Arc::new(Mutex::new("off".to_string())),
            stream_session_manager: super::stream_session::StreamSessionManager::new(),
            feature_profile,
            pod_cache: Arc::new(Mutex::new(HashMap::new())),
            pending_index_task_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            parse_cancel_flags: Arc::new(Mutex::new(HashMap::new())),
            pull_diagnostics_orchestrator: super::diagnostics::PullDiagnosticsOrchestrator::new(),
            semantic_analyzer_cache: Arc::new(Mutex::new(HashMap::new())),
            #[cfg(feature = "workspace")]
            indexing_in_progress: Arc::new(AtomicBool::new(false)),
            #[cfg(feature = "workspace")]
            permission_denied_shown: Arc::new(AtomicBool::new(false)),
            #[cfg(not(target_arch = "wasm32"))]
            critic_analyzer: Mutex::new(None),
            #[cfg(not(target_arch = "wasm32"))]
            critic_runtime_override: Mutex::new(None),
            #[cfg(not(target_arch = "wasm32"))]
            skip_perlcritic_command_check: AtomicBool::new(false),
            #[cfg(not(target_arch = "wasm32"))]
            critic_workspace_warnings_sent: Mutex::new(HashSet::new()),
            ai_inline_backend: Mutex::new(None),
        }
    }
}

impl Default for LspServer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for LspServer {
    fn drop(&mut self) {
        let outbound = std::mem::replace(&mut self.outbound, outbound::closed_sender());
        drop(outbound);

        if let Some(handle) = self.outbound_writer_handle.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::{self, Cursor};
    use std::sync::Arc;
    use std::thread;
    use std::time::{Duration, Instant};

    struct SlowVecWriter {
        inner: Arc<Mutex<Vec<u8>>>,
        pause: Duration,
    }

    impl Write for SlowVecWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            thread::sleep(self.pause);
            self.inner.lock().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn drop_waits_for_writer_flush_after_closing_sender() {
        let output = Arc::new(Mutex::new(Vec::new()));
        let writer = SlowVecWriter { inner: Arc::clone(&output), pause: Duration::from_millis(60) };
        let server = LspServer::with_io(Box::new(Cursor::new(Vec::<u8>::new())), Box::new(writer));

        server.notify("window/logMessage", json!({"type": 4, "message": "flush me"})).unwrap();

        let start = Instant::now();
        drop(server);

        assert!(
            start.elapsed() >= Duration::from_millis(40),
            "drop returned before the writer thread had time to flush"
        );

        let bytes = output.lock().clone();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("window/logMessage"));
        assert!(text.contains("flush me"));
    }
}
