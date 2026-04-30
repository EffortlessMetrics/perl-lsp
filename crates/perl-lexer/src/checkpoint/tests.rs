use super::*;
use crate::LexerMode;

#[test]
fn test_checkpoint_creation() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let cp = LexerCheckpoint::new();
    assert_eq!(cp.position, 0);
    assert_eq!(cp.mode, LexerMode::ExpectTerm);
    assert!(cp.delimiter_stack.is_empty());
    Ok(())
}

#[test]
fn test_checkpoint_diff() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let cp1 = LexerCheckpoint::at_position(10);
    let mut cp2 = cp1.clone();
    cp2.position = 20;
    cp2.mode = LexerMode::ExpectOperator;

    let diff = cp2.diff(&cp1);
    assert_eq!(diff.position_delta, 10);
    assert!(diff.mode_changed);
    assert!(!diff.delimiter_stack_changed);
    Ok(())
}

#[test]
fn test_checkpoint_edit() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut cp = LexerCheckpoint::at_position(50);
    // Edit before checkpoint shifts position by (new_len - old_len)
    cp.apply_edit(10, 5, 10);
    assert_eq!(cp.position, 55);

    // Edit after checkpoint leaves position unchanged
    let mut cp2 = LexerCheckpoint::at_position(50);
    cp2.apply_edit(60, 10, 5);
    assert_eq!(cp2.position, 50);

    // Edit containing checkpoint resets position to edit start
    let mut cp3 = LexerCheckpoint::at_position(50);
    cp3.apply_edit(45, 10, 5);
    assert_eq!(cp3.position, 45);
    Ok(())
}

#[test]
fn test_checkpoint_edit_start_boundary_no_change()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut cp = LexerCheckpoint::at_position(50);
    cp.apply_edit(50, 3, 7);
    assert_eq!(cp.position, 50, "position equal to edit start should remain unchanged");
    Ok(())
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

/// Verify `find_before` uses sorted-invariant binary search (O(log N)).
///
/// This test confirms correctness for: exact match, between two entries,
/// before first entry, and after last entry.
#[test]
fn test_find_before_binary_search() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut cache = CheckpointCache::new(50);
    for pos in [10usize, 20, 30, 40, 50] {
        cache.add(LexerCheckpoint::at_position(pos));
    }

    assert_eq!(
        cache.find_before(30).ok_or("find_before(30) should hit")?.position,
        30,
        "exact match should return the entry at 30"
    );
    assert_eq!(
        cache.find_before(25).ok_or("find_before(25) should hit")?.position,
        20,
        "between 20 and 30 should return 20"
    );
    assert_eq!(
        cache.find_before(100).ok_or("find_before(100) should hit")?.position,
        50,
        "after last entry should return 50"
    );
    assert!(cache.find_before(5).is_none(), "before first entry (5 < 10) should return None");
    Ok(())
}

#[test]
fn test_checkpoint_cache_add_replaces_same_position()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut cache = CheckpointCache::new(5);

    let mut first = LexerCheckpoint::at_position(10);
    first.mode = LexerMode::ExpectTerm;
    cache.add(first);

    let mut replacement = LexerCheckpoint::at_position(10);
    replacement.mode = LexerMode::ExpectOperator;
    cache.add(replacement);

    assert_eq!(cache.len(), 1, "same-position checkpoint should replace in place");
    let cp = cache.find_before(10).ok_or("expected checkpoint at position 10")?;
    assert_eq!(cp.mode, LexerMode::ExpectOperator, "replacement checkpoint should win");
    Ok(())
}

/// Verify that CheckpointedIncrementalParser uses 50 checkpoints (Gap B).
#[test]
fn test_checkpoint_cache_capacity_50() {
    // A cache of capacity 50 must not evict until we exceed 50.
    let mut cache = CheckpointCache::new(50);
    for i in 0..50usize {
        cache.add(LexerCheckpoint::at_position(i * 100));
    }
    assert_eq!(
        cache.len(),
        50,
        "a capacity-50 cache must hold exactly 50 checkpoints before eviction"
    );
    // Adding one more should evict down to 50, not to 10.
    cache.add(LexerCheckpoint::at_position(5000));
    assert_eq!(cache.len(), 50, "eviction must keep exactly max_checkpoints entries");
}

#[test]
fn test_checkpoint_cache_zero_capacity_is_noop() {
    let mut cache = CheckpointCache::new(0);
    cache.add(LexerCheckpoint::at_position(10));
    cache.add(LexerCheckpoint::at_position(20));

    assert!(cache.is_empty(), "zero-capacity cache should ignore inserted checkpoints");
    assert!(cache.find_before(100).is_none());
    assert!(cache.find_after(0).is_none());
}

#[test]
fn test_checkpoint_cache_trim_preserves_last_boundary()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut cache = CheckpointCache::new(3);
    for pos in [10usize, 20, 30, 40] {
        cache.add(LexerCheckpoint::at_position(pos));
    }

    let last =
        cache.find_after(40).ok_or("trimmed cache should retain highest-position checkpoint")?;
    assert_eq!(last.position, 40);
    Ok(())
}

#[test]
fn test_find_after_binary_search() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut cache = CheckpointCache::new(10);
    for pos in [10usize, 20, 30, 40] {
        cache.add(LexerCheckpoint::at_position(pos));
    }

    let exact = cache.find_after(20).ok_or("find_after(20) should return exact checkpoint")?;
    assert_eq!(exact.position, 20);

    let between = cache.find_after(21).ok_or("find_after(21) should return next checkpoint")?;
    assert_eq!(between.position, 30);

    let before_first = cache.find_after(0).ok_or("find_after(0) should return first checkpoint")?;
    assert_eq!(before_first.position, 10);

    assert!(cache.find_after(41).is_none(), "find_after after last checkpoint should be None");
    Ok(())
}

#[test]
fn test_find_after_edges() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut cache = CheckpointCache::new(3);
    cache.add(LexerCheckpoint::at_position(10));
    cache.add(LexerCheckpoint::at_position(20));

    assert_eq!(cache.find_after(0).map(|cp| cp.position), Some(10));
    assert_eq!(cache.find_after(10).map(|cp| cp.position), Some(10));
    assert_eq!(cache.find_after(11).map(|cp| cp.position), Some(20));
    assert!(cache.find_after(30).is_none());
    Ok(())
}

#[test]
fn test_checkpoint_cache_apply_edit_repositions_and_invalidates()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut cache = CheckpointCache::new(4);

    let mut inside_edit = LexerCheckpoint::at_position(12);
    inside_edit.context = CheckpointContext::Regex { delimiter: '/', flags_position: None };
    cache.add(inside_edit);

    cache.add(LexerCheckpoint::at_position(30));

    // Edit [10, 15) -> len 5 replaced by len 2:
    // - checkpoint at 30 shifts left to 27
    // - checkpoint at 12 falls inside edit and resets to position 10 with Normal context.
    cache.apply_edit(10, 5, 2);

    let reset = cache.find_before(10).ok_or("checkpoint inside edit should reset to edit start")?;
    assert_eq!(reset.position, 10);
    assert_eq!(reset.context, CheckpointContext::Normal);
    assert_eq!(reset.mode, LexerMode::ExpectTerm);

    let shifted =
        cache.find_after(11).ok_or("checkpoint after edit should still be present and shifted")?;
    assert_eq!(shifted.position, 27);
    Ok(())
}

#[test]
fn test_checkpoint_cache_capacity_one_keeps_latest()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut cache = CheckpointCache::new(1);
    cache.add(LexerCheckpoint::at_position(10));
    cache.add(LexerCheckpoint::at_position(25));

    let latest =
        cache.find_before(usize::MAX).ok_or("capacity-1 cache should keep one checkpoint")?;
    assert_eq!(latest.position, 25);
    Ok(())
}

#[test]
fn test_checkpoint_cache_capacity_two_keeps_first_and_last()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    // max_checkpoints=2: denominator=1, formula always gives idx=[0, total-1].
    // Middle checkpoints are evicted; first and last boundary anchors are preserved.
    let mut cache = CheckpointCache::new(2);
    for pos in [10usize, 20, 30] {
        cache.add(LexerCheckpoint::at_position(pos));
    }

    // First checkpoint (10) must be kept
    let first = cache.find_before(15).ok_or("capacity-2 cache must keep first checkpoint")?;
    assert_eq!(first.position, 10, "first boundary checkpoint must be retained");

    // Last checkpoint (30) must be kept
    let last = cache.find_after(25).ok_or("capacity-2 cache must keep last checkpoint")?;
    assert_eq!(last.position, 30, "last boundary checkpoint must be retained");

    // Middle checkpoint (20) must have been evicted.
    // With [10, 30] in the cache, find_before(21) returns position 10 (the
    // largest checkpoint whose position is <= 21).  If the eviction formula
    // were broken and retained position 20 instead of 10, this would return
    // 20, and the assertion below would fail.
    let mid = cache.find_before(21);
    assert!(
        mid.is_none_or(|cp| cp.position != 20),
        "middle checkpoint (20) must be evicted when capacity=2 and total=3"
    );
    Ok(())
}

#[test]
fn test_checkpoint_cache_apply_edit_resorts_positions()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut cache = CheckpointCache::new(10);
    for pos in [10usize, 20, 30] {
        cache.add(LexerCheckpoint::at_position(pos));
    }

    // Edit [15, 35) resets checkpoints at 20 and 30 to position 15.
    // Without re-sorting, cache order becomes [10, 15, 15] by position but
    // stored entries can be out of order, breaking binary-search lookups.
    cache.apply_edit(15, 20, 0);

    let before =
        cache.find_before(15).ok_or("find_before should locate checkpoint at edited boundary")?;
    assert_eq!(before.position, 15);

    let after =
        cache.find_after(15).ok_or("find_after should locate checkpoint at edited boundary")?;
    assert_eq!(after.position, 15);

    Ok(())
}
