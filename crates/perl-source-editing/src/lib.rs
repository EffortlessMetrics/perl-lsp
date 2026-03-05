//! Source text editing heuristics.
//!
//! This crate has a single responsibility: provide reusable source-text
//! heuristics for insertion points and lightweight display helpers.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

/// Find the start byte offset of the current statement at `pos`.
#[must_use]
pub fn find_statement_start(source: &str, pos: usize) -> usize {
    let search_pos = pos.min(source.len());
    source[..search_pos]
        .char_indices()
        .rev()
        .find_map(|(idx, ch)| ((ch == ';') || (ch == '\n')).then_some(idx + ch.len_utf8()))
        .unwrap_or(0)
}

/// Return indentation (leading spaces/tabs) for the line containing `pos`.
#[must_use]
pub fn get_indent_at(source: &str, pos: usize) -> String {
    let clamped_pos = pos.min(source.len());
    let line_start = source[..clamped_pos].rfind('\n').map_or(0, |idx| idx + 1);

    source[line_start..].chars().take_while(|ch| *ch == ' ' || *ch == '\t').collect()
}

/// Find pragma insertion position (just after shebang if present).
#[must_use]
pub fn find_pragma_insert_position(source: &str) -> usize {
    if source.starts_with("#!") { source.find('\n').map_or(source.len(), |idx| idx + 1) } else { 0 }
}

/// Find import insertion position after shebang and existing import/require lines.
#[must_use]
pub fn find_import_insert_position(source: &str, lines: &[String]) -> usize {
    let mut pos = find_pragma_insert_position(source);

    for line in lines {
        if line.starts_with("use ") || line.starts_with("require ") {
            if let Some(idx) = source.find(line) {
                pos = idx + line.len() + 1;
            }
        } else if !line.is_empty() && !line.starts_with('#') {
            break;
        }
    }

    pos
}

/// Truncate an expression for display with `...` suffix when required.
#[must_use]
pub fn truncate_expr(expr: &str, max_len: usize) -> String {
    if expr.len() <= max_len {
        return expr.to_string();
    }

    if max_len <= 3 {
        return "...".chars().take(max_len).collect();
    }

    format!("{}...", &expr[..max_len - 3])
}

/// Return true when `source` contains non-ASCII content.
#[must_use]
pub fn has_non_ascii_content(source: &str) -> bool {
    !source.is_ascii()
}
