//! Breakpoint validation using AST analysis
//!
//! This module provides AST-based validation for breakpoint locations.
//! It checks whether a given line number contains executable code or is
//! a non-executable location like a comment, blank line, or heredoc interior.

use crate::BreakpointError;
use perl_parser::Parser;
use perl_parser::ast::{Node, NodeKind};
use ropey::Rope;

/// Reason why a breakpoint was rejected or adjusted
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationReason {
    /// The line is blank (whitespace only)
    BlankLine,
    /// The line contains only comments
    CommentLine,
    /// The breakpoint is inside heredoc content
    HeredocInterior,
    /// The line number exceeds the file length
    LineOutOfRange,
    /// Unable to parse the source file
    ParseError,
}

impl std::fmt::Display for ValidationReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationReason::BlankLine => write!(f, "Breakpoint set on blank line"),
            ValidationReason::CommentLine => write!(f, "Breakpoint set on comment or blank line"),
            ValidationReason::HeredocInterior => write!(f, "Breakpoint set inside heredoc content"),
            ValidationReason::LineOutOfRange => write!(f, "Line number exceeds file length"),
            ValidationReason::ParseError => write!(f, "Unable to parse source file"),
        }
    }
}

/// Result of breakpoint validation
#[derive(Debug, Clone)]
pub struct BreakpointValidation {
    /// Whether the breakpoint is valid and can be set
    pub verified: bool,
    /// The line number (may be adjusted to nearest valid line)
    pub line: i64,
    /// Column number (optional)
    pub column: Option<i64>,
    /// Reason for rejection if not verified
    pub reason: Option<ValidationReason>,
    /// Human-readable message describing the validation result
    pub message: Option<String>,
}

impl BreakpointValidation {
    /// Create a successful validation result
    pub fn verified(line: i64, column: Option<i64>) -> Self {
        Self { verified: true, line, column, reason: None, message: None }
    }

    /// Create a failed validation result
    pub fn rejected(line: i64, reason: ValidationReason) -> Self {
        let message = Some(reason.to_string());
        Self { verified: false, line, column: None, reason: Some(reason), message }
    }

    /// Create a validation result with an adjusted line
    pub fn adjusted(new_line: i64, reason: ValidationReason) -> Self {
        let message = Some(format!("{}, adjusted to line {}", reason, new_line));
        Self { verified: true, line: new_line, column: None, reason: Some(reason), message }
    }
}

/// Trait for breakpoint validation
pub trait BreakpointValidator {
    /// Validate a breakpoint at the given line number (1-based)
    fn validate(&self, line: i64) -> BreakpointValidation;

    /// Validate a breakpoint with optional column
    fn validate_with_column(&self, line: i64, column: Option<i64>) -> BreakpointValidation;

    /// Check if a line contains executable code
    fn is_executable_line(&self, line: i64) -> bool;
}

/// AST-based breakpoint validator
///
/// Uses the Perl parser to build an AST and validate breakpoint locations
/// against the parsed structure.
pub struct AstBreakpointValidator {
    /// The parsed AST
    ast: Node,
    /// Rope for efficient line/byte position mapping
    rope: Rope,
    /// Original source code
    source: String,
}

impl AstBreakpointValidator {
    /// Create a new validator for the given source code
    ///
    /// # Arguments
    ///
    /// * `source` - The Perl source code to validate against
    ///
    /// # Errors
    ///
    /// Returns an error if the source cannot be parsed.
    pub fn new(source: &str) -> Result<Self, BreakpointError> {
        let mut parser = Parser::new(source);
        let ast = parser.parse().map_err(|e| BreakpointError::ParseError(format!("{:?}", e)))?;
        let rope = Rope::from_str(source);
        Ok(Self { ast, rope, source: source.to_string() })
    }

    /// Get the line range (start byte, end byte) for a given 1-based line number
    fn line_byte_range(&self, line: i64) -> Option<(usize, usize)> {
        let line_idx = (line - 1).max(0) as usize;
        if line_idx >= self.rope.len_lines() {
            return None;
        }

        let line_start = self.rope.line_to_byte(line_idx);
        let line_end = if line_idx + 1 < self.rope.len_lines() {
            self.rope.line_to_byte(line_idx + 1)
        } else {
            self.rope.len_bytes()
        };

        Some((line_start, line_end))
    }

    /// Check if a line contains only comments or whitespace
    fn is_comment_or_blank_line(&self, line_start: usize, line_end: usize) -> bool {
        let line_text = &self.source[line_start..line_end.min(self.source.len())];

        // Fast path: Check if blank (only whitespace)
        if line_text.trim().is_empty() {
            return true;
        }

        // Fast path: Check if comment (starts with # after whitespace)
        if line_text.trim_start().starts_with('#') {
            return true;
        }

        // AST-based validation: Check if line contains only comment nodes
        self.has_only_comments_in_range(line_start, line_end)
    }

    /// Check if all nodes in a range are comments
    ///
    /// Note: Comments are stripped during lexing and not represented in the AST.
    /// The fast path in `is_comment_or_blank_line` handles comment detection.
    /// This function checks if there are no executable nodes in the range.
    fn has_only_comments_in_range(&self, start: usize, end: usize) -> bool {
        self.has_only_comments_in_range_node(&self.ast, start, end)
    }

    fn has_only_comments_in_range_node(&self, node: &Node, start: usize, end: usize) -> bool {
        // Check if node overlaps with line range
        if node.location.start >= end || node.location.end <= start {
            return false;
        }

        match &node.kind {
            NodeKind::Program { statements } => {
                // Get all nodes that overlap with the line range
                let nodes_in_range: Vec<_> = statements
                    .iter()
                    .filter(|s| s.location.start < end && s.location.end > start)
                    .collect();

                // If no AST nodes in range, it's a blank/comment line
                nodes_in_range.is_empty()
            }
            // Any other node type means there's executable code
            _ => false,
        }
    }

    /// Check if a byte offset is inside a heredoc interior (body content)
    fn is_inside_heredoc_interior(&self, byte_offset: usize) -> bool {
        self.is_inside_heredoc_interior_node(&self.ast, byte_offset)
    }

    #[allow(clippy::only_used_in_recursion)]
    fn is_inside_heredoc_interior_node(&self, node: &Node, byte_offset: usize) -> bool {
        // Check if this is a heredoc with a body span containing the offset
        if let NodeKind::Heredoc { body_span: Some(span), .. } = &node.kind {
            if byte_offset >= span.start && byte_offset < span.end {
                return true;
            }
        }

        // Recursively check all children
        let mut found = false;
        node.for_each_child(|child| {
            if !found && self.is_inside_heredoc_interior_node(child, byte_offset) {
                found = true;
            }
        });
        found
    }
}

impl BreakpointValidator for AstBreakpointValidator {
    fn validate(&self, line: i64) -> BreakpointValidation {
        self.validate_with_column(line, None)
    }

    fn validate_with_column(&self, line: i64, column: Option<i64>) -> BreakpointValidation {
        // Get byte range for the line
        let Some((line_start, line_end)) = self.line_byte_range(line) else {
            return BreakpointValidation::rejected(line, ValidationReason::LineOutOfRange);
        };

        // Validation 1: Inside heredoc interior
        // Check BEFORE comment/blank check because heredoc interior lines have no AST nodes
        // and would otherwise be incorrectly classified as blank/comment lines
        if self.is_inside_heredoc_interior(line_start) {
            return BreakpointValidation::rejected(line, ValidationReason::HeredocInterior);
        }

        // Validation 2: Comment or blank line
        if self.is_comment_or_blank_line(line_start, line_end) {
            // Check if the line is truly blank or just a comment
            let line_text = &self.source[line_start..line_end.min(self.source.len())];
            let reason = if line_text.trim().is_empty() {
                ValidationReason::BlankLine
            } else {
                ValidationReason::CommentLine
            };
            return BreakpointValidation::rejected(line, reason);
        }

        // Breakpoint is valid
        BreakpointValidation::verified(line, column)
    }

    fn is_executable_line(&self, line: i64) -> bool {
        self.validate(line).verified
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::must;

    #[test]
    fn test_validate_executable_line() {
        let source = "my $x = 1;\n";
        let validator = must(AstBreakpointValidator::new(source));

        let result = validator.validate(1);
        assert!(result.verified);
        assert_eq!(result.line, 1);
        assert!(result.reason.is_none());
    }

    #[test]
    fn test_validate_comment_line() {
        let source = "# This is a comment\nmy $x = 1;\n";
        let validator = must(AstBreakpointValidator::new(source));

        let result = validator.validate(1);
        assert!(!result.verified);
        assert_eq!(result.reason, Some(ValidationReason::CommentLine));
    }

    #[test]
    fn test_validate_blank_line() {
        let source = "my $x = 1;\n\nmy $y = 2;\n";
        let validator = must(AstBreakpointValidator::new(source));

        let result = validator.validate(2);
        assert!(!result.verified);
        assert_eq!(result.reason, Some(ValidationReason::BlankLine));
    }

    #[test]
    fn test_validate_line_out_of_range() {
        let source = "my $x = 1;\n";
        let validator = must(AstBreakpointValidator::new(source));

        let result = validator.validate(100);
        assert!(!result.verified);
        assert_eq!(result.reason, Some(ValidationReason::LineOutOfRange));
    }

    #[test]
    fn test_is_executable_line() {
        let source = "# comment\nmy $x = 1;\n\nmy $y = 2;\n";
        let validator = must(AstBreakpointValidator::new(source));

        assert!(!validator.is_executable_line(1)); // comment
        assert!(validator.is_executable_line(2)); // code
        assert!(!validator.is_executable_line(3)); // blank
        assert!(validator.is_executable_line(4)); // code
    }
}
