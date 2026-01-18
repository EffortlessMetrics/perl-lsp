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
use super::timeout_detection::{ParseOutcome, TimeoutRisk, categorize_error};

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
    /// Line number (1-based) where error occurred
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_number: Option<usize>,
    /// Column number (1-based) where error occurred
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
    /// Found token kind (if parseable from error)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub found_token: Option<String>,
    /// Expected token(s) (if parseable from error)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    /// Code snippet around the error (1-2 lines)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_snippet: Option<String>,
}

/// Parse byte offset from error message (e.g., "at 334")
fn parse_byte_offset(message: &str) -> Option<usize> {
    // Look for "at N" pattern at end of message
    if let Some(idx) = message.rfind(" at ") {
        let offset_str = message[idx + 4..].trim();
        offset_str.parse().ok()
    } else {
        None
    }
}

/// Convert byte offset to line and column (1-based)
fn byte_offset_to_line_col(source: &str, byte_offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    let mut current_offset = 0;

    for ch in source.chars() {
        if current_offset >= byte_offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
        current_offset += ch.len_utf8();
    }

    (line, col)
}

/// Extract code snippet around a line (up to 2 lines context)
fn extract_snippet(source: &str, line_number: usize) -> String {
    let line_idx = line_number.saturating_sub(1); // Convert to 0-based
    let line = match source.lines().nth(line_idx) {
        Some(l) => l,
        None => return String::new(),
    };

    // Truncate long lines to keep report manageable
    if line.len() > 80 {
        format!("{}...", &line[..77])
    } else {
        line.to_string()
    }
}

/// Parse found/expected tokens from error message
///
/// Handles error formats like:
/// - "Unexpected token: expected expression, found Comma at 334"
/// - "expected RightParen, found Identifier at 2018"
fn parse_token_info(message: &str) -> (Option<String>, Option<String>) {
    let mut found = None;
    let mut expected = None;

    // Look for the pattern "expected X, found Y"
    // Handle both "expected X" and "Unexpected token: expected X" formats
    if let Some(exp_idx) = message.rfind("expected ") {
        let after_expected = &message[exp_idx + 9..];
        if let Some(comma_idx) = after_expected.find(", found ") {
            expected = Some(after_expected[..comma_idx].to_string());
            let after_found = &after_expected[comma_idx + 8..];
            // Strip " at N" suffix if present
            let found_token = if let Some(at_idx) = after_found.rfind(" at ") {
                &after_found[..at_idx]
            } else {
                after_found
            };
            found = Some(found_token.to_string());
        }
    }

    (found, expected)
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
    let sources: HashMap<PathBuf, String> =
        corpus_files.iter().map(|f| (f.path.clone(), f.content.clone())).collect();

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

                // Extract location info from error message
                let byte_offset = parse_byte_offset(message);
                let (line_number, column) = if let Some(offset) = byte_offset {
                    let (l, c) = byte_offset_to_line_col(source, offset);
                    (Some(l), Some(c))
                } else {
                    (None, None)
                };

                // Extract code snippet if we have a line number
                let code_snippet =
                    line_number.map(|ln| extract_snippet(source, ln)).filter(|s| !s.is_empty());

                // Parse found/expected tokens
                let (found_token, expected) = parse_token_info(message);

                // Add to failing files list
                failing_files.push(FailingFile {
                    path: path.display().to_string(),
                    category: category_str,
                    error_message: message.clone(),
                    line_number,
                    column,
                    found_token,
                    expected,
                    code_snippet,
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
                    line_number: None,
                    column: None,
                    found_token: None,
                    expected: None,
                    code_snippet: None,
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

    #[test]
    fn test_parse_byte_offset() {
        assert_eq!(parse_byte_offset("Unexpected token at 334"), Some(334));
        assert_eq!(parse_byte_offset("expected RightParen, found Identifier at 2018"), Some(2018));
        assert_eq!(parse_byte_offset("no offset here"), None);
        assert_eq!(parse_byte_offset("at the end at 42"), Some(42));
    }

    #[test]
    fn test_byte_offset_to_line_col() {
        let source = "line1\nline2\nline3";
        // Offset 0 = line 1, col 1
        assert_eq!(byte_offset_to_line_col(source, 0), (1, 1));
        // Offset 2 = line 1, col 3 (within "line1")
        assert_eq!(byte_offset_to_line_col(source, 2), (1, 3));
        // Offset 6 = line 2, col 1 (start of "line2")
        assert_eq!(byte_offset_to_line_col(source, 6), (2, 1));
        // Offset 8 = line 2, col 3 (within "line2")
        assert_eq!(byte_offset_to_line_col(source, 8), (2, 3));
    }

    #[test]
    fn test_extract_snippet() {
        let source = "line1\nline2 with more content\nline3";
        assert_eq!(extract_snippet(source, 1), "line1");
        assert_eq!(extract_snippet(source, 2), "line2 with more content");
        assert_eq!(extract_snippet(source, 3), "line3");
        // Out of range
        assert_eq!(extract_snippet(source, 100), "");
    }

    #[test]
    fn test_parse_token_info() {
        // Test "Unexpected token: expected X, found Y at N" format
        let (found, expected) =
            parse_token_info("Unexpected token: expected expression, found Comma at 334");
        assert_eq!(expected, Some("expression".to_string()));
        assert_eq!(found, Some("Comma".to_string()));

        // Test "expected X, found Y at N" format
        let (found, expected) = parse_token_info("expected RightParen, found Identifier at 2018");
        assert_eq!(expected, Some("RightParen".to_string()));
        assert_eq!(found, Some("Identifier".to_string()));

        // Test no match
        let (found, expected) = parse_token_info("random error message");
        assert_eq!(expected, None);
        assert_eq!(found, None);

        // Test without "at N" suffix
        let (found, expected) = parse_token_info("expected Semicolon, found Newline");
        assert_eq!(expected, Some("Semicolon".to_string()));
        assert_eq!(found, Some("Newline".to_string()));
    }
}
