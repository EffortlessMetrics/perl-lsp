use perl_parser_core::error_recovery::ErrorRecovery;
use perl_parser_core::{
    BudgetTracker,
    ParseBudget,
    ast_v2::NodeKind as V2NodeKind,
    error_recovery::{ParseError as RecoveryParseError, RecoveryResult, SyncPoint},
    // ParserContext
    parser_context::ParserContext,
};

#[test]
fn create_error_node_at_token() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = ParserContext::new("my $x;".to_string());
    let node = ctx.create_error_node(
        "test error".to_string(),
        vec!["something".to_string()],
        None,
    );

    match &node.kind {
        V2NodeKind::Error {
            message,
            expected,
            partial,
        } => {
            assert_eq!(message, "test error");
            assert_eq!(expected, &["something"]);
            assert!(partial.is_none());
        }
        other => return Err(format!("expected Error node, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn create_error_node_at_eof() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = ParserContext::new(String::new());
    let node = ctx.create_error_node("eof error".to_string(), vec![], None);

    match &node.kind {
        V2NodeKind::Error { message, .. } => {
            assert_eq!(message, "eof error");
        }
        other => return Err(format!("expected Error node, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn is_sync_point_semicolon() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = ParserContext::new(";".to_string());
    assert!(ctx.is_sync_point(SyncPoint::Semicolon));
    assert!(!ctx.is_sync_point(SyncPoint::CloseBrace));
    assert!(!ctx.is_sync_point(SyncPoint::Keyword));
    Ok(())
}

#[test]
fn is_sync_point_close_brace() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = ParserContext::new("}".to_string());
    assert!(ctx.is_sync_point(SyncPoint::CloseBrace));
    assert!(!ctx.is_sync_point(SyncPoint::Semicolon));
    Ok(())
}

#[test]
fn is_sync_point_keyword() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = ParserContext::new("my $x".to_string());
    assert!(ctx.is_sync_point(SyncPoint::Keyword));
    Ok(())
}

#[test]
fn is_sync_point_eof_on_empty() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = ParserContext::new(String::new());
    assert!(ctx.is_sync_point(SyncPoint::Eof));
    assert!(!ctx.is_sync_point(SyncPoint::Semicolon));
    Ok(())
}

#[test]
fn synchronize_skips_to_sync_point() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = ParserContext::new("foo bar ;".to_string());
    let skipped = ctx.synchronize(&[SyncPoint::Semicolon]);
    // It should have found the semicolon
    assert!(skipped || ctx.is_eof());
    Ok(())
}

#[test]
fn synchronize_at_sync_point_returns_false() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = ParserContext::new(";".to_string());
    // Already at a semicolon sync point
    let result = ctx.synchronize(&[SyncPoint::Semicolon]);
    // skip_until returns 0 when already at sync point, so synchronize returns false
    assert!(!result);
    Ok(())
}

#[test]
fn recover_with_node_adds_error_and_creates_node() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = ParserContext::new("foo ;".to_string());
    let error = RecoveryParseError::new("bad token".to_string(), ctx.current_position_range());

    let node = ctx.recover_with_node(error);

    match &node.kind {
        V2NodeKind::Error { message, .. } => {
            assert_eq!(message, "bad token");
        }
        other => return Err(format!("expected Error, got {:?}", other).into()),
    }

    let errors = ctx.take_errors();
    assert!(!errors.is_empty(), "error should have been recorded");
    Ok(())
}

#[test]
fn skip_until_with_budget_at_sync_point() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = ParserContext::new(";".to_string());
    let budget = ParseBudget::default();
    let mut tracker = BudgetTracker::new();

    let result = ctx.skip_until_with_budget(&[SyncPoint::Semicolon], &budget, &mut tracker);
    assert_eq!(result, RecoveryResult::AtSyncPoint);
    Ok(())
}

#[test]
fn skip_until_with_budget_reaches_eof() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = ParserContext::new("foo bar baz".to_string());
    let budget = ParseBudget::for_ide();
    let mut tracker = BudgetTracker::new();

    let result = ctx.skip_until_with_budget(&[SyncPoint::Semicolon], &budget, &mut tracker);
    assert_eq!(result, RecoveryResult::ReachedEof);
    assert!(tracker.tokens_skipped > 0, "should have skipped tokens");
    Ok(())
}

#[test]
fn skip_until_with_budget_exhaustion() -> Result<(), Box<dyn std::error::Error>> {
    let budget = ParseBudget {
        max_tokens_skipped: 1,
        ..ParseBudget::strict()
    };
    let mut ctx = ParserContext::new("foo bar baz qux ;".to_string());
    let mut tracker = BudgetTracker::new();

    let result = ctx.skip_until_with_budget(&[SyncPoint::Semicolon], &budget, &mut tracker);
    // Should exhaust budget before finding semicolon
    assert_eq!(result, RecoveryResult::BudgetExhausted);
    Ok(())
}

#[test]
fn skip_until_with_budget_eof_empty() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = ParserContext::new(String::new());
    let budget = ParseBudget::default();
    let mut tracker = BudgetTracker::new();

    let result = ctx.skip_until_with_budget(&[SyncPoint::Semicolon], &budget, &mut tracker);
    assert_eq!(result, RecoveryResult::ReachedEof);
    Ok(())
}

#[test]
fn skip_until_with_budget_recovers() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = ParserContext::new("foo ;".to_string());
    let budget = ParseBudget::for_ide();
    let mut tracker = BudgetTracker::new();

    let result = ctx.skip_until_with_budget(&[SyncPoint::Semicolon], &budget, &mut tracker);
    match result {
        RecoveryResult::Recovered(n) => {
            assert!(n > 0, "should have skipped at least one token")
        }
        other => return Err(format!("expected Recovered, got {:?}", other).into()),
    }
    Ok(())
}
