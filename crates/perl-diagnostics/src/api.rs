//! Public API re-exports.
//!
//! Explicit per-symbol re-exports (no wildcards).
//! This prevents silent breakage from future module edits that might introduce colliding names.

// Re-export from codes module
pub use crate::codes::{DiagnosticCategory, DiagnosticCode, DiagnosticSeverity, DiagnosticTag};

// Re-export from types module
pub use crate::types::{Diagnostic, RelatedInformation};

// Note: DiagnosticSeverity and DiagnosticTag are canonically defined in codes::
// and re-exported via types::. api.rs re-exports them via the canonical codes:: path.

// Re-export from catalog module
pub use crate::catalog::{
    DiagnosticMeta, bareword_filehandle, critic_severity_1, critic_severity_2, critic_severity_3,
    critic_severity_4, critic_severity_5, diagnostic_meta, duplicate_package, duplicate_sub,
    eval_error_flow, from_message, implicit_return, missing_package_declaration, missing_return,
    missing_strict, missing_warnings, parse_error, syntax_error, two_arg_open, undefined_var,
    unexpected_eof, unused_var,
};
