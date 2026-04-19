//! Behavior tests for IncrementalParser: boundary conditions, merging, and clear().
//!
//! These tests target mutation-killing boundary conditions not covered by the
//! happy-path tests in the inline #[cfg(test)] block.

use perl_lsp_performance::IncrementalParser;

// ---------------------------------------------------------------------------
// needs_reparse: overlap detection boundary conditions
// ---------------------------------------------------------------------------

#[test]
fn needs_reparse_returns_false_when_no_changed_regions() {
    let parser = IncrementalParser::new();
    assert!(
        !parser.needs_reparse(0, 100),
        "empty change list → nothing needs reparse"
    );
}

#[test]
fn needs_reparse_node_ending_exactly_at_region_start_does_not_overlap() {
    let mut parser = IncrementalParser::new();
    // region (10, 20)
    parser.mark_changed(10, 20);
    // node (5, 10): node_end == region_start → node_end > *start is 10 > 10 = false
    assert!(
        !parser.needs_reparse(5, 10),
        "node ending at region start must not be flagged as needing reparse"
    );
}

#[test]
fn needs_reparse_node_starting_exactly_at_region_end_does_not_overlap() {
    let mut parser = IncrementalParser::new();
    // region (10, 20)
    parser.mark_changed(10, 20);
    // node (20, 30): node_start == region_end → node_start < *end is 20 < 20 = false
    assert!(
        !parser.needs_reparse(20, 30),
        "node starting at region end must not be flagged as needing reparse"
    );
}

#[test]
fn needs_reparse_node_spanning_entire_region_overlaps() {
    let mut parser = IncrementalParser::new();
    parser.mark_changed(10, 20);
    // node (5, 25) fully contains region (10, 20)
    assert!(
        parser.needs_reparse(5, 25),
        "node spanning entire region must need reparse"
    );
}

#[test]
fn needs_reparse_region_spanning_entire_node_overlaps() {
    let mut parser = IncrementalParser::new();
    parser.mark_changed(0, 100);
    // node (10, 20) fully inside region (0, 100)
    assert!(
        parser.needs_reparse(10, 20),
        "node fully inside region must need reparse"
    );
}

#[test]
fn needs_reparse_partial_left_overlap() {
    let mut parser = IncrementalParser::new();
    parser.mark_changed(10, 20);
    // node (8, 12): overlaps left edge of region
    assert!(
        parser.needs_reparse(8, 12),
        "partial left overlap must need reparse"
    );
}

#[test]
fn needs_reparse_partial_right_overlap() {
    let mut parser = IncrementalParser::new();
    parser.mark_changed(10, 20);
    // node (18, 25): overlaps right edge of region
    assert!(
        parser.needs_reparse(18, 25),
        "partial right overlap must need reparse"
    );
}

#[test]
fn needs_reparse_with_multiple_disjoint_regions_only_matches_overlapping() {
    let mut parser = IncrementalParser::new();
    parser.mark_changed(10, 20);
    parser.mark_changed(30, 40);
    // Between the two regions: (22, 28) → should not overlap
    assert!(
        !parser.needs_reparse(22, 28),
        "node between regions must not need reparse"
    );
    // Overlaps second region: (35, 45) → should overlap
    assert!(
        parser.needs_reparse(35, 45),
        "node overlapping second region must need reparse"
    );
}

// ---------------------------------------------------------------------------
// merge_overlapping_regions: triggered by mark_changed
// ---------------------------------------------------------------------------

#[test]
fn mark_changed_merges_fully_overlapping_regions() {
    let mut parser = IncrementalParser::new();
    parser.mark_changed(0, 50);
    parser.mark_changed(10, 30); // fully inside first
    // After merge, should still be (0, 50)
    assert!(
        parser.needs_reparse(0, 1),
        "start of merged region must still trigger reparse"
    );
    assert!(
        parser.needs_reparse(49, 51),
        "end of merged region must still trigger reparse"
    );
}

#[test]
fn mark_changed_merges_adjacent_touching_regions_into_one() {
    let mut parser = IncrementalParser::new();
    parser.mark_changed(0, 10);
    parser.mark_changed(10, 20); // adjacent (not overlapping, but touching)
    // The merge logic checks `start <= current.1` which is `10 <= 10` = true → merge
    // So (0, 10) + (10, 20) → (0, 20)
    assert!(
        parser.needs_reparse(5, 15),
        "node spanning the boundary of adjacent touching regions must need reparse"
    );
}

#[test]
fn mark_changed_does_not_merge_non_adjacent_regions() {
    let mut parser = IncrementalParser::new();
    parser.mark_changed(0, 10);
    parser.mark_changed(20, 30); // gap between 10 and 20
    // Node in the gap: (11, 19) → must not overlap with either region
    assert!(
        !parser.needs_reparse(11, 19),
        "node in gap between regions must not need reparse"
    );
}

#[test]
fn mark_changed_three_overlapping_regions_collapse_to_one() {
    let mut parser = IncrementalParser::new();
    parser.mark_changed(0, 15);
    parser.mark_changed(10, 25);
    parser.mark_changed(20, 35);
    // All three overlap sequentially → should collapse to (0, 35)
    assert!(parser.needs_reparse(0, 1));
    assert!(parser.needs_reparse(30, 36));
}

// ---------------------------------------------------------------------------
// clear()
// ---------------------------------------------------------------------------

#[test]
fn clear_removes_all_changed_regions() {
    let mut parser = IncrementalParser::new();
    parser.mark_changed(10, 20);
    parser.mark_changed(30, 40);
    assert!(
        parser.needs_reparse(15, 25),
        "before clear: region must trigger reparse"
    );

    parser.clear();

    assert!(
        !parser.needs_reparse(15, 25),
        "after clear: no region should trigger reparse"
    );
    assert!(
        !parser.needs_reparse(35, 45),
        "after clear: second region should also be cleared"
    );
}

#[test]
fn clear_on_empty_parser_does_not_panic() {
    let mut parser = IncrementalParser::new();
    parser.clear(); // should be a no-op
    assert!(!parser.needs_reparse(0, 100));
}

#[test]
fn mark_changed_after_clear_works_correctly() {
    let mut parser = IncrementalParser::new();
    parser.mark_changed(10, 20);
    parser.clear();
    parser.mark_changed(50, 60);

    assert!(
        !parser.needs_reparse(10, 20),
        "old region must not exist after clear"
    );
    assert!(
        parser.needs_reparse(55, 65),
        "new region after clear must work"
    );
}
