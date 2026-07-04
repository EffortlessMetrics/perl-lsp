//! The tree-sitter-shaped node model.

use serde::{Deserialize, Serialize};

/// A tree-sitter `Point`: a 0-based `(row, column)`, column in UTF-8 code units
/// (bytes) — matching tree-sitter's own convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TsPoint {
    /// 0-based line.
    pub row: u32,
    /// 0-based column, in UTF-8 bytes from the line start.
    pub column: u32,
}

/// A tree-sitter-compatible node: a named node with a kind, byte and point
/// ranges, and named children.
///
/// The native parser's AST exposes only *named* nodes (no anonymous token
/// nodes), so every `TsNode` here is `named = true`. That is a documented
/// difference from a full tree-sitter grammar, which also surfaces anonymous
/// nodes for punctuation/keywords.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TsNode {
    /// The node type, snake_cased from the native `NodeKind` (e.g.
    /// `function_call`, `expression_statement`).
    pub kind: String,
    /// Always `true` — the native AST only carries named nodes.
    pub named: bool,
    /// Inclusive start byte offset.
    pub start_byte: u32,
    /// Exclusive end byte offset.
    pub end_byte: u32,
    /// Start point.
    pub start_point: TsPoint,
    /// End point.
    pub end_point: TsPoint,
    /// Named children, in source order.
    pub children: Vec<TsNode>,
}

impl TsNode {
    /// The number of named children.
    #[must_use]
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// The `n`th named child, if present.
    #[must_use]
    pub fn child(&self, n: usize) -> Option<&TsNode> {
        self.children.get(n)
    }

    /// Total number of nodes in this subtree (including `self`).
    #[must_use]
    pub fn descendant_count(&self) -> usize {
        1 + self.children.iter().map(TsNode::descendant_count).sum::<usize>()
    }
}

/// Convert a native PascalCase `NodeKind` name to a snake_case tree-sitter node
/// type (e.g. `ExpressionStatement` → `expression_statement`).
#[must_use]
pub fn pascal_to_snake(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (i, ch) in name.char_indices() {
        if ch.is_ascii_uppercase() {
            if i != 0 {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snake_cases_kind_names() {
        assert_eq!(pascal_to_snake("Program"), "program");
        assert_eq!(pascal_to_snake("ExpressionStatement"), "expression_statement");
        assert_eq!(pascal_to_snake("FunctionCall"), "function_call");
        assert_eq!(pascal_to_snake("Use"), "use");
    }

    #[test]
    fn descendant_count_walks_the_subtree() {
        let leaf = TsNode {
            kind: "number".into(),
            named: true,
            start_byte: 0,
            end_byte: 1,
            start_point: TsPoint { row: 0, column: 0 },
            end_point: TsPoint { row: 0, column: 1 },
            children: Vec::new(),
        };
        let root = TsNode { children: vec![leaf.clone(), leaf], ..leaf_root() };
        assert_eq!(root.descendant_count(), 3);
        assert_eq!(root.child_count(), 2);
    }

    fn leaf_root() -> TsNode {
        TsNode {
            kind: "program".into(),
            named: true,
            start_byte: 0,
            end_byte: 2,
            start_point: TsPoint { row: 0, column: 0 },
            end_point: TsPoint { row: 0, column: 2 },
            children: Vec::new(),
        }
    }
}
