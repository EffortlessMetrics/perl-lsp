//! Full JSON-RPC LSP Server implementation
//!
//! This module provides a complete Language Server Protocol implementation
//! that can be used with any LSP-compatible editor.

use crate::runtime::workspace_folder::WorkspaceFolderState;

mod client_requests;
mod constructors;
pub(crate) mod diagnostic_debounce;
mod diagnostics;
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
mod window;
mod workspace;
mod workspace_folder;

// Re-export protocol types for backward compatibility
// Tests and external code import these from perl_lsp::
pub use crate::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};

// Re-export window types for public API
pub use window::{MessageType, ShowDocumentOptions};

use perl_parser::{
    Parser,
    ast::{Node, NodeKind},
    declaration::ParentMap,
    performance::{AstCache, SymbolIndex},
    perl_critic::BuiltInAnalyzer,
    position::LineStartsCache,
    tdd_basic::TestGenerator,
    test_runner::{TestKind, TestRunner},
};

use crate::call_hierarchy_provider::CallHierarchyProvider;
use crate::cancellation::{GLOBAL_CANCELLATION_REGISTRY, PerlLspCancellationToken};
use perl_lsp_feature_governance::FeatureProfile;

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
        extract_module_reference_extended, get_text_around_offset, offset_to_position,
        position_to_offset,
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
    atomic::{AtomicBool, AtomicI64, AtomicU32, Ordering},
};
use std::time::Instant;
use url::Url;

#[cfg(feature = "workspace")]
use perl_parser::workspace_index::{
    IndexCoordinator, LspWorkspaceSymbol, WorkspaceIndex, uri_to_fs_path,
};
#[cfg(feature = "workspace")]
use perl_position_tracking::{WireLocation, WirePosition, WireRange};

#[cfg(feature = "workspace")]
use crate::fallback::text::extract_text_based_symbols;

pub(super) fn source_path_from_uri(uri: &str) -> Option<PathBuf> {
    Url::parse(uri)
        .ok()
        .and_then(|value| if value.scheme() == "file" { value.to_file_path().ok() } else { None })
}

fn workspace_folder_path(folder: &WorkspaceFolderState) -> Option<PathBuf> {
    folder.path.clone().or_else(|| source_path_from_uri(&folder.uri))
}

fn workspace_folder_matches_doc_uri(folder: &WorkspaceFolderState, doc_uri: &str) -> bool {
    let doc_path = source_path_from_uri(doc_uri);
    match (doc_path, workspace_folder_path(folder)) {
        (Some(doc_path), Some(folder_path)) => doc_path.starts_with(folder_path),
        _ => {
            let folder_uri = folder.uri.trim_end_matches('/');
            doc_uri == folder.uri
                || doc_uri == folder_uri
                || doc_uri.strip_prefix(folder_uri).is_some_and(|suffix| suffix.starts_with('/'))
        }
    }
}

/// Lightweight view of a document for scan-heavy operations
///
/// This struct provides the minimal data needed for workspace-wide scans
/// (code lens resolve, reference counting) without requiring the full
/// DocumentState. Using this snapshot pattern allows the documents lock
/// to be released before CPU-intensive work begins.
///
/// ## Design Rationale
/// - `uri`: Needed to construct LSP Location responses
/// - `text`: Needed for text-based fallback searches (regex, line iteration)
/// - `ast`: Arc clone allows AST traversal without deep copying the tree
///
/// The rope, line_starts cache, parent_map, and other fields are omitted
/// as they're not typically needed for bulk scan operations.
pub(crate) struct DocumentScanView {
    /// Document URI for constructing Location responses
    #[allow(dead_code)] // Preserved for future scan operations that build Location responses
    pub uri: String,
    /// Document text content for text-based searches
    pub text: String,
    /// Optional AST reference (Arc clone) for AST-based operations
    pub ast: Option<Arc<perl_parser::ast::Node>>,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingWorkspaceConfigurationRequest {
    pub(crate) folder_uris: Vec<String>,
    pub(crate) includes_global: bool,
    pub(crate) sent_at: Instant,
}

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
    cancelled: Arc<Mutex<HashSet<Value>>>,
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
    next_request_id: Arc<AtomicI64>,
    /// Pending workspace/configuration reverse requests keyed by request ID.
    pending_workspace_configuration_requests:
        Arc<Mutex<HashMap<i64, PendingWorkspaceConfigurationRequest>>>,
    /// Active progress tokens for work done progress tracking
    progress_tokens: Arc<Mutex<HashSet<String>>>,
    /// Maps progress tokens to their originating request IDs for cancellation routing
    progress_token_to_request: Arc<Mutex<HashMap<String, Value>>>,
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
    /// Cache of extracted POD documentation keyed by resolved file path.
    pod_cache: Arc<Mutex<HashMap<PathBuf, perl_pod::PodDoc>>>,
    /// Cache of SemanticAnalyzer results keyed by (normalized_uri, content_hash).
    ///
    /// Avoids re-running the full O(n) AST traversal on repeated hover/definition
    /// requests to the same document version. Content hash provides automatic
    /// invalidation when source text changes — no TTL needed.
    pub(crate) semantic_analyzer_cache:
        Arc<Mutex<HashMap<(String, u64), Arc<crate::semantic::SemanticAnalyzer>>>>,
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
    pub(crate) ai_inline_backend:
        Mutex<Option<Arc<dyn perl_lsp_inline_completion::InlineCompletionBackend>>>,
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
    /// Active feature profile for this server instance.
    pub(crate) const fn feature_profile(&self) -> FeatureProfile {
        self.feature_profile
    }

    /// Get the registered AI inline-completion backend, if any.
    ///
    /// Returns `None` when no backend has been registered (the default).
    /// The returned `Arc` is a cheap clone suitable for use outside the lock.
    pub(crate) fn ai_backend(
        &self,
    ) -> Option<Arc<dyn perl_lsp_inline_completion::InlineCompletionBackend>> {
        self.ai_inline_backend.lock().clone()
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

        // Resolve API key from environment variable
        let api_key = std::env::var(&ai_config.api_key_env).unwrap_or_default();
        if api_key.is_empty() {
            tracing::warn!(env_var = %ai_config.api_key_env, "AI completion enabled but env var is empty or unset");
            *self.ai_inline_backend.lock() = None;
            return;
        }

        let provider_config = perl_lsp_ai_provider::OpenAiConfig {
            endpoint: ai_config.endpoint.clone(),
            model: ai_config.model.clone(),
            api_key,
            timeout_ms: ai_config.timeout_ms,
        };

        let limiter = Arc::new(perl_lsp_ai_provider::RateLimiter::new(
            ai_config.rate_limit_rps,
            ai_config.max_inflight,
        ));

        let provider = perl_lsp_ai_provider::OpenAiProvider::new(provider_config, limiter);
        *self.ai_inline_backend.lock() = Some(Arc::new(provider));

        tracing::info!(endpoint = %ai_config.endpoint, model = %ai_config.model, "AI inline completion backend configured");
    }

    /// Get the subprocess runtime for external tool execution (perltidy, perlcritic).
    ///
    /// Returns a new `OsSubprocessRuntime` for executing external processes.
    /// This is used by formatting and linting providers.
    pub fn subprocess_runtime(&self) -> perl_lsp_tooling::OsSubprocessRuntime {
        perl_lsp_tooling::OsSubprocessRuntime::new()
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

    // =========================================================================
    // Workspace folder helpers
    // =========================================================================

    /// Find the workspace folder containing a document URI.
    ///
    /// Returns the first workspace folder whose URI is a prefix of the document URI.
    /// Returns `None` if no workspace folder contains the document.
    #[must_use]
    pub fn folder_for_doc_uri(&self, doc_uri: &str) -> Option<WorkspaceFolderState> {
        self.workspace_folders
            .lock()
            .iter()
            .find(|folder| workspace_folder_matches_doc_uri(folder, doc_uri))
            .cloned()
    }

    /// Get the effective workspace config for a document's folder.
    ///
    /// Returns the effective workspace configuration for the folder containing
    /// the document, or `None` if the document is not in any workspace folder.
    #[must_use]
    pub fn config_for_doc(&self, doc_uri: &str) -> Option<perl_lsp_config::WorkspaceConfig> {
        self.workspace_folders
            .lock()
            .iter()
            .find(|folder| workspace_folder_matches_doc_uri(folder, doc_uri))
            .map(|folder| folder.effective_workspace_config.clone())
    }

    /// Get all include paths for a document (from its folder and others).
    ///
    /// Returns a vector of include paths from all workspace folders, with the
    /// current folder's paths first. This ordering is useful for module resolution
    /// where the current folder should take precedence.
    #[must_use]
    pub fn include_paths_for_doc(&self, doc_uri: &str) -> Vec<std::path::PathBuf> {
        let mut paths = Vec::new();
        let folders = self.workspace_folders.lock();

        // Add current folder's include paths first
        if let Some(current_folder) =
            folders.iter().find(|folder| workspace_folder_matches_doc_uri(folder, doc_uri))
        {
            for include_path in &current_folder.effective_workspace_config.include_paths {
                // Resolve relative paths against the folder path
                let resolved = if let Some(folder_path) = workspace_folder_path(current_folder) {
                    if std::path::Path::new(include_path).is_absolute() {
                        std::path::PathBuf::from(include_path)
                    } else {
                        folder_path.join(include_path)
                    }
                } else {
                    std::path::PathBuf::from(include_path)
                };

                if !paths.contains(&resolved) {
                    paths.push(resolved);
                }
            }
        }

        // Add other folders' include paths
        for folder in folders.iter() {
            if !workspace_folder_matches_doc_uri(folder, doc_uri) {
                for include_path in &folder.effective_workspace_config.include_paths {
                    let resolved = if let Some(folder_path) = workspace_folder_path(folder) {
                        if std::path::Path::new(include_path).is_absolute() {
                            std::path::PathBuf::from(include_path)
                        } else {
                            folder_path.join(include_path)
                        }
                    } else {
                        std::path::PathBuf::from(include_path)
                    };

                    if !paths.contains(&resolved) {
                        paths.push(resolved);
                    }
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
        if let Some(current_folder) =
            folders.iter().find(|folder| workspace_folder_matches_doc_uri(folder, doc_uri))
        {
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
    pub(crate) fn publish_diagnostics_debounced(&self, uri: &str) {
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
}
