//! IDE integration module for Perl parser
//!
//! This module provides comprehensive Language Server Protocol (LSP) integration
//! for Perl development, including completion, diagnostics, refactoring, and
//! navigation features.
//!
//! # LSP Workflow Integration
//!
//! Core component in the Parse → Index → Navigate → Complete → Analyze pipeline:
//! 1. **Parse**: AST generation with semantic analysis
//! 2. **Index**: Workspace symbol table construction
//! 3. **Navigate**: Go-to-definition and reference finding
//! 4. **Complete**: Context-aware completion
//! 5. **Analyze**: Code actions and refactoring
//!
//! # Module Organization
//!
//! - **lsp_compat**: LSP compatibility layer with protocol implementations
//!   - `completion`: Code completion provider
//!   - `diagnostics`: Diagnostic and error reporting
//!   - `references`: Reference finding and navigation
//!   - `rename`: Safe symbol renaming
//!   - `code_actions`: Intelligent code actions and refactoring
//!   - `semantic_tokens`: Semantic highlighting
//!   - `workspace_symbols`: Workspace-wide symbol search
//!   - `inlay_hints`: Contextual inlay hints
//!   - `type_hierarchy`: Type hierarchy analysis
//!
//! # Performance Characteristics
//!
//! - **LSP response times**: <50ms for typical operations
//! - **Memory usage**: ~10MB for full workspace analysis
//! - **Incremental updates**: <1ms for single-character changes
//! - **Workspace scaling**: Designed for 50K+ file workspaces
//!
//! # Usage Examples
//!
//! ```rust
//! use perl_parser::ide::lsp_compat::{
//!     completion::CompletionProvider,
//!     diagnostics::DiagnosticProvider,
//!     references::ReferenceProvider,
//! };
//! use lsp_types::*;
//! use url::Url;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create providers
//! let completion_provider = CompletionProvider::new();
//! let diagnostic_provider = DiagnosticProvider::new();
//! let reference_provider = ReferenceProvider::new();
//!
//! // Use LSP providers
//! let uri = Url::parse("file:///example.pl")?;
//! let position = Position::new(0, 10);
//!
//! // Code completion
//! let completion_params = CompletionParams {
//!     text_document_position: TextDocumentPositionParams {
//!         text_document: TextDocumentIdentifier { uri: uri.clone() },
//!         position,
//!     },
//!     work_done_progress_params: Default::default(),
//!     partial_result_params: Default::default(),
//!     context: None,
//! };
//!
//! let completions = completion_provider.complete(completion_params);
//!
//! // Diagnostics
//! // (would need parse result)
//!
//! // References
//! let reference_params = ReferenceParams {
//!     text_document_position: TextDocumentPositionParams {
//!         text_document: TextDocumentIdentifier { uri },
//!         position,
//!     },
//!     context: ReferenceContext {
//!         include_declaration: true,
//!     },
//!     work_done_progress_params: Default::default(),
//!     partial_result_params: Default::default(),
//! };
//!
//! let references = reference_provider.find_references(reference_params);
//! # Ok(())
//! # }
//! ```

pub mod lsp_compat;

// Re-export commonly used types for convenience
pub use lsp_compat::{
    completion::CompletionProvider,
    diagnostics::DiagnosticProvider,
    references::ReferenceProvider,
    rename::RenameProvider,
    code_actions::CodeActionProvider,
    semantic_tokens::SemanticTokensProvider,
    workspace_symbols::WorkspaceSymbolProvider,
    inlay_hints::InlayHintsProvider,
    type_hierarchy::TypeHierarchyProvider,
};