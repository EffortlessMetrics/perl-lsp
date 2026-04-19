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
#[must_use]
pub fn find_statement_start(source: &str, pos: usize) -> usize {
    let mut i = pos.saturating_sub(1);
    let bytes = source.as_bytes();

    while i > 0 {
        if bytes.get(i).is_some_and(|b| *b == b';' || *b == b'\n') {
            return i + 1;
        }
        i = i.saturating_sub(1);
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
            NodeKind::If {
                condition,
                then_branch,
                elsif_branches,
                else_branch,
            } => {
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
        assert_eq!(
            find_statement_start(src, pos),
            src.find('\n').unwrap_or(0) + 1
        );
    }

    #[test]
    fn declaration_position_delegates_to_statement_start() {
        let src = "print 'a';\nprint 'b';";
        let pos = src.find("'b'").unwrap_or(0);
        assert_eq!(
            find_declaration_position(src, pos),
            find_statement_start(src, pos)
        );
    }

    #[test]
    fn captures_whitespace_indent() {
        let src = "if (1) {\n    say 'x';\n}\n";
        let pos = src.find("say").unwrap_or(0);
        assert_eq!(get_indent_at(src, pos), "    ");
    }
}
