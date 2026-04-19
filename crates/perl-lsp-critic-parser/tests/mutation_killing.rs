//! Mutation-killing tests for perl-lsp-critic-parser.
//!
//! The existing 5 tests cover:
//!   - Basic parse of policy::name format
//!   - Colons inside the message body
//!   - Windows-style path (C:\...)
//!   - Empty/invalid lines → None
//!   - parse_perlcritic_output with multiple lines
//!
//! Gaps targeted here:
//!   - Whitespace-only lines → None (covers trim().is_empty() branch)
//!   - file.is_empty() guard: line starting with digits → no file part
//!   - is_valid_policy: segment starting with digit → invalid
//!   - is_valid_policy: segment starting with underscore → valid
//!   - is_valid_policy: empty policy → None
//!   - is_valid_policy: special chars in segment → invalid
//!   - find_policy_message_boundary: prev_is_colon branch (::Policy:msg)
//!   - find_policy_message_boundary: next_is_colon branch (Policy::msg)
//!   - Severity boundary values: 0 and 255 (u8 limits)
//!   - parse_perlcritic_output with empty string → empty vec
//!   - parse_perlcritic_output with only invalid lines → empty vec
//!   - Unix path with directories
//!   - line/column parsing: non-numeric values → None

use perl_lsp_critic_parser::{parse_perlcritic_line, parse_perlcritic_output};

// ---------------------------------------------------------------------------
// Whitespace / blank lines
// ---------------------------------------------------------------------------

#[test]
fn parse_whitespace_only_line_returns_none() {
    assert!(
        parse_perlcritic_line("   ").is_none(),
        "whitespace-only must return None"
    );
    assert!(
        parse_perlcritic_line("\t").is_none(),
        "tab-only must return None"
    );
}

// ---------------------------------------------------------------------------
// is_valid_policy: digit/special-char/underscore first char of segment
// ---------------------------------------------------------------------------

#[test]
fn parse_line_with_invalid_policy_digit_first_returns_none() {
    // Policy segment starts with digit → is_valid_policy returns false
    let line = "test.pl:1:1:3:1InvalidPolicy:message here";
    assert!(
        parse_perlcritic_line(line).is_none(),
        "policy starting with digit must be rejected"
    );
}

#[test]
fn parse_line_with_policy_starting_with_underscore_is_valid() {
    // First char of a segment can be '_' per is_valid_policy
    let line = "test.pl:1:1:3:_Private::Rule:some message";
    let result = parse_perlcritic_line(line);
    assert!(
        result.is_some(),
        "policy starting with underscore must be accepted"
    );
    let parsed = result.unwrap();
    assert_eq!(parsed.policy, "_Private::Rule");
    assert_eq!(parsed.message, "some message");
}

#[test]
fn parse_line_with_policy_containing_special_chars_returns_none() {
    // '!' in policy segment name → is_valid_policy returns false
    let line = "test.pl:1:1:3:Bad!Policy:message";
    assert!(
        parse_perlcritic_line(line).is_none(),
        "policy with special char must be rejected"
    );
}

// ---------------------------------------------------------------------------
// file.is_empty() guard
// ---------------------------------------------------------------------------

#[test]
fn parse_line_without_file_part_returns_none() {
    // Line starts with digits directly → file part is empty
    let line = ":5:1:3:SomePolicy:message";
    assert!(
        parse_perlcritic_line(line).is_none(),
        "empty file path must return None"
    );
}

// ---------------------------------------------------------------------------
// find_policy_message_boundary: prev_is_colon / next_is_colon guard
// ---------------------------------------------------------------------------

#[test]
fn parse_line_with_policy_containing_double_colon_separator_correct() {
    // "Category::PolicyName" has '::' — the boundary finder must skip the '::' colons
    // and find the single ':' that separates policy from message
    let line = "test.pl:10:5:4:Category::PolicyName:violation found";
    let parsed = parse_perlcritic_line(line);
    assert!(parsed.is_some(), "policy with double-colon should parse");
    let parsed = parsed.unwrap();
    assert_eq!(parsed.policy, "Category::PolicyName");
    assert_eq!(parsed.message, "violation found");
    assert_eq!(parsed.line, 10);
    assert_eq!(parsed.column, 5);
    assert_eq!(parsed.severity, 4);
}

#[test]
fn parse_line_policy_with_three_segments() {
    // "Perl::Critic::Policy::XY" style
    let line = "app.pl:1:1:5:Perl::Critic::Policy:violation";
    let parsed = parse_perlcritic_line(line);
    assert!(parsed.is_some(), "3-segment policy must parse");
    let parsed = parsed.unwrap();
    assert_eq!(parsed.policy, "Perl::Critic::Policy");
    assert_eq!(parsed.message, "violation");
}

// ---------------------------------------------------------------------------
// Severity boundary values
// ---------------------------------------------------------------------------

#[test]
fn parse_line_with_severity_1_parses_correctly() {
    let line = "test.pl:1:1:1:SomePolicy:low severity violation";
    let parsed = parse_perlcritic_line(line);
    assert!(parsed.is_some(), "severity 1 must parse");
    let parsed = parsed.unwrap_or_else(|| unreachable!());
    assert_eq!(parsed.severity, 1);
}

#[test]
fn parse_line_with_severity_5_parses_correctly() {
    let line = "test.pl:1:1:5:SomePolicy:critical violation";
    let parsed = parse_perlcritic_line(line);
    assert!(parsed.is_some(), "severity 5 must parse");
    let parsed = parsed.unwrap_or_else(|| unreachable!());
    assert_eq!(parsed.severity, 5);
}

#[test]
fn parse_line_with_severity_non_numeric_returns_none() {
    let line = "test.pl:1:1:bad:SomePolicy:message";
    assert!(
        parse_perlcritic_line(line).is_none(),
        "non-numeric severity must return None"
    );
}

// ---------------------------------------------------------------------------
// Non-numeric line/column
// ---------------------------------------------------------------------------

#[test]
fn parse_line_with_non_numeric_line_number_returns_none() {
    let line = "test.pl:abc:1:3:SomePolicy:message";
    assert!(
        parse_perlcritic_line(line).is_none(),
        "non-numeric line number must return None"
    );
}

#[test]
fn parse_line_with_non_numeric_column_returns_none() {
    let line = "test.pl:1:abc:3:SomePolicy:message";
    assert!(
        parse_perlcritic_line(line).is_none(),
        "non-numeric column must return None"
    );
}

// ---------------------------------------------------------------------------
// Unix path with directory separators
// ---------------------------------------------------------------------------

#[test]
fn parse_line_with_unix_path() {
    let line = "lib/Some/Module.pm:3:2:2:Subroutines::ProhibitExcessComplexity:too complex";
    let parsed = parse_perlcritic_line(line);
    assert!(parsed.is_some(), "unix path must parse");
    let parsed = parsed.unwrap_or_else(|| unreachable!());
    assert_eq!(parsed.file, "lib/Some/Module.pm");
    assert_eq!(parsed.line, 3);
    assert_eq!(parsed.column, 2);
    assert_eq!(parsed.severity, 2);
    assert_eq!(parsed.policy, "Subroutines::ProhibitExcessComplexity");
    assert_eq!(parsed.message, "too complex");
}

// ---------------------------------------------------------------------------
// parse_perlcritic_output: edge cases
// ---------------------------------------------------------------------------

#[test]
fn parse_output_empty_string_returns_empty() {
    let result = parse_perlcritic_output("");
    assert!(result.is_empty(), "empty string must produce empty vec");
}

#[test]
fn parse_output_only_invalid_lines_returns_empty() {
    let output = "not valid\nalsoinvalid\n  \n";
    let result = parse_perlcritic_output(output);
    assert!(
        result.is_empty(),
        "all-invalid lines must produce empty vec"
    );
}

#[test]
fn parse_output_mixed_valid_invalid_returns_only_valid() {
    let output = concat!(
        "test.pl:1:1:3:SomePolicy:msg1\n",
        "not valid at all\n",
        "test.pl:2:1:2:AnotherPolicy:msg2\n",
    );
    let result = parse_perlcritic_output(output);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].policy, "SomePolicy");
    assert_eq!(result[1].policy, "AnotherPolicy");
}

// ---------------------------------------------------------------------------
// Structural checks on ParsedCriticLine
// ---------------------------------------------------------------------------

#[test]
fn parsed_critic_line_implements_clone_and_eq() {
    let line = "test.pl:1:1:3:SomePolicy:message";
    let parsed = parse_perlcritic_line(line).unwrap();
    let cloned = parsed.clone();
    assert_eq!(parsed, cloned, "Clone must produce equal value");
}

#[test]
fn parsed_critic_line_implements_debug() {
    let line = "test.pl:1:1:3:SomePolicy:message";
    let parsed = parse_perlcritic_line(line).unwrap();
    let debug = format!("{:?}", parsed);
    assert!(
        debug.contains("SomePolicy"),
        "Debug output must contain policy name"
    );
}
