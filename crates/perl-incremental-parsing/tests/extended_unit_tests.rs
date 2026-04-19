//! Extended unit tests for perl-incremental-parsing.
//!
//! Covers the full public API surface:
//! - LineIndex: byte↔position mapping edge cases
//! - IncrementalState: creation, checkpoint lookup, multi-edit apply
//! - Edit: LSP change conversion, full-document replacement
//! - apply_edits: single/multi-edit paths, fallback thresholds
//! - IncrementalEdit / IncrementalEditSet: arithmetic, overlap, apply
//! - IncrementalDocument: single & batch edits, cache, metrics, accessors
//! - SimpleIncrementalParser: initial, incremental, structural, default
//! - CheckpointedIncrementalParser: parse, edit, stats, cache clear
//! - IncrementalParserV2: value edits, whitespace, advanced reuse, metrics
//! - IncrementalTree: node-map lookup, find_containing_node
//! - IncrementalMetrics: efficiency, performance category
//! - AdvancedReuseAnalyzer: reuse analysis, ReuseConfig, ReuseAnalysisResult
//! - Integration: lsp_pos_to_byte, byte_to_lsp_pos, DocumentParser, IncrementalConfig

use perl_incremental_parsing::incremental::incremental_advanced_reuse::{
    AdvancedReuseAnalyzer, ReuseConfig,
};
use perl_incremental_parsing::incremental::incremental_checkpoint::{
    CheckpointedIncrementalParser, SimpleEdit,
};
use perl_incremental_parsing::incremental::incremental_document::IncrementalDocument;
use perl_incremental_parsing::incremental::incremental_edit::{
    IncrementalEdit, IncrementalEditSet,
};
use perl_incremental_parsing::incremental::incremental_integration::{
    DocumentParser, IncrementalConfig, byte_to_lsp_pos, lsp_pos_to_byte,
};
use perl_incremental_parsing::incremental::incremental_simple::SimpleIncrementalParser;
use perl_incremental_parsing::incremental::incremental_v2::{
    IncrementalMetrics, IncrementalParserV2, IncrementalTree,
};
use perl_incremental_parsing::incremental::{
    Edit, IncrementalState, LineIndex, ParseCheckpoint, ScopeSnapshot, apply_edits,
};
use perl_incremental_parsing::position::Position;
use perl_incremental_parsing::{Node, NodeKind, Parser};

use lsp_types::{Range as LspRange, TextDocumentContentChangeEvent};
use ropey::Rope;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_ok(source: &str) -> Result<Node, Box<dyn std::error::Error>> {
    let mut p = Parser::new(source);
    Ok(p.parse()?)
}

fn make_edit(start: usize, old_end: usize, new_end: usize) -> perl_incremental_parsing::edit::Edit {
    perl_incremental_parsing::edit::Edit::new(
        start,
        old_end,
        new_end,
        Position::new(start, 0, 0),
        Position::new(old_end, 0, 0),
        Position::new(new_end, 0, 0),
    )
}

// =========================================================================
// LineIndex tests
// =========================================================================

#[test]
fn line_index_empty_string() {
    let idx = LineIndex::new("");
    assert_eq!(idx.byte_to_position(0), (0, 0));
}

#[test]
fn line_index_single_line() {
    let idx = LineIndex::new("hello");
    assert_eq!(idx.byte_to_position(0), (0, 0));
    assert_eq!(idx.byte_to_position(3), (0, 3));
}

#[test]
fn line_index_multi_line() {
    let idx = LineIndex::new("ab\ncd\nef");
    // line 0: bytes 0..2  (a b)
    // line 1: bytes 3..4  (c d)
    // line 2: bytes 6..7  (e f)
    assert_eq!(idx.byte_to_position(0), (0, 0));
    assert_eq!(idx.byte_to_position(3), (1, 0));
    assert_eq!(idx.byte_to_position(6), (2, 0));
}

#[test]
fn line_index_position_to_byte_roundtrip() {
    let idx = LineIndex::new("abc\ndef\nghi\n");
    // line 1 col 0 -> byte 4
    let byte = idx.position_to_byte(1, 0);
    assert_eq!(byte, Some(4));
    let pos = idx.byte_to_position(4);
    assert_eq!(pos, (1, 0));
}

#[test]
fn line_index_position_to_byte_out_of_range() {
    let idx = LineIndex::new("hi");
    assert_eq!(idx.position_to_byte(99, 0), None);
}

#[test]
fn line_index_trailing_newline() {
    let idx = LineIndex::new("a\n");
    // line 0: byte 0  (a)
    // line 1: byte 2  (empty)
    assert_eq!(idx.position_to_byte(1, 0), Some(2));
}

// =========================================================================
// ScopeSnapshot / ParseCheckpoint
// =========================================================================

#[test]
fn scope_snapshot_default_is_empty() {
    let s = ScopeSnapshot::default();
    assert!(s.package_name.is_empty());
    assert!(s.locals.is_empty());
    assert!(s.our_vars.is_empty());
    assert!(s.parent_isa.is_empty());
}

#[test]
fn parse_checkpoint_fields_accessible() {
    let cp = ParseCheckpoint {
        byte: 42,
        scope_snapshot: ScopeSnapshot::default(),
        node_id: 7,
    };
    assert_eq!(cp.byte, 42);
    assert_eq!(cp.node_id, 7);
}

// =========================================================================
// IncrementalState tests
// =========================================================================

#[test]
fn incremental_state_new_simple() {
    let state = IncrementalState::new("my $x = 1;".to_string());
    assert_eq!(state.source, "my $x = 1;");
    assert!(!state.tokens.is_empty());
    assert!(!state.lex_checkpoints.is_empty());
}

#[test]
fn incremental_state_new_empty() {
    let state = IncrementalState::new(String::new());
    assert!(state.source.is_empty());
}

#[test]
fn incremental_state_new_multiline() {
    let src = "my $a = 1;\nmy $b = 2;\n";
    let state = IncrementalState::new(src.to_string());
    assert!(state.tokens.len() >= 2);
    assert!(!state.lex_checkpoints.is_empty());
}

#[test]
fn incremental_state_find_lex_checkpoint_beginning() {
    let state = IncrementalState::new("my $x = 1; my $y = 2;".to_string());
    let cp = state.find_lex_checkpoint(0);
    assert!(cp.is_some());
    if let Some(c) = cp {
        assert_eq!(c.byte, 0);
    }
}

#[test]
fn incremental_state_find_lex_checkpoint_middle() {
    let state = IncrementalState::new("my $a = 1; my $b = 2;".to_string());
    let cp = state.find_lex_checkpoint(12);
    assert!(cp.is_some());
}

#[test]
fn incremental_state_find_parse_checkpoint_empty_source() {
    let state = IncrementalState::new(String::new());
    // May or may not have parse checkpoints for empty source
    let _cp = state.find_parse_checkpoint(0);
}

#[test]
fn incremental_state_find_parse_checkpoint_with_sub() {
    let state = IncrementalState::new("sub foo { return 1; }".to_string());
    // Should have parse checkpoints for subroutine
    let _cp = state.find_parse_checkpoint(5);
}

// =========================================================================
// Edit (mod.rs) tests
// =========================================================================

#[test]
fn edit_from_lsp_change_with_range() {
    let line_index = LineIndex::new("hello world");
    let change = TextDocumentContentChangeEvent {
        range: Some(LspRange {
            start: lsp_types::Position {
                line: 0,
                character: 6,
            },
            end: lsp_types::Position {
                line: 0,
                character: 11,
            },
        }),
        range_length: None,
        text: "Perl".to_string(),
    };

    let edit = Edit::from_lsp_change(&change, &line_index, "hello world");
    assert!(edit.is_some());
    if let Some(e) = edit {
        assert_eq!(e.start_byte, 6);
        assert_eq!(e.old_end_byte, 11);
        assert_eq!(e.new_end_byte, 10);
        assert_eq!(e.new_text, "Perl");
    }
}

#[test]
fn edit_from_lsp_change_full_document() {
    let line_index = LineIndex::new("old");
    let change = TextDocumentContentChangeEvent {
        range: None,
        range_length: None,
        text: "new content".to_string(),
    };

    let edit = Edit::from_lsp_change(&change, &line_index, "old");
    assert!(edit.is_some());
    if let Some(e) = edit {
        assert_eq!(e.start_byte, 0);
        assert_eq!(e.old_end_byte, 3);
        assert_eq!(e.new_end_byte, 11);
    }
}

// =========================================================================
// apply_edits tests
// =========================================================================

#[test]
fn apply_edits_single_small_edit() -> Result<(), Box<dyn std::error::Error>> {
    let mut state = IncrementalState::new("my $x = 42;".to_string());
    let edit = Edit {
        start_byte: 8,
        old_end_byte: 10,
        new_end_byte: 12,
        new_text: "9999".to_string(),
    };
    let result = apply_edits(&mut state, &[edit])?;
    assert!(!result.changed_ranges.is_empty());
    assert!(result.reparsed_bytes > 0);
    assert!(state.source.contains("9999"));
    Ok(())
}

#[test]
fn apply_edits_multiple_edits_triggers_full_reparse() -> Result<(), Box<dyn std::error::Error>> {
    let mut state = IncrementalState::new("my $a = 1; my $b = 2;".to_string());
    let edits = vec![
        Edit {
            start_byte: 8,
            old_end_byte: 9,
            new_end_byte: 10,
            new_text: "11".to_string(),
        },
        Edit {
            start_byte: 19,
            old_end_byte: 20,
            new_end_byte: 21,
            new_text: "22".to_string(),
        },
    ];
    let result = apply_edits(&mut state, &edits)?;
    assert!(!result.changed_ranges.is_empty());
    Ok(())
}

#[test]
fn apply_edits_empty_list() -> Result<(), Box<dyn std::error::Error>> {
    let mut state = IncrementalState::new("my $x = 1;".to_string());
    let result = apply_edits(&mut state, &[])?;
    // Empty edits should still trigger full reparse (multiple=0 path)
    assert!(!result.changed_ranges.is_empty());
    Ok(())
}

#[test]
fn apply_edits_large_edit_falls_back_to_full_reparse() -> Result<(), Box<dyn std::error::Error>> {
    let src = "my $x = 1;".to_string();
    let mut state = IncrementalState::new(src);
    // Large new text (>1024 bytes)
    let big_text = "a".repeat(2000);
    let edit = Edit {
        start_byte: 8,
        old_end_byte: 9,
        new_end_byte: 8 + big_text.len(),
        new_text: big_text,
    };
    let result = apply_edits(&mut state, &[edit])?;
    assert!(result.reparsed_bytes > 0);
    Ok(())
}

// =========================================================================
// IncrementalEdit tests
// =========================================================================

#[test]
fn incremental_edit_new_basic() {
    let edit = IncrementalEdit::new(10, 20, "replacement".to_string());
    assert_eq!(edit.start_byte, 10);
    assert_eq!(edit.old_end_byte, 20);
    assert_eq!(edit.new_end_byte(), 21);
    assert_eq!(edit.byte_shift(), 1);
}

#[test]
fn incremental_edit_insertion_at_point() {
    let edit = IncrementalEdit::new(5, 5, "inserted".to_string());
    assert_eq!(edit.byte_shift(), 8);
    assert_eq!(edit.new_end_byte(), 13);
}

#[test]
fn incremental_edit_deletion() {
    let edit = IncrementalEdit::new(0, 10, String::new());
    assert_eq!(edit.byte_shift(), -10);
    assert_eq!(edit.new_end_byte(), 0);
}

#[test]
fn incremental_edit_overlaps_true() {
    let edit = IncrementalEdit::new(5, 15, "x".to_string());
    assert!(edit.overlaps(10, 20));
    assert!(edit.overlaps(0, 10));
    assert!(edit.overlaps(5, 15));
}

#[test]
fn incremental_edit_overlaps_false() {
    let edit = IncrementalEdit::new(5, 10, "x".to_string());
    assert!(!edit.overlaps(10, 20));
    assert!(!edit.overlaps(0, 5));
}

#[test]
fn incremental_edit_is_before() {
    let edit = IncrementalEdit::new(0, 5, "x".to_string());
    assert!(edit.is_before(5));
    assert!(edit.is_before(10));
    assert!(!edit.is_before(3));
}

#[test]
fn incremental_edit_is_after() {
    let edit = IncrementalEdit::new(10, 15, "x".to_string());
    assert!(edit.is_after(10));
    assert!(edit.is_after(5));
    assert!(!edit.is_after(11));
}

#[test]
fn incremental_edit_with_positions() {
    let start_pos = Position::new(10, 1, 5);
    let end_pos = Position::new(20, 1, 15);
    let edit = IncrementalEdit::with_positions(10, 20, "hello".to_string(), start_pos, end_pos);
    assert_eq!(edit.start_position.byte, 10);
    assert_eq!(edit.old_end_position.byte, 20);
}

// =========================================================================
// IncrementalEditSet tests
// =========================================================================

#[test]
fn edit_set_empty() {
    let set = IncrementalEditSet::new();
    assert!(set.is_empty());
    assert_eq!(set.total_byte_shift(), 0);
}

#[test]
fn edit_set_add_and_sort() {
    let mut set = IncrementalEditSet::new();
    set.add(IncrementalEdit::new(10, 15, "b".to_string()));
    set.add(IncrementalEdit::new(0, 5, "a".to_string()));
    assert!(!set.is_empty());
    set.sort();
    assert_eq!(set.edits[0].start_byte, 0);
    assert_eq!(set.edits[1].start_byte, 10);
}

#[test]
fn edit_set_sort_reverse() {
    let mut set = IncrementalEditSet::new();
    set.add(IncrementalEdit::new(0, 5, "a".to_string()));
    set.add(IncrementalEdit::new(10, 15, "b".to_string()));
    set.sort_reverse();
    assert_eq!(set.edits[0].start_byte, 10);
    assert_eq!(set.edits[1].start_byte, 0);
}

#[test]
fn edit_set_total_byte_shift() {
    let mut set = IncrementalEditSet::new();
    set.add(IncrementalEdit::new(0, 5, "abc".to_string())); // shift = -2
    set.add(IncrementalEdit::new(10, 10, "xx".to_string())); // shift = +2
    assert_eq!(set.total_byte_shift(), 0);
}

#[test]
fn edit_set_apply_to_string_single() {
    let mut set = IncrementalEditSet::new();
    set.add(IncrementalEdit::new(0, 5, "HI".to_string()));
    let result = set.apply_to_string("hello world");
    assert_eq!(result, "HI world");
}

#[test]
fn edit_set_apply_to_string_multiple() {
    let mut set = IncrementalEditSet::new();
    set.add(IncrementalEdit::new(0, 5, "Hello".to_string()));
    set.add(IncrementalEdit::new(6, 11, "Perl".to_string()));
    let result = set.apply_to_string("hello world");
    assert_eq!(result, "Hello Perl");
}

#[test]
fn edit_set_apply_to_empty_source() {
    let set = IncrementalEditSet::new();
    let result = set.apply_to_string("");
    assert_eq!(result, "");
}

#[test]
fn edit_set_apply_insert_only() {
    let mut set = IncrementalEditSet::new();
    set.add(IncrementalEdit::new(3, 3, " brave new".to_string()));
    let result = set.apply_to_string("my world");
    assert_eq!(result, "my  brave newworld");
}

// =========================================================================
// IncrementalDocument tests
// =========================================================================

#[test]
fn incremental_document_new_simple() {
    let doc = IncrementalDocument::new("my $x = 1;".to_string());
    assert!(doc.is_ok());
    if let Ok(d) = doc {
        assert_eq!(d.version, 0);
        assert!(!d.source.is_empty());
        assert!(d.metrics.last_parse_time_ms >= 0.0);
    }
}

#[test]
fn incremental_document_tree_accessor() {
    let doc = IncrementalDocument::new("my $a = 42;".to_string());
    if let Ok(d) = doc {
        let tree = d.tree();
        assert!(matches!(tree.kind, NodeKind::Program { .. }));
    }
}

#[test]
fn incremental_document_text_accessor() {
    let doc = IncrementalDocument::new("print 1;".to_string());
    if let Ok(d) = doc {
        assert_eq!(d.text(), "print 1;");
    }
}

#[test]
fn incremental_document_metrics_accessor() {
    let doc = IncrementalDocument::new("my $x = 1;".to_string());
    if let Ok(d) = doc {
        let m = d.metrics();
        assert!(m.last_parse_time_ms >= 0.0);
    }
}

#[test]
fn incremental_document_apply_edit_changes_source() {
    let doc = IncrementalDocument::new("my $x = 42;".to_string());
    if let Ok(mut d) = doc {
        let edit = IncrementalEdit::new(8, 10, "99".to_string());
        let result = d.apply_edit(edit);
        assert!(result.is_ok());
        assert!(d.source.contains("99"));
        assert_eq!(d.version, 1);
    }
}

#[test]
fn incremental_document_apply_batch_edits() {
    let doc = IncrementalDocument::new("my $a = 1; my $b = 2;".to_string());
    if let Ok(mut d) = doc {
        let mut edits = IncrementalEditSet::new();
        edits.add(IncrementalEdit::new(8, 9, "11".to_string()));
        edits.add(IncrementalEdit::new(19, 20, "22".to_string()));
        let result = d.apply_edits(&edits);
        assert!(result.is_ok());
    }
}

#[test]
fn incremental_document_set_cache_max_size() {
    let doc = IncrementalDocument::new("my $x = 1;".to_string());
    if let Ok(mut d) = doc {
        d.set_cache_max_size(500);
        assert_eq!(d.subtree_cache.max_size, 500);
    }
}

#[test]
fn incremental_document_cache_populated_after_new() {
    let doc = IncrementalDocument::new("my $x = 1; my $y = 2;".to_string());
    if let Ok(d) = doc {
        // Cache should have entries after initial parse
        assert!(!d.subtree_cache.by_range.is_empty());
    }
}

// =========================================================================
// SimpleIncrementalParser tests
// =========================================================================

#[test]
fn simple_parser_default_creates_new() {
    let parser = SimpleIncrementalParser::default();
    assert_eq!(parser.reused_nodes, 0);
    assert_eq!(parser.reparsed_nodes, 0);
}

#[test]
fn simple_parser_initial_parse() {
    let mut parser = SimpleIncrementalParser::new();
    let result = parser.parse("my $x = 1;");
    assert!(result.is_ok());
    assert!(parser.reparsed_nodes > 0);
    assert_eq!(parser.reused_nodes, 0);
}

#[test]
fn simple_parser_incremental_value_edit() {
    let mut parser = SimpleIncrementalParser::new();
    let _ = parser.parse("my $x = 42;");

    parser.edit(make_edit(8, 10, 12));
    let result = parser.parse("my $x = 4242;");
    assert!(result.is_ok());
    // Should reuse some nodes
    assert!(parser.reused_nodes > 0);
}

#[test]
fn simple_parser_structural_change_falls_back() {
    let mut parser = SimpleIncrementalParser::new();
    let _ = parser.parse("my $x = 1;");

    // Edit that inserts an if-block - structural change
    parser.edit(make_edit(0, 10, 30));
    let result = parser.parse("if (1) { my $x = 1; } else { }");
    assert!(result.is_ok());
}

#[test]
fn simple_parser_parse_twice_without_edit() {
    let mut parser = SimpleIncrementalParser::new();
    let _ = parser.parse("my $x = 1;");
    // Second parse without edits falls back to full parse
    let result = parser.parse("my $x = 1;");
    assert!(result.is_ok());
}

// =========================================================================
// CheckpointedIncrementalParser tests
// =========================================================================

#[test]
fn checkpointed_parser_initial_parse() {
    let mut parser = CheckpointedIncrementalParser::new();
    let result = parser.parse("my $x = 1;\nmy $y = 2;\n".to_string());
    assert!(result.is_ok());
    let stats = parser.stats();
    assert_eq!(stats.total_parses, 1);
    assert_eq!(stats.incremental_parses, 0);
}

#[test]
fn checkpointed_parser_apply_edit() {
    let mut parser = CheckpointedIncrementalParser::new();
    let _ = parser.parse("my $x = 42; my $y = 99;".to_string());
    let edit = SimpleEdit {
        start: 8,
        end: 10,
        new_text: "55".to_string(),
    };
    let result = parser.apply_edit(&edit);
    assert!(result.is_ok());
    let stats = parser.stats();
    assert_eq!(stats.incremental_parses, 1);
}

#[test]
fn checkpointed_parser_multiple_edits_sequential() {
    let mut parser = CheckpointedIncrementalParser::new();
    let _ = parser.parse("my $x = 1;\n".repeat(10));

    let edit1 = SimpleEdit {
        start: 8,
        end: 9,
        new_text: "42".to_string(),
    };
    let _ = parser.apply_edit(&edit1);

    let edit2 = SimpleEdit {
        start: 20,
        end: 21,
        new_text: "99".to_string(),
    };
    let _ = parser.apply_edit(&edit2);

    let stats = parser.stats();
    assert_eq!(stats.incremental_parses, 2);
    assert!(stats.total_parses >= 3);
}

#[test]
fn checkpointed_parser_clear_caches() {
    let mut parser = CheckpointedIncrementalParser::new();
    let _ = parser.parse("my $x = 1;".to_string());
    parser.clear_caches();
    // After clearing, should still be able to apply edits (falls back to full parse)
    let edit = SimpleEdit {
        start: 8,
        end: 9,
        new_text: "2".to_string(),
    };
    let result = parser.apply_edit(&edit);
    assert!(result.is_ok());
}

#[test]
fn checkpointed_parser_default_impl() {
    let parser = CheckpointedIncrementalParser::default();
    let stats = parser.stats();
    assert_eq!(stats.total_parses, 0);
}

#[test]
fn checkpointed_parser_stats_tokens_relexed() {
    let mut parser = CheckpointedIncrementalParser::new();
    let _ = parser.parse("my $a = 1; my $b = 2;".to_string());
    let edit = SimpleEdit {
        start: 8,
        end: 9,
        new_text: "42".to_string(),
    };
    let _ = parser.apply_edit(&edit);
    let stats = parser.stats();
    assert!(stats.tokens_relexed > 0);
}

// =========================================================================
// IncrementalParserV2 tests
// =========================================================================

#[test]
fn v2_parser_initial_parse() {
    let mut parser = IncrementalParserV2::new();
    let result = parser.parse("my $x = 1;");
    assert!(result.is_ok());
    assert_eq!(parser.reused_nodes, 0);
    assert!(parser.reparsed_nodes > 0);
}

#[test]
fn v2_parser_value_edit_reuses_nodes() {
    let mut parser = IncrementalParserV2::new();
    let _ = parser.parse("my $x = 42;");

    parser.edit(make_edit(8, 10, 12));
    let result = parser.parse("my $x = 9999;");
    assert!(result.is_ok());
    // v2 should attempt incremental reuse
    let total = parser.reused_nodes + parser.reparsed_nodes;
    assert!(total > 0);
}

#[test]
fn v2_parser_with_reuse_config() {
    let config = ReuseConfig {
        min_confidence: 0.5,
        max_position_shift: 500,
        aggressive_structural_matching: true,
        enable_content_reuse: true,
        max_analysis_depth: 5,
    };
    let mut parser = IncrementalParserV2::with_reuse_config(config);
    let result = parser.parse("my $x = 1;");
    assert!(result.is_ok());
}

#[test]
fn v2_parser_get_metrics() {
    let mut parser = IncrementalParserV2::new();
    let _ = parser.parse("my $x = 1;");
    let metrics = parser.get_metrics();
    assert_eq!(metrics.edit_count, 0);
}

#[test]
fn v2_parser_reset_metrics() {
    let mut parser = IncrementalParserV2::new();
    let _ = parser.parse("my $x = 1;");
    parser.reset_metrics();
    let metrics = parser.get_metrics();
    assert_eq!(metrics.nodes_reused, 0);
    assert_eq!(metrics.nodes_reparsed, 0);
}

#[test]
fn v2_parser_set_reuse_config() {
    let mut parser = IncrementalParserV2::new();
    let _ = parser.parse("my $x = 1;");
    let config = ReuseConfig {
        min_confidence: 0.9,
        max_position_shift: 100,
        aggressive_structural_matching: false,
        enable_content_reuse: false,
        max_analysis_depth: 3,
    };
    parser.set_reuse_config(config);
    // Should still be able to parse after config change
    let result = parser.parse("my $x = 2;");
    assert!(result.is_ok());
}

#[test]
fn v2_parser_used_advanced_reuse_false_on_first_parse() {
    let mut parser = IncrementalParserV2::new();
    let _ = parser.parse("my $x = 1;");
    assert!(!parser.used_advanced_reuse());
}

#[test]
fn v2_parser_get_reuse_efficiency_report() {
    let mut parser = IncrementalParserV2::new();
    let _ = parser.parse("my $x = 1;");
    let report = parser.get_reuse_efficiency_report();
    assert!(!report.is_empty());
}

#[test]
fn v2_parser_get_last_reuse_analysis_none_on_first_parse() {
    let mut parser = IncrementalParserV2::new();
    let _ = parser.parse("my $x = 1;");
    assert!(parser.get_last_reuse_analysis().is_none());
}

#[test]
fn v2_parser_multiple_value_edits() {
    let mut parser = IncrementalParserV2::new();
    let _ = parser.parse("my $x = 42;");

    parser.edit(make_edit(8, 10, 10));
    let _ = parser.parse("my $x = 55;");

    parser.edit(make_edit(8, 10, 10));
    let result = parser.parse("my $x = 77;");
    assert!(result.is_ok());
}

// =========================================================================
// IncrementalTree tests
// =========================================================================

#[test]
fn incremental_tree_new_builds_node_map() {
    let root = parse_ok("my $x = 1;");
    if let Ok(r) = root {
        let tree = IncrementalTree::new(r, "my $x = 1;".to_string());
        assert_eq!(tree.source, "my $x = 1;");
    }
}

#[test]
fn incremental_tree_find_containing_node_root() {
    let root = parse_ok("my $x = 42;");
    if let Ok(r) = root {
        let tree = IncrementalTree::new(r, "my $x = 42;".to_string());
        let node = tree.find_containing_node(0, 11);
        assert!(node.is_some());
    }
}

#[test]
fn incremental_tree_find_containing_node_inner() {
    let root = parse_ok("my $x = 42;");
    if let Ok(r) = root {
        let tree = IncrementalTree::new(r, "my $x = 42;".to_string());
        // The number literal "42" should be findable
        let node = tree.find_containing_node(8, 10);
        assert!(node.is_some());
    }
}

#[test]
fn incremental_tree_find_containing_node_out_of_range() {
    let root = parse_ok("my $x = 1;");
    if let Ok(r) = root {
        let tree = IncrementalTree::new(r, "my $x = 1;".to_string());
        let node = tree.find_containing_node(100, 200);
        assert!(node.is_none());
    }
}

// =========================================================================
// IncrementalMetrics tests
// =========================================================================

#[test]
fn metrics_new_default() {
    let m = IncrementalMetrics::new();
    assert_eq!(m.parse_time_micros, 0);
    assert_eq!(m.nodes_reused, 0);
    assert_eq!(m.nodes_reparsed, 0);
    assert_eq!(m.edit_count, 0);
}

#[test]
fn metrics_efficiency_percentage_zero() {
    let m = IncrementalMetrics::new();
    let eff = m.efficiency_percentage();
    assert!((eff - 0.0).abs() < f64::EPSILON);
}

#[test]
fn metrics_efficiency_percentage_half() {
    let m = IncrementalMetrics {
        nodes_reused: 50,
        nodes_reparsed: 50,
        ..Default::default()
    };
    let eff = m.efficiency_percentage();
    assert!((eff - 50.0).abs() < 0.1);
}

#[test]
fn metrics_efficiency_percentage_full() {
    let m = IncrementalMetrics {
        nodes_reused: 100,
        nodes_reparsed: 0,
        ..Default::default()
    };
    let eff = m.efficiency_percentage();
    assert!((eff - 100.0).abs() < 0.1);
}

#[test]
fn metrics_is_sub_millisecond() {
    let m = IncrementalMetrics {
        parse_time_micros: 500,
        ..Default::default()
    };
    assert!(m.is_sub_millisecond());

    let m2 = IncrementalMetrics {
        parse_time_micros: 1500,
        ..Default::default()
    };
    assert!(!m2.is_sub_millisecond());
}

#[test]
fn metrics_performance_category_excellent() {
    let m = IncrementalMetrics {
        parse_time_micros: 50,
        ..Default::default()
    };
    assert_eq!(m.performance_category(), "Excellent (<100µs)");
}

#[test]
fn metrics_performance_category_very_good() {
    let m = IncrementalMetrics {
        parse_time_micros: 200,
        ..Default::default()
    };
    assert_eq!(m.performance_category(), "Very Good (<500µs)");
}

#[test]
fn metrics_performance_category_good() {
    let m = IncrementalMetrics {
        parse_time_micros: 800,
        ..Default::default()
    };
    assert_eq!(m.performance_category(), "Good (<1ms)");
}

#[test]
fn metrics_performance_category_acceptable() {
    let m = IncrementalMetrics {
        parse_time_micros: 3000,
        ..Default::default()
    };
    assert_eq!(m.performance_category(), "Acceptable (<5ms)");
}

#[test]
fn metrics_performance_category_needs_optimization() {
    let m = IncrementalMetrics {
        parse_time_micros: 10_000,
        ..Default::default()
    };
    assert_eq!(m.performance_category(), "Needs Optimization (>5ms)");
}

// =========================================================================
// AdvancedReuseAnalyzer tests
// =========================================================================

#[test]
fn advanced_analyzer_new() {
    let analyzer = AdvancedReuseAnalyzer::new();
    assert_eq!(analyzer.analysis_stats.nodes_analyzed, 0);
}

#[test]
fn advanced_analyzer_default() {
    let analyzer = AdvancedReuseAnalyzer::default();
    assert_eq!(analyzer.analysis_stats.structural_matches, 0);
}

#[test]
fn advanced_analyzer_with_config() {
    let config = ReuseConfig::default();
    let analyzer = AdvancedReuseAnalyzer::with_config(config);
    assert_eq!(analyzer.analysis_stats.nodes_analyzed, 0);
}

#[test]
fn advanced_analyzer_analyze_identical_trees() {
    let mut analyzer = AdvancedReuseAnalyzer::new();
    let tree1 = parse_ok("my $x = 1;");
    let tree2 = parse_ok("my $x = 1;");
    if let (Ok(t1), Ok(t2)) = (tree1, tree2) {
        let edits = perl_incremental_parsing::edit::EditSet::new();
        let config = ReuseConfig::default();
        let result = analyzer.analyze_reuse_opportunities(&t1, &t2, &edits, &config);
        assert!(result.total_old_nodes > 0);
        assert!(result.total_new_nodes > 0);
    }
}

#[test]
fn advanced_analyzer_analyze_different_trees() {
    let mut analyzer = AdvancedReuseAnalyzer::new();
    let tree1 = parse_ok("my $x = 1;");
    let tree2 = parse_ok("my $y = 2;");
    if let (Ok(t1), Ok(t2)) = (tree1, tree2) {
        let edits = perl_incremental_parsing::edit::EditSet::new();
        let config = ReuseConfig::default();
        let result = analyzer.analyze_reuse_opportunities(&t1, &t2, &edits, &config);
        let summary = result.performance_summary();
        assert!(!summary.is_empty());
    }
}

#[test]
fn reuse_config_default_values() {
    let config = ReuseConfig::default();
    assert!((config.min_confidence - 0.75).abs() < f64::EPSILON);
    assert_eq!(config.max_position_shift, 1000);
    assert!(config.aggressive_structural_matching);
    assert!(config.enable_content_reuse);
    assert_eq!(config.max_analysis_depth, 10);
}

#[test]
fn reuse_analysis_result_meets_efficiency_target() {
    let mut analyzer = AdvancedReuseAnalyzer::new();
    let tree1 = parse_ok("my $x = 1;");
    let tree2 = parse_ok("my $x = 1;");
    if let (Ok(t1), Ok(t2)) = (tree1, tree2) {
        let edits = perl_incremental_parsing::edit::EditSet::new();
        let config = ReuseConfig::default();
        let result = analyzer.analyze_reuse_opportunities(&t1, &t2, &edits, &config);
        // Checking the API works regardless of actual value
        let _ = result.meets_efficiency_target(0.0);
        let _ = result.meets_efficiency_target(100.0);
    }
}

// =========================================================================
// Integration: lsp_pos_to_byte / byte_to_lsp_pos
// =========================================================================

#[test]
fn lsp_pos_to_byte_start_of_document() {
    let rope = Rope::from_str("Hello\nWorld\n");
    assert_eq!(lsp_pos_to_byte(&rope, 0, 0), 0);
}

#[test]
fn lsp_pos_to_byte_second_line() {
    let rope = Rope::from_str("Hello\nWorld\n");
    assert_eq!(lsp_pos_to_byte(&rope, 1, 0), 6);
}

#[test]
fn lsp_pos_to_byte_middle_of_line() {
    let rope = Rope::from_str("Hello\nWorld\n");
    assert_eq!(lsp_pos_to_byte(&rope, 1, 3), 9);
}

#[test]
fn lsp_pos_to_byte_past_end_of_lines() {
    let rope = Rope::from_str("Hi");
    let result = lsp_pos_to_byte(&rope, 100, 0);
    assert_eq!(result, rope.len_bytes());
}

#[test]
fn byte_to_lsp_pos_start() {
    let rope = Rope::from_str("Hello\nWorld\n");
    assert_eq!(byte_to_lsp_pos(&rope, 0), (0, 0));
}

#[test]
fn byte_to_lsp_pos_second_line() {
    let rope = Rope::from_str("Hello\nWorld\n");
    assert_eq!(byte_to_lsp_pos(&rope, 6), (1, 0));
}

#[test]
fn byte_to_lsp_pos_middle() {
    let rope = Rope::from_str("Hello\nWorld\n");
    assert_eq!(byte_to_lsp_pos(&rope, 9), (1, 3));
}

#[test]
fn byte_to_lsp_pos_past_end_clamped() {
    let rope = Rope::from_str("Hi");
    let (line, _col) = byte_to_lsp_pos(&rope, 1000);
    // Should clamp to end of document
    assert!(line <= 1);
}

// =========================================================================
// DocumentParser tests
// =========================================================================

#[test]
fn document_parser_full_mode() {
    let config = IncrementalConfig {
        enabled: false,
        ..Default::default()
    };
    let result = DocumentParser::new("my $x = 1;".to_string(), &config);
    assert!(result.is_ok());
    if let Ok(dp) = result {
        assert_eq!(dp.content(), "my $x = 1;");
        assert!(dp.ast().is_some());
        assert!(dp.metrics().is_none());
    }
}

#[test]
fn document_parser_incremental_mode() {
    let config = IncrementalConfig {
        enabled: true,
        target_parse_time_ms: 1.0,
        max_cache_size: 100,
    };
    let result = DocumentParser::new("my $x = 1;".to_string(), &config);
    assert!(result.is_ok());
    if let Ok(dp) = result {
        assert_eq!(dp.content(), "my $x = 1;");
        assert!(dp.ast().is_some());
        assert!(dp.metrics().is_some());
    }
}

#[test]
fn document_parser_apply_full_document_change() {
    let config = IncrementalConfig {
        enabled: false,
        ..Default::default()
    };
    let result = DocumentParser::new("my $x = 1;".to_string(), &config);
    if let Ok(mut dp) = result {
        let change = serde_json::json!({ "text": "my $y = 2;" });
        let _ = dp.apply_changes(&[change], &config);
        assert_eq!(dp.content(), "my $y = 2;");
    }
}

// =========================================================================
// IncrementalConfig tests
// =========================================================================

#[test]
fn incremental_config_default() {
    let config = IncrementalConfig::default();
    assert!((config.target_parse_time_ms - 1.0).abs() < f64::EPSILON);
    assert_eq!(config.max_cache_size, 10000);
}

// =========================================================================
// Cross-module integration tests
// =========================================================================

#[test]
fn round_trip_parse_edit_reparse_via_state() -> Result<(), Box<dyn std::error::Error>> {
    let mut state = IncrementalState::new("my $x = 1; my $y = 2;".to_string());
    let edit = Edit {
        start_byte: 8,
        old_end_byte: 9,
        new_end_byte: 10,
        new_text: "42".to_string(),
    };
    let result = apply_edits(&mut state, &[edit])?;
    assert!(state.source.contains("42"));
    assert!(result.reparsed_bytes > 0);
    Ok(())
}

#[test]
fn edit_set_apply_then_parse() {
    let mut set = IncrementalEditSet::new();
    set.add(IncrementalEdit::new(8, 9, "42".to_string()));
    let new_source = set.apply_to_string("my $x = 1;");
    assert_eq!(new_source, "my $x = 42;");
    let result = parse_ok(&new_source);
    assert!(result.is_ok());
}

#[test]
fn v2_parser_edit_then_reparse_preserves_program_structure() {
    let mut parser = IncrementalParserV2::new();
    let r1 = parser.parse("my $x = 42;");
    assert!(r1.is_ok());

    parser.edit(make_edit(8, 10, 12));
    let r2 = parser.parse("my $x = 9999;");
    assert!(r2.is_ok());

    if let Ok(tree) = r2 {
        assert!(matches!(tree.kind, NodeKind::Program { .. }));
    }
}

#[test]
fn simple_edit_to_original_edit_conversion() {
    let se = SimpleEdit {
        start: 5,
        end: 10,
        new_text: "hello".to_string(),
    };
    let original = se.to_original_edit();
    assert_eq!(original.start_byte, 5);
    assert_eq!(original.old_end_byte, 10);
    assert_eq!(original.new_end_byte, 10);
}

#[test]
fn incremental_state_rope_consistent_with_source() {
    let src = "my $x = 1;\nmy $y = 2;\n";
    let state = IncrementalState::new(src.to_string());
    let rope_text: String = state.rope.chars().collect();
    assert_eq!(rope_text, src);
}

#[test]
fn line_index_and_lsp_pos_consistency() {
    let text = "abc\ndef\n";
    let idx = LineIndex::new(text);
    let rope = Rope::from_str(text);

    // line 1, col 0 => byte 4
    assert_eq!(idx.position_to_byte(1, 0), Some(4));
    assert_eq!(lsp_pos_to_byte(&rope, 1, 0), 4);
}

#[test]
fn checkpointed_parser_preserves_program_kind() {
    let mut parser = CheckpointedIncrementalParser::new();
    let result = parser.parse("my $x = 1;".to_string());
    if let Ok(tree) = result {
        assert!(matches!(tree.kind, NodeKind::Program { .. }));
    }
}

#[test]
fn incremental_document_version_increments() {
    let doc = IncrementalDocument::new("my $x = 1;".to_string());
    if let Ok(mut d) = doc {
        assert_eq!(d.version, 0);
        let _ = d.apply_edit(IncrementalEdit::new(8, 9, "2".to_string()));
        assert_eq!(d.version, 1);
        let _ = d.apply_edit(IncrementalEdit::new(8, 9, "3".to_string()));
        assert_eq!(d.version, 2);
    }
}
