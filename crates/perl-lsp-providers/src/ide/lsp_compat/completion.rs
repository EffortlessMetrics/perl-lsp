//! Deprecated completion compatibility module.
//!
//! This module preserves legacy imports through
//! `perl_lsp_providers::ide::lsp_compat::completion`.
//! Prefer `perl_lsp_providers::completion` for new code.
//!
//! # LSP Workflow Integration
//!
//! Part of the Parse → Index → Navigate → Complete → Analyze pipeline:
//! 1. **Parse**: Source code parsed into AST
//! 2. **Index**: Symbols extracted and indexed
//! 3. **Navigate**: Cross-file symbol resolution
//! 4. **Complete**: Context-aware completion suggestions (this module)
//! 5. **Analyze**: Semantic analysis and refactoring
//!
//! # Protocol and Client Capabilities
//!
//! - **Client capabilities**: Respects `textDocument.completion` client capabilities
//!   including snippet support, commit characters, and completion item kinds.
//! - **Protocol compliance**: Implements `textDocument/completion` request from
//!   LSP 3.17 specification with support for completion item resolve.
//!
//! # Usage Examples
//!
//! ```rust
//! use perl_parser::Parser;
//!
//! let code = "my $x = 42;";
//! let mut parser = Parser::new(code);
//! match parser.parse() {
//!     Ok(ast) => println!("Parsed successfully"),
//!     Err(e) => eprintln!("Parse error: {}", e),
//! }
//! ```
//!
//! # See Also
//!
//! - [`crate::ide::lsp_compat::diagnostics`] for diagnostic publishing
//! - [`crate::ide::lsp_compat::code_actions`] for code action support
//! - [`perl_lsp_completion::CompletionProvider`] for the main completion implementation

pub use perl_lsp_completion::*;
