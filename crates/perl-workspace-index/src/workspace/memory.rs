//! Memory profiling infrastructure for large workspace symbol resolution.
//!
//! This module provides pure-Rust memory estimation for the workspace index,
//! enabling baseline documentation and scaling analysis without external
//! allocator tooling (heaptrack, valgrind, etc.).
//!
//! # Usage
//!
//! ```rust,ignore
//! use perl_workspace::workspace::memory::MemorySnapshot;
//! use perl_workspace::workspace_index::WorkspaceIndex;
//!
//! let index = WorkspaceIndex::new();
//! // ... index files ...
//! let snap = MemorySnapshot::capture(&index);
//! println!("{}", snap.to_report_string());
//! ```
//!
//! # Limitations
//!
//! Estimates use `String::len()` for heap strings and `std::mem::size_of` for
//! stack-allocated struct fields. They do not account for allocator overhead,
//! heap fragmentation, or internal `HashMap` bucket arrays. For precise
//! measurement, pair with `dhat` (Linux) or platform heap profilers.
//!
//! # Feature Gate
//!
//! This module is only compiled when the `memory-profiling` feature is enabled.

use super::workspace_index::WorkspaceIndex;
use std::fmt;

/// A point-in-time estimate of workspace index memory usage.
///
/// Sizes are approximations based on summing the byte lengths of all
/// heap-allocated `String` values and the `size_of` stack portions of
/// composite structs. HashMap bucket overhead and allocator padding are
/// not included.
///
/// Use [`MemorySnapshot::capture`] to snapshot a live [`WorkspaceIndex`].
#[derive(Clone, Debug, Default)]
pub struct MemorySnapshot {
    /// Number of indexed files at capture time.
    pub file_count: usize,

    /// Total number of symbols across all files at capture time.
    pub symbol_count: usize,

    /// Estimated bytes used by the per-file data (symbols, references,
    /// dependencies, content hashes).
    pub files_bytes: usize,

    /// Estimated bytes used by the global qualified-name to URI symbol map.
    pub symbols_bytes: usize,

    /// Estimated bytes used by the global reference index.
    pub global_refs_bytes: usize,

    /// Estimated bytes used by the document store (raw source texts).
    pub document_store_bytes: usize,
}

impl MemorySnapshot {
    /// Capture a memory snapshot of the current [`WorkspaceIndex`] state.
    ///
    /// This acquires read locks on all index components and walks their
    /// contents to estimate heap usage. Intended for offline profiling;
    /// do not call on the LSP hot path.
    pub fn capture(index: &WorkspaceIndex) -> Self {
        index.memory_snapshot()
    }

    /// Total estimated bytes across all components.
    pub fn total_estimated_bytes(&self) -> usize {
        self.files_bytes + self.symbols_bytes + self.global_refs_bytes + self.document_store_bytes
    }

    /// Estimated bytes per indexed symbol.
    ///
    /// Returns 0 when no symbols are indexed.
    pub fn bytes_per_symbol(&self) -> usize {
        if self.symbol_count == 0 {
            return 0;
        }
        self.total_estimated_bytes() / self.symbol_count
    }

    /// Estimated bytes per indexed file.
    ///
    /// Returns 0 when no files are indexed.
    pub fn bytes_per_file(&self) -> usize {
        if self.file_count == 0 {
            return 0;
        }
        self.total_estimated_bytes() / self.file_count
    }

    /// Format this snapshot as a human-readable report string.
    pub fn to_report_string(&self) -> String {
        let total = self.total_estimated_bytes();
        format!(
            "MemorySnapshot {{ files: {} ({} B), symbols map: {} ({} B), \
             global_refs: {} B, doc_store: {} B, total: {} B, \
             {} symbols, {} B/symbol }}",
            self.file_count,
            self.files_bytes,
            self.symbol_count,
            self.symbols_bytes,
            self.global_refs_bytes,
            self.document_store_bytes,
            total,
            self.symbol_count,
            self.bytes_per_symbol(),
        )
    }
}

impl fmt::Display for MemorySnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_report_string())
    }
}

/// A collection of [`MemorySnapshot`]s at increasing file-count checkpoints.
///
/// Used to characterise scaling behaviour (linear, sub-linear, or unexpected
/// growth) without external tooling.
///
/// # Example
///
/// ```rust,ignore
/// use perl_workspace::workspace::memory::{MemorySnapshot, ScaleReport};
/// use perl_workspace::workspace_index::WorkspaceIndex;
///
/// let mut report = ScaleReport::new();
/// for scale in [100, 500, 1000] {
///     let index = WorkspaceIndex::new();
///     // ... index `scale` files ...
///     report.add_checkpoint(scale, MemorySnapshot::capture(&index));
/// }
/// println!("{}", report);
/// ```
#[derive(Debug, Default)]
pub struct ScaleReport {
    /// Ordered list of (file_count, snapshot) pairs.
    checkpoints: Vec<(usize, MemorySnapshot)>,
}

impl ScaleReport {
    /// Create an empty scale report.
    pub fn new() -> Self {
        Self {
            checkpoints: Vec::new(),
        }
    }

    /// Add a checkpoint with the nominal file count and its snapshot.
    pub fn add_checkpoint(&mut self, file_count: usize, snap: MemorySnapshot) {
        self.checkpoints.push((file_count, snap));
    }

    /// Return a slice of all checkpoints.
    pub fn checkpoints(&self) -> &[(usize, MemorySnapshot)] {
        &self.checkpoints
    }

    /// Compute a rough scaling factor between the first and last checkpoint.
    ///
    /// Returns `None` if fewer than two checkpoints exist or memory is zero.
    pub fn scaling_factor(&self) -> Option<f64> {
        if self.checkpoints.len() < 2 {
            return None;
        }
        let first = self.checkpoints.first()?;
        let last = self.checkpoints.last()?;

        let mem_first = first.1.total_estimated_bytes();
        let mem_last = last.1.total_estimated_bytes();
        let file_first = first.0;
        let file_last = last.0;

        if mem_first == 0 || file_first == 0 || file_last == 0 {
            return None;
        }

        // Expected linear memory at last scale = mem_first * (file_last / file_first)
        // Actual / expected gives the scaling factor relative to linear
        let expected_linear = mem_first as f64 * (file_last as f64 / file_first as f64);
        Some(mem_last as f64 / expected_linear)
    }
}

impl fmt::Display for ScaleReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{:<10} {:>12} {:>12} {:>12} {:>14}",
            "files", "total_B", "symbols", "B/sym", "B/file"
        )?;
        writeln!(f, "{}", "-".repeat(64))?;
        for (file_count, snap) in &self.checkpoints {
            writeln!(
                f,
                "{:<10} {:>12} {:>12} {:>12} {:>14}",
                file_count,
                snap.total_estimated_bytes(),
                snap.symbol_count,
                snap.bytes_per_symbol(),
                snap.bytes_per_file(),
            )?;
        }
        if let Some(factor) = self.scaling_factor() {
            writeln!(f, "\nScaling factor vs linear: {:.2}x", factor)?;
        }
        Ok(())
    }
}
