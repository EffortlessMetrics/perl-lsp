//! Pull-based diagnostics infrastructure (LSP server specific)
//!
//! **NOTE**: Pull diagnostics remain in the `perl-lsp` crate.
//!
//! This module is LSP server infrastructure that depends on `DocumentState`
//! and other LSP server components. The core `DiagnosticsProvider` has been
//! moved to `perl_lsp_providers::ide::lsp_compat::diagnostics`.
//!
//! # Architecture
//!
//! - **Core diagnostics**: `perl_lsp_providers::ide::lsp_compat::diagnostics`
//!   - Pure diagnostic analysis and provider logic
//!   - No LSP server dependencies
//!
//! - **Pull diagnostics**: `perl_lsp::features::diagnostics::pull`
//!   - LSP server infrastructure for pull-based diagnostics
//!   - Depends on DocumentState and LSP server types
//!
//! # Migration
//!
//! ```ignore
//! // For core diagnostics:
//! use perl_lsp_providers::ide::lsp_compat::diagnostics::DiagnosticsProvider;
//!
//! // For pull diagnostics infrastructure:
//! use perl_lsp::features::diagnostics::pull::PullDiagnosticsProvider;
//! ```
