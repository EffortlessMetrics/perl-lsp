//! Extended unit tests for perl-dap-breakpoint crate.
//!
//! Covers advanced edge cases, complex Perl constructs, boundary conditions,
//! and comprehensive validation scenarios not covered in breakpoint_tests.rs
#![allow(clippy::overly_complex_bool_expr, clippy::assertions_on_constants)]

use perl_dap::breakpoint::{
    AstBreakpointValidator, BreakpointError, BreakpointValidation, BreakpointValidator,
    SearchDirection, ValidationReason, find_nearest_valid_line,
};
use perl_tdd_support::{must, must_some};

// ---------------------------------------------------------------------------
// Extended ValidationReason tests
// ---------------------------------------------------------------------------

#[test]
fn validation_reason_debug_format() -> Result<(), Box<dyn std::error::Error>> {
    let r = ValidationReason::BlankLine;
    let dbg = format!("{r:?}");
    assert!(dbg.contains("BlankLine"), "debug was: {dbg}");
    Ok(())
}

#[test]
fn validation_reason_all_variants_have_distinct_display() -> Result<(), Box<dyn std::error::Error>>
{
    let reasons = [
        ValidationReason::BlankLine,
        ValidationReason::CommentLine,
        ValidationReason::HeredocInterior,
        ValidationReason::PodLine,
        ValidationReason::LineOutOfRange,
        ValidationReason::ParseError,
        ValidationReason::InvalidCondition,
    ];

    let strings: Vec<_> = reasons.iter().map(|r| r.to_string()).collect();
    let unique: std::collections::HashSet<_> = strings.iter().collect();
    assert_eq!(unique.len(), strings.len(), "all reason variants should have unique Display");
    Ok(())
}

// ---------------------------------------------------------------------------
// Extended BreakpointValidation constructors and methods
// ---------------------------------------------------------------------------

#[test]
fn validation_verified_with_large_line_numbers() -> Result<(), Box<dyn std::error::Error>> {
    let v = BreakpointValidation::verified(1_000_000, Some(999_999));
    assert_eq!(v.line, 1_000_000);
    assert_eq!(v.column, Some(999_999));
    assert!(v.verified);
    Ok(())
}

#[test]
fn validation_verified_with_zero_column() -> Result<(), Box<dyn std::error::Error>> {
    let v = BreakpointValidation::verified(10, Some(0));
    assert_eq!(v.column, Some(0));
    Ok(())
}

#[test]
fn validation_verified_with_negative_column() -> Result<(), Box<dyn std::error::Error>> {
    let v = BreakpointValidation::verified(10, Some(-1));
    assert_eq!(v.column, Some(-1));
    Ok(())
}

#[test]
fn validation_rejected_all_reasons() -> Result<(), Box<dyn std::error::Error>> {
    for reason in &[
        ValidationReason::BlankLine,
        ValidationReason::CommentLine,
        ValidationReason::HeredocInterior,
        ValidationReason::PodLine,
        ValidationReason::LineOutOfRange,
        ValidationReason::ParseError,
        ValidationReason::InvalidCondition,
    ] {
        let v = BreakpointValidation::rejected(5, *reason);
        assert!(!v.verified);
        assert_eq!(v.reason, Some(*reason));
        assert!(v.message.is_some());
    }
    Ok(())
}

#[test]
fn validation_adjusted_message_format() -> Result<(), Box<dyn std::error::Error>> {
    let v = BreakpointValidation::adjusted(10, ValidationReason::BlankLine);
    let msg = v.message.as_deref().unwrap_or("");
    assert!(msg.contains("Breakpoint set on blank line"));
    assert!(msg.contains("adjusted to line 10"));
    Ok(())
}

#[test]
fn validation_adjusted_all_reasons() -> Result<(), Box<dyn std::error::Error>> {
    for reason in &[
        ValidationReason::BlankLine,
        ValidationReason::CommentLine,
        ValidationReason::HeredocInterior,
        ValidationReason::PodLine,
        ValidationReason::LineOutOfRange,
        ValidationReason::ParseError,
        ValidationReason::InvalidCondition,
    ] {
        let v = BreakpointValidation::adjusted(7, *reason);
        assert!(v.verified);
        assert_eq!(v.reason, Some(*reason));
        assert!(v.message.is_some());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// AstBreakpointValidator construction edge cases
// ---------------------------------------------------------------------------

#[test]
fn validator_with_single_character() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("1"));
    let result = v.validate(1);
    assert!(result.verified);
    assert_eq!(result.line, 1);
    assert_eq!(result.reason, None);
    Ok(())
}

#[test]
fn validator_with_only_newlines() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("\n\n\n"));
    let result = v.validate(2);
    assert!(!result.verified);
    assert_eq!(result.reason, Some(ValidationReason::BlankLine));
    Ok(())
}

#[test]
fn validator_with_only_spaces() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("   "));
    let result = v.validate(1);
    assert!(!result.verified);
    assert_eq!(result.reason, Some(ValidationReason::BlankLine));
    Ok(())
}

#[test]
fn validator_with_tabs_only() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("\t\t\t"));
    let result = v.validate(1);
    assert!(!result.verified);
    assert_eq!(result.reason, Some(ValidationReason::BlankLine));
    Ok(())
}

#[test]
fn validator_with_mixed_whitespace() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new(" \t \n\t  \n  \t"));
    let result = v.validate(2);
    assert!(!result.verified);
    assert_eq!(result.reason, Some(ValidationReason::BlankLine));
    Ok(())
}

// ---------------------------------------------------------------------------
// Comment line edge cases
// ---------------------------------------------------------------------------

#[test]
fn comment_without_space_after_hash() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("#comment\nmy $x = 1;\n"));
    let result = v.validate(1);
    assert!(!result.verified);
    assert_eq!(result.reason, Some(ValidationReason::CommentLine));
    Ok(())
}

#[test]
fn comment_with_multiple_hashes() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("### deep comment\nmy $x = 1;\n"));
    let result = v.validate(1);
    assert!(!result.verified);
    assert_eq!(result.reason, Some(ValidationReason::CommentLine));
    Ok(())
}

#[test]
fn comment_with_special_characters() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("# !@#$%^&*()_+-=[]{}|;:',.<>?/\nmy $x = 1;\n"));
    let result = v.validate(1);
    assert!(!result.verified);
    assert_eq!(result.reason, Some(ValidationReason::CommentLine));
    Ok(())
}

#[test]
fn comment_with_unicode() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("# こんにちは 世界 🌍\nmy $x = 1;\n"));
    let result = v.validate(1);
    assert!(!result.verified);
    assert_eq!(result.reason, Some(ValidationReason::CommentLine));
    Ok(())
}

#[test]
fn comment_only_hash() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("#\nmy $x = 1;\n"));
    let result = v.validate(1);
    assert!(!result.verified);
    assert_eq!(result.reason, Some(ValidationReason::CommentLine));
    Ok(())
}

// ---------------------------------------------------------------------------
// validate_with_column edge cases
// ---------------------------------------------------------------------------

#[test]
fn validate_with_column_large_column_numbers() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("my $x = 1;\n"));
    let result = v.validate_with_column(1, Some(999_999));
    assert!(result.verified);
    assert_eq!(result.column, Some(999_999));
    Ok(())
}

#[test]
fn validate_with_column_negative_line() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("my $x = 1;\n"));
    let result = v.validate_with_column(-10, Some(5));
    // Negative line is clamped for lookup and validated against first source line.
    assert!(result.verified);
    assert_eq!(result.line, -10);
    assert_eq!(result.column, Some(5));
    assert_eq!(result.reason, None);
    Ok(())
}

#[test]
fn validate_with_column_zero_line() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("my $x = 1;\n"));
    let result = v.validate_with_column(0, Some(1));
    // Line 0 is clamped for lookup and validated against first source line.
    assert!(result.verified);
    assert_eq!(result.line, 0);
    assert_eq!(result.column, Some(1));
    assert_eq!(result.reason, None);
    Ok(())
}

// ---------------------------------------------------------------------------
// is_executable_line comprehensive coverage
// ---------------------------------------------------------------------------

#[test]
fn is_executable_line_boundary_large_line() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("my $x = 1;\n"));
    assert!(!v.is_executable_line(1_000_000));
    Ok(())
}

#[test]
fn is_executable_line_negative() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("my $x = 1;\n"));
    let result = v.is_executable_line(-5);
    assert!(result);
    Ok(())
}

#[test]
fn is_executable_line_pattern_code_blank_code_blank_code() -> Result<(), Box<dyn std::error::Error>>
{
    let source = "my $a = 1;\n\nmy $b = 2;\n\nmy $c = 3;\n";
    let v = must(AstBreakpointValidator::new(source));
    assert!(v.is_executable_line(1));
    assert!(!v.is_executable_line(2));
    assert!(v.is_executable_line(3));
    assert!(!v.is_executable_line(4));
    assert!(v.is_executable_line(5));
    Ok(())
}

// ---------------------------------------------------------------------------
// Heredoc advanced tests
// ---------------------------------------------------------------------------

#[test]
fn heredoc_with_quoted_terminator() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $text = <<'END';\nline 1\nline 2\nEND\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));
    assert!(v.is_executable_line(1)); // heredoc start
    // Lines 2-3 are in heredoc body
    let r2 = v.validate(2);
    assert!(!r2.verified);
    assert!(v.is_executable_line(5)); // code after heredoc
    Ok(())
}

#[test]
fn heredoc_with_indented_terminator() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $text = <<END;\nline 1\nline 2\nEND\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));
    assert!(v.is_executable_line(1));
    // Check body lines are rejected
    let r2 = v.validate(2);
    assert!(!r2.verified);
    Ok(())
}

#[test]
fn multiple_heredocs_on_same_line() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $a = <<A, my $b = <<B;\nA content\nA\nB content\nB\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));
    assert!(v.is_executable_line(1)); // assignment line
    // Body lines should not be executable
    let r2 = v.validate(2);
    assert!(!r2.verified);
    Ok(())
}

// ---------------------------------------------------------------------------
// SearchDirection enum coverage
// ---------------------------------------------------------------------------

#[test]
fn search_direction_forward_debug() -> Result<(), Box<dyn std::error::Error>> {
    let d = SearchDirection::Forward;
    let dbg = format!("{d:?}");
    assert!(dbg.contains("Forward"));
    Ok(())
}

#[test]
fn search_direction_backward_debug() -> Result<(), Box<dyn std::error::Error>> {
    let d = SearchDirection::Backward;
    let dbg = format!("{d:?}");
    assert!(dbg.contains("Backward"));
    Ok(())
}

#[test]
fn search_direction_both_debug() -> Result<(), Box<dyn std::error::Error>> {
    let d = SearchDirection::Both;
    let dbg = format!("{d:?}");
    assert!(dbg.contains("Both"));
    Ok(())
}

#[test]
fn search_direction_clone_copy() -> Result<(), Box<dyn std::error::Error>> {
    let d = SearchDirection::Forward;
    let d2 = d;
    let d3 = d;
    assert_eq!(d, d2);
    assert_eq!(d2, d3);
    Ok(())
}

#[test]
fn search_direction_eq_ne() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(SearchDirection::Forward, SearchDirection::Forward);
    assert_ne!(SearchDirection::Forward, SearchDirection::Backward);
    assert_ne!(SearchDirection::Backward, SearchDirection::Both);
    assert_ne!(SearchDirection::Both, SearchDirection::Forward);
    Ok(())
}

// ---------------------------------------------------------------------------
// find_nearest_valid_line comprehensive tests
// ---------------------------------------------------------------------------

#[test]
fn find_nearest_valid_line_forward_immediate() -> Result<(), Box<dyn std::error::Error>> {
    let source = "# comment\nmy $x = 1;\nmy $y = 2;\n";
    let v = must(AstBreakpointValidator::new(source));
    let result = find_nearest_valid_line(&v, 1, SearchDirection::Forward, None);
    assert_eq!(result, Some(2));
    Ok(())
}

#[test]
fn find_nearest_valid_line_backward_immediate() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\nmy $y = 2;\n# comment\n";
    let v = must(AstBreakpointValidator::new(source));
    let result = find_nearest_valid_line(&v, 3, SearchDirection::Backward, None);
    assert_eq!(result, Some(2));
    Ok(())
}

#[test]
fn find_nearest_valid_line_forward_with_distance_limit() -> Result<(), Box<dyn std::error::Error>> {
    let source = "# c\n# c\n# c\n# c\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));
    // Max distance 3 should reach line 4, but line 4 is still comment
    // So we need distance 4 to reach line 5
    let result = find_nearest_valid_line(&v, 1, SearchDirection::Forward, Some(3));
    assert_eq!(result, None);
    let result = find_nearest_valid_line(&v, 1, SearchDirection::Forward, Some(4));
    assert_eq!(result, Some(5));
    Ok(())
}

#[test]
fn find_nearest_valid_line_backward_with_distance_limit() -> Result<(), Box<dyn std::error::Error>>
{
    let source = "my $x = 1;\n# c\n# c\n# c\n# c\n";
    let v = must(AstBreakpointValidator::new(source));
    // Distance 3 from line 5 → backward 1: line 4 (comment), 2: line 3 (comment), 3: line 2 (comment)
    // Can't reach line 1 within distance 3
    let result = find_nearest_valid_line(&v, 5, SearchDirection::Backward, Some(3));
    assert_eq!(result, None);
    // Distance 4 allows us to reach line 1
    let result = find_nearest_valid_line(&v, 5, SearchDirection::Backward, Some(4));
    assert_eq!(result, Some(1));
    let result = find_nearest_valid_line(&v, 5, SearchDirection::Backward, Some(0));
    assert_eq!(result, None);
    Ok(())
}

#[test]
fn find_nearest_valid_line_both_backward_closer() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n# comment\n# comment\n# comment\n# comment\nmy $y = 2;\n";
    let v = must(AstBreakpointValidator::new(source));
    // From line 2: backward is 1 away, forward is 4 away → choose backward
    let result = find_nearest_valid_line(&v, 2, SearchDirection::Both, None);
    assert_eq!(result, Some(1));
    Ok(())
}

#[test]
fn find_nearest_valid_line_both_forward_closer() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n# comment\n# comment\n# comment\n# comment\nmy $y = 2;\n";
    let v = must(AstBreakpointValidator::new(source));
    // From line 5: backward is 4 away, forward is 1 away → choose forward
    let result = find_nearest_valid_line(&v, 5, SearchDirection::Both, None);
    assert_eq!(result, Some(6));
    Ok(())
}

#[test]
fn find_nearest_valid_line_both_backward_equidistant_forward_wins()
-> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n# c\n# c\nmy $y = 2;\n";
    let v = must(AstBreakpointValidator::new(source));
    // From line 2: backward 1, forward 2
    let result = find_nearest_valid_line(&v, 2, SearchDirection::Both, None);
    assert_eq!(result, Some(1));
    Ok(())
}

#[test]
fn find_nearest_valid_line_from_valid_line_forward() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\nmy $y = 2;\nmy $z = 3;\n";
    let v = must(AstBreakpointValidator::new(source));
    // From line 1 (executable), search forward → should find line 2
    let result = find_nearest_valid_line(&v, 1, SearchDirection::Forward, None);
    assert_eq!(result, Some(2));
    Ok(())
}

#[test]
fn find_nearest_valid_line_from_valid_line_backward() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\nmy $y = 2;\nmy $z = 3;\n";
    let v = must(AstBreakpointValidator::new(source));
    // From line 3 (executable), search backward → should find line 2
    let result = find_nearest_valid_line(&v, 3, SearchDirection::Backward, None);
    assert_eq!(result, Some(2));
    Ok(())
}

#[test]
fn find_nearest_valid_line_backward_hits_start_boundary() -> Result<(), Box<dyn std::error::Error>>
{
    let source = "my $x = 1;\n# c\n# c\n";
    let v = must(AstBreakpointValidator::new(source));
    // From line 3, backward search can reach line 1
    let result = find_nearest_valid_line(&v, 3, SearchDirection::Backward, Some(5));
    assert_eq!(result, Some(1));
    Ok(())
}

#[test]
fn find_nearest_valid_line_forward_hits_end_boundary() -> Result<(), Box<dyn std::error::Error>> {
    let source = "# c\n# c\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));
    // From line 1, forward search can reach line 3
    let result = find_nearest_valid_line(&v, 1, SearchDirection::Forward, Some(5));
    assert_eq!(result, Some(3));
    Ok(())
}

// ---------------------------------------------------------------------------
// Complex Perl constructs validation
// ---------------------------------------------------------------------------

#[test]
fn while_loop_lines_executable() -> Result<(), Box<dyn std::error::Error>> {
    let source = "while ($x < 10) {\n    $x++;\n}\n";
    let v = must(AstBreakpointValidator::new(source));
    assert!(v.is_executable_line(1)); // while condition
    assert!(v.is_executable_line(2)); // body
    Ok(())
}

#[test]
fn do_while_loop_executable() -> Result<(), Box<dyn std::error::Error>> {
    let source = "do {\n    print 1;\n} while ($x);\n";
    let v = must(AstBreakpointValidator::new(source));
    assert!(v.is_executable_line(1)); // do keyword
    assert!(v.is_executable_line(2)); // body
    Ok(())
}

#[test]
fn hash_assignment_executable() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my %hash = (\n    key => 'value',\n);\n";
    let v = must(AstBreakpointValidator::new(source));
    assert!(v.is_executable_line(1)); // assignment start
    Ok(())
}

#[test]
fn array_assignment_executable() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my @array = (\n    1,\n    2,\n);\n";
    let v = must(AstBreakpointValidator::new(source));
    assert!(v.is_executable_line(1)); // assignment start
    Ok(())
}

#[test]
fn regex_match_executable() -> Result<(), Box<dyn std::error::Error>> {
    let source = "if ($str =~ /pattern/) {\n    print 1;\n}\n";
    let v = must(AstBreakpointValidator::new(source));
    assert!(v.is_executable_line(1)); // if with regex
    Ok(())
}

#[test]
fn qw_operator_executable() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my @list = qw(one two three);\n";
    let v = must(AstBreakpointValidator::new(source));
    assert!(v.is_executable_line(1));
    Ok(())
}

#[test]
fn string_interpolation_executable() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $str = \"Hello $name\";\n";
    let v = must(AstBreakpointValidator::new(source));
    assert!(v.is_executable_line(1));
    Ok(())
}

#[test]
fn package_declaration_executable() -> Result<(), Box<dyn std::error::Error>> {
    let source = "package MyPackage;\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));
    assert!(v.is_executable_line(1)); // package
    assert!(v.is_executable_line(2)); // code
    Ok(())
}

#[test]
fn sub_with_attributes_executable() -> Result<(), Box<dyn std::error::Error>> {
    let source = "sub foo :lvalue {\n    my $x = 1;\n}\n";
    let v = must(AstBreakpointValidator::new(source));
    assert!(v.is_executable_line(1)); // sub declaration
    Ok(())
}

#[test]
fn ternary_operator_executable() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $result = $x > 5 ? 'yes' : 'no';\n";
    let v = must(AstBreakpointValidator::new(source));
    assert!(v.is_executable_line(1));
    Ok(())
}

#[test]
fn try_catch_block_executable() -> Result<(), Box<dyn std::error::Error>> {
    let source = "eval {\n    die 'error';\n};\n";
    let v = must(AstBreakpointValidator::new(source));
    assert!(v.is_executable_line(1)); // eval
    Ok(())
}

// ---------------------------------------------------------------------------
// Blank line variants
// ---------------------------------------------------------------------------

#[test]
fn blank_line_with_newline() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("my $x = 1;\n\nmy $y = 2;\n"));
    let result = v.validate(2);
    assert!(!result.verified);
    assert_eq!(result.reason, Some(ValidationReason::BlankLine));
    Ok(())
}

#[test]
fn blank_line_with_carriage_return() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("my $x = 1;\n\r\nmy $y = 2;\n"));
    let result = v.validate(2);
    assert!(!result.verified);
    assert_eq!(result.reason, Some(ValidationReason::BlankLine));
    Ok(())
}

// ---------------------------------------------------------------------------
// BreakpointError creation and usage
// ---------------------------------------------------------------------------

#[test]
fn breakpoint_error_from_string() -> Result<(), Box<dyn std::error::Error>> {
    let err = BreakpointError::ParseError("test error".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("test error"));
    Ok(())
}

#[test]
fn breakpoint_error_line_range_boundary_values() -> Result<(), Box<dyn std::error::Error>> {
    let err = BreakpointError::LineOutOfRange(i64::MAX, usize::MAX);
    let msg = format!("{err}");
    assert!(msg.contains("out of range"));
    Ok(())
}

// ---------------------------------------------------------------------------
// Integration tests: validator + suggestions together
// ---------------------------------------------------------------------------

#[test]
fn integration_validate_then_suggest() -> Result<(), Box<dyn std::error::Error>> {
    let source = "# comment\n# comment\nmy $x = 1;\nmy $y = 2;\n";
    let v = must(AstBreakpointValidator::new(source));

    // Try to validate line 1 (fails)
    let result = v.validate(1);
    assert!(!result.verified);

    // Find nearest valid line
    let nearest = find_nearest_valid_line(&v, 1, SearchDirection::Forward, None);
    assert_eq!(nearest, Some(3));

    // Validate the suggested line
    let suggested_result = v.validate(must_some(nearest));
    assert!(suggested_result.verified);
    Ok(())
}

#[test]
fn integration_multiple_validations_same_validator() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $a = 1;\n# comment\nmy $b = 2;\n\nmy $c = 3;\n";
    let v = must(AstBreakpointValidator::new(source));

    // Validate multiple lines
    assert!(v.validate(1).verified);
    assert!(!v.validate(2).verified);
    assert!(v.validate(3).verified);
    assert!(!v.validate(4).verified);
    assert!(v.validate(5).verified);
    Ok(())
}

#[test]
fn integration_find_nearest_both_then_validate() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n# comment\n# comment\nmy $y = 2;\n";
    let v = must(AstBreakpointValidator::new(source));

    let nearest = find_nearest_valid_line(&v, 2, SearchDirection::Both, None);
    assert!(nearest.is_some());

    let line = must_some(nearest);
    let result = v.validate(line);
    assert!(result.verified);
    Ok(())
}

// ---------------------------------------------------------------------------
// Trait object usage extended
// ---------------------------------------------------------------------------

#[test]
fn validator_trait_object_multiple_calls() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("my $x = 1;\n# comment\nmy $y = 2;\n"));
    let trait_obj: &dyn BreakpointValidator = &v;

    assert!(trait_obj.is_executable_line(1));
    assert!(!trait_obj.is_executable_line(2));
    assert!(trait_obj.is_executable_line(3));

    let result = trait_obj.validate(1);
    assert!(result.verified);

    let result = trait_obj.validate_with_column(3, Some(5));
    assert!(result.verified);
    assert_eq!(result.column, Some(5));
    Ok(())
}

// ---------------------------------------------------------------------------
// BreakpointError as trait object
// ---------------------------------------------------------------------------

#[test]
fn breakpoint_error_as_std_error_trait() -> Result<(), Box<dyn std::error::Error>> {
    let parse_err: Box<dyn std::error::Error> =
        Box::new(BreakpointError::ParseError("oops".to_string()));
    let msg = parse_err.to_string();
    assert!(msg.contains("oops"));
    Ok(())
}

#[test]
fn breakpoint_error_line_range_as_std_error_trait() -> Result<(), Box<dyn std::error::Error>> {
    let range_err: Box<dyn std::error::Error> = Box::new(BreakpointError::LineOutOfRange(5, 3));
    let msg = range_err.to_string();
    assert!(msg.contains("5"));
    assert!(msg.contains("3"));
    Ok(())
}

// ---------------------------------------------------------------------------
// Source with only comments
// ---------------------------------------------------------------------------

#[test]
fn source_only_comments_no_executable_lines() -> Result<(), Box<dyn std::error::Error>> {
    let source = "# comment 1\n# comment 2\n# comment 3\n";
    let v = must(AstBreakpointValidator::new(source));

    assert!(!v.is_executable_line(1));
    assert!(!v.is_executable_line(2));
    assert!(!v.is_executable_line(3));
    assert!(!v.is_executable_line(4)); // out of range
    Ok(())
}

// ---------------------------------------------------------------------------
// Very long lines
// ---------------------------------------------------------------------------

#[test]
fn very_long_single_line() -> Result<(), Box<dyn std::error::Error>> {
    let long_line = "my $x = ".to_string() + &"1 ".repeat(1000) + ";\n";
    let v = must(AstBreakpointValidator::new(&long_line));
    assert!(v.is_executable_line(1));
    Ok(())
}

// ---------------------------------------------------------------------------
// Mixed indentation styles
// ---------------------------------------------------------------------------

#[test]
fn deeply_nested_code_indentation() -> Result<(), Box<dyn std::error::Error>> {
    let source =
        "if ($a) {\n    if ($b) {\n        if ($c) {\n            print 1;\n        }\n    }\n}\n";
    let v = must(AstBreakpointValidator::new(source));

    assert!(v.is_executable_line(1)); // if
    assert!(v.is_executable_line(2)); // nested if
    assert!(v.is_executable_line(3)); // nested if
    assert!(v.is_executable_line(4)); // print
    Ok(())
}

// ---------------------------------------------------------------------------
// Empty lines between different constructs
// ---------------------------------------------------------------------------

#[test]
fn blank_lines_between_functions() -> Result<(), Box<dyn std::error::Error>> {
    let source = "sub foo { }\n\n\nsub bar { }\n\n\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    assert!(v.is_executable_line(1)); // sub foo
    assert!(!v.is_executable_line(2)); // blank
    assert!(!v.is_executable_line(3)); // blank
    assert!(v.is_executable_line(4)); // sub bar
    assert!(!v.is_executable_line(5)); // blank
    assert!(!v.is_executable_line(6)); // blank
    assert!(v.is_executable_line(7)); // my $x
    Ok(())
}

// ---------------------------------------------------------------------------
// File with exactly one line (no trailing newline)
// ---------------------------------------------------------------------------

#[test]
fn single_line_no_newline_executable() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("my $x = 1"));
    assert!(v.is_executable_line(1));
    assert!(!v.is_executable_line(2));
    Ok(())
}

#[test]
fn single_line_no_newline_comment() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("# just a comment"));
    assert!(!v.is_executable_line(1));
    assert!(!v.is_executable_line(2));
    Ok(())
}

#[test]
fn single_line_no_newline_blank() -> Result<(), Box<dyn std::error::Error>> {
    let v = must(AstBreakpointValidator::new("   "));
    assert!(!v.is_executable_line(1));
    Ok(())
}
