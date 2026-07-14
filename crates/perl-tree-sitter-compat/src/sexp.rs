//! Render a [`TsNode`] tree as a tree-sitter S-expression.

use crate::node::TsNode;

/// Render a tree as a compact S-expression, e.g. `(program (use) (package))`.
///
/// This is the **named-node** S-expression form: it lists named node kinds
/// nested by structure, so tree-sitter test corpora that assert on named-node
/// S-expressions can run against the native parser's output. It intentionally
/// omits what the native AST does not model — tree-sitter's field labels
/// (`left:`/`right:`) and anonymous punctuation/keyword nodes — so it is not a
/// byte-for-byte match of an arbitrary tree-sitter grammar's `to_sexp()`.
#[must_use]
pub fn to_sexp(node: &TsNode) -> String {
    let mut out = String::new();
    write_sexp(node, &mut out);
    out
}

fn write_sexp(node: &TsNode, out: &mut String) {
    out.push('(');
    out.push_str(&node.kind);
    for child in &node.children {
        out.push(' ');
        write_sexp(child, out);
    }
    out.push(')');
}

/// Render a tree as an indented, multi-line S-expression for human reading.
#[must_use]
pub fn to_sexp_pretty(node: &TsNode) -> String {
    let mut out = String::new();
    write_pretty(node, 0, &mut out);
    out
}

fn write_pretty(node: &TsNode, depth: usize, out: &mut String) {
    for _ in 0..depth {
        out.push_str("  ");
    }
    out.push('(');
    out.push_str(&node.kind);
    if node.children.is_empty() {
        out.push(')');
    } else {
        for child in &node.children {
            out.push('\n');
            write_pretty(child, depth + 1, out);
        }
        out.push(')');
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{TsNode, TsPoint};

    fn node(kind: &str, children: Vec<TsNode>) -> TsNode {
        TsNode {
            kind: kind.into(),
            named: true,
            start_byte: 0,
            end_byte: 0,
            start_point: TsPoint { row: 0, column: 0 },
            end_point: TsPoint { row: 0, column: 0 },
            children,
        }
    }

    #[test]
    fn leaf_is_parenthesized_kind() {
        assert_eq!(to_sexp(&node("number", vec![])), "(number)");
    }

    #[test]
    fn nested_sexp() {
        let tree = node("program", vec![node("use", vec![]), node("package", vec![])]);
        assert_eq!(to_sexp(&tree), "(program (use) (package))");
    }

    #[test]
    fn pretty_indents_children() {
        let tree = node("program", vec![node("use", vec![])]);
        assert_eq!(to_sexp_pretty(&tree), "(program\n  (use))");
    }

    #[test]
    fn sexp_matches_parsed_output() {
        let tree = crate::convert::parse_to_tree("use strict;\n").unwrap();
        let sexp = to_sexp(&tree);
        assert!(sexp.starts_with("(program"), "root is program: {sexp}");
        assert!(sexp.contains("(use"), "contains a use node: {sexp}");
    }
}
