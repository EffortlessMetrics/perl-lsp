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
/// Non-positive line inputs are clamped to line 1, and the end index is
/// rejected when either bound is past the available line count.
fn normalize_line_bounds(
    start_line: i64,
    end_line: i64,
    line_count: usize,
) -> Option<(usize, usize)> {
    if line_count == 0 {
        return None;
    }

    let start_1_based = start_line.max(1) as usize;
    let end_1_based = end_line.max(1) as usize;
    if start_1_based > line_count || end_1_based > line_count {
        return None;
    }

    let start_idx = start_1_based.saturating_sub(1);
    let end_idx = end_1_based.saturating_sub(1);

    (start_idx <= end_idx).then_some((start_idx, end_idx))
}

/// Build a byte-level mask for regions that should be considered executable code.
///
/// Bytes that belong to Perl comments (`# ...`) or single/double-quoted string
/// literals are marked `false` and should be ignored by lightweight regex scans.
fn code_byte_mask(line: &str) -> Vec<bool> {
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

        match b {
            b'#' => {
                for byte in mask.iter_mut().take(bytes.len()).skip(i) {
                    *byte = false;
                }
                break;
            }
            b'\'' => {
                if !is_legacy_namespace_separator(bytes, i) {
                    mask[i] = false;
                    in_single = true;
                }
            }
            b'"' => {
                mask[i] = false;
                in_double = true;
            }
            b'q' => {
                if let Some(end_idx) = quote_like_end(bytes, i) {
                    for byte in mask.iter_mut().take(end_idx).skip(i) {
                        *byte = false;
                    }
                    i = end_idx;
                    continue;
                }
            }
            _ => {}
        }

        i += 1;
    }

    mask
}

/// Return the end offset (exclusive) for a Perl quote-like operator span starting at `i`.
///
/// Handles `q`, `qq`, `qw`, `qx`, and `qr` forms with optional whitespace before delimiters.
fn quote_like_end(bytes: &[u8], i: usize) -> Option<usize> {
    if i > 0 && (is_ident_char(bytes[i - 1]) || is_variable_sigil(bytes[i - 1])) {
        return None;
    }

    let mut j = i + 1;
    if j < bytes.len() && matches!(bytes[j], b'q' | b'w' | b'x' | b'r') {
        j += 1;
    }

    if j < bytes.len() && is_ident_char(bytes[j]) {
        return None;
    }

    while j < bytes.len() && bytes[j].is_ascii_whitespace() {
        j += 1;
    }
    if j >= bytes.len() {
        return None;
    }

    let open = bytes[j];
    let close = match open {
        b'(' => b')',
        b'[' => b']',
        b'{' => b'}',
        b'<' => b'>',
        _ => open,
    };

    let mut k = j + 1;
    let mut depth = 1usize;
    let mut escaped = false;
    while k < bytes.len() {
        let b = bytes[k];
        if escaped {
            escaped = false;
            k += 1;
            continue;
        }
        if b == b'\\' {
            escaped = true;
            k += 1;
            continue;
        }
        if open != close && b == open {
            depth += 1;
            k += 1;
            continue;
        }
        if b == close {
            depth = depth.saturating_sub(1);
            k += 1;
            if depth == 0 {
                return Some(k);
            }
            continue;
        }
        k += 1;
    }

    Some(bytes.len())
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn is_variable_sigil(b: u8) -> bool {
    matches!(b, b'$' | b'@' | b'%' | b'&')
}

fn is_legacy_namespace_separator(bytes: &[u8], i: usize) -> bool {
    if i == 0 || i + 1 >= bytes.len() {
        return false;
    }
    is_ident_char(bytes[i - 1]) && is_ident_char(bytes[i + 1])
}

/// Extract unique variable names from source code within a line range.
///
/// Lines are 1-based. Returns deduplicated variable names with their sigils.
pub fn extract_variable_names(source: &str, start_line: i64, end_line: i64) -> Vec<String> {
    let Some(re) = PERL_VAR_RE.as_ref() else {
        return Vec::new();
    };
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
        for cap in re.captures_iter(line) {
            if let Some(m) = cap.get(0) {
                if !code_mask[m.start()..m.end()].iter().all(|is_code| *is_code) {
                    continue;
                }
                let name = m.as_str();
                if !is_special_variable_name(name) && seen.insert(name.to_string()) {
                    names.push(name.to_string());
                }
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
    let Some(re) = PERL_VAR_RE.as_ref() else {
        return Vec::new();
    };
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
        for cap in re.captures_iter(line) {
            if let Some(m) = cap.get(0) {
                if !code_mask[m.start()..m.end()].iter().all(|is_code| *is_code) {
                    continue;
                }
                let var_name = m.as_str();
                if is_special_variable_name(var_name) {
                    continue;
                }
                if !seen_on_line.insert((idx, var_name.to_string())) {
                    continue;
                }
                let column = (m.start() + 1) as i64;
                let text = match runtime_values.and_then(|rv| rv.get(var_name)) {
                    Some(rv) => format_inline_value(var_name, rv),
                    None => format!("{} = ?", var_name),
                };
                inline_values.push(InlineValueText { line: (idx + 1) as i64, column, text });
            }
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

    let Some(re) = SCALAR_VAR_RE.as_ref() else {
        return Vec::new();
    };
    let mut inline_values = Vec::new();

    for (idx, line) in lines.iter().enumerate().skip(start_idx).take(end_idx - start_idx + 1) {
        let code_mask = code_byte_mask(line);
        for cap in re.captures_iter(line) {
            if let Some(m) = cap.get(0) {
                if !code_mask[m.start()..m.end()].iter().all(|is_code| *is_code) {
                    continue;
                }
                let var_text = m.as_str();
                let column = (m.start() + 1) as i64;
                inline_values.push(InlineValueText {
                    line: (idx + 1) as i64,
                    column,
                    text: format!("{} = ?", var_text),
                });
            }
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
    fn test_quote_like_operators_are_ignored() {
        let source =
            r#"my $real = 1; my $s = q{$fake}; my $qq = qq($also_fake); my $rx = qr/$regex_fake/;"#;
        let names = extract_variable_names(source, 1, 1);
        assert!(names.contains(&"$real".to_string()));
        assert!(names.contains(&"$s".to_string()));
        assert!(names.contains(&"$qq".to_string()));
        assert!(names.contains(&"$rx".to_string()));
        assert!(!names.contains(&"$fake".to_string()));
        assert!(!names.contains(&"$also_fake".to_string()));
        assert!(!names.contains(&"$regex_fake".to_string()));
    }
}
