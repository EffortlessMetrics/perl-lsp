//! Report generation for corpus audit
//!
//! This module generates a comprehensive machine-readable report
//! containing all audit findings.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

use super::corpus::{CorpusFile, CorpusInventory};
use super::ga_alignment::GAFeatureCoverage;
use super::nodekind_analysis::NodeKindStats;
use super::timeout_detection::{ParseOutcome, TimeoutRisk};

/// Comprehensive audit report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    /// Report metadata
    pub metadata: ReportMetadata,
    /// Corpus inventory
    pub inventory: CorpusInventory,
    /// Parse outcomes summary
    pub parse_outcomes: ParseOutcomesSummary,
    /// NodeKind coverage statistics
    pub nodekind_coverage: NodeKindStats,
    /// GA feature coverage
    pub ga_coverage: GAFeatureCoverage,
    /// Timeout/hang risks
    pub timeout_risks: Vec<TimeoutRisk>,
}

/// Report metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMetadata {
    /// Report generation timestamp
    pub generated_at: String,
    /// Report version
    pub version: String,
    /// Audit duration in seconds
    pub duration_secs: u64,
}

/// Summary of parse outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseOutcomesSummary {
    /// Total files parsed
    pub total: usize,
    /// Successfully parsed files
    pub ok: usize,
    /// Files with parse errors
    pub error: usize,
    /// Files that timed out
    pub timeout: usize,
    /// Files that caused panics
    pub panic: usize,
}

/// Generate a comprehensive audit report
pub fn generate_report(
    corpus_files: Vec<CorpusFile>,
    parse_results: std::collections::HashMap<PathBuf, ParseOutcome>,
    nodekind_stats: NodeKindStats,
    ga_coverage: GAFeatureCoverage,
    timeout_risks: Vec<TimeoutRisk>,
    duration: Duration,
) -> AuditReport {
    // Generate inventory
    let inventory = super::corpus::generate_inventory(&corpus_files);

    // Generate parse outcomes summary
    let parse_outcomes = generate_parse_outcomes_summary(&parse_results);

    // Generate metadata
    let metadata = ReportMetadata {
        generated_at: Utc::now().to_rfc3339(),
        version: "0.1.0".to_string(),
        duration_secs: duration.as_secs(),
    };

    AuditReport {
        metadata,
        inventory,
        parse_outcomes,
        nodekind_coverage: nodekind_stats,
        ga_coverage,
        timeout_risks,
    }
}

/// Generate parse outcomes summary from parse results
fn generate_parse_outcomes_summary(
    parse_results: &std::collections::HashMap<PathBuf, ParseOutcome>,
) -> ParseOutcomesSummary {
    let total = parse_results.len();
    let mut ok = 0;
    let mut error = 0;
    let mut timeout = 0;
    let mut panic = 0;

    for outcome in parse_results.values() {
        match outcome {
            ParseOutcome::Ok { .. } => ok += 1,
            ParseOutcome::Error { .. } => error += 1,
            ParseOutcome::Timeout { .. } => timeout += 1,
            ParseOutcome::Panic { .. } => panic += 1,
        }
    }

    ParseOutcomesSummary {
        total,
        ok,
        error,
        timeout,
        panic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_parse_outcomes_summary() {
        let mut results = std::collections::HashMap::new();
        results.insert(
            PathBuf::from("test1.pl"),
            ParseOutcome::Ok { duration_ms: 100 },
        );
        results.insert(
            PathBuf::from("test2.pl"),
            ParseOutcome::Error {
                message: "error".to_string(),
            },
        );
        results.insert(
            PathBuf::from("test3.pl"),
            ParseOutcome::Timeout { timeout_ms: 1000 },
        );
        results.insert(
            PathBuf::from("test4.pl"),
            ParseOutcome::Panic {
                message: "panic".to_string(),
            },
        );

        let summary = generate_parse_outcomes_summary(&results);

        assert_eq!(summary.total, 4);
        assert_eq!(summary.ok, 1);
        assert_eq!(summary.error, 1);
        assert_eq!(summary.timeout, 1);
        assert_eq!(summary.panic, 1);
    }

    #[test]
    fn test_report_metadata() {
        let metadata = ReportMetadata {
            generated_at: "2025-01-07T10:00:00Z".to_string(),
            version: "0.1.0".to_string(),
            duration_secs: 30,
        };

        assert_eq!(metadata.version, "0.1.0");
        assert_eq!(metadata.duration_secs, 30);
    }
}
