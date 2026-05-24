//! Contract tests for the public inline-values surface.
//!
//! `inline_values::collect_inline_values` already has light coverage in
//! `dap_coverage_audit_tests.rs`, but its companion APIs -
//! `extract_variable_names`, `format_inline_value`, and
//! `collect_inline_values_with_runtime` - were not directly exercised.
//!
//! These tests target the documented behaviour of each public function:
//! line-bound normalization, code/string/comment masking, sigil-based
//! formatting, runtime-value enrichment, and the special-variable filter.

use std::collections::HashMap;

use perl_dap::inline_values::{
    collect_inline_values_with_runtime, extract_variable_names, format_inline_value,
};

// ---- extract_variable_names ------------------------------------------------

#[test]
fn extract_variable_names_returns_empty_for_empty_source() {
    let names = extract_variable_names("", 1, 10);
    assert!(names.is_empty());
}

#[test]
fn extract_variable_names_returns_empty_when_start_past_end_of_file() {
    let names = extract_variable_names("my $x = 1;\n", 100, 200);
    assert!(names.is_empty());
}

#[test]
fn extract_variable_names_clamps_negative_start_to_first_line() {
    // start_line below 1 should be clamped, not produce a panic or empty result
    let names = extract_variable_names("my $first = 1;\n", -5, 5);
    assert!(names.contains(&"$first".to_string()), "got names {names:?}");
}

#[test]
fn extract_variable_names_clamps_end_beyond_file_to_last_line() {
    let source = "my $x = 1;\nmy $y = 2;\n";
    let names = extract_variable_names(source, 1, 999);
    assert!(names.contains(&"$x".to_string()), "got names {names:?}");
    assert!(names.contains(&"$y".to_string()), "got names {names:?}");
}

#[test]
fn extract_variable_names_picks_up_array_and_hash_sigils() {
    let source = "my @arr = (1, 2, 3);\nmy %map = (a => 1);\n";
    let names = extract_variable_names(source, 1, 2);
    assert!(names.contains(&"@arr".to_string()), "expected @arr in {names:?}");
    assert!(names.contains(&"%map".to_string()), "expected %map in {names:?}");
}

#[test]
fn extract_variable_names_deduplicates_repeats_across_lines() {
    let source = "$x = 1;\nprint $x;\n$x++;\n";
    let names = extract_variable_names(source, 1, 3);
    let count = names.iter().filter(|n| *n == "$x").count();
    assert_eq!(count, 1, "expected $x to appear once, got {names:?}");
}

#[test]
fn extract_variable_names_skips_variables_inside_strings() {
    let source = r#"my $real = 1; print "literal $fake here";"#;
    let names = extract_variable_names(source, 1, 1);
    assert!(names.contains(&"$real".to_string()), "expected $real in {names:?}");
    assert!(!names.contains(&"$fake".to_string()), "must skip string content: {names:?}");
}

#[test]
fn extract_variable_names_skips_variables_inside_comments() {
    let source = "my $real = 1; # mention $ignored\n";
    let names = extract_variable_names(source, 1, 1);
    assert!(names.contains(&"$real".to_string()), "expected $real in {names:?}");
    assert!(!names.contains(&"$ignored".to_string()), "must skip comment content: {names:?}");
}

#[test]
fn extract_variable_names_skips_listed_special_variables() {
    // $_, @ARGV, %ENV are Perl special globals on the filter list; @_ is not.
    let source = "my $regular = $_;\nfor my $arg (@ARGV) { print $ENV{HOME}; }\n";
    let names = extract_variable_names(source, 1, 2);
    assert!(names.contains(&"$regular".to_string()), "expected $regular in {names:?}");
    assert!(!names.contains(&"$_".to_string()), "must skip $_ in {names:?}");
    assert!(!names.contains(&"@ARGV".to_string()), "must skip @ARGV in {names:?}");
}

// ---- format_inline_value ---------------------------------------------------

#[test]
fn format_inline_value_renders_scalar_with_trimmed_value() {
    assert_eq!(format_inline_value("$x", "  42  "), "$x = 42");
}

#[test]
fn format_inline_value_truncates_long_scalar_with_ellipsis() {
    let long = "a".repeat(80);
    let rendered = format_inline_value("$msg", &long);
    // The contract: scalars > 60 chars get the first 57 chars and an ellipsis.
    assert!(rendered.starts_with("$msg = "));
    assert!(rendered.ends_with("..."), "expected ellipsis suffix, got {rendered}");
    // 57 chars + "..." = 60 total in the value segment (after the "$msg = " prefix)
    let value_part = rendered.strip_prefix("$msg = ").unwrap_or("");
    assert_eq!(value_part.chars().count(), 60, "unexpected truncation shape: {rendered}");
}

#[test]
fn format_inline_value_keeps_60_char_scalar_intact() {
    // Exactly 60 chars is the boundary; it should not truncate.
    let exactly_60 = "x".repeat(60);
    let rendered = format_inline_value("$x", &exactly_60);
    assert!(!rendered.ends_with("..."), "60-char scalar must not be truncated: {rendered}");
    assert!(rendered.ends_with(&exactly_60));
}

#[test]
fn format_inline_value_renders_array_with_element_count() {
    assert_eq!(format_inline_value("@arr", "5"), "@arr = (5 elements)");
}

#[test]
fn format_inline_value_renders_array_with_question_mark_for_non_numeric() {
    assert_eq!(format_inline_value("@arr", "(1, 2, 3)"), "@arr = (? elements)");
}

#[test]
fn format_inline_value_renders_hash_with_key_count() {
    assert_eq!(format_inline_value("%h", "3"), "%h = (3 keys)");
}

#[test]
fn format_inline_value_renders_hash_with_question_mark_for_non_numeric() {
    assert_eq!(format_inline_value("%h", "(a => 1, b => 2)"), "%h = (? keys)");
}

#[test]
fn format_inline_value_handles_array_count_with_surrounding_whitespace() {
    assert_eq!(format_inline_value("@arr", "  7\n"), "@arr = (7 elements)");
}

#[test]
fn format_inline_value_treats_empty_name_as_scalar() {
    // Defensive: an empty name falls into the scalar path (sigil defaults to '$').
    let rendered = format_inline_value("", "value");
    assert_eq!(rendered, " = value");
}

// ---- collect_inline_values_with_runtime ------------------------------------

#[test]
fn runtime_inline_values_use_placeholder_when_no_map_provided() {
    let source = "my $x = 1;\n";
    let values = collect_inline_values_with_runtime(source, 1, 1, None);
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].line, 1);
    assert_eq!(values[0].text, "$x = ?");
    assert!(values[0].column >= 1, "column should be 1-based");
}

#[test]
fn runtime_inline_values_use_runtime_map_when_available() {
    let source = "my $x = 1;\n";
    let mut runtime = HashMap::new();
    runtime.insert("$x".to_string(), "42".to_string());

    let values = collect_inline_values_with_runtime(source, 1, 1, Some(&runtime));
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].text, "$x = 42");
}

#[test]
fn runtime_inline_values_format_arrays_and_hashes_via_runtime_lookup() {
    let source = "my @arr = (1, 2, 3);\nmy %h = (a => 1, b => 2);\n";
    let mut runtime = HashMap::new();
    runtime.insert("@arr".to_string(), "3".to_string());
    runtime.insert("%h".to_string(), "2".to_string());

    let values = collect_inline_values_with_runtime(source, 1, 2, Some(&runtime));
    let texts: Vec<&str> = values.iter().map(|v| v.text.as_str()).collect();
    assert!(texts.contains(&"@arr = (3 elements)"), "got {texts:?}");
    assert!(texts.contains(&"%h = (2 keys)"), "got {texts:?}");
}

#[test]
fn runtime_inline_values_dedupe_within_a_line() {
    // The same variable referenced twice on one line should only produce one
    // inline value entry for that line.
    let source = "$x = $x + 1;\n";
    let values = collect_inline_values_with_runtime(source, 1, 1, None);
    let on_line_one: Vec<_> = values.iter().filter(|v| v.line == 1 && v.text == "$x = ?").collect();
    assert_eq!(on_line_one.len(), 1, "got {values:?}");
}

#[test]
fn runtime_inline_values_keep_same_name_on_distinct_lines() {
    // The dedupe key includes the line index, so the same name on different
    // lines is reported once per line.
    let source = "$x = 1;\n$x = 2;\n";
    let values = collect_inline_values_with_runtime(source, 1, 2, None);
    let xs: Vec<_> = values.iter().filter(|v| v.text == "$x = ?").collect();
    assert_eq!(xs.len(), 2, "expected one entry per line, got {values:?}");
    assert!(xs.iter().any(|v| v.line == 1));
    assert!(xs.iter().any(|v| v.line == 2));
}

#[test]
fn runtime_inline_values_skip_variables_inside_strings_and_comments() {
    let source = "my $real = 1; # $hidden\nprint \"$fake\";\n";
    let values = collect_inline_values_with_runtime(source, 1, 2, None);
    let texts: Vec<&str> = values.iter().map(|v| v.text.as_str()).collect();
    assert!(texts.contains(&"$real = ?"), "got {texts:?}");
    assert!(!texts.iter().any(|t| t.starts_with("$hidden")), "must skip comment: {texts:?}");
    assert!(!texts.iter().any(|t| t.starts_with("$fake")), "must skip string: {texts:?}");
}

#[test]
fn runtime_inline_values_returns_empty_when_lines_collapse_to_nothing() {
    // start past last line returns an empty result, with no panic
    let values = collect_inline_values_with_runtime("my $x = 1;\n", 50, 60, None);
    assert!(values.is_empty());
}

#[test]
fn runtime_inline_values_returns_empty_for_empty_source() {
    assert!(collect_inline_values_with_runtime("", 1, 10, None).is_empty());
}
