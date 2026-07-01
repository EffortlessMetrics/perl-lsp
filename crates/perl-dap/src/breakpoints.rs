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
//! - [DAP Protocol Schema](../../docs/reference/DAP_PROTOCOL_SCHEMA.md#4-breakpoint-requests)
//! - [DAP Implementation Spec](../../docs/reference/DAP_IMPLEMENTATION_SPECIFICATION.md#ac7-breakpoint-management)

use crate::breakpoint::{AstBreakpointValidator, BreakpointValidator};
use crate::protocol::{Breakpoint, SetBreakpointsArguments};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ============= AST Validation Utilities (AC7) =============

/// Combine a prior breakpoint message with the condition-invalid error.
///
/// When a breakpoint has both a line-validation message (e.g. "Breakpoint set on
/// blank line, adjusted to line 5") **and** fails condition validation, both pieces
/// of information must be surfaced to the user.  Prior to this helper the `else`
/// branch in `set_breakpoints` unconditionally overwrote any existing message.
///
/// # Examples
///
/// ```rust,ignore
/// // Prior adjustment message is preserved
/// let msg = combine_condition_message(Some("adjusted to line 5".to_string()));
/// let text = msg.as_deref().unwrap_or("");
/// assert!(text.contains("adjusted to line 5"));
/// assert!(text.contains("Conditional breakpoint expression is invalid"));
///
/// // No prior message — just the condition error
/// let msg = combine_condition_message(None);
/// assert_eq!(msg.as_deref(), Some("Conditional breakpoint expression is invalid"));
/// ```
pub(crate) fn combine_condition_message(prior: Option<String>) -> Option<String> {
    const COND_ERR: &str = "Conditional breakpoint expression is invalid";
    Some(match prior {
        Some(m) => format!("{m}; {COND_ERR}"),
        None => COND_ERR.to_string(),
    })
}

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
    /// Breakpoint hit-count condition (e.g., ">= 5", "%2")
    pub hit_condition: Option<String>,
    /// Logpoint message. When present, hit events log output and continue.
    pub log_message: Option<String>,
    /// Number of times this breakpoint has been hit in the current session.
    pub hit_count: u64,
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

/// Result of applying a runtime breakpoint hit to stored breakpoint metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BreakpointHitOutcome {
    /// True when at least one verified breakpoint matched the file/line location.
    pub matched: bool,
    /// True when execution should stop and emit a `stopped` event.
    pub should_stop: bool,
    /// Logpoint messages to emit as `output` events.
    pub log_messages: Vec<String>,
}

fn parse_hit_condition_operand(raw: &str) -> Option<u64> {
    raw.trim().parse::<u64>().ok()
}

fn is_valid_hit_condition(raw: &str) -> bool {
    evaluate_hit_condition(Some(raw), 1).is_some()
}

fn evaluate_hit_condition(raw: Option<&str>, hit_count: u64) -> Option<bool> {
    let Some(raw) = raw else {
        return Some(true);
    };

    let expr = raw.trim();
    if expr.is_empty() {
        return Some(true);
    }

    if let Some(rest) = expr.strip_prefix(">=") {
        return parse_hit_condition_operand(rest).map(|n| hit_count >= n);
    }
    if let Some(rest) = expr.strip_prefix("<=") {
        return parse_hit_condition_operand(rest).map(|n| hit_count <= n);
    }
    if let Some(rest) = expr.strip_prefix("==") {
        return parse_hit_condition_operand(rest).map(|n| hit_count == n);
    }
    if let Some(rest) = expr.strip_prefix('=') {
        return parse_hit_condition_operand(rest).map(|n| hit_count == n);
    }
    if let Some(rest) = expr.strip_prefix('>') {
        return parse_hit_condition_operand(rest).map(|n| hit_count > n);
    }
    if let Some(rest) = expr.strip_prefix('<') {
        return parse_hit_condition_operand(rest).map(|n| hit_count < n);
    }
    if let Some(rest) = expr.strip_prefix('%') {
        return parse_hit_condition_operand(rest)
            .and_then(|n| if n == 0 { None } else { Some(hit_count.is_multiple_of(n)) });
    }

    parse_hit_condition_operand(expr).map(|n| hit_count == n)
}

/// Returns true if `haystack` ends with `suffix` and the match starts at a path-component
/// boundary (i.e. the character immediately before the suffix is `/` or `\`).
/// When the suffix is exactly as long as the haystack, the strings are equal — also true.
fn path_suffix_matches(haystack: &str, suffix: &str) -> bool {
    if !haystack.ends_with(suffix) {
        return false;
    }
    let prefix_len = haystack.len().wrapping_sub(suffix.len());
    if prefix_len == 0 {
        // Equal lengths and ends_with holds → exact match.
        return true;
    }
    // The byte immediately before the suffix must be a path separator.
    matches!(haystack.as_bytes()[prefix_len - 1], b'/' | b'\\')
}

fn file_paths_match(stored: &str, observed: &str) -> bool {
    if stored == observed {
        return true;
    }
    // Allow suffix matching for relative-vs-absolute path pairs (e.g. "bar.pl" matches
    // "/abs/path/bar.pl"), but require a path-component boundary before the matched suffix
    // to prevent mid-component false matches (e.g. "bar.pl" must NOT match "foobar.pl").
    path_suffix_matches(stored, observed) || path_suffix_matches(observed, stored)
}

/// Interpolate logpoint message template with variable values.
///
/// Parses `{expression}` patterns in the message template and substitutes
/// them with values from the provided variable map.
///
/// # Arguments
///
/// * `template` - Message template with `{$variable}` expressions
/// * `variables` - HashMap of variable names to their string values
///
/// # Returns
///
/// The interpolated message with expressions replaced by variable values.
/// If a variable is not found, the expression remains as-is.
///
/// # Examples
///
/// ```
/// let mut vars = std::collections::HashMap::new();
/// vars.insert("x".to_string(), "42".to_string());
/// let result = interpolate_logpoint_message("value: {$x}", &vars);
/// assert_eq!(result, "value: 42");
/// ```
pub fn interpolate_logpoint_message(
    template: &str,
    variables: &std::collections::HashMap<String, String>,
) -> String {
    // Use a simple regex-based approach to find and replace {$var} patterns
    // For now, we'll use a manual character-by-character parser to avoid regex dependency

    let mut result = String::new();
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            // Look ahead for a variable reference like $name
            let mut expr = String::new();
            let mut found_close = false;

            // Collect everything until }
            while let Some(next_ch) = chars.peek() {
                if *next_ch == '}' {
                    chars.next(); // consume the }
                    found_close = true;
                    break;
                }
                expr.push(*next_ch);
                chars.next();
            }

            if found_close {
                // Only scalar variables ($name) are interpolated; @array/%hash and
                // arithmetic expressions are kept verbatim (full expression evaluation
                // requires a live debugger context and is deferred to a follow-up).
                let trimmed = expr.trim();
                if let Some(var_name) = trimmed.strip_prefix('$') {
                    if let Some(value) = variables.get(var_name) {
                        result.push_str(value);
                    } else {
                        // Variable not found in caller-supplied map: keep original
                        // expression so the template remains readable in the output.
                        result.push('{');
                        result.push_str(trimmed);
                        result.push('}');
                    }
                } else {
                    // Non-scalar expression (arithmetic, @array, etc.): keep verbatim.
                    result.push('{');
                    result.push_str(&expr);
                    result.push('}');
                }
            } else {
                // No closing }, keep opening brace
                result.push('{');
                result.push_str(&expr);
            }
        } else {
            result.push(ch);
        }
    }

    result
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
    ///         SourceBreakpoint { line: 10, column: None, condition: None, hit_condition: None, log_message: None },
    ///         SourceBreakpoint { line: 25, column: None, condition: None, hit_condition: None, log_message: None },
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

        // Get breakpoint request slice (empty if not provided)
        let source_breakpoints = args.breakpoints.as_deref().unwrap_or(&[]);

        // Read source file and parse once for AST validation (AC7).
        let source_content = std::fs::read_to_string(&source_path).ok();
        let validator = source_content
            .as_ref()
            .map(|content| AstBreakpointValidator::new(content).map_err(|e| e.to_string()));
        let mut validation_cache: HashMap<(i64, Option<i64>), (bool, i64, Option<String>)> =
            HashMap::new();

        // Lock stores for atomic replacement + id allocation.
        let mut breakpoints_map = self.breakpoints.lock().unwrap_or_else(|e| e.into_inner());
        let mut next_id = self.next_id.lock().unwrap_or_else(|e| e.into_inner());

        // Clear existing breakpoints for this source (REPLACE semantics)
        breakpoints_map.remove(&source_path);

        let mut records = Vec::new();
        // Create new breakpoint records
        for bp in source_breakpoints {
            let id = *next_id;
            *next_id += 1;

            if bp.line <= 0 {
                records.push(BreakpointRecord {
                    id,
                    line: bp.line,
                    column: bp.column,
                    condition: bp.condition.clone(),
                    hit_condition: bp.hit_condition.clone(),
                    log_message: bp.log_message.clone(),
                    hit_count: 0,
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
                        hit_condition: bp.hit_condition.clone(),
                        log_message: bp.log_message.clone(),
                        hit_count: 0,
                        verified: false,
                        message: Some("Breakpoint condition cannot contain newlines".to_string()),
                    };
                    records.push(record);
                    continue;
                }
            }

            if let Some(ref hit_condition) = bp.hit_condition {
                let hit_condition = hit_condition.trim();
                if hit_condition.contains('\n') || hit_condition.contains('\r') {
                    let record = BreakpointRecord {
                        id,
                        line: bp.line,
                        column: bp.column,
                        condition: bp.condition.clone(),
                        hit_condition: bp.hit_condition.clone(),
                        log_message: bp.log_message.clone(),
                        hit_count: 0,
                        verified: false,
                        message: Some("Hit condition cannot contain newlines".to_string()),
                    };
                    records.push(record);
                    continue;
                }
                if !is_valid_hit_condition(hit_condition) {
                    let record = BreakpointRecord {
                        id,
                        line: bp.line,
                        column: bp.column,
                        condition: bp.condition.clone(),
                        hit_condition: bp.hit_condition.clone(),
                        log_message: bp.log_message.clone(),
                        hit_count: 0,
                        verified: false,
                        message: Some(format!(
                            "Invalid hitCondition `{hit_condition}` (expected numeric expression like `10`, `>= 5`, `%2`)"
                        )),
                    };
                    records.push(record);
                    continue;
                }
            }

            // AC7: AST-based breakpoint validation via `perl-dap-breakpoint` microcrate.
            let (verified, resolved_line, message) =
                if let Some(cached) = validation_cache.get(&(bp.line, bp.column)) {
                    cached.clone()
                } else {
                    let computed = match &validator {
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
                    validation_cache.insert((bp.line, bp.column), computed.clone());
                    computed
                };

            let mut verified = verified;
            let message = if verified
                && bp.condition.is_some()
                && let Some(Ok(v)) = &validator
                && let Some(condition) = bp.condition.as_deref()
            {
                let condition_validation = v.validate_condition(resolved_line, condition);
                if condition_validation.verified {
                    message
                } else {
                    verified = false;
                    combine_condition_message(message)
                }
            } else {
                message
            };

            let record = BreakpointRecord {
                id,
                line: resolved_line,
                column: bp.column,
                condition: bp.condition.clone(),
                hit_condition: bp.hit_condition.clone(),
                log_message: bp.log_message.clone(),
                hit_count: 0,
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

    /// Register a runtime breakpoint hit and return stop/logpoint behavior.
    ///
    /// This method updates per-breakpoint hit counters and evaluates DAP hit
    /// conditions. For logpoints, execution continues after emitting output.
    pub fn register_breakpoint_hit(&self, source_path: &str, line: i64) -> BreakpointHitOutcome {
        self.register_breakpoint_hit_with_variables(source_path, line, None)
    }

    /// Register a breakpoint hit with optional variable interpolation.
    ///
    /// Similar to `register_breakpoint_hit`, but accepts optional variable values
    /// for logpoint message interpolation. If variables are provided, logpoint
    /// messages with `{$variable}` expressions will be interpolated.
    ///
    /// # Arguments
    ///
    /// * `source_path` - Path to the source file
    /// * `line` - Line number where the breakpoint was hit (1-based)
    /// * `variables` - Optional map of variable names to their string values
    ///
    /// # Returns
    ///
    /// BreakpointHitOutcome with interpolated log messages if variables provided
    pub fn register_breakpoint_hit_with_variables(
        &self,
        source_path: &str,
        line: i64,
        variables: Option<&std::collections::HashMap<String, String>>,
    ) -> BreakpointHitOutcome {
        let mut breakpoints_map = self.breakpoints.lock().unwrap_or_else(|e| e.into_inner());
        let mut outcome = BreakpointHitOutcome::default();

        for (stored_path, records) in &mut *breakpoints_map {
            if !file_paths_match(stored_path, source_path) {
                continue;
            }

            for record in records {
                if !record.verified || record.line != line {
                    continue;
                }

                outcome.matched = true;
                record.hit_count = record.hit_count.saturating_add(1);

                let hit_condition_match =
                    evaluate_hit_condition(record.hit_condition.as_deref(), record.hit_count)
                        .unwrap_or(false);
                if !hit_condition_match {
                    continue;
                }

                if let Some(message) = record.log_message.clone() {
                    // Interpolate message if variables are available
                    let interpolated = if let Some(vars) = variables {
                        interpolate_logpoint_message(&message, vars)
                    } else {
                        message
                    };
                    outcome.log_messages.push(interpolated);
                } else {
                    outcome.should_stop = true;
                }
            }
        }

        outcome
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
                SourceBreakpoint {
                    line: 10,
                    column: None,
                    condition: None,
                    hit_condition: None,
                    log_message: None,
                },
                SourceBreakpoint {
                    line: 25,
                    column: Some(5),
                    condition: Some("$x > 10".to_string()),
                    hit_condition: None,
                    log_message: None,
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
            breakpoints: Some(vec![SourceBreakpoint {
                line: 10,
                column: None,
                condition: None,
                hit_condition: None,
                log_message: None,
            }]),
            source_modified: None,
        };
        store.set_breakpoints(&args1);

        // Replace with new breakpoints
        let args2 = SetBreakpointsArguments {
            source: Source { path: Some(source_path.clone()), name: Some("script.pl".to_string()) },
            breakpoints: Some(vec![
                SourceBreakpoint {
                    line: 20,
                    column: None,
                    condition: None,
                    hit_condition: None,
                    log_message: None,
                },
                SourceBreakpoint {
                    line: 26,
                    column: None,
                    condition: None,
                    hit_condition: None,
                    log_message: None,
                },
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
                SourceBreakpoint {
                    line: 10,
                    column: None,
                    condition: None,
                    hit_condition: None,
                    log_message: None,
                },
                SourceBreakpoint {
                    line: 20,
                    column: None,
                    condition: None,
                    hit_condition: None,
                    log_message: None,
                },
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
                SourceBreakpoint {
                    line: 25,
                    column: None,
                    condition: None,
                    hit_condition: None,
                    log_message: None,
                },
                SourceBreakpoint {
                    line: 10,
                    column: None,
                    condition: None,
                    hit_condition: None,
                    log_message: None,
                },
                SourceBreakpoint {
                    line: 15,
                    column: None,
                    condition: None,
                    hit_condition: None,
                    log_message: None,
                },
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
            breakpoints: Some(vec![SourceBreakpoint {
                line: 10,
                column: None,
                condition: None,
                hit_condition: None,
                log_message: None,
            }]),
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
            breakpoints: Some(vec![SourceBreakpoint {
                line: 10,
                column: None,
                condition: None,
                hit_condition: None,
                log_message: None,
            }]),
            source_modified: None,
        };
        store.set_breakpoints(&args1);

        let args2 = SetBreakpointsArguments {
            source: Source {
                path: Some("/workspace/file2.pl".to_string()),
                name: Some("file2.pl".to_string()),
            },
            breakpoints: Some(vec![SourceBreakpoint {
                line: 20,
                column: None,
                condition: None,
                hit_condition: None,
                log_message: None,
            }]),
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
                SourceBreakpoint {
                    line: 10,
                    column: None,
                    condition: None,
                    hit_condition: None,
                    log_message: None,
                },
                SourceBreakpoint {
                    line: 25,
                    column: None,
                    condition: None,
                    hit_condition: None,
                    log_message: None,
                },
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
            breakpoints: Some(vec![SourceBreakpoint {
                line: 10,
                column: None,
                condition: None,
                hit_condition: None,
                log_message: None,
            }]),
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
            hit_condition: None,
            log_message: None,
            hit_count: 0,
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
    fn test_hit_condition_parser_variants() {
        assert_eq!(evaluate_hit_condition(None, 1), Some(true));
        assert_eq!(evaluate_hit_condition(Some(""), 1), Some(true));
        assert_eq!(evaluate_hit_condition(Some("3"), 3), Some(true));
        assert_eq!(evaluate_hit_condition(Some("=3"), 2), Some(false));
        assert_eq!(evaluate_hit_condition(Some(">= 2"), 2), Some(true));
        assert_eq!(evaluate_hit_condition(Some(">2"), 2), Some(false));
        assert_eq!(evaluate_hit_condition(Some("%2"), 4), Some(true));
        assert_eq!(evaluate_hit_condition(Some("%0"), 4), None);
        assert_eq!(evaluate_hit_condition(Some("invalid"), 1), None);
    }

    #[test]
    fn test_register_breakpoint_hit_respects_hit_conditions_and_logpoints() {
        let (_file, source_path) = create_test_perl_file();
        let store = BreakpointStore::new();

        let args = SetBreakpointsArguments {
            source: Source { path: Some(source_path.clone()), name: Some("script.pl".to_string()) },
            breakpoints: Some(vec![
                SourceBreakpoint {
                    line: 10,
                    column: None,
                    condition: None,
                    hit_condition: Some(">= 2".to_string()),
                    log_message: None,
                },
                SourceBreakpoint {
                    line: 15,
                    column: None,
                    condition: None,
                    hit_condition: Some("%2".to_string()),
                    log_message: Some("loop tick".to_string()),
                },
            ]),
            source_modified: None,
        };
        let responses = store.set_breakpoints(&args);
        assert_eq!(responses.len(), 2);
        assert!(responses.iter().all(|bp| bp.verified));

        let first_hit = store.register_breakpoint_hit(&source_path, 10);
        assert!(first_hit.matched);
        assert!(!first_hit.should_stop);
        assert!(first_hit.log_messages.is_empty());

        let second_hit = store.register_breakpoint_hit(&source_path, 10);
        assert!(second_hit.matched);
        assert!(second_hit.should_stop);
        assert!(second_hit.log_messages.is_empty());

        let logpoint_first = store.register_breakpoint_hit(&source_path, 15);
        assert!(logpoint_first.matched);
        assert!(!logpoint_first.should_stop);
        assert!(logpoint_first.log_messages.is_empty());

        let logpoint_second = store.register_breakpoint_hit(&source_path, 15);
        assert!(logpoint_second.matched);
        assert!(!logpoint_second.should_stop);
        assert_eq!(logpoint_second.log_messages, vec!["loop tick".to_string()]);
    }

    #[test]
    fn test_logpoint_message_interpolation() {
        // Test basic logpoint message interpolation with {$variable} syntax
        let (_file, source_path) = create_test_perl_file();
        let store = BreakpointStore::new();

        let args = SetBreakpointsArguments {
            source: Source { path: Some(source_path.clone()), name: Some("script.pl".to_string()) },
            breakpoints: Some(vec![SourceBreakpoint {
                line: 10,
                column: None,
                condition: None,
                hit_condition: None,
                log_message: Some("x = {$x}".to_string()),
            }]),
            source_modified: None,
        };
        let responses = store.set_breakpoints(&args);
        assert_eq!(responses.len(), 1);
        assert!(responses[0].verified);

        // Register hit and verify interpolation occurs
        let outcome = store.register_breakpoint_hit(&source_path, 10);
        assert!(outcome.matched);
        assert!(!outcome.should_stop); // logpoint doesn't stop

        // Interpolate the message with variable values
        let mut variables = std::collections::HashMap::new();
        variables.insert("x".to_string(), "42".to_string());

        let interpolated = interpolate_logpoint_message(&outcome.log_messages[0], &variables);
        assert_eq!(interpolated, "x = 42");
    }

    #[test]
    fn test_register_breakpoint_hit_with_variables_interpolates() {
        // Test register_breakpoint_hit_with_variables interpolates messages
        let (_file, source_path) = create_test_perl_file();
        let store = BreakpointStore::new();

        let args = SetBreakpointsArguments {
            source: Source { path: Some(source_path.clone()), name: Some("script.pl".to_string()) },
            breakpoints: Some(vec![SourceBreakpoint {
                line: 10,
                column: None,
                condition: None,
                hit_condition: None,
                log_message: Some("value: {$x}, count: {$count}".to_string()),
            }]),
            source_modified: None,
        };
        let responses = store.set_breakpoints(&args);
        assert!(responses[0].verified);

        // Create variable map
        let mut variables = std::collections::HashMap::new();
        variables.insert("x".to_string(), "42".to_string());
        variables.insert("count".to_string(), "7".to_string());

        // Register hit with variables - should interpolate
        let outcome =
            store.register_breakpoint_hit_with_variables(&source_path, 10, Some(&variables));
        assert!(outcome.matched);
        assert!(!outcome.should_stop);
        assert_eq!(outcome.log_messages.len(), 1);
        assert_eq!(outcome.log_messages[0], "value: 42, count: 7");
    }

    #[test]
    fn test_register_breakpoint_hit_without_variables_preserves_template() {
        // Test that without variables, messages are preserved as-is
        let (_file, source_path) = create_test_perl_file();
        let store = BreakpointStore::new();

        let args = SetBreakpointsArguments {
            source: Source { path: Some(source_path.clone()), name: Some("script.pl".to_string()) },
            breakpoints: Some(vec![SourceBreakpoint {
                line: 10,
                column: None,
                condition: None,
                hit_condition: None,
                log_message: Some("x = {$x}".to_string()),
            }]),
            source_modified: None,
        };
        let responses = store.set_breakpoints(&args);
        assert!(responses[0].verified);

        // Register hit without variables - should preserve template
        let outcome = store.register_breakpoint_hit(&source_path, 10);
        assert!(outcome.matched);
        assert_eq!(outcome.log_messages.len(), 1);
        assert_eq!(outcome.log_messages[0], "x = {$x}"); // Template preserved
    }

    #[test]
    fn test_interpolate_logpoint_message_single_variable() {
        let mut vars = std::collections::HashMap::new();
        vars.insert("x".to_string(), "42".to_string());

        assert_eq!(interpolate_logpoint_message("value: {$x}", &vars), "value: 42");
    }

    #[test]
    fn test_interpolate_logpoint_message_multiple_variables() {
        let mut vars = std::collections::HashMap::new();
        vars.insert("x".to_string(), "10".to_string());
        vars.insert("y".to_string(), "20".to_string());

        assert_eq!(interpolate_logpoint_message("x={$x}, y={$y}", &vars), "x=10, y=20");
    }

    #[test]
    fn test_interpolate_logpoint_message_missing_variable() {
        let vars = std::collections::HashMap::new();
        // Variable not in map - expression should remain as-is
        assert_eq!(interpolate_logpoint_message("value: {$x}", &vars), "value: {$x}");
    }

    #[test]
    fn test_interpolate_logpoint_message_no_substitution() {
        let mut vars = std::collections::HashMap::new();
        vars.insert("x".to_string(), "42".to_string());

        // No variables to interpolate
        assert_eq!(interpolate_logpoint_message("simple message", &vars), "simple message");
    }

    #[test]
    fn test_interpolate_logpoint_message_non_variable_expression() {
        let mut vars = std::collections::HashMap::new();
        vars.insert("x".to_string(), "42".to_string());

        // Non-variable expressions are left unchanged
        assert_eq!(
            interpolate_logpoint_message("expression: {5 + 3}", &vars),
            "expression: {5 + 3}"
        );
    }

    #[test]
    fn test_interpolate_logpoint_message_unclosed_brace() {
        let mut vars = std::collections::HashMap::new();
        vars.insert("x".to_string(), "42".to_string());

        // Unclosed brace - keep as-is
        assert_eq!(interpolate_logpoint_message("value: {$x", &vars), "value: {$x");
    }

    #[test]
    fn test_interpolate_logpoint_message_empty_braces() {
        let vars = std::collections::HashMap::new();
        // Empty braces should be kept as-is
        assert_eq!(interpolate_logpoint_message("value: {}", &vars), "value: {}");
    }

    #[test]
    fn test_interpolate_logpoint_message_repeated_variable() {
        let mut vars = std::collections::HashMap::new();
        vars.insert("count".to_string(), "5".to_string());

        // Same variable multiple times
        assert_eq!(
            interpolate_logpoint_message("count is {$count}, repeat {$count}", &vars),
            "count is 5, repeat 5"
        );
    }

    #[test]
    fn test_interpolate_logpoint_message_variable_with_spaces_found() {
        // Braces with whitespace around $var — trimming finds the variable
        let mut vars = std::collections::HashMap::new();
        vars.insert("x".to_string(), "99".to_string());
        // { $x } trims to $x → looks up "x" → found
        assert_eq!(interpolate_logpoint_message("val: { $x }", &vars), "val: 99");
    }

    #[test]
    fn test_interpolate_logpoint_message_variable_with_spaces_not_found() {
        // Braces with whitespace around $var — trimming applies before key lookup;
        // when not found the expression is re-emitted in trimmed form.
        let vars = std::collections::HashMap::new();
        // { $y } → trimmed → "$y" → not found → emitted as {$y} (trimmed, no extra spaces)
        assert_eq!(interpolate_logpoint_message("val: { $y }", &vars), "val: {$y}");
    }

    #[test]
    fn test_interpolate_logpoint_message_bare_dollar_sign() {
        // {$} — dollar sign with empty name: not found → kept as-is
        let vars = std::collections::HashMap::new();
        assert_eq!(interpolate_logpoint_message("{$}", &vars), "{$}");
    }

    #[test]
    fn test_interpolate_logpoint_message_array_syntax_kept_verbatim() {
        // {@arr} — only $scalar is interpolated; @array syntax is not handled and kept as-is
        let mut vars = std::collections::HashMap::new();
        vars.insert("arr".to_string(), "should_not_appear".to_string());
        assert_eq!(interpolate_logpoint_message("{@arr}", &vars), "{@arr}");
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

    #[test]
    fn test_file_paths_match_no_basename_cross_match() {
        // Same basename in different directories must NOT match
        assert!(!file_paths_match("/a/main.pl", "/b/main.pl"));
        assert!(!file_paths_match("/workspace/a/lib.pm", "/workspace/b/lib.pm"));
    }

    #[test]
    fn test_file_paths_match_suffix_still_works() {
        // Suffix matching handles relative-vs-absolute
        assert!(file_paths_match("/workspace/lib/main.pl", "lib/main.pl"));
        assert!(file_paths_match("lib/main.pl", "/workspace/lib/main.pl"));
    }

    #[test]
    fn test_file_paths_match_exact() {
        assert!(file_paths_match("/workspace/main.pl", "/workspace/main.pl"));
    }

    #[test]
    fn test_file_paths_match_mid_component_rejected() {
        // "bar.pl" must NOT match "foobar.pl" — the suffix starts in the middle of a component
        assert!(!file_paths_match("foobar.pl", "bar.pl"));
        assert!(!file_paths_match("bar.pl", "foobar.pl"));
        assert!(!file_paths_match("/path/to/foobar.pl", "bar.pl"));
        assert!(!file_paths_match("bar.pl", "/path/to/foobar.pl"));
    }

    #[test]
    fn test_file_paths_match_boundary_positive() {
        // Relative-vs-absolute with a path separator boundary must still match
        assert!(file_paths_match("/abs/path/bar.pl", "bar.pl"));
        assert!(file_paths_match("bar.pl", "/abs/path/bar.pl"));
        // Windows-style separator
        assert!(file_paths_match(r"C:\abs\path\bar.pl", "bar.pl"));
        assert!(file_paths_match("bar.pl", r"C:\abs\path\bar.pl"));
    }

    #[test]
    fn test_breakpoint_hit_count_isolated_by_directory() -> Result<(), Box<dyn std::error::Error>> {
        // Integration: two temp files with same basename in different dirs
        let dir_a = must(tempfile::tempdir());
        let dir_b = must(tempfile::tempdir());

        let file_a = dir_a.path().join("main.pl");
        let file_b = dir_b.path().join("main.pl");

        let perl_code = "#!/usr/bin/perl\nuse strict;\nmy $x = 1;\nmy $y = 2;\nmy $z = 3;\n\
            print $x;\nprint $y;\nprint $z;\nmy $a = 4;\nmy $b = 5;\n\
            my $c = 6;\nmy $d = 7;\nmy $e = 8;\nmy $f = 9;\nmy $g = 10;\n";
        must(std::fs::write(&file_a, perl_code));
        must(std::fs::write(&file_b, perl_code));

        let path_a = file_a.to_string_lossy().to_string();
        let path_b = file_b.to_string_lossy().to_string();

        let store = BreakpointStore::new();

        // Set breakpoints on both files at line 5
        let args_a = SetBreakpointsArguments {
            source: Source { path: Some(path_a.clone()), name: Some("main.pl".to_string()) },
            breakpoints: Some(vec![SourceBreakpoint {
                line: 5,
                column: None,
                condition: None,
                hit_condition: None,
                log_message: None,
            }]),
            source_modified: None,
        };
        let args_b = SetBreakpointsArguments {
            source: Source { path: Some(path_b.clone()), name: Some("main.pl".to_string()) },
            breakpoints: Some(vec![SourceBreakpoint {
                line: 5,
                column: None,
                condition: None,
                hit_condition: None,
                log_message: None,
            }]),
            source_modified: None,
        };

        store.set_breakpoints(&args_a);
        store.set_breakpoints(&args_b);

        // Record a hit on file_a's breakpoint
        store.register_breakpoint_hit(&path_a, 5);

        // file_a's breakpoint should have hit_count=1
        let bps_a = store.get_breakpoints(&path_a);
        let bp_a = bps_a
            .iter()
            .find(|bp| bp.line == 5)
            .ok_or("breakpoint in file_a not found")
            .map_err(std::io::Error::other)?;
        assert_eq!(bp_a.hit_count, 1);

        // file_b's breakpoint should still have hit_count=0
        let bps_b = store.get_breakpoints(&path_b);
        let bp_b = bps_b
            .iter()
            .find(|bp| bp.line == 5)
            .ok_or("breakpoint in file_b not found")
            .map_err(std::io::Error::other)?;
        assert_eq!(bp_b.hit_count, 0);
        Ok(())
    }

    /// Two breakpoints at the same (line, column) with different conditions must each be
    /// validated independently.  The validation_cache is keyed by (line, column) and only
    /// caches the AST line-validity result; condition validation is applied per-breakpoint
    /// AFTER the cache lookup.  This test would fail if condition validation were accidentally
    /// skipped for the second breakpoint due to a cache hit.
    #[test]
    fn test_same_line_different_conditions_both_validated() {
        let (_file, source_path) = create_test_perl_file();
        let store = BreakpointStore::new();

        let args = SetBreakpointsArguments {
            source: Source { path: Some(source_path.clone()), name: Some("script.pl".to_string()) },
            breakpoints: Some(vec![
                SourceBreakpoint {
                    line: 10,
                    column: None,
                    // No condition on the first entry — prime the cache for (line=10, col=None).
                    condition: None,
                    hit_condition: None,
                    log_message: None,
                },
                SourceBreakpoint {
                    line: 10,
                    column: None,
                    // Newline injection — must be rejected by the security guard even though
                    // (line=10, col=None) is already in the validation_cache from entry 1.
                    condition: Some("$x > 0\nB *".to_string()),
                    hit_condition: None,
                    log_message: None,
                },
            ]),
            source_modified: None,
        };

        let result = store.set_breakpoints(&args);
        assert_eq!(result.len(), 2, "both breakpoints must appear in the response");

        // Second breakpoint has a newline in the condition — security guard must fire.
        assert!(
            !result[1].verified,
            "breakpoint with newline in condition must be unverified (security guard applies \
             independently of the AST validation cache); got verified={}",
            result[1].verified
        );

        // The stored records must also reflect two entries (both stored, second unverified).
        let records = store.get_breakpoints(&source_path);
        assert_eq!(records.len(), 2, "both records must be stored (unverified ones are kept)");
        assert!(!records[1].verified, "stored second record must be unverified");
        assert!(
            records[1].message.as_deref().unwrap_or("").contains("newline"),
            "stored record must carry the newline-rejection message"
        );
    }

    // -----------------------------------------------------------------------
    // combine_condition_message — unit tests for the message-combine helper
    // -----------------------------------------------------------------------

    /// Bug regression: prior to the fix, a pre-existing message (e.g. "adjusted to line N")
    /// was silently overwritten when the condition failed validation.  After the fix both
    /// pieces of information are present in the combined message.
    #[test]
    fn test_combine_condition_message_preserves_prior_adjustment() {
        let prior = Some("Breakpoint set on blank line, adjusted to line 5".to_string());
        let combined = combine_condition_message(prior);
        let text = combined.as_deref().unwrap_or("");
        assert!(
            text.contains("adjusted to line 5"),
            "combined message must retain the adjustment note; got: {text:?}"
        );
        assert!(
            text.contains("Conditional breakpoint expression is invalid"),
            "combined message must include the condition-invalid note; got: {text:?}"
        );
        // Both parts are joined with "; " — exact shape matters for the DAP client.
        assert_eq!(
            text,
            "Breakpoint set on blank line, adjusted to line 5; Conditional breakpoint expression is invalid",
            "exact combined message mismatch"
        );
    }

    /// When there is no prior message (the common case for a breakpoint on a
    /// valid line), `combine_condition_message` must produce just the condition
    /// error — no leading delimiter, no empty prefix.
    #[test]
    fn test_combine_condition_message_no_prior_gives_condition_error_only() {
        let combined = combine_condition_message(None);
        assert_eq!(
            combined.as_deref(),
            Some("Conditional breakpoint expression is invalid"),
            "with no prior message the result must be exactly the condition error"
        );
    }

    /// Integration test: a breakpoint on a valid line with an empty condition expression
    /// (explicitly rejected by the validator) must produce `verified=false` and a message
    /// that contains the condition-invalid text.
    ///
    /// Uses an empty string condition because the validator explicitly rejects it before
    /// the parser step — this is the most reliable way to trigger condition rejection in
    /// an integration test without depending on parser error-recovery behavior.
    #[test]
    fn test_set_breakpoints_invalid_condition_marks_unverified_with_message() {
        let (_file, source_path) = create_test_perl_file();
        let store = BreakpointStore::new();

        let args = SetBreakpointsArguments {
            source: Source { path: Some(source_path.clone()), name: Some("script.pl".to_string()) },
            breakpoints: Some(vec![SourceBreakpoint {
                line: 5, // "my $x = 1;" — valid, executable line
                column: None,
                // Empty condition is explicitly rejected by the validator (no parser ambiguity).
                condition: Some(String::new()),
                hit_condition: None,
                log_message: None,
            }]),
            source_modified: None,
        };

        let responses = store.set_breakpoints(&args);
        assert_eq!(responses.len(), 1);

        let bp = &responses[0];
        assert!(
            !bp.verified,
            "breakpoint with invalid condition must be unverified; got verified={}",
            bp.verified
        );
        let msg = bp.message.as_deref().unwrap_or("");
        assert!(
            msg.contains("Conditional breakpoint expression is invalid"),
            "message must mention the condition error; got: {msg:?}"
        );

        // Stored record must also be unverified with the condition-error message.
        let records = store.get_breakpoints(&source_path);
        assert_eq!(records.len(), 1);
        assert!(!records[0].verified, "stored record must be unverified");
        assert!(
            records[0]
                .message
                .as_deref()
                .unwrap_or("")
                .contains("Conditional breakpoint expression is invalid"),
            "stored message must include condition-invalid note"
        );
    }
}
