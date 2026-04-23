use perl_parser_core::{
    BudgetTracker,
    // AST (v1) types used by Parser
    Node as V1Node,
    NodeKind as V1NodeKind,
    // Error types and recovery
    ParseError as CatastrophicParseError,
    ParseOutput,
    SourceLocation,
};

fn make_empty_program() -> V1Node {
    V1Node::new(V1NodeKind::Program { statements: vec![] }, SourceLocation::new(0, 0))
}

#[test]
fn finish_preserves_all_fields() -> Result<(), Box<dyn std::error::Error>> {
    let ast = make_empty_program();
    let errors =
        vec![CatastrophicParseError::syntax("e1", 0), CatastrophicParseError::syntax("e2", 5)];
    let mut tracker = BudgetTracker::new();
    tracker.errors_emitted = 7;
    tracker.tokens_skipped = 33;
    tracker.recoveries_attempted = 4;
    tracker.max_depth_reached = 12;
    tracker.current_depth = 2;

    let output = ParseOutput::finish(ast, errors, tracker, true);
    assert_eq!(output.error_count(), 2);
    assert!(output.has_errors());
    assert!(!output.is_ok());
    assert!(output.terminated_early);
    assert_eq!(output.budget_usage.errors_emitted, 7);
    assert_eq!(output.budget_usage.tokens_skipped, 33);
    assert_eq!(output.budget_usage.recoveries_attempted, 4);
    assert_eq!(output.budget_usage.max_depth_reached, 12);
    assert_eq!(output.budget_usage.current_depth, 2);
    Ok(())
}

#[test]
fn success_output_is_clean() -> Result<(), Box<dyn std::error::Error>> {
    let output = ParseOutput::success(make_empty_program());
    assert!(output.is_ok());
    assert!(!output.has_errors());
    assert_eq!(output.error_count(), 0);
    assert!(!output.terminated_early);
    assert_eq!(output.budget_usage.errors_emitted, 0);
    assert_eq!(output.budget_usage.tokens_skipped, 0);
    Ok(())
}

#[test]
fn with_errors_sets_error_count_in_tracker() -> Result<(), Box<dyn std::error::Error>> {
    let errors = vec![
        CatastrophicParseError::UnexpectedEof,
        CatastrophicParseError::RecursionLimit,
        CatastrophicParseError::InvalidString,
    ];
    let output = ParseOutput::with_errors(make_empty_program(), errors);
    assert_eq!(output.budget_usage.errors_emitted, 3);
    assert!(!output.terminated_early);
    Ok(())
}
