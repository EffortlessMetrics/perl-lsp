//! Comprehensive tests for perl-dap-breakpoint crate.
//!
//! Covers: validator construction, line classification, column handling,
//! BreakpointValidation constructors, ValidationReason Display,
//! BreakpointError Display, heredoc interiors, suggestion engine edge cases,
//! and the public re-exports from lib.rs.

use perl_dap::breakpoint::{
    AstBreakpointValidator, BreakpointError, BreakpointValidation, BreakpointValidator,
    SearchDirection, ValidationReason, find_nearest_valid_line,
};
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// BreakpointValidation constructors
// ---------------------------------------------------------------------------

#[test]
fn validation_verified_sets_fields() -> Result<(), Box<dyn std::error::Error>> {
    let v = BreakpointValidation::verified(5, Some(10));
    assert!(v.verified);
    assert_eq!(v.line, 5);
    assert_eq!(v.column, Some(10));
    assert!(v.reason.is_none());
    assert!(v.message.is_none());
    Ok(())
}

#[test]
fn validation_verified_no_column() -> Result<(), Box<dyn std::error::Error>> {
    let v = BreakpointValidation::verified(1, None);
    assert!(v.verified);
    assert_eq!(v.column, None);
    Ok(())
}

#[test]
fn validation_rejected_sets_fields() -> Result<(), Box<dyn std::error::Error>> {
    let v = BreakpointValidation::rejected(3, ValidationReason::BlankLine);
    assert!(!v.verified);
    assert_eq!(v.line, 3);
    assert!(v.column.is_none());
    assert_eq!(v.reason, Some(ValidationReason::BlankLine));
    assert!(v.message.is_some());
    Ok(())
}

#[test]
fn validation_adjusted_sets_fields() -> Result<(), Box<dyn std::error::Error>> {
    let v = BreakpointValidation::adjusted(7, ValidationReason::CommentLine);
    assert!(v.verified);
    assert_eq!(v.line, 7);
    assert!(v.column.is_none());
    assert_eq!(v.reason, Some(ValidationReason::CommentLine));
    let msg = v.message.as_deref().unwrap_or("");
    assert!(msg.contains("adjusted to line 7"), "message was: {msg}");
    Ok(())
}

// ---------------------------------------------------------------------------
// ValidationReason Display
// ---------------------------------------------------------------------------

#[test]
fn validation_reason_display() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        ValidationReason::BlankLine.to_string(),
        "Breakpoint set on blank line"
    );
    assert_eq!(
        ValidationReason::CommentLine.to_string(),
        "Breakpoint set on comment or blank line"
    );
    assert_eq!(
        ValidationReason::HeredocInterior.to_string(),
        "Breakpoint set inside heredoc content"
    );
    assert_eq!(
        ValidationReason::PodLine.to_string(),
        "Breakpoint set inside POD documentation"
    );
    assert_eq!(
        ValidationReason::LineOutOfRange.to_string(),
        "Line number exceeds file length"
    );
    assert_eq!(
        ValidationReason::ParseError.to_string(),
        "Unable to parse source file"
    );
    assert_eq!(
        ValidationReason::InvalidCondition.to_string(),
        "Conditional breakpoint expression is invalid"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// BreakpointError Display
// ---------------------------------------------------------------------------

#[test]
fn breakpoint_error_parse_display() -> Result<(), Box<dyn std::error::Error>> {
    let err = BreakpointError::ParseError("bad syntax".to_string());
    let msg = err.to_string();
    assert!(msg.contains("bad syntax"), "message was: {msg}");
    Ok(())
}

#[test]
fn breakpoint_error_line_out_of_range_display() -> Result<(), Box<dyn std::error::Error>> {
    let err = BreakpointError::LineOutOfRange(99, 10);
    let msg = err.to_string();
    assert!(msg.contains("99"), "message was: {msg}");
    assert!(msg.contains("10"), "message was: {msg}");
    Ok(())
}

// ---------------------------------------------------------------------------
// AstBreakpointValidator construction
// ---------------------------------------------------------------------------

#[test]
fn validator_new_with_empty_source() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new(""));
    // Line 1 should be out of range for empty source
    let result = v.validate(1);
    assert!(!result.verified);
    Ok(())
}

#[test]
fn validator_new_with_valid_perl() -> Result<(), Box<dyn std::error::Error>> {
    let _v = must(AstBreakpointValidator::new("my $x = 1;\n"));
    Ok(())
}

// ---------------------------------------------------------------------------
// validate_with_column
// ---------------------------------------------------------------------------

#[test]
fn validate_with_column_passes_column_through() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("my $x = 1;\n"));
    let result = v.validate_with_column(1, Some(5));
    assert!(result.verified);
    assert_eq!(result.column, Some(5));
    Ok(())
}

#[test]
fn validate_with_column_none_column() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("my $x = 1;\n"));
    let result = v.validate_with_column(1, None);
    assert!(result.verified);
    assert_eq!(result.column, None);
    Ok(())
}

#[test]
fn validate_with_column_rejected_ignores_column() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("# comment\n"));
    let result = v.validate_with_column(1, Some(3));
    assert!(!result.verified);
    assert!(result.column.is_none());
    Ok(())
}

// ---------------------------------------------------------------------------
// Line classification edge cases
// ---------------------------------------------------------------------------

#[test]
fn comment_with_leading_whitespace() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new(
        "   # indented comment\nmy $x = 1;\n",
    ));
    let result = v.validate(1);
    assert!(!result.verified);
    assert_eq!(result.reason, Some(ValidationReason::CommentLine));
    Ok(())
}

#[test]
fn whitespace_only_line_is_blank() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new(
        "my $x = 1;\n   \t  \nmy $y = 2;\n",
    ));
    let result = v.validate(2);
    assert!(!result.verified);
    assert_eq!(result.reason, Some(ValidationReason::BlankLine));
    Ok(())
}

#[test]
fn multiple_statements_executable() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $a = 1;\nmy $b = 2;\nmy $c = 3;\n";
    let v = must(AstBreakpointValidator::new(source));
    assert!(v.is_executable_line(1));
    assert!(v.is_executable_line(2));
    assert!(v.is_executable_line(3));
    Ok(())
}

#[test]
fn line_zero_out_of_range() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("my $x = 1;\n"));
    // Line 0 is invalid (1-based indexing); line_byte_range clamps to 0 index
    // but the validator should still handle it
    let result = v.validate(0);
    // Line 0 maps to index -1 clamped to 0, same as line 1
    // The code does `(line - 1).max(0)` so line 0 → index 0 → line 1 content
    assert!(result.verified);
    assert_eq!(result.line, 0);
    Ok(())
}

#[test]
fn negative_line_number() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("my $x = 1;\n"));
    let result = v.validate(-5);
    // Negative lines: (-5 - 1).max(0) = 0, maps to first line
    // Should not crash
    assert!(result.verified);
    assert_eq!(result.line, -5);
    Ok(())
}

#[test]
fn last_line_without_trailing_newline() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("my $x = 1;"));
    let result = v.validate(1);
    assert!(result.verified);
    Ok(())
}

#[test]
fn subroutine_declaration_is_executable() -> Result<(), Box<dyn std::error::Error>> {
    let source = "sub foo {\n    my $x = 1;\n}\n";
    let v = must(AstBreakpointValidator::new(source));
    assert!(v.is_executable_line(1)); // sub declaration line
    assert!(v.is_executable_line(2)); // body
    Ok(())
}

#[test]
fn mixed_code_and_comments() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n# middle comment\nmy $y = 2;\n# end comment\n";
    let v = must(AstBreakpointValidator::new(source));
    assert!(v.is_executable_line(1));
    assert!(!v.is_executable_line(2));
    assert!(v.is_executable_line(3));
    assert!(!v.is_executable_line(4));
    Ok(())
}

// ---------------------------------------------------------------------------
// Heredoc interior validation
// ---------------------------------------------------------------------------

#[test]
fn heredoc_body_is_not_executable() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $text = <<END;\nline one\nline two\nEND\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));
    // Line 1: heredoc start → executable
    assert!(v.is_executable_line(1));
    // Lines 2-3: heredoc body → not executable (heredoc interior)
    let r2 = v.validate(2);
    let r3 = v.validate(3);
    assert!(!r2.verified);
    assert!(!r3.verified);
    // Line 5: executable code after heredoc
    assert!(v.is_executable_line(5));
    Ok(())
}

// ---------------------------------------------------------------------------
// Suggestion: find_nearest_valid_line
// ---------------------------------------------------------------------------

#[test]
fn suggestion_forward_from_blank() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n\nmy $y = 2;\n";
    let v = must(AstBreakpointValidator::new(source));
    let result = find_nearest_valid_line(&v, 2, SearchDirection::Forward, None);
    assert_eq!(result, Some(3));
    Ok(())
}

#[test]
fn suggestion_backward_from_blank() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n\nmy $y = 2;\n";
    let v = must(AstBreakpointValidator::new(source));
    let result = find_nearest_valid_line(&v, 2, SearchDirection::Backward, None);
    assert_eq!(result, Some(1));
    Ok(())
}

#[test]
fn suggestion_both_equidistant_prefers_forward() -> Result<(), Box<dyn std::error::Error>> {
    // Lines: 1=code, 2=blank, 3=code → equidistant, forward should win (f_dist <= b_dist)
    let source = "my $x = 1;\n\nmy $y = 2;\n";
    let v = must(AstBreakpointValidator::new(source));
    let result = find_nearest_valid_line(&v, 2, SearchDirection::Both, None);
    // Both are distance 1, forward wins (f_dist <= b_dist)
    assert_eq!(result, Some(3));
    Ok(())
}

#[test]
fn suggestion_forward_past_eof_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n# comment\n";
    let v = must(AstBreakpointValidator::new(source));
    let result = find_nearest_valid_line(&v, 2, SearchDirection::Forward, None);
    assert_eq!(result, None);
    Ok(())
}

#[test]
fn suggestion_backward_from_line1_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    let source = "# comment\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));
    let result = find_nearest_valid_line(&v, 1, SearchDirection::Backward, None);
    assert_eq!(result, None);
    Ok(())
}

#[test]
fn suggestion_max_distance_zero() -> Result<(), Box<dyn std::error::Error>> {
    let source = "# comment\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));
    // max_distance=0 means the loop range is 1..=0 which is empty
    let result = find_nearest_valid_line(&v, 1, SearchDirection::Forward, Some(0));
    assert_eq!(result, None);
    Ok(())
}

#[test]
fn suggestion_forward_skips_multiple_blanks() -> Result<(), Box<dyn std::error::Error>> {
    let source = "# c\n\n\n\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));
    let result = find_nearest_valid_line(&v, 1, SearchDirection::Forward, None);
    assert_eq!(result, Some(5));
    Ok(())
}

#[test]
fn suggestion_all_blank_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    let source = "\n\n\n";
    let v = must(AstBreakpointValidator::new(source));
    let result = find_nearest_valid_line(&v, 2, SearchDirection::Both, None);
    assert_eq!(result, None);
    Ok(())
}

// ---------------------------------------------------------------------------
// Complex Perl constructs
// ---------------------------------------------------------------------------

#[test]
fn if_else_block_lines_executable() -> Result<(), Box<dyn std::error::Error>> {
    let source = "if ($x) {\n    print 1;\n} else {\n    print 2;\n}\n";
    let v = must(AstBreakpointValidator::new(source));
    assert!(v.is_executable_line(1)); // if
    assert!(v.is_executable_line(2)); // print 1
    assert!(v.is_executable_line(4)); // print 2
    Ok(())
}

#[test]
fn for_loop_lines_executable() -> Result<(), Box<dyn std::error::Error>> {
    let source = "for my $i (1..10) {\n    print $i;\n}\n";
    let v = must(AstBreakpointValidator::new(source));
    assert!(v.is_executable_line(1));
    assert!(v.is_executable_line(2));
    Ok(())
}

#[test]
fn use_strict_is_executable() -> Result<(), Box<dyn std::error::Error>> {
    let source = "use strict;\nuse warnings;\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));
    assert!(v.is_executable_line(1));
    assert!(v.is_executable_line(2));
    assert!(v.is_executable_line(3));
    Ok(())
}

// ---------------------------------------------------------------------------
// BreakpointError is std::error::Error
// ---------------------------------------------------------------------------

#[test]
fn breakpoint_error_implements_error_trait() -> Result<(), Box<dyn std::error::Error>> {
    let err: Box<dyn std::error::Error> = Box::new(BreakpointError::ParseError("test".to_string()));
    assert!(err.to_string().contains("test"));
    Ok(())
}

// ---------------------------------------------------------------------------
// ValidationReason Clone + Copy + Eq
// ---------------------------------------------------------------------------

#[test]
fn validation_reason_clone_copy_eq() -> Result<(), Box<dyn std::error::Error>> {
    let r = ValidationReason::BlankLine;
    let r2 = r; // Copy
    let r3 = r; // Copy
    assert_eq!(r2, r3); // Eq
    assert_eq!(r, ValidationReason::BlankLine);
    assert_ne!(r, ValidationReason::CommentLine);
    assert_ne!(r, ValidationReason::PodLine);
    assert_ne!(r, ValidationReason::InvalidCondition);
    assert_eq!(ValidationReason::PodLine, ValidationReason::PodLine);
    assert_eq!(
        ValidationReason::InvalidCondition,
        ValidationReason::InvalidCondition
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// BreakpointValidation Clone
// ---------------------------------------------------------------------------

#[test]
fn breakpoint_validation_clone() -> Result<(), Box<dyn std::error::Error>> {
    let v = BreakpointValidation::verified(1, Some(2));
    let v2 = v.clone();
    assert_eq!(v2.line, 1);
    assert_eq!(v2.column, Some(2));
    assert!(v2.verified);
    Ok(())
}

// ---------------------------------------------------------------------------
// Trait object usage (BreakpointValidator is object-safe)
// ---------------------------------------------------------------------------

#[test]
fn validator_trait_dispatch() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("my $x = 1;\n"));
    let validator: &dyn BreakpointValidator = &v;
    assert!(validator.is_executable_line(1));
    assert!(!validator.is_executable_line(100));
    Ok(())
}

// ---------------------------------------------------------------------------
// validate delegates to validate_with_column(line, None)
// ---------------------------------------------------------------------------

#[test]
fn validate_delegates_to_validate_with_column() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("my $x = 1;\n"));
    let r1 = v.validate(1);
    let r2 = v.validate_with_column(1, None);
    assert_eq!(r1.verified, r2.verified);
    assert_eq!(r1.line, r2.line);
    assert_eq!(r1.column, r2.column);
    assert_eq!(r1.reason, r2.reason);
    Ok(())
}

// ---------------------------------------------------------------------------
// Multi-line source with trailing content
// ---------------------------------------------------------------------------

#[test]
fn large_source_many_lines() -> Result<(), Box<dyn std::error::Error>> {
    let mut source = String::new();
    for i in 1..=50 {
        source.push_str(&format!("my $v{i} = {i};\n"));
    }
    let v = must(AstBreakpointValidator::new(&source));
    // Spot check first, middle, last
    assert!(v.is_executable_line(1));
    assert!(v.is_executable_line(25));
    assert!(v.is_executable_line(50));
    // Out of range
    assert!(!v.is_executable_line(51));
    Ok(())
}

// ---------------------------------------------------------------------------
// BreakpointError Debug
// ---------------------------------------------------------------------------

#[test]
fn breakpoint_error_debug_format() -> Result<(), Box<dyn std::error::Error>> {
    let err = BreakpointError::ParseError("oops".to_string());
    let dbg = format!("{err:?}");
    assert!(dbg.contains("ParseError"), "debug was: {dbg}");
    Ok(())
}

// ---------------------------------------------------------------------------
// must_err helper for invalid source (if parser rejects it)
// Note: The Perl parser is very permissive, so we test the error path
// by checking the BreakpointError type can be constructed and used.
// ---------------------------------------------------------------------------

#[test]
fn breakpoint_error_variants_are_distinct() -> Result<(), Box<dyn std::error::Error>> {
    let e1 = BreakpointError::ParseError("a".to_string());
    let e2 = BreakpointError::LineOutOfRange(1, 10);
    // Different Display messages
    assert_ne!(e1.to_string(), e2.to_string());
    Ok(())
}
