//! Regression tests for issue #2856: `and` operator doesn't consume
//! comma-separated RHS expressions.
//!
//! Pattern: `/test/ and $x = 1, last;`
//! Root cause: `parse_word_and_expr_with()` called `parse_word_not_expr()` which
//! went through `parse_assignment()` without comma handling. Fixed by applying
//! the same comma-collection logic that `parse_word_or_expr()` uses.

mod cpan_test_helpers;
use cpan_test_helpers::*;

// === Core regression: and + comma-separated RHS ===

#[test]
fn test_and_comma_rhs_basic() {
    // Basic pattern: regex and assignment, last
    assert_clean_parse("/test/ and $x = 1, last;");
}

#[test]
fn test_and_comma_rhs_two_assignments() {
    // Two assignments on the RHS of `and`
    assert_clean_parse("/test/ and $x = 1, $y = 2;");
}

#[test]
fn test_and_comma_rhs_print_close() {
    // Real-world pattern from IPC/Cmd.pm
    assert_clean_parse(r#"open(F, $f) and print F "data\n", close F;"#);
}

#[test]
fn test_and_comma_rhs_last() {
    // Pattern: condition and assignment, control flow verb
    assert_clean_parse("$ok and $n = 1, last;");
}

#[test]
fn test_and_comma_rhs_next() {
    assert_clean_parse("$ok and $n++, next;");
}

#[test]
fn test_and_comma_rhs_three_elements() {
    // Three comma-separated elements on `and` RHS
    assert_clean_parse("$ok and $a = 1, $b = 2, $c = 3;");
}

// === Real-world patterns from affected CPAN files ===

#[test]
fn test_and_comma_file_copy_pattern() {
    // Pattern from File/Copy.pm
    assert_clean_parse(r#"$ok and $copied++, last;"#);
}

#[test]
fn test_and_comma_text_balanced_pattern() {
    // Pattern from Text/Balanced.pm: track state and advance
    assert_clean_parse("$match and $pos = $next, $depth++, last;");
}

#[test]
fn test_and_comma_ipc_cmd_pattern() {
    // Pattern from IPC/Cmd.pm: write data and track
    assert_clean_parse(r#"$ok and syswrite($fh, $data), $written += length($data);"#);
}

#[test]
fn test_and_comma_unicode_ucd_pattern() {
    // Pattern from Unicode/UCD.pm: condition and list of updates
    assert_clean_parse("defined $val and $result = $val, $found++, last;");
}

// === and + comma with string on RHS ===

#[test]
fn test_and_comma_rhs_with_string() {
    assert_clean_parse(r#"/error/ and $msg = "found", last;"#);
}

// === Ensure `or` with commas still works (no regression) ===

#[test]
fn test_or_comma_rhs_still_works() {
    assert_clean_parse("/test/ or $x = 1, last;");
}

#[test]
fn test_or_comma_rhs_two_elements() {
    assert_clean_parse("/test/ or $x = 1, $y = 2;");
}

// === and without comma still works (no regression) ===

#[test]
fn test_and_no_comma_still_works() {
    assert_clean_parse("$a and $b;");
}

#[test]
fn test_and_assignment_no_comma() {
    assert_clean_parse("$a and $b = 1;");
}

#[test]
fn test_and_die_no_comma() {
    assert_clean_parse(r#"$ok and die "failed";"#);
}

// === and chains with commas ===

#[test]
fn test_and_chain_with_comma_on_last() {
    // `and` chain where the rightmost has commas
    assert_clean_parse("$a and $b and $x = 1, last;");
}

// === not on RHS with comma ===

#[test]
fn test_and_not_rhs_no_comma() {
    // not applies only to its operand; comma is still collected
    assert_clean_parse("$a and not $b;");
}
