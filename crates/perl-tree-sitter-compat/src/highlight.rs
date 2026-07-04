//! Syntax-highlight captures over a [`TsNode`] tree.
//!
//! Maps node kinds to tree-sitter highlight capture names (`keyword`,
//! `function`, `variable`, `string`, …). This is a **node-granular** first
//! slice: a highlight spans a whole mapped node, not individual tokens, so a
//! keyword node covers its statement span. It provides a usable capture map for
//! the native tree; a token-precise highlighter is a later refinement.

use serde::{Deserialize, Serialize};

use crate::node::TsNode;

/// One highlight: a byte range and the capture that applies to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Highlight {
    /// Inclusive start byte.
    pub start_byte: u32,
    /// Exclusive end byte.
    pub end_byte: u32,
    /// The tree-sitter capture name (without the leading `@`).
    pub capture: &'static str,
}

/// The capture for a node kind, if it is highlighted.
#[must_use]
pub fn capture_for(kind: &str) -> Option<&'static str> {
    let capture = match kind {
        "use" | "no" | "package" | "if" | "unless" | "while" | "until" | "for" | "foreach"
        | "return" | "eval" | "do" | "defer" | "try" | "phase_block" | "last" | "next" | "redo" => {
            "keyword"
        }
        "subroutine" => "function",
        "function_call" | "method_call" | "indirect_call" => "function.call",
        "variable" | "variable_with_attributes" | "typeglob" => "variable",
        "string" | "heredoc" => "string",
        "regex" | "substitution" => "string.regex",
        "number" => "number",
        "undef" => "constant.builtin",
        _ => return None,
    };
    Some(capture)
}

/// Collect the highlights for a tree, in source (pre-order) order.
#[must_use]
pub fn highlights(tree: &TsNode) -> Vec<Highlight> {
    let mut out = Vec::new();
    collect(tree, &mut out);
    out
}

fn collect(node: &TsNode, out: &mut Vec<Highlight>) {
    if let Some(capture) = capture_for(&node.kind) {
        out.push(Highlight { start_byte: node.start_byte, end_byte: node.end_byte, capture });
    }
    for child in &node.children {
        collect(child, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_kinds() {
        assert_eq!(capture_for("use"), Some("keyword"));
        assert_eq!(capture_for("subroutine"), Some("function"));
        assert_eq!(capture_for("variable"), Some("variable"));
        assert_eq!(capture_for("string"), Some("string"));
        assert_eq!(capture_for("number"), Some("number"));
        assert_eq!(capture_for("program"), None);
        assert_eq!(capture_for("block"), None);
    }

    #[test]
    fn highlights_a_parsed_tree() {
        let tree = crate::convert::parse_to_tree("use strict;\nmy $x = 42;\n").unwrap();
        let hl = highlights(&tree);
        assert!(hl.iter().any(|h| h.capture == "keyword"), "use → keyword; got {hl:?}");
        assert!(hl.iter().any(|h| h.capture == "number"), "42 → number; got {hl:?}");
        // Highlights are ordered by source position.
        assert!(hl.windows(2).all(|w| w[0].start_byte <= w[1].start_byte), "ordered");
    }
}
