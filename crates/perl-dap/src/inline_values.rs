//! Inline value extraction for DAP inlineValues requests.
//!
//! This module provides a lightweight, regex-based implementation for inline
//! values. It mirrors the LSP inlineValue provider by returning text hints for
//! scalar variables within a specified line range.

use once_cell::sync::Lazy;
use regex::Regex;

use crate::protocol::InlineValueText;

/// Regex for matching Perl scalar variables.
/// Stored as Option to avoid panics; if compilation fails, inline values are skipped.
static SCALAR_VAR_RE: Lazy<Option<Regex>> =
    Lazy::new(|| Regex::new(r"\$[A-Za-z_][A-Za-z0-9_]*").or_else(|_| Regex::new(r"\$\w+")).ok());

/// Collect inline values for scalar variables within a line range.
///
/// Lines and columns are 1-based to match the DAP defaults.
pub fn collect_inline_values(source: &str, start_line: i64, end_line: i64) -> Vec<InlineValueText> {
    let lines: Vec<&str> = source.lines().collect();
    if lines.is_empty() {
        return Vec::new();
    }

    let start_idx = start_line.saturating_sub(1) as usize;
    let mut end_idx = end_line.saturating_sub(1) as usize;
    if end_idx >= lines.len() {
        end_idx = lines.len() - 1;
    }

    let Some(re) = SCALAR_VAR_RE.as_ref() else {
        return Vec::new(); // No regex available - graceful degradation
    };
    let mut inline_values = Vec::new();

    for (idx, line) in lines.iter().enumerate().skip(start_idx).take(end_idx - start_idx + 1) {
        for cap in re.captures_iter(line) {
            if let Some(m) = cap.get(0) {
                let var_text = m.as_str();
                let column = (m.start() + 1) as i64; // 1-based
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
    fn test_collect_inline_values() {
        let source = "my $x = 1;\nmy $y = $x + 2;";
        let values = collect_inline_values(source, 1, 2);
        assert!(values.iter().any(|v| v.text.contains("$x")));
        assert!(values.iter().any(|v| v.text.contains("$y")));
    }
}
