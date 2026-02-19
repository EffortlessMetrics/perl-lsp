//! Shared constants and helpers for NodeKind coverage tests.
//!
//! Extracted from `semantic_nodekind_coverage_tests.rs` so that both the semantic
//! inline-snippet tests and the corpus-level coverage tests share a single
//! source-of-truth list.

// Not every helper is used by every test binary that includes this module.
#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

use perl_parser::ast::Node;

/// Source-of-truth list: update ONLY when NodeKind changes.
/// (Do not include HeredocDepthLimit — that is a lexer/token budget error path, not a NodeKind.)
pub const ALL_NODE_KIND_NAMES: &[&str] = &[
    "Program",
    "ExpressionStatement",
    "VariableDeclaration",
    "VariableListDeclaration",
    "Variable",
    "VariableWithAttributes",
    "Assignment",
    "Binary",
    "Ternary",
    "Unary",
    "Diamond",
    "Ellipsis",
    "Undef",
    "Readline",
    "Glob",
    "Typeglob",
    "Number",
    "String",
    "Heredoc",
    "ArrayLiteral",
    "HashLiteral",
    "Block",
    "Eval",
    "Do",
    "Try",
    "If",
    "LabeledStatement",
    "While",
    "Tie",
    "Untie",
    "For",
    "Foreach",
    "Given",
    "When",
    "Default",
    "StatementModifier",
    "Subroutine",
    "Prototype",
    "Signature",
    "MandatoryParameter",
    "OptionalParameter",
    "SlurpyParameter",
    "NamedParameter",
    "Method",
    "Return",
    "LoopControl",
    "MethodCall",
    "FunctionCall",
    "IndirectCall",
    "Regex",
    "Match",
    "Substitution",
    "Transliteration",
    "Package",
    "Use",
    "No",
    "PhaseBlock",
    "DataSection",
    "Class",
    "Format",
    "Identifier",
    "Error",
    "MissingExpression",
    "MissingStatement",
    "MissingIdentifier",
    "MissingBlock",
    "UnknownRest",
];

/// Synthetic/recovery NodeKinds that cannot be reliably produced by parsing
/// well-formed corpus files. These are covered by a manual AST fixture instead.
pub const SYNTHETIC_NODE_KIND_NAMES: &[&str] = &[
    "Error",
    "MissingExpression",
    "MissingStatement",
    "MissingIdentifier",
    "MissingBlock",
    "UnknownRest",
];

/// Recursively collect all NodeKind names present in an AST.
pub fn collect_node_kinds(node: &Node, out: &mut HashSet<&'static str>) {
    out.insert(node.kind.kind_name());
    node.for_each_child(|child| collect_node_kinds(child, out));
}

/// Recursively collect NodeKind names, recording which label (e.g. file name)
/// produced each kind.
pub fn collect_node_kinds_labeled(
    node: &Node,
    label: &str,
    out: &mut HashMap<&'static str, HashSet<String>>,
) {
    out.entry(node.kind.kind_name()).or_default().insert(label.to_string());
    node.for_each_child(|child| collect_node_kinds_labeled(child, label, out));
}

/// Return `true` if the AST contains a node with the given kind name.
pub fn has_node_kind(ast: &Node, expected: &str) -> bool {
    if ast.kind.kind_name() == expected {
        return true;
    }
    let mut found = false;
    ast.for_each_child(|child| {
        if !found && has_node_kind(child, expected) {
            found = true;
        }
    });
    found
}

/// Find the first node with the given kind name (depth-first).
pub fn find_first_node_of_kind<'a>(node: &'a Node, expected: &str) -> Option<&'a Node> {
    if node.kind.kind_name() == expected {
        return Some(node);
    }
    let mut found: Option<&'a Node> = None;
    node.for_each_child(|child| {
        if found.is_none() {
            found = find_first_node_of_kind(child, expected);
        }
    });
    found
}

/// Return the set of NodeKind names that MUST appear in the corpus
/// (all kinds minus synthetic/recovery kinds).
pub fn corpus_required_kinds() -> HashSet<&'static str> {
    ALL_NODE_KIND_NAMES.iter().copied().filter(|k| !SYNTHETIC_NODE_KIND_NAMES.contains(k)).collect()
}
