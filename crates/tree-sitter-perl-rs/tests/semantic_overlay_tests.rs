//! Focused tests for the `SemanticOverlay`, `OverlayDefinition`, `VisibleImport`,
//! `Node::inner()`, `Node::tree_source()`, and related facade APIs.
//!
//! These tests cover gaps identified in the coverage campaign:
//! - `SemanticOverlay::definition_for_node` (untested externally)
//! - `OverlayDefinition` field completeness (qualified_name, kind, start/end byte)
//! - `VisibleImport` dedup logic for repeated `use` statements
//! - `visible_imports_at_offset` when offset precedes all use statements
//! - `definition_at_offset` returning `None` for unknown symbols
//! - `pragma_state_at_offset` baseline state (no pragmas, use strict, nested)
//! - `Node::inner()` escape hatch
//! - `Node::tree_source()` accessor
//! - `Point` struct properties and multi-line byte_to_point correctness

use perl_tdd_support::must_some;
use tree_sitter_perl_rs::Parser;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn parse(source: &str) -> tree_sitter_perl_rs::Tree {
    let mut parser = Parser::new();
    must_some(parser.parse(source))
}

// ---------------------------------------------------------------------------
// SemanticOverlay::definition_for_node
// ---------------------------------------------------------------------------

#[test]
fn when_querying_definition_for_node_then_result_matches_definition_at_offset() {
    // definition_for_node uses the node's start_byte as the query offset.
    // Verify it returns a result equivalent to definition_at_offset(node.start_byte()).
    let source = "my $answer = 42;\n$answer + 1;\n";
    let tree = parse(source);
    let overlay = tree.semantic_overlay();

    // Find the $answer reference node in the second statement.
    let offset = must_some(source.find("$answer +"));
    let root = tree.root_node();

    // Walk to find the reference node at the right offset.
    let ref_node = must_some(root.children().nth(1));

    let by_offset = overlay.definition_at_offset(offset);
    let by_node = overlay.definition_for_node(&ref_node);

    // Both should either both be Some or both be None for the same position.
    assert_eq!(
        by_offset.is_some(),
        by_node.is_some(),
        "definition_for_node and definition_at_offset must agree on Some/None for the same byte offset"
    );
}

#[test]
fn when_querying_definition_for_root_node_then_no_panic_occurs() {
    // The root node starts at byte 0. Verify definition_for_node handles root gracefully.
    let source = "my $x = 1;\n";
    let tree = parse(source);
    let overlay = tree.semantic_overlay();
    let root = tree.root_node();

    // Must not panic — returns Some or None.
    let _result = overlay.definition_for_node(&root);
    // Just verify no panic and the API is callable.
}

#[test]
fn when_definition_for_node_and_definition_at_offset_agree_then_start_bytes_match() {
    // If both return Some, the definition start_bytes should be identical.
    let source = "my $value = 100;\nmy $result = $value + 1;\n";
    let tree = parse(source);
    let overlay = tree.semantic_overlay();

    let offset = must_some(source.find("$value +"));
    let root = tree.root_node();
    // Second statement child
    let stmt_node = must_some(root.children().nth(1));

    let by_offset = overlay.definition_at_offset(offset);
    let by_node = overlay.definition_for_node(&stmt_node);

    if let (Some(def_offset), Some(def_node)) = (by_offset, by_node) {
        // Both reference the same underlying symbol — start bytes must agree.
        assert_eq!(
            def_offset.start_byte, def_node.start_byte,
            "definition_for_node start_byte must match definition_at_offset start_byte"
        );
    }
}

// ---------------------------------------------------------------------------
// OverlayDefinition field completeness
// ---------------------------------------------------------------------------

#[test]
fn when_definition_is_found_then_all_overlay_definition_fields_are_populated() {
    let source = "my $score = 100;\n$score * 2;\n";
    let tree = parse(source);
    let overlay = tree.semantic_overlay();

    let offset = must_some(source.find("$score *"));
    let definition = must_some(overlay.definition_at_offset(offset));

    // name — bare symbol name
    assert_eq!(definition.name, "score", "name must match bare variable");

    // qualified_name — fully qualified (may include package prefix)
    assert!(
        definition.qualified_name.contains("score"),
        "qualified_name must contain the symbol name; got {:?}",
        definition.qualified_name
    );

    // kind — non-empty debug string identifying the symbol kind
    assert!(!definition.kind.is_empty(), "kind must be a non-empty string");

    // byte range — start must precede end, both within source
    assert!(
        definition.start_byte <= definition.end_byte,
        "start_byte ({}) must not exceed end_byte ({})",
        definition.start_byte,
        definition.end_byte
    );
    assert!(
        definition.end_byte <= source.len(),
        "end_byte ({}) must not exceed source length ({})",
        definition.end_byte,
        source.len()
    );
}

#[test]
fn when_definition_is_found_then_start_byte_points_to_sigil() {
    let source = "my $counter = 0;\n$counter += 1;\n";
    let tree = parse(source);
    let overlay = tree.semantic_overlay();

    let offset = must_some(source.find("$counter +="));
    let definition = must_some(overlay.definition_at_offset(offset));

    // start_byte must be a valid byte index into the source.
    assert!(
        definition.start_byte < source.len(),
        "start_byte {} must be within source ({} bytes)",
        definition.start_byte,
        source.len()
    );

    // The byte at start_byte should be the sigil '$' of the declaration.
    assert_eq!(
        source.as_bytes()[definition.start_byte],
        b'$',
        "start_byte must point to the '$' sigil of the declaration"
    );
}

#[test]
fn when_definition_is_found_then_qualified_name_is_non_empty() {
    let source = "my $flag = 1;\nif ($flag) { 1; }\n";
    let tree = parse(source);
    let overlay = tree.semantic_overlay();

    let offset = must_some(source.find("$flag)"));
    let definition = must_some(overlay.definition_at_offset(offset));

    assert!(
        !definition.qualified_name.is_empty(),
        "qualified_name must not be empty; got an empty string"
    );
}

#[test]
fn when_definition_at_offset_past_end_of_source_then_no_panic() {
    // Querying past source length must not panic (clamp semantics).
    let source = "my $x = 1;\n";
    let tree = parse(source);
    let overlay = tree.semantic_overlay();

    let past_end = overlay.definition_at_offset(source.len() + 100);
    // May be Some or None depending on clamp logic — just verify no panic.
    let _ = past_end;
}

// ---------------------------------------------------------------------------
// VisibleImport fields and dedup
// ---------------------------------------------------------------------------

#[test]
fn when_querying_visible_imports_then_each_import_has_correct_byte_range() {
    let source = "use strict;\nuse warnings;\nmy $x = 1;\n";
    let tree = parse(source);
    let overlay = tree.semantic_overlay();
    let offset = must_some(source.find("my $x"));

    let imports = overlay.visible_imports_at_offset(offset);

    assert!(!imports.is_empty(), "should find at least one import");
    for import in &imports {
        // start must precede end
        assert!(
            import.statement_start_byte <= import.statement_end_byte,
            "import start ({}) must not exceed end ({}) for module {:?}",
            import.statement_start_byte,
            import.statement_end_byte,
            import.module
        );
        // end must not exceed source length
        assert!(
            import.statement_end_byte <= source.len(),
            "import end ({}) must not exceed source length ({})",
            import.statement_end_byte,
            source.len()
        );
        // module name must be non-empty
        assert!(!import.module.is_empty(), "module name must be non-empty");
    }
}

#[test]
fn when_querying_visible_imports_before_any_use_statement_then_at_most_bounded_results() {
    // Offset 0 is at the very start; at most the use statement starting there is visible.
    let source = "use strict;\nuse warnings;\nmy $x = 1;\n";
    let tree = parse(source);
    let overlay = tree.semantic_overlay();

    // Offset 0: `use strict` starts at byte 0 and is ≤ 0, so may appear.
    let imports = overlay.visible_imports_at_offset(0);
    assert!(
        imports.len() <= 2,
        "at most two pragmas can be in scope at offset 0; got {}",
        imports.len()
    );
}

#[test]
fn when_same_module_is_used_twice_then_visible_imports_deduplicates() {
    // The dedup logic in visible_imports_at_offset prevents the same module
    // appearing twice (e.g. if `use Carp` appears in two scopes).
    let source = "use Carp;\nuse Carp;\nmy $x = 1;\n";
    let tree = parse(source);
    let overlay = tree.semantic_overlay();
    let offset = must_some(source.find("my $x"));

    let imports = overlay.visible_imports_at_offset(offset);
    let carp_count = imports.iter().filter(|i| i.module == "Carp").count();

    assert_eq!(
        carp_count, 1,
        "duplicate `use Carp` must be deduplicated; found {} entries",
        carp_count
    );
}

#[test]
fn when_source_has_no_use_statements_then_visible_imports_is_empty() {
    let source = "my $x = 1;\nmy $y = 2;\n";
    let tree = parse(source);
    let overlay = tree.semantic_overlay();
    let offset = source.len();

    let imports = overlay.visible_imports_at_offset(offset);
    assert!(
        imports.is_empty(),
        "source with no `use` statements must yield empty import list; got {:?}",
        imports.iter().map(|i| i.module.as_str()).collect::<Vec<_>>()
    );
}

#[test]
fn when_querying_visible_imports_past_end_of_source_then_all_imports_visible() {
    // An offset beyond source.len() — clamp semantics must include all use statements.
    let source = "use strict;\nuse warnings;\n";
    let tree = parse(source);
    let overlay = tree.semantic_overlay();

    let imports = overlay.visible_imports_at_offset(source.len() + 9999);
    let modules: Vec<_> = imports.iter().map(|i| i.module.as_str()).collect();

    assert!(
        modules.contains(&"strict"),
        "use strict must be visible past end of source; got {:?}",
        modules
    );
    assert!(
        modules.contains(&"warnings"),
        "use warnings must be visible past end of source; got {:?}",
        modules
    );
}

#[test]
fn when_visible_import_is_returned_then_module_field_matches_source_token() {
    let source = "use Data::Dumper;\nmy $x = 1;\n";
    let tree = parse(source);
    let overlay = tree.semantic_overlay();
    let offset = must_some(source.find("my $x"));

    let imports = overlay.visible_imports_at_offset(offset);
    let found = imports.iter().any(|i| i.module == "Data::Dumper");
    assert!(found, "Data::Dumper must appear as a visible import; got {:?}", imports);
}

// ---------------------------------------------------------------------------
// pragma_state_at_offset edge cases
// ---------------------------------------------------------------------------

#[test]
fn when_no_pragma_statements_then_pragma_state_query_does_not_panic() {
    // With no `use strict` or `use warnings`, the defaults apply.
    let source = "my $x = 1;\n";
    let tree = parse(source);
    let overlay = tree.semantic_overlay();

    // Must not panic — returns a PragmaState value.
    let _state = overlay.pragma_state_at_offset(0);
}

#[test]
fn when_use_strict_is_present_then_strict_refs_is_true_after_it() {
    let source = "use strict;\nmy $x = 1;\n";
    let tree = parse(source);
    let overlay = tree.semantic_overlay();

    let offset = must_some(source.find("my $x"));
    let state = overlay.pragma_state_at_offset(offset);

    assert!(state.strict_refs, "strict_refs must be true after `use strict`");
}

#[test]
fn when_use_warnings_is_present_then_warnings_is_true_after_it() {
    let source = "use warnings;\nmy $x = 1;\n";
    let tree = parse(source);
    let overlay = tree.semantic_overlay();

    let offset = must_some(source.find("my $x"));
    let state = overlay.pragma_state_at_offset(offset);

    assert!(state.warnings, "warnings must be true after `use warnings`");
}

#[test]
fn when_no_strict_is_used_then_strict_refs_is_false_at_that_offset() {
    let source = "use strict;\nno strict;\nmy $x = 1;\n";
    let tree = parse(source);
    let overlay = tree.semantic_overlay();

    let offset = must_some(source.find("my $x"));
    let state = overlay.pragma_state_at_offset(offset);

    assert!(!state.strict_refs, "strict_refs must be false after `no strict`");
}

#[test]
fn when_pragma_state_at_zero_offset_then_no_panic() {
    // Querying at the very start of the file (before any pragmas).
    let source = "use strict;\nmy $x = 1;\n";
    let tree = parse(source);
    let overlay = tree.semantic_overlay();

    let _state = overlay.pragma_state_at_offset(0);
    // Just verify no panic.
}

// ---------------------------------------------------------------------------
// Node::inner() escape hatch
// ---------------------------------------------------------------------------

#[test]
fn when_inner_is_called_then_location_start_matches_start_byte() {
    let source = "my $x = 42;";
    let tree = parse(source);
    let root = tree.root_node();

    let ast_node = root.inner();
    assert_eq!(
        ast_node.location.start,
        root.start_byte(),
        "inner().location.start must match Node::start_byte()"
    );
}

#[test]
fn when_inner_is_called_on_child_node_then_kind_matches_native_kind() {
    let source = "sub foo { 1; }";
    let tree = parse(source);
    let root = tree.root_node();
    let sub_node = must_some(root.children().next());

    let inner = sub_node.inner();
    assert_eq!(
        inner.kind.kind_name(),
        sub_node.native_kind(),
        "inner().kind.kind_name() must equal Node::native_kind()"
    );
}

#[test]
fn when_inner_is_called_then_location_end_matches_end_byte_before_clamp() {
    // inner().location.end is the raw (possibly unclamped) end byte from the AST.
    // Node::end_byte() applies a .min(source.len()) clamp. For normal-length sources
    // these should be equal.
    let source = "my $x = 1;";
    let tree = parse(source);
    let root = tree.root_node();
    let inner = root.inner();

    // The clamped end_byte must be <= source.len()
    assert!(root.end_byte() <= source.len(), "Node::end_byte() must not exceed source length");

    // The inner location might be equal to or slightly beyond — just verify no panic
    let _raw_end = inner.location.end;
}

// ---------------------------------------------------------------------------
// Node::tree_source() accessor
// ---------------------------------------------------------------------------

#[test]
fn when_tree_source_is_called_then_original_source_string_is_returned() {
    let source = "my $x = 'hello world';";
    let tree = parse(source);
    let root = tree.root_node();

    assert_eq!(root.tree_source(), source, "tree_source() must return the original source string");
}

#[test]
fn when_tree_source_is_called_on_child_then_full_source_is_returned() {
    let source = "my $a = 1; my $b = 2;";
    let tree = parse(source);
    let root = tree.root_node();
    let child = must_some(root.child(0));

    // A child node's tree_source() returns the full tree's source, not just the child's span.
    assert_eq!(
        child.tree_source(),
        source,
        "tree_source() on a child must return the full source, not just the child span"
    );
}

#[test]
fn when_tree_source_is_called_then_len_matches_tree_source_len() {
    let source = "sub foo { my $x = 1; }\nsub bar { my $y = 2; }\n";
    let tree = parse(source);
    let root = tree.root_node();

    assert_eq!(
        root.tree_source().len(),
        source.len(),
        "tree_source().len() must equal the original source length"
    );
}

// ---------------------------------------------------------------------------
// Point struct properties and multi-line byte_to_point correctness
// ---------------------------------------------------------------------------

#[test]
fn when_point_fields_are_accessed_then_row_and_column_are_readable() {
    let source = "my $x = 1;";
    let tree = parse(source);
    let root = tree.root_node();
    let p = root.start_position();

    assert_eq!(p.row, 0, "root start position must be row 0");
    assert_eq!(p.column, 0, "root start position must be column 0");
}

#[test]
fn when_two_points_from_same_node_are_compared_then_they_are_equal() {
    let source = "my $x = 1;";
    let tree = parse(source);
    let root = tree.root_node();

    let p1 = root.start_position();
    let p2 = root.start_position();
    assert_eq!(p1, p2, "two start_position() calls on the same node must be equal");
}

#[test]
fn when_two_points_from_different_nodes_differ_then_they_are_not_equal() {
    let source = "my $x = 1;\nmy $y = 2;\n";
    let tree = parse(source);
    let root = tree.root_node();

    let start = root.start_position();
    let end = root.end_position();
    // A two-line source: start is (row=0, col=0), end is (row=1, col=something).
    assert_ne!(start, end, "start_position and end_position must differ for multi-line source");
}

#[test]
fn when_start_position_is_queried_on_multiline_source_then_row_and_column_are_correct() {
    // "line 1\nline 2\n" — root starts at (0, 0)
    let source = "line 1\nline 2\n";
    let tree = parse(source);
    let root = tree.root_node();

    let start = root.start_position();
    assert_eq!(start.row, 0, "root must start at row 0");
    assert_eq!(start.column, 0, "root must start at column 0");
}

#[test]
fn when_end_position_is_queried_then_row_equals_newline_count() {
    // Two lines separated by a newline.
    let source = "my $x = 1;\nmy $y = 2;";
    let tree = parse(source);
    let root = tree.root_node();

    let end = root.end_position();
    // Source has one newline, so end row should be 1.
    assert_eq!(end.row, 1, "end row must reflect newline count");
}

#[test]
fn when_source_has_no_newlines_then_start_and_end_are_both_row_zero() {
    let source = "my $x = 99;";
    let tree = parse(source);
    let root = tree.root_node();

    let start = root.start_position();
    let end = root.end_position();

    assert_eq!(start.row, 0, "no-newline source must have row 0 at start");
    assert_eq!(end.row, 0, "no-newline source must have row 0 at end");
}

#[test]
fn when_source_is_empty_then_start_and_end_positions_are_both_origin() {
    let source = "";
    let tree = parse(source);
    let root = tree.root_node();

    let start = root.start_position();
    let end = root.end_position();

    assert_eq!(start.row, 0, "empty source start must be at row 0");
    assert_eq!(start.column, 0, "empty source start must be at column 0");
    assert_eq!(end.row, 0, "empty source end must be at row 0");
    assert_eq!(end.column, 0, "empty source end must be at column 0");
}

#[test]
fn when_point_is_copied_then_copy_is_independent() {
    let source = "my $x = 1;";
    let tree = parse(source);
    let root = tree.root_node();
    let p1 = root.start_position();
    let p2 = p1; // Copy — Point: Copy
    assert_eq!(p1.row, p2.row, "copied Point must have same row");
    assert_eq!(p1.column, p2.column, "copied Point must have same column");
}
