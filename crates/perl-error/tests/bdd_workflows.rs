//! Behavior-driven tests for the `perl-error` crate.
//!
//! These scenarios validate user-visible behavior in a Given/When/Then style
//! across error creation, context enrichment, budget tracking, classification,
//! and recovery outcomes.

use perl_ast::{Node, NodeKind, SourceLocation};
use perl_error::classifier::{ErrorClassifier, ParseErrorKind};
use perl_error::recovery::{ParseError as RecoveryParseError, RecoveryResult};
use perl_error::{BudgetTracker, ParseBudget, ParseError, get_error_contexts};
use perl_position_tracking::{Position, Range};
use perl_tdd_support::must_some;

#[test]
fn given_unexpected_token_when_requesting_fix_hint_then_semicolon_suggestion_is_returned()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let error = ParseError::unexpected("';'", "newline", 13);

    // When
    let suggestion = must_some(error.suggestion());

    // Then
    assert!(suggestion.contains("semicolon"), "got: {suggestion}");
    Ok(())
}

#[test]
fn given_source_with_error_locations_when_enriching_context_then_line_and_column_are_resolved()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "my $x = 1\nmy $y = ;\n";
    let errors = vec![
        ParseError::syntax("expected expression", 16),
        ParseError::unexpected("';'", "newline", 17),
    ];

    // When
    let contexts = get_error_contexts(&errors, source);

    // Then
    assert_eq!(contexts.len(), 2);
    assert_eq!(contexts[0].line, 1);
    assert_eq!(contexts[0].column, 6);
    assert_eq!(contexts[0].source_line, "my $y = ;");
    assert!(contexts[0].suggestion.is_none());

    assert_eq!(contexts[1].line, 1);
    assert_eq!(contexts[1].column, 7);
    assert_eq!(contexts[1].source_line, "my $y = ;");
    let suggestion = must_some(contexts[1].suggestion.clone());
    assert!(suggestion.contains("semicolon"), "got: {suggestion}");
    Ok(())
}

#[test]
fn given_strict_budget_when_multiple_recoveries_attempted_then_budget_exhaustion_is_detected()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let budget = ParseBudget::strict();
    let mut tracker = BudgetTracker::new();

    // When
    for _ in 0..budget.max_recoveries {
        assert!(tracker.begin_recovery(&budget), "recovery should still be allowed");
    }

    // Then
    assert!(tracker.recoveries_exhausted(&budget));
    assert!(!tracker.begin_recovery(&budget), "expected strict budget to be exhausted");
    Ok(())
}

#[test]
fn given_unclosed_string_source_when_classifying_then_diagnostic_and_suggestion_match()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let classifier = ErrorClassifier::new();
    let source = "my $name = \"perl\nprint $name;\n";
    let error_node = Node::new(NodeKind::MissingExpression, SourceLocation { start: 10, end: 14 });

    // When
    let kind = classifier.classify(&error_node, source);
    let message = classifier.get_diagnostic_message(&kind);
    let suggestion = must_some(classifier.get_suggestion(&kind));

    // Then
    assert_eq!(kind, ParseErrorKind::UnclosedString);
    assert!(message.contains("Unclosed string"), "got: {message}");
    assert!(suggestion.contains("closing quote"), "got: {suggestion}");
    Ok(())
}

#[test]
fn given_recovery_parse_error_builder_when_fields_are_chained_then_recovery_payload_is_complete()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let range = Range::new(Position::new(3, 4, 34), Position::new(3, 7, 37));

    // When
    let error = RecoveryParseError::new("missing expression".to_string(), range)
        .with_expected(vec!["term".to_string(), "identifier".to_string()])
        .with_found(";".to_string())
        .with_hint("insert an operand before ';'".to_string());

    // Then
    assert_eq!(error.message, "missing expression");
    assert_eq!(error.range, range);
    assert_eq!(error.expected, vec!["term", "identifier"]);
    assert_eq!(error.found, ";");
    let hint = must_some(error.recovery_hint);
    assert!(hint.contains("operand"), "got: {hint}");
    Ok(())
}

#[test]
fn given_budget_exhaustion_result_when_matching_outcome_then_parser_can_stop_cleanly()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let result = RecoveryResult::BudgetExhausted;

    // When
    let should_stop =
        matches!(result, RecoveryResult::BudgetExhausted | RecoveryResult::ReachedEof);

    // Then
    assert!(should_stop);
    Ok(())
}
