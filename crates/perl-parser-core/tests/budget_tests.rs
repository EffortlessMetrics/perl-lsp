use perl_parser_core::{BudgetTracker, ParseBudget};

#[test]
fn default_budget_is_reasonable() -> Result<(), Box<dyn std::error::Error>> {
    let budget = ParseBudget::default();
    assert!(budget.max_errors > 0);
    assert!(budget.max_depth > 0);
    assert!(budget.max_tokens_skipped > 0);
    assert!(budget.max_recoveries > 0);
    Ok(())
}

#[test]
fn ide_budget_is_more_permissive() -> Result<(), Box<dyn std::error::Error>> {
    let strict = ParseBudget::strict();
    let ide = ParseBudget::for_ide();
    assert!(ide.max_errors >= strict.max_errors);
    assert!(ide.max_tokens_skipped >= strict.max_tokens_skipped);
    Ok(())
}

#[test]
fn unlimited_budget() -> Result<(), Box<dyn std::error::Error>> {
    let budget = ParseBudget::unlimited();
    assert!(budget.max_errors > 1000);
    assert!(budget.max_depth > 1000);
    Ok(())
}

#[test]
fn tracker_initially_zero() -> Result<(), Box<dyn std::error::Error>> {
    let tracker = BudgetTracker::new();
    assert_eq!(tracker.errors_emitted, 0);
    assert_eq!(tracker.current_depth, 0);
    assert_eq!(tracker.max_depth_reached, 0);
    assert_eq!(tracker.tokens_skipped, 0);
    assert_eq!(tracker.recoveries_attempted, 0);
    Ok(())
}

#[test]
fn tracker_record_error() -> Result<(), Box<dyn std::error::Error>> {
    let mut tracker = BudgetTracker::new();
    tracker.record_error();
    assert_eq!(tracker.errors_emitted, 1);
    tracker.record_error();
    assert_eq!(tracker.errors_emitted, 2);
    Ok(())
}

#[test]
fn tracker_depth_enter_exit() -> Result<(), Box<dyn std::error::Error>> {
    let mut tracker = BudgetTracker::new();
    tracker.enter_depth();
    assert_eq!(tracker.current_depth, 1);
    assert_eq!(tracker.max_depth_reached, 1);

    tracker.enter_depth();
    assert_eq!(tracker.current_depth, 2);
    assert_eq!(tracker.max_depth_reached, 2);

    tracker.exit_depth();
    assert_eq!(tracker.current_depth, 1);
    assert_eq!(tracker.max_depth_reached, 2); // max doesn't decrease
    Ok(())
}

#[test]
fn tracker_record_skip() -> Result<(), Box<dyn std::error::Error>> {
    let mut tracker = BudgetTracker::new();
    tracker.record_skip(5);
    assert_eq!(tracker.tokens_skipped, 5);
    tracker.record_skip(3);
    assert_eq!(tracker.tokens_skipped, 8);
    Ok(())
}

#[test]
fn tracker_errors_exhausted() -> Result<(), Box<dyn std::error::Error>> {
    let budget = ParseBudget {
        max_errors: 2,
        ..ParseBudget::default()
    };
    let mut tracker = BudgetTracker::new();

    assert!(!tracker.errors_exhausted(&budget));
    tracker.record_error();
    assert!(!tracker.errors_exhausted(&budget));
    tracker.record_error();
    assert!(tracker.errors_exhausted(&budget));
    Ok(())
}

#[test]
fn tracker_depth_would_exceed() -> Result<(), Box<dyn std::error::Error>> {
    let budget = ParseBudget {
        max_depth: 2,
        ..ParseBudget::default()
    };
    let mut tracker = BudgetTracker::new();

    assert!(!tracker.depth_would_exceed(&budget));
    tracker.enter_depth();
    assert!(!tracker.depth_would_exceed(&budget));
    tracker.enter_depth();
    assert!(tracker.depth_would_exceed(&budget));
    Ok(())
}

#[test]
fn tracker_skip_would_exceed() -> Result<(), Box<dyn std::error::Error>> {
    let budget = ParseBudget {
        max_tokens_skipped: 10,
        ..ParseBudget::default()
    };
    let tracker = BudgetTracker::new();

    assert!(!tracker.skip_would_exceed(&budget, 5));
    assert!(!tracker.skip_would_exceed(&budget, 10));
    assert!(tracker.skip_would_exceed(&budget, 11));
    Ok(())
}

#[test]
fn tracker_begin_recovery_checks_budget() -> Result<(), Box<dyn std::error::Error>> {
    let budget = ParseBudget {
        max_recoveries: 1,
        ..ParseBudget::default()
    };
    let mut tracker = BudgetTracker::new();

    assert!(
        tracker.begin_recovery(&budget),
        "first recovery should succeed"
    );
    assert!(
        !tracker.begin_recovery(&budget),
        "second recovery should fail"
    );
    Ok(())
}

#[test]
fn tracker_can_skip_more() -> Result<(), Box<dyn std::error::Error>> {
    let budget = ParseBudget {
        max_tokens_skipped: 5,
        ..ParseBudget::default()
    };
    let tracker = BudgetTracker::new();

    assert!(tracker.can_skip_more(&budget, 3));
    assert!(tracker.can_skip_more(&budget, 5));
    assert!(!tracker.can_skip_more(&budget, 6));
    Ok(())
}

#[test]
fn tracker_record_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let mut tracker = BudgetTracker::new();
    tracker.record_recovery();
    assert_eq!(tracker.recoveries_attempted, 1);
    Ok(())
}
