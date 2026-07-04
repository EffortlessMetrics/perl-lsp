//! Integration tests for the tree-sitter-compat adapter.
#![allow(clippy::unwrap_used)]

use perl_tree_sitter_compat::{highlights, parse_to_tree, to_sexp};

#[test]
fn sexp_round_trips_a_realistic_snippet() {
    let tree = parse_to_tree("package App;\nuse strict;\nsub run { return 42; }\n1;\n").unwrap();
    let sexp = to_sexp(&tree);
    assert!(sexp.starts_with("(program"), "root program: {sexp}");
    assert!(sexp.contains("(package"), "package node present");
    assert!(sexp.contains("(use"), "use node present");
    assert!(sexp.contains("(subroutine"), "subroutine node present");
}

#[test]
fn nodes_carry_byte_and_point_ranges() {
    let tree = parse_to_tree("package App;\n").unwrap();
    assert_eq!(tree.start_byte, 0);
    assert_eq!(tree.start_point.row, 0);
    assert_eq!(tree.start_point.column, 0);
    // Every node's end is >= its start.
    assert!(all_ranges_valid(&tree));
}

#[test]
fn highlights_cover_keywords_variables_and_literals() {
    let tree = parse_to_tree("use strict;\nmy $count = 3;\nmy $name = \"x\";\n").unwrap();
    let hl = highlights(&tree);
    assert!(hl.iter().any(|h| h.capture == "keyword"));
    assert!(hl.iter().any(|h| h.capture == "variable"));
    assert!(hl.iter().any(|h| h.capture == "number"));
    assert!(hl.iter().any(|h| h.capture == "string"));
}

#[test]
fn serializes_to_json() {
    let tree = parse_to_tree("1;\n").unwrap();
    let json = serde_json::to_string(&tree).unwrap();
    let back: perl_tree_sitter_compat::TsNode = serde_json::from_str(&json).unwrap();
    assert_eq!(tree, back);
}

#[test]
fn deterministic_across_parses() {
    let src = "package App;\nsub a { 1 }\nsub b { 2 }\n";
    assert_eq!(to_sexp(&parse_to_tree(src).unwrap()), to_sexp(&parse_to_tree(src).unwrap()));
}

fn all_ranges_valid(node: &perl_tree_sitter_compat::TsNode) -> bool {
    node.end_byte >= node.start_byte && node.children.iter().all(all_ranges_valid)
}
