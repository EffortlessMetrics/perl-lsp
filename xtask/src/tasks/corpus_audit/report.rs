//! Report generation for corpus audit
//!
//! This module generates a comprehensive machine-readable report
//! containing all audit findings.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use super::corpus::{CorpusFile, CorpusInventory};
use super::ga_alignment::GAFeatureCoverage;
use super::nodekind_analysis::NodeKindStats;
use super::timeout_detection::{categorize_error, ParseOutcome, TimeoutRisk};

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
    /// Error breakdown by category (Issue #180)
    #[serde(default)]
    pub error_by_category: HashMap<String, usize>,
    /// List of failing files with details (Issue #180)
    #[serde(default)]
    pub failing_files: Vec<FailingFile>,
}

/// Details about a file that failed to parse (Issue #180)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailingFile {
    /// Path to the failing file
    pub path: String,
    /// Error category for targeting improvements
    pub category: String,
    /// Error message
    pub error_message: String,
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

    // Build source content map for error categorization
    let sources: HashMap<PathBuf, String> = corpus_files
        .iter()
        .map(|f| (f.path.clone(), f.content.clone()))
        .collect();

    // Generate parse outcomes summary with category breakdown
    let parse_outcomes = generate_parse_outcomes_summary(&parse_results, &sources);

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
    sources: &HashMap<PathBuf, String>,
) -> ParseOutcomesSummary {
    let total = parse_results.len();
    let mut ok = 0;
    let mut error = 0;
    let mut timeout = 0;
    let mut panic = 0;
    let mut error_by_category: HashMap<String, usize> = HashMap::new();
    let mut failing_files: Vec<FailingFile> = Vec::new();

    for (path, outcome) in parse_results {
        match outcome {
            ParseOutcome::Ok { .. } => ok += 1,
            ParseOutcome::Error { message } => {
                error += 1;

                // Categorize the error
                let source = sources.get(path).map(|s| s.as_str()).unwrap_or("");
                let category = categorize_error(message, source);
                let category_str = category.to_string();

                // Update category counts
                *error_by_category.entry(category_str.clone()).or_insert(0) += 1;

                // Add to failing files list
                failing_files.push(FailingFile {
                    path: path.display().to_string(),
                    category: category_str,
                    error_message: message.clone(),
                });
            }
            ParseOutcome::Timeout { .. } => timeout += 1,
            ParseOutcome::Panic { message } => {
                panic += 1;
                // Panics are also added to failing files for visibility
                failing_files.push(FailingFile {
                    path: path.display().to_string(),
                    category: "Panic".to_string(),
                    error_message: message.clone(),
                });
            }
        }
    }

    // Sort failing files by path for consistent output
    failing_files.sort_by(|a, b| a.path.cmp(&b.path));

    ParseOutcomesSummary { total, ok, error, timeout, panic, error_by_category, failing_files }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_parse_outcomes_summary() {
        let mut results = std::collections::HashMap::new();
        results.insert(PathBuf::from("test1.pl"), ParseOutcome::Ok { duration_ms: 100 });
        results.insert(
            PathBuf::from("test2.pl"),
            ParseOutcome::Error { message: "error".to_string() },
        );
        results.insert(PathBuf::from("test3.pl"), ParseOutcome::Timeout { timeout_ms: 1000 });
        results.insert(
            PathBuf::from("test4.pl"),
            ParseOutcome::Panic { message: "panic".to_string() },
        );

        // Create empty sources map for test
        let sources: HashMap<PathBuf, String> = HashMap::new();

        let summary = generate_parse_outcomes_summary(&results, &sources);

        assert_eq!(summary.total, 4);
        assert_eq!(summary.ok, 1);
        assert_eq!(summary.error, 1);
        assert_eq!(summary.timeout, 1);
        assert_eq!(summary.panic, 1);
        // Error should be categorized as General when no source is provided
        assert_eq!(summary.error_by_category.get("General"), Some(&1));
        assert_eq!(summary.failing_files.len(), 2); // error + panic
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
