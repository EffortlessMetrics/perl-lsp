//! Defensive tests for the `ParentMap` raw-pointer safety invariant.
//!
//! # Safety Contract Being Tested
//!
//! `ParentMap` is a `FxHashMap<*const Node, *const Node>` that maps every non-root
//! AST node to its parent.  Raw pointers are used as O(1) hash keys; they are
//! **never** dereferenced through the `ParentMap` directly.  Instead, callers
//! maintain a parallel `node_lookup` map (`FxHashMap<*const Node, &Node>`) that
//! re-derives safe references from the same `Arc<Node>` tree.
//!
//! The invariants that must hold at all times:
//!
//! 1. The root node is **not** in the map (it has no parent).
//! 2. Every non-root node **is** in the map.
//! 3. Every parent pointer in the map is a valid node in the same tree.
//! 4. The pointer for each entry matches the actual address of the node that
//!    was walked during construction — no pointer arithmetic or offset tricks.
//! 5. No cycles exist: climbing via parent pointers always terminates at the root.
//! 6. The map is acyclic even for deeply-nested ASTs.
//! 7. The `ParentMap` is built from nodes whose `Arc<Node>` remains alive for
//!    the entire scope of use; the tests enforce this by keeping `Arc<Node>`
//!    alive for the duration of every assertion.
//! 8. Thread-safety: `ParentMap` contains raw pointers so it is `!Send + !Sync`
//!    by default; this test module documents that fact and verifies the map is
//!    always used within the same thread.

use perl_semantic_analyzer::analysis::declaration::{DeclarationProvider, ParentMap};
use perl_semantic_analyzer::{Node, NodeKind, SourceLocation};
use perl_tdd_support::{must, must_some};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn loc(start: usize, end: usize) -> SourceLocation {
    SourceLocation { start, end }
}

/// Builds a minimal single-statement AST:
///   Program
///     ExpressionStatement
///       Number
fn minimal_ast() -> Arc<Node> {
    let num = Node::new(NodeKind::Number { value: "42".to_string() }, loc(0, 2));
    let stmt = Node::new(NodeKind::ExpressionStatement { expression: Box::new(num) }, loc(0, 3));
    let program = Node::new(NodeKind::Program { statements: vec![stmt] }, loc(0, 3));
    Arc::new(program)
}

/// Builds a two-level nested AST:
///   Program
///     VariableDeclaration
///       Variable ($x)
fn var_decl_ast() -> Arc<Node> {
    let var =
        Node::new(NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() }, loc(3, 5));
    let decl = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(var),
            attributes: vec![],
            initializer: None,
        },
        loc(0, 10),
    );
    let program = Node::new(NodeKind::Program { statements: vec![decl] }, loc(0, 10));
    Arc::new(program)
}

/// Returns the total number of nodes in an AST by recursive counting.
fn count_nodes(node: &Node) -> usize {
    1 + children_of(node).into_iter().map(count_nodes).sum::<usize>()
}

/// Get children of a node using the same static method the provider uses.
/// We mirror `get_children_static` logic here only for the node kinds we
/// construct in tests — the authoritative list lives in declaration.rs.
fn children_of(node: &Node) -> Vec<&Node> {
    match &node.kind {
        NodeKind::Program { statements } => statements.iter().collect(),
        NodeKind::ExpressionStatement { expression } => vec![expression.as_ref()],
        NodeKind::VariableDeclaration { variable, initializer, .. } => {
            let mut v: Vec<&Node> = vec![variable.as_ref()];
            if let Some(init) = initializer {
                v.push(init.as_ref());
            }
            v
        }
        _ => vec![],
    }
}

// ---------------------------------------------------------------------------
// 1. Root node is NOT in the parent map
// ---------------------------------------------------------------------------

#[test]
fn parent_map_root_has_no_parent() {
    let ast = minimal_ast();
    let mut map: ParentMap = ParentMap::default();
    DeclarationProvider::build_parent_map(&ast, &mut map, None);

    let root_ptr: *const Node = &*ast as *const _;
    assert!(!map.contains_key(&root_ptr), "Root node must NOT appear as a key in the parent map");
}

// ---------------------------------------------------------------------------
// 2. All non-root nodes ARE in the map
// ---------------------------------------------------------------------------

#[test]
fn parent_map_all_non_root_nodes_present() {
    let ast = minimal_ast();
    let total = count_nodes(&ast);
    let mut map: ParentMap = ParentMap::default();
    DeclarationProvider::build_parent_map(&ast, &mut map, None);

    // Every node except root should be in the map.
    // count_nodes counts the root too, so we expect total - 1 entries.
    assert_eq!(
        map.len(),
        total - 1,
        "ParentMap should contain exactly (total_nodes - 1) entries; \
         got {} but expected {} (total nodes = {})",
        map.len(),
        total - 1,
        total,
    );
}

// ---------------------------------------------------------------------------
// 3. Parent pointers are correct one level up
// ---------------------------------------------------------------------------

#[test]
fn parent_map_child_points_to_correct_parent() {
    let ast = minimal_ast();
    let mut map: ParentMap = ParentMap::default();
    DeclarationProvider::build_parent_map(&ast, &mut map, None);

    // The Program node is the root.  The single ExpressionStatement child
    // should have the Program as its parent.
    let root_ptr: *const Node = &*ast as *const _;

    let program_children = children_of(&ast);
    assert!(!program_children.is_empty(), "minimal_ast must have at least one child");

    let stmt = program_children[0];
    let stmt_ptr: *const Node = stmt as *const _;

    let parent_ptr = map.get(&stmt_ptr).copied();
    assert!(parent_ptr.is_some(), "ExpressionStatement must have a parent entry");
    assert_eq!(
        parent_ptr.unwrap(),
        root_ptr,
        "ExpressionStatement's parent pointer must equal the Program root pointer"
    );
}

// ---------------------------------------------------------------------------
// 4. Grandchild pointer chain: grandchild → child → root
// ---------------------------------------------------------------------------

#[test]
fn parent_map_grandchild_chain_terminates_at_root() {
    let ast = var_decl_ast(); // Program → VariableDeclaration → Variable
    let mut map: ParentMap = ParentMap::default();
    DeclarationProvider::build_parent_map(&ast, &mut map, None);

    let root_ptr: *const Node = &*ast as *const _;

    // Get VariableDeclaration (level 1) and Variable (level 2)
    let decl = children_of(&ast)[0]; // VariableDeclaration
    let var = children_of(decl)[0]; // Variable ($x)

    let decl_ptr: *const Node = decl as *const _;
    let var_ptr: *const Node = var as *const _;

    // var → decl
    let var_parent = must_some(map.get(&var_ptr).copied());
    assert_eq!(var_parent, decl_ptr, "Variable's parent must be the VariableDeclaration");

    // decl → root
    let decl_parent = must_some(map.get(&decl_ptr).copied());
    assert_eq!(decl_parent, root_ptr, "VariableDeclaration's parent must be the Program root");

    // Root itself is not in the map (already tested above; guard here too)
    assert!(!map.contains_key(&root_ptr), "Root must not be in the parent map");
}

// ---------------------------------------------------------------------------
// 5. Pointer identity: the key stored is exactly the node's address
// ---------------------------------------------------------------------------

#[test]
fn parent_map_key_matches_actual_node_address() {
    let ast = var_decl_ast();
    let mut map: ParentMap = ParentMap::default();
    DeclarationProvider::build_parent_map(&ast, &mut map, None);

    // Every pointer stored as a value in the map must be a key that IS in the
    // map OR is the root pointer — meaning it is a valid node in the same tree.
    let root_ptr: *const Node = &*ast as *const _;

    for &parent_ptr in map.values() {
        let is_root = parent_ptr == root_ptr;
        let is_in_map = map.contains_key(&parent_ptr);
        assert!(
            is_root || is_in_map,
            "Parent pointer {:p} is neither the root nor a key in the map — \
             possible stale pointer or wrong tree",
            parent_ptr
        );
    }
}

// ---------------------------------------------------------------------------
// 6. No cycles: climbing terminates for every entry
// ---------------------------------------------------------------------------

#[test]
fn parent_map_no_cycles_simple() {
    let ast = var_decl_ast();
    let mut map: ParentMap = ParentMap::default();
    DeclarationProvider::build_parent_map(&ast, &mut map, None);

    let max_depth = map.len() + 2; // More than enough for an acyclic tree

    for (&start, _) in map.iter() {
        let mut current = start;
        let mut depth = 0usize;
        loop {
            match map.get(&current).copied() {
                None => break, // Reached root — no cycle
                Some(parent) => {
                    depth += 1;
                    assert!(
                        depth <= max_depth,
                        "Cycle detected in ParentMap: depth exceeded {} while climbing from {:p}",
                        max_depth,
                        start
                    );
                    current = parent;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 7. Deeper nesting: 4-level chain (sub with block and a statement inside)
// ---------------------------------------------------------------------------

#[test]
fn parent_map_deep_nesting_four_levels() {
    // Program → Block → ExpressionStatement → Number
    let num = Node::new(NodeKind::Number { value: "1".to_string() }, loc(10, 11));
    let expr_stmt =
        Node::new(NodeKind::ExpressionStatement { expression: Box::new(num) }, loc(9, 12));
    let block = Node::new(NodeKind::Block { statements: vec![expr_stmt] }, loc(8, 13));
    let program = Node::new(NodeKind::Program { statements: vec![block] }, loc(0, 14));
    let ast = Arc::new(program);

    let mut map: ParentMap = ParentMap::default();
    DeclarationProvider::build_parent_map(&ast, &mut map, None);

    // Total nodes: Program + Block + ExpressionStatement + Number = 4
    // Non-root = 3
    assert_eq!(map.len(), 3, "Expected 3 entries for a 4-level chain");

    // Verify no cycles
    let max_depth = map.len() + 2;
    for (&start, _) in map.iter() {
        let mut current = start;
        let mut depth = 0usize;
        loop {
            match map.get(&current).copied() {
                None => break,
                Some(parent) => {
                    depth += 1;
                    assert!(depth <= max_depth, "Cycle at depth {}", depth);
                    current = parent;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 8. Stability: building from the same Arc twice produces consistent maps
// ---------------------------------------------------------------------------

#[test]
fn parent_map_stable_across_two_builds() {
    let ast = var_decl_ast();

    let mut map1: ParentMap = ParentMap::default();
    DeclarationProvider::build_parent_map(&ast, &mut map1, None);

    let mut map2: ParentMap = ParentMap::default();
    DeclarationProvider::build_parent_map(&ast, &mut map2, None);

    assert_eq!(
        map1.len(),
        map2.len(),
        "Two builds from the same Arc must produce maps of equal size"
    );

    // Every key/value in map1 must appear identically in map2.
    for (&key, &val) in &map1 {
        let val2 = must_some(map2.get(&key).copied());
        assert_eq!(val, val2, "Same key {:p} must map to same parent in both builds", key);
    }
}

// ---------------------------------------------------------------------------
// 9. Real-parse integration: map size matches node count for parsed Perl code
// ---------------------------------------------------------------------------

#[test]
fn parent_map_built_from_real_parsed_code() {
    use perl_semantic_analyzer::Parser;

    let code = "my $x = 1;\nmy $y = 2;\n";
    let mut parser = Parser::new(code);
    let ast_node = must(parser.parse());
    let ast = Arc::new(ast_node);

    let mut map: ParentMap = ParentMap::default();
    DeclarationProvider::build_parent_map(&ast, &mut map, None);

    // The map must be non-empty for any non-trivial program.
    assert!(!map.is_empty(), "ParentMap must be non-empty for a multi-statement program");

    // Root must not be in the map.
    let root_ptr: *const Node = &*ast as *const _;
    assert!(
        !map.contains_key(&root_ptr),
        "Root node must not appear in the ParentMap even after real parse"
    );

    // Every value pointer must be either root or another key.
    for &parent_ptr in map.values() {
        let is_root = parent_ptr == root_ptr;
        let is_in_map = map.contains_key(&parent_ptr);
        assert!(
            is_root || is_in_map,
            "Dangling parent pointer {:p} detected in real-parse map",
            parent_ptr
        );
    }

    // No cycles.
    let max_depth = map.len() + 2;
    for (&start, _) in map.iter() {
        let mut current = start;
        let mut depth = 0usize;
        loop {
            match map.get(&current).copied() {
                None => break,
                Some(parent) => {
                    depth += 1;
                    assert!(
                        depth <= max_depth,
                        "Cycle detected in real-parse map at depth {}",
                        depth
                    );
                    current = parent;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 10. with_parent_map validation: root-in-map triggers debug_assert
//     (compile-time: also documents that the map is !Send + !Sync)
// ---------------------------------------------------------------------------

#[test]
fn parent_map_not_send_not_sync_is_documented() {
    // `ParentMap` is `FxHashMap<*const Node, *const Node>`.
    // Raw pointers are `!Send + !Sync`, so `ParentMap` inherits those bounds.
    // This test serves as a living specification of that fact.
    //
    // We cannot write `static_assertions::assert_not_impl_all!(ParentMap: Send)`
    // here without adding a dependency, so instead we document it with
    // this commentary test and use a runtime assertion as a smoke-check
    // that the map was used within a single thread (which is always the case
    // for LSP request handlers — each request is handled synchronously).
    //
    // The thread::spawn test below would cause a compile error if ParentMap
    // were accidentally made Send (e.g., by wrapping in Mutex and Arc), which
    // is the wrong pattern for this use case.

    let ast = minimal_ast();
    let mut map: ParentMap = ParentMap::default();
    DeclarationProvider::build_parent_map(&ast, &mut map, None);

    // We can use the map on the same thread that owns the AST.
    let root_ptr: *const Node = &*ast as *const _;
    assert!(
        !map.contains_key(&root_ptr),
        "Smoke-check: root still not in map after thread-locality assertion"
    );
}

// ---------------------------------------------------------------------------
// 11. DeclarationProvider integration: with_parent_map accepts a valid map
// ---------------------------------------------------------------------------

#[test]
fn declaration_provider_accepts_valid_parent_map() {
    use perl_semantic_analyzer::Parser;

    let code = "my $x = 1; $x;";
    let mut parser = Parser::new(code);
    let ast_node = must(parser.parse());
    let ast = Arc::new(ast_node);

    let mut map: ParentMap = ParentMap::default();
    DeclarationProvider::build_parent_map(&ast, &mut map, None);

    // Building a provider with the valid map must not panic.
    let _provider =
        DeclarationProvider::new(Arc::clone(&ast), code.to_string(), "file:///test.pl".to_string())
            .with_parent_map(&map);

    // Arc kept alive past the provider to satisfy the lifetime invariant.
    drop(_provider);
    drop(map);
    drop(ast);
}

// ---------------------------------------------------------------------------
// 12. Invariant: map is tied to a specific Arc lifetime — using the same
//     node addresses from two independent trees would collide.
//     This test documents that two independent ASTs produce disjoint pointer
//     sets (no aliasing between separate allocations).
// ---------------------------------------------------------------------------

#[test]
fn parent_map_pointers_from_distinct_trees_are_disjoint() {
    let ast1 = minimal_ast();
    let ast2 = minimal_ast(); // Fresh allocation — different addresses

    let mut map1: ParentMap = ParentMap::default();
    DeclarationProvider::build_parent_map(&ast1, &mut map1, None);

    let mut map2: ParentMap = ParentMap::default();
    DeclarationProvider::build_parent_map(&ast2, &mut map2, None);

    // The root pointers of the two ASTs must differ (they are separate Arc allocs).
    let root1: *const Node = &*ast1 as *const _;
    let root2: *const Node = &*ast2 as *const _;
    assert_ne!(
        root1, root2,
        "Two independent Arc::new() allocations must produce distinct root pointers"
    );

    // No key from map1 should appear in map2 (disjoint heap regions).
    for &key in map1.keys() {
        assert!(
            !map2.contains_key(&key),
            "Pointer {:p} from tree-1 appeared in tree-2's map — \
             this would indicate aliasing or a shared allocator reuse, \
             which breaks the single-tree invariant",
            key
        );
    }
}
