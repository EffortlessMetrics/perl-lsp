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
///
/// `max_len` is interpreted as a character limit (not bytes) to remain UTF-8 safe.
#[must_use]
pub fn truncate_expr(expr: &str, max_len: usize) -> String {
    let expr_len = expr.chars().count();
    if expr_len <= max_len {
        return expr.to_string();
    }

    if max_len <= 3 {
        return "...".chars().take(max_len).collect();
    }

    let prefix: String = expr.chars().take(max_len - 3).collect();
    format!("{prefix}...")
}

/// Return true when `source` contains non-ASCII content.
#[must_use]
pub fn has_non_ascii_content(source: &str) -> bool {
    !source.is_ascii()
}

#[cfg(test)]
mod tests {
    use super::{
        find_import_insert_position, find_pragma_insert_position, find_statement_start,
        get_indent_at, has_non_ascii_content, truncate_expr,
    };

    #[test]
    fn finds_statement_start_after_semicolon() {
        let source = "my $x = 1;\n    my $y = 2;";
        let pos = source.find("$y").unwrap_or_default();

        assert_eq!(find_statement_start(source, pos), source.find('\n').unwrap_or_default() + 1);
    }

    #[test]
    fn finds_statement_start_from_start_when_no_delimiter_exists() {
        let source = "my $x = 1";

        assert_eq!(find_statement_start(source, usize::MAX), 0);
    }

    #[test]
    fn gets_indent_for_current_line_with_clamped_position() {
        let source = "sub demo {\n\t    my $x = 1;\n}\n";

        assert_eq!(get_indent_at(source, usize::MAX), "");
        assert_eq!(get_indent_at(source, source.find("my").unwrap_or_default()), "\t    ");
    }

    #[test]
    fn finds_pragma_insert_position_with_and_without_shebang() {
        let with_shebang = "#!/usr/bin/env perl\nuse strict;\n";
        let without_shebang = "use strict;\n";

        assert_eq!(find_pragma_insert_position(with_shebang), 20);
        assert_eq!(find_pragma_insert_position(without_shebang), 0);
    }

    #[test]
    fn finds_import_insert_position_after_existing_imports_and_comments() {
        let source = "#!/usr/bin/env perl\n# comment\nuse strict;\nrequire Foo;\nmy $x = 1;\n";
        let lines = source.lines().map(str::to_string).collect::<Vec<_>>();

        assert_eq!(
            find_import_insert_position(source, &lines),
            source.find("my $x").unwrap_or_default()
        );
    }

    #[test]
    fn stops_import_scan_at_first_non_import_code_line() {
        let source = "#!/usr/bin/env perl\nmy $x = 1;\nuse strict;\n";
        let lines = source.lines().map(str::to_string).collect::<Vec<_>>();

        assert_eq!(find_import_insert_position(source, &lines), 20);
    }

    #[test]
    fn truncates_utf8_expressions_without_splitting_codepoints() {
        assert_eq!(truncate_expr("héllo🙂world", 7), "héll...");
        assert_eq!(truncate_expr("héllo", 10), "héllo");
        assert_eq!(truncate_expr("abcdef", 2), "..");
    }

    #[test]
    fn detects_non_ascii_content() {
        assert!(!has_non_ascii_content("plain ascii"));
        assert!(has_non_ascii_content("naïve"));
    }
}
