//! Selection range provider for LSP.
//!
//! Provides expand/shrink selection functionality by building nested selection
//! ranges through parent AST traversal.

use perl_parser_core::ast::Node;
use rustc_hash::FxHashMap;
use serde_json::{Value, json};

/// Build nested selection range objects by climbing parent map.
pub fn selection_chain(
    ast: &Node,
    parent_map: &FxHashMap<*const Node, *const Node>,
    offset: usize,
    to_pos16: &impl Fn(usize) -> (u32, u32),
) -> Value {
    // Find leaf node at offset
    let leaf = perl_semantic_analyzer::declaration::find_node_at_offset(ast, offset).unwrap_or(ast);
    let mut node_lookup = FxHashMap::default();
    build_node_lookup(ast, &mut node_lookup);

    let mut current_ptr = leaf as *const Node;
    let mut acc = None;

    loop {
        let Some(node) = node_lookup.get(&current_ptr).copied() else {
            break;
        };

        let (sl, sc) = to_pos16(node.location.start);
        let (el, ec) = to_pos16(node.location.end);

        let here = json!({
            "range": {
                "start": {"line": sl, "character": sc},
                "end": {"line": el, "character": ec}
            },
            "parent": acc
        });

        acc = Some(here);

        // Move to parent
        if let Some(&parent_ptr) = parent_map.get(&current_ptr) {
            current_ptr = parent_ptr;
        } else {
            break;
        }
    }

    acc.unwrap_or_else(|| {
        json!({
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 0, "character": 0}
            }
        })
    })
}

fn build_node_lookup<'a>(node: &'a Node, map: &mut FxHashMap<*const Node, &'a Node>) {
    map.insert(node as *const Node, node);
    for child in perl_semantic_analyzer::declaration::get_node_children(node) {
        build_node_lookup(child, map);
    }
}

/// Helper to build parent map for an AST
pub fn build_parent_map(ast: &Node) -> FxHashMap<*const Node, *const Node> {
    let mut map = FxHashMap::default();
    build_parent_map_impl(ast, None, &mut map);
    map
}

fn build_parent_map_impl(
    node: &Node,
    parent: Option<*const Node>,
    map: &mut FxHashMap<*const Node, *const Node>,
) {
    let node_ptr = node as *const Node;

    if let Some(parent_ptr) = parent {
        map.insert(node_ptr, parent_ptr);
    }

    for child in perl_semantic_analyzer::declaration::get_node_children(node) {
        build_parent_map_impl(child, Some(node_ptr), map);
    }
}
