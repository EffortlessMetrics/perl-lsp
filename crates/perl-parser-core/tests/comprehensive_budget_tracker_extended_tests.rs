use perl_parser_core::{BudgetTracker, ParseBudget};

#[test]
fn depth_tracking_max_depth_reached() -> Result<(), Box<dyn std::error::Error>> {
    let mut tracker = BudgetTracker::new();
    tracker.enter_depth();
    tracker.enter_depth();
    tracker.enter_depth();
    assert_eq!(tracker.max_depth_reached, 3);
    assert_eq!(tracker.current_depth, 3);
    tracker.exit_depth();
    assert_eq!(tracker.current_depth, 2);
    // max_depth_reached stays at 3
    assert_eq!(tracker.max_depth_reached, 3);
    Ok(())
}

#[test]
fn exit_depth_at_zero_saturates() -> Result<(), Box<dyn std::error::Error>> {
    let mut tracker = BudgetTracker::new();
    assert_eq!(tracker.current_depth, 0);
    tracker.exit_depth();
    // Should not underflow
    assert_eq!(tracker.current_depth, 0);
    Ok(())
}

#[test]
fn record_skip_saturating_add() -> Result<(), Box<dyn std::error::Error>> {
    let mut tracker = BudgetTracker::new();
    tracker.tokens_skipped = usize::MAX - 1;
    tracker.record_skip(5);
    // Should saturate at usize::MAX
    assert_eq!(tracker.tokens_skipped, usize::MAX);
    Ok(())
}

#[test]
fn record_error_saturating() -> Result<(), Box<dyn std::error::Error>> {
    let mut tracker = BudgetTracker::new();
    tracker.errors_emitted = usize::MAX - 1;
    tracker.record_error();
    assert_eq!(tracker.errors_emitted, usize::MAX);
    tracker.record_error();
    // Should stay at MAX
    assert_eq!(tracker.errors_emitted, usize::MAX);
    Ok(())
}

#[test]
fn begin_recovery_increments_and_returns_true() -> Result<(), Box<dyn std::error::Error>> {
    let budget = ParseBudget { max_recoveries: 3, ..ParseBudget::default() };
    let mut tracker = BudgetTracker::new();
    assert!(tracker.begin_recovery(&budget));
    assert_eq!(tracker.recoveries_attempted, 1);
    assert!(tracker.begin_recovery(&budget));
    assert_eq!(tracker.recoveries_attempted, 2);
    assert!(tracker.begin_recovery(&budget));
    assert_eq!(tracker.recoveries_attempted, 3);
    // 4th attempt should fail
    assert!(!tracker.begin_recovery(&budget));
    assert_eq!(tracker.recoveries_attempted, 3);
    Ok(())
}

#[test]
fn parse_budget_for_ide_equals_default() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(ParseBudget::for_ide(), ParseBudget::default());
    Ok(())
}

#[test]
fn parse_budget_unlimited_values() -> Result<(), Box<dyn std::error::Error>> {
    let unlimited = ParseBudget::unlimited();
    assert_eq!(unlimited.max_errors, usize::MAX);
    assert_eq!(unlimited.max_depth, usize::MAX);
    assert_eq!(unlimited.max_tokens_skipped, usize::MAX);
    assert_eq!(unlimited.max_recoveries, usize::MAX);
    Ok(())
}

#[test]
fn parse_budget_strict_values() -> Result<(), Box<dyn std::error::Error>> {
    let strict = ParseBudget::strict();
    assert_eq!(strict.max_errors, 10);
    assert_eq!(strict.max_depth, 64);
    assert_eq!(strict.max_tokens_skipped, 100);
    assert_eq!(strict.max_recoveries, 50);
    Ok(())
}
