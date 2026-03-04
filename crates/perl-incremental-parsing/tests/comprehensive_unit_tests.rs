//! Comprehensive unit tests for perl-incremental-parsing.
//!
//! Covers:
//! - LineIndex: byte↔position mapping
//! - IncrementalState: creation, checkpoint lookup, edit application
//! - Edit: LSP change conversion, full-document changes
//! - apply_edits / full_reparse: single & multiple edits, fallback paths
//! - IncrementalEdit / IncrementalEditSet: arithmetic, overlap, application
//! - IncrementalDocument: single & batch edits, subtree cache, metrics
//! - SimpleIncrementalParser: initial parse, incremental reparse, structural changes
//! - CheckpointedIncrementalParser: parsing, edit application, cache management
//! - IncrementalParserV2: value edits, whitespace edits, advanced reuse
//! - IncrementalTree: node-map lookup
//! - IncrementalMetrics: efficiency calculation
//! - Integration helpers: LSP position conversion, DocumentParser, IncrementalConfig

use perl_incremental_parsing::incremental::incremental_advanced_reuse::{
    AdvancedReuseAnalyzer, ReuseConfig,
};
use perl_incremental_parsing::incremental::incremental_checkpoint::{
    CheckpointedIncrementalParser, SimpleEdit,
};
use perl_incremental_parsing::incremental::incremental_document::{
    IncrementalDocument, SubtreeCache,
};
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
    Edit, IncrementalState, LexCheckpoint, LineIndex, ScopeSnapshot, apply_edits,
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

// =========================================================================
// LineIndex
// =========================================================================

#[test]
fn line_index_empty_string() -> Result<(), Box<dyn std::error::Error>> {
    let li = LineIndex::new("");
    let (line, col) = li.byte_to_position(0);
    assert_eq!(line, 0);
    assert_eq!(col, 0);
    let byte = li.position_to_byte(0, 0);
    assert_eq!(byte, Some(0));
    Ok(())
}

#[test]
fn line_index_single_line() -> Result<(), Box<dyn std::error::Error>> {
    let li = LineIndex::new("hello");
    assert_eq!(li.byte_to_position(0), (0, 0));
    assert_eq!(li.byte_to_position(3), (0, 3));
    assert_eq!(li.position_to_byte(0, 3), Some(3));
    Ok(())
}

#[test]
fn line_index_multiline() -> Result<(), Box<dyn std::error::Error>> {
    let text = "abc\ndef\nghi";
    let li = LineIndex::new(text);

    // Start of second line
    assert_eq!(li.byte_to_position(4), (1, 0));
    // Middle of third line
    assert_eq!(li.byte_to_position(9), (2, 1));

    assert_eq!(li.position_to_byte(1, 0), Some(4));
    assert_eq!(li.position_to_byte(2, 1), Some(9));
    Ok(())
}

#[test]
fn line_index_out_of_range_line() -> Result<(), Box<dyn std::error::Error>> {
    let li = LineIndex::new("a\nb");
    // Line 99 does not exist
    assert_eq!(li.position_to_byte(99, 0), None);
    Ok(())
}

// =========================================================================
// ScopeSnapshot / ParseCheckpoint / LexCheckpoint — construction smoke
// =========================================================================

#[test]
fn scope_snapshot_default() -> Result<(), Box<dyn std::error::Error>> {
    let ss = ScopeSnapshot::default();
    assert!(ss.package_name.is_empty());
    assert!(ss.locals.is_empty());
    assert!(ss.our_vars.is_empty());
    assert!(ss.parent_isa.is_empty());
    Ok(())
}

#[test]
fn lex_checkpoint_clone() -> Result<(), Box<dyn std::error::Error>> {
    let cp =
        LexCheckpoint { byte: 42, mode: perl_lexer::LexerMode::ExpectTerm, line: 1, column: 5 };
    let cp2 = cp;
    assert_eq!(cp2.byte, 42);
    assert_eq!(cp2.line, 1);
    assert_eq!(cp2.column, 5);
    Ok(())
}

// =========================================================================
// IncrementalState — creation & checkpoint lookup
// =========================================================================

#[test]
fn incremental_state_new_simple() -> Result<(), Box<dyn std::error::Error>> {
    let src = "my $x = 1;";
    let state = IncrementalState::new(src.to_string());

    assert_eq!(state.source, src);
    assert!(!state.tokens.is_empty());
    assert!(!state.lex_checkpoints.is_empty());
    // The rope should agree with the source length.
    assert_eq!(state.rope.len_bytes(), src.len());
    Ok(())
}

#[test]
fn incremental_state_new_empty_source() -> Result<(), Box<dyn std::error::Error>> {
    // Empty source should not panic.
    let state = IncrementalState::new(String::new());
    assert!(state.source.is_empty());
    assert!(state.tokens.is_empty());
    Ok(())
}

#[test]
fn incremental_state_new_invalid_perl() -> Result<(), Box<dyn std::error::Error>> {
    // Invalid Perl produces an Error AST node — should not panic.
    let state = IncrementalState::new("}{}{".to_string());
    // We just verify construction succeeded.
    assert!(!state.source.is_empty());
    Ok(())
}

#[test]
fn find_lex_checkpoint_returns_nearest() -> Result<(), Box<dyn std::error::Error>> {
    let src = "my $x = 1; my $y = 2;";
    let state = IncrementalState::new(src.to_string());

    // Checkpoint at byte 0 should always exist.
    let cp = state.find_lex_checkpoint(0);
    assert!(cp.is_some());
    assert_eq!(cp.map(|c| c.byte), Some(0));

    // A checkpoint before a byte well inside the source.
    let cp2 = state.find_lex_checkpoint(15);
    assert!(cp2.is_some());
    assert!(cp2.map(|c| c.byte <= 15).unwrap_or(false));
    Ok(())
}

#[test]
fn find_parse_checkpoint_with_subroutine() -> Result<(), Box<dyn std::error::Error>> {
    let src = "sub foo { return 1; }";
    let state = IncrementalState::new(src.to_string());

    // Subroutines create parse checkpoints.
    assert!(!state.parse_checkpoints.is_empty());

    let cp = state.find_parse_checkpoint(5);
    assert!(cp.is_some());
    Ok(())
}

// =========================================================================
// Edit — construction & from_lsp_change
// =========================================================================

#[test]
fn edit_from_lsp_full_document_change() -> Result<(), Box<dyn std::error::Error>> {
    let old = "my $x = 1;";
    let li = LineIndex::new(old);
    let change = TextDocumentContentChangeEvent {
        range: None,
        range_length: None,
        text: "my $x = 2;".to_string(),
    };
    let edit = Edit::from_lsp_change(&change, &li, old);
    assert!(edit.is_some());
    let e = edit.unwrap();
    assert_eq!(e.start_byte, 0);
    assert_eq!(e.old_end_byte, old.len());
    assert_eq!(e.new_text, "my $x = 2;");
    Ok(())
}

#[test]
fn edit_from_lsp_range_change() -> Result<(), Box<dyn std::error::Error>> {
    let old = "my $x = 42;";
    let li = LineIndex::new(old);
    let change = TextDocumentContentChangeEvent {
        range: Some(LspRange {
            start: lsp_types::Position { line: 0, character: 8 },
            end: lsp_types::Position { line: 0, character: 10 },
        }),
        range_length: None,
        text: "99".to_string(),
    };
    let edit = Edit::from_lsp_change(&change, &li, old);
    assert!(edit.is_some());
    let e = edit.unwrap();
    assert_eq!(e.start_byte, 8);
    assert_eq!(e.old_end_byte, 10);
    assert_eq!(e.new_text, "99");
    Ok(())
}

// =========================================================================
// apply_edits — single edit, incremental path
// =========================================================================

#[test]
fn apply_single_small_edit() -> Result<(), Box<dyn std::error::Error>> {
    let src = "my $x = 42;";
    let mut state = IncrementalState::new(src.to_string());

    let edit =
        Edit { start_byte: 8, old_end_byte: 10, new_end_byte: 10, new_text: "99".to_string() };

    let result = apply_edits(&mut state, &[edit])?;
    assert!(!result.changed_ranges.is_empty());
    assert!(result.reparsed_bytes > 0);
    assert!(state.source.contains("99"));
    Ok(())
}

#[test]
fn apply_single_insertion_edit() -> Result<(), Box<dyn std::error::Error>> {
    let src = "my $x = 1;";
    let mut state = IncrementalState::new(src.to_string());

    // Insert "00" after "1" making it "100"
    let edit =
        Edit { start_byte: 9, old_end_byte: 9, new_end_byte: 11, new_text: "00".to_string() };

    let result = apply_edits(&mut state, &[edit])?;
    assert!(state.source.contains("100"));
    assert!(!result.changed_ranges.is_empty());
    Ok(())
}

#[test]
fn apply_single_deletion_edit() -> Result<(), Box<dyn std::error::Error>> {
    let src = "my $xyz = 1;";
    let mut state = IncrementalState::new(src.to_string());

    // Delete "yz" from "$xyz" -> "$x"
    let edit = Edit { start_byte: 5, old_end_byte: 7, new_end_byte: 5, new_text: String::new() };

    let result = apply_edits(&mut state, &[edit])?;
    assert!(state.source.contains("$x ="));
    assert!(!result.changed_ranges.is_empty());
    Ok(())
}

// =========================================================================
// apply_edits — large edit falls back to full reparse
// =========================================================================

#[test]
fn apply_large_edit_triggers_full_reparse() -> Result<(), Box<dyn std::error::Error>> {
    let src = "my $x = 1;";
    let mut state = IncrementalState::new(src.to_string());

    // A multi-line edit > 10 newlines triggers full reparse.
    let big_text = "my $a = 1;\n".repeat(15);
    let edit = Edit {
        start_byte: 0,
        old_end_byte: src.len(),
        new_end_byte: big_text.len(),
        new_text: big_text,
    };

    let result = apply_edits(&mut state, &[edit])?;
    // Full reparse covers the whole document.
    assert_eq!(result.changed_ranges.len(), 1);
    assert_eq!(result.changed_ranges[0], 0..state.source.len());
    Ok(())
}

// =========================================================================
// apply_edits — multiple edits fall back to full reparse
// =========================================================================

#[test]
fn apply_multiple_edits() -> Result<(), Box<dyn std::error::Error>> {
    let src = "my $x = 1; my $y = 2;";
    let mut state = IncrementalState::new(src.to_string());

    let e1 = Edit { start_byte: 8, old_end_byte: 9, new_end_byte: 10, new_text: "10".to_string() };
    let e2 =
        Edit { start_byte: 19, old_end_byte: 20, new_end_byte: 21, new_text: "20".to_string() };

    let result = apply_edits(&mut state, &[e1, e2])?;
    // Multiple edits -> full reparse path.
    assert!(!result.changed_ranges.is_empty());
    Ok(())
}

// =========================================================================
// apply_edits — oversized edit exceeding MAX_EDIT_SIZE
// =========================================================================

#[test]
fn apply_edits_exceeding_max_size() -> Result<(), Box<dyn std::error::Error>> {
    let src = "my $x = 1;";
    let mut state = IncrementalState::new(src.to_string());

    // Create an edit larger than 64 KB.
    let huge = "a".repeat(65 * 1024);
    let edit =
        Edit { start_byte: 0, old_end_byte: src.len(), new_end_byte: huge.len(), new_text: huge };

    let result = apply_edits(&mut state, &[edit])?;
    assert_eq!(result.changed_ranges.len(), 1);
    Ok(())
}

// =========================================================================
// IncrementalEdit & IncrementalEditSet
// =========================================================================

#[test]
fn incremental_edit_new_end_byte() -> Result<(), Box<dyn std::error::Error>> {
    let e = IncrementalEdit::new(5, 10, "hello".to_string());
    assert_eq!(e.new_end_byte(), 10);
    assert_eq!(e.byte_shift(), 0);
    Ok(())
}

#[test]
fn incremental_edit_insertion_shift() -> Result<(), Box<dyn std::error::Error>> {
    let e = IncrementalEdit::new(5, 5, "inserted".to_string());
    assert_eq!(e.new_end_byte(), 13);
    assert_eq!(e.byte_shift(), 8);
    Ok(())
}

#[test]
fn incremental_edit_deletion_shift() -> Result<(), Box<dyn std::error::Error>> {
    let e = IncrementalEdit::new(5, 15, String::new());
    assert_eq!(e.new_end_byte(), 5);
    assert_eq!(e.byte_shift(), -10);
    Ok(())
}

#[test]
fn incremental_edit_overlaps() -> Result<(), Box<dyn std::error::Error>> {
    let e = IncrementalEdit::new(10, 20, "x".to_string());
    assert!(e.overlaps(15, 25)); // partial overlap
    assert!(e.overlaps(5, 15)); // partial overlap from left
    assert!(!e.overlaps(20, 30)); // no overlap (adjacent)
    assert!(!e.overlaps(0, 10)); // no overlap (adjacent from left)
    Ok(())
}

#[test]
fn incremental_edit_is_before_after() -> Result<(), Box<dyn std::error::Error>> {
    let e = IncrementalEdit::new(10, 20, "x".to_string());
    assert!(e.is_before(20));
    assert!(e.is_before(25));
    assert!(!e.is_before(15));
    assert!(e.is_after(10));
    assert!(e.is_after(5));
    assert!(!e.is_after(15));
    Ok(())
}

#[test]
fn incremental_edit_with_positions() -> Result<(), Box<dyn std::error::Error>> {
    let sp = Position::new(10, 1, 5);
    let ep = Position::new(20, 1, 15);
    let e = IncrementalEdit::with_positions(10, 20, "repl".to_string(), sp, ep);
    assert_eq!(e.start_byte, 10);
    assert_eq!(e.old_end_byte, 20);
    assert_eq!(e.new_end_byte(), 14);
    Ok(())
}

#[test]
fn incremental_edit_set_sort_and_apply() -> Result<(), Box<dyn std::error::Error>> {
    let mut es = IncrementalEditSet::new();
    assert!(es.is_empty());
    assert_eq!(es.total_byte_shift(), 0);

    es.add(IncrementalEdit::new(6, 11, "Perl".to_string()));
    es.add(IncrementalEdit::new(0, 5, "Hello".to_string()));

    es.sort();
    assert_eq!(es.edits[0].start_byte, 0);

    es.sort_reverse();
    assert_eq!(es.edits[0].start_byte, 6);

    let result = es.apply_to_string("hello world");
    assert_eq!(result, "Hello Perl");
    Ok(())
}

#[test]
fn incremental_edit_set_apply_empty() -> Result<(), Box<dyn std::error::Error>> {
    let es = IncrementalEditSet::new();
    let result = es.apply_to_string("unchanged");
    assert_eq!(result, "unchanged");
    Ok(())
}

// =========================================================================
// IncrementalDocument
// =========================================================================

#[test]
fn incremental_document_new() -> Result<(), Box<dyn std::error::Error>> {
    let doc = IncrementalDocument::new("my $x = 1;".to_string())?;
    assert_eq!(doc.version, 0);
    assert_eq!(doc.source, "my $x = 1;");
    assert!(doc.metrics().last_parse_time_ms >= 0.0);
    Ok(())
}

#[test]
fn incremental_document_apply_single_edit() -> Result<(), Box<dyn std::error::Error>> {
    let src = "my $x = 42; my $y = 100;";
    let mut doc = IncrementalDocument::new(src.to_string())?;

    // Change "42" to "43"
    let pos = src.find("42").ok_or("expected '42'")?;
    let edit = IncrementalEdit::new(pos, pos + 2, "43".to_string());
    doc.apply_edit(edit)?;

    assert_eq!(doc.version, 1);
    assert!(doc.source.contains("43"));
    Ok(())
}

#[test]
fn incremental_document_apply_batch_edits() -> Result<(), Box<dyn std::error::Error>> {
    let src = "my $a = 10; my $b = 20;";
    let mut doc = IncrementalDocument::new(src.to_string())?;

    let mut edits = IncrementalEditSet::new();
    let p1 = src.find("10").ok_or("expected '10'")?;
    edits.add(IncrementalEdit::new(p1, p1 + 2, "15".to_string()));

    let p2 = src.find("20").ok_or("expected '20'")?;
    edits.add(IncrementalEdit::new(p2, p2 + 2, "25".to_string()));

    doc.apply_edits(&edits)?;
    assert_eq!(doc.version, 1);
    assert!(doc.source.contains("15"));
    assert!(doc.source.contains("25"));
    Ok(())
}

#[test]
fn incremental_document_tree_and_text() -> Result<(), Box<dyn std::error::Error>> {
    let doc = IncrementalDocument::new("my $x = 1;".to_string())?;
    assert_eq!(doc.text(), "my $x = 1;");
    // tree() should return a valid root.
    let _tree = doc.tree();
    Ok(())
}

#[test]
fn incremental_document_cache_max_size() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = IncrementalDocument::new("my $x = 1;".to_string())?;
    doc.set_cache_max_size(5);
    // After reducing cache size, eviction should not panic.
    Ok(())
}

// =========================================================================
// SubtreeCache
// =========================================================================

#[test]
fn subtree_cache_default() -> Result<(), Box<dyn std::error::Error>> {
    let cache = SubtreeCache::default();
    assert!(cache.by_content.is_empty());
    assert!(cache.by_range.is_empty());
    assert!(cache.lru.is_empty());
    Ok(())
}

// =========================================================================
// SimpleIncrementalParser
// =========================================================================

#[test]
fn simple_incremental_parser_initial_parse() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = SimpleIncrementalParser::new();
    let tree = parser.parse("my $x = 42;")?;

    assert!(parser.reparsed_nodes > 0);
    assert_eq!(parser.reused_nodes, 0);
    if let NodeKind::Program { statements } = &tree.kind {
        assert!(!statements.is_empty());
    }
    Ok(())
}

#[test]
fn simple_incremental_parser_value_edit_reuse() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = SimpleIncrementalParser::new();
    let _t1 = parser.parse("my $x = 42;")?;

    // Edit: "42" (bytes 8..10) -> "4242" (bytes 8..12)
    parser.edit(perl_incremental_parsing::edit::Edit::new(
        8,
        10,
        12,
        Position::new(8, 1, 9),
        Position::new(10, 1, 11),
        Position::new(12, 1, 13),
    ));

    let _t2 = parser.parse("my $x = 4242;")?;
    assert!(parser.reused_nodes > 0);
    Ok(())
}

#[test]
fn simple_incremental_parser_structural_change() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = SimpleIncrementalParser::new();
    let _t1 = parser.parse("if (1) { print 1; }")?;

    // Structural edit touching the if-block triggers full reparse.
    parser.edit(perl_incremental_parsing::edit::Edit::new(
        0,
        19,
        21,
        Position::new(0, 1, 1),
        Position::new(19, 1, 20),
        Position::new(21, 1, 22),
    ));

    let _t2 = parser.parse("while (1) { print 1; }")?;
    // After structural change the parser falls back; reparsed_nodes > 0.
    assert!(parser.reparsed_nodes > 0);
    Ok(())
}

#[test]
fn simple_incremental_parser_default_trait() -> Result<(), Box<dyn std::error::Error>> {
    let parser = SimpleIncrementalParser::default();
    assert_eq!(parser.reused_nodes, 0);
    assert_eq!(parser.reparsed_nodes, 0);
    Ok(())
}

// =========================================================================
// CheckpointedIncrementalParser
// =========================================================================

#[test]
fn checkpointed_parser_initial_parse() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = CheckpointedIncrementalParser::new();
    let tree = parser.parse("my $x = 42;\nmy $y = 99;\n".to_string())?;

    assert_eq!(parser.stats().total_parses, 1);
    assert_eq!(parser.stats().incremental_parses, 0);

    if let NodeKind::Program { statements } = &tree.kind {
        assert!(statements.len() >= 2);
    }
    Ok(())
}

#[test]
fn checkpointed_parser_apply_edit() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = CheckpointedIncrementalParser::new();
    let _ = parser.parse("my $x = 42;\nmy $y = 99;\n".to_string())?;

    let edit = SimpleEdit { start: 8, end: 10, new_text: "4242".to_string() };
    let tree2 = parser.apply_edit(&edit)?;

    assert_eq!(parser.stats().total_parses, 2);
    assert_eq!(parser.stats().incremental_parses, 1);

    if let NodeKind::Program { statements } = &tree2.kind {
        assert!(statements.len() >= 2);
    }
    Ok(())
}

#[test]
fn checkpointed_parser_multiple_edits() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = CheckpointedIncrementalParser::new();
    let source = "my $x = 1;\n".repeat(20);
    let _ = parser.parse(source)?;

    let e1 = SimpleEdit { start: 8, end: 9, new_text: "42".to_string() };
    let _ = parser.apply_edit(&e1)?;

    let e2 = SimpleEdit { start: 20, end: 21, new_text: "99".to_string() };
    let _ = parser.apply_edit(&e2)?;

    assert_eq!(parser.stats().incremental_parses, 2);
    assert!(parser.stats().tokens_relexed > 0);
    Ok(())
}

#[test]
fn checkpointed_parser_clear_caches() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = CheckpointedIncrementalParser::new();
    let _ = parser.parse("my $x = 1;".to_string())?;
    parser.clear_caches();
    // After clearing, a subsequent edit should still work.
    let edit = SimpleEdit { start: 8, end: 9, new_text: "99".to_string() };
    let _ = parser.apply_edit(&edit)?;
    Ok(())
}

#[test]
fn checkpointed_parser_default_trait() -> Result<(), Box<dyn std::error::Error>> {
    let parser = CheckpointedIncrementalParser::default();
    assert_eq!(parser.stats().total_parses, 0);
    Ok(())
}

// =========================================================================
// IncrementalParserV2
// =========================================================================

#[test]
fn v2_parser_initial_parse() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = IncrementalParserV2::new();
    let tree = parser.parse("my $x = 42;")?;

    assert!(parser.reparsed_nodes > 0);
    assert_eq!(parser.reused_nodes, 0);
    if let NodeKind::Program { statements } = &tree.kind {
        assert!(!statements.is_empty());
    }
    Ok(())
}

#[test]
fn v2_parser_value_edit_incremental() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = IncrementalParserV2::new();
    let _ = parser.parse("my $x = 42;")?;

    // Edit "42" -> "99"
    parser.edit(perl_incremental_parsing::edit::Edit::new(
        8,
        10,
        10,
        Position::new(8, 1, 9),
        Position::new(10, 1, 11),
        Position::new(10, 1, 11),
    ));

    let _ = parser.parse("my $x = 99;")?;
    // Should achieve some level of reuse.
    assert!(parser.reused_nodes > 0 || parser.reparsed_nodes > 0);
    Ok(())
}

#[test]
fn v2_parser_with_custom_reuse_config() -> Result<(), Box<dyn std::error::Error>> {
    let config = ReuseConfig {
        min_confidence: 0.5,
        max_position_shift: 2000,
        aggressive_structural_matching: false,
        enable_content_reuse: true,
        max_analysis_depth: 5,
    };
    let mut parser = IncrementalParserV2::with_reuse_config(config);
    let _ = parser.parse("my $x = 42;")?;
    assert!(parser.reparsed_nodes > 0);
    Ok(())
}

#[test]
fn v2_parser_empty_source() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = IncrementalParserV2::new();
    // Empty source should parse to a program with no statements.
    let tree = parser.parse("")?;
    if let NodeKind::Program { statements } = &tree.kind {
        assert!(statements.is_empty());
    }
    Ok(())
}

// =========================================================================
// IncrementalTree — node map lookup
// =========================================================================

#[test]
fn incremental_tree_find_containing_node() -> Result<(), Box<dyn std::error::Error>> {
    let root = parse_ok("my $x = 42;")?;
    let tree = IncrementalTree::new(root, "my $x = 42;".to_string());

    // The root program node should contain byte 0..11.
    let node = tree.find_containing_node(0, 11);
    assert!(node.is_some());

    // A byte range inside the tree should also find something.
    let inner = tree.find_containing_node(8, 10);
    assert!(inner.is_some());
    Ok(())
}

#[test]
fn incremental_tree_find_outside_range() -> Result<(), Box<dyn std::error::Error>> {
    let root = parse_ok("my $x = 1;")?;
    let tree = IncrementalTree::new(root, "my $x = 1;".to_string());

    // Looking for a range completely outside the tree.
    let node = tree.find_containing_node(100, 200);
    assert!(node.is_none());
    Ok(())
}

// =========================================================================
// IncrementalMetrics
// =========================================================================

#[test]
fn metrics_efficiency_percentage() -> Result<(), Box<dyn std::error::Error>> {
    let mut m = IncrementalMetrics::new();
    // Zero nodes -> 0%
    assert!((m.efficiency_percentage() - 0.0).abs() < f64::EPSILON);

    m.nodes_reused = 80;
    m.nodes_reparsed = 20;
    assert!((m.efficiency_percentage() - 80.0).abs() < f64::EPSILON);
    Ok(())
}

#[test]
fn metrics_sub_millisecond() -> Result<(), Box<dyn std::error::Error>> {
    let mut m = IncrementalMetrics::new();
    m.parse_time_micros = 500;
    assert!(m.is_sub_millisecond());
    m.parse_time_micros = 1500;
    assert!(!m.is_sub_millisecond());
    Ok(())
}

#[test]
fn metrics_performance_category() -> Result<(), Box<dyn std::error::Error>> {
    let mut m = IncrementalMetrics::new();
    m.parse_time_micros = 50;
    assert_eq!(m.performance_category(), "Excellent (<100µs)");
    m.parse_time_micros = 300;
    assert_eq!(m.performance_category(), "Very Good (<500µs)");
    m.parse_time_micros = 800;
    assert_eq!(m.performance_category(), "Good (<1ms)");
    m.parse_time_micros = 3000;
    assert_eq!(m.performance_category(), "Acceptable (<5ms)");
    m.parse_time_micros = 10000;
    assert_eq!(m.performance_category(), "Needs Optimization (>5ms)");
    Ok(())
}

// =========================================================================
// AdvancedReuseAnalyzer
// =========================================================================

#[test]
fn advanced_reuse_analyzer_default() -> Result<(), Box<dyn std::error::Error>> {
    let analyzer = AdvancedReuseAnalyzer::default();
    assert_eq!(analyzer.analysis_stats.nodes_analyzed, 0);
    Ok(())
}

#[test]
fn advanced_reuse_analyzer_with_config() -> Result<(), Box<dyn std::error::Error>> {
    let config = ReuseConfig {
        min_confidence: 0.9,
        max_position_shift: 500,
        aggressive_structural_matching: false,
        enable_content_reuse: false,
        max_analysis_depth: 3,
    };
    let analyzer = AdvancedReuseAnalyzer::with_config(config);
    assert_eq!(analyzer.analysis_stats.nodes_analyzed, 0);
    Ok(())
}

#[test]
fn advanced_reuse_analyzer_analyze() -> Result<(), Box<dyn std::error::Error>> {
    let old = parse_ok("my $x = 42;")?;
    let new = parse_ok("my $x = 99;")?;

    let mut edits = perl_incremental_parsing::edit::EditSet::new();
    edits.add(perl_incremental_parsing::edit::Edit::new(
        8,
        10,
        10,
        Position::new(8, 1, 9),
        Position::new(10, 1, 11),
        Position::new(10, 1, 11),
    ));

    let config = ReuseConfig::default();
    let mut analyzer = AdvancedReuseAnalyzer::new();
    let result = analyzer.analyze_reuse_opportunities(&old, &new, &edits, &config);

    assert!(result.total_old_nodes > 0);
    assert!(result.total_new_nodes > 0);
    Ok(())
}

// =========================================================================
// Integration helpers: lsp_pos_to_byte / byte_to_lsp_pos
// =========================================================================

#[test]
fn lsp_position_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let text = "Hello\nWorld\nFoo";
    let rope = Rope::from_str(text);

    // Start of document
    assert_eq!(lsp_pos_to_byte(&rope, 0, 0), 0);
    assert_eq!(byte_to_lsp_pos(&rope, 0), (0, 0));

    // Start of second line
    assert_eq!(lsp_pos_to_byte(&rope, 1, 0), 6);
    assert_eq!(byte_to_lsp_pos(&rope, 6), (1, 0));

    // Middle of third line
    assert_eq!(lsp_pos_to_byte(&rope, 2, 2), 14);
    assert_eq!(byte_to_lsp_pos(&rope, 14), (2, 2));
    Ok(())
}

#[test]
fn lsp_pos_beyond_end() -> Result<(), Box<dyn std::error::Error>> {
    let rope = Rope::from_str("hi");
    // Line beyond document end should clamp to len_bytes.
    let byte = lsp_pos_to_byte(&rope, 99, 0);
    assert_eq!(byte, rope.len_bytes());
    Ok(())
}

#[test]
fn byte_to_lsp_pos_clamped() -> Result<(), Box<dyn std::error::Error>> {
    let rope = Rope::from_str("hi");
    // Byte beyond end should clamp.
    let (line, col) = byte_to_lsp_pos(&rope, 9999);
    assert_eq!(line, 0);
    assert_eq!(col, 2);
    Ok(())
}

// =========================================================================
// IncrementalConfig defaults
// =========================================================================

#[test]
fn incremental_config_defaults() -> Result<(), Box<dyn std::error::Error>> {
    let config = IncrementalConfig::default();
    assert!((config.target_parse_time_ms - 1.0).abs() < f64::EPSILON);
    assert_eq!(config.max_cache_size, 10000);
    Ok(())
}

// =========================================================================
// DocumentParser — Full mode
// =========================================================================

#[test]
fn document_parser_full_mode() -> Result<(), Box<dyn std::error::Error>> {
    let config = IncrementalConfig { enabled: false, ..IncrementalConfig::default() };
    let dp = DocumentParser::new("my $x = 1;".to_string(), &config)?;

    assert_eq!(dp.content(), "my $x = 1;");
    assert!(dp.ast().is_some());
    assert!(dp.metrics().is_none());
    Ok(())
}

#[test]
fn document_parser_full_mode_apply_changes() -> Result<(), Box<dyn std::error::Error>> {
    let config = IncrementalConfig { enabled: false, ..IncrementalConfig::default() };
    let mut dp = DocumentParser::new("my $x = 1;".to_string(), &config)?;

    let change = serde_json::json!({ "text": "my $x = 2;" });
    dp.apply_changes(&[change], &config)?;

    assert_eq!(dp.content(), "my $x = 2;");
    Ok(())
}

// =========================================================================
// DocumentParser — Incremental mode
// =========================================================================

#[test]
fn document_parser_incremental_mode() -> Result<(), Box<dyn std::error::Error>> {
    let config =
        IncrementalConfig { enabled: true, target_parse_time_ms: 1.0, max_cache_size: 1000 };
    let dp = DocumentParser::new("my $x = 1;".to_string(), &config)?;

    assert_eq!(dp.content(), "my $x = 1;");
    assert!(dp.ast().is_some());
    assert!(dp.metrics().is_some());
    Ok(())
}

#[test]
fn document_parser_incremental_full_text_change() -> Result<(), Box<dyn std::error::Error>> {
    let config =
        IncrementalConfig { enabled: true, target_parse_time_ms: 1.0, max_cache_size: 1000 };
    let mut dp = DocumentParser::new("my $x = 1;".to_string(), &config)?;

    // Full document replacement (no range).
    let change = serde_json::json!({ "text": "my $y = 99;" });
    dp.apply_changes(&[change], &config)?;

    assert_eq!(dp.content(), "my $y = 99;");
    Ok(())
}

#[test]
fn document_parser_incremental_range_change() -> Result<(), Box<dyn std::error::Error>> {
    let config =
        IncrementalConfig { enabled: true, target_parse_time_ms: 1.0, max_cache_size: 1000 };
    let mut dp = DocumentParser::new("my $x = 42;".to_string(), &config)?;

    // Incremental range change: replace "42" with "99".
    let change = serde_json::json!({
        "range": {
            "start": { "line": 0, "character": 8 },
            "end": { "line": 0, "character": 10 }
        },
        "text": "99"
    });
    dp.apply_changes(&[change], &config)?;

    assert!(dp.content().contains("99"));
    Ok(())
}

// =========================================================================
// ReuseConfig defaults
// =========================================================================

#[test]
fn reuse_config_default_values() -> Result<(), Box<dyn std::error::Error>> {
    let rc = ReuseConfig::default();
    assert!((rc.min_confidence - 0.75).abs() < f64::EPSILON);
    assert_eq!(rc.max_position_shift, 1000);
    assert!(rc.aggressive_structural_matching);
    assert!(rc.enable_content_reuse);
    assert_eq!(rc.max_analysis_depth, 10);
    Ok(())
}

// =========================================================================
// Edit — edge cases
// =========================================================================

#[test]
fn edit_noop_replacement() -> Result<(), Box<dyn std::error::Error>> {
    let src = "my $x = 42;";
    let mut state = IncrementalState::new(src.to_string());

    // Replace "42" with "42" — no actual change.
    let edit =
        Edit { start_byte: 8, old_end_byte: 10, new_end_byte: 10, new_text: "42".to_string() };
    let result = apply_edits(&mut state, &[edit])?;
    assert!(state.source.contains("42"));
    assert!(!result.changed_ranges.is_empty());
    Ok(())
}

// =========================================================================
// IncrementalState — multiline code with packages
// =========================================================================

#[test]
fn incremental_state_with_package() -> Result<(), Box<dyn std::error::Error>> {
    let src = "package Foo;\nsub bar { return 1; }\n";
    let state = IncrementalState::new(src.to_string());

    // Should have parse checkpoints for both package and subroutine.
    assert!(state.parse_checkpoints.len() >= 2);

    // Scope should capture the package name.
    let pkg_cp = state.parse_checkpoints.iter().find(|cp| cp.scope_snapshot.package_name == "Foo");
    assert!(pkg_cp.is_some());
    Ok(())
}

// =========================================================================
// SimpleEdit — to_original_edit conversion
// =========================================================================

#[test]
fn simple_edit_to_original_edit() -> Result<(), Box<dyn std::error::Error>> {
    let se = SimpleEdit { start: 5, end: 10, new_text: "hello".to_string() };
    let oe = se.to_original_edit();
    assert_eq!(oe.start_byte, 5);
    assert_eq!(oe.old_end_byte, 10);
    assert_eq!(oe.new_end_byte, 10); // 5 + 5
    Ok(())
}

// =========================================================================
// Larger integration: edit then verify AST shape
// =========================================================================

#[test]
fn integration_edit_preserves_statement_count() -> Result<(), Box<dyn std::error::Error>> {
    let src = "my $a = 1;\nmy $b = 2;\nmy $c = 3;\n";
    let mut state = IncrementalState::new(src.to_string());

    // Change "1" to "10"
    let edit =
        Edit { start_byte: 8, old_end_byte: 9, new_end_byte: 10, new_text: "10".to_string() };
    let _ = apply_edits(&mut state, &[edit])?;

    // Re-parse and verify three statements remain.
    let mut parser = Parser::new(&state.source);
    let ast = parser.parse()?;
    if let NodeKind::Program { statements } = &ast.kind {
        assert_eq!(statements.len(), 3);
    }
    Ok(())
}

#[test]
fn integration_checkpointed_parser_preserves_structure() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = CheckpointedIncrementalParser::new();
    let t1 = parser.parse("my $x = 42;\nmy $y = 99;\n".to_string())?;

    let edit = SimpleEdit { start: 8, end: 10, new_text: "4242".to_string() };
    let t2 = parser.apply_edit(&edit)?;

    // Both trees should have the same number of top-level statements.
    if let (NodeKind::Program { statements: s1 }, NodeKind::Program { statements: s2 }) =
        (&t1.kind, &t2.kind)
    {
        assert_eq!(s1.len(), s2.len());
    }
    Ok(())
}

// =========================================================================
// SymbolPriority ordering
// =========================================================================

#[test]
fn symbol_priority_ordering() -> Result<(), Box<dyn std::error::Error>> {
    use perl_incremental_parsing::incremental::incremental_document::SymbolPriority;

    assert!(SymbolPriority::Low < SymbolPriority::Medium);
    assert!(SymbolPriority::Medium < SymbolPriority::High);
    assert!(SymbolPriority::High < SymbolPriority::Critical);
    Ok(())
}

// =========================================================================
// LineIndex — CRLF handling
// =========================================================================

#[test]
fn line_index_crlf() -> Result<(), Box<dyn std::error::Error>> {
    let text = "abc\r\ndef";
    let li = LineIndex::new(text);
    // '\n' is at byte 4, so line 1 starts at byte 5.
    assert_eq!(li.byte_to_position(5), (1, 0));
    assert_eq!(li.position_to_byte(1, 0), Some(5));
    Ok(())
}

// =========================================================================
// IncrementalState — variable declarations track scope
// =========================================================================

#[test]
fn incremental_state_variable_scope_tracking() -> Result<(), Box<dyn std::error::Error>> {
    let src = "my $x = 1; my $y = 2;";
    let state = IncrementalState::new(src.to_string());

    // Tokens should include variable tokens.
    assert!(state.tokens.len() >= 4);
    Ok(())
}
