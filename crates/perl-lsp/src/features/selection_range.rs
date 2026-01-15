// crates/perl-parser/src/selection_range.rs
use perl_parser::ast::Node;
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
    let leaf = crate::declaration::find_node_at_offset(ast, offset).unwrap_or(ast);

    let mut current_ptr = leaf as *const Node;
    let mut acc = None;

    loop {
        // We know these pointers are valid because they come from our AST
        // Using allow(unsafe) here is valid since we control the AST lifetime
        #[allow(unsafe_code)]
        let node = unsafe { &*current_ptr };

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

    for child in crate::declaration::get_node_children(node) {
        build_parent_map_impl(child, Some(node_ptr), map);
    }
}
