//! Breakpoint validation using AST analysis
//!
//! This module provides AST-based validation for breakpoint locations.
//! It checks whether a given line number contains executable code or is
//! a non-executable location like a comment, blank line, or heredoc interior.

use super::BreakpointError;
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
    /// The line is inside a POD documentation section
    PodLine,
    /// The line number exceeds the file length
    LineOutOfRange,
    /// Unable to parse the source file
    ParseError,
    /// A conditional breakpoint expression is invalid
    InvalidCondition,
}

impl std::fmt::Display for ValidationReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationReason::BlankLine => write!(f, "Breakpoint set on blank line"),
            ValidationReason::CommentLine => write!(f, "Breakpoint set on comment or blank line"),
            ValidationReason::HeredocInterior => write!(f, "Breakpoint set inside heredoc content"),
            ValidationReason::PodLine => write!(f, "Breakpoint set inside POD documentation"),
            ValidationReason::LineOutOfRange => write!(f, "Line number exceeds file length"),
            ValidationReason::ParseError => write!(f, "Unable to parse source file"),
            ValidationReason::InvalidCondition => {
                write!(f, "Conditional breakpoint expression is invalid")
            }
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

    /// Validate a conditional breakpoint expression
    ///
    /// Checks that the condition string is a syntactically valid Perl expression
    /// that can be used as a breakpoint condition.
    fn validate_condition(&self, line: i64, condition: &str) -> BreakpointValidation;
}

/// A byte range representing start (inclusive) and end (exclusive) offsets
#[derive(Debug, Clone, Copy)]
struct ByteRange {
    start: usize,
    end: usize,
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
    /// Byte ranges of POD sections (=head1 ... =cut)
    pod_regions: Vec<ByteRange>,
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
        let pod_regions = Self::find_pod_regions(source);
        Ok(Self { ast, rope, source: source.to_string(), pod_regions })
    }

    /// Scan source text for POD documentation regions.
    ///
    /// POD begins with a line matching `=<directive>` (e.g. `=head1`, `=pod`, `=over`)
    /// at column 0 (or after a newline) and ends with `=cut` on its own line.
    /// If no `=cut` is found the POD extends to EOF.
    fn find_pod_regions(source: &str) -> Vec<ByteRange> {
        let mut regions = Vec::new();
        let mut pod_start: Option<usize> = None;
        let mut offset = 0;

        for line in source.split('\n') {
            let trimmed = line.trim_end_matches('\r');
            if pod_start.is_some() {
                // We are inside a POD section -- look for =cut
                if trimmed == "=cut" {
                    let end = offset + line.len();
                    if let Some(start) = pod_start.take() {
                        regions.push(ByteRange { start, end });
                    }
                }
            } else if Self::is_pod_directive(trimmed) {
                pod_start = Some(offset);
            }
            offset += line.len() + 1; // +1 for the '\n' delimiter
        }

        // If POD was never closed, extend to EOF
        if let Some(start) = pod_start {
            regions.push(ByteRange { start, end: source.len() });
        }

        regions
    }

    /// Returns `true` when `line` (already trimmed of trailing CR/LF) looks
    /// like a POD directive that opens a documentation section.
    fn is_pod_directive(line: &str) -> bool {
        // POD directives: =head1-4, =pod, =over, =back, =begin, =end, =for, =encoding, =item
        // Must start with '=' followed by a letter.
        if !line.starts_with('=') {
            return false;
        }
        let after_eq = &line[1..];
        // Must start with an ASCII letter to be a POD directive
        after_eq.starts_with(|c: char| c.is_ascii_alphabetic())
    }

    /// Check if a byte offset falls inside any POD region
    fn is_inside_pod_region(&self, byte_offset: usize) -> bool {
        self.pod_regions.iter().any(|r| byte_offset >= r.start && byte_offset < r.end)
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

        // Validation 2: Inside POD documentation section
        // Check BEFORE comment/blank because POD lines have no AST nodes and would otherwise
        // be incorrectly classified as blank or comment lines
        if self.is_inside_pod_region(line_start) {
            return BreakpointValidation::rejected(line, ValidationReason::PodLine);
        }

        // Validation 3: Comment or blank line
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

    fn validate_condition(&self, line: i64, condition: &str) -> BreakpointValidation {
        // First validate the line itself
        let line_result = self.validate(line);
        if !line_result.verified {
            return line_result;
        }

        // Validate the condition expression
        let trimmed = condition.trim();

        // Empty condition is always invalid
        if trimmed.is_empty() {
            return BreakpointValidation::rejected(line, ValidationReason::InvalidCondition);
        }

        // Reject conditions containing dangerous constructs that should not
        // appear in a debug-time expression evaluated on every hit.
        if Self::condition_has_dangerous_construct(trimmed) {
            return BreakpointValidation::rejected(line, ValidationReason::InvalidCondition);
        }

        // Try to parse the condition as a Perl expression.
        // We wrap it in a statement context so the parser can handle it.
        let wrapped = format!("if ({trimmed}) {{ 1; }}");
        let mut parser = Parser::new(&wrapped);
        match parser.parse() {
            Ok(_) => BreakpointValidation::verified(line, None),
            Err(_) => BreakpointValidation::rejected(line, ValidationReason::InvalidCondition),
        }
    }
}

impl AstBreakpointValidator {
    /// Detect dangerous constructs that should not be used in breakpoint conditions.
    ///
    /// Breakpoint conditions are evaluated on every hit and should be pure
    /// expressions without side effects that could alter program state.
    fn condition_has_dangerous_construct(condition: &str) -> bool {
        // System/exec calls
        if condition.contains("system(")
            || condition.contains("exec(")
            || condition.contains("qx(")
            || condition.contains("qx{")
            || condition.contains("qx/")
        {
            return true;
        }

        // Backtick command execution
        if condition.contains('`') {
            return true;
        }

        // File operations that mutate state
        if condition.contains("unlink(")
            || condition.contains("rename(")
            || condition.contains("rmdir(")
            || condition.contains("mkdir(")
        {
            return true;
        }

        // eval string (eval BLOCK is less dangerous but string eval can do anything)
        // We check for `eval "` or `eval '` or `eval $` patterns
        let eval_pattern = condition.find("eval");
        if let Some(idx) = eval_pattern {
            let after = &condition[idx + 4..];
            let after_trimmed = after.trim_start();
            // eval followed by a string/variable (not a block) is dangerous
            if after_trimmed.starts_with('"')
                || after_trimmed.starts_with('\'')
                || after_trimmed.starts_with('$')
            {
                return true;
            }
        }

        false
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

    // -----------------------------------------------------------------------
    // POD line detection
    // -----------------------------------------------------------------------

    #[test]
    fn test_pod_head1_line_rejected() {
        let source = "my $x = 1;\n\n=head1 NAME\n\nSome pod text\n\n=cut\n\nmy $y = 2;\n";
        let validator = must(AstBreakpointValidator::new(source));

        // =head1 line
        let result = validator.validate(3);
        assert!(!result.verified);
        assert_eq!(result.reason, Some(ValidationReason::PodLine));
    }

    #[test]
    fn test_pod_body_text_rejected() {
        let source = "my $x = 1;\n\n=head1 NAME\n\nSome pod text\n\n=cut\n\nmy $y = 2;\n";
        let validator = must(AstBreakpointValidator::new(source));

        // "Some pod text" line (line 5)
        let result = validator.validate(5);
        assert!(!result.verified);
        assert_eq!(result.reason, Some(ValidationReason::PodLine));
    }

    #[test]
    fn test_pod_cut_line_rejected() {
        let source = "my $x = 1;\n\n=head1 NAME\n\nSome pod text\n\n=cut\n\nmy $y = 2;\n";
        let validator = must(AstBreakpointValidator::new(source));

        // =cut line (line 7)
        let result = validator.validate(7);
        assert!(!result.verified);
        assert_eq!(result.reason, Some(ValidationReason::PodLine));
    }

    #[test]
    fn test_code_after_pod_is_executable() {
        let source = "my $x = 1;\n\n=head1 NAME\n\nSome pod text\n\n=cut\n\nmy $y = 2;\n";
        let validator = must(AstBreakpointValidator::new(source));

        // Code before POD (line 1)
        assert!(validator.is_executable_line(1));
        // Code after POD (line 9)
        assert!(validator.is_executable_line(9));
    }

    #[test]
    fn test_pod_without_cut_extends_to_eof() {
        let source = "my $x = 1;\n=pod\nThis is pod documentation\nThat never ends\n";
        let validator = must(AstBreakpointValidator::new(source));

        assert!(validator.is_executable_line(1));
        let r2 = validator.validate(2);
        assert!(!r2.verified);
        assert_eq!(r2.reason, Some(ValidationReason::PodLine));
        let r3 = validator.validate(3);
        assert!(!r3.verified);
        assert_eq!(r3.reason, Some(ValidationReason::PodLine));
        let r4 = validator.validate(4);
        assert!(!r4.verified);
        assert_eq!(r4.reason, Some(ValidationReason::PodLine));
    }

    #[test]
    fn test_multiple_pod_sections() {
        let source = "my $a = 1;\n\n=head1 SYNOPSIS\n\nFirst section\n\n=cut\n\nmy $b = 2;\n\n=head2 METHODS\n\nSecond section\n\n=cut\n\nmy $c = 3;\n";
        let validator = must(AstBreakpointValidator::new(source));

        assert!(validator.is_executable_line(1)); // my $a = 1;
        assert_eq!(validator.validate(3).reason, Some(ValidationReason::PodLine)); // =head1
        assert_eq!(validator.validate(5).reason, Some(ValidationReason::PodLine)); // First section
        assert_eq!(validator.validate(7).reason, Some(ValidationReason::PodLine)); // =cut
        assert!(validator.is_executable_line(9)); // my $b = 2;
        assert_eq!(validator.validate(11).reason, Some(ValidationReason::PodLine)); // =head2
        assert!(validator.is_executable_line(17)); // my $c = 3;
    }

    // -----------------------------------------------------------------------
    // Conditional breakpoint validation
    // -----------------------------------------------------------------------

    #[test]
    fn test_condition_valid_comparison() {
        let source = "my $x = 1;\nmy $y = 2;\n";
        let validator = must(AstBreakpointValidator::new(source));

        let result = validator.validate_condition(1, "$x > 5");
        assert!(result.verified);
    }

    #[test]
    fn test_condition_valid_equality() {
        let source = "my $x = 1;\n";
        let validator = must(AstBreakpointValidator::new(source));

        let result = validator.validate_condition(1, "$x == 42");
        assert!(result.verified);
    }

    #[test]
    fn test_condition_valid_string_eq() {
        let source = "my $name = 'test';\n";
        let validator = must(AstBreakpointValidator::new(source));

        let result = validator.validate_condition(1, "$name eq 'hello'");
        assert!(result.verified);
    }

    #[test]
    fn test_condition_empty_rejected() {
        let source = "my $x = 1;\n";
        let validator = must(AstBreakpointValidator::new(source));

        let result = validator.validate_condition(1, "");
        assert!(!result.verified);
        assert_eq!(result.reason, Some(ValidationReason::InvalidCondition));
    }

    #[test]
    fn test_condition_whitespace_only_rejected() {
        let source = "my $x = 1;\n";
        let validator = must(AstBreakpointValidator::new(source));

        let result = validator.validate_condition(1, "   ");
        assert!(!result.verified);
        assert_eq!(result.reason, Some(ValidationReason::InvalidCondition));
    }

    #[test]
    fn test_condition_system_call_rejected() {
        let source = "my $x = 1;\n";
        let validator = must(AstBreakpointValidator::new(source));

        let result = validator.validate_condition(1, "system('rm -rf /')");
        assert!(!result.verified);
        assert_eq!(result.reason, Some(ValidationReason::InvalidCondition));
    }

    #[test]
    fn test_condition_backtick_rejected() {
        let source = "my $x = 1;\n";
        let validator = must(AstBreakpointValidator::new(source));

        let result = validator.validate_condition(1, "`ls`");
        assert!(!result.verified);
        assert_eq!(result.reason, Some(ValidationReason::InvalidCondition));
    }

    #[test]
    fn test_condition_exec_rejected() {
        let source = "my $x = 1;\n";
        let validator = must(AstBreakpointValidator::new(source));

        let result = validator.validate_condition(1, "exec('/bin/sh')");
        assert!(!result.verified);
        assert_eq!(result.reason, Some(ValidationReason::InvalidCondition));
    }

    #[test]
    fn test_condition_unlink_rejected() {
        let source = "my $x = 1;\n";
        let validator = must(AstBreakpointValidator::new(source));

        let result = validator.validate_condition(1, "unlink('/tmp/foo')");
        assert!(!result.verified);
        assert_eq!(result.reason, Some(ValidationReason::InvalidCondition));
    }

    #[test]
    fn test_condition_eval_string_rejected() {
        let source = "my $x = 1;\n";
        let validator = must(AstBreakpointValidator::new(source));

        let result = validator.validate_condition(1, "eval \"dangerous code\"");
        assert!(!result.verified);
        assert_eq!(result.reason, Some(ValidationReason::InvalidCondition));
    }

    #[test]
    fn test_condition_on_invalid_line_rejected() {
        let source = "# comment\nmy $x = 1;\n";
        let validator = must(AstBreakpointValidator::new(source));

        // Line 1 is a comment, condition cannot be set there
        let result = validator.validate_condition(1, "$x > 0");
        assert!(!result.verified);
        assert_eq!(result.reason, Some(ValidationReason::CommentLine));
    }

    #[test]
    fn test_condition_defined_check() {
        let source = "my $x = undef;\n";
        let validator = must(AstBreakpointValidator::new(source));

        let result = validator.validate_condition(1, "defined($x)");
        assert!(result.verified);
    }

    #[test]
    fn test_condition_logical_operators() {
        let source = "my $x = 1;\n";
        let validator = must(AstBreakpointValidator::new(source));

        let result = validator.validate_condition(1, "$x > 0 && $x < 100");
        assert!(result.verified);
    }

    // -----------------------------------------------------------------------
    // POD region detection internals
    // -----------------------------------------------------------------------

    #[test]
    fn test_is_pod_directive_basic() {
        assert!(AstBreakpointValidator::is_pod_directive("=head1 NAME"));
        assert!(AstBreakpointValidator::is_pod_directive("=head2 METHODS"));
        assert!(AstBreakpointValidator::is_pod_directive("=pod"));
        assert!(AstBreakpointValidator::is_pod_directive("=cut"));
        assert!(AstBreakpointValidator::is_pod_directive("=over 4"));
        assert!(AstBreakpointValidator::is_pod_directive("=back"));
        assert!(AstBreakpointValidator::is_pod_directive("=begin html"));
        assert!(AstBreakpointValidator::is_pod_directive("=end html"));
        assert!(AstBreakpointValidator::is_pod_directive("=for text"));
        assert!(AstBreakpointValidator::is_pod_directive("=encoding utf8"));
        assert!(AstBreakpointValidator::is_pod_directive("=item *"));
    }

    #[test]
    fn test_is_pod_directive_rejects_non_pod() {
        assert!(!AstBreakpointValidator::is_pod_directive("my $x = 1;"));
        assert!(!AstBreakpointValidator::is_pod_directive("# comment"));
        assert!(!AstBreakpointValidator::is_pod_directive(""));
        assert!(!AstBreakpointValidator::is_pod_directive("=123"));
        assert!(!AstBreakpointValidator::is_pod_directive("=="));
    }

    #[test]
    fn test_find_pod_regions_empty() {
        let regions = AstBreakpointValidator::find_pod_regions("my $x = 1;\n");
        assert!(regions.is_empty());
    }

    #[test]
    fn test_find_pod_regions_single_section() {
        let source = "my $x = 1;\n=head1 NAME\nTest\n=cut\nmy $y = 2;\n";
        let regions = AstBreakpointValidator::find_pod_regions(source);
        assert_eq!(regions.len(), 1);
        // The region should cover from "=head1" through "=cut"
        let text = &source[regions[0].start..regions[0].end];
        assert!(text.starts_with("=head1"));
        assert!(text.ends_with("=cut"));
    }

    #[test]
    fn test_find_pod_regions_unclosed() {
        let source = "my $x = 1;\n=pod\nSome docs\n";
        let regions = AstBreakpointValidator::find_pod_regions(source);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].end, source.len());
    }
}
