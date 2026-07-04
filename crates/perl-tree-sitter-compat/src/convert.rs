//! Convert a native parse into a tree-sitter-compatible [`TsNode`] tree.

use perl_parser_core::{Node, Parser};
use perl_workspace_core::Utf8LineIndex;

use crate::node::{TsNode, TsPoint, pascal_to_snake};

/// A failure to produce a tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeError {
    /// The source could not be parsed as Perl.
    ParseFailed,
}

impl std::fmt::Display for TreeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseFailed => write!(f, "could not parse source as Perl"),
        }
    }
}

impl std::error::Error for TreeError {}

/// Parse Perl `source` and return its tree-sitter-compatible tree.
///
/// # Errors
/// Returns [`TreeError::ParseFailed`] when the parser cannot produce a tree.
pub fn parse_to_tree(source: &str) -> Result<TsNode, TreeError> {
    let ast = {
        let mut parser = Parser::new(source);
        parser.parse().map_err(|_| TreeError::ParseFailed)?
    };
    let line_index = Utf8LineIndex::new(source);
    Ok(to_ts_node(&ast, &line_index))
}

/// Convert one native [`Node`] (and its named children) into a [`TsNode`].
///
/// Children are emitted in the native AST's structural visit order (which
/// follows source order for each node's components); point columns are UTF-8
/// byte offsets within the row, matching tree-sitter's `Point`.
#[must_use]
pub fn to_ts_node(node: &Node, line_index: &Utf8LineIndex) -> TsNode {
    let start_byte = u32::try_from(node.location.start).unwrap_or(u32::MAX);
    let end_byte = u32::try_from(node.location.end).unwrap_or(u32::MAX);
    let (start_row, start_column) = line_index.line_col(start_byte);
    let (end_row, end_column) = line_index.line_col(end_byte);
    TsNode {
        kind: pascal_to_snake(node.kind.kind_name()),
        named: true,
        start_byte,
        end_byte,
        start_point: TsPoint { row: start_row, column: start_column },
        end_point: TsPoint { row: end_row, column: end_column },
        children: node.children().iter().map(|child| to_ts_node(child, line_index)).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_package_to_a_tree() {
        let tree = parse_to_tree("package App;\nsub run { 1 }\n1;\n").unwrap();
        assert_eq!(tree.kind, "program");
        assert!(tree.named);
        // The program spans the whole file.
        assert_eq!(tree.start_byte, 0);
        assert!(tree.descendant_count() > 1, "tree has nested nodes");
    }

    #[test]
    fn points_are_zero_based_and_track_lines() {
        let tree = parse_to_tree("package App;\nsub run { 1 }\n").unwrap();
        assert_eq!(tree.start_point, TsPoint { row: 0, column: 0 });
        // Find a node that starts on line 1 (the sub).
        assert!(
            has_node_starting_on_row(&tree, 1),
            "some node starts on row 1 (the sub on the second line)"
        );
    }

    #[test]
    fn parse_failure_is_an_error_not_a_panic() {
        let bad = "{".repeat(5000);
        // Either an error or a recovered tree — never a panic.
        let _ = parse_to_tree(&bad);
    }

    #[test]
    fn tree_error_displays_readably() {
        assert_eq!(TreeError::ParseFailed.to_string(), "could not parse source as Perl");
    }

    fn has_node_starting_on_row(node: &TsNode, row: u32) -> bool {
        node.start_point.row == row
            || node.children.iter().any(|c| has_node_starting_on_row(c, row))
    }
}
