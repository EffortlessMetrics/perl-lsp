//! Perl Language Server Protocol Runtime
//!
//! This crate provides the production-ready LSP server runtime for Perl, implementing the
//! Language Server Protocol specification for seamless integration with editors like VSCode,
//! Neovim, Emacs, and any LSP-compatible client.
//!
//! The runtime handles all protocol communication, message framing, state management, and
//! feature dispatching, while delegating parsing and analysis to the `perl_parser` engine.
//!
//! # Quick Start
//!
//! ## Running the Server
//!
//! The simplest way to start the server is via the binary:
//!
//! ```bash
//! # Install from source
//! cargo install --path crates/perl-lsp
//!
//! # Run in stdio mode (default)
//! perl-lsp --stdio
//!
//! # Check health status
//! perl-lsp --health
//!
//! # Show version information
//! perl-lsp --version
//! ```
//!
//! ## Programmatic Usage
//!
//! Use the runtime as a library to embed LSP support:
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! perl_lsp::run_stdio()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Custom Server Configuration
//!
//! For advanced use cases, create a server instance directly:
//!
//! ```no_run
//! use perl_lsp::LspServer;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut server = LspServer::new();
//! server.run()?;
//! # Ok(())
//! # }
//! ```
//!
//! # Architecture
//!
//! The runtime is organized into distinct layers for maintainability and extensibility:
//!
//! ## Protocol Layer
//!
//! - **[`protocol`]**: JSON-RPC message types and LSP protocol definitions
//! - **[`transport`]**: Message framing and transport (stdio, TCP, WebSocket)
//! - **[`dispatch`]**: Request routing and method dispatch logic
//!
//! ## State Management
//!
//! - **[`state`]**: Document and workspace state management
//! - **[`textdoc`]**: Text document synchronization and version tracking
//! - **[`runtime`]**: Server lifecycle and initialization management
//!
//! ## Feature Providers
//!
//! The [`features`] module contains all LSP capability implementations:
//!
//! - **Completion**: Context-aware code completion with type inference
//! - **Hover**: Symbol information and documentation on hover
//! - **Definition**: Go-to-definition with cross-file support
//! - **References**: Find all references across workspace
//! - **Rename**: Symbol renaming with conflict detection
//! - **Diagnostics**: Real-time syntax and semantic validation
//! - **Formatting**: Code formatting via perltidy integration
//! - **Semantic Tokens**: Fine-grained syntax highlighting
//! - **Code Actions**: Quick fixes and refactoring suggestions
//! - **Code Lens**: Inline actionable metadata
//! - **Inlay Hints**: Parameter names and type information
//! - **Call Hierarchy**: Function call navigation
//! - **Type Hierarchy**: Class inheritance navigation
//! - **Document Links**: File and URL navigation
//! - **Folding**: Code folding for blocks and regions
//! - **Selection Range**: Smart text selection expansion
//! - **Signature Help**: Function signature assistance
//!
//! ## Utilities
//!
//! - **[`convert`]**: Conversions between [`perl_parser`] types and [`lsp_types`]
//! - **[`util`]**: URI handling, UTF-16 conversion, and position mapping
//! - **[`fallback`]**: Text-based fallback when parsing fails
//! - **[`cancellation`]**: Request cancellation infrastructure
//! - **[`diagnostics_catalog`]**: Stable diagnostic codes
//!
//! ## Request Handling
//!
//! - **[`handlers`]**: Request and notification handlers
//! - **[`server`]**: Public server API and message processing loop
//!
//! # Protocol Support
//!
//! The server implements LSP 3.17 specification features:
//!
//! ## Lifecycle Methods
//!
//! - `initialize` - Server initialization and capability negotiation
//! - `initialized` - Post-initialization notification
//! - `shutdown` - Graceful server shutdown
//! - `exit` - Server process termination
//!
//! ## Document Synchronization
//!
//! - `textDocument/didOpen` - Document opened in editor
//! - `textDocument/didChange` - Document content changed
//! - `textDocument/didSave` - Document saved to disk
//! - `textDocument/didClose` - Document closed in editor
//!
//! ## Language Features
//!
//! See [`features`] module documentation for complete feature list and capabilities.
//!
//! # Communication Modes
//!
//! ## Stdio Mode (Default)
//!
//! Standard input/output transport for editor integration:
//!
//! ```bash
//! perl-lsp --stdio
//! ```
//!
//! Editors configure this mode in their LSP client settings:
//!
//! ### VSCode Configuration
//!
//! ```json
//! {
//!   "perl.lsp.command": "perl-lsp",
//!   "perl.lsp.args": ["--stdio"]
//! }
//! ```
//!
//! ### Neovim Configuration
//!
//! ```lua
//! require'lspconfig'.perl.setup{
//!   cmd = { "perl-lsp", "--stdio" }
//! }
//! ```
//!
//! ## Socket Mode
//!
//! TCP socket transport for remote or debugging scenarios:
//!
//! ```bash
//! perl-lsp --socket --port 9257
//! ```
//!
//! Connect via TCP socket from any LSP client supporting network transport.
//!
//! # Features Module
//!
//! All LSP capabilities are implemented in the [`features`] module, organized by category:
//!
//! ```text
//! features/
//! ├── completion.rs           # Code completion
//! ├── diagnostics.rs          # Real-time validation
//! ├── formatting.rs           # Code formatting
//! ├── hover.rs                # Hover information
//! ├── references.rs           # Find references
//! ├── rename.rs               # Symbol renaming
//! ├── semantic_tokens.rs      # Syntax highlighting
//! ├── code_actions.rs         # Quick fixes
//! ├── code_lens_provider.rs   # Code lens
//! ├── inlay_hints.rs          # Inline hints
//! └── ...
//! ```
//!
//! # State Management
//!
//! The server maintains state across requests:
//!
//! - **Document State**: Parsed ASTs, symbol tables, and version tracking
//! - **Workspace Index**: Cross-file symbol resolution and references
//! - **Configuration**: Client-provided settings and capabilities
//! - **Diagnostics**: Cached validation results for incremental updates
//!
//! # Error Handling
//!
//! The runtime implements comprehensive error recovery:
//!
//! - **Parse Errors**: Graceful degradation to text-based features
//! - **Protocol Errors**: JSON-RPC error responses with diagnostic codes
//! - **Cancellation**: Request cancellation for long-running operations
//! - **Fallback**: Text-based completion/hover when parsing fails
//!
//! # Performance Optimizations
//!
//! - **Incremental Parsing**: Reuse unchanged AST nodes on edits
//! - **Lazy Indexing**: Index files on-demand rather than at startup
//! - **Caching**: Cache parse results and symbol tables
//! - **Cancellation**: Cancel stale requests when new ones arrive
//! - **Streaming**: Stream large responses to avoid blocking
//!
//! # Integration with perl-parser
//!
//! The runtime delegates all parsing and analysis to `perl_parser`:
//!
//! ```rust,ignore
//! use perl_parser::Parser;
//! use perl_lsp::convert::to_lsp_diagnostic;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let code = "my $x = 1;";
//! let mut parser = Parser::new(code);
//! let ast = parser.parse()?;
//!
//! // Convert parser errors to LSP diagnostics
//! let diagnostics: Vec<_> = parser.errors()
//!     .iter()
//!     .map(|e| to_lsp_diagnostic(e, &ast))
//!     .collect();
//! ```
//!
//! # Testing
//!
//! The runtime includes comprehensive test coverage:
//!
//! ```bash
//! # Run all LSP runtime tests
//! cargo test -p perl-lsp
//!
//! # Run with adaptive threading for resource-constrained environments
//! RUST_TEST_THREADS=2 cargo test -p perl-lsp
//!
//! # Test specific feature
//! cargo test -p perl-lsp completion
//! ```
//!
//! # Logging and Diagnostics
//!
//! Enable logging for debugging:
//!
//! ```bash
//! perl-lsp --stdio --log
//! ```
//!
//! Logs are written to stderr, separate from LSP protocol communication on stdout/stdin.
//!
//! # Client Capabilities
//!
//! The server adapts to client capabilities negotiated during initialization:
//!
//! - **Dynamic Registration**: Supports dynamic capability registration
//! - **Workspace Folders**: Multi-root workspace support
//! - **Configuration**: Client-side configuration changes
//! - **Pull Diagnostics**: Diagnostic pull model (LSP 3.17+)
//!
//! # Security Considerations
//!
//! - **Sandboxed Execution**: No arbitrary code execution
//! - **Path Validation**: URI validation and path sanitization
//! - **Resource Limits**: Memory and time budgets for operations
//! - **Input Validation**: Strict protocol message validation
//!
//! # Migration from perl-parser
//!
//! This crate was extracted from `perl_parser` to separate LSP runtime concerns
//! from parsing engine logic. The migration maintains API compatibility while
//! improving modularity.
//!
//! Legacy code can continue using `perl_parser::lsp_server::LspServer` with
//! deprecation warnings pointing to this crate.
//!
//! # Related Crates
//!
//! - `perl_parser`: Parsing engine and analysis infrastructure
//! - `perl_lexer`: Context-aware Perl tokenizer
//! - `perl_corpus`: Comprehensive test corpus
//! - `perl_dap`: Debug Adapter Protocol implementation
//!
//! # Documentation
//!
//! - **LSP Guide**: See `docs/LSP_IMPLEMENTATION_GUIDE.md` in the repository
//! - **Capability Policy**: See `docs/LSP_CAPABILITY_POLICY.md`
//! - **Commands Reference**: See `docs/COMMANDS_REFERENCE.md`
//!
//! # Usage Example
//!
//! Complete example of running the LSP server:
//!
//! ```no_run
//! use perl_lsp::LspServer;
//!
//! let mut server = LspServer::new();
//!
//! match server.run() {
//!     Ok(()) => println!("Server exited cleanly"),
//!     Err(e) => eprintln!("Server error: {}", e),
//! }
//! ```

#![deny(unsafe_code)]
#![warn(missing_docs)]
#![allow(missing_docs)] // Temporarily allow until migration is complete
#![allow(
    // Migrated from perl-parser - these patterns are acceptable in LSP runtime code
    clippy::collapsible_match,
    clippy::only_used_in_recursion,
    clippy::while_let_loop,
    clippy::needless_range_loop,
    clippy::for_kv_map,
    clippy::arc_with_non_send_sync,
    clippy::mutable_key_type,
    clippy::new_without_default,
    clippy::if_same_then_else
)]

// Module declarations - migrated from perl-parser
pub mod cancellation;
pub mod convert;
pub mod diagnostics_catalog;
pub mod dispatch;
pub mod execute_command;
pub mod fallback;
pub mod features;
pub mod handlers;
pub mod protocol;
pub mod runtime;
pub mod server;
pub mod state;
pub mod textdoc;
pub mod transport;
pub mod util;

// Re-exports for key types
pub use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
pub use server::LspServer;

/// DAP bridge adapter re-export
#[cfg(feature = "dap-phase1")]
pub use perl_dap::BridgeAdapter;

// =============================================================================
// Internal compatibility re-exports (crate-internal, not API surface)
// =============================================================================
// These re-exports allow migrated code to use `crate::...` paths for engine
// pieces while we incrementally update paths to `perl_parser::...`

/// Parser re-export for migrated code
pub(crate) use perl_parser::Parser;

/// Position utilities re-export
pub(crate) mod position {
    pub use perl_parser::position::*;
}

/// Declaration types re-export
pub(crate) mod declaration {
    pub use perl_parser::declaration::*;
}

/// Workspace index re-export
pub(crate) mod workspace_index {
    pub use perl_parser::workspace_index::*;
}

/// Symbol types re-export
pub(crate) mod symbol {
    pub use perl_parser::symbol::*;
}

/// AST types re-export
pub(crate) mod ast {
    pub use perl_parser::ast::*;
}

/// Feature re-exports for old intra-crate paths
pub(crate) mod code_actions_enhanced {
    #[allow(unused_imports)]
    pub use crate::features::code_actions_enhanced::*;
}

pub(crate) mod code_lens_provider {
    pub use crate::features::code_lens_provider::*;
}

pub(crate) mod diagnostics {
    #[allow(unused_imports)]
    pub use crate::features::diagnostics::*;
}

// More feature re-exports for runtime imports
pub(crate) mod inlay_hints {
    pub use crate::features::inlay_hints::*;
}

pub(crate) mod document_links {
    pub use crate::features::document_links::*;
}

pub(crate) mod lsp_document_link {
    pub use crate::features::lsp_document_link::*;
}

pub(crate) mod selection_range {
    pub use crate::features::selection_range::*;
}

pub(crate) mod linked_editing {
    pub use crate::features::linked_editing::*;
}

pub(crate) mod code_actions_pragmas {
    pub use crate::features::code_actions_pragmas::*;
}

// Engine re-exports for runtime
pub(crate) mod perl_critic {
    pub use perl_parser::perl_critic::*;
}

pub(crate) mod semantic {
    pub use perl_parser::semantic::*;
}

pub(crate) mod error {
    pub use perl_parser::error::*;
}

pub(crate) mod completion {
    pub use crate::features::completion::*;
}

pub(crate) mod on_type_formatting {
    pub use crate::features::on_type_formatting::*;
}

pub(crate) mod inline_completions {
    pub use crate::features::inline_completions::*;
}

pub(crate) mod type_hierarchy {
    pub use crate::features::type_hierarchy::*;
}

// Re-export SourceLocation at crate root for convenience
pub(crate) use perl_parser::ast::SourceLocation;

// Engine modules needed by runtime
pub(crate) mod type_inference {
    pub use perl_parser::type_inference::*;
}

pub(crate) mod builtin_signatures {
    pub use perl_parser::builtin_signatures::*;
}

pub(crate) mod workspace_rename {
    pub use crate::features::workspace_rename::*;
}

pub(crate) mod semantic_tokens {
    pub use crate::features::semantic_tokens::*;
}

pub(crate) mod call_hierarchy_provider;

// Parser module re-export for tests using crate::parser::Parser
pub(crate) mod parser {
    #[allow(unused_imports)]
    pub use perl_parser::parser::*;
}

// Folding re-export
pub(crate) mod folding {
    pub use crate::features::folding::*;
}

// References re-export
pub(crate) mod references {
    #[allow(unused_imports)]
    pub use crate::features::references::*;
}

// Rename re-export
pub(crate) mod rename {
    #[allow(unused_imports)]
    pub use crate::features::rename::*;
}

// Signature help re-export
pub(crate) mod signature_help {
    #[allow(unused_imports)]
    pub use crate::features::signature_help::*;
}

/// Run the LSP server in stdio mode.
///
/// This is the main entry point for the LSP server. It reads JSON-RPC messages from stdin
/// and writes responses to stdout, following the Language Server Protocol specification.
///
/// # Errors
///
/// Returns an error if:
/// - The transport layer fails to initialize
/// - Message framing or parsing fails
/// - The server encounters an unrecoverable error
///
/// # Example
///
/// ```no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// perl_lsp::run_stdio()?;
/// # Ok(())
/// # }
/// ```
pub fn run_stdio() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = LspServer::new();
    server.run().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}
