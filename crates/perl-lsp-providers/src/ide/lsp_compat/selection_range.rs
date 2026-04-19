//! Selection range provider for LSP.
//!
//! Provides expand/shrink selection functionality by building nested selection
//! ranges through parent AST traversal.  The chain walks from the leaf node at
//! the cursor offset up to the file root, inserting intermediate "synthetic"
//! ranges for constructs where the AST hierarchy would otherwise create a gap
//! (e.g. subroutine name -> signature -> body -> full sub).

use perl_parser_core::ast::{Node, NodeKind};
use rustc_hash::FxHashMap;
use serde_json::{Value, json};

// ---------------------------------------------------------------------------
// JSON helper
// ---------------------------------------------------------------------------

fn range_json(
    start: usize,
    end: usize,
    parent: Option<Value>,
    to_pos16: &impl Fn(usize) -> (u32, u32),
) -> Value {
    let (sl, sc) = to_pos16(start);
    let (el, ec) = to_pos16(end);
    json!({
        "range": {
            "start": {"line": sl, "character": sc},
            "end":   {"line": el, "character": ec}
        },
        "parent": parent
    })
}

// ---------------------------------------------------------------------------
// Synthetic intermediate ranges
// ---------------------------------------------------------------------------

/// Collect synthetic sub-node ranges that should be inserted *before* the node
/// itself in the chain (innermost first).  Each entry is a `(start, end)` pair.
fn synthetic_ranges(node: &Node) -> Vec<(usize, usize)> {
    let mut extras = Vec::new();
    match &node.kind {
        // Subroutine: emit name_span, then signature span, then body span
        // so the expansion goes: name -> signature -> body -> full sub
        NodeKind::Subroutine {
            name_span,
            signature,
            body,
            ..
        } => {
            if let Some(span) = name_span {
                extras.push((span.start, span.end));
            }
            if let Some(sig) = signature {
                extras.push((sig.location.start, sig.location.end));
            }
            extras.push((body.location.start, body.location.end));
        }
        // Method: name is a String, but we don't have a span for it — use
        // signature and body.
        NodeKind::Method {
            signature, body, ..
        } => {
            if let Some(sig) = signature {
                extras.push((sig.location.start, sig.location.end));
            }
            extras.push((body.location.start, body.location.end));
        }
        _ => {}
    }
    extras
}

// ---------------------------------------------------------------------------
// Core API
// ---------------------------------------------------------------------------

/// Build nested selection range objects by climbing parent map.
///
/// The chain walks from the deepest AST node that spans the cursor offset up
/// through every ancestor, deduplicating ranges with identical spans and
/// injecting synthetic intermediate ranges for subroutines (name, signature,
/// body).  The outermost range is always the file-level range.
pub fn selection_chain(
    ast: &Node,
    parent_map: &FxHashMap<*const Node, *const Node>,
    offset: usize,
    to_pos16: &impl Fn(usize) -> (u32, u32),
) -> Value {
    // Find leaf node at offset using the comprehensive traversal
    let leaf = find_deepest_node_at_offset(ast, offset).unwrap_or(ast);
    let mut node_lookup = FxHashMap::default();
    build_node_lookup(ast, &mut node_lookup);

    // Collect the path from leaf -> root as (start, end) pairs.
    let mut ranges: Vec<(usize, usize)> = Vec::new();

    let mut current_ptr = leaf as *const Node;
    let mut first = true;
    loop {
        let Some(node) = node_lookup.get(&current_ptr).copied() else {
            break;
        };

        // For the *first* (deepest) node, inject any synthetic sub-ranges
        // that contain the offset.  This handles the case where the cursor
        // sits on a subroutine name or signature: those areas are not
        // separate child nodes, so they must be injected as inner ranges.
        if first {
            let synthetics = synthetic_ranges(node);
            // Filter to synthetics that contain the offset, smallest first
            let mut applicable: Vec<(usize, usize)> = synthetics
                .into_iter()
                .filter(|&(s, e)| offset >= s && offset <= e)
                .collect();
            applicable.sort_by_key(|&(s, e)| e - s);
            for synth in applicable {
                ranges.push(synth);
            }
            first = false;
        }

        let span = (node.location.start, node.location.end);
        ranges.push(span);

        // Also inject synthetics from the parent node (for when the cursor
        // is deeper than the sub, e.g. inside the body).
        if let Some(&parent_ptr) = parent_map.get(&current_ptr) {
            if let Some(parent_node) = node_lookup.get(&parent_ptr).copied() {
                let synthetics = synthetic_ranges(parent_node);
                let mut applicable: Vec<(usize, usize)> = synthetics
                    .into_iter()
                    .filter(|&(s, e)| offset >= s && offset <= e)
                    .collect();
                applicable.sort_by_key(|&(s, e)| e - s);
                for synth in applicable {
                    ranges.push(synth);
                }
            }
        }

        // Move to parent
        if let Some(&parent_ptr) = parent_map.get(&current_ptr) {
            current_ptr = parent_ptr;
        } else {
            break;
        }
    }

    // Ensure file-level range is the outermost
    let file_span = (ast.location.start, ast.location.end);
    if ranges.last().is_none_or(|&last| last != file_span) {
        ranges.push(file_span);
    }

    // Sort all ranges by size (smallest first) to build a properly nested
    // chain, then deduplicate.
    ranges.sort_by_key(|&(s, e)| e.saturating_sub(s));
    ranges.dedup();

    // Keep only ranges that strictly grow (each must encompass the previous)
    let mut deduped: Vec<(usize, usize)> = Vec::new();
    for &span in &ranges {
        if let Some(&prev) = deduped.last() {
            // Must be strictly larger (encompass previous)
            if span.0 <= prev.0 && span.1 >= prev.1 && (span.0 < prev.0 || span.1 > prev.1) {
                deduped.push(span);
            }
        } else {
            deduped.push(span);
        }
    }

    if deduped.is_empty() {
        deduped.push(file_span);
    }

    // Build JSON chain from outermost (last) to innermost (first)
    let mut acc: Option<Value> = None;
    for &(start, end) in deduped.iter().rev() {
        acc = Some(range_json(start, end, acc, to_pos16));
    }

    acc.unwrap_or_else(|| range_json(0, 0, None, to_pos16))
}

// ---------------------------------------------------------------------------
// Tree traversal helpers (using Node::children() for full coverage)
// ---------------------------------------------------------------------------

/// Find the deepest AST node whose span contains `offset`.
///
/// Uses `Node::children()` which covers all `NodeKind` variants, unlike the
/// legacy `get_node_children` helper that only handles a subset.
///
/// **Note**: Some parser node locations do not encompass their initializers
/// (e.g. `VariableDeclaration` spans only `my $var` but its `String` child
/// can extend beyond).  We therefore always recurse into children even when
/// the parent span does not strictly contain the offset.
fn find_deepest_node_at_offset(node: &Node, offset: usize) -> Option<&Node> {
    // Always check children first -- a child might contain the offset even
    // when the parent's recorded span does not fully encompass it.
    for child in node.children() {
        if let Some(found) = find_deepest_node_at_offset(child, offset) {
            return Some(found);
        }
    }
    // Only claim *this* node if the offset is within its own span.
    if offset >= node.location.start && offset <= node.location.end {
        return Some(node);
    }
    None
}

fn build_node_lookup<'a>(node: &'a Node, map: &mut FxHashMap<*const Node, &'a Node>) {
    map.insert(node as *const Node, node);
    for child in node.children() {
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

    for child in node.children() {
        build_parent_map_impl(child, Some(node_ptr), map);
    }
}
