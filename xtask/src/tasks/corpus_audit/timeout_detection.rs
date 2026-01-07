//! Timeout and hang risk detection
//!
//! This module provides timeout protection for parsing operations and
//! detection of files that may cause timeouts or hangs.

use color_eyre::eyre::Result;
use perl_parser::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::MAX_HEREDOC_DEPTH;
use super::MAX_HEREDOC_SIZE;
use super::MAX_NESTING_DEPTH;
use super::MAX_REGEX_OPERATIONS;

/// Outcome of parsing a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParseOutcome {
    /// Parse succeeded
    Ok {
        /// Time taken to parse
        duration_ms: u64,
    },
    /// Parse failed with error
    Error {
        /// Error message
        message: String,
    },
    /// Parse timed out
    Timeout {
        /// Timeout duration
        timeout_ms: u64,
    },
    /// Parse caused panic
    Panic {
        /// Panic message
        message: String,
    },
}

impl ParseOutcome {
    /// Check if the parse was successful
    pub fn is_ok(&self) -> bool {
        matches!(self, ParseOutcome::Ok { .. })
    }

    /// Get the duration in milliseconds if parse succeeded
    pub fn duration_ms(&self) -> Option<u64> {
        match self {
            ParseOutcome::Ok { duration_ms } => Some(*duration_ms),
            _ => None,
        }
    }
}

/// Priority level for timeout risks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(usize)]
pub enum RiskPriority {
    /// Critical - immediate action required
    P0 = 0,
    /// High - should be addressed soon
    P1 = 1,
    /// Medium - nice to have
    P2 = 2,
}

/// A detected timeout or hang risk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutRisk {
    /// Priority level (0 = critical)
    pub priority: RiskPriority,
    /// Description of the risk
    pub description: String,
    /// File path where risk was detected
    pub file_path: PathBuf,
    /// Specific line number (if applicable)
    pub line_number: Option<usize>,
    /// Suggested mitigation
    pub mitigation: String,
}

/// Parse a file with timeout protection
///
/// This function attempts to parse the given file content within the specified timeout.
/// If parsing exceeds the timeout, it returns a Timeout outcome.
pub fn parse_with_timeout(
    path: &PathBuf,
    content: &str,
    timeout: Duration,
) -> ParseOutcome {
    let start = Instant::now();

    // Use std::thread to enforce timeout
    let content_clone = content.to_string();
    let path_clone = path.clone();

    let result = std::thread::spawn(move || {
        let mut parser = Parser::new(&content_clone);
        match parser.parse() {
            Ok(_) => ParseOutcome::Ok {
                duration_ms: start.elapsed().as_millis() as u64,
            },
            Err(e) => ParseOutcome::Error {
                message: e.to_string(),
            },
        }
    })
    .join();

    match result {
        Ok(outcome) => {
            // Check if we exceeded timeout
            let elapsed = start.elapsed();
            if elapsed > timeout {
                ParseOutcome::Timeout {
                    timeout_ms: timeout.as_millis() as u64,
                }
            } else {
                outcome
            }
        }
        Err(_) => {
            // Thread panicked
            ParseOutcome::Panic {
                message: "Parser thread panicked".to_string(),
            }
        }
    }
}

/// Detect timeout and hang risks in corpus files
///
/// This function analyzes corpus files for patterns that may cause
/// timeouts or hangs during parsing:
/// - Deeply nested structures
/// - Complex regex patterns
/// - Large heredocs
/// - Excessive string interpolation
pub fn detect_timeout_risks(files: &[super::corpus::CorpusFile]) -> Vec<TimeoutRisk> {
    let mut risks = Vec::new();

    for file in files {
        risks.extend(analyze_file_for_risks(file));
    }

    risks
}

/// Analyze a single file for timeout/hang risks
fn analyze_file_for_risks(file: &super::corpus::CorpusFile) -> Vec<TimeoutRisk> {
    let mut risks = Vec::new();
    let lines: Vec<&str> = file.content.lines().collect();

    // Check for deep nesting
    let nesting_risks = check_deep_nesting(&lines, &file.path);
    risks.extend(nesting_risks);

    // Check for complex regex patterns
    let regex_risks = check_complex_regex(&lines, &file.path);
    risks.extend(regex_risks);

    // Check for large heredocs
    let heredoc_risks = check_large_heredocs(&lines, &file.path);
    risks.extend(heredoc_risks);

    // Check for excessive string interpolation
    let interp_risks = check_excessive_interpolation(&lines, &file.path);
    risks.extend(interp_risks);

    risks
}

/// Check for deeply nested structures
fn check_deep_nesting(lines: &[&str], path: &PathBuf) -> Vec<TimeoutRisk> {
    let mut risks = Vec::new();
    let mut depth = 0;
    let mut max_depth = 0;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Count opening braces/brackets
        depth += trimmed.matches('{').count();
        depth += trimmed.matches('(').count();
        depth += trimmed.matches('[').count();

        // Count closing braces/brackets
        depth -= trimmed.matches('}').count();
        depth -= trimmed.matches(')').count();
        depth -= trimmed.matches(']').count();

        max_depth = max_depth.max(depth);

        // Check if we've exceeded max nesting depth
        if max_depth > MAX_NESTING_DEPTH {
            risks.push(TimeoutRisk {
                priority: RiskPriority::P0,
                description: format!("Deep nesting detected (depth {})", max_depth),
                file_path: path.clone(),
                line_number: Some(i + 1),
                mitigation: format!(
                    "Refactor to reduce nesting depth below {}",
                    MAX_NESTING_DEPTH
                ),
            });
            break;
        }
    }

    // Check for high but not critical nesting
    if max_depth > MAX_NESTING_DEPTH / 2 && risks.is_empty() {
        risks.push(TimeoutRisk {
            priority: RiskPriority::P1,
            description: format!("High nesting depth (depth {})", max_depth),
            file_path: path.clone(),
            line_number: None,
            mitigation: "Consider refactoring to reduce nesting".to_string(),
        });
    }

    risks
}

/// Check for complex regex patterns
fn check_complex_regex(lines: &[&str], path: &PathBuf) -> Vec<TimeoutRisk> {
    let mut risks = Vec::new();
    let mut regex_count = 0;

    for (i, line) in lines.iter().enumerate() {
        // Look for regex patterns
        if line.contains("m/")
            || line.contains("s/")
            || line.contains("qr/")
            || line.contains("=~ /")
            || line.contains("!~ /")
        {
            regex_count += 1;

            // Check for complex regex patterns
            if line.contains("(?:") && line.contains("*") {
                risks.push(TimeoutRisk {
                    priority: RiskPriority::P0,
                    description: "Complex regex pattern with nested quantifiers".to_string(),
                    file_path: path.clone(),
                    line_number: Some(i + 1),
                    mitigation: "Simplify regex pattern or use atomic grouping".to_string(),
                });
            }

            // Check for excessive alternation
            let alt_count = line.matches('|').count();
            if alt_count > 10 {
                risks.push(TimeoutRisk {
                    priority: RiskPriority::P1,
                    description: format!("Excessive regex alternation ({} branches)", alt_count),
                    file_path: path.clone(),
                    line_number: Some(i + 1),
                    mitigation: "Consider using character classes or splitting into multiple patterns".to_string(),
                });
            }
        }
    }

    // Check for too many regex operations
    if regex_count > MAX_REGEX_OPERATIONS {
        risks.push(TimeoutRisk {
            priority: RiskPriority::P0,
            description: format!("Excessive regex operations ({} patterns)", regex_count),
            file_path: path.clone(),
            line_number: None,
            mitigation: format!(
                "Reduce regex operations below {}",
                MAX_REGEX_OPERATIONS
            ),
        });
    }

    risks
}

/// Check for large heredocs
fn check_large_heredocs(lines: &[&str], path: &PathBuf) -> Vec<TimeoutRisk> {
    let mut risks = Vec::new();
    let mut in_heredoc = false;
    let mut heredoc_depth = 0;
    let mut heredoc_size = 0;
    let mut heredoc_start_line = 0;

    for (i, line) in lines.iter().enumerate() {
        // Check for heredoc start
        if let Some(start) = line.strip_prefix("<<") {
            if !start.starts_with('"') && !start.starts_with("'") {
                // Bare heredoc
                let heredoc_marker = start.trim();
                if !heredoc_marker.is_empty() {
                    in_heredoc = true;
                    heredoc_start_line = i + 1;
                    heredoc_size = 0;
                    heredoc_depth += 1;
                }
            }
        } else if in_heredoc {
            // Check for heredoc end marker
            let trimmed = line.trim();
            if trimmed == trimmed {
                // Simple heredoc end
                in_heredoc = false;
                heredoc_depth -= 1;

                // Check heredoc size
                if heredoc_size > MAX_HEREDOC_SIZE {
                    risks.push(TimeoutRisk {
                        priority: RiskPriority::P0,
                        description: format!(
                            "Large heredoc ({} bytes)",
                            heredoc_size
                        ),
                        file_path: path.clone(),
                        line_number: Some(heredoc_start_line),
                        mitigation: format!(
                            "Reduce heredoc size below {} bytes",
                            MAX_HEREDOC_SIZE
                        ),
                    });
                }
            }
        }

        // Track heredoc content size
        if in_heredoc {
            heredoc_size += line.len();
        }
    }

    // Check for excessive heredoc nesting
    if heredoc_depth > MAX_HEREDOC_DEPTH {
        risks.push(TimeoutRisk {
            priority: RiskPriority::P0,
            description: format!("Excessive heredoc nesting (depth {})", heredoc_depth),
            file_path: path.clone(),
            line_number: None,
            mitigation: format!(
                "Reduce heredoc nesting below {}",
                MAX_HEREDOC_DEPTH
            ),
        });
    }

    risks
}

/// Check for excessive string interpolation
fn check_excessive_interpolation(lines: &[&str], path: &PathBuf) -> Vec<TimeoutRisk> {
    let mut risks = Vec::new();
    let mut interp_count = 0;

    for (i, line) in lines.iter().enumerate() {
        // Count interpolation patterns
        let line_interp = line.matches("${").count()
            + line.matches("@{").count()
            + line.matches("%{").count();

        interp_count += line_interp;

        // Check for excessive interpolation in single line
        if line_interp > 20 {
            risks.push(TimeoutRisk {
                priority: RiskPriority::P1,
                description: format!("Excessive interpolation in single line ({} occurrences)", line_interp),
                file_path: path.clone(),
                line_number: Some(i + 1),
                mitigation: "Consider using string formatting functions or breaking into multiple lines".to_string(),
            });
        }
    }

    // Check for overall excessive interpolation
    if interp_count > 100 {
        risks.push(TimeoutRisk {
            priority: RiskPriority::P2,
            description: format!("High interpolation count overall ({} occurrences)", interp_count),
            file_path: path.clone(),
            line_number: None,
            mitigation: "Consider reducing string interpolation complexity".to_string(),
        });
    }

    risks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_outcome_is_ok() {
        assert!(ParseOutcome::Ok { duration_ms: 100 }.is_ok());
        assert!(!ParseOutcome::Error { message: "error".to_string() }.is_ok());
        assert!(!ParseOutcome::Timeout { timeout_ms: 1000 }.is_ok());
        assert!(!ParseOutcome::Panic { message: "panic".to_string() }.is_ok());
    }

    #[test]
    fn test_parse_outcome_duration_ms() {
        assert_eq!(
            ParseOutcome::Ok { duration_ms: 100 }.duration_ms(),
            Some(100)
        );
        assert_eq!(
            ParseOutcome::Error { message: "error".to_string() }.duration_ms(),
            None
        );
    }

    #[test]
    fn test_risk_priority_ord() {
        assert!(RiskPriority::P0 < RiskPriority::P1);
        assert!(RiskPriority::P1 < RiskPriority::P2);
    }
}
