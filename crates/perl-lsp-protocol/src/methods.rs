//! LSP method name constants for standardized request/notification routing.
//!
//! This module centralizes all LSP method identifiers to ensure consistency across
//! dispatch logic, capability registration, and error reporting. All constants follow
//! the LSP 3.17 specification naming conventions.
//!
//! # Organization
//!
//! Constants are grouped by functional area:
//! - **Lifecycle**: `INITIALIZE`, `SHUTDOWN`, `EXIT`
//! - **Text Document Synchronization**: `TEXT_DOCUMENT_DID_OPEN`, `TEXT_DOCUMENT_DID_CHANGE`
//! - **Language Features**: Completion, navigation, symbols, formatting, refactoring
//! - **Workspace Features**: Symbol search, configuration, file operations
//! - **Special Methods**: Cancellation, testing, experimental
//!
//! # Usage Examples
//!
//! ## Basic Request Dispatching
//!
//! ```rust
//! use perl_lsp_protocol::methods;
//!
//! fn handle_request(method: &str) {
//!     match method {
//!         methods::INITIALIZE => handle_initialize(),
//!         methods::TEXT_DOCUMENT_HOVER => handle_hover(),
//!         methods::WORKSPACE_SYMBOL => handle_workspace_symbol(),
//!         methods::SHUTDOWN => handle_shutdown(),
//!         _ => handle_unknown_method(),
//!     }
//! }
//! # fn handle_initialize() {}
//! # fn handle_hover() {}
//! # fn handle_workspace_symbol() {}
//! # fn handle_shutdown() {}
//! # fn handle_unknown_method() {}
//! ```
//!
//! ## Cancellation Registration
//!
//! ```rust
//! use perl_lsp_protocol::methods;
//!
//! fn should_support_cancellation(method: &str) -> bool {
//!     matches!(
//!         method,
//!         methods::TEXT_DOCUMENT_COMPLETION
//!             | methods::TEXT_DOCUMENT_HOVER
//!             | methods::TEXT_DOCUMENT_DEFINITION
//!             | methods::TEXT_DOCUMENT_REFERENCES
//!             | methods::WORKSPACE_SYMBOL
//!     )
//! }
//! ```
//!
//! ## Server-to-Client Notifications
//!
//! ```rust
//! use perl_lsp_protocol::methods;
//!
//! fn send_diagnostics(uri: &str) {
//!     // Send diagnostics using the standard method name
//!     send_notification(
//!         methods::TEXT_DOCUMENT_PUBLISH_DIAGNOSTICS,
//!         uri
//!     );
//! }
//!
//! fn refresh_semantic_tokens() {
//!     // Request client to refresh semantic tokens
//!     send_request(methods::WORKSPACE_SEMANTIC_TOKENS_REFRESH);
//! }
//! # fn send_notification(_method: &str, _uri: &str) {}
//! # fn send_request(_method: &str) {}
//! ```
//!
//! # Benefits
//!
//! - **Type Safety**: Compile-time verification of method names
//! - **Consistency**: Single source of truth for all LSP methods
//! - **Discoverability**: IDE auto-completion shows all available methods
//! - **Maintainability**: Easy to add new methods or update existing ones
//! - **Documentation**: Each constant is documented with its purpose

// ============================================================================
// Lifecycle Methods
// ============================================================================

/// Initialize request - first request from client to server
pub const INITIALIZE: &str = "initialize";

/// Initialized notification - sent after initialize response
pub const INITIALIZED: &str = "initialized";

/// Shutdown request - graceful server shutdown
pub const SHUTDOWN: &str = "shutdown";

/// Exit notification - terminate server process
pub const EXIT: &str = "exit";

// ============================================================================
// Text Document Synchronization
// ============================================================================

/// Document opened notification
pub const TEXT_DOCUMENT_DID_OPEN: &str = "textDocument/didOpen";

/// Document changed notification
pub const TEXT_DOCUMENT_DID_CHANGE: &str = "textDocument/didChange";

/// Document closed notification
pub const TEXT_DOCUMENT_DID_CLOSE: &str = "textDocument/didClose";

/// Document saved notification
pub const TEXT_DOCUMENT_DID_SAVE: &str = "textDocument/didSave";

/// Document will save notification
pub const TEXT_DOCUMENT_WILL_SAVE: &str = "textDocument/willSave";

/// Document will save with edits request
pub const TEXT_DOCUMENT_WILL_SAVE_WAIT_UNTIL: &str = "textDocument/willSaveWaitUntil";

/// Publish diagnostics notification (server to client)
pub const TEXT_DOCUMENT_PUBLISH_DIAGNOSTICS: &str = "textDocument/publishDiagnostics";

// ============================================================================
// Language Features - Completion
// ============================================================================

/// Code completion request
pub const TEXT_DOCUMENT_COMPLETION: &str = "textDocument/completion";

/// Completion item resolve request
pub const COMPLETION_ITEM_RESOLVE: &str = "completionItem/resolve";

// ============================================================================
// Language Features - Navigation
// ============================================================================

/// Hover information request
pub const TEXT_DOCUMENT_HOVER: &str = "textDocument/hover";

/// Signature help request
pub const TEXT_DOCUMENT_SIGNATURE_HELP: &str = "textDocument/signatureHelp";

/// Go to definition request
pub const TEXT_DOCUMENT_DEFINITION: &str = "textDocument/definition";

/// Go to declaration request
pub const TEXT_DOCUMENT_DECLARATION: &str = "textDocument/declaration";

/// Go to type definition request
pub const TEXT_DOCUMENT_TYPE_DEFINITION: &str = "textDocument/typeDefinition";

/// Go to implementation request
pub const TEXT_DOCUMENT_IMPLEMENTATION: &str = "textDocument/implementation";

/// Find references request
pub const TEXT_DOCUMENT_REFERENCES: &str = "textDocument/references";

// ============================================================================
// Language Features - Document Symbols
// ============================================================================

/// Document symbols request
pub const TEXT_DOCUMENT_DOCUMENT_SYMBOL: &str = "textDocument/documentSymbol";

/// Document highlight request
pub const TEXT_DOCUMENT_DOCUMENT_HIGHLIGHT: &str = "textDocument/documentHighlight";

// ============================================================================
// Language Features - Code Actions
// ============================================================================

/// Code action request
pub const TEXT_DOCUMENT_CODE_ACTION: &str = "textDocument/codeAction";

/// Code action resolve request
pub const CODE_ACTION_RESOLVE: &str = "codeAction/resolve";

/// Code lens request
pub const TEXT_DOCUMENT_CODE_LENS: &str = "textDocument/codeLens";

/// Code lens resolve request
pub const CODE_LENS_RESOLVE: &str = "codeLens/resolve";

// ============================================================================
// Language Features - Formatting
// ============================================================================

/// Document formatting request
pub const TEXT_DOCUMENT_FORMATTING: &str = "textDocument/formatting";

/// Range formatting request
pub const TEXT_DOCUMENT_RANGE_FORMATTING: &str = "textDocument/rangeFormatting";

/// Multiple ranges formatting request
pub const TEXT_DOCUMENT_RANGES_FORMATTING: &str = "textDocument/rangesFormatting";

/// On-type formatting request
pub const TEXT_DOCUMENT_ON_TYPE_FORMATTING: &str = "textDocument/onTypeFormatting";

// ============================================================================
// Language Features - Refactoring
// ============================================================================

/// Prepare rename request
pub const TEXT_DOCUMENT_PREPARE_RENAME: &str = "textDocument/prepareRename";

/// Rename request
pub const TEXT_DOCUMENT_RENAME: &str = "textDocument/rename";

/// Linked editing range request
pub const TEXT_DOCUMENT_LINKED_EDITING_RANGE: &str = "textDocument/linkedEditingRange";

// ============================================================================
// Language Features - Semantic Tokens
// ============================================================================

/// Semantic tokens full document request
pub const TEXT_DOCUMENT_SEMANTIC_TOKENS_FULL: &str = "textDocument/semanticTokens/full";

/// Semantic tokens range request
pub const TEXT_DOCUMENT_SEMANTIC_TOKENS_RANGE: &str = "textDocument/semanticTokens/range";

// ============================================================================
// Language Features - Inlay Hints
// ============================================================================

/// Inlay hints request
pub const TEXT_DOCUMENT_INLAY_HINT: &str = "textDocument/inlayHint";

/// Inlay hint resolve request
pub const INLAY_HINT_RESOLVE: &str = "inlayHint/resolve";

// ============================================================================
// Language Features - Document Links
// ============================================================================

/// Document links request
pub const TEXT_DOCUMENT_DOCUMENT_LINK: &str = "textDocument/documentLink";

/// Document link resolve request
pub const DOCUMENT_LINK_RESOLVE: &str = "documentLink/resolve";

// ============================================================================
// Language Features - Folding
// ============================================================================

/// Folding range request
pub const TEXT_DOCUMENT_FOLDING_RANGE: &str = "textDocument/foldingRange";

/// Selection range request
pub const TEXT_DOCUMENT_SELECTION_RANGE: &str = "textDocument/selectionRange";

// ============================================================================
// Language Features - Type Hierarchy
// ============================================================================

/// Prepare type hierarchy request
pub const TEXT_DOCUMENT_PREPARE_TYPE_HIERARCHY: &str = "textDocument/prepareTypeHierarchy";

/// Type hierarchy prepare (alternate/deprecated)
pub const TYPE_HIERARCHY_PREPARE: &str = "typeHierarchy/prepare";

/// Type hierarchy supertypes request
pub const TYPE_HIERARCHY_SUPERTYPES: &str = "typeHierarchy/supertypes";

/// Type hierarchy subtypes request
pub const TYPE_HIERARCHY_SUBTYPES: &str = "typeHierarchy/subtypes";

// ============================================================================
// Language Features - Call Hierarchy
// ============================================================================

/// Prepare call hierarchy request
pub const TEXT_DOCUMENT_PREPARE_CALL_HIERARCHY: &str = "textDocument/prepareCallHierarchy";

/// Call hierarchy incoming calls request
pub const CALL_HIERARCHY_INCOMING_CALLS: &str = "callHierarchy/incomingCalls";

/// Call hierarchy outgoing calls request
pub const CALL_HIERARCHY_OUTGOING_CALLS: &str = "callHierarchy/outgoingCalls";

// ============================================================================
// Language Features - Diagnostics
// ============================================================================

/// Document diagnostic request
pub const TEXT_DOCUMENT_DIAGNOSTIC: &str = "textDocument/diagnostic";

/// Workspace diagnostic request
pub const WORKSPACE_DIAGNOSTIC: &str = "workspace/diagnostic";

// ============================================================================
// Language Features - Inline Features
// ============================================================================

/// Inline completion request
pub const TEXT_DOCUMENT_INLINE_COMPLETION: &str = "textDocument/inlineCompletion";

/// Inline value request (debugging)
pub const TEXT_DOCUMENT_INLINE_VALUE: &str = "textDocument/inlineValue";

// ============================================================================
// Language Features - Colors
// ============================================================================

/// Document color request
pub const TEXT_DOCUMENT_DOCUMENT_COLOR: &str = "textDocument/documentColor";

/// Color presentation request
pub const TEXT_DOCUMENT_COLOR_PRESENTATION: &str = "textDocument/colorPresentation";

// ============================================================================
// Language Features - Monikers
// ============================================================================

/// Moniker request
pub const TEXT_DOCUMENT_MONIKER: &str = "textDocument/moniker";

// ============================================================================
// Workspace Features
// ============================================================================

/// Workspace symbols request
pub const WORKSPACE_SYMBOL: &str = "workspace/symbol";

/// Workspace symbol resolve request
pub const WORKSPACE_SYMBOL_RESOLVE: &str = "workspace/symbol/resolve";

/// Execute command request
pub const WORKSPACE_EXECUTE_COMMAND: &str = "workspace/executeCommand";

/// Apply workspace edit request (server to client)
pub const WORKSPACE_APPLY_EDIT: &str = "workspace/applyEdit";

/// Configuration request (server to client)
pub const WORKSPACE_CONFIGURATION: &str = "workspace/configuration";

/// Text document content request (virtual documents)
pub const WORKSPACE_TEXT_DOCUMENT_CONTENT: &str = "workspace/textDocumentContent";

// ============================================================================
// Workspace Features - File Operations
// ============================================================================

/// Files will be created notification
pub const WORKSPACE_WILL_CREATE_FILES: &str = "workspace/willCreateFiles";

/// Files created notification
pub const WORKSPACE_DID_CREATE_FILES: &str = "workspace/didCreateFiles";

/// Files will be renamed notification
pub const WORKSPACE_WILL_RENAME_FILES: &str = "workspace/willRenameFiles";

/// Files renamed notification
pub const WORKSPACE_DID_RENAME_FILES: &str = "workspace/didRenameFiles";

/// Files will be deleted notification
pub const WORKSPACE_WILL_DELETE_FILES: &str = "workspace/willDeleteFiles";

/// Files deleted notification
pub const WORKSPACE_DID_DELETE_FILES: &str = "workspace/didDeleteFiles";

// ============================================================================
// Workspace Features - Configuration and Watchers
// ============================================================================

/// Workspace folders changed notification
pub const WORKSPACE_DID_CHANGE_WORKSPACE_FOLDERS: &str = "workspace/didChangeWorkspaceFolders";

/// Configuration changed notification
pub const WORKSPACE_DID_CHANGE_CONFIGURATION: &str = "workspace/didChangeConfiguration";

/// Watched files changed notification
pub const WORKSPACE_DID_CHANGE_WATCHED_FILES: &str = "workspace/didChangeWatchedFiles";

// ============================================================================
// Workspace Features - Refresh Requests (server to client)
// ============================================================================

/// Code lens refresh request
pub const WORKSPACE_CODE_LENS_REFRESH: &str = "workspace/codeLens/refresh";

/// Semantic tokens refresh request
pub const WORKSPACE_SEMANTIC_TOKENS_REFRESH: &str = "workspace/semanticTokens/refresh";

/// Inlay hint refresh request
pub const WORKSPACE_INLAY_HINT_REFRESH: &str = "workspace/inlayHint/refresh";

/// Inline value refresh request
pub const WORKSPACE_INLINE_VALUE_REFRESH: &str = "workspace/inlineValue/refresh";

/// Diagnostic refresh request
pub const WORKSPACE_DIAGNOSTIC_REFRESH: &str = "workspace/diagnostic/refresh";

/// Folding range refresh request
pub const WORKSPACE_FOLDING_RANGE_REFRESH: &str = "workspace/foldingRange/refresh";

/// Text document content refresh request (virtual documents)
pub const WORKSPACE_TEXT_DOCUMENT_CONTENT_REFRESH: &str = "workspace/textDocumentContent/refresh";

// ============================================================================
// Notebook Document Features
// ============================================================================

/// Notebook document opened notification
pub const NOTEBOOK_DOCUMENT_DID_OPEN: &str = "notebookDocument/didOpen";

/// Notebook document changed notification
pub const NOTEBOOK_DOCUMENT_DID_CHANGE: &str = "notebookDocument/didChange";

/// Notebook document saved notification
pub const NOTEBOOK_DOCUMENT_DID_SAVE: &str = "notebookDocument/didSave";

/// Notebook document closed notification
pub const NOTEBOOK_DOCUMENT_DID_CLOSE: &str = "notebookDocument/didClose";

// ============================================================================
// Window Features
// ============================================================================

/// Show message notification (server to client) - displays a message in the UI
pub const WINDOW_SHOW_MESSAGE: &str = "window/showMessage";

/// Log message notification (server to client) - logs a message to the output channel
pub const WINDOW_LOG_MESSAGE: &str = "window/logMessage";

/// Show message request (server to client) - displays a message with action buttons
pub const WINDOW_SHOW_MESSAGE_REQUEST: &str = "window/showMessageRequest";

/// Show document request (server to client) - opens a document in the editor
pub const WINDOW_SHOW_DOCUMENT: &str = "window/showDocument";

/// Work done progress create request (server to client) - creates a progress indicator
pub const WINDOW_WORK_DONE_PROGRESS_CREATE: &str = "window/workDoneProgress/create";

/// Work done progress cancel notification
pub const WINDOW_WORK_DONE_PROGRESS_CANCEL: &str = "window/workDoneProgress/cancel";

// ============================================================================
// Special Methods
// ============================================================================

/// Cancel request notification
pub const CANCEL_REQUEST: &str = "$/cancelRequest";

/// Progress notification (bidirectional) - reports work done progress updates
pub const DOLLAR_PROGRESS: &str = "$/progress";

/// Test slow operation (testing only)
pub const TEST_SLOW_OPERATION: &str = "$/test/slowOperation";

// ============================================================================
// Experimental Features
// ============================================================================

/// Test discovery (experimental)
pub const EXPERIMENTAL_TEST_DISCOVERY: &str = "experimental/testDiscovery";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifecycle_methods() {
        assert_eq!(INITIALIZE, "initialize");
        assert_eq!(INITIALIZED, "initialized");
        assert_eq!(SHUTDOWN, "shutdown");
        assert_eq!(EXIT, "exit");
    }

    #[test]
    fn test_text_document_methods() {
        assert_eq!(TEXT_DOCUMENT_HOVER, "textDocument/hover");
        assert_eq!(TEXT_DOCUMENT_COMPLETION, "textDocument/completion");
        assert_eq!(TEXT_DOCUMENT_DEFINITION, "textDocument/definition");
        assert_eq!(TEXT_DOCUMENT_REFERENCES, "textDocument/references");
    }

    #[test]
    fn test_workspace_methods() {
        assert_eq!(WORKSPACE_SYMBOL, "workspace/symbol");
        assert_eq!(WORKSPACE_EXECUTE_COMMAND, "workspace/executeCommand");
        assert_eq!(WORKSPACE_DIAGNOSTIC, "workspace/diagnostic");
    }

    #[test]
    fn test_hierarchy_methods() {
        assert_eq!(CALL_HIERARCHY_INCOMING_CALLS, "callHierarchy/incomingCalls");
        assert_eq!(CALL_HIERARCHY_OUTGOING_CALLS, "callHierarchy/outgoingCalls");
        assert_eq!(TYPE_HIERARCHY_SUPERTYPES, "typeHierarchy/supertypes");
        assert_eq!(TYPE_HIERARCHY_SUBTYPES, "typeHierarchy/subtypes");
    }

    #[test]
    fn test_special_methods() {
        assert_eq!(CANCEL_REQUEST, "$/cancelRequest");
        assert_eq!(DOLLAR_PROGRESS, "$/progress");
        assert_eq!(TEST_SLOW_OPERATION, "$/test/slowOperation");
    }

    #[test]
    fn test_window_methods() {
        assert_eq!(WINDOW_SHOW_MESSAGE, "window/showMessage");
        assert_eq!(WINDOW_LOG_MESSAGE, "window/logMessage");
        assert_eq!(WINDOW_SHOW_MESSAGE_REQUEST, "window/showMessageRequest");
        assert_eq!(WINDOW_SHOW_DOCUMENT, "window/showDocument");
        assert_eq!(WINDOW_WORK_DONE_PROGRESS_CREATE, "window/workDoneProgress/create");
        assert_eq!(WINDOW_WORK_DONE_PROGRESS_CANCEL, "window/workDoneProgress/cancel");
    }

    #[test]
    fn test_notification_methods() {
        assert_eq!(TEXT_DOCUMENT_DID_OPEN, "textDocument/didOpen");
        assert_eq!(TEXT_DOCUMENT_DID_CHANGE, "textDocument/didChange");
        assert_eq!(TEXT_DOCUMENT_DID_SAVE, "textDocument/didSave");
        assert_eq!(WORKSPACE_DID_CHANGE_CONFIGURATION, "workspace/didChangeConfiguration");
    }

    #[test]
    fn test_refresh_methods() {
        assert_eq!(WORKSPACE_CODE_LENS_REFRESH, "workspace/codeLens/refresh");
        assert_eq!(WORKSPACE_SEMANTIC_TOKENS_REFRESH, "workspace/semanticTokens/refresh");
        assert_eq!(WORKSPACE_INLAY_HINT_REFRESH, "workspace/inlayHint/refresh");
        assert_eq!(WORKSPACE_DIAGNOSTIC_REFRESH, "workspace/diagnostic/refresh");
    }

    #[test]
    fn test_constants_are_unique() {
        // Verify no duplicate values (except intentional aliases like TYPE_HIERARCHY_PREPARE)
        let all_methods = vec![
            INITIALIZE,
            SHUTDOWN,
            TEXT_DOCUMENT_HOVER,
            TEXT_DOCUMENT_COMPLETION,
            WORKSPACE_SYMBOL,
            CALL_HIERARCHY_INCOMING_CALLS,
            // Add more as needed
        ];

        for (i, method1) in all_methods.iter().enumerate() {
            for (j, method2) in all_methods.iter().enumerate() {
                if i != j {
                    assert_ne!(method1, method2, "Found duplicate method: {}", method1);
                }
            }
        }
    }
}
