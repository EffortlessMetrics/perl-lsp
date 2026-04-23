//! Diagnostics provider (delegated to perl-lsp-providers).

pub mod pull;
pub use pull::{PullDiagnosticsContext, PullDiagnosticsProvider};

// Re-export core diagnostics types from perl-lsp-diagnostics
pub use perl_lsp_rs_core::providers::diagnostics::{
    Diagnostic, DiagnosticSeverity, DiagnosticTag, DiagnosticsProvider, RelatedInformation,
};
