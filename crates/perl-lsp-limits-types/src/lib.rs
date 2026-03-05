//! Configuration model for LSP operation limits and bounded behavior.
//!
//! This crate contains the `LspLimits` data shape and profile constructors,
//! while runtime/global state wrappers are provided by `perl-lsp-limits`.

use std::time::Duration;

/// Central configuration for all LSP operation limits
///
/// All handlers should reference these limits rather than defining their own
/// constants. This enables consistent behavior and easy tuning.
#[derive(Debug, Clone)]
pub struct LspLimits {
    /// Maximum workspace/symbol results (default: 200)
    pub workspace_symbol_cap: usize,
    /// Maximum textDocument/references results (default: 500)
    pub references_cap: usize,
    /// Maximum textDocument/completion results (default: 100)
    pub completion_cap: usize,
    /// Maximum textDocument/documentSymbol results (default: 500)
    pub document_symbol_cap: usize,
    /// Maximum textDocument/codeLens results (default: 100)
    pub code_lens_cap: usize,
    /// Maximum diagnostics per file (default: 200)
    pub diagnostics_per_file_cap: usize,
    /// Maximum inlay hints per file (default: 500)
    pub inlay_hints_cap: usize,

    /// Maximum AST cache entries (default: 100)
    pub ast_cache_max_entries: usize,
    /// AST cache TTL in seconds (default: 300 = 5 minutes)
    pub ast_cache_ttl_secs: u64,
    /// Maximum symbol cache entries (default: 1000)
    pub symbol_cache_max_entries: usize,

    /// Maximum files to index (default: 10,000)
    pub max_indexed_files: usize,
    /// Maximum symbols per file (default: 5,000)
    pub max_symbols_per_file: usize,
    /// Maximum total symbols in index (default: 500,000)
    pub max_total_symbols: usize,
    /// Parse storm threshold - pending parses before degradation (default: 10)
    pub parse_storm_threshold: usize,

    /// Deadline for workspace folder scan (default: 30s)
    pub workspace_scan_deadline: Duration,
    /// Deadline for single file indexing (default: 5s)
    pub file_index_deadline: Duration,
    /// Deadline for reference search across workspace (default: 2s)
    pub reference_search_deadline: Duration,
    /// Deadline for regex scan operations (default: 1s)
    pub regex_scan_deadline: Duration,
    /// Deadline for filesystem operations (default: 500ms)
    pub fs_operation_deadline: Duration,
    /// Deadline for semantic tokens computation (default: 2s)
    pub semantic_tokens_deadline: Duration,
    /// Deadline for code lens resolve operations (default: 1s)
    pub code_lens_resolve_deadline: Duration,
    /// Deadline for completion operations (default: 500ms)
    pub completion_deadline: Duration,

    /// Whether to return partial results on timeout (default: true)
    pub return_partial_on_timeout: bool,
    /// Whether to include open documents when index is degraded (default: true)
    pub include_open_docs_when_degraded: bool,
}

impl Default for LspLimits {
    fn default() -> Self {
        Self {
            workspace_symbol_cap: 200,
            references_cap: 500,
            completion_cap: 100,
            document_symbol_cap: 500,
            code_lens_cap: 100,
            diagnostics_per_file_cap: 200,
            inlay_hints_cap: 500,
            ast_cache_max_entries: 100,
            ast_cache_ttl_secs: 300,
            symbol_cache_max_entries: 1000,
            max_indexed_files: 10_000,
            max_symbols_per_file: 5_000,
            max_total_symbols: 500_000,
            parse_storm_threshold: 10,
            workspace_scan_deadline: Duration::from_secs(30),
            file_index_deadline: Duration::from_secs(5),
            reference_search_deadline: Duration::from_secs(2),
            regex_scan_deadline: Duration::from_secs(1),
            fs_operation_deadline: Duration::from_millis(500),
            semantic_tokens_deadline: Duration::from_secs(2),
            code_lens_resolve_deadline: Duration::from_secs(1),
            completion_deadline: Duration::from_millis(500),
            return_partial_on_timeout: true,
            include_open_docs_when_degraded: true,
        }
    }
}

impl LspLimits {
    /// Create limits optimized for large workspaces (10K+ files)
    pub fn large_workspace() -> Self {
        Self {
            max_indexed_files: 50_000,
            max_total_symbols: 2_000_000,
            workspace_scan_deadline: Duration::from_secs(120),
            ..Default::default()
        }
    }

    /// Create limits optimized for resource-constrained environments
    pub fn constrained() -> Self {
        Self {
            ast_cache_max_entries: 50,
            max_indexed_files: 5_000,
            max_total_symbols: 100_000,
            workspace_scan_deadline: Duration::from_secs(15),
            reference_search_deadline: Duration::from_secs(1),
            ..Default::default()
        }
    }

    /// Update limits from LSP settings.
    ///
    /// Reads from the `perl.limits` section of settings.
    pub fn update_from_value(&mut self, settings: &serde_json::Value) {
        if let Some(limits) = settings.get("limits") {
            if let Some(v) = limits.get("workspaceSymbolCap").and_then(|v| v.as_u64()) {
                self.workspace_symbol_cap = v as usize;
            }
            if let Some(v) = limits.get("referencesCap").and_then(|v| v.as_u64()) {
                self.references_cap = v as usize;
            }
            if let Some(v) = limits.get("completionCap").and_then(|v| v.as_u64()) {
                self.completion_cap = v as usize;
            }
            if let Some(v) = limits.get("astCacheMaxEntries").and_then(|v| v.as_u64()) {
                self.ast_cache_max_entries = v as usize;
            }
            if let Some(v) = limits.get("maxIndexedFiles").and_then(|v| v.as_u64()) {
                self.max_indexed_files = v as usize;
            }
            if let Some(v) = limits.get("maxTotalSymbols").and_then(|v| v.as_u64()) {
                self.max_total_symbols = v as usize;
            }
            if let Some(v) = limits.get("workspaceScanDeadlineMs").and_then(|v| v.as_u64()) {
                self.workspace_scan_deadline = Duration::from_millis(v);
            }
            if let Some(v) = limits.get("referenceSearchDeadlineMs").and_then(|v| v.as_u64()) {
                self.reference_search_deadline = Duration::from_millis(v);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_limits() {
        let limits = LspLimits::default();
        assert_eq!(limits.workspace_symbol_cap, 200);
        assert_eq!(limits.references_cap, 500);
        assert_eq!(limits.max_indexed_files, 10_000);
    }

    #[test]
    fn test_large_workspace_limits() {
        let limits = LspLimits::large_workspace();
        assert_eq!(limits.max_indexed_files, 50_000);
        assert_eq!(limits.max_total_symbols, 2_000_000);
    }

    #[test]
    fn test_constrained_limits() {
        let limits = LspLimits::constrained();
        assert_eq!(limits.max_indexed_files, 5_000);
        assert_eq!(limits.ast_cache_max_entries, 50);
    }

    #[test]
    fn test_update_from_value() {
        let mut limits = LspLimits::default();
        let settings = serde_json::json!({
            "limits": {
                "workspaceSymbolCap": 300,
                "maxIndexedFiles": 20000
            }
        });
        limits.update_from_value(&settings);
        assert_eq!(limits.workspace_symbol_cap, 300);
        assert_eq!(limits.max_indexed_files, 20_000);
    }
}
