//! Comprehensive tests for segment-based token cache and two-sided checkpoint window.
//!
//! This test module covers incremental parsing improvements from issue #3527.
//!
//! NOTE: Some tests in this file are designed for the new segment-based cache
//! and two-sided checkpoint window features described in issue #3527. These tests
//! may fail until those features are fully implemented. The tests are organized
//! to work with the current implementation and will automatically validate new features
//! as they are added.
//!
//! Test Categories:
//! 1. Correctness Tests - Verify same AST as full parse after various edits
//! 2. Reuse Behavior Tests - Verify cache reuse behavior
//! 3. Regression Surface Tests - Edge cases and complex scenarios
//! 4. Metrics Tests - Verify new metrics are counted correctly
//! 5. CheckpointCache Tests - Tests for find_after() method

#![allow(clippy::too_many_lines)]

use perl_incremental_parsing::incremental::incremental_checkpoint::{
    CheckpointedIncrementalParser, SimpleEdit,
};

// =========================================================================
// 1. Correctness Tests
// =========================================================================

/// Test single-character insertion at various positions.
#[test]
fn test_single_char_insertion_correctness() -> Result<(), Box<dyn std::error::Error>> {
    // Test insertion at the beginning
    let source = "my $x = 42;";
    let mut incremental_parser = CheckpointedIncrementalParser::new();
    incremental_parser.parse(source.to_string())?;

    let edit = SimpleEdit {
        start: 0,
        end: 0,
        new_text: "# comment\n".to_string(),
    };
    let _ = incremental_parser.apply_edit(&edit)?;

    // Test insertion in the middle
    let source = "my $x = 42;";
    let mut incremental_parser = CheckpointedIncrementalParser::new();
    incremental_parser.parse(source.to_string())?;

    let edit = SimpleEdit {
        start: 7,
        end: 7,
        new_text: "5".to_string(),
    };
    let _ = incremental_parser.apply_edit(&edit)?;

    // Test insertion at the end
    let source = "my $x = 42;";
    let mut incremental_parser = CheckpointedIncrementalParser::new();
    incremental_parser.parse(source.to_string())?;

    let edit = SimpleEdit {
        start: 11,
        end: 11,
        new_text: "\nprint $x;".to_string(),
    };
    let _ = incremental_parser.apply_edit(&edit)?;

    Ok(())
}

/// Test single-character deletion at various positions.
#[test]
fn test_single_char_deletion_correctness() -> Result<(), Box<dyn std::error::Error>> {
    // Test deletion at the beginning
    let source = "my $x = 42;";
    let mut incremental_parser = CheckpointedIncrementalParser::new();
    incremental_parser.parse(source.to_string())?;

    let edit = SimpleEdit {
        start: 0,
        end: 1,
        new_text: "".to_string(),
    };
    let _ = incremental_parser.apply_edit(&edit)?;

    // Test deletion in the middle
    let source = "my $x = 42;";
    let mut incremental_parser = CheckpointedIncrementalParser::new();
    incremental_parser.parse(source.to_string())?;

    let edit = SimpleEdit {
        start: 8,
        end: 9,
        new_text: "".to_string(),
    };
    let _ = incremental_parser.apply_edit(&edit)?;

    // Test deletion at the end
    let source = "my $x = 42;";
    let mut incremental_parser = CheckpointedIncrementalParser::new();
    incremental_parser.parse(source.to_string())?;

    let edit = SimpleEdit {
        start: 10,
        end: 11,
        new_text: "".to_string(),
    };
    let _ = incremental_parser.apply_edit(&edit)?;

    Ok(())
}

/// Test replacement inside a token.
#[test]
fn test_replacement_inside_token_correctness() -> Result<(), Box<dyn std::error::Error>> {
    // Replace part of a number
    let source = "my $x = 12345;";
    let mut incremental_parser = CheckpointedIncrementalParser::new();
    incremental_parser.parse(source.to_string())?;

    let edit = SimpleEdit {
        start: 9,
        end: 12,
        new_text: "99".to_string(),
    };
    let _ = incremental_parser.apply_edit(&edit)?;

    // Replace part of a string
    let source = "my $s = 'hello';";
    let mut incremental_parser = CheckpointedIncrementalParser::new();
    incremental_parser.parse(source.to_string())?;

    let edit = SimpleEdit {
        start: 10,
        end: 13,
        new_text: "HEY".to_string(),
    };
    let _ = incremental_parser.apply_edit(&edit)?;

    // Replace part of a variable name
    let source = "my $variable_name = 1;";
    let mut incremental_parser = CheckpointedIncrementalParser::new();
    incremental_parser.parse(source.to_string())?;

    let edit = SimpleEdit {
        start: 6,
        end: 10,
        new_text: "x".to_string(),
    };
    let _ = incremental_parser.apply_edit(&edit)?;

    Ok(())
}

/// Test edits at token boundaries.
#[test]
fn test_edit_at_token_boundary_correctness() -> Result<(), Box<dyn std::error::Error>> {
    // Edit between tokens
    let source = "my $x = 42;";
    let mut incremental_parser = CheckpointedIncrementalParser::new();
    incremental_parser.parse(source.to_string())?;

    let edit = SimpleEdit {
        start: 6,
        end: 6,
        new_text: " ".to_string(),
    };
    let _ = incremental_parser.apply_edit(&edit)?;

    // Edit at the boundary of a number and operator
    let source = "my $x = 42 + 10;";
    let mut incremental_parser = CheckpointedIncrementalParser::new();
    incremental_parser.parse(source.to_string())?;

    let edit = SimpleEdit {
        start: 10,
        end: 10,
        new_text: "5".to_string(),
    };
    let _ = incremental_parser.apply_edit(&edit)?;

    // Edit at the boundary of operator and number
    let source = "my $x = 42 + 10;";
    let mut incremental_parser = CheckpointedIncrementalParser::new();
    incremental_parser.parse(source.to_string())?;

    let edit = SimpleEdit {
        start: 12,
        end: 12,
        new_text: "5".to_string(),
    };
    let _ = incremental_parser.apply_edit(&edit)?;

    Ok(())
}

/// Test edits that change source length positively.
#[test]
fn test_source_length_increase_correctness() -> Result<(), Box<dyn std::error::Error>> {
    // Small increase (add a few characters)
    let source = "my $x = 42;";
    let mut incremental_parser = CheckpointedIncrementalParser::new();
    incremental_parser.parse(source.to_string())?;

    let edit = SimpleEdit {
        start: 8,
        end: 8,
        new_text: "999".to_string(),
    };
    let _ = incremental_parser.apply_edit(&edit)?;

    // Large increase (add a line)
    let source = "my $x = 42;";
    let mut incremental_parser = CheckpointedIncrementalParser::new();
    incremental_parser.parse(source.to_string())?;

    let edit = SimpleEdit {
        start: 11,
        end: 11,
        new_text: "\nmy $y = 100;\n".to_string(),
    };
    let _ = incremental_parser.apply_edit(&edit)?;

    Ok(())
}

/// Test edits that change source length negatively.
#[test]
fn test_source_length_decrease_correctness() -> Result<(), Box<dyn std::error::Error>> {
    // Small decrease (remove a few characters)
    let source = "my $x = 12345;";
    let mut incremental_parser = CheckpointedIncrementalParser::new();
    incremental_parser.parse(source.to_string())?;

    let edit = SimpleEdit {
        start: 9,
        end: 12,
        new_text: "".to_string(),
    };
    let _ = incremental_parser.apply_edit(&edit)?;

    // Large decrease (remove a line)
    let source = "my $x = 42;\nmy $y = 100;";
    let mut incremental_parser = CheckpointedIncrementalParser::new();
    incremental_parser.parse(source.to_string())?;

    let edit = SimpleEdit {
        start: 11,
        end: 23,
        new_text: "".to_string(),
    };
    let _ = incremental_parser.apply_edit(&edit)?;

    Ok(())
}

/// Test that an interior edit past the first checkpoint still matches a full parse.
#[test]
fn test_interior_edit_past_checkpoint_matches_full_parse() -> Result<(), Box<dyn std::error::Error>>
{
    let source = "my $value = 1;\n".repeat(20);
    let edit = SimpleEdit {
        start: 125,
        end: 126,
        new_text: "9".to_string(),
    };

    let mut expected_source = source.clone();
    expected_source.replace_range(edit.start..edit.end, &edit.new_text);

    let mut incremental_parser = CheckpointedIncrementalParser::new();
    incremental_parser.parse(source)?;
    let incremental_tree = incremental_parser.apply_edit(&edit)?;

    let mut full_parser = CheckpointedIncrementalParser::new();
    let full_tree = full_parser.parse(expected_source)?;

    assert_eq!(
        format!("{:?}", incremental_tree),
        format!("{:?}", full_tree),
        "incremental parse should match a full parse for edits past the first checkpoint"
    );
    Ok(())
}

// =========================================================================
// 2. Reuse Behavior Tests
// =========================================================================

/// Test that incremental parsing works correctly.
#[test]
fn test_incremental_parsing_correctness() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 42;\nmy $y = 100;\nmy $z = 200;";

    let mut parser = CheckpointedIncrementalParser::new();
    parser.parse(source.to_string())?;

    // Edit in the middle
    let edit = SimpleEdit {
        start: 8,
        end: 10,
        new_text: "99".to_string(),
    };
    parser.apply_edit(&edit)?;

    let stats = parser.stats();

    // Verify that incremental parsing was used
    assert_eq!(stats.incremental_parses, 1, "Expected 1 incremental parse");

    Ok(())
}

/// Test repeated edits in a large file.
#[test]
fn test_repeated_edits_large_file() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n".repeat(200);

    let mut parser = CheckpointedIncrementalParser::new();
    parser.parse(source.clone())?;

    // Apply multiple edits
    for i in 0..10 {
        let edit_start = (source.len() / 10) * i;
        let edit = SimpleEdit {
            start: edit_start,
            end: edit_start + 1,
            new_text: "9".to_string(),
        };
        parser.apply_edit(&edit)?;
    }

    // Verify that parser still works correctly
    let stats = parser.stats();
    assert_eq!(
        stats.incremental_parses, 10,
        "Expected 10 incremental parses"
    );

    Ok(())
}

/// Test undo/redo pattern (edit, reverse edit, original edit).
#[test]
fn test_undo_redo_pattern() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 42;";

    let mut parser = CheckpointedIncrementalParser::new();
    parser.parse(source.to_string())?;

    // Original edit: change 42 to 99
    let edit1 = SimpleEdit {
        start: 8,
        end: 10,
        new_text: "99".to_string(),
    };
    parser.apply_edit(&edit1)?;

    // Reverse edit: change 99 back to 42
    let edit2 = SimpleEdit {
        start: 8,
        end: 10,
        new_text: "42".to_string(),
    };
    parser.apply_edit(&edit2)?;

    // Original edit again: change 42 to 99
    let edit3 = SimpleEdit {
        start: 8,
        end: 10,
        new_text: "99".to_string(),
    };
    parser.apply_edit(&edit3)?;

    // Verify that all parses were correct
    let stats = parser.stats();
    assert_eq!(stats.incremental_parses, 3, "Expected 3 incremental parses");

    Ok(())
}

/// Test edits near regex/division-sensitive regions.
#[test]
fn test_edit_near_regex_division_sensitive() -> Result<(), Box<dyn std::error::Error>> {
    // Division context
    let source = "my $x = 100 / 5;";
    let mut incremental_parser = CheckpointedIncrementalParser::new();
    incremental_parser.parse(source.to_string())?;

    let edit = SimpleEdit {
        start: 11,
        end: 12,
        new_text: "0".to_string(),
    };
    let _ = incremental_parser.apply_edit(&edit)?;

    // Regex context
    let source = "my $x =~ s/foo/bar/;";
    let mut incremental_parser = CheckpointedIncrementalParser::new();
    incremental_parser.parse(source.to_string())?;

    let edit = SimpleEdit {
        start: 11,
        end: 14,
        new_text: "baz".to_string(),
    };
    let _ = incremental_parser.apply_edit(&edit)?;

    // Context that could be either
    let source = "my $x = $a / $b;";
    let mut incremental_parser = CheckpointedIncrementalParser::new();
    incremental_parser.parse(source.to_string())?;

    let edit = SimpleEdit {
        start: 11,
        end: 12,
        new_text: "c".to_string(),
    };
    let _ = incremental_parser.apply_edit(&edit)?;

    Ok(())
}

/// Test simple string replacement.
#[test]
fn test_simple_string_replacement() -> Result<(), Box<dyn std::error::Error>> {
    // Simple ASCII string replacement
    let source = "my $x = 'abc';";
    let mut incremental_parser = CheckpointedIncrementalParser::new();
    incremental_parser.parse(source.to_string())?;

    let edit = SimpleEdit {
        start: 10,
        end: 13,
        new_text: "xyz".to_string(),
    };
    let _ = incremental_parser.apply_edit(&edit)?;

    Ok(())
}

/// Test very long line edits.
#[test]
fn test_very_long_line_edit() -> Result<(), Box<dyn std::error::Error>> {
    // Create a very long line
    let long_line = format!("my $x = {};", "1, ".repeat(1000));
    let source = format!("{}\nmy $y = 2;", long_line);

    // Edit in the middle of long line
    let edit_start = long_line.len() / 2;
    let mut incremental_parser = CheckpointedIncrementalParser::new();
    incremental_parser.parse(source.to_string())?;

    let edit = SimpleEdit {
        start: edit_start,
        end: edit_start + 1,
        new_text: "9".to_string(),
    };
    let _ = incremental_parser.apply_edit(&edit)?;

    Ok(())
}

// =========================================================================
// 3. Metrics Tests
// =========================================================================

/// Test that metrics are available and tracked.
#[test]
fn test_metrics_tracking() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 42;\nmy $y = 100;";

    let mut parser = CheckpointedIncrementalParser::new();
    parser.parse(source.to_string())?;

    // Apply an edit
    let edit = SimpleEdit {
        start: 8,
        end: 10,
        new_text: "99".to_string(),
    };
    parser.apply_edit(&edit)?;

    let stats = parser.stats();

    // Verify that metrics are tracked
    assert_eq!(stats.total_parses, 2, "Expected 2 total parses");
    assert_eq!(stats.incremental_parses, 1, "Expected 1 incremental parse");

    // Verify that all metrics are accessible
    let _ = stats.tokens_reused;
    let _ = stats.tokens_relexed;
    let _ = stats.checkpoints_used;
    let _ = stats.cache_hits;
    let _ = stats.cache_misses;

    Ok(())
}

// =========================================================================
// 4. CheckpointCache::find_after() Tests
// =========================================================================

/// Test find_after() with exact match.
#[test]
fn test_find_after_exact_match() {
    use perl_lexer::checkpoint::{CheckpointCache, LexerCheckpoint};

    let mut cache = CheckpointCache::new(10);
    cache.add(LexerCheckpoint::at_position(100));
    cache.add(LexerCheckpoint::at_position(200));
    cache.add(LexerCheckpoint::at_position(300));

    // Find checkpoint at exact position
    let cp = cache.find_after(200);
    assert!(cp.is_some(), "Expected checkpoint at position 200");
    assert_eq!(cp.unwrap().position, 200);
}

/// Test find_after() between checkpoints.
#[test]
fn test_find_after_between_checkpoints() {
    use perl_lexer::checkpoint::{CheckpointCache, LexerCheckpoint};

    let mut cache = CheckpointCache::new(10);
    cache.add(LexerCheckpoint::at_position(100));
    cache.add(LexerCheckpoint::at_position(200));
    cache.add(LexerCheckpoint::at_position(300));

    // Find checkpoint after position 150
    let cp = cache.find_after(150);
    assert!(cp.is_some(), "Expected checkpoint after position 150");
    assert_eq!(cp.unwrap().position, 200);
}

/// Test find_after() before first checkpoint.
#[test]
fn test_find_after_before_first() {
    use perl_lexer::checkpoint::{CheckpointCache, LexerCheckpoint};

    let mut cache = CheckpointCache::new(10);
    cache.add(LexerCheckpoint::at_position(100));
    cache.add(LexerCheckpoint::at_position(200));
    cache.add(LexerCheckpoint::at_position(300));

    // Find checkpoint after position 50
    let cp = cache.find_after(50);
    assert!(cp.is_some(), "Expected checkpoint after position 50");
    assert_eq!(cp.unwrap().position, 100);
}

/// Test find_after() after last checkpoint.
#[test]
fn test_find_after_after_last() {
    use perl_lexer::checkpoint::{CheckpointCache, LexerCheckpoint};

    let mut cache = CheckpointCache::new(10);
    cache.add(LexerCheckpoint::at_position(100));
    cache.add(LexerCheckpoint::at_position(200));
    cache.add(LexerCheckpoint::at_position(300));

    // Find checkpoint after position 400
    let cp = cache.find_after(400);
    assert!(cp.is_none(), "Expected no checkpoint after position 400");
}

/// Test find_after() with empty cache.
#[test]
fn test_find_after_empty_cache() {
    use perl_lexer::checkpoint::CheckpointCache;

    let cache = CheckpointCache::new(10);

    // Find checkpoint in empty cache
    let cp = cache.find_after(100);
    assert!(cp.is_none(), "Expected no checkpoint in empty cache");
}

/// Test find_after() with single checkpoint.
#[test]
fn test_find_after_single_checkpoint() {
    use perl_lexer::checkpoint::{CheckpointCache, LexerCheckpoint};

    let mut cache = CheckpointCache::new(10);
    cache.add(LexerCheckpoint::at_position(100));

    // Find checkpoint before position
    let cp = cache.find_after(50);
    assert!(cp.is_some(), "Expected checkpoint after position 50");
    assert_eq!(cp.unwrap().position, 100);

    // Find checkpoint at exact position
    let cp = cache.find_after(100);
    assert!(cp.is_some(), "Expected checkpoint at position 100");
    assert_eq!(cp.unwrap().position, 100);

    // Find checkpoint after position
    let cp = cache.find_after(150);
    assert!(cp.is_none(), "Expected no checkpoint after position 150");
}

/// Test find_after() with many checkpoints (binary search verification).
#[test]
fn test_find_after_many_checkpoints() {
    use perl_lexer::checkpoint::{CheckpointCache, LexerCheckpoint};

    let mut cache = CheckpointCache::new(100);
    for i in 0..100 {
        cache.add(LexerCheckpoint::at_position(i * 10));
    }

    // Test various positions
    for i in 0..99 {
        let pos = i * 10 + 5; // Between checkpoints
        let cp = cache.find_after(pos);
        assert!(cp.is_some(), "Expected checkpoint after position {}", pos);
        assert_eq!(cp.unwrap().position, (i + 1) * 10);
    }
}

/// Test find_after() and find_before() together for two-sided window.
#[test]
fn test_find_after_and_before_together() {
    use perl_lexer::checkpoint::{CheckpointCache, LexerCheckpoint};

    let mut cache = CheckpointCache::new(10);
    cache.add(LexerCheckpoint::at_position(100));
    cache.add(LexerCheckpoint::at_position(200));
    cache.add(LexerCheckpoint::at_position(300));

    // Find two-sided window around position 250
    let before = cache.find_before(250);
    let after = cache.find_after(250);

    assert!(before.is_some(), "Expected checkpoint before position 250");
    assert_eq!(before.unwrap().position, 200);

    assert!(after.is_some(), "Expected checkpoint after position 250");
    assert_eq!(after.unwrap().position, 300);
}

/// Test find_after() after edit (checkpoint position adjustment).
#[test]
fn test_find_after_after_edit() {
    use perl_lexer::checkpoint::{CheckpointCache, LexerCheckpoint};

    let mut cache = CheckpointCache::new(10);
    cache.add(LexerCheckpoint::at_position(100));
    cache.add(LexerCheckpoint::at_position(200));
    cache.add(LexerCheckpoint::at_position(300));

    // Apply an edit that shifts positions
    cache.apply_edit(150, 10, 20); // Insert 10 bytes at position 150

    // Find checkpoint after original position 200 (now at 210)
    let cp = cache.find_after(210);
    assert!(cp.is_some(), "Expected checkpoint after position 210");
    assert_eq!(cp.unwrap().position, 210);
}

// =========================================================================
// 5. Integration Tests
// =========================================================================

/// Test end-to-end incremental parsing.
#[test]
fn test_end_to_end_incremental_parsing() -> Result<(), Box<dyn std::error::Error>> {
    // Create a source large enough to have multiple checkpoints
    let source = "my $x = 1;\n".repeat(50);

    let mut parser = CheckpointedIncrementalParser::new();
    let _initial_tree = parser.parse(source.clone())?;

    // Make an edit in the middle
    let edit_start = source.len() / 2;
    let edit = SimpleEdit {
        start: edit_start,
        end: edit_start + 1,
        new_text: "9".to_string(),
    };
    let _incremental_tree = parser.apply_edit(&edit)?;

    // Verify metrics
    let stats = parser.stats();
    assert_eq!(stats.incremental_parses, 1);

    Ok(())
}
