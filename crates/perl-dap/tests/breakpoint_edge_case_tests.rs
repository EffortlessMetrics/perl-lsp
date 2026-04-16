//! Edge case tests for perl-dap-breakpoint crate.
//!
//! Covers:
//! - Multi-line statement validation (heredocs, qw() lists spanning lines)
//! - Breakpoint on package declaration lines
//! - Hit condition operators with edge values (0, max u64)
//! - Breakpoint on empty lines
//! - Breakpoint on comment-only lines
//! - Duplicate breakpoint handling (same line, different conditions)

use perl_dap::breakpoint::{
    AstBreakpointValidator, BreakpointValidator, SearchDirection, ValidationReason,
    find_nearest_valid_line,
};
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Multi-line statement validation: heredocs
// ---------------------------------------------------------------------------

#[test]
fn heredoc_body_lines_rejected_as_interior() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $text = <<EOF;\nHello world\nSecond line\nThird line\nEOF\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    // Line 1: heredoc start statement -- executable
    assert!(v.is_executable_line(1));

    // Lines 2-4: heredoc body -- rejected as heredoc interior
    for line in 2..=4 {
        let result = v.validate(line);
        assert!(!result.verified, "line {line} should not be verified");
        assert_eq!(
            result.reason,
            Some(ValidationReason::HeredocInterior),
            "line {line} should be HeredocInterior"
        );
    }

    // Line 6: code after heredoc -- executable
    assert!(v.is_executable_line(6));
    Ok(())
}

#[test]
fn heredoc_single_quote_body_lines_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $text = <<'MARKER';\nNo $interpolation here\nAnother line\nMARKER\nprint 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    assert!(v.is_executable_line(1));
    let r2 = v.validate(2);
    assert!(!r2.verified);
    let r3 = v.validate(3);
    assert!(!r3.verified);
    assert!(v.is_executable_line(5));
    Ok(())
}

#[test]
fn heredoc_double_quote_body_lines_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $text = <<\"END\";\nInterpolated $var here\nLine two\nEND\nprint 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    assert!(v.is_executable_line(1));
    let r2 = v.validate(2);
    assert!(!r2.verified);
    let r3 = v.validate(3);
    assert!(!r3.verified);
    assert!(v.is_executable_line(5));
    Ok(())
}

#[test]
fn heredoc_empty_body_terminator_on_next_line() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $text = <<EOF;\nEOF\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    assert!(v.is_executable_line(1));
    // Line 3: code after empty heredoc
    assert!(v.is_executable_line(3));
    Ok(())
}

#[test]
fn heredoc_suggestion_skips_body() -> Result<(), Box<dyn std::error::Error>> {
    let source = "# comment\nmy $t = <<END;\nbody1\nbody2\nEND\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    // From line 1 (comment), searching forward should skip heredoc body and find executable
    let nearest = find_nearest_valid_line(&v, 1, SearchDirection::Forward, None);
    assert_eq!(nearest, Some(2)); // the heredoc start itself is executable
    Ok(())
}

// ---------------------------------------------------------------------------
// Multi-line statement validation: qw() lists spanning lines
// ---------------------------------------------------------------------------

#[test]
fn qw_single_line_executable() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my @list = qw(foo bar baz);\nprint 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    assert!(v.is_executable_line(1));
    assert!(v.is_executable_line(2));
    Ok(())
}

#[test]
fn qw_multiline_start_is_executable() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my @list = qw(\n    foo\n    bar\n    baz\n);\nprint 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    // Line 1: assignment with qw( start -- executable
    assert!(v.is_executable_line(1));
    // Line 6: code after qw -- executable
    assert!(v.is_executable_line(6));
    Ok(())
}

#[test]
fn qw_multiline_with_braces() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my @list = qw{\n    alpha\n    beta\n    gamma\n};\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    assert!(v.is_executable_line(1));
    assert!(v.is_executable_line(6));
    Ok(())
}

// ---------------------------------------------------------------------------
// Breakpoint on package declaration lines
// ---------------------------------------------------------------------------

#[test]
fn package_declaration_simple() -> Result<(), Box<dyn std::error::Error>> {
    let source = "package MyApp;\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    let result = v.validate(1);
    assert!(result.verified, "package declaration should be executable");
    assert_eq!(result.line, 1);
    assert!(result.reason.is_none());
    Ok(())
}

#[test]
fn package_declaration_with_version() -> Result<(), Box<dyn std::error::Error>> {
    let source = "package MyApp 1.00;\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    let result = v.validate(1);
    assert!(result.verified, "package with version should be executable");
    Ok(())
}

#[test]
fn package_nested_namespace() -> Result<(), Box<dyn std::error::Error>> {
    let source = "package My::App::Deeply::Nested;\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    assert!(v.is_executable_line(1));
    assert!(v.is_executable_line(2));
    Ok(())
}

#[test]
fn package_block_form() -> Result<(), Box<dyn std::error::Error>> {
    let source = "package MyApp {\n    my $x = 1;\n}\n";
    let v = must(AstBreakpointValidator::new(source));

    assert!(v.is_executable_line(1));
    assert!(v.is_executable_line(2));
    Ok(())
}

#[test]
fn package_with_comment_before() -> Result<(), Box<dyn std::error::Error>> {
    let source = "# This is the main package\npackage MyApp;\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    let r1 = v.validate(1);
    assert!(!r1.verified);
    assert_eq!(r1.reason, Some(ValidationReason::CommentLine));

    let r2 = v.validate(2);
    assert!(r2.verified);
    Ok(())
}

#[test]
fn package_followed_by_use_statements() -> Result<(), Box<dyn std::error::Error>> {
    let source = "package MyApp;\nuse strict;\nuse warnings;\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    assert!(v.is_executable_line(1)); // package
    assert!(v.is_executable_line(2)); // use strict
    assert!(v.is_executable_line(3)); // use warnings
    assert!(v.is_executable_line(4)); // my $x
    Ok(())
}

#[test]
fn multiple_package_declarations() -> Result<(), Box<dyn std::error::Error>> {
    let source = "package Foo;\nmy $a = 1;\npackage Bar;\nmy $b = 2;\n";
    let v = must(AstBreakpointValidator::new(source));

    assert!(v.is_executable_line(1)); // package Foo
    assert!(v.is_executable_line(2)); // my $a
    assert!(v.is_executable_line(3)); // package Bar
    assert!(v.is_executable_line(4)); // my $b
    Ok(())
}

// ---------------------------------------------------------------------------
// Hit condition operators with edge values
// ---------------------------------------------------------------------------

#[test]
fn hit_condition_modulo_operator() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    let result = v.validate_condition(1, "$x % 3 == 0");
    assert!(result.verified, "modulo condition should parse");
    Ok(())
}

#[test]
fn hit_condition_greater_equal() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    let result = v.validate_condition(1, "$x >= 10");
    assert!(result.verified, ">= condition should parse");
    Ok(())
}

#[test]
fn hit_condition_less_equal() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    let result = v.validate_condition(1, "$x <= 100");
    assert!(result.verified, "<= condition should parse");
    Ok(())
}

#[test]
fn hit_condition_equality() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    let result = v.validate_condition(1, "$x == 42");
    assert!(result.verified, "== condition should parse");
    Ok(())
}

#[test]
fn hit_condition_greater_than() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    let result = v.validate_condition(1, "$x > 5");
    assert!(result.verified, "> condition should parse");
    Ok(())
}

#[test]
fn hit_condition_less_than() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    let result = v.validate_condition(1, "$x < 999");
    assert!(result.verified, "< condition should parse");
    Ok(())
}

#[test]
fn hit_condition_not_equal() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    let result = v.validate_condition(1, "$x != 0");
    assert!(result.verified, "!= condition should parse");
    Ok(())
}

#[test]
fn hit_condition_with_zero_value() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 0;\n";
    let v = must(AstBreakpointValidator::new(source));

    let result = v.validate_condition(1, "$x == 0");
    assert!(result.verified, "comparison with 0 should parse");

    let result = v.validate_condition(1, "$x > 0");
    assert!(result.verified, "> 0 should parse");

    let result = v.validate_condition(1, "$x >= 0");
    assert!(result.verified, ">= 0 should parse");

    let result = v.validate_condition(1, "$x < 0");
    assert!(result.verified, "< 0 should parse");

    let result = v.validate_condition(1, "$x <= 0");
    assert!(result.verified, "<= 0 should parse");

    let result = v.validate_condition(1, "$x % 0");
    assert!(result.verified, "modulo 0 expression should parse (runtime error, not syntax)");
    Ok(())
}

#[test]
fn hit_condition_with_large_numeric_value() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    // u64::MAX = 18446744073709551615
    let result = v.validate_condition(1, "$x == 18446744073709551615");
    assert!(result.verified, "comparison with u64::MAX should parse");

    let result = v.validate_condition(1, "$x >= 18446744073709551615");
    assert!(result.verified, ">= u64::MAX should parse");

    let result = v.validate_condition(1, "$x <= 18446744073709551615");
    assert!(result.verified, "<= u64::MAX should parse");

    let result = v.validate_condition(1, "$x > 18446744073709551615");
    assert!(result.verified, "> u64::MAX should parse");

    let result = v.validate_condition(1, "$x < 18446744073709551615");
    assert!(result.verified, "< u64::MAX should parse");

    let result = v.validate_condition(1, "$x % 18446744073709551615");
    assert!(result.verified, "modulo u64::MAX should parse");
    Ok(())
}

#[test]
fn hit_condition_negative_values() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    let result = v.validate_condition(1, "$x > -1");
    assert!(result.verified, "> negative should parse");

    let result = v.validate_condition(1, "$x == -999");
    assert!(result.verified, "== negative should parse");

    let result = v.validate_condition(1, "$x <= -100");
    assert!(result.verified, "<= negative should parse");
    Ok(())
}

#[test]
fn hit_condition_chained_operators() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    let result = v.validate_condition(1, "$x >= 0 && $x <= 100");
    assert!(result.verified, "chained >= and <= should parse");

    let result = v.validate_condition(1, "$x > 0 && $x < 100 && $x % 2 == 0");
    assert!(result.verified, "triple chain with modulo should parse");

    let result = v.validate_condition(1, "$x == 1 || $x == 2 || $x == 3");
    assert!(result.verified, "chained || with == should parse");
    Ok(())
}

#[test]
fn hit_condition_modulo_with_edge_expressions() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    let result = v.validate_condition(1, "$x % 1 == 0");
    assert!(result.verified, "modulo 1 should parse");

    let result = v.validate_condition(1, "$x % 2 == 1");
    assert!(result.verified, "modulo 2 with remainder should parse");

    let result = v.validate_condition(1, "($x % 10) >= 5");
    assert!(result.verified, "parenthesized modulo should parse");
    Ok(())
}

// ---------------------------------------------------------------------------
// Breakpoint on empty lines
// ---------------------------------------------------------------------------

#[test]
fn empty_line_between_statements() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n\nmy $y = 2;\n";
    let v = must(AstBreakpointValidator::new(source));

    let result = v.validate(2);
    assert!(!result.verified);
    assert_eq!(result.reason, Some(ValidationReason::BlankLine));
    Ok(())
}

#[test]
fn multiple_empty_lines_all_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n\n\n\nmy $y = 2;\n";
    let v = must(AstBreakpointValidator::new(source));

    for line in 2..=4 {
        let result = v.validate(line);
        assert!(!result.verified, "line {line} should not be verified");
        assert_eq!(
            result.reason,
            Some(ValidationReason::BlankLine),
            "line {line} should be BlankLine"
        );
    }
    Ok(())
}

#[test]
fn empty_line_at_start_of_file() -> Result<(), Box<dyn std::error::Error>> {
    let source = "\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    let r1 = v.validate(1);
    assert!(!r1.verified);
    assert_eq!(r1.reason, Some(ValidationReason::BlankLine));

    assert!(v.is_executable_line(2));
    Ok(())
}

#[test]
fn empty_line_at_end_of_file() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n\n";
    let v = must(AstBreakpointValidator::new(source));

    assert!(v.is_executable_line(1));
    let r2 = v.validate(2);
    assert!(!r2.verified);
    assert_eq!(r2.reason, Some(ValidationReason::BlankLine));
    Ok(())
}

#[test]
fn empty_line_with_only_whitespace_characters() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n  \t  \nmy $y = 2;\n";
    let v = must(AstBreakpointValidator::new(source));

    let result = v.validate(2);
    assert!(!result.verified);
    assert_eq!(result.reason, Some(ValidationReason::BlankLine));
    Ok(())
}

#[test]
fn empty_line_suggestion_forward() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n\n\nmy $y = 2;\n";
    let v = must(AstBreakpointValidator::new(source));

    let nearest = find_nearest_valid_line(&v, 2, SearchDirection::Forward, None);
    assert_eq!(nearest, Some(4));
    Ok(())
}

#[test]
fn empty_line_suggestion_backward() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n\n\nmy $y = 2;\n";
    let v = must(AstBreakpointValidator::new(source));

    let nearest = find_nearest_valid_line(&v, 3, SearchDirection::Backward, None);
    assert_eq!(nearest, Some(1));
    Ok(())
}

#[test]
fn empty_line_suggestion_both_finds_closer() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n\n\n\n\nmy $y = 2;\n";
    let v = must(AstBreakpointValidator::new(source));

    // Line 2: 1 away from line 1, 4 away from line 6 -- backward wins
    let nearest = find_nearest_valid_line(&v, 2, SearchDirection::Both, None);
    assert_eq!(nearest, Some(1));

    // Line 5: 4 away from line 1, 1 away from line 6 -- forward wins
    let nearest = find_nearest_valid_line(&v, 5, SearchDirection::Both, None);
    assert_eq!(nearest, Some(6));
    Ok(())
}

// ---------------------------------------------------------------------------
// Breakpoint on comment-only lines
// ---------------------------------------------------------------------------

#[test]
fn comment_only_line_simple() -> Result<(), Box<dyn std::error::Error>> {
    let source = "# this is a comment\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    let result = v.validate(1);
    assert!(!result.verified);
    assert_eq!(result.reason, Some(ValidationReason::CommentLine));
    Ok(())
}

#[test]
fn comment_only_line_with_leading_spaces() -> Result<(), Box<dyn std::error::Error>> {
    let source = "    # indented comment\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    let result = v.validate(1);
    assert!(!result.verified);
    assert_eq!(result.reason, Some(ValidationReason::CommentLine));
    Ok(())
}

#[test]
fn comment_only_line_with_leading_tabs() -> Result<(), Box<dyn std::error::Error>> {
    let source = "\t\t# tabbed comment\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    let result = v.validate(1);
    assert!(!result.verified);
    assert_eq!(result.reason, Some(ValidationReason::CommentLine));
    Ok(())
}

#[test]
fn multiple_comment_lines_all_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let source = "# comment 1\n# comment 2\n# comment 3\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    for line in 1..=3 {
        let result = v.validate(line);
        assert!(!result.verified, "line {line} should not be verified");
        assert_eq!(
            result.reason,
            Some(ValidationReason::CommentLine),
            "line {line} should be CommentLine"
        );
    }
    assert!(v.is_executable_line(4));
    Ok(())
}

#[test]
fn comment_block_with_blank_line_mixed() -> Result<(), Box<dyn std::error::Error>> {
    let source = "# comment\n\n# another comment\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    let r1 = v.validate(1);
    assert!(!r1.verified);
    assert_eq!(r1.reason, Some(ValidationReason::CommentLine));

    let r2 = v.validate(2);
    assert!(!r2.verified);
    assert_eq!(r2.reason, Some(ValidationReason::BlankLine));

    let r3 = v.validate(3);
    assert!(!r3.verified);
    assert_eq!(r3.reason, Some(ValidationReason::CommentLine));

    assert!(v.is_executable_line(4));
    Ok(())
}

#[test]
fn comment_suggestion_skips_all_comments() -> Result<(), Box<dyn std::error::Error>> {
    let source = "# c1\n# c2\n# c3\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    let nearest = find_nearest_valid_line(&v, 1, SearchDirection::Forward, None);
    assert_eq!(nearest, Some(4));
    Ok(())
}

#[test]
fn inline_comment_line_still_executable() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1; # inline comment\nmy $y = 2;\n";
    let v = must(AstBreakpointValidator::new(source));

    // Line with inline comment has executable code -- should be verified
    assert!(v.is_executable_line(1));
    Ok(())
}

// ---------------------------------------------------------------------------
// Duplicate breakpoint handling (same line, different conditions)
// ---------------------------------------------------------------------------

#[test]
fn same_line_multiple_conditions_all_valid() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 10;\nmy $y = 20;\n";
    let v = must(AstBreakpointValidator::new(source));

    // Multiple different conditions on the same line all validate independently
    let r1 = v.validate_condition(1, "$x > 5");
    assert!(r1.verified, "first condition on line 1 should be valid");

    let r2 = v.validate_condition(1, "$x == 10");
    assert!(r2.verified, "second condition on line 1 should be valid");

    let r3 = v.validate_condition(1, "$x < 100");
    assert!(r3.verified, "third condition on line 1 should be valid");
    Ok(())
}

#[test]
fn same_line_one_valid_one_invalid_condition() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    let r_valid = v.validate_condition(1, "$x > 0");
    assert!(r_valid.verified);

    let r_invalid = v.validate_condition(1, "");
    assert!(!r_invalid.verified);
    assert_eq!(r_invalid.reason, Some(ValidationReason::InvalidCondition));
    Ok(())
}

#[test]
fn same_line_unconditional_and_conditional() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    // Unconditional breakpoint (just validate the line)
    let r_uncond = v.validate(1);
    assert!(r_uncond.verified);

    // Conditional breakpoint on the same line
    let r_cond = v.validate_condition(1, "$x == 1");
    assert!(r_cond.verified);
    Ok(())
}

#[test]
fn same_line_different_operator_conditions() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $count = 0;\n";
    let v = must(AstBreakpointValidator::new(source));

    // Different operators on the same variable, same line
    let operators = [
        "$count % 10 == 0",
        "$count >= 100",
        "$count <= 50",
        "$count == 42",
        "$count > 0",
        "$count < 1000",
        "$count != 7",
    ];

    for cond in &operators {
        let result = v.validate_condition(1, cond);
        assert!(result.verified, "condition '{cond}' should be valid on line 1");
    }
    Ok(())
}

#[test]
fn same_line_condition_then_dangerous_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    // Valid condition first
    let r_valid = v.validate_condition(1, "$x > 0");
    assert!(r_valid.verified);

    // Dangerous condition on same line -- rejected
    let r_danger = v.validate_condition(1, "system('echo hi')");
    assert!(!r_danger.verified);
    assert_eq!(r_danger.reason, Some(ValidationReason::InvalidCondition));
    Ok(())
}

// ---------------------------------------------------------------------------
// Repeated validation stability (same validator, many calls)
// ---------------------------------------------------------------------------

#[test]
fn repeated_validate_same_line_gives_same_result() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n# comment\n\nmy $y = 2;\n";
    let v = must(AstBreakpointValidator::new(source));

    // Validate each line multiple times -- results must be stable
    for _ in 0..10 {
        assert!(v.validate(1).verified);
        assert!(!v.validate(2).verified);
        assert!(!v.validate(3).verified);
        assert!(v.validate(4).verified);
    }
    Ok(())
}

#[test]
fn repeated_condition_validate_stable() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    for _ in 0..10 {
        let result = v.validate_condition(1, "$x > 0");
        assert!(result.verified);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Condition on invalid lines
// ---------------------------------------------------------------------------

#[test]
fn condition_on_empty_line_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n\nmy $y = 2;\n";
    let v = must(AstBreakpointValidator::new(source));

    let result = v.validate_condition(2, "$x > 0");
    assert!(!result.verified);
    assert_eq!(result.reason, Some(ValidationReason::BlankLine));
    Ok(())
}

#[test]
fn condition_on_comment_line_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let source = "# comment\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    let result = v.validate_condition(1, "$x == 1");
    assert!(!result.verified);
    assert_eq!(result.reason, Some(ValidationReason::CommentLine));
    Ok(())
}

#[test]
fn condition_on_out_of_range_line_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    let result = v.validate_condition(999, "$x > 0");
    assert!(!result.verified);
    assert_eq!(result.reason, Some(ValidationReason::LineOutOfRange));
    Ok(())
}

#[test]
fn condition_on_heredoc_interior_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $t = <<END;\nbody line\nEND\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    let result = v.validate_condition(2, "$x > 0");
    assert!(!result.verified);
    // Heredoc interior takes priority
    assert_eq!(result.reason, Some(ValidationReason::HeredocInterior));
    Ok(())
}

// ---------------------------------------------------------------------------
// Mixed multi-line constructs with breakpoint placement
// ---------------------------------------------------------------------------

#[test]
fn heredoc_followed_by_blank_and_comment() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $t = <<END;\nheredoc body\nEND\n\n# comment\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    assert!(v.is_executable_line(1)); // heredoc start
    let r2 = v.validate(2);
    assert!(!r2.verified); // heredoc body

    let r4 = v.validate(4);
    assert!(!r4.verified);
    assert_eq!(r4.reason, Some(ValidationReason::BlankLine));

    let r5 = v.validate(5);
    assert!(!r5.verified);
    assert_eq!(r5.reason, Some(ValidationReason::CommentLine));

    assert!(v.is_executable_line(6));
    Ok(())
}

#[test]
fn package_with_pod_and_heredoc() -> Result<(), Box<dyn std::error::Error>> {
    let source = "package MyApp;\n\n=head1 NAME\n\nMyApp - test\n\n=cut\n\nmy $t = <<END;\nheredoc body\nEND\nmy $x = 1;\n";
    let v = must(AstBreakpointValidator::new(source));

    // package line
    assert!(v.is_executable_line(1));

    // blank line
    let r2 = v.validate(2);
    assert!(!r2.verified);
    assert_eq!(r2.reason, Some(ValidationReason::BlankLine));

    // POD section
    let r3 = v.validate(3);
    assert!(!r3.verified);
    assert_eq!(r3.reason, Some(ValidationReason::PodLine));

    let r5 = v.validate(5);
    assert!(!r5.verified);
    assert_eq!(r5.reason, Some(ValidationReason::PodLine));

    // heredoc start
    assert!(v.is_executable_line(9));

    // heredoc body
    let r10 = v.validate(10);
    assert!(!r10.verified);

    // code after heredoc
    assert!(v.is_executable_line(12));
    Ok(())
}

// ---------------------------------------------------------------------------
// Edge: file consisting entirely of empty lines
// ---------------------------------------------------------------------------

#[test]
fn file_all_empty_lines() -> Result<(), Box<dyn std::error::Error>> {
    let source = "\n\n\n\n\n";
    let v = must(AstBreakpointValidator::new(source));

    for line in 1..=5 {
        let result = v.validate(line);
        assert!(!result.verified, "line {line} should not be verified");
        assert_eq!(
            result.reason,
            Some(ValidationReason::BlankLine),
            "line {line} should be BlankLine"
        );
    }

    // No valid lines in any direction
    let nearest = find_nearest_valid_line(&v, 3, SearchDirection::Both, None);
    assert_eq!(nearest, None);
    Ok(())
}

// ---------------------------------------------------------------------------
// Edge: file consisting entirely of comments
// ---------------------------------------------------------------------------

#[test]
fn file_all_comments() -> Result<(), Box<dyn std::error::Error>> {
    let source = "# line 1\n# line 2\n# line 3\n";
    let v = must(AstBreakpointValidator::new(source));

    for line in 1..=3 {
        let result = v.validate(line);
        assert!(!result.verified, "line {line} should not be verified");
        assert_eq!(
            result.reason,
            Some(ValidationReason::CommentLine),
            "line {line} should be CommentLine"
        );
    }

    let nearest = find_nearest_valid_line(&v, 2, SearchDirection::Both, None);
    assert_eq!(nearest, None);
    Ok(())
}

// ---------------------------------------------------------------------------
// Edge: condition with string comparison operators
// ---------------------------------------------------------------------------

#[test]
fn condition_string_operators() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $name = 'test';\n";
    let v = must(AstBreakpointValidator::new(source));

    let result = v.validate_condition(1, "$name eq 'foo'");
    assert!(result.verified, "eq should parse");

    let result = v.validate_condition(1, "$name ne 'bar'");
    assert!(result.verified, "ne should parse");

    let result = v.validate_condition(1, "$name lt 'zzz'");
    assert!(result.verified, "lt should parse");

    let result = v.validate_condition(1, "$name gt 'aaa'");
    assert!(result.verified, "gt should parse");

    let result = v.validate_condition(1, "$name le 'test'");
    assert!(result.verified, "le should parse");

    let result = v.validate_condition(1, "$name ge 'test'");
    assert!(result.verified, "ge should parse");
    Ok(())
}

// ---------------------------------------------------------------------------
// Edge: condition with special Perl expressions
// ---------------------------------------------------------------------------

#[test]
fn condition_defined_and_ref_checks() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = undef;\n";
    let v = must(AstBreakpointValidator::new(source));

    let result = v.validate_condition(1, "defined($x)");
    assert!(result.verified);

    let result = v.validate_condition(1, "ref($x) eq 'HASH'");
    assert!(result.verified);
    Ok(())
}

#[test]
fn condition_array_scalar_context() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my @arr = (1, 2, 3);\n";
    let v = must(AstBreakpointValidator::new(source));

    let result = v.validate_condition(1, "scalar(@arr) > 0");
    assert!(result.verified);
    Ok(())
}
