//! Same-file find-all-references support for Perl LSP navigation.
//!
//! This microcrate focuses on one responsibility: locating references to a
//! variable or subroutine within a single parsed Perl source file.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use perl_parser_core::ast::{Node, NodeKind};
use perl_qualified_name::split_qualified_name;

/// Return `(start_offset, end_offset)` pairs for same-file references.
pub fn find_references_single_file(ast: &Node, offset: usize) -> Option<Vec<(usize, usize)>> {
    let needle = find_node_at_offset(ast, offset)?;

    let (want_kind, want_pkg, want_name, want_sigil) = match &needle.kind {
        NodeKind::Variable { sigil, name } => {
            ("var", "main".to_string(), name.clone(), sigil.chars().next())
        }
        NodeKind::FunctionCall { name, .. } => {
            let (pkg, bare) = split_qualified_name(name);
            ("sub", pkg.unwrap_or("main").to_string(), bare.to_string(), None)
        }
        NodeKind::Subroutine { name: Some(name), .. } => {
            let (pkg, bare) = split_qualified_name(name);
            ("sub", pkg.unwrap_or("main").to_string(), bare.to_string(), None)
        }
        _ => return None,
    };

    let mut matches = Vec::new();
    collect_references(ast, &mut matches, want_kind, &want_pkg, &want_name, want_sigil);
    Some(matches)
}

fn collect_references(
    node: &Node,
    matches: &mut Vec<(usize, usize)>,
    want_kind: &str,
    want_pkg: &str,
    want_name: &str,
    want_sigil: Option<char>,
) {
    let location = &node.location;
    match &node.kind {
        NodeKind::Variable { sigil, name } if want_kind == "var" => {
            if sigil.chars().next() == want_sigil && name == want_name {
                matches.push((location.start, location.end));
            }
        }
        NodeKind::FunctionCall { name, .. } if want_kind == "sub" => {
            let (pkg, bare) = split_qualified_name(name);
            if bare == want_name && pkg.unwrap_or("main") == want_pkg {
                matches.push((location.start, location.end));
            }
        }
        NodeKind::Subroutine { name: Some(name), .. } if want_kind == "sub" => {
            if name == want_name {
                matches.push((location.start, location.end));
            }
        }
        _ => {}
    }

    for child in node.children() {
        collect_references(child, matches, want_kind, want_pkg, want_name, want_sigil);
    }
}

fn find_node_at_offset(node: &Node, offset: usize) -> Option<&Node> {
    if offset < node.location.start || offset > node.location.end {
        return None;
    }

    for child in node.children() {
        if let Some(found) = find_node_at_offset(child, offset) {
            return Some(found);
        }
    }

    Some(node)
}
