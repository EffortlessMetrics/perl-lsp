use perl_lexer::{CheckpointCache, LexerCheckpoint, LexerMode, Position};

type R = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn edit_before_checkpoint_updates_position_and_invalidates_line_column() {
    let mut cp = LexerCheckpoint::at_position(20);
    cp.current_pos = Position::new(20, 3, 9);

    cp.apply_edit(5, 2, 7);

    assert_eq!(cp.position, 25);
    assert_eq!(cp.current_pos.byte, 25);
    assert_eq!(cp.current_pos.line, 0);
    assert_eq!(cp.current_pos.column, 0);
}

#[test]
fn edit_spanning_checkpoint_resets_state_conservatively() {
    let mut cp = LexerCheckpoint::at_position(30);
    cp.mode = LexerMode::ExpectOperator;
    cp.delimiter_stack.push('{');
    cp.current_pos = Position::new(30, 5, 8);

    cp.apply_edit(25, 10, 4);

    assert_eq!(cp.position, 25);
    assert_eq!(cp.mode, LexerMode::ExpectTerm);
    assert!(cp.delimiter_stack.is_empty());
    assert_eq!(cp.current_pos.byte, 25);
    assert_eq!(cp.current_pos.line, 0);
    assert_eq!(cp.current_pos.column, 0);
}

#[test]
fn insertion_at_checkpoint_boundary_is_invalidated() {
    let mut cp = LexerCheckpoint::at_position(30);
    cp.mode = LexerMode::ExpectOperator;
    cp.current_pos = Position::new(30, 3, 4);

    cp.apply_edit(30, 0, 4);

    assert_eq!(cp.position, 30);
    assert_eq!(cp.mode, LexerMode::ExpectTerm);
    assert_eq!(cp.current_pos.line, 0);
    assert_eq!(cp.current_pos.column, 0);
}

#[test]
fn newline_edit_before_checkpoint_marks_line_column_unknown() {
    let mut cp = LexerCheckpoint::at_position(12);
    cp.current_pos = Position::new(12, 1, 13);

    // Inserting a newline before the checkpoint changes line/column, which cannot
    // be derived safely from byte lengths alone.
    cp.apply_edit(4, 0, 1);

    assert_eq!(cp.position, 13);
    assert_eq!(cp.current_pos.byte, 13);
    assert_eq!(cp.current_pos.line, 0);
    assert_eq!(cp.current_pos.column, 0);
}

#[test]
fn checkpoint_cache_apply_edit_preserves_sorted_unique_invariants() -> R {
    let mut cache = CheckpointCache::new(10);
    cache.add(LexerCheckpoint::at_position(10));
    cache.add(LexerCheckpoint::at_position(20));
    cache.add(LexerCheckpoint::at_position(30));

    // Deleting bytes before 20 and 30 moves them to 15 and 25, preserving order.
    cache.apply_edit(15, 5, 0);

    let before_100 = cache.find_before(100).ok_or("expected checkpoint")?;
    assert_eq!(before_100.position, 25);

    // Deleting [5, 30) maps both shifted checkpoints to 5; cache should deduplicate.
    cache.apply_edit(5, 25, 0);

    let before_6 = cache.find_before(6).ok_or("expected checkpoint at 5")?;
    assert_eq!(before_6.position, 5);

    let after_5 = cache.find_after(5).ok_or("expected checkpoint at 5")?;
    assert_eq!(after_5.position, 5);

    let after_6 = cache.find_after(6);
    assert!(after_6.is_none(), "duplicate positions should be collapsed");
    Ok(())
}

#[test]
fn malformed_large_edit_lengths_do_not_panic() {
    let mut cp = LexerCheckpoint::at_position(10);
    cp.apply_edit(usize::MAX - 2, 100, usize::MAX);
    assert!(cp.position <= usize::MAX);

    let mut cache = CheckpointCache::new(4);
    cache.add(LexerCheckpoint::at_position(0));
    cache.add(LexerCheckpoint::at_position(1));
    cache.apply_edit(usize::MAX - 1, usize::MAX, usize::MAX);

    assert!(cache.find_before(usize::MAX).is_some());
}
