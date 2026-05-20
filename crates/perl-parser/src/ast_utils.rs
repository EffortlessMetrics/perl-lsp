//! AST utilities for Perl LSP microcrates.
//!
//! This crate has a narrow responsibility: provide AST and source-text helpers
//! used by higher-level LSP features (for example, code actions).

#![deny(unsafe_code)]
#![warn(missing_docs)]

use perl_ast::{Node, NodeKind};

/// Find the best position to insert a declaration.
#[must_use]
pub fn find_declaration_position(source: &str, error_pos: usize) -> usize {
    find_statement_start(source, error_pos)
}

/// Find the start of the current statement.
///
/// Scans backwards from `pos` for the nearest `;` or `\n` and returns the
/// byte index immediately after it, or 0 if no boundary is found.
#[must_use]
pub fn find_statement_start(source: &str, pos: usize) -> usize {
    let bytes = source.as_bytes();
    let start = pos.min(bytes.len());

    for i in (0..start).rev() {
        if bytes[i] == b';' || bytes[i] == b'\n' {
            return i + 1;
        }
    }

    0
}

/// Find a good position to insert a function.
///
/// Current policy inserts at end-of-file.
#[must_use]
pub fn find_function_insert_position(source: &str) -> usize {
    source.len()
}

/// Find the most specific node covering the provided byte range.
#[allow(clippy::only_used_in_recursion)]
#[must_use]
pub fn find_node_at_range(node: &Node, range: (usize, usize)) -> Option<&Node> {
    if node.location.start <= range.0 && node.location.end >= range.1 {
        match &node.kind {
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                for stmt in statements {
                    if let Some(result) = find_node_at_range(stmt, range) {
                        return Some(result);
                    }
                }
            }
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                if let Some(result) = find_node_at_range(condition, range) {
                    return Some(result);
                }
                if let Some(result) = find_node_at_range(then_branch, range) {
                    return Some(result);
                }
                for (cond, branch) in elsif_branches {
                    if let Some(result) = find_node_at_range(cond, range) {
                        return Some(result);
                    }
                    if let Some(result) = find_node_at_range(branch, range) {
                        return Some(result);
                    }
                }
                if let Some(branch) = else_branch
                    && let Some(result) = find_node_at_range(branch, range)
                {
                    return Some(result);
                }
            }
            NodeKind::Binary { left, right, .. } => {
                if let Some(result) = find_node_at_range(left, range) {
                    return Some(result);
                }
                if let Some(result) = find_node_at_range(right, range) {
                    return Some(result);
                }
            }
            _ => {}
        }
        return Some(node);
    }

    None
}

/// Get indentation at a position.
#[must_use]
pub fn get_indent_at(source: &str, pos: usize) -> String {
    let line_start = source[..pos].rfind('\n').map_or(0, |p| p + 1);
    let line = &source[line_start..];

    let mut indent = String::new();
    for ch in line.chars() {
        if ch == ' ' || ch == '\t' {
            indent.push(ch);
        } else {
            break;
        }
    }
    indent
}

#[cfg(test)]
mod tests {
    use super::{find_declaration_position, find_statement_start, get_indent_at};

    #[test]
    fn finds_statement_start_after_semicolon() {
        let src = "my $x = 1;\nmy $y = 2;";
        let pos = src.find("$y").unwrap_or(0);
        assert_eq!(find_statement_start(src, pos), src.find('\n').unwrap_or(0) + 1);
    }

    #[test]
    fn finds_statement_start_when_terminator_is_at_index_zero() {
        // Regression: the backwards scan must inspect byte 0. Previously the
        // loop guard `while i > 0` skipped index 0, so a terminator at the
        // very start of the source was never recognised.
        assert_eq!(find_statement_start(";x", 1), 1);
        assert_eq!(find_statement_start("\nfoo", 1), 1);
    }

    #[test]
    fn returns_zero_when_no_terminator_precedes_pos() {
        assert_eq!(find_statement_start("abc", 3), 0);
        assert_eq!(find_statement_start("", 0), 0);
        assert_eq!(find_statement_start("abc", 0), 0);
    }

    #[test]
    fn handles_pos_beyond_source_len() {
        let src = "foo;bar";
        assert_eq!(find_statement_start(src, 100), 4);
    }

    #[test]
    fn declaration_position_delegates_to_statement_start() {
        let src = "print 'a';\nprint 'b';";
        let pos = src.find("'b'").unwrap_or(0);
        assert_eq!(find_declaration_position(src, pos), find_statement_start(src, pos));
    }

    #[test]
    fn captures_whitespace_indent() {
        let src = "if (1) {\n    say 'x';\n}\n";
        let pos = src.find("say").unwrap_or(0);
        assert_eq!(get_indent_at(src, pos), "    ");
    }
}
