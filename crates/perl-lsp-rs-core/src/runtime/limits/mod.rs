#![warn(missing_docs)]
//! Central configuration for LSP operation limits and bounded behavior
//!
//! This module provides a single source of truth for all resource limits,
//! result caps, and deadlines used throughout the LSP server. This ensures
//! consistent behavior and makes limit tuning straightforward.
//!
//! # Design Goals
//!
//! - **Bounded memory**: All caches have hard caps with LRU eviction
//! - **Bounded latency**: All loops have deadlines to prevent blocking
//! - **Bounded results**: All list operations have caps for client safety
//! - **Graceful degradation**: Exceed limits → degrade, don't crash
//!
//! # Usage
//!
//! ```rust,ignore
//! use perl_lsp_rs_core::runtime::limits::LspLimits;
//!
//! let limits = LspLimits::default();
//! let results = my_query().take(limits.references_result_cap);
//! ```

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// Memory budget configuration for OOM protection.
///
/// Thresholds are approximate: the server tracks explicitly allocated memory
/// rather than querying the OS. Actual RSS is typically 1.5–3x higher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryBudget {
    /// Byte count at which the server enters warning mode (default: 512 MB).
    pub warning_threshold_bytes: usize,
    /// Byte count at which the server enters critical mode (default: 1 GB).
    pub critical_threshold_bytes: usize,
    /// Maximum total bytes in the AST cache (default: 128 MB).
    pub ast_cache_max_bytes: usize,
}

impl Default for MemoryBudget {
    fn default() -> Self {
        Self {
            warning_threshold_bytes: 512 * 1024 * 1024,
            critical_threshold_bytes: 1024 * 1024 * 1024,
            ast_cache_max_bytes: 128 * 1024 * 1024,
        }
    }
}

impl MemoryBudget {
    /// Budget for resource-constrained environments (low-RAM containers).
    pub fn constrained() -> Self {
        Self {
            warning_threshold_bytes: 128 * 1024 * 1024,
            critical_threshold_bytes: 256 * 1024 * 1024,
            ast_cache_max_bytes: 32 * 1024 * 1024,
        }
    }

    /// Budget for large workspaces on developer machines with ample RAM.
    pub fn large_workspace() -> Self {
        Self {
            warning_threshold_bytes: 2 * 1024 * 1024 * 1024,
            critical_threshold_bytes: 4 * 1024 * 1024 * 1024,
            ast_cache_max_bytes: 512 * 1024 * 1024,
        }
    }
}

/// Current memory pressure level for gating degradation behaviors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryPressure {
    /// Memory usage is within normal bounds. No action needed.
    Normal,
    /// Memory usage has exceeded the warning threshold.
    Warning,
    /// Memory usage has exceeded the critical threshold.
    Critical,
}

impl MemoryPressure {
    /// Returns `true` if the server should degrade non-essential work.
    ///
    /// ```
    /// use perl_lsp_rs_core::runtime::limits::{MemoryBudget, MemoryMonitor, MemoryPressure};
    ///
    /// let monitor = MemoryMonitor::new(MemoryBudget::default());
    /// if monitor.pressure().should_degrade() {
    ///     // skip non-essential indexing
    /// }
    /// ```
    #[inline]
    pub fn should_degrade(self) -> bool {
        self >= MemoryPressure::Warning
    }

    /// Returns `true` if the server is in critical memory state.
    #[inline]
    pub fn is_critical(self) -> bool {
        self == MemoryPressure::Critical
    }
}

/// Lightweight approximate memory tracker. Thread-safe via lock-free atomics.
///
/// ```
/// use perl_lsp_rs_core::runtime::limits::{MemoryBudget, MemoryMonitor};
///
/// let monitor = MemoryMonitor::new(MemoryBudget::default());
/// monitor.record_alloc(1024 * 1024);
/// if let Some(msg) = monitor.pressure_log_message() {
///     eprintln!("{}", msg);
/// }
/// monitor.record_free(1024 * 1024);
/// ```
pub struct MemoryMonitor {
    tracked: AtomicUsize,
    budget: MemoryBudget,
}

impl MemoryMonitor {
    /// Create a new monitor with the given budget.
    pub fn new(budget: MemoryBudget) -> Self {
        Self { tracked: AtomicUsize::new(0), budget }
    }

    /// Record that `bytes` have been allocated.
    #[inline]
    pub fn record_alloc(&self, bytes: usize) {
        self.tracked.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record that `bytes` have been freed. Saturates at zero (no underflow).
    #[inline]
    pub fn record_free(&self, bytes: usize) {
        let mut current = self.tracked.load(Ordering::Relaxed);
        loop {
            let new_val = current.saturating_sub(bytes);
            match self.tracked.compare_exchange_weak(
                current,
                new_val,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }
    }

    /// Return the current count of tracked bytes.
    #[inline]
    pub fn tracked_bytes(&self) -> usize {
        self.tracked.load(Ordering::Relaxed)
    }

    /// Return the current memory pressure level.
    pub fn pressure(&self) -> MemoryPressure {
        let bytes = self.tracked_bytes();
        if bytes >= self.budget.critical_threshold_bytes {
            MemoryPressure::Critical
        } else if bytes >= self.budget.warning_threshold_bytes {
            MemoryPressure::Warning
        } else {
            MemoryPressure::Normal
        }
    }

    /// Returns `true` if `proposed_bytes` is within the AST cache memory limit.
    #[inline]
    pub fn ast_cache_has_budget(&self, proposed_bytes: usize) -> bool {
        proposed_bytes <= self.budget.ast_cache_max_bytes
    }

    /// Return a log message for the current pressure, or `None` when normal.
    pub fn pressure_log_message(&self) -> Option<String> {
        let bytes = self.tracked_bytes();
        match self.pressure() {
            MemoryPressure::Normal => None,
            MemoryPressure::Warning => Some(format!(
                "Memory warning: tracked usage {:.1} MB exceeds warning threshold {:.1} MB",
                bytes as f64 / (1024.0 * 1024.0),
                self.budget.warning_threshold_bytes as f64 / (1024.0 * 1024.0),
            )),
            MemoryPressure::Critical => Some(format!(
                "Memory critical: tracked usage {:.1} MB exceeds critical threshold {:.1} MB",
                bytes as f64 / (1024.0 * 1024.0),
                self.budget.critical_threshold_bytes as f64 / (1024.0 * 1024.0),
            )),
        }
    }
}

/// Central configuration for all LSP operation limits
///
/// All handlers should reference these limits rather than defining their own
/// constants. This enables consistent behavior and easy tuning.
#[derive(Debug, Clone)]
pub struct LspLimits {
    // =========================================================================
    // Result Caps
    // =========================================================================
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

    // =========================================================================
    // Cache Limits
    // =========================================================================
    /// Maximum AST cache entries (default: 100)
    pub ast_cache_max_entries: usize,

    /// AST cache TTL in seconds (default: 300 = 5 minutes)
    pub ast_cache_ttl_secs: u64,

    /// Maximum symbol cache entries (default: 1000)
    pub symbol_cache_max_entries: usize,

    // =========================================================================
    // Index Limits
    // =========================================================================
    /// Maximum files to index (default: 10,000)
    pub max_indexed_files: usize,

    /// Maximum symbols per file (default: 5,000)
    pub max_symbols_per_file: usize,

    /// Maximum total symbols in index (default: 500,000)
    pub max_total_symbols: usize,

    /// Parse storm threshold - pending parses before degradation (default: 10)
    pub parse_storm_threshold: usize,

    /// Maximum file size in bytes before skipping parse (default: 1MB)
    ///
    /// Files exceeding this limit will be stored with empty AST and
    /// no diagnostics to prevent the parser from hanging on huge files.
    pub max_file_size_bytes: usize,

    // =========================================================================
    // Deadlines
    // =========================================================================
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

    // =========================================================================
    // Degradation Behavior
    // =========================================================================
    /// Whether to return partial results on timeout (default: true)
    pub return_partial_on_timeout: bool,

    /// Whether to include open documents when index is degraded (default: true)
    pub include_open_docs_when_degraded: bool,

    // =========================================================================
    // Memory Budget
    // =========================================================================
    /// Memory thresholds for OOM protection and degradation mode.
    ///
    /// See [`MemoryBudget`] for field documentation and tuning guidance.
    pub memory_budget: MemoryBudget,
}

impl Default for LspLimits {
    fn default() -> Self {
        Self {
            // Result caps
            workspace_symbol_cap: 200,
            references_cap: 500,
            completion_cap: 100,
            document_symbol_cap: 500,
            code_lens_cap: 100,
            diagnostics_per_file_cap: 200,
            inlay_hints_cap: 500,

            // Cache limits
            ast_cache_max_entries: 100,
            ast_cache_ttl_secs: 300,
            symbol_cache_max_entries: 1000,

            // Index limits
            max_indexed_files: 10_000,
            max_symbols_per_file: 5_000,
            max_total_symbols: 500_000,
            parse_storm_threshold: 10,
            max_file_size_bytes: 1_024 * 1_024, // 1MB

            // Deadlines
            workspace_scan_deadline: Duration::from_secs(30),
            file_index_deadline: Duration::from_secs(5),
            reference_search_deadline: Duration::from_secs(2),
            regex_scan_deadline: Duration::from_secs(1),
            fs_operation_deadline: Duration::from_millis(500),
            semantic_tokens_deadline: Duration::from_secs(2),
            code_lens_resolve_deadline: Duration::from_secs(1),
            completion_deadline: Duration::from_millis(500),

            // Degradation behavior
            return_partial_on_timeout: true,
            include_open_docs_when_degraded: true,

            // Memory budget
            memory_budget: MemoryBudget::default(),
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
            memory_budget: MemoryBudget::large_workspace(),
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
            memory_budget: MemoryBudget::constrained(),
            ..Default::default()
        }
    }

    /// Update limits from LSP settings
    ///
    /// Reads from the `perl.limits` section of settings.
    pub fn update_from_value(&mut self, settings: &serde_json::Value) {
        if let Some(limits) = settings.get("limits") {
            // Result caps
            if let Some(v) = limits.get("workspaceSymbolCap").and_then(|v| v.as_u64()) {
                self.workspace_symbol_cap = v as usize;
            }
            if let Some(v) = limits.get("referencesCap").and_then(|v| v.as_u64()) {
                self.references_cap = v as usize;
            }
            if let Some(v) = limits.get("completionCap").and_then(|v| v.as_u64()) {
                self.completion_cap = v as usize;
            }

            // Cache limits
            if let Some(v) = limits.get("astCacheMaxEntries").and_then(|v| v.as_u64()) {
                self.ast_cache_max_entries = v as usize;
            }

            // Index limits
            if let Some(v) = limits.get("maxIndexedFiles").and_then(|v| v.as_u64()) {
                self.max_indexed_files = v as usize;
            }
            if let Some(v) = limits.get("maxTotalSymbols").and_then(|v| v.as_u64()) {
                self.max_total_symbols = v as usize;
            }

            // File size limit
            if let Some(v) = limits.get("maxFileSizeBytes").and_then(|v| v.as_u64()) {
                self.max_file_size_bytes = v as usize;
            }

            // Deadlines (in milliseconds)
            if let Some(v) = limits.get("workspaceScanDeadlineMs").and_then(|v| v.as_u64()) {
                self.workspace_scan_deadline = Duration::from_millis(v);
            }
            if let Some(v) = limits.get("referenceSearchDeadlineMs").and_then(|v| v.as_u64()) {
                self.reference_search_deadline = Duration::from_millis(v);
            }

            // Memory budget
            if let Some(v) = limits.get("memoryWarningThresholdBytes").and_then(|v| v.as_u64()) {
                self.memory_budget.warning_threshold_bytes = v as usize;
            }
            if let Some(v) = limits.get("memoryCriticalThresholdBytes").and_then(|v| v.as_u64()) {
                self.memory_budget.critical_threshold_bytes = v as usize;
            }
            if let Some(v) = limits.get("astCacheMaxMemoryBytes").and_then(|v| v.as_u64()) {
                self.memory_budget.ast_cache_max_bytes = v as usize;
            }
        }
    }
}

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

/// Get current maximum file size in bytes
#[inline]
pub fn max_file_size_bytes() -> usize {
    LSP_LIMITS.read().map(|l| l.max_file_size_bytes).unwrap_or(1_024 * 1_024)
}

/// Get current memory warning threshold in bytes
#[inline]
pub fn memory_warning_threshold_bytes() -> usize {
    LSP_LIMITS.read().map(|l| l.memory_budget.warning_threshold_bytes).unwrap_or(512 * 1024 * 1024)
}

/// Get current memory critical threshold in bytes
#[inline]
pub fn memory_critical_threshold_bytes() -> usize {
    LSP_LIMITS
        .read()
        .map(|l| l.memory_budget.critical_threshold_bytes)
        .unwrap_or(1024 * 1024 * 1024)
}

/// Get current AST cache maximum memory in bytes
#[inline]
pub fn ast_cache_max_memory_bytes() -> usize {
    LSP_LIMITS.read().map(|l| l.memory_budget.ast_cache_max_bytes).unwrap_or(128 * 1024 * 1024)
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
        assert_eq!(limits.max_file_size_bytes, 1_024 * 1_024);
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
