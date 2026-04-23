//! Inline value extraction for DAP inlineValues requests.
//!
//! This module provides both a lightweight regex-based implementation and
//! a runtime-enriched version that queries the Perl debugger for actual
//! variable values.

use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::{HashMap, HashSet};

use crate::protocol::InlineValueText;

/// Regex for matching Perl variables (scalars, arrays, hashes).
/// Stored as Option to avoid panics; if compilation fails, inline values are skipped.
static PERL_VAR_RE: Lazy<Option<Regex>> = Lazy::new(|| {
    Regex::new(r"[$@%][A-Za-z_][A-Za-z0-9_]*(?:(?:::|')[A-Za-z_][A-Za-z0-9_]*)*").ok()
});

/// Regex for braced Perl variables (`${name}`, `@{Pkg::arr}`, `%{cfg}`).
static BRACED_PERL_VAR_RE: Lazy<Option<Regex>> = Lazy::new(|| {
    Regex::new(r"([$@%])\{([A-Za-z_][A-Za-z0-9_]*(?:(?:::|')[A-Za-z_][A-Za-z0-9_]*)*)\}").ok()
});

/// Legacy regex for scalar-only matching (used by `DapDispatcher`).
static SCALAR_VAR_RE: Lazy<Option<Regex>> =
    Lazy::new(|| Regex::new(r"\$[A-Za-z_][A-Za-z0-9_]*(?:(?:::|')[A-Za-z_][A-Za-z0-9_]*)*").ok());

/// Special Perl variables that should not be shown inline.
const SPECIAL_VARS: &[&str] = &[
    "$_", "$!", "$@", "$/", "$\\", "$,", "$;", "$\"", "$^", "$~", "$=", "$-", "$%", "$^W", "$^O",
    "$0", "$^T", "%ENV", "%SIG", "@ARGV", "@ISA", "@INC",
];

/// Check if a variable name is a special variable that should be excluded.
fn is_special_variable_name(name: &str) -> bool {
    SPECIAL_VARS.contains(&name)
}

/// Convert 1-based line bounds into inclusive 0-based indexes.
///
/// Non-positive line inputs are clamped to line 1. End bounds past the
/// available line count are clamped to the final line.
fn normalize_line_bounds(
    start_line: i64,
    end_line: i64,
    line_count: usize,
) -> Option<(usize, usize)> {
    if line_count == 0 {
        return None;
    }

    let start_1_based = start_line.max(1) as usize;
    if start_1_based > line_count {
        return None;
    }
    let end_1_based = (end_line.max(1) as usize).min(line_count);

    let start_idx = start_1_based.saturating_sub(1);
    let end_idx = end_1_based.saturating_sub(1);

    (start_idx <= end_idx).then_some((start_idx, end_idx))
}

/// Build a byte-level mask for regions that should be considered executable code.
///
/// Bytes that belong to Perl comments (`# ...`) or single/double-quoted string
/// literals are marked `false` and should be ignored by lightweight regex scans.
fn code_byte_mask(line: &str) -> Vec<bool> {
    fn is_ident_byte(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || byte == b'_'
    }

    let bytes = line.as_bytes();
    let mut mask = vec![true; bytes.len()];
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];

        if in_single {
            mask[i] = false;
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'\'' {
                in_single = false;
            }
            i += 1;
            continue;
        }

        if in_double {
            mask[i] = false;
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_double = false;
            }
            i += 1;
            continue;
        }

        if let Some(end_idx) = parse_quote_like_operator(bytes, i) {
            for byte in mask.iter_mut().take(end_idx).skip(i) {
                *byte = false;
            }
            i = end_idx;
            continue;
        }

        match b {
            b'#' => {
                if is_perl_array_length_marker(bytes, i) {
                    i += 1;
                    continue;
                }
                for byte in mask.iter_mut().take(bytes.len()).skip(i) {
                    *byte = false;
                }
                break;
            }
            b'\'' => {
                let prev_is_ident = i > 0 && is_ident_byte(bytes[i - 1]);
                let next_is_ident = (i + 1) < bytes.len() && is_ident_byte(bytes[i + 1]);

                if prev_is_ident && next_is_ident {
                    // Legacy Perl namespace separator (e.g. `$Foo'bar`).
                    i += 1;
                    continue;
                }

                mask[i] = false;
                in_single = true;
            }
            b'"' => {
                mask[i] = false;
                in_double = true;
            }
            _ => {}
        }

        i += 1;
    }

    mask
}

fn is_perl_array_length_marker(bytes: &[u8], idx: usize) -> bool {
    if idx > 0 && bytes[idx - 1] == b'$' {
        return true;
    }
    idx > 1 && bytes[idx - 1] == b'{' && bytes[idx - 2] == b'$'
}

/// Parse Perl quote-like operators (`q`, `qq`, `qw`, `qr`, `qx`) at `start`.
///
/// Returns the end index (exclusive) of the full quote-like segment when found.
fn parse_quote_like_operator(bytes: &[u8], start: usize) -> Option<usize> {
    let prev_is_ident =
        start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_');
    if prev_is_ident {
        return None;
    }

    let operators =
        [b"qq".as_slice(), b"qw".as_slice(), b"qr".as_slice(), b"qx".as_slice(), b"q".as_slice()];
    for op in operators {
        let op_end = start + op.len();
        if op_end > bytes.len() || &bytes[start..op_end] != op {
            continue;
        }

        let mut idx = op_end;
        while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
            idx += 1;
        }
        if idx >= bytes.len() {
            return None;
        }

        let open = bytes[idx];
        if open.is_ascii_alphanumeric() || open == b'_' {
            continue;
        }
        let (close, paired) = matching_delimiter(open);
        idx += 1;
        let mut escaped = false;
        let mut depth = if paired { 1usize } else { 0usize };

        while idx < bytes.len() {
            let cur = bytes[idx];
            if escaped {
                escaped = false;
                idx += 1;
                continue;
            }
            if cur == b'\\' {
                escaped = true;
                idx += 1;
                continue;
            }

            if paired {
                if cur == open {
                    depth += 1;
                } else if cur == close {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Some(idx + 1);
                    }
                }
            } else if cur == close {
                return Some(idx + 1);
            }
            idx += 1;
        }

        return Some(bytes.len());
    }

    None
}

fn matching_delimiter(open: u8) -> (u8, bool) {
    match open {
        b'(' => (b')', true),
        b'[' => (b']', true),
        b'{' => (b'}', true),
        b'<' => (b'>', true),
        _ => (open, false),
    }
}

fn is_identifier_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn collect_line_variables(line: &str, include_non_scalars: bool) -> Vec<(usize, usize, String)> {
    let mut matches = Vec::new();

    let base_re = if include_non_scalars { PERL_VAR_RE.as_ref() } else { SCALAR_VAR_RE.as_ref() };
    if let Some(re) = base_re {
        for cap in re.captures_iter(line) {
            if let Some(m) = cap.get(0) {
                matches.push((m.start(), m.end(), m.as_str().to_string()));
            }
        }
    }

    if let Some(re) = BRACED_PERL_VAR_RE.as_ref() {
        for cap in re.captures_iter(line) {
            let (Some(full_match), Some(sigil_match), Some(name_match)) =
                (cap.get(0), cap.get(1), cap.get(2))
            else {
                continue;
            };
            if !include_non_scalars && sigil_match.as_str() != "$" {
                continue;
            }
            matches.push((
                full_match.start(),
                full_match.end(),
                format!("{}{}", sigil_match.as_str(), name_match.as_str()),
            ));
        }
    }

    matches.sort_by_key(|(start, _, _)| *start);
    matches
}

/// Extract unique variable names from source code within a line range.
///
/// Lines are 1-based. Returns deduplicated variable names with their sigils.
pub fn extract_variable_names(source: &str, start_line: i64, end_line: i64) -> Vec<String> {
    if PERL_VAR_RE.is_none() && BRACED_PERL_VAR_RE.is_none() {
        return Vec::new();
    }
    let lines: Vec<&str> = source.lines().collect();
    if lines.is_empty() {
        return Vec::new();
    }

    let Some((start_idx, end_idx)) = normalize_line_bounds(start_line, end_line, lines.len())
    else {
        return Vec::new();
    };

    let mut seen = HashSet::new();
    let mut names = Vec::new();

    for line in lines.iter().skip(start_idx).take(end_idx - start_idx + 1) {
        let code_mask = code_byte_mask(line);
        for (start, end, name) in collect_line_variables(line, true) {
            if !code_mask[start..end].iter().all(|is_code| *is_code) {
                continue;
            }
            if !is_special_variable_name(&name) && seen.insert(name.clone()) {
                names.push(name);
            }
        }
    }

    names
}

/// Format a variable's inline value with Perl-idiomatic formatting.
///
/// - Scalars: `$x = "value"`
/// - Arrays: `@arr = (3 elements)`
/// - Hashes: `%hash = (5 keys)`
/// - Blessed refs: `$obj = Foo=HASH(...)`
pub fn format_inline_value(name: &str, raw_value: &str) -> String {
    let sigil = name.chars().next().unwrap_or('$');
    match sigil {
        '@' => {
            let count = parse_array_element_count(raw_value);
            format!("{name} = ({count} elements)")
        }
        '%' => {
            let count = parse_hash_key_count(raw_value);
            format!("{name} = ({count} keys)")
        }
        _ => {
            let trimmed = raw_value.trim();
            if trimmed.len() > 60 {
                let preview: String = trimmed.chars().take(57).collect();
                format!("{name} = {}...", preview)
            } else {
                format!("{name} = {trimmed}")
            }
        }
    }
}

/// Parse an array element count from a debugger response.
fn parse_array_element_count(raw: &str) -> &str {
    let trimmed = raw.trim();
    if trimmed.chars().all(|c| c.is_ascii_digit()) {
        return trimmed;
    }
    "?"
}

/// Parse a hash key count from a debugger response.
fn parse_hash_key_count(raw: &str) -> &str {
    let trimmed = raw.trim();
    if trimmed.chars().all(|c| c.is_ascii_digit()) {
        return trimmed;
    }
    "?"
}

/// Collect inline values with runtime variable resolution.
///
/// When `runtime_values` is provided, variables are displayed with their
/// actual values from the debugger. Otherwise, a `= ?` placeholder is used.
///
/// Lines and columns are 1-based to match the DAP defaults.
pub fn collect_inline_values_with_runtime(
    source: &str,
    start_line: i64,
    end_line: i64,
    runtime_values: Option<&HashMap<String, String>>,
) -> Vec<InlineValueText> {
    if PERL_VAR_RE.is_none() && BRACED_PERL_VAR_RE.is_none() {
        return Vec::new();
    }
    let lines: Vec<&str> = source.lines().collect();
    if lines.is_empty() {
        return Vec::new();
    }

    let Some((start_idx, end_idx)) = normalize_line_bounds(start_line, end_line, lines.len())
    else {
        return Vec::new();
    };

    let mut inline_values = Vec::new();
    let mut seen_on_line: HashSet<(usize, String)> = HashSet::new();

    for (idx, line) in lines.iter().enumerate().skip(start_idx).take(end_idx - start_idx + 1) {
        let code_mask = code_byte_mask(line);
        for (start, end, var_name) in collect_line_variables(line, true) {
            if !code_mask[start..end].iter().all(|is_code| *is_code) {
                continue;
            }
            if is_special_variable_name(&var_name) {
                continue;
            }
            if !seen_on_line.insert((idx, var_name.clone())) {
                continue;
            }
            let column = (start + 1) as i64;
            let text = match runtime_values.and_then(|rv| rv.get(&var_name)) {
                Some(rv) => format_inline_value(&var_name, rv),
                None => format!("{} = ?", var_name),
            };
            inline_values.push(InlineValueText { line: (idx + 1) as i64, column, text });
        }
    }

    inline_values
}

/// Legacy: Collect inline values for scalar variables within a line range.
///
/// Lines and columns are 1-based to match the DAP defaults.
/// Kept for backward compatibility with `DapDispatcher`.
pub fn collect_inline_values(source: &str, start_line: i64, end_line: i64) -> Vec<InlineValueText> {
    let lines: Vec<&str> = source.lines().collect();
    if lines.is_empty() {
        return Vec::new();
    }

    let Some((start_idx, end_idx)) = normalize_line_bounds(start_line, end_line, lines.len())
    else {
        return Vec::new();
    };

    if SCALAR_VAR_RE.is_none() && BRACED_PERL_VAR_RE.is_none() {
        return Vec::new();
    }
    let mut inline_values = Vec::new();

    for (idx, line) in lines.iter().enumerate().skip(start_idx).take(end_idx - start_idx + 1) {
        let code_mask = code_byte_mask(line);
        for (start, end, var_text) in collect_line_variables(line, false) {
            if !code_mask[start..end].iter().all(|is_code| *is_code) {
                continue;
            }
            let column = (start + 1) as i64;
            inline_values.push(InlineValueText {
                line: (idx + 1) as i64,
                column,
                text: format!("{} = ?", var_text),
            });
        }
    }

    inline_values
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_inline_values_legacy() {
        let source = "my $x = 1;\nmy $y = $x + 2;";
        let values = collect_inline_values(source, 1, 2);
        assert!(values.iter().any(|v| v.text.contains("$x")));
        assert!(values.iter().any(|v| v.text.contains("$y")));
    }

    #[test]
    fn test_scalar_inline_value() {
        let source = "my $name = \"Hello\";";
        let mut rv = HashMap::new();
        rv.insert("$name".to_string(), "'Hello'".to_string());
        let values = collect_inline_values_with_runtime(source, 1, 1, Some(&rv));
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].text, "$name = 'Hello'");
    }

    #[test]
    fn test_array_inline_value() {
        let source = "my @items = (1, 2, 3);";
        let mut rv = HashMap::new();
        rv.insert("@items".to_string(), "3".to_string());
        let values = collect_inline_values_with_runtime(source, 1, 1, Some(&rv));
        assert_eq!(values[0].text, "@items = (3 elements)");
    }

    #[test]
    fn test_hash_inline_value() {
        let source = "my %config = (a => 1);";
        let mut rv = HashMap::new();
        rv.insert("%config".to_string(), "5".to_string());
        let values = collect_inline_values_with_runtime(source, 1, 1, Some(&rv));
        assert_eq!(values[0].text, "%config = (5 keys)");
    }

    #[test]
    fn test_blessed_ref_inline_value() {
        let source = "my $obj = Foo->new();";
        let mut rv = HashMap::new();
        rv.insert("$obj".to_string(), "Foo=HASH(0xdeadbeef)".to_string());
        let values = collect_inline_values_with_runtime(source, 1, 1, Some(&rv));
        assert_eq!(values[0].text, "$obj = Foo=HASH(0xdeadbeef)");
    }

    #[test]
    fn test_empty_collections() {
        let mut rv = HashMap::new();
        rv.insert("@empty".to_string(), "0".to_string());
        rv.insert("%none".to_string(), "0".to_string());
        let source = "my @empty; my %none;";
        let values = collect_inline_values_with_runtime(source, 1, 1, Some(&rv));
        assert!(values.iter().any(|v| v.text == "@empty = (0 elements)"));
        assert!(values.iter().any(|v| v.text == "%none = (0 keys)"));
    }

    #[test]
    fn test_deduplication_per_line() {
        let source = "$x = $x + $x;";
        let values = collect_inline_values_with_runtime(source, 1, 1, None);
        assert_eq!(values.len(), 1);
    }

    #[test]
    fn test_special_vars_excluded() {
        let source = "print $_; warn $!; my $val = 1;";
        let values = collect_inline_values_with_runtime(source, 1, 1, None);
        assert_eq!(values.len(), 1);
        assert!(values[0].text.contains("$val"));
    }

    #[test]
    fn test_extract_variable_names() {
        let source = "my $x = 1;\nmy @arr = (1,2,3);\nmy %h = (a => 1);";
        let names = extract_variable_names(source, 1, 3);
        assert!(names.contains(&"$x".to_string()));
        assert!(names.contains(&"@arr".to_string()));
        assert!(names.contains(&"%h".to_string()));
    }

    #[test]
    fn test_extract_variable_names_with_namespace_qualifiers() {
        let source = "our $Foo::bar = 1;\nour @My::Pkg::items = (1);\nour %App::Config::opts = ();";
        let names = extract_variable_names(source, 1, 3);
        assert!(names.contains(&"$Foo::bar".to_string()));
        assert!(names.contains(&"@My::Pkg::items".to_string()));
        assert!(names.contains(&"%App::Config::opts".to_string()));
    }

    #[test]
    fn test_extract_variable_names_with_legacy_namespace_qualifiers() {
        let source = "our $Foo'bar = 1;\nour @My'Pkg'items = (1);\nour %App'Config'opts = ();";
        let names = extract_variable_names(source, 1, 3);
        assert!(names.contains(&"$Foo'bar".to_string()));
        assert!(names.contains(&"@My'Pkg'items".to_string()));
        assert!(names.contains(&"%App'Config'opts".to_string()));
    }

    #[test]
    fn test_no_runtime_fallback() {
        let source = "my $x = 1;";
        let values = collect_inline_values_with_runtime(source, 1, 1, None);
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].text, "$x = ?");
    }

    #[test]
    fn test_scalar_truncation_uses_char_boundaries() {
        let source = "my $name = 1;";
        let long_value = "é".repeat(80);
        let mut rv = HashMap::new();
        rv.insert("$name".to_string(), long_value);

        let values = collect_inline_values_with_runtime(source, 1, 1, Some(&rv));
        assert_eq!(values.len(), 1);
        assert!(values[0].text.starts_with("$name = "));
        assert!(values[0].text.ends_with("..."));
    }

    #[test]
    fn test_non_positive_line_bounds_are_clamped() {
        let source = "my $x = 1;\nmy $y = 2;";
        let names = extract_variable_names(source, 0, 1);
        assert_eq!(names, vec!["$x".to_string()]);

        let values = collect_inline_values_with_runtime(source, 0, 0, None);
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].text, "$x = ?");
    }

    #[test]
    fn test_inverted_line_bounds_return_empty() {
        let source = "my $x = 1;\nmy $y = 2;";
        assert!(extract_variable_names(source, 2, 1).is_empty());
        assert!(collect_inline_values_with_runtime(source, 2, 1, None).is_empty());
        assert!(collect_inline_values(source, 2, 1).is_empty());
    }

    #[test]
    fn test_out_of_range_line_bounds_return_empty() {
        let source = "my $x = 1;\nmy $y = 2;";
        assert!(extract_variable_names(source, 3, 3).is_empty());
        assert!(collect_inline_values_with_runtime(source, 3, 3, None).is_empty());
        assert!(collect_inline_values(source, 3, 3).is_empty());
    }

    #[test]
    fn test_end_line_past_file_is_clamped() {
        let source = "my $x = 1;\nmy $y = 2;";

        let names = extract_variable_names(source, 2, 999);
        assert_eq!(names, vec!["$y".to_string()]);

        let values = collect_inline_values_with_runtime(source, 2, 999, None);
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].text, "$y = ?");

        let legacy_values = collect_inline_values(source, 2, 999);
        assert_eq!(legacy_values.len(), 1);
        assert_eq!(legacy_values[0].text, "$y = ?");
    }

    #[test]
    fn test_runtime_inline_value_for_namespaced_scalar() {
        let source = "our $Foo::bar = 1;";
        let mut rv = HashMap::new();
        rv.insert("$Foo::bar".to_string(), "42".to_string());

        let values = collect_inline_values_with_runtime(source, 1, 1, Some(&rv));
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].text, "$Foo::bar = 42");
    }

    #[test]
    fn test_runtime_inline_value_for_legacy_namespaced_scalar() {
        let source = "our $Foo'bar = 1;";
        let mut rv = HashMap::new();
        rv.insert("$Foo'bar".to_string(), "42".to_string());

        let values = collect_inline_values_with_runtime(source, 1, 1, Some(&rv));
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].text, "$Foo'bar = 42");
    }

    #[test]
    fn test_variable_like_tokens_in_strings_and_comments_are_ignored() {
        let source = "my $real = 1; my $msg = \"$fake\"; # $commented";
        let names = extract_variable_names(source, 1, 1);
        assert!(names.contains(&"$real".to_string()));
        assert!(names.contains(&"$msg".to_string()));
        assert!(!names.contains(&"$fake".to_string()));
        assert!(!names.contains(&"$commented".to_string()));
    }

    #[test]
    fn test_inline_values_ignore_strings_and_comments() {
        let source = "my $real = 1; print \"$ignored\"; # and $commented";
        let values = collect_inline_values_with_runtime(source, 1, 1, None);
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].text, "$real = ?");
    }

    #[test]
    fn test_array_length_marker_does_not_start_comment() {
        let source = "my $len = $#arr; my $next = 1;";

        let names = extract_variable_names(source, 1, 1);
        assert!(names.contains(&"$len".to_string()));
        assert!(names.contains(&"$next".to_string()));

        let values = collect_inline_values_with_runtime(source, 1, 1, None);
        assert!(values.iter().any(|v| v.text == "$len = ?"));
        assert!(values.iter().any(|v| v.text == "$next = ?"));
    }

    #[test]
    fn test_braced_array_length_marker_does_not_start_comment() {
        let source = "my $len = ${#arr}; my $next = 1;";

        let names = extract_variable_names(source, 1, 1);
        assert!(names.contains(&"$len".to_string()));
        assert!(names.contains(&"$next".to_string()));
    }

    #[test]
    fn test_variable_like_tokens_in_quote_like_operators_are_ignored() {
        let source = "my $real = 1; my $str = qq{$fake}; my $lit = q[$ghost];";
        let names = extract_variable_names(source, 1, 1);
        assert!(names.contains(&"$real".to_string()));
        assert!(names.contains(&"$str".to_string()));
        assert!(names.contains(&"$lit".to_string()));
        assert!(!names.contains(&"$fake".to_string()));
        assert!(!names.contains(&"$ghost".to_string()));
    }

    #[test]
    fn test_inline_values_ignore_quote_like_operators() {
        let source = "my $real = 1; my $str = qq($fake); my $lit = q/$ghost/;";
        let values = collect_inline_values_with_runtime(source, 1, 1, None);
        assert_eq!(values.len(), 3);
        assert!(values.iter().any(|v| v.text == "$real = ?"));
        assert!(values.iter().any(|v| v.text == "$str = ?"));
        assert!(values.iter().any(|v| v.text == "$lit = ?"));
        assert!(!values.iter().any(|v| v.text.contains("$fake")));
        assert!(!values.iter().any(|v| v.text.contains("$ghost")));
    }

    #[test]
    fn test_extract_variable_names_supports_braced_variables() {
        let source = "my ${scalar_name} = 1;\nmy @{Pkg::items};\nmy %{App::Config::opts};";
        let names = extract_variable_names(source, 1, 3);
        assert!(names.contains(&"$scalar_name".to_string()));
        assert!(names.contains(&"@Pkg::items".to_string()));
        assert!(names.contains(&"%App::Config::opts".to_string()));
    }

    #[test]
    fn test_inline_values_support_braced_scalars_with_runtime_values() {
        let source = "my ${scalar_name} = 1;";
        let mut rv = HashMap::new();
        rv.insert("$scalar_name".to_string(), "7".to_string());
        let values = collect_inline_values_with_runtime(source, 1, 1, Some(&rv));
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].text, "$scalar_name = 7");
    }

    #[test]
    fn test_legacy_inline_values_support_braced_scalars() {
        let source = "my ${scalar_name} = 1;";
        let values = collect_inline_values(source, 1, 1);
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].text, "$scalar_name = ?");
    }
}
