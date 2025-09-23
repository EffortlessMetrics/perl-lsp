//! LSP Error helpers
//!
//! Provides consistent error responses for the LSP server

use crate::lsp_server::JsonRpcError;
use serde_json::{Value, json};

/// LSP error codes (from the spec)
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    pub const SERVER_ERROR_START: i32 = -32099;
    pub const SERVER_ERROR_END: i32 = -32000;
    pub const SERVER_NOT_INITIALIZED: i32 = -32002;
    pub const UNKNOWN_ERROR_CODE: i32 = -32001;
    pub const REQUEST_CANCELLED: i32 = -32800;
    pub const CONTENT_MODIFIED: i32 = -32801;
    pub const SERVER_CANCELLED: i32 = -32802; // LSP 3.17: server-side cancellation
    pub const REQUEST_FAILED: i32 = -32803;
}

/// Create a method not found error for LSP operations during Perl parsing workflow
///
/// # Arguments
///
/// * `method` - The LSP method name that was not found or supported
///
/// # Returns
///
/// A [`JsonRpcError`] with METHOD_NOT_FOUND code for client response
///
/// # Email Processing Context
///
/// This occurs when LSP clients request unsupported features during Perl script analysis
/// in the LSP workflow. Common during IDE integration when processing large Perl files
/// where certain language features may be disabled for performance optimization.
///
/// # Examples
///
/// ```rust
/// use perl_parser::lsp_errors::method_not_found;
///
/// let error = method_not_found("textDocument/semanticTokens");
/// assert_eq!(error.code, -32601);
/// ```
pub fn method_not_found(method: &str) -> JsonRpcError {
    JsonRpcError {
        code: error_codes::METHOD_NOT_FOUND,
        message: format!("Method '{}' not found or not supported", method),
        data: None,
    }
}

/// Create a method not found error for unadvertised features in Perl parsing workflows
///
/// # Returns
///
/// A [`JsonRpcError`] indicating the method was not advertised in server capabilities
///
/// # Email Processing Context
///
/// Occurs when LSP clients attempt to use features not enabled during LSP workflow
/// processing. This helps enforce capability boundaries during large-scale Perl analysis
/// where resource constraints require selective feature enabling.
///
/// # Examples
///
/// ```rust
/// use perl_parser::lsp_errors::method_not_advertised;
///
/// let error = method_not_advertised();
/// assert_eq!(error.code, -32601);
/// ```
pub fn method_not_advertised() -> JsonRpcError {
    JsonRpcError {
        code: error_codes::METHOD_NOT_FOUND,
        message: "Method not advertised in server capabilities".to_string(),
        data: None,
    }
}

/// Create a server cancelled error for operations terminated during Perl parsing
///
/// # Arguments
///
/// * `message` - Descriptive message about why the operation was cancelled
///
/// # Returns
///
/// A [`JsonRpcError`] with SERVER_CANCELLED code (-32802) for LSP 3.17 compliance
///
/// # Email Processing Context
///
/// Used when long-running Perl analysis operations are cancelled to maintain system
/// responsiveness during 50GB+ Perl file processing. Enables graceful cancellation
/// of expensive operations like workspace-wide symbol indexing.
///
/// # Examples
///
/// ```rust
/// use perl_parser::lsp_errors::server_cancelled;
///
/// let error = server_cancelled("Operation cancelled due to memory pressure");
/// assert_eq!(error.code, -32802);
/// ```
pub fn server_cancelled(message: &str) -> JsonRpcError {
    JsonRpcError { code: error_codes::SERVER_CANCELLED, message: message.to_string(), data: None }
}

/// Create an invalid params error for malformed LSP requests during Perl parsing
///
/// # Arguments
///
/// * `message` - Descriptive error message about the parameter validation failure
///
/// # Returns
///
/// A JSON [`Value`] containing the error code and message for LSP response
///
/// # Email Processing Context
///
/// Occurs when LSP clients send malformed parameters during Perl script analysis
/// operations. Common when processing complex PST structures where parameter
/// validation ensures data integrity throughout the LSP workflow.
pub fn invalid_params(message: &str) -> Value {
    json!({
        "code": error_codes::INVALID_PARAMS,
        "message": message
    })
}

/// Create an internal error for unexpected failures during Perl parsing workflows
///
/// # Arguments
///
/// * `message` - Descriptive error message about the internal failure
///
/// # Returns
///
/// A JSON [`Value`] containing the error code and message for LSP response
///
/// # Email Processing Context
///
/// Used when unexpected internal failures occur during LSP workflow processing,
/// such as memory allocation failures during large Perl codebase analysis or threading
/// issues during concurrent Perl parsing operations.
pub fn internal_error(message: &str) -> Value {
    json!({
        "code": error_codes::INTERNAL_ERROR,
        "message": message
    })
}

/// Create an invalid parameters error for LSP operations during Perl parsing workflow
///
/// # Arguments
///
/// * `message` - Descriptive error message about the parameter validation failure
///
/// # Returns
///
/// A [`JsonRpcError`] with INVALID_PARAMS code for LSP response
///
/// # Email Processing Context
///
/// Occurs when LSP clients send malformed parameters during Perl script analysis.
/// Common when processing complex Perl structures where parameter validation ensures
/// data integrity throughout the Parse → Index → Navigate → Complete → Analyze workflow.
pub fn invalid_params_err(message: &str) -> JsonRpcError {
    JsonRpcError { code: error_codes::INVALID_PARAMS, message: message.to_string(), data: None }
}

/// Create an internal error for unexpected failures during Perl parsing workflows
///
/// # Arguments
///
/// * `message` - Descriptive error message about the internal failure
///
/// # Returns
///
/// A [`JsonRpcError`] with INTERNAL_ERROR code for LSP response
///
/// # Email Processing Context
///
/// Used when unexpected internal failures occur during LSP workflow processing,
/// such as memory allocation failures during large Perl codebase analysis or threading
/// issues during concurrent Perl parsing operations. Recovery involves graceful degradation.
pub fn internal_error_err(message: &str) -> JsonRpcError {
    JsonRpcError { code: error_codes::INTERNAL_ERROR, message: message.to_string(), data: None }
}

/// Configuration structure for tracking which LSP features are advertised during Perl parsing
///
/// This struct manages feature availability during LSP workflow operations, allowing
/// selective enabling/disabling of LSP capabilities based on processing requirements
/// and resource constraints during large-scale Perl analysis.
///
/// # Performance Context
///
/// Feature advertisement is optimized for 50GB+ Perl codebase processing scenarios where
/// resource-intensive features may be dynamically disabled to maintain throughput
/// and memory efficiency during enterprise-scale Perl analysis.
pub struct AdvertisedFeatures {
    /// Enable code lens support for Perl script analysis
    pub code_lens: bool,
    /// Enable call hierarchy navigation during Perl parsing
    pub call_hierarchy: bool,
    /// Enable type hierarchy analysis for Perl objects in Perl scripts
    pub type_hierarchy: bool,
    /// Enable inlay hints for Perl parsing workflows
    pub inlay_hints: bool,
    /// Enable semantic token highlighting during Perl script analysis
    pub semantic_tokens: bool,
    /// Enable code actions for Perl parsing workflow optimization
    pub code_actions: bool,
    /// Enable symbol rename operations across Perl parsing pipeline
    pub rename: bool,
    /// Enable document link resolution in Perl code
    pub document_links: bool,
    /// Enable selection range provider for Perl script navigation
    pub selection_ranges: bool,
    /// Enable on-type formatting during Perl script editing
    pub on_type_formatting: bool,
    /// Enable pull-based diagnostics for Perl parsing workflows
    pub pull_diagnostics: bool,
}

impl Default for AdvertisedFeatures {
    fn default() -> Self {
        // Match production BuildFlags
        Self {
            code_lens: false,
            call_hierarchy: true,
            type_hierarchy: true,
            inlay_hints: true,
            semantic_tokens: true,
            code_actions: true,
            rename: true,
            document_links: true,
            selection_ranges: true,
            on_type_formatting: true,
            pull_diagnostics: true,
        }
    }
}

impl AdvertisedFeatures {
    /// Create GA-lock features configuration for production Perl parsing workflows
    ///
    /// # Returns
    ///
    /// An [`AdvertisedFeatures`] instance with all features disabled for maximum stability
    ///
    /// # Email Processing Context
    ///
    /// This conservative configuration is used during production LSP workflow processing
    /// of large Perl files where stability takes precedence over IDE features. Ensures
    /// minimal resource consumption during enterprise-scale Perl analysis operations.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_parser::lsp_errors::AdvertisedFeatures;
    ///
    /// let features = AdvertisedFeatures::ga_lock();
    /// assert!(!features.code_lens);
    /// assert!(!features.semantic_tokens);
    /// ```
    pub fn ga_lock() -> Self {
        Self {
            code_lens: false,
            call_hierarchy: false,
            type_hierarchy: false,
            inlay_hints: false,
            semantic_tokens: false,
            code_actions: false,
            rename: false,
            document_links: false,
            selection_ranges: false,
            on_type_formatting: false,
            pull_diagnostics: false,
        }
    }

    /// Check if an LSP method should be refused based on advertised features during Perl parsing
    ///
    /// # Arguments
    ///
    /// * `method` - The LSP method name to check against advertised capabilities
    ///
    /// # Returns
    ///
    /// `true` if the method should be refused, `false` if it should be allowed
    ///
    /// # Email Processing Context
    ///
    /// This method enforces capability boundaries during LSP workflow operations,
    /// preventing resource-intensive LSP operations when processing large Perl files.
    /// Helps maintain performance targets during enterprise-scale Perl analysis.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use perl_parser::lsp_errors::AdvertisedFeatures;
    ///
    /// let features = AdvertisedFeatures::ga_lock();
    /// assert!(features.should_refuse("textDocument/codeLens"));
    /// assert!(!features.should_refuse("textDocument/hover")); // Core feature always allowed
    /// ```
    pub fn should_refuse(&self, method: &str) -> bool {
        match method {
            "textDocument/codeLens" => !self.code_lens,
            "textDocument/prepareCallHierarchy"
            | "callHierarchy/incomingCalls"
            | "callHierarchy/outgoingCalls" => !self.call_hierarchy,
            "textDocument/prepareTypeHierarchy"
            | "typeHierarchy/supertypes"
            | "typeHierarchy/subtypes" => !self.type_hierarchy,
            _ => false, // Allow by default for core features
        }
    }
}
