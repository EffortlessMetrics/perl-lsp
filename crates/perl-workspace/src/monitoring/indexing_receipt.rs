use super::EarlyExitReason;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::Duration;

const SLOWEST_FILE_LIMIT: usize = 10;

/// Coarse phase boundaries currently observable by the startup indexer.
///
/// The workspace index API currently performs parsing, declaration/reference
/// extraction, and commit work inside one `index_file` call.  That bundled
/// operation is kept explicit here so receipts do not claim a finer split than
/// the producer can prove.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndexingPhase {
    /// Discover candidate workspace files.
    Discovery,
    /// Read one workspace file from disk.
    Read,
    /// Parse/extract/commit one file through the current index API boundary.
    IndexFileOperation,
}

impl IndexingPhase {
    fn field_name(self) -> &'static str {
        match self {
            Self::Discovery => "discovery_us",
            Self::Read => "read_us",
            Self::IndexFileOperation => "index_file_operation_us",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct WorkspaceIndexingFileTiming {
    path: PathBuf,
    read_ms: u64,
    index_ms: u64,
    total_ms: u64,
}

/// Per-run receipt for startup workspace indexing.
///
/// This records the first-hour evidence that coarse phase transitions cannot
/// answer: read cost, indexing cost, throughput, skipped/error counts, and the
/// slowest files in the run.
///
/// The legacy `*_ms` fields are retained for compatibility with existing
/// consumers and slow-file summaries. `phase_us` is the finer-grained phase
/// view. These views have different aggregation boundaries and must not be
/// added together; consumers should choose one representation for a timing
/// calculation. In particular, `read_ms` and `index_ms` only include
/// successfully indexed files, while `phase_us` also records measured phase
/// work for files that later fail.
#[non_exhaustive]
#[derive(Debug, Default)]
pub struct WorkspaceIndexingReceipt {
    discovered_files: usize,
    indexed_files: usize,
    read_errors: usize,
    index_errors: usize,
    discovery_ms: u64,
    read_ms: u64,
    index_ms: u64,
    phase_us: std::collections::BTreeMap<&'static str, u64>,
    slowest_files: Vec<WorkspaceIndexingFileTiming>,
}

impl WorkspaceIndexingReceipt {
    /// Record the discovery phase result and elapsed duration.
    pub fn record_discovery(&mut self, discovered_files: usize, elapsed: Duration) {
        self.discovered_files = discovered_files;
        self.discovery_ms = duration_ms(elapsed);
    }

    /// Record a measured startup phase without inventing sub-phase precision.
    pub fn record_phase(&mut self, phase: IndexingPhase, elapsed: Duration) {
        let elapsed_us = duration_us(elapsed);
        let entry = self.phase_us.entry(phase.field_name()).or_default();
        *entry = entry.saturating_add(elapsed_us);
    }

    /// Record a file read failure during indexing.
    pub fn record_read_error(&mut self) {
        self.read_errors += 1;
    }

    /// Record an indexing failure after a file was discovered.
    pub fn record_index_error(&mut self) {
        self.index_errors += 1;
    }

    /// Record the read and index timing for one successfully indexed file.
    pub fn record_indexed_file(
        &mut self,
        path: &Path,
        read_elapsed: Duration,
        index_elapsed: Duration,
    ) {
        let read_ms = duration_ms(read_elapsed);
        let index_ms = duration_ms(index_elapsed);
        let total_ms = read_ms.saturating_add(index_ms);

        self.indexed_files += 1;
        self.read_ms = self.read_ms.saturating_add(read_ms);
        self.index_ms = self.index_ms.saturating_add(index_ms);

        self.slowest_files.push(WorkspaceIndexingFileTiming {
            path: path.to_path_buf(),
            read_ms,
            index_ms,
            total_ms,
        });
        self.slowest_files.sort_by(|left, right| {
            right.total_ms.cmp(&left.total_ms).then_with(|| left.path.cmp(&right.path))
        });
        if self.slowest_files.len() > SLOWEST_FILE_LIMIT {
            self.slowest_files.truncate(SLOWEST_FILE_LIMIT);
        }
    }

    /// Emit a structured tracing receipt for the completed indexing run.
    pub fn log(&self, total_elapsed: Duration, early_exit: Option<EarlyExitReason>) {
        let receipt = self.summary_json(total_elapsed, early_exit);
        tracing::info!(
            target: "perl_lsp::workspace_indexing",
            receipt = %receipt,
            "Workspace indexing receipt"
        );

        for (rank, file) in self.slowest_files.iter().enumerate() {
            tracing::debug!(
                target: "perl_lsp::workspace_indexing",
                rank = rank + 1,
                path = %file.path.display(),
                read_ms = file.read_ms,
                index_ms = file.index_ms,
                total_ms = file.total_ms,
                "Slow workspace indexing file"
            );
        }
    }

    fn summary_json(&self, total_elapsed: Duration, early_exit: Option<EarlyExitReason>) -> Value {
        let total_elapsed_ms = duration_ms(total_elapsed);
        json!({
            "discovered_files": self.discovered_files,
            "indexed_files": self.indexed_files,
            "read_errors": self.read_errors,
            "index_errors": self.index_errors,
            "discovery_ms": self.discovery_ms,
            "read_ms": self.read_ms,
            "index_ms": self.index_ms,
            "phase_us": self.phase_us,
            "total_elapsed_ms": total_elapsed_ms,
            "files_per_second": files_per_second(self.indexed_files, total_elapsed),
            "early_exit": early_exit.map(|reason| format!("{reason:?}")),
            "slowest_files": self
                .slowest_files
                .iter()
                .map(|file| {
                    json!({
                        "path": file.path.display().to_string(),
                        "read_ms": file.read_ms,
                        "index_ms": file.index_ms,
                        "total_ms": file.total_ms,
                    })
                })
                .collect::<Vec<_>>(),
        })
    }
}

fn duration_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn duration_us(duration: Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

fn files_per_second(indexed_files: usize, total_elapsed: Duration) -> f64 {
    let elapsed_secs = total_elapsed.as_secs_f64();
    files_per_second_from_elapsed_secs(indexed_files, elapsed_secs)
}

fn files_per_second_from_elapsed_secs(indexed_files: usize, elapsed_secs: f64) -> f64 {
    if elapsed_secs <= f64::EPSILON {
        return indexed_files as f64;
    }
    indexed_files as f64 / elapsed_secs
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{Result, anyhow};

    #[test]
    fn receipt_keeps_top_ten_slowest_files_by_total_duration() -> Result<()> {
        let mut receipt = WorkspaceIndexingReceipt::default();

        for i in 0..12 {
            receipt.record_indexed_file(
                Path::new(&format!("lib/File{i}.pm")),
                Duration::from_millis(i),
                Duration::from_millis(20 - i),
            );
        }
        receipt.record_indexed_file(
            Path::new("lib/Slow.pm"),
            Duration::from_millis(9),
            Duration::from_millis(99),
        );

        assert_eq!(receipt.slowest_files.len(), SLOWEST_FILE_LIMIT);
        let first = receipt.slowest_files.first().ok_or_else(|| anyhow!("missing slowest file"))?;
        assert_eq!(first.path, PathBuf::from("lib/Slow.pm"));
        assert_eq!(first.total_ms, 108);
        assert!(
            receipt.slowest_files.iter().all(|file| file.total_ms >= 20),
            "fast files should be trimmed from the bounded receipt"
        );

        Ok(())
    }

    #[test]
    fn record_indexed_file_boundary_discriminator_when_slowest_files_len_exceeds_limit()
    -> Result<()> {
        let mut receipt = WorkspaceIndexingReceipt::default();
        let input_that_hits_boundary = SLOWEST_FILE_LIMIT + 1;

        assert!(input_that_hits_boundary > SLOWEST_FILE_LIMIT);

        for i in 0..input_that_hits_boundary {
            receipt.record_indexed_file(
                Path::new(&format!("lib/Boundary{i}.pm")),
                Duration::from_millis(1),
                Duration::from_millis(u64::try_from(i)?),
            );
        }

        assert_eq!(receipt.slowest_files.len(), SLOWEST_FILE_LIMIT);
        assert!(
            receipt.slowest_files.iter().all(|file| file.path != Path::new("lib/Boundary0.pm")),
            "the fastest timing should be trimmed when the limit is exceeded"
        );

        Ok(())
    }

    #[test]
    fn files_per_second_call_presence_observer() {
        let throughput = files_per_second(2, Duration::from_millis(500));

        assert_eq!(throughput, 4.0);
    }

    #[test]
    fn files_per_second_boundary_discriminator() {
        let throughput = files_per_second(7, Duration::ZERO);

        assert_eq!(throughput, 7.0);
    }

    #[test]
    fn files_per_second_epsilon_boundary_discriminator() {
        let throughput = files_per_second_from_elapsed_secs(7, f64::EPSILON);

        assert_eq!(throughput, 7.0);
    }

    #[test]
    fn log_call_presence_observer_for_summary_receipt() {
        let receipt = WorkspaceIndexingReceipt::default();

        receipt.log(Duration::from_millis(1), Some(EarlyExitReason::FileLimit));

        let summary =
            receipt.summary_json(Duration::from_millis(1), Some(EarlyExitReason::FileLimit));
        assert_eq!(summary["indexed_files"], 0);
        assert_eq!(summary["early_exit"], "FileLimit");
    }

    #[test]
    fn log_call_presence_observer_for_slowest_files() {
        let mut receipt = WorkspaceIndexingReceipt::default();
        receipt.record_indexed_file(
            Path::new("lib/Observed.pm"),
            Duration::from_millis(2),
            Duration::from_millis(3),
        );

        receipt.log(Duration::from_millis(10), None);

        let summary = receipt.summary_json(Duration::from_millis(10), None);
        assert_eq!(summary["slowest_files"][0]["path"], "lib/Observed.pm");
        assert_eq!(summary["slowest_files"][0]["total_ms"], 5);
    }

    #[test]
    fn summary_json_reports_phase_totals_and_early_exit() -> Result<()> {
        let mut receipt = WorkspaceIndexingReceipt::default();
        receipt.record_discovery(3, Duration::from_millis(4));
        receipt.record_phase(IndexingPhase::Discovery, Duration::from_micros(4_500));
        receipt.record_phase(IndexingPhase::Read, Duration::from_micros(1_250));
        receipt.record_phase(IndexingPhase::IndexFileOperation, Duration::from_micros(8_750));
        receipt.record_read_error();
        receipt.record_index_error();
        receipt.record_indexed_file(
            Path::new("lib/App.pm"),
            Duration::from_millis(5),
            Duration::from_millis(7),
        );

        let summary = receipt
            .summary_json(Duration::from_millis(20), Some(EarlyExitReason::InitialTimeBudget));

        assert_eq!(summary["discovered_files"], 3);
        assert_eq!(summary["indexed_files"], 1);
        assert_eq!(summary["read_errors"], 1);
        assert_eq!(summary["index_errors"], 1);
        assert_eq!(summary["discovery_ms"], 4);
        assert_eq!(summary["read_ms"], 5);
        assert_eq!(summary["index_ms"], 7);
        assert_eq!(summary["total_elapsed_ms"], 20);
        assert_eq!(summary["early_exit"], "InitialTimeBudget");
        assert_eq!(summary["slowest_files"][0]["path"], "lib/App.pm");
        assert_eq!(summary["phase_us"]["discovery_us"], 4_500);
        assert_eq!(summary["phase_us"]["read_us"], 1_250);
        assert_eq!(summary["phase_us"]["index_file_operation_us"], 8_750);
        Ok(())
    }
}
