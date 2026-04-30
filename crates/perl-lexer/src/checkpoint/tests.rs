use super::*;
use crate::LexerMode;

#[test]
fn test_checkpoint_creation() {
    let cp = LexerCheckpoint::new();
    assert_eq!(cp.position, 0);
    assert_eq!(cp.mode, LexerMode::ExpectTerm);
    assert!(cp.delimiter_stack.is_empty());
}

#[test]
fn test_checkpoint_diff() {
    let cp1 = LexerCheckpoint::at_position(10);
    let mut cp2 = cp1.clone();
    cp2.position = 20;
    cp2.mode = LexerMode::ExpectOperator;

    let diff = cp2.diff(&cp1);
    assert_eq!(diff.position_delta, 10);
    assert!(diff.mode_changed);
    assert!(!diff.delimiter_stack_changed);
}

#[test]
fn test_checkpoint_edit() {
    let mut cp = LexerCheckpoint::at_position(50);
    cp.apply_edit(10, 5, 10);
    assert_eq!(cp.position, 55);

    let mut cp2 = LexerCheckpoint::at_position(50);
    cp2.apply_edit(60, 10, 5);
    assert_eq!(cp2.position, 50);

    let mut cp3 = LexerCheckpoint::at_position(50);
    cp3.apply_edit(45, 10, 5);
    assert_eq!(cp3.position, 45);
}

#[test]
fn test_apply_edit_at_exact_start_does_not_shift() {
    let mut cp = LexerCheckpoint::at_position(40);
    cp.apply_edit(40, 3, 10);
    assert_eq!(cp.position, 40, "checkpoint at edit start is stable anchor");
}

#[test]
fn test_find_after_edge_behavior() {
    let mut cache = CheckpointCache::new(4);
    for pos in [10usize, 20, 30] {
        cache.add(LexerCheckpoint::at_position(pos));
    }

    assert_eq!(cache.find_after(0).map(|cp| cp.position), Some(10));
    assert_eq!(cache.find_after(20).map(|cp| cp.position), Some(20));
    assert_eq!(cache.find_after(21).map(|cp| cp.position), Some(30));
    assert_eq!(cache.find_after(31).map(|cp| cp.position), None);
}

#[test]
fn test_checkpoint_cache() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut cache = CheckpointCache::new(3);
    cache.add(LexerCheckpoint::at_position(10));
    cache.add(LexerCheckpoint::at_position(20));
    cache.add(LexerCheckpoint::at_position(30));
    cache.add(LexerCheckpoint::at_position(40));

    assert_eq!(cache.len(), 3);
    let cp = cache.find_before(25).ok_or("Expected checkpoint before position 25")?;
    assert_eq!(cp.position, 20);
    Ok(())
}
