//! Full JSON-RPC LSP Server implementation
//!
//! This module provides a complete Language Server Protocol implementation
//! that can be used with any LSP-compatible editor.

use crate::runtime::diagnostics::PullDiagnosticsOrchestrator;
use crate::runtime::types::{
    DocumentScanView, PendingWorkspaceConfigurationRequest, ServerRequestId,
    best_workspace_folder_for_doc, source_path_from_uri, workspace_folder_path,
};
use crate::runtime::workspace_folder::WorkspaceFolderState;

mod client_requests;
mod constructors;
pub(crate) mod diagnostic_debounce;
pub(crate) mod diagnostics;
mod dispatch;
mod document_access;
/// File discovery abstraction for workspace scanning
pub mod file_discovery;
/// File watcher change debouncer for bulk operation handling
pub mod file_watcher_debounce;
mod language;
mod lifecycle;
mod notebook;
pub(crate) mod outbound;
mod refresh;
/// Routing module for lifecycle-aware index access
pub mod routing;
pub(crate) mod scheduler;
mod serving;
pub(crate) mod stream_session;
mod symbol_extraction;
mod test_api;
mod test_runners;
mod text_sync;
mod types;
mod window;
mod workspace;
mod workspace_folder;
#[cfg(feature = "workspace")]
mod workspace_progress;

// Re-export protocol types for backward compatibility
// Tests and external code import these from perl_lsp::
pub use crate::protocol::{JsonRpcError, JsonRpcId, JsonRpcRequest, JsonRpcResponse};

// Re-export window types for public API
pub use window::{MessageType, ShowDocumentOptions};

use perl_lsp_rs_core::tooling::performance::{AstCache, SymbolIndex};
use perl_lsp_rs_core::tooling::perl_critic::BuiltInAnalyzer;
use perl_parser::{
    Parser,
    ast::{Node, NodeKind},
    declaration::ParentMap,
    position::LineStartsCache,
    tdd_basic::TestGenerator,
    test_runner::{TestKind, TestRunner},
};

use crate::call_hierarchy_provider::CallHierarchyProvider;
use crate::cancellation::{GLOBAL_CANCELLATION_REGISTRY, PerlLspCancellationToken};
// Wave G3 (#4535): perl-lsp-feature-governance absorbed into perl-lsp-rs-core::governance
use perl_lsp_rs_core::governance::FeatureProfile;
use perl_lsp_rs_core::runtime::tuning::RuntimeTuning;

// Import LSP providers from features (these moved from perl-parser to perl-lsp)
use crate::features::{
    // code_actions.rs - original AST-based provider
    code_actions::{CodeActionKind as InternalCodeActionKind, CodeActionsProvider},
    code_actions_enhanced::EnhancedCodeActionsProvider,
    // code_actions_provider.rs - V2 diagnostic-based provider
    code_actions_provider::{
        CodeActionKind as InternalCodeActionKindV2, CodeActionsProvider as CodeActionsProviderV2,
    },
    code_lens_provider::{CodeLensProvider, get_shebang_lens, resolve_code_lens},
    diagnostics::{DiagnosticSeverity as InternalDiagnosticSeverity, DiagnosticsProvider},
    document_highlight::DocumentHighlightProvider,
    formatting::{CodeFormatter, FormattingOptions},
    implementation_provider::ImplementationProvider,
    semantic_tokens_provider::{SemanticTokensProvider, encode_semantic_tokens},
    type_hierarchy::TypeHierarchyProvider,
};

use crate::{
    // Import fallback implementations
    fallback::text::extract_text_based_code_lenses,
    // Import from new modular lsp structure
    // Note: JsonRpcError, JsonRpcRequest, JsonRpcResponse are pub use'd above
    protocol::{
        CONTENT_MODIFIED, INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, REQUEST_CANCELLED,
        cancelled_response_with_method, document_not_found_error, enhanced_error,
    },
    state::{
        ClientCapabilities, DocumentState, ServerConfig, WorkspaceConfig,
        normalize_package_separator,
    },
    transport::{ContentLengthMessageReader, log_response},
    // Import text processing helpers
    util::{
        byte_to_line_col, byte_to_utf16_col, extract_module_reference,
        extract_module_reference_extended, get_text_around_offset, get_text_window_around_offset,
        offset_to_position, position_to_offset,
    },
};
use md5;
use parking_lot::Mutex;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering},
};
use url::Url;

#[cfg(feature = "workspace")]
use perl_parser::workspace_index::{
    IndexCoordinator, LspWorkspaceSymbol, WorkspaceIndex, uri_to_fs_path,
};
#[cfg(feature = "workspace")]
use perl_position_tracking::{WireLocation, WirePosition, WireRange};

#[cfg(feature = "workspace")]
use crate::fallback::text::extract_text_based_symbols;

// Note: FQN_RE regex moved to language/navigation.rs

// Note: Error codes and cancelled_response imported from crate::lsp::protocol

// Note: ClientCapabilities imported from crate::lsp::state::document

/// LSP server that handles JSON-RPC communication
pub struct LspServer {
    /// Document contents indexed by URI
    pub(crate) documents: Arc<Mutex<HashMap<String, DocumentState>>>,
    /// Whether the `initialize` request has been received
    initialize_requested: AtomicBool,
    /// Whether the server is initialized
    initialized: AtomicBool,
    /// Whether shutdown was received (for LSP-compliant exit handling)
    shutdown_received: AtomicBool,
    /// Index coordinator for workspace-wide features with lifecycle management
    #[cfg(feature = "workspace")]
    pub(crate) index_coordinator: Option<Arc<IndexCoordinator>>,
    /// AST cache for performance
    ast_cache: Arc<AstCache>,
    /// Symbol index for fast lookups
    symbol_index: Arc<Mutex<SymbolIndex>>,
    /// Server configuration
    pub(crate) config: Arc<Mutex<ServerConfig>>,
    /// Synchronized input reader
    reader: Arc<Mutex<Box<dyn BufRead + Send>>>,
    /// Outbound message sender (channel-based, decoupled from I/O).
    outbound: outbound::OutboundSender,
    /// Join handle for the outbound writer thread.
    ///
    /// `Drop` swaps `outbound` with a closed sender, drops the live sender to
    /// close the channel, then joins this thread so buffered bytes are flushed
    /// before the server is deallocated.
    outbound_writer_handle: Option<std::thread::JoinHandle<()>>,
    /// Client capabilities (behind mutex for interior mutability — written once during initialize)
    client_capabilities: Mutex<ClientCapabilities>,
    /// Cancelled request IDs
    cancelled: Arc<Mutex<HashSet<JsonRpcId>>>,
    /// Workspace folders with full state representation
    ///
    /// This replaces the previous `Vec<String>` approach to support multi-root
    /// workspaces with per-folder configuration. The old string-based approach
    /// is maintained via `workspace_folder_uris()` for backward compatibility.
    workspace_folders: Arc<Mutex<Vec<WorkspaceFolderState>>>,
    /// Root path for module resolution
    root_path: Arc<Mutex<Option<PathBuf>>>,
    /// Advertised server capabilities
    advertised_features: Mutex<crate::protocol::capabilities::AdvertisedFeatures>,
    /// Client supports pull diagnostics
    client_supports_pull_diags: Arc<AtomicBool>,
    /// Workspace configuration for module resolution
    workspace_config: Arc<Mutex<WorkspaceConfig>>,
    /// Atomic counter for generating unique request IDs
    next_request_id: Arc<AtomicI32>,
    /// Pending workspace/configuration reverse requests keyed by request ID.
    pending_workspace_configuration_requests:
        Arc<Mutex<HashMap<ServerRequestId, PendingWorkspaceConfigurationRequest>>>,
    /// Active progress tokens for work done progress tracking
    progress_tokens: Arc<Mutex<HashSet<String>>>,
    /// Maps progress tokens to their originating request IDs for cancellation routing
    progress_token_to_request: Arc<Mutex<HashMap<String, JsonRpcId>>>,
    /// Refresh controller for debounced client refresh requests
    refresh_controller: refresh::RefreshController,
    /// Diagnostic publication debouncer (installed after Arc wrapping in Scheduler::new)
    diagnostic_debouncer: Mutex<Option<diagnostic_debounce::DiagnosticDebouncer>>,
    /// File watcher change debouncer (installed after Arc wrapping in Scheduler::new)
    file_watcher_debouncer: Mutex<Option<file_watcher_debounce::FileWatcherDebouncer>>,
    /// Notebook document store (LSP 3.17)
    pub(crate) notebook_store: notebook::NotebookStore,
    /// Trace level set by client via $/setTrace (off, messages, verbose)
    trace_level: Arc<Mutex<String>>,
    /// Stream session manager for progressive inline completion.
    stream_session_manager: stream_session::StreamSessionManager,
    /// Runtime feature profile selected by launch arguments or compiled default.
    feature_profile: FeatureProfile,
    /// Runtime workload tuning (e2e mode, diagnostic scope, debounce, indexing gates).
    ///
    /// Resolved at construction by layering env vars and CLI args on top of the
    /// compiled defaults. The server treats this as read-only after construction.
    pub(crate) runtime_tuning: RuntimeTuning,
    /// Monotonic counter bumped every time `start_workspace_indexing` is
    /// called (before any internal guards). Lets tests observe whether
    /// the `initialized` gate fired, without needing to set up a real
    /// workspace on disk.
    pub(crate) workspace_indexing_invocation_count: Arc<std::sync::atomic::AtomicUsize>,
    /// Cache of extracted POD documentation keyed by resolved file path.
    pod_cache: Arc<Mutex<HashMap<PathBuf, perl_pod::PodDoc>>>,
    /// Cache of SemanticAnalyzer results keyed by (normalized_uri, content_hash).
    ///
    /// Avoids re-running the full O(n) AST traversal on repeated hover/definition
    /// requests to the same document version. Content hash provides automatic
    /// invalidation when source text changes — no TTL needed.
    pub(crate) semantic_analyzer_cache:
        Arc<Mutex<HashMap<(String, u64), Arc<crate::semantic::SemanticAnalyzer>>>>,
    /// Last provider-local decision receipt by provider name.
    ///
    /// `perl.explainProviderDecision` can attach these transient per-server
    /// receipts when the caller does not provide a request-local receipt.
    pub(crate) provider_decision_traces: Arc<Mutex<HashMap<String, Value>>>,
    /// Short-TTL cache for module prefix directory scans (issue #8514).
    ///
    /// Typing a multi-segment `use` prefix (e.g. `use Mojo::Cont|`) triggers a
    /// filesystem scan on every keystroke. This cache avoids repeated scans of the
    /// same subdirectory within the 1-second interactive typing burst window.
    ///
    /// Cache is runtime-owned (per-server instance) because `CompletionProvider`
    /// is reconstructed per request and cannot hold persistent state.
    pub(crate) module_scan_cache:
        Arc<perl_lsp_rs_core::providers::completion::module_scan_cache::ModuleCompletionScanCache>,
    /// Count of background workspace indexing tasks currently in flight.
    ///
    /// Incremented before spawning a background `index_file` task, decremented
    /// when it completes.  Used by tests to observe that indexing was detached
    /// from the synchronous handler (issue #2352).
    pub(crate) pending_index_task_count: Arc<std::sync::atomic::AtomicUsize>,
    /// Per-document cancellation flags for stale-parse cancellation.
    ///
    /// When `didChange` #2 arrives while `didChange` #1 is still parsing,
    /// setting the old flag to `true` interrupts the in-progress parse
    /// cooperatively (via `Parser::check_cancelled`).
    pub(crate) parse_cancel_flags: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
    /// Pull diagnostics orchestrator for coordinating diagnostic operations.
    pub(crate) pull_diagnostics_orchestrator: PullDiagnosticsOrchestrator,
    /// Guard that prevents concurrent workspace indexing scans.
    ///
    /// Set to `true` when `start_workspace_indexing` spawns a background thread,
    /// cleared to `false` when that thread completes (via RAII drop guard in all
    /// exit paths including panics).
    #[cfg(feature = "workspace")]
    indexing_in_progress: Arc<AtomicBool>,
    /// One-time guard for the `window/showMessage` permission-denied warning.
    ///
    /// Set to `true` after the first permission-denied file is encountered during
    /// workspace indexing so the user is not spammed when multiple files are
    /// unreadable.  The per-file `textDocument/publishDiagnostics` is NOT gated
    /// by this flag — it repeats for every affected file.
    #[cfg(feature = "workspace")]
    permission_denied_shown: Arc<AtomicBool>,
    /// One-time guard for the `window/showMessage` workspace-root-undetected warning.
    ///
    /// Set to `true` after the first module resolution attempt when no workspace
    /// root is configured, so the user is warned once per server session rather
    /// than on every resolution call.  Uses an instance-level flag (not a
    /// process-level `Once`) so that each `LspServer` instance tracks its own
    /// session independently.
    pub(crate) root_undetected_shown: Arc<AtomicBool>,
    /// Shared Perl::Critic analyzer for the diagnostic pipeline.
    ///
    /// Lazily initialized on first use and reused across diagnostic cycles so
    /// the per-instance violation cache survives between `textDocument/didChange`
    /// events.  `invalidate_cache` is called on `didChange`; the whole entry is
    /// reset to `None` when `perlcritic_enabled`, `perlcritic_severity`, or
    /// `perlcritic_profile` changes via `didChangeConfiguration`.
    ///
    /// Only present on non-WASM targets (subprocess execution is unavailable
    /// on WASM).
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) critic_analyzer: Mutex<Option<crate::perl_critic::CriticAnalyzer>>,
    /// Subprocess runtime override for the `CriticAnalyzer`.
    ///
    /// When `Some`, the lazy-init path in `collect_external_perlcritic_diagnostics`
    /// uses this runtime instead of `OsSubprocessRuntime`.  Always `None` in
    /// production; set to a `MockSubprocessRuntime` by the test helper
    /// `LspServer::test_install_mock_critic_runtime` so that tests can exercise
    /// the full diagnostic pipeline without spawning a real `perlcritic` process.
    ///
    /// Using a separate runtime override (rather than pre-building the analyzer)
    /// ensures that config-sensitive values such as the auto-discovered
    /// `.perlcriticrc` profile path are still resolved at analysis time.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) critic_runtime_override:
        Mutex<Option<std::sync::Arc<dyn perl_subprocess_runtime::SubprocessRuntime>>>,
    /// When `true`, skip the `command_exists("perlcritic")` guard during
    /// diagnostic collection.  Always present on non-WASM targets but only
    /// settable to `true` through the test API exposed via
    /// `#[cfg(any(test, feature = "expose_lsp_test_api"))]`.
    ///
    /// Initialized to `false`; only the test helper methods flip this.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) skip_perlcritic_command_check: AtomicBool,
    /// Deduplication set for workspace-scoped Perl::Critic warning notifications.
    ///
    /// Keys are stable identifiers (for example, `missing-binary` or
    /// `missing-profile:/abs/path`) so repeated diagnostic cycles do not spam
    /// users with identical `window/showMessage` warnings.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) critic_workspace_warnings_sent: Mutex<std::collections::HashSet<String>>,
    /// Optional AI inline-completion backend.
    ///
    /// When `Some`, the `handle_inline_completion` handler will attempt
    /// AI-backed completions before falling back to deterministic rules.
    /// Set to `None` by default; a backend can be registered later.
    pub(crate) ai_inline_backend: Mutex<
        Option<Arc<dyn perl_lsp_rs_core::providers::inline_completion::InlineCompletionBackend>>,
    >,
}

#[cfg(any(test, feature = "expose_lsp_test_api"))]
/// Point-in-time counts for per-document memory pressure gauges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryStateSnapshot {
    /// Number of open documents in the LSP document store.
    pub documents: usize,
    /// Total bytes of source text held by open document state.
    pub open_text_bytes: usize,
    /// Number of cached semantic analyzer entries.
    pub semantic_analyzer_cache: usize,
    /// Number of per-document parse cancellation flags still retained.
    pub parse_cancel_flags: usize,
    /// Number of active inline-completion stream sessions.
    pub stream_sessions: usize,
    /// Number of background index tasks currently in flight.
    pub pending_index_tasks: usize,
    /// Number of cached POD documents.
    pub pod_cache_entries: usize,
}

#[cfg(any(test, feature = "expose_lsp_test_api"))]
/// Point-in-time counts for async runtime pressure gauges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePressureSnapshot {
    /// Number of background index tasks currently in flight.
    pub pending_index_tasks: usize,
    /// Number of unique file-watcher URIs waiting in the debounce window.
    pub file_watcher_pending_uris: usize,
    /// Number of unique diagnostic URIs waiting in the debounce window.
    pub diagnostic_debounce_pending_uris: usize,
    /// Number of workspace/configuration requests waiting for client replies.
    pub pending_workspace_configuration_requests: usize,
    /// Number of refresh timers currently inside their debounce window.
    pub refresh_debounce_active: usize,
    /// Number of active inline-completion stream sessions.
    pub active_stream_sessions: usize,
}

// SAFETY: LspServer is not auto-Send/Sync because DocumentState contains
// ParentMap which has `*const Node` raw pointers. However, these pointers
// are only accessed through the `documents: Arc<Mutex<...>>` field, which
// provides proper synchronization. All other fields are either atomic,
// behind Mutex/Arc, or inherently Send+Sync.
#[allow(unsafe_code)]
unsafe impl Send for LspServer {}
#[allow(unsafe_code)]
unsafe impl Sync for LspServer {}

// Note: DocumentState, ServerConfig, and normalize_package_separator are
// imported from crate::lsp::state::{document, config}

// =========================================================================
// Core accessors and server lifecycle
// =========================================================================

#[allow(dead_code)]
impl LspServer {
    fn resolve_ai_api_key(
        ai_config: &perl_lsp_rs_core::config::AiCompletionConfig,
    ) -> Option<String> {
        Self::resolve_ai_api_key_with(ai_config, |name| {
            std::env::var(name).ok().filter(|value| !value.is_empty())
        })
    }

    fn resolve_ai_api_key_with<F>(
        ai_config: &perl_lsp_rs_core::config::AiCompletionConfig,
        mut read_env: F,
    ) -> Option<String>
    where
        F: FnMut(&str) -> Option<String>,
    {
        let configured = read_env(&ai_config.api_key_env);
        if configured.is_some() {
            return configured;
        }

        // Compatibility fallback: many OpenAI-compatible clients (including Gemini CLI setups)
        // export provider-specific key names instead of OPENAI_API_KEY.
        const FALLBACK_API_KEY_ENVS: [&str; 2] = ["GEMINI_API_KEY", "GOOGLE_API_KEY"];
        FALLBACK_API_KEY_ENVS.iter().find_map(|name| read_env(name))
    }

    /// Active feature profile for this server instance.
    pub(crate) const fn feature_profile(&self) -> FeatureProfile {
        self.feature_profile
    }

    /// Active runtime tuning for this server instance.
    pub const fn runtime_tuning(&self) -> RuntimeTuning {
        self.runtime_tuning
    }

    /// Whether `initialized` should trigger an eager workspace-wide
    /// indexing scan. The default for normal editor sessions is `true`;
    /// e2e harness mode defaults to `false` so latency tests do not pay
    /// for indexing they will not consult. The user can override either
    /// way via `--eager-workspace-indexing` /
    /// `PERL_LSP_EAGER_WORKSPACE_INDEXING`.
    pub fn should_start_workspace_indexing(&self) -> bool {
        self.runtime_tuning.eager_workspace_indexing
    }

    /// Count of times `start_workspace_indexing` was invoked. Used by
    /// tests to assert the e2e startup gate fires (or doesn't). Never
    /// reset; monotonic for the lifetime of the server.
    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    pub fn workspace_indexing_invocation_count(&self) -> usize {
        self.workspace_indexing_invocation_count.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Get the registered AI inline-completion backend, if any.
    ///
    /// Returns `None` when no backend has been registered (the default).
    /// The returned `Arc` is a cheap clone suitable for use outside the lock.
    pub(crate) fn ai_backend(
        &self,
    ) -> Option<Arc<dyn perl_lsp_rs_core::providers::inline_completion::InlineCompletionBackend>>
    {
        self.ai_inline_backend.lock().clone()
    }

    /// Runtime feature gate for future next-edit suggestions.
    ///
    /// This boundary is intentionally default-off. Even when explicit config
    /// enables the gate, the current runtime still reports that no editor-visible
    /// next-edit provider is registered.
    pub(crate) fn next_edit_feature_gate(
        &self,
    ) -> perl_lsp_rs_core::providers::inline_completion::NextEditFeatureGate {
        if self.config.lock().next_edit.enabled {
            perl_lsp_rs_core::providers::inline_completion::NextEditFeatureGate::explicit_enabled()
        } else {
            perl_lsp_rs_core::providers::inline_completion::NextEditFeatureGate::default()
        }
    }

    /// Evaluate the next-edit scaffold against runtime configuration.
    ///
    /// The returned response is a boundary proof only; it never produces
    /// editor-visible suggestions until a future provider is deliberately wired.
    pub(crate) fn next_edit_scaffold_response(
        &self,
        context: perl_lsp_rs_core::providers::inline_completion::PreparedInlineCompletionContext,
    ) -> perl_lsp_rs_core::providers::inline_completion::NextEditResponse {
        let mut request =
            perl_lsp_rs_core::providers::inline_completion::NextEditRequest::receipt_only(context);
        request.gate = self.next_edit_feature_gate();
        perl_lsp_rs_core::providers::inline_completion::NextEditProvider.suggest(&request)
    }

    /// Refresh the AI inline-completion backend based on current configuration.
    ///
    /// When `ai_completion.enabled` is `true` and the API key environment variable
    /// resolves to a non-empty string, constructs an `OpenAiProvider` and stores it.
    /// Otherwise clears the backend to `None`, disabling AI completions.
    ///
    /// Called during initialization (after project config is loaded) and on every
    /// `didChangeConfiguration` notification that touches the `aiCompletion` section.
    pub(crate) fn refresh_ai_backend(&self) {
        let ai_config = self.config.lock().ai_completion.clone();

        if !ai_config.enabled {
            *self.ai_inline_backend.lock() = None;
            return;
        }

        // Resolve API key from configured env var with compatibility aliases for
        // common OpenAI-compatible providers.
        let Some(api_key) = Self::resolve_ai_api_key(&ai_config) else {
            tracing::warn!(env_var = %ai_config.api_key_env, "AI completion enabled but API key env var is empty or unset");
            *self.ai_inline_backend.lock() = None;
            return;
        };

        let mut provider_config = perl_lsp_rs_core::providers::ai::OpenAiConfig::new(
            ai_config.endpoint.clone(),
            ai_config.model.clone(),
            api_key,
            ai_config.timeout_ms,
        );
        provider_config.api_key_header = ai_config.api_key_header.clone();
        provider_config.api_key_prefix = ai_config.api_key_prefix.clone();

        let limiter = Arc::new(perl_lsp_rs_core::providers::ai::RateLimiter::new(
            ai_config.rate_limit_rps,
            ai_config.max_inflight,
        ));

        let provider =
            perl_lsp_rs_core::providers::ai::OpenAiProvider::new(provider_config, limiter);
        *self.ai_inline_backend.lock() = Some(Arc::new(provider));

        tracing::info!(endpoint = %ai_config.endpoint, model = %ai_config.model, "AI inline completion backend configured");
    }

    /// Get the subprocess runtime for external tool execution (perltidy, perlcritic).
    ///
    /// Returns a new `OsSubprocessRuntime` for executing external processes.
    /// This is used by formatting and linting providers.
    pub fn subprocess_runtime(&self) -> perl_lsp_rs_core::tooling::OsSubprocessRuntime {
        perl_lsp_rs_core::tooling::OsSubprocessRuntime::new()
    }

    /// Cancel any in-progress parse for `uri` and return a fresh token.
    ///
    /// Sets the previous flag to `true` (interrupting the in-flight parse),
    /// inserts a new `false` flag, and returns an `Arc` clone of the new flag
    /// for the caller to pass to `Parser::new_with_cancellation`.
    pub(crate) fn new_parse_token(&self, uri: &str) -> Arc<AtomicBool> {
        let mut flags = self.parse_cancel_flags.lock();
        if let Some(old) = flags.get(uri) {
            old.store(true, Ordering::Release);
        }
        let new_flag = Arc::new(AtomicBool::new(false));
        flags.insert(uri.to_string(), Arc::clone(&new_flag));
        new_flag
    }

    /// Access the stream session manager for progressive inline completion.
    pub(crate) fn stream_sessions(&self) -> &stream_session::StreamSessionManager {
        &self.stream_session_manager
    }

    pub(crate) fn uri_key_variants(&self, uri: &str) -> Vec<String> {
        fn push_unique(keys: &mut Vec<String>, key: String) {
            if !keys.iter().any(|existing| existing == &key) {
                keys.push(key);
            }
        }

        fn push_windows_drive_case_variant(keys: &mut Vec<String>, key: &str) {
            for prefix in ["file:///", "file://localhost/"] {
                let prefix_len = prefix.len();
                let bytes = key.as_bytes();
                if bytes.len() <= prefix_len + 2
                    || !key.starts_with(prefix)
                    || bytes[prefix_len + 1] != b':'
                    || bytes[prefix_len + 2] != b'/'
                    || !bytes[prefix_len].is_ascii_alphabetic()
                {
                    continue;
                }

                let current = char::from(bytes[prefix_len]);
                let toggled = if current.is_ascii_lowercase() {
                    current.to_ascii_uppercase()
                } else {
                    current.to_ascii_lowercase()
                };
                let mut variant = key.to_string();
                variant.replace_range(prefix_len..=prefix_len, &toggled.to_string());
                push_unique(keys, variant);
            }
        }

        let mut uri_keys = Vec::new();
        push_unique(&mut uri_keys, uri.to_string());
        push_unique(&mut uri_keys, self.normalize_uri_key(uri));

        if let Some(path) = source_path_from_uri(uri) {
            if let Ok(file_url) = url::Url::from_file_path(&path) {
                let file_uri = file_url.to_string();
                push_unique(&mut uri_keys, file_uri.clone());
                push_unique(&mut uri_keys, self.normalize_uri_key(&file_uri));
            }
        }

        for key in uri_keys.clone() {
            push_windows_drive_case_variant(&mut uri_keys, &key);
        }

        uri_keys
    }

    /// Evict open-document session state for a URI without deleting workspace
    /// index entries for the file on disk.
    ///
    /// `textDocument/didClose` means the editor closed its buffer; it does not
    /// mean the source file was deleted from the workspace. Sweep both raw and
    /// normalized keys so URI spelling differences do not retain per-document
    /// caches after close.
    pub(crate) fn evict_open_document_session_state(&self, uri: &str) {
        let uri_keys = self.uri_key_variants(uri);

        for key in &uri_keys {
            self.stream_sessions().cancel_for_uri(key);
            self.ast_cache.remove(key);
            self.clear_document_symbols(key);
        }

        {
            let mut cache = self.semantic_analyzer_cache.lock();
            cache.retain(|(cached_uri, _), _| !uri_keys.iter().any(|key| key == cached_uri));
        }

        {
            let mut documents = self.documents.lock();
            for key in &uri_keys {
                if let Some(doc) = documents.remove(key) {
                    doc.generation.store(u32::MAX, Ordering::Release);
                }
            }
        }

        {
            let mut flags = self.parse_cancel_flags.lock();
            for key in &uri_keys {
                if let Some(flag) = flags.remove(key) {
                    flag.store(true, Ordering::Release);
                }
            }
        }

        for key in &uri_keys {
            if let Some(path) = source_path_from_uri(key) {
                self.pod_cache.lock().remove(&path);

                #[cfg(not(target_arch = "wasm32"))]
                self.pull_diagnostics_orchestrator.invalidate_file_cache(&path);
            }
        }
    }

    /// Evict all state for a file that no longer exists in the workspace.
    pub(crate) fn evict_deleted_file_state(&self, uri: &str) {
        let uri_keys = self.uri_key_variants(uri);
        #[cfg(feature = "workspace")]
        if let Some(coordinator) = self.coordinator() {
            for key in &uri_keys {
                coordinator.index().remove_file(key);
            }
        }

        self.evict_open_document_session_state(uri);
    }

    /// Evict open-document state and workspace index state for a removed folder.
    pub(crate) fn evict_workspace_folder_state(&self, folder_uri: &str) {
        let folder_keys = self.uri_key_variants(folder_uri);
        let docs_to_evict = {
            let documents = self.documents.lock();
            documents
                .keys()
                .filter(|doc_uri| {
                    folder_keys.iter().any(|folder_key| doc_uri.starts_with(folder_key))
                })
                .cloned()
                .collect::<Vec<_>>()
        };

        for doc_uri in docs_to_evict {
            tracing::debug!(uri = %doc_uri, "Evicting document from removed workspace");
            self.evict_open_document_session_state(&doc_uri);
        }

        #[cfg(feature = "workspace")]
        if let Some(coordinator) = self.coordinator() {
            for folder_key in &folder_keys {
                coordinator.index().remove_folder(folder_key);
            }
        }
    }

    /// Capture test/debug counters for retained per-document state.
    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    pub fn memory_state_snapshot(&self) -> MemoryStateSnapshot {
        let documents = self.documents.lock();
        let open_text_bytes = documents.values().map(|doc| doc.text.len()).sum();
        let document_count = documents.len();
        drop(documents);

        MemoryStateSnapshot {
            documents: document_count,
            open_text_bytes,
            semantic_analyzer_cache: self.semantic_analyzer_cache.lock().len(),
            parse_cancel_flags: self.parse_cancel_flags.lock().len(),
            stream_sessions: self.stream_sessions().len(),
            pending_index_tasks: self.pending_index_task_count.load(Ordering::SeqCst),
            pod_cache_entries: self.pod_cache.lock().len(),
        }
    }

    /// Capture test/debug counters for async task and debounce pressure.
    #[cfg(any(test, feature = "expose_lsp_test_api"))]
    pub fn runtime_pressure_snapshot(&self) -> RuntimePressureSnapshot {
        let diagnostic_debounce_pending_uris = self
            .diagnostic_debouncer
            .lock()
            .as_ref()
            .map_or(0, diagnostic_debounce::DiagnosticDebouncer::pending_uris);
        let file_watcher_pending_uris = self
            .file_watcher_debouncer
            .lock()
            .as_ref()
            .map_or(0, file_watcher_debounce::FileWatcherDebouncer::pending_uris);

        RuntimePressureSnapshot {
            pending_index_tasks: self.pending_index_task_count.load(Ordering::SeqCst),
            file_watcher_pending_uris,
            diagnostic_debounce_pending_uris,
            pending_workspace_configuration_requests: self
                .pending_workspace_configuration_requests
                .lock()
                .len(),
            refresh_debounce_active: self.refresh_controller.debounce_active_count(),
            active_stream_sessions: self.stream_sessions().len(),
        }
    }

    // =========================================================================
    // Workspace folder helpers
    // =========================================================================

    /// Find the workspace folder containing a document URI.
    ///
    /// Returns the most-specific (deepest) workspace folder whose URI is a
    /// prefix of the document URI. Returns `None` if no workspace folder
    /// contains the document. See `best_workspace_folder_for_doc` for the
    /// rationale for preferring deepest over first-match.
    #[must_use]
    pub fn folder_for_doc_uri(&self, doc_uri: &str) -> Option<WorkspaceFolderState> {
        let folders = self.workspace_folders.lock();
        best_workspace_folder_for_doc(&folders, doc_uri).cloned()
    }

    /// Get the effective workspace config for a document's folder.
    ///
    /// Returns the effective workspace configuration for the most-specific
    /// (deepest) folder containing the document, or `None` if the document is
    /// not in any workspace folder.
    #[must_use]
    pub fn config_for_doc(
        &self,
        doc_uri: &str,
    ) -> Option<perl_lsp_rs_core::config::WorkspaceConfig> {
        let folders = self.workspace_folders.lock();
        best_workspace_folder_for_doc(&folders, doc_uri)
            .map(|folder| folder.effective_workspace_config.clone())
    }

    /// Get all include paths for a document (from its folder and others).
    ///
    /// Returns a vector of resolved absolute include paths from all workspace folders,
    /// with the current folder's paths first. Merges `PERL5LIB` entries according to
    /// each folder's `use_perl5lib` / `perl5lib_precedence` settings so that module
    /// resolution and diagnostics agree on which paths are searchable.
    #[must_use]
    pub fn include_paths_for_doc(&self, doc_uri: &str) -> Vec<std::path::PathBuf> {
        let perl5lib_paths = std::env::var("PERL5LIB")
            .map(|v| perl_lsp_rs_core::config::WorkspaceConfig::parse_perl5lib(&v))
            .unwrap_or_default();

        let mut paths = Vec::new();
        let folders = self.workspace_folders.lock();

        // Resolve one effective include path string to an absolute PathBuf.
        let resolve_one = |include_path: &str, folder: &WorkspaceFolderState| -> PathBuf {
            if std::path::Path::new(include_path).is_absolute() {
                PathBuf::from(include_path)
            } else if let Some(folder_path) = workspace_folder_path(folder) {
                folder_path.join(include_path)
            } else {
                PathBuf::from(include_path)
            }
        };

        // Add the most-specific (deepest) matching folder's paths first
        // (effective_include_paths merges PERL5LIB).
        let best = best_workspace_folder_for_doc(&folders, doc_uri);
        if let Some(current_folder) = best {
            let effective =
                current_folder.effective_workspace_config.effective_include_paths(&perl5lib_paths);
            for include_path in &effective {
                let resolved = resolve_one(include_path, current_folder);
                if !paths.contains(&resolved) {
                    paths.push(resolved);
                }
            }
        }

        // Add other folders' include paths. Skip only the best folder so that
        // outer folders in a nested workspace contribute as fallback roots.
        for folder in folders.iter() {
            if best.is_some_and(|b| b.uri == folder.uri) {
                continue;
            }
            let effective =
                folder.effective_workspace_config.effective_include_paths(&perl5lib_paths);
            for include_path in &effective {
                let resolved = resolve_one(include_path, folder);
                if !paths.contains(&resolved) {
                    paths.push(resolved);
                }
            }
        }

        paths
    }

    /// Get ordered search scopes for a document (current folder first, then others).
    ///
    /// Returns a vector of workspace folders ordered by relevance:
    /// 1. The folder containing the document (if any)
    /// 2. All other workspace folders
    ///
    /// This ordering is useful for module resolution and symbol search operations
    /// where the current folder should take precedence.
    #[must_use]
    pub fn search_scopes_for_doc(&self, doc_uri: &str) -> Vec<WorkspaceFolderState> {
        let folders = self.workspace_folders.lock();
        if let Some(current_folder) = best_workspace_folder_for_doc(&folders, doc_uri) {
            let mut scopes = vec![current_folder.clone()];
            for folder in folders.iter() {
                if folder.uri != current_folder.uri {
                    scopes.push(folder.clone());
                }
            }
            scopes
        } else {
            folders.iter().cloned().collect()
        }
    }

    /// Build resolution context for a document.
    ///
    /// Creates a unified resolution context with ordered search scopes:
    /// 1. Current document's workspace folder (first)
    /// 2. Other workspace folders, in registration order
    ///
    /// If no document URI is provided, uses all folders in registration order.
    #[must_use]
    pub fn build_resolution_context(
        &self,
        doc_uri: Option<&str>,
    ) -> crate::runtime::lifecycle::module_resolution::ResolutionContext {
        use crate::runtime::lifecycle::module_resolution::{ResolutionContext, ResolutionScope};

        let mut search_scopes = Vec::new();

        if let Some(uri) = doc_uri {
            // Get ordered search scopes for this document
            let folder_scopes = self.search_scopes_for_doc(uri);

            for folder in folder_scopes {
                let scope = ResolutionScope {
                    folder_uri: folder.uri.clone(),
                    include_paths: folder.effective_workspace_config.include_paths.clone(),
                    use_system_inc: folder.effective_workspace_config.use_system_inc,
                };
                search_scopes.push(scope);
            }
        } else {
            // No document context - use all folders in registration order
            let folders = self.workspace_folders.lock();
            for folder in folders.iter() {
                let scope = ResolutionScope {
                    folder_uri: folder.uri.clone(),
                    include_paths: folder.effective_workspace_config.include_paths.clone(),
                    use_system_inc: folder.effective_workspace_config.use_system_inc,
                };
                search_scopes.push(scope);
            }
        }

        ResolutionContext { doc_uri: doc_uri.map(|u| u.to_string()), search_scopes }
    }

    /// Get all workspace folder URIs (for backward compatibility).
    ///
    /// This method provides compatibility with code that expects a simple list
    /// of URI strings rather than the full `WorkspaceFolderState` objects.
    #[must_use]
    pub fn workspace_folder_uris(&self) -> Vec<String> {
        self.workspace_folders.lock().iter().map(|f| f.uri.clone()).collect()
    }

    /// Get all workspace folders as a cloned vector.
    ///
    /// This is useful for operations that need to work with all folders
    /// without holding the lock for an extended period.
    #[must_use]
    pub fn all_workspace_folders(&self) -> Vec<WorkspaceFolderState> {
        self.workspace_folders.lock().clone()
    }

    /// Get the number of workspace folders.
    #[must_use]
    pub fn workspace_folder_count(&self) -> usize {
        self.workspace_folders.lock().len()
    }

    /// Send a notification to the client via the outbound channel
    fn notify(&self, method: &str, params: Value) -> io::Result<()> {
        self.outbound.send_notification(method, params)
    }

    /// Acquire a lock on the documents map
    ///
    /// This helper centralizes lock acquisition behavior. parking_lot locks
    /// cannot be poisoned, so this always succeeds (or blocks until available).
    #[inline]
    pub(crate) fn documents_guard(
        &self,
    ) -> parking_lot::MutexGuard<'_, HashMap<String, DocumentState>> {
        self.documents.lock()
    }

    /// Create a lightweight snapshot of all document URIs and text content
    ///
    /// This method minimizes lock hold time by copying only the URI and text
    /// fields needed for scan-heavy operations (regex searches, text-based
    /// fallbacks). The lock is released immediately after the snapshot is
    /// created, allowing other operations to proceed while scanning.
    ///
    /// ## Performance Characteristics
    /// - Lock hold time: O(n) where n is the number of documents (just cloning strings)
    /// - Memory usage: ~1x total text size (only text is cloned, not AST/rope)
    /// - Use case: Text-based reference searches, regex scans across workspace
    #[inline]
    pub(crate) fn documents_text_snapshot(&self) -> Vec<(String, String)> {
        let docs = self.documents_guard();
        docs.iter().map(|(k, v)| (k.clone(), v.text.clone())).collect()
    }

    /// Create a snapshot for scan operations that may need AST access
    ///
    /// This method provides a more comprehensive snapshot that includes the
    /// AST reference (as Arc clone) in addition to URI and text. This allows
    /// scan-heavy operations to work with both text and AST without holding
    /// the documents lock during CPU-intensive work.
    ///
    /// ## Performance Characteristics
    /// - Lock hold time: O(n) where n is the number of documents
    /// - Memory usage: ~1x text size + Arc refs (AST is shared, not cloned)
    /// - Use case: Code lens resolve, reference counting across workspace
    #[inline]
    pub(crate) fn documents_scan_snapshot(&self) -> Vec<DocumentScanView> {
        let docs = self.documents_guard();
        docs.iter()
            .map(|(k, v)| DocumentScanView {
                uri: k.clone(),
                text: v.text.clone(),
                ast: v.ast.clone(),
            })
            .collect()
    }

    /// Get the index coordinator for lifecycle-aware index access
    ///
    /// Returns a reference to the IndexCoordinator, which provides:
    /// - `state()`: Lock-free check of current index state (Building/Ready/Degraded)
    /// - `index()`: Access to underlying WorkspaceIndex for queries
    /// - `notify_change(uri)`: Notify of file change (tracks parse storm)
    /// - `notify_parse_complete(uri)`: Notify parse done (may trigger recovery)
    /// - `query(full, partial)`: Automatic dispatch based on state
    ///
    /// ## Usage Pattern
    /// ```rust,ignore
    /// if let Some(coordinator) = self.coordinator() {
    ///     coordinator.notify_change(&uri);
    ///     // ... do parsing work ...
    ///     coordinator.notify_parse_complete(&uri);
    /// }
    /// ```
    #[cfg(feature = "workspace")]
    #[inline]
    pub(crate) fn coordinator(&self) -> Option<&Arc<IndexCoordinator>> {
        self.index_coordinator.as_ref()
    }

    /// Coordinator stub when workspace feature is disabled
    ///
    /// Returns None since no coordinator is available without workspace indexing.
    #[cfg(not(feature = "workspace"))]
    #[inline]
    pub(crate) fn coordinator(&self) -> Option<&()> {
        None
    }

    /// Get the workspace index through the coordinator (DEPRECATED for handler use)
    ///
    /// **WARNING**: Do NOT use this method in LSP handlers. Use one of:
    /// - `route_index_access(self.coordinator())` for query operations
    /// - `coordinator.index()` directly for mutation operations
    ///
    /// This method exists for backwards compatibility and diagnostic purposes only.
    /// The grep guard in `scripts/gate-local.sh` enforces this restriction.
    ///
    /// # Usage in handlers
    ///
    /// Query operations (completion, references, navigation):
    /// ```rust,ignore
    /// let mode = route_index_access(self.coordinator());
    /// match mode {
    ///     IndexAccessMode::Full(coord) => { coord.index() }
    ///     IndexAccessMode::Partial(_) | IndexAccessMode::None => { /* fallback */ }
    /// }
    /// ```
    ///
    /// Mutation operations (text sync, file watcher):
    /// ```rust,ignore
    /// if let Some(coordinator) = self.coordinator() {
    ///     coordinator.notify_change(uri);
    ///     let _ = coordinator.index().index_file(url, content);
    ///     coordinator.notify_parse_complete(uri);
    /// }
    /// ```
    #[cfg(feature = "workspace")]
    #[inline]
    #[allow(dead_code)] // Kept for diagnostics/compatibility, not used in handlers
    pub(crate) fn workspace_index(&self) -> Option<Arc<WorkspaceIndex>> {
        self.coordinator().map(|c| Arc::clone(c.index()))
    }

    // Method implementations live in sibling modules:
    //   dispatch/        - handle_request, request routing
    //   language/         - all textDocument/* and workspace/* handlers
    //   client_requests   - server-to-client refresh requests
    //   constructors      - new(), with_io(), with_output(), Default
    //   document_access   - URI normalization, position conversion, document lookup
    //   symbol_extraction - AST symbol extraction and reference counting
    //   test_runners      - run_test, run_test_file
    //   test_api          - #[cfg(test)] public wrappers

    /// Number of background workspace-indexing tasks currently in flight.
    ///
    /// Returns 0 when all background `index_file` tasks have completed.
    /// Intended for tests that need to observe the async-indexing behaviour
    /// introduced by issue #2352.
    pub fn pending_index_tasks(&self) -> usize {
        self.pending_index_task_count.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Install the diagnostic debouncer (called from Scheduler::new after Arc wrapping).
    pub(crate) fn install_diagnostic_debouncer(
        &self,
        debouncer: diagnostic_debounce::DiagnosticDebouncer,
    ) {
        *self.diagnostic_debouncer.lock() = Some(debouncer);
    }

    /// Publish diagnostics with trailing-edge debouncing.
    ///
    /// If a debouncer is installed (normal runtime via Scheduler), the publication
    /// is deferred until a quiet period elapses. If no debouncer is installed
    /// (unit tests that construct LspServer directly), falls through to immediate
    /// publication.
    ///
    /// When [`RuntimeTuning::diagnostic_debounce_is_immediate`] is true (e.g. e2e
    /// mode), the debouncer is bypassed and diagnostics publish synchronously —
    /// the worker thread's millisecond-granularity wakeup would otherwise mask
    /// the latency we are trying to measure.
    pub(crate) fn publish_diagnostics_debounced(&self, uri: &str) {
        if self.runtime_tuning.diagnostic_debounce_is_immediate() {
            self.publish_diagnostics(uri);
            return;
        }
        let guard = self.diagnostic_debouncer.lock();
        if let Some(ref d) = *guard {
            d.schedule(uri);
        } else {
            drop(guard);
            self.publish_diagnostics(uri);
        }
    }

    /// Install the file watcher debouncer (called from Scheduler::new after Arc wrapping).
    pub fn install_file_watcher_debouncer(
        &self,
        debouncer: file_watcher_debounce::FileWatcherDebouncer,
    ) {
        *self.file_watcher_debouncer.lock() = Some(debouncer);
    }

    /// Schedule a file watcher URI for debounced batch processing.
    ///
    /// Returns `true` if a debouncer is installed (production runtime) and the
    /// URI was queued, `false` if no debouncer is present (unit-test path).
    pub fn schedule_file_watcher_uri(&self, uri: &str) -> bool {
        let guard = self.file_watcher_debouncer.lock();
        if let Some(ref d) = *guard {
            d.schedule(uri);
            true
        } else {
            false
        }
    }
}

// Helper functions for non-blocking handlers

pub(crate) fn location_from_path(p: &Path) -> serde_json::Value {
    // Try to convert path to URI, fall back to empty string if conversion fails
    let uri = Url::from_file_path(p).map(|u| u.to_string()).unwrap_or_default();
    // Jump to start of file or try to find 'package' later if you prefer
    serde_json::json!({
        "uri": uri,
        "range": { "start": { "line": 0, "character": 0}, "end": { "line": 0, "character": 0} }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::formatting::FormatRange;
    use crate::runtime::types::workspace_folder_matches_doc_uri;
    use perl_lsp_rs_core::config::AiCompletionConfig;
    use perl_lsp_rs_core::providers::inline_completion::{
        NextEditGateSource, NextEditStatus, PreparedInlineCompletionContext,
    };

    fn next_edit_test_context() -> PreparedInlineCompletionContext {
        PreparedInlineCompletionContext {
            prefix: "use My::".to_string(),
            current_line: "use My::".to_string(),
            previous_non_empty_line: Some("use strict;".to_string()),
            current_function: None,
            current_package: Some("Demo".to_string()),
            variables: vec!["$got".to_string()],
            imports: vec!["strict".to_string(), "warnings".to_string()],
        }
    }

    static AI_TEST_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    struct AiTestEnvGuard {
        key: &'static str,
        previous: Option<std::ffi::OsString>,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl AiTestEnvGuard {
        // required for std::env::set_var in Rust 2024; the guard serializes and restores test env.
        #[allow(unsafe_code)]
        fn set(key: &'static str, value: &str) -> Result<Self, Box<dyn std::error::Error>> {
            let lock = AI_TEST_ENV_LOCK
                .lock()
                .map_err(|_| std::io::Error::other("AI test env lock poisoned"))?;
            let previous = std::env::var_os(key);
            unsafe { std::env::set_var(key, value) };
            Ok(Self { key, previous, _lock: lock })
        }
    }

    impl Drop for AiTestEnvGuard {
        // required for std::env::set_var/remove_var in Rust 2024; restores captured test env.
        #[allow(unsafe_code)]
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => unsafe { std::env::set_var(self.key, value) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }

    #[test]
    fn workspace_folder_matching_supports_non_file_uri_schemes() {
        let folder = WorkspaceFolderState::new("vscode-remote://ssh-remote+dev/workspace".into());
        assert!(workspace_folder_matches_doc_uri(
            &folder,
            "vscode-remote://ssh-remote+dev/workspace/lib/Foo.pm"
        ));
        assert!(!workspace_folder_matches_doc_uri(
            &folder,
            "vscode-remote://ssh-remote+dev/other/lib/Foo.pm"
        ));
    }

    #[test]
    fn workspace_folder_matching_supports_non_file_uri_with_trailing_slash() {
        let folder = WorkspaceFolderState::new("vscode-remote://ssh-remote+dev/workspace/".into());
        assert!(workspace_folder_matches_doc_uri(
            &folder,
            "vscode-remote://ssh-remote+dev/workspace/lib/Foo.pm"
        ));
        assert!(!workspace_folder_matches_doc_uri(
            &folder,
            "vscode-remote://ssh-remote+dev/workspace-other/lib/Foo.pm"
        ));
    }

    /// In a nested multi-root workspace (e.g. `/repo` and `/repo/app`), a
    /// document inside the inner folder matches both. `folder_for_doc_uri`,
    /// `config_for_doc`, and `include_paths_for_doc` must all agree on the
    /// most-specific (deepest) folder so that root and per-doc config don't
    /// disagree. This mirrors the existing deepest-match behavior of
    /// `workspace_root_for_doc` in `runtime/lifecycle/module_resolution.rs`.
    #[test]
    fn nested_workspace_folders_select_most_specific_for_config_and_includes() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        let app = repo.join("app");
        std::fs::create_dir_all(&app).expect("create nested dirs");

        let repo_uri = url::Url::from_file_path(&repo).expect("repo URI").to_string();
        let app_uri = url::Url::from_file_path(&app).expect("app URI").to_string();
        let doc_path = app.join("script").join("run.pl");
        std::fs::create_dir_all(doc_path.parent().expect("doc parent")).expect("create doc parent");
        std::fs::write(&doc_path, "use strict;\n").expect("write doc");
        let doc_uri = url::Url::from_file_path(&doc_path).expect("doc URI").to_string();

        // Distinct configs so we can verify we got the inner one.
        let mut outer_state = WorkspaceFolderState::new(repo_uri.clone());
        outer_state.effective_workspace_config.include_paths = vec!["outer_lib".into()];
        let mut inner_state = WorkspaceFolderState::new(app_uri.clone());
        inner_state.effective_workspace_config.include_paths = vec!["inner_lib".into()];

        // Register the outer FIRST. With first-match (the old behavior) this
        // would mistakenly win for documents inside the inner folder.
        let server = LspServer::default();
        {
            let mut folders = server.workspace_folders.lock();
            folders.push(outer_state.clone());
            folders.push(inner_state.clone());
        }

        // folder_for_doc_uri picks the deepest match.
        let picked = server.folder_for_doc_uri(&doc_uri).expect("folder must match");
        assert_eq!(picked.uri, app_uri, "expected inner folder for nested doc");

        // config_for_doc returns the inner folder's config.
        let cfg = server.config_for_doc(&doc_uri).expect("config must resolve");
        assert_eq!(
            cfg.include_paths,
            vec!["inner_lib".to_string()],
            "expected inner folder's includePaths for nested doc",
        );

        // include_paths_for_doc lists inner paths first, then outer as fallback.
        let resolved = server.include_paths_for_doc(&doc_uri);
        let inner_first = app.join("inner_lib");
        let outer_fallback = repo.join("outer_lib");
        let inner_pos =
            resolved.iter().position(|p| p == &inner_first).expect("inner_lib must be present");
        let outer_pos = resolved
            .iter()
            .position(|p| p == &outer_fallback)
            .expect("outer_lib must be present as fallback");
        assert!(
            inner_pos < outer_pos,
            "inner folder's includePaths must precede outer folder's; got {resolved:?}",
        );

        // search_scopes_for_doc puts the inner folder first.
        let scopes = server.search_scopes_for_doc(&doc_uri);
        assert!(!scopes.is_empty(), "expected at least one scope");
        assert_eq!(scopes[0].uri, app_uri, "first scope must be inner folder");
    }

    /// Documents outside every workspace folder must produce a `None`
    /// `folder_for_doc_uri`. The fallback "all folders, registration order"
    /// behavior is exercised separately by `search_scopes_for_doc`.
    #[test]
    fn workspace_folder_outside_all_folders_returns_none() {
        let temp = tempfile::tempdir().expect("tempdir");
        let inside = temp.path().join("inside");
        let outside = temp.path().join("outside");
        std::fs::create_dir_all(&inside).expect("create inside");
        std::fs::create_dir_all(&outside).expect("create outside");

        let inside_uri = url::Url::from_file_path(&inside).expect("inside URI").to_string();
        let doc_uri =
            url::Url::from_file_path(outside.join("doc.pl")).expect("doc URI").to_string();

        let server = LspServer::default();
        server.workspace_folders.lock().push(WorkspaceFolderState::new(inside_uri));
        assert!(
            server.folder_for_doc_uri(&doc_uri).is_none(),
            "doc outside all folders must not match any folder",
        );
        assert!(server.config_for_doc(&doc_uri).is_none());
    }

    #[test]
    fn next_edit_runtime_boundary_defaults_disabled() {
        let server = LspServer::new();

        let gate = server.next_edit_feature_gate();
        assert!(!gate.enabled);
        assert_eq!(gate.source, NextEditGateSource::DefaultOff);

        let response = server.next_edit_scaffold_response(next_edit_test_context());
        assert_eq!(response.status, NextEditStatus::Disabled);
        assert!(response.suggestions.is_empty());
    }

    #[test]
    fn next_edit_runtime_boundary_honors_explicit_config_without_provider_registration() {
        let server = LspServer::new();

        server.handle_did_change_configuration(Some(json!({
            "settings": {
                "perl": {
                    "nextEdit": {
                        "enabled": true
                    }
                }
            }
        })));

        let gate = server.next_edit_feature_gate();
        assert!(gate.enabled);
        assert_eq!(gate.source, NextEditGateSource::ExplicitConfig);

        let response = server.next_edit_scaffold_response(next_edit_test_context());
        assert_eq!(response.status, NextEditStatus::RuntimeProviderNotRegistered);
        assert!(response.suggestions.is_empty());
    }

    #[test]
    fn next_edit_runtime_boundary_can_be_disabled_after_explicit_config() {
        let server = LspServer::new();

        server.handle_did_change_configuration(Some(json!({
            "settings": {
                "perl": {
                    "nextEdit": {
                        "enabled": true
                    }
                }
            }
        })));
        assert!(server.next_edit_feature_gate().enabled);

        server.handle_did_change_configuration(Some(json!({
            "settings": {
                "perl": {
                    "nextEdit": {
                        "enabled": false
                    }
                }
            }
        })));

        let gate = server.next_edit_feature_gate();
        assert!(!gate.enabled);
        assert_eq!(gate.source, NextEditGateSource::DefaultOff);

        let response = server.next_edit_scaffold_response(next_edit_test_context());
        assert_eq!(response.status, NextEditStatus::Disabled);
        assert!(response.suggestions.is_empty());
    }

    #[test]
    fn runtime_pressure_snapshot_reports_async_queues() -> Result<(), Box<dyn std::error::Error>> {
        use std::time::{Duration, Instant};

        let server = LspServer::new();
        let uri = "file:///runtime-pressure.pl";

        server.install_diagnostic_debouncer(
            diagnostic_debounce::DiagnosticDebouncer::with_interval(Duration::from_mins(1), |_| {}),
        );
        server.install_file_watcher_debouncer(
            file_watcher_debounce::FileWatcherDebouncer::with_interval(
                Duration::from_mins(1),
                |_| {},
            ),
        );

        server.publish_diagnostics_debounced(uri);
        assert!(server.schedule_file_watcher_uri(uri));
        server.stream_sessions().start_session(stream_session::SessionKey {
            uri: uri.to_string(),
            document_version: 1,
            line: 0,
            character: 0,
        });
        let request_id = ServerRequestId::new(1).ok_or("valid request id")?;
        server.pending_workspace_configuration_requests.lock().insert(
            request_id,
            PendingWorkspaceConfigurationRequest {
                folder_uris: vec!["file:///".to_string()],
                includes_global_item: true,
                created_at: Instant::now(),
            },
        );

        let snapshot = (0..50)
            .find_map(|_| {
                let snapshot = server.runtime_pressure_snapshot();
                if snapshot.diagnostic_debounce_pending_uris == 1
                    && snapshot.file_watcher_pending_uris == 1
                {
                    Some(snapshot)
                } else {
                    std::thread::sleep(Duration::from_millis(10));
                    None
                }
            })
            .expect("debouncer workers should report pending URI pressure");

        assert_eq!(snapshot.pending_index_tasks, 0);
        assert_eq!(snapshot.diagnostic_debounce_pending_uris, 1);
        assert_eq!(snapshot.file_watcher_pending_uris, 1);
        assert_eq!(snapshot.pending_workspace_configuration_requests, 1);
        assert_eq!(snapshot.active_stream_sessions, 1);
        Ok(())
    }

    #[test]
    fn source_path_from_uri_accepts_absolute_filesystem_paths()
    -> Result<(), Box<dyn std::error::Error>> {
        let path = std::env::current_dir()?.join("lib/Foo.pm");
        let raw_path = path.to_string_lossy();

        assert_eq!(source_path_from_uri(raw_path.as_ref()), Some(path));
        Ok(())
    }

    #[test]
    fn source_path_from_uri_accepts_local_file_uris() {
        let from_file_uri = source_path_from_uri("file:///tmp/from-uri.pl");
        assert!(from_file_uri.is_some_and(|path| path.ends_with("from-uri.pl")));

        let from_localhost_uri = source_path_from_uri("file://localhost/tmp/localhost.pl");
        assert!(from_localhost_uri.is_some_and(|path| path.ends_with("localhost.pl")));
    }

    #[test]
    fn source_path_from_uri_rejects_relative_filesystem_paths() {
        assert_eq!(source_path_from_uri("lib/Foo.pm"), None);
    }

    #[test]
    fn end_position_handles_trailing_final_newline() {
        let server = LspServer::new();
        let content = "package Foo;\n";
        let pos = server.get_document_end_position(content);
        assert_eq!(pos, json!({"line": 1, "character": 0}));
    }

    #[test]
    fn end_position_handles_missing_final_newline() {
        let server = LspServer::new();
        let content = "package Foo;";
        let pos = server.get_document_end_position(content);
        assert_eq!(pos, json!({"line": 0, "character": content.len()}));
    }

    #[test]
    fn code_action_append_uses_document_end() {
        use ropey::Rope;
        use std::sync::Arc;

        let server = LspServer::new();
        let uri = "file:///test.pl";
        let text = "package Foo;"; // No trailing newline
        let rope = Rope::from_str(text);
        let line_starts = LineStartsCache::new_rope(&rope);
        server.documents.lock().insert(
            uri.to_string(),
            DocumentState {
                rope,
                text: text.to_string(),
                version: 1,
                ast: None,
                parse_errors: Vec::new(),
                parent_map: ParentMap::default(),
                line_starts,
                generation: Arc::new(AtomicU32::new(0)),
                degradation_tier: crate::state::DegradationTier::Minimal,
                #[cfg(feature = "incremental")]
                incremental_doc: None,
                #[cfg(feature = "incremental")]
                incremental_state: None,
            },
        );

        let result =
            server.handle_code_actions_pragmas(Some(json!({"textDocument": {"uri": uri}})));
        if let Ok(Some(result)) = result {
            if let Some(actions) = result.as_array() {
                assert!(!actions.is_empty());
                let edit = &actions[0]["edit"]["changes"][uri][0]["range"];
                let end = server.get_document_end_position(text);
                assert_eq!(edit["start"], end);
                assert_eq!(edit["end"], end);
            }
        }
    }

    #[test]
    fn formatting_edit_has_correct_end_position() {
        let code = "sub test{my$x=1;return$x;}";
        let server = LspServer::new();
        let end = server.get_document_end_position(code);
        let range = FormatRange::whole_document(code);

        if let (Some(line), Some(character)) = (end["line"].as_u64(), end["character"].as_u64()) {
            assert_eq!(range.end.line, line as u32);
            assert_eq!(range.end.character, character as u32);
        }
    }

    #[test]
    fn resolve_ai_api_key_prefers_configured_env_var() {
        let config = AiCompletionConfig {
            api_key_env: "OPENAI_API_KEY".to_string(),
            ..AiCompletionConfig::default()
        };
        let read_env = |name: &str| match name {
            "OPENAI_API_KEY" => Some("openai-key".to_string()),
            "GEMINI_API_KEY" => Some("gemini-key".to_string()),
            _ => None,
        };

        assert_eq!(
            LspServer::resolve_ai_api_key_with(&config, read_env).as_deref(),
            Some("openai-key")
        );
    }

    #[test]
    fn resolve_ai_api_key_uses_gemini_fallback_when_configured_var_missing() {
        let config = AiCompletionConfig::default();
        let read_env = |name: &str| match name {
            "OPENAI_API_KEY" => None,
            "GEMINI_API_KEY" => Some("gemini-key".to_string()),
            _ => None,
        };

        assert_eq!(
            LspServer::resolve_ai_api_key_with(&config, read_env).as_deref(),
            Some("gemini-key")
        );
    }

    #[test]
    fn refresh_ai_backend_installs_connector_auth_backend() -> Result<(), Box<dyn std::error::Error>>
    {
        const KEY_ENV: &str = "PERL_LSP_TEST_CONNECTOR_API_KEY";
        let _env_guard = AiTestEnvGuard::set(KEY_ENV, "connector-key")?;

        let server = LspServer::new();
        {
            let mut config = server.config.lock();
            config.ai_completion = AiCompletionConfig {
                enabled: true,
                endpoint: "https://connector.example/v1/chat/completions".to_string(),
                model: "custom-code-model".to_string(),
                api_key_env: KEY_ENV.to_string(),
                api_key_header: "x-api-key".to_string(),
                api_key_prefix: None,
                ..AiCompletionConfig::default()
            };
        }

        server.refresh_ai_backend();

        assert!(server.ai_backend().is_some());
        Ok(())
    }

    // --- include_paths_for_doc tests ---

    #[test]
    fn include_paths_for_doc_resolves_relative_paths_against_folder_root()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");
        std::fs::create_dir_all(&workspace)?;
        let doc_uri = url::Url::from_file_path(workspace.join("script.pl"))
            .map_err(|_| "bad uri")?
            .to_string();

        let server = LspServer::new();
        let workspace_uri =
            url::Url::from_directory_path(&workspace).map_err(|_| "bad folder uri")?.to_string();
        {
            let mut folders = server.workspace_folders.lock();
            let mut config = perl_lsp_rs_core::config::WorkspaceConfig::default();
            config.include_paths = vec!["lib".to_string(), "t/lib".to_string()];
            config.use_perl5lib = false;
            folders.push(
                WorkspaceFolderState::new(workspace_uri)
                    .with_path(workspace.clone())
                    .with_effective_workspace_config(config),
            );
        }

        let paths = server.include_paths_for_doc(&doc_uri);
        assert!(
            paths.contains(&workspace.join("lib")),
            "expected workspace/lib in paths, got: {paths:?}"
        );
        assert!(
            paths.contains(&workspace.join("t/lib")),
            "expected workspace/t/lib in paths, got: {paths:?}"
        );
        Ok(())
    }

    #[test]
    fn include_paths_for_doc_deduplicates_across_folders() -> Result<(), Box<dyn std::error::Error>>
    {
        let temp = tempfile::tempdir()?;
        let folder_a = temp.path().join("a");
        let folder_b = temp.path().join("b");
        std::fs::create_dir_all(&folder_a)?;
        std::fs::create_dir_all(&folder_b)?;

        let doc_uri = url::Url::from_file_path(folder_a.join("script.pl"))
            .map_err(|_| "bad uri")?
            .to_string();

        let server = LspServer::new();
        {
            let mut folders = server.workspace_folders.lock();
            let mut config_a = perl_lsp_rs_core::config::WorkspaceConfig::default();
            config_a.include_paths = vec!["lib".to_string()];
            config_a.use_perl5lib = false;
            let mut config_b = perl_lsp_rs_core::config::WorkspaceConfig::default();
            config_b.include_paths = vec!["lib".to_string()];
            config_b.use_perl5lib = false;
            folders.push(
                WorkspaceFolderState::new(
                    url::Url::from_directory_path(&folder_a).map_err(|_| "bad uri")?.to_string(),
                )
                .with_path(folder_a.clone())
                .with_effective_workspace_config(config_a),
            );
            folders.push(
                WorkspaceFolderState::new(
                    url::Url::from_directory_path(&folder_b).map_err(|_| "bad uri")?.to_string(),
                )
                .with_path(folder_b.clone())
                .with_effective_workspace_config(config_b),
            );
        }

        let paths = server.include_paths_for_doc(&doc_uri);
        let lib_a = folder_a.join("lib");
        let lib_b = folder_b.join("lib");
        // Both resolved, but they're different absolute paths — no dedup expected here
        assert!(paths.contains(&lib_a), "expected folder_a/lib");
        assert!(paths.contains(&lib_b), "expected folder_b/lib");
        // No duplicates in the result
        assert_eq!(
            paths.len(),
            paths.iter().collect::<std::collections::HashSet<_>>().len(),
            "include_paths_for_doc must not contain duplicate entries"
        );
        Ok(())
    }

    #[test]
    fn include_paths_for_doc_respects_use_perl5lib_false() -> Result<(), Box<dyn std::error::Error>>
    {
        // When use_perl5lib=false, effective_include_paths must not inject PERL5LIB entries.
        // We verify this by building a WorkspaceConfig with an explicit include_paths list and
        // use_perl5lib=false, then ensuring the result matches exactly that list (resolved).
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");
        std::fs::create_dir_all(&workspace)?;
        let doc_uri = url::Url::from_file_path(workspace.join("script.pl"))
            .map_err(|_| "bad uri")?
            .to_string();

        let server = LspServer::new();
        let workspace_uri =
            url::Url::from_directory_path(&workspace).map_err(|_| "bad folder uri")?.to_string();
        {
            let mut folders = server.workspace_folders.lock();
            let mut config = perl_lsp_rs_core::config::WorkspaceConfig::default();
            config.include_paths = vec!["lib".to_string()];
            config.use_perl5lib = false; // env PERL5LIB must be ignored
            folders.push(
                WorkspaceFolderState::new(workspace_uri)
                    .with_path(workspace.clone())
                    .with_effective_workspace_config(config),
            );
        }

        let paths = server.include_paths_for_doc(&doc_uri);
        // Only the configured "lib" path should appear; no PERL5LIB injections.
        assert_eq!(paths, vec![workspace.join("lib")]);
        Ok(())
    }
}
