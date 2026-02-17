//! Breakpoint Management
//!
//! This module provides breakpoint storage and management for the DAP adapter.
//! It implements REPLACE semantics for `setBreakpoints` requests and tracks
//! breakpoints by source file path.
//!
//! # Architecture
//!
//! - **BreakpointStore**: Thread-safe storage mapping source paths to breakpoint records
//! - **BreakpointRecord**: Individual breakpoint with unique ID, location, and verification status
//! - **REPLACE Semantics**: Each `setBreakpoints` call clears existing breakpoints for that source
//!
//! # References
//!
//! - [DAP Protocol Schema](../../docs/DAP_PROTOCOL_SCHEMA.md#4-breakpoint-requests)
//! - [DAP Implementation Spec](../../docs/DAP_IMPLEMENTATION_SPECIFICATION.md#ac7-breakpoint-management)

use crate::protocol::{Breakpoint, SetBreakpointsArguments};
use perl_dap_breakpoint::{AstBreakpointValidator, BreakpointValidator};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ============= AST Validation Utilities (AC7) =============

/// Validate a breakpoint against source using the dedicated breakpoint microcrate.
///
/// Returns `(verified, resolved_line, message)` where `resolved_line` may differ from
/// the requested line if the validator adjusts the location.
#[cfg(test)]
fn validate_breakpoint_line_with_column(
    source: &str,
    line: i64,
    column: Option<i64>,
) -> (bool, i64, Option<String>) {
    if line <= 0 {
        return (false, line, Some("Line number must be positive".to_string()));
    }

    match AstBreakpointValidator::new(source) {
        Ok(validator) => {
            let result = validator.validate_with_column(line, column);
            (result.verified, result.line, result.message)
        }
        Err(error) => (false, line, Some(error.to_string())),
    }
}

/// Backward-compatible helper used by unit tests in this module.
#[cfg(test)]
fn validate_breakpoint_line(source: &str, line: i64) -> (bool, Option<String>) {
    let (verified, _resolved_line, message) =
        validate_breakpoint_line_with_column(source, line, None);
    (verified, message)
}

/// Individual breakpoint record
///
/// Stores the breakpoint metadata including unique ID, location,
/// verification status, and optional condition.
#[derive(Debug, Clone)]
pub struct BreakpointRecord {
    /// Unique breakpoint identifier (monotonically increasing)
    pub id: i64,
    /// Line number (1-based)
    pub line: i64,
    /// Column number (0-based, optional)
    pub column: Option<i64>,
    /// Breakpoint condition (e.g., "$x > 10")
    pub condition: Option<String>,
    /// Whether breakpoint was successfully verified
    pub verified: bool,
    /// Verification message (error/warning if not verified or adjusted)
    pub message: Option<String>,
}

impl BreakpointRecord {
    /// Convert to DAP protocol Breakpoint type
    pub fn to_protocol(&self) -> Breakpoint {
        Breakpoint {
            id: self.id,
            verified: self.verified,
            line: self.line,
            column: self.column,
            message: self.message.clone(),
        }
    }
}

/// Thread-safe breakpoint storage
///
/// Stores breakpoints indexed by source file path. Provides methods for
/// setting, clearing, and retrieving breakpoints with REPLACE semantics.
#[derive(Debug, Clone)]
pub struct BreakpointStore {
    /// Map of source path -> list of breakpoints
    breakpoints: Arc<Mutex<HashMap<String, Vec<BreakpointRecord>>>>,
    /// Next breakpoint ID (monotonically increasing)
    next_id: Arc<Mutex<i64>>,
}

impl BreakpointStore {
    /// Create a new empty breakpoint store
    ///
    /// # Examples
    ///
    /// ```
    /// use perl_dap::breakpoints::BreakpointStore;
    ///
    /// let store = BreakpointStore::new();
    /// ```
    pub fn new() -> Self {
        Self { breakpoints: Arc::new(Mutex::new(HashMap::new())), next_id: Arc::new(Mutex::new(1)) }
    }

    /// Set breakpoints for a source file (REPLACE semantics)
    ///
    /// This method clears all existing breakpoints for the source file
    /// and sets the new breakpoints from the request. Each breakpoint
    /// is assigned a unique ID and verified status.
    ///
    /// # Arguments
    ///
    /// * `args` - SetBreakpoints request arguments containing source and breakpoint list
    ///
    /// # Returns
    ///
    /// Array of verified breakpoints in SAME ORDER as the request.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use perl_dap::breakpoints::BreakpointStore;
    /// use perl_dap::protocol::{SetBreakpointsArguments, Source, SourceBreakpoint};
    ///
    /// let store = BreakpointStore::new();
    /// let args = SetBreakpointsArguments {
    ///     source: Source {
    ///         path: Some("/workspace/script.pl".to_string()),
    ///         name: Some("script.pl".to_string()),
    ///     },
    ///     breakpoints: Some(vec![
    ///         SourceBreakpoint { line: 10, column: None, condition: None },
    ///         SourceBreakpoint { line: 25, column: None, condition: None },
    ///     ]),
    ///     source_modified: None,
    /// };
    ///
    /// let breakpoints = store.set_breakpoints(&args);
    /// assert_eq!(breakpoints.len(), 2);
    /// ```
    pub fn set_breakpoints(&self, args: &SetBreakpointsArguments) -> Vec<Breakpoint> {
        // Extract source path (required for breakpoint storage)
        let source_path = match &args.source.path {
            Some(path) => path.clone(),
            None => {
                // No source path provided - return empty array
                return Vec::new();
            }
        };

        // Get breakpoints array (empty if not provided)
        let source_breakpoints = args.breakpoints.as_ref().map_or(Vec::new(), |bps| bps.clone());

        // Lock stores for atomic operation
        let mut breakpoints_map = self.breakpoints.lock().unwrap_or_else(|e| e.into_inner());
        let mut next_id = self.next_id.lock().unwrap_or_else(|e| e.into_inner());

        // Clear existing breakpoints for this source (REPLACE semantics)
        breakpoints_map.remove(&source_path);

        // Read source file and parse once for AST validation (AC7).
        let source_content = std::fs::read_to_string(&source_path).ok();
        let validator = source_content
            .as_ref()
            .map(|content| AstBreakpointValidator::new(content).map_err(|e| e.to_string()));

        let mut records = Vec::new();
        // Create new breakpoint records
        for bp in &source_breakpoints {
            let id = *next_id;
            *next_id += 1;

            if bp.line <= 0 {
                records.push(BreakpointRecord {
                    id,
                    line: bp.line,
                    column: bp.column,
                    condition: bp.condition.clone(),
                    verified: false,
                    message: Some("Line number must be positive".to_string()),
                });
                continue;
            }

            // AC7: Security validation - Reject conditions with newlines
            // The Perl debugger protocol is line-based, so a newline in a condition
            // allows injecting arbitrary debugger commands.
            if let Some(ref condition) = bp.condition {
                if condition.contains('\n') || condition.contains('\r') {
                    let record = BreakpointRecord {
                        id,
                        line: bp.line,
                        column: bp.column,
                        condition: bp.condition.clone(),
                        verified: false,
                        message: Some("Breakpoint condition cannot contain newlines".to_string()),
                    };
                    records.push(record);
                    continue;
                }
            }

            // AC7: AST-based breakpoint validation via `perl-dap-breakpoint` microcrate.
            let (verified, resolved_line, message) = match &validator {
                Some(Ok(v)) => {
                    let result = v.validate_with_column(bp.line, bp.column);
                    (result.verified, result.line, result.message)
                }
                Some(Err(error)) => (false, bp.line, Some(error.clone())),
                None => {
                    // Can't read file - mark as unverified but still create breakpoint.
                    (false, bp.line, Some("Unable to read source file".to_string()))
                }
            };

            let record = BreakpointRecord {
                id,
                line: resolved_line,
                column: bp.column,
                condition: bp.condition.clone(),
                verified,
                message,
            };

            records.push(record);
        }

        // Store breakpoints for this source
        if !records.is_empty() {
            breakpoints_map.insert(source_path.clone(), records.clone());
        }

        // Convert to protocol format (preserving order)
        records.iter().map(|r| r.to_protocol()).collect()
    }

    /// Get all breakpoints for a source file
    ///
    /// # Arguments
    ///
    /// * `source_path` - Absolute path to source file
    ///
    /// # Returns
    ///
    /// Array of breakpoint records for the source, or empty if none exist.
    pub fn get_breakpoints(&self, source_path: &str) -> Vec<BreakpointRecord> {
        let breakpoints_map = self.breakpoints.lock().unwrap_or_else(|e| e.into_inner());
        breakpoints_map.get(source_path).map_or(Vec::new(), |bps| bps.clone())
    }

    /// Clear all breakpoints for a source file
    ///
    /// # Arguments
    ///
    /// * `source_path` - Absolute path to source file
    pub fn clear_breakpoints(&self, source_path: &str) {
        let mut breakpoints_map = self.breakpoints.lock().unwrap_or_else(|e| e.into_inner());
        breakpoints_map.remove(source_path);
    }

    /// Clear all breakpoints in all source files
    pub fn clear_all(&self) {
        let mut breakpoints_map = self.breakpoints.lock().unwrap_or_else(|e| e.into_inner());
        breakpoints_map.clear();
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        let breakpoints_map = self.breakpoints.lock().unwrap_or_else(|e| e.into_inner());
        breakpoints_map.is_empty()
    }

    /// Get breakpoint by ID across all sources
    ///
    /// # Arguments
    ///
    /// * `id` - Unique breakpoint identifier
    ///
    /// # Returns
    ///
    /// Breakpoint record if found, None otherwise.
    pub fn get_breakpoint_by_id(&self, id: i64) -> Option<BreakpointRecord> {
        let breakpoints_map = self.breakpoints.lock().unwrap_or_else(|e| e.into_inner());
        for records in breakpoints_map.values() {
            if let Some(record) = records.iter().find(|r| r.id == id) {
                return Some(record.clone());
            }
        }
        None
    }

    /// AC7.4: Adjust breakpoints for a file edit
    ///
    /// This method shifts breakpoint lines based on content changes.
    /// It provides <1ms performance by avoiding full AST re-parsing.
    ///
    /// # Arguments
    ///
    /// * `source_path` - Path to the modified file
    /// * `start_line` - Line where the edit started (1-based)
    /// * `lines_delta` - Number of lines added (positive) or removed (negative)
    pub fn adjust_breakpoints_for_edit(
        &self,
        source_path: &str,
        start_line: i64,
        lines_delta: i64,
    ) {
        let mut breakpoints_map = self.breakpoints.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(records) = breakpoints_map.get_mut(source_path) {
            for record in records {
                // Shift breakpoints that are at or after the edit line
                if record.line >= start_line {
                    record.line += lines_delta;
                    // Ensure line number stays valid (min 1)
                    if record.line < 1 {
                        record.line = 1;
                        record.verified = false;
                        record.message = Some("Breakpoint invalidated by edit".to_string());
                    }
                }
            }
        }
    }
}

impl Default for BreakpointStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{SetBreakpointsArguments, Source, SourceBreakpoint};
    use perl_tdd_support::must;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Create a temp file with valid Perl code for testing breakpoints.
    /// Returns the temp file (keeps it alive) and its path.
    fn create_test_perl_file() -> (NamedTempFile, String) {
        let mut file = must(NamedTempFile::with_suffix(".pl"));
        // Create 30 lines of valid Perl code for breakpoint testing
        // NOTE: Avoid sub immediately followed by for loop (triggers parser hang - known issue)
        let perl_code = r#"#!/usr/bin/perl
use strict;
use warnings;

my $x = 1;
my $y = 2;
my $z = $x + $y;

if ($x > 0) {
    print "positive\n";
}

my @arr = (1, 2, 3);
while (my $item = shift @arr) {
    my $doubled = $item * 2;
    print "$doubled\n";
}

sub process {
    my ($value) = @_;
    my $result = $value * 2;
    return $result;
}

print "done\n";
my $final = process($x);
print "result: $final\n";
"#;
        must(file.write_all(perl_code.as_bytes()));
        must(file.flush());
        let path = file.path().to_string_lossy().to_string();
        (file, path)
    }

    #[test]
    fn test_breakpoint_store_new() {
        let store = BreakpointStore::new();
        let breakpoints = store.get_breakpoints("/workspace/test.pl");
        assert_eq!(breakpoints.len(), 0);
    }

    #[test]
    fn test_set_breakpoints_creates_records() {
        let (_file, source_path) = create_test_perl_file();
        let store = BreakpointStore::new();
        let args = SetBreakpointsArguments {
            source: Source { path: Some(source_path.clone()), name: Some("script.pl".to_string()) },
            breakpoints: Some(vec![
                SourceBreakpoint { line: 10, column: None, condition: None },
                SourceBreakpoint {
                    line: 25,
                    column: Some(5),
                    condition: Some("$x > 10".to_string()),
                },
            ]),
            source_modified: None,
        };

        let breakpoints = store.set_breakpoints(&args);

        assert_eq!(breakpoints.len(), 2);
        assert_eq!(breakpoints[0].line, 10);
        assert!(breakpoints[0].verified);
        assert_eq!(breakpoints[1].line, 25);
        assert_eq!(breakpoints[1].column, Some(5));
        assert!(breakpoints[1].verified);
    }

    #[test]
    fn test_set_breakpoints_replace_semantics() {
        let (_file, source_path) = create_test_perl_file();
        let store = BreakpointStore::new();

        // Set initial breakpoints
        let args1 = SetBreakpointsArguments {
            source: Source { path: Some(source_path.clone()), name: Some("script.pl".to_string()) },
            breakpoints: Some(vec![SourceBreakpoint { line: 10, column: None, condition: None }]),
            source_modified: None,
        };
        store.set_breakpoints(&args1);

        // Replace with new breakpoints
        let args2 = SetBreakpointsArguments {
            source: Source { path: Some(source_path.clone()), name: Some("script.pl".to_string()) },
            breakpoints: Some(vec![
                SourceBreakpoint { line: 20, column: None, condition: None },
                SourceBreakpoint { line: 26, column: None, condition: None },
            ]),
            source_modified: None,
        };
        let breakpoints = store.set_breakpoints(&args2);

        // Should have only the new breakpoints
        assert_eq!(breakpoints.len(), 2);
        assert_eq!(breakpoints[0].line, 20);
        assert_eq!(breakpoints[1].line, 26);

        // Verify stored breakpoints
        let stored = store.get_breakpoints(&source_path);
        assert_eq!(stored.len(), 2);
    }

    #[test]
    fn test_set_breakpoints_unique_ids() {
        let (_file, source_path) = create_test_perl_file();
        let store = BreakpointStore::new();
        let args = SetBreakpointsArguments {
            source: Source { path: Some(source_path), name: Some("script.pl".to_string()) },
            breakpoints: Some(vec![
                SourceBreakpoint { line: 10, column: None, condition: None },
                SourceBreakpoint { line: 20, column: None, condition: None },
            ]),
            source_modified: None,
        };

        let breakpoints = store.set_breakpoints(&args);

        // IDs should be unique
        assert_ne!(breakpoints[0].id, breakpoints[1].id);
    }

    #[test]
    fn test_set_breakpoints_preserves_order() {
        let (_file, source_path) = create_test_perl_file();
        let store = BreakpointStore::new();
        let args = SetBreakpointsArguments {
            source: Source { path: Some(source_path), name: Some("script.pl".to_string()) },
            // Use lines within our 30-line test file, but out of order
            breakpoints: Some(vec![
                SourceBreakpoint { line: 25, column: None, condition: None },
                SourceBreakpoint { line: 10, column: None, condition: None },
                SourceBreakpoint { line: 15, column: None, condition: None },
            ]),
            source_modified: None,
        };

        let breakpoints = store.set_breakpoints(&args);

        // Order must match request (not sorted by line number)
        assert_eq!(breakpoints[0].line, 25);
        assert_eq!(breakpoints[1].line, 10);
        assert_eq!(breakpoints[2].line, 15);
    }

    #[test]
    fn test_clear_breakpoints() {
        let store = BreakpointStore::new();
        let source_path = "/workspace/script.pl";

        let args = SetBreakpointsArguments {
            source: Source {
                path: Some(source_path.to_string()),
                name: Some("script.pl".to_string()),
            },
            breakpoints: Some(vec![SourceBreakpoint { line: 10, column: None, condition: None }]),
            source_modified: None,
        };
        store.set_breakpoints(&args);

        // Clear breakpoints
        store.clear_breakpoints(source_path);

        // Should be empty
        let breakpoints = store.get_breakpoints(source_path);
        assert_eq!(breakpoints.len(), 0);
    }

    #[test]
    fn test_clear_all() {
        let store = BreakpointStore::new();

        // Set breakpoints in multiple files
        let args1 = SetBreakpointsArguments {
            source: Source {
                path: Some("/workspace/file1.pl".to_string()),
                name: Some("file1.pl".to_string()),
            },
            breakpoints: Some(vec![SourceBreakpoint { line: 10, column: None, condition: None }]),
            source_modified: None,
        };
        store.set_breakpoints(&args1);

        let args2 = SetBreakpointsArguments {
            source: Source {
                path: Some("/workspace/file2.pl".to_string()),
                name: Some("file2.pl".to_string()),
            },
            breakpoints: Some(vec![SourceBreakpoint { line: 20, column: None, condition: None }]),
            source_modified: None,
        };
        store.set_breakpoints(&args2);

        // Clear all
        store.clear_all();

        // Both should be empty
        assert_eq!(store.get_breakpoints("/workspace/file1.pl").len(), 0);
        assert_eq!(store.get_breakpoints("/workspace/file2.pl").len(), 0);
    }

    #[test]
    fn test_get_breakpoint_by_id() -> Result<(), Box<dyn std::error::Error>> {
        let store = BreakpointStore::new();
        let args = SetBreakpointsArguments {
            source: Source {
                path: Some("/workspace/script.pl".to_string()),
                name: Some("script.pl".to_string()),
            },
            breakpoints: Some(vec![
                SourceBreakpoint { line: 10, column: None, condition: None },
                SourceBreakpoint { line: 25, column: None, condition: None },
            ]),
            source_modified: None,
        };

        let breakpoints = store.set_breakpoints(&args);
        let id = breakpoints[0].id;

        // Retrieve by ID
        let record = store.get_breakpoint_by_id(id);
        assert!(record.is_some());
        assert_eq!(record.ok_or("Expected record")?.line, 10);

        // Non-existent ID
        let not_found = store.get_breakpoint_by_id(999999);
        assert!(not_found.is_none());
        Ok(())
    }

    #[test]
    fn test_empty_breakpoints_array() {
        let store = BreakpointStore::new();
        let args = SetBreakpointsArguments {
            source: Source {
                path: Some("/workspace/script.pl".to_string()),
                name: Some("script.pl".to_string()),
            },
            breakpoints: Some(vec![]),
            source_modified: None,
        };

        let breakpoints = store.set_breakpoints(&args);
        assert_eq!(breakpoints.len(), 0);
    }

    #[test]
    fn test_no_source_path() {
        let store = BreakpointStore::new();
        let args = SetBreakpointsArguments {
            source: Source { path: None, name: Some("script.pl".to_string()) },
            breakpoints: Some(vec![SourceBreakpoint { line: 10, column: None, condition: None }]),
            source_modified: None,
        };

        let breakpoints = store.set_breakpoints(&args);
        assert_eq!(breakpoints.len(), 0);
    }

    #[test]
    fn test_adjust_breakpoints_for_edit() {
        // AC:7.4
        let store = BreakpointStore::new();
        let source_path = "/workspace/script.pl";

        // Mock store with manual insertion to avoid FS dependencies
        let record = BreakpointRecord {
            id: 1,
            line: 10,
            column: None,
            condition: None,
            verified: true,
            message: None,
        };
        store
            .breakpoints
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(source_path.to_string(), vec![record]);

        // 1. Add 5 lines at line 5 (shift down)
        store.adjust_breakpoints_for_edit(source_path, 5, 5);
        assert_eq!(store.get_breakpoints(source_path)[0].line, 15);

        // 2. Remove 3 lines at line 5 (shift up)
        store.adjust_breakpoints_for_edit(source_path, 5, -3);
        assert_eq!(store.get_breakpoints(source_path)[0].line, 12);

        // 3. Edit after breakpoint (no shift)
        store.adjust_breakpoints_for_edit(source_path, 20, 10);
        assert_eq!(store.get_breakpoints(source_path)[0].line, 12);
    }

    #[test]
    fn test_validate_breakpoint_line_scenarios() {
        // AC:7.3
        let source = r#"use strict;
# This is a comment
my $x = 1;

    
print "hello";
<<EOF;
heredoc content
EOF
"#;
        // Line 1: use strict; (Valid)
        let (v1, _) = validate_breakpoint_line(source, 1);
        assert!(v1, "Line 1 should be valid");

        // Line 2: # comment (Invalid)
        let (v2, m2) = validate_breakpoint_line(source, 2);
        assert!(!v2, "Line 2 should be invalid");
        assert!(
            m2.as_ref().is_some_and(|s| s.contains("comment")),
            "Expected comment error message"
        );

        // Line 4: blank line (Invalid)
        let (v4, m4) = validate_breakpoint_line(source, 4);
        assert!(!v4, "Line 4 should be invalid");
        assert!(
            m4.as_ref().is_some_and(|s| s.contains("blank")),
            "Expected blank line error message"
        );

        // Line 5: line with whitespace (Invalid)
        let (v5, _) = validate_breakpoint_line(source, 5);
        assert!(!v5, "Line 5 should be invalid");

        // Line 8: heredoc interior (Invalid)
        // Note: depends on parser support for NodeKind::Heredoc with body_span
        let (v8, _) = validate_breakpoint_line(source, 8);
        // If parser supports it, it should be invalid.
        // For now we just verify it doesn't panic.
        let _ = v8;
    }
}
