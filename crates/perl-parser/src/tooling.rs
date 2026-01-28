//! Compatibility re-export of tooling integrations.

/// Performance utilities for LSP feature optimization.
pub mod performance {
    pub use perl_lsp_tooling::performance::{AstCache, IncrementalParser, SymbolIndex, parallel};
}

/// Perl critic integration for linting.
pub mod perl_critic {
    #[cfg(not(feature = "lsp-compat"))]
    pub use perl_lsp_tooling::perl_critic::ViolationSummary;
    pub use perl_lsp_tooling::perl_critic::{
        BuiltInAnalyzer, CriticAnalyzer, CriticConfig, Policy, QuickFix, Severity, TextEdit,
        Violation,
    };
}

/// Perltidy integration for formatting.
pub mod perltidy {
    pub use perl_lsp_tooling::perltidy::*;
}
