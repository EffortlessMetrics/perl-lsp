//! Shared constants and helpers for NodeKind coverage tests.
//!
//! All canonical lists are re-exported from `perl_parser::ast::NodeKind` — there
//! is exactly one source-of-truth (in `perl-ast`).

// This module is included in multiple test binaries, each of which uses a
// different subset of helpers.  Individual items carry `#[allow(dead_code)]`
// where they are only consumed by some binaries.

use std::collections::{BTreeMap, BTreeSet};

use perl_parser::ast::{Node, NodeKind};

/// Re-export: canonical list of **all** `kind_name()` strings.
#[allow(dead_code)]
pub const ALL_NODE_KIND_NAMES: &[&str] = NodeKind::ALL_KIND_NAMES;

/// Re-export: synthetic/recovery NodeKinds.
#[allow(dead_code)]
pub const SYNTHETIC_NODE_KIND_NAMES: &[&str] = NodeKind::RECOVERY_KIND_NAMES;

/// Recursively collect all NodeKind names present in an AST.
#[allow(dead_code)]
pub fn collect_node_kinds(node: &Node, out: &mut BTreeSet<&'static str>) {
    out.insert(node.kind.kind_name());
    node.for_each_child(|child| collect_node_kinds(child, out));
}

/// Recursively collect NodeKind names, recording which label (e.g. file name)
/// produced each kind.
#[allow(dead_code)]
pub fn collect_node_kinds_labeled(
    node: &Node,
    label: &str,
    out: &mut BTreeMap<&'static str, BTreeSet<String>>,
) {
    out.entry(node.kind.kind_name()).or_default().insert(label.to_string());
    node.for_each_child(|child| collect_node_kinds_labeled(child, label, out));
}

/// Recursively collect NodeKind names together with their parent kind name.
///
/// For the root node the parent is `None`.
#[allow(dead_code)]
pub fn collect_node_kinds_with_parents(
    node: &Node,
    parent_kind: Option<&'static str>,
    out: &mut BTreeMap<&'static str, BTreeSet<&'static str>>,
) {
    let kind = node.kind.kind_name();
    if let Some(pk) = parent_kind {
        out.entry(kind).or_default().insert(pk);
    }
    node.for_each_child(|child| collect_node_kinds_with_parents(child, Some(kind), out));
}

/// Return `true` if the AST contains a node with the given kind name.
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
pub fn corpus_required_kinds() -> BTreeSet<&'static str> {
    ALL_NODE_KIND_NAMES.iter().copied().filter(|k| !SYNTHETIC_NODE_KIND_NAMES.contains(k)).collect()
}
