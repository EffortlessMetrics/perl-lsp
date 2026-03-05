//! Central configuration accessors for LSP operation limits and bounded behavior.
//!
//! This crate provides runtime/global state wrappers around
//! [`perl_lsp_limits_types::LspLimits`] to expose thread-safe access throughout
//! the language server process.
//!
//! # Usage
//!
//! ```rust,ignore
//! use perl_lsp_limits::LspLimits;
//!
//! let limits = LspLimits::default();
//! let results = my_query().take(limits.references_cap);
//! ```

use std::time::Duration;

pub use perl_lsp_limits_types::LspLimits;

/// Global singleton for LSP limits
///
/// Initialized with default values, can be updated via LSP settings.
/// Thread-safe via internal locking.
pub static LSP_LIMITS: std::sync::LazyLock<std::sync::RwLock<LspLimits>> =
    std::sync::LazyLock::new(|| std::sync::RwLock::new(LspLimits::default()));

/// Get current workspace symbol cap
#[inline]
pub fn workspace_symbol_cap() -> usize {
    LSP_LIMITS.read().map(|l| l.workspace_symbol_cap).unwrap_or(200)
}

/// Get current references cap
#[inline]
pub fn references_cap() -> usize {
    LSP_LIMITS.read().map(|l| l.references_cap).unwrap_or(500)
}

/// Get current completion cap
#[inline]
pub fn completion_cap() -> usize {
    LSP_LIMITS.read().map(|l| l.completion_cap).unwrap_or(100)
}

/// Get current reference search deadline
#[inline]
pub fn reference_search_deadline() -> Duration {
    LSP_LIMITS.read().map(|l| l.reference_search_deadline).unwrap_or(Duration::from_secs(2))
}

/// Get current regex scan deadline
#[inline]
pub fn regex_scan_deadline() -> Duration {
    LSP_LIMITS.read().map(|l| l.regex_scan_deadline).unwrap_or(Duration::from_secs(1))
}

/// Get current code lens cap
#[inline]
pub fn code_lens_cap() -> usize {
    LSP_LIMITS.read().map(|l| l.code_lens_cap).unwrap_or(100)
}

/// Get current document symbol cap
#[inline]
pub fn document_symbol_cap() -> usize {
    LSP_LIMITS.read().map(|l| l.document_symbol_cap).unwrap_or(500)
}

/// Get current semantic tokens deadline
#[inline]
pub fn semantic_tokens_deadline() -> Duration {
    LSP_LIMITS.read().map(|l| l.semantic_tokens_deadline).unwrap_or(Duration::from_secs(2))
}

/// Get current code lens resolve deadline
#[inline]
pub fn code_lens_resolve_deadline() -> Duration {
    LSP_LIMITS.read().map(|l| l.code_lens_resolve_deadline).unwrap_or(Duration::from_secs(1))
}

/// Get current completion deadline
#[inline]
pub fn completion_deadline() -> Duration {
    LSP_LIMITS.read().map(|l| l.completion_deadline).unwrap_or(Duration::from_millis(500))
}

/// Get current inlay hints cap
#[inline]
pub fn inlay_hints_cap() -> usize {
    LSP_LIMITS.read().map(|l| l.inlay_hints_cap).unwrap_or(500)
}

/// Get current diagnostics per file cap
#[inline]
pub fn diagnostics_per_file_cap() -> usize {
    LSP_LIMITS.read().map(|l| l.diagnostics_per_file_cap).unwrap_or(200)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_singleton_uses_default_limits() {
        let workspace_cap =
            LSP_LIMITS.read().map(|limits| limits.workspace_symbol_cap).unwrap_or(0);
        assert_eq!(workspace_cap, 200);
    }

    #[test]
    fn accessors_return_expected_defaults() {
        assert_eq!(workspace_symbol_cap(), 200);
        assert_eq!(references_cap(), 500);
        assert_eq!(completion_cap(), 100);
    }
}
