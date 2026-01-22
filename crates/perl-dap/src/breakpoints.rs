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
use perl_parser::ast::{Node, NodeKind};
use perl_parser::Parser;
use ropey::Rope;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ============= AST Validation Utilities (AC7) =============

/// Check if a line contains only comments or whitespace
fn is_comment_or_blank_line(ast: &Node, line_start: usize, line_end: usize, source: &str) -> bool {
    // Fast path: Check if blank (only whitespace)
    let line_text = &source[line_start..line_end.min(source.len())];
    if line_text.trim().is_empty() {
        return true;
    }

    // Fast path: Check if comment (starts with # after whitespace)
    if line_text.trim_start().starts_with('#') {
        return true;
    }

    // AST-based validation: Check if line contains only comment nodes
    has_only_comments_in_range(ast, line_start, line_end)
}

/// Check if all nodes in a range are comments
fn has_only_comments_in_range(node: &Node, start: usize, end: usize) -> bool {
    // Check if node overlaps with line range
    if node.location.start >= end || node.location.end <= start {
        return false;
    }

    match &node.kind {
        NodeKind::Program { statements } => {
            // Get all nodes that overlap with the line range
            let nodes_in_range: Vec<_> =
                statements.iter().filter(|s| s.location.start < end && s.location.end > start).collect();

            if nodes_in_range.is_empty() {
                return true; // No nodes = blank line
            }

            // All nodes must be comments
            nodes_in_range.iter().all(|s| matches!(s.kind, NodeKind::Comment { .. }))
        }
        NodeKind::Comment { .. } => true,
        _ => false,
    }
}

/// Check if a byte offset is inside a heredoc interior
fn is_inside_heredoc_interior(node: &Node, byte_offset: usize) -> bool {
    // Check if offset is within this node
    if byte_offset < node.location.start || byte_offset >= node.location.end {
        // Check children
        return match &node.kind {
            NodeKind::Program { statements } => {
                statements.iter().any(|s| is_inside_heredoc_interior(s, byte_offset))
            }
            NodeKind::Heredoc { .. } => {
                // The heredoc node itself - offset is in range
                true
            }
            _ => {
                // Recursively check other node types with children
                false
            }
        };
    }

    // Node contains the offset, check if it's a heredoc
    matches!(node.kind, NodeKind::Heredoc { .. })
}

/// Validate a breakpoint against the AST
///
/// Returns (verified, message) where:
/// - verified: true if the breakpoint is valid
/// - message: error/warning message if not verified
fn validate_breakpoint_line(source: &str, line: i64) -> (bool, Option<String>) {
    // Parse the source file
    let mut parser = Parser::new(source);
    let ast = match parser.parse() {
        Ok(ast) => ast,
        Err(_) => {
            // Parse error - allow breakpoint but mark as unverified
            return (false, Some("Unable to parse source file".to_string()));
        }
    };

    // Build rope for line mapping
    let rope = Rope::from_str(source);

    // Convert 1-based line to 0-based
    let line_idx = (line - 1).max(0) as usize;

    // Check if line is valid
    if line_idx >= rope.len_lines() {
        return (false, Some("Line number exceeds file length".to_string()));
    }

    // Get byte range for the line
    let line_start = rope.line_to_byte(line_idx);
    let line_end = if line_idx + 1 < rope.len_lines() {
        rope.line_to_byte(line_idx + 1)
    } else {
        rope.len_bytes()
    };

    // Validation 1: Comment or blank line
    if is_comment_or_blank_line(&ast, line_start, line_end, source) {
        return (false, Some("Breakpoint set on comment or blank line".to_string()));
    }

    // Validation 2: Inside heredoc interior
    // Check the start of the line
    if is_inside_heredoc_interior(&ast, line_start) {
        return (false, Some("Breakpoint set inside heredoc content".to_string()));
    }

    // Breakpoint is valid
    (true, None)
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

        // Read source file for AST validation (AC7)
        let source_content = std::fs::read_to_string(&source_path).ok();

        // Create new breakpoint records
        let mut records = Vec::new();
        for bp in &source_breakpoints {
            let id = *next_id;
            *next_id += 1;

            // AC7: AST-based breakpoint validation
            let (verified, message) = if let Some(ref content) = source_content {
                validate_breakpoint_line(content, bp.line)
            } else {
                // Can't read file - mark as unverified but still create breakpoint
                (false, Some("Unable to read source file".to_string()))
            };

            let record = BreakpointRecord {
                id,
                line: bp.line,
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

    #[test]
    fn test_breakpoint_store_new() {
        let store = BreakpointStore::new();
        let breakpoints = store.get_breakpoints("/workspace/test.pl");
        assert_eq!(breakpoints.len(), 0);
    }

    #[test]
    fn test_set_breakpoints_creates_records() {
        let store = BreakpointStore::new();
        let args = SetBreakpointsArguments {
            source: Source {
                path: Some("/workspace/script.pl".to_string()),
                name: Some("script.pl".to_string()),
            },
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
        let store = BreakpointStore::new();
        let source_path = "/workspace/script.pl";

        // Set initial breakpoints
        let args1 = SetBreakpointsArguments {
            source: Source {
                path: Some(source_path.to_string()),
                name: Some("script.pl".to_string()),
            },
            breakpoints: Some(vec![SourceBreakpoint { line: 10, column: None, condition: None }]),
            source_modified: None,
        };
        store.set_breakpoints(&args1);

        // Replace with new breakpoints
        let args2 = SetBreakpointsArguments {
            source: Source {
                path: Some(source_path.to_string()),
                name: Some("script.pl".to_string()),
            },
            breakpoints: Some(vec![
                SourceBreakpoint { line: 20, column: None, condition: None },
                SourceBreakpoint { line: 30, column: None, condition: None },
            ]),
            source_modified: None,
        };
        let breakpoints = store.set_breakpoints(&args2);

        // Should have only the new breakpoints
        assert_eq!(breakpoints.len(), 2);
        assert_eq!(breakpoints[0].line, 20);
        assert_eq!(breakpoints[1].line, 30);

        // Verify stored breakpoints
        let stored = store.get_breakpoints(source_path);
        assert_eq!(stored.len(), 2);
    }

    #[test]
    fn test_set_breakpoints_unique_ids() {
        let store = BreakpointStore::new();
        let args = SetBreakpointsArguments {
            source: Source {
                path: Some("/workspace/script.pl".to_string()),
                name: Some("script.pl".to_string()),
            },
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
        let store = BreakpointStore::new();
        let args = SetBreakpointsArguments {
            source: Source {
                path: Some("/workspace/script.pl".to_string()),
                name: Some("script.pl".to_string()),
            },
            breakpoints: Some(vec![
                SourceBreakpoint { line: 100, column: None, condition: None },
                SourceBreakpoint { line: 50, column: None, condition: None },
                SourceBreakpoint { line: 75, column: None, condition: None },
            ]),
            source_modified: None,
        };

        let breakpoints = store.set_breakpoints(&args);

        // Order must match request (not sorted by line number)
        assert_eq!(breakpoints[0].line, 100);
        assert_eq!(breakpoints[1].line, 50);
        assert_eq!(breakpoints[2].line, 75);
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
    fn test_get_breakpoint_by_id() {
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
        assert_eq!(record.unwrap().line, 10);

        // Non-existent ID
        let not_found = store.get_breakpoint_by_id(999999);
        assert!(not_found.is_none());
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
}
