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
fn test_checkpoint_edit_at_end_boundary_shifts() {
    let mut cp = LexerCheckpoint::at_position(20);
    cp.apply_edit(10, 10, 3);
    assert_eq!(cp.position, 13, "position at edit end should shift, not reset");
}

#[test]
fn test_checkpoint_edit_inside_resets_state() {
    let mut cp = LexerCheckpoint::at_position(15);
    cp.mode = LexerMode::ExpectOperator;
    cp.after_sub = true;
    cp.context = CheckpointContext::Regex { delimiter: '/', flags_position: Some(18) };

    cp.apply_edit(10, 10, 2);
    assert_eq!(cp.position, 10);
    assert_eq!(cp.mode, LexerMode::ExpectTerm);
    assert!(!cp.after_sub);
    assert_eq!(cp.context, CheckpointContext::Normal);
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
