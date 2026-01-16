//! Diagnostics provider (delegated to perl-lsp-providers).

pub mod pull;
pub use pull::PullDiagnosticsProvider;

// Re-export core diagnostics types from perl-lsp-providers
pub use perl_lsp_providers::ide::lsp_compat::diagnostics::{
    Diagnostic, DiagnosticSeverity, DiagnosticTag, DiagnosticsProvider, RelatedInformation,
};
