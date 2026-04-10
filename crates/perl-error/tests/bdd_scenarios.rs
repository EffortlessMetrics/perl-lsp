//! Behavior-driven scenarios for `perl-error`.
//!
//! These tests encode end-to-end behavior using Given/When/Then structure,
//! emphasizing parser-facing outcomes.

use perl_ast::{Node, NodeKind, SourceLocation};
use perl_error::classifier::{ErrorClassifier, ParseErrorKind};
use perl_error::{BudgetTracker, ParseBudget, ParseError, ParseOutput, get_error_contexts};

fn error_node_at(byte_offset: usize) -> Node {
    Node::new(
        NodeKind::Error {
            message: "test".into(),
            expected: vec![],
            found: None,
            partial: None,
        },
        SourceLocation { start: byte_offset, end: byte_offset },
    )
}

fn program_node() -> Node {
    Node::new(NodeKind::Program { statements: vec![] }, SourceLocation { start: 0, end: 0 })
}

#[test]
fn given_unclosed_string_when_classified_then_reports_unclosed_string_kind(
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let classifier = ErrorClassifier::new();
    let source = "my $msg = \"unterminated";
    let error_node = error_node_at(source.len().saturating_sub(1));

    // When
    let kind = classifier.classify(&error_node, source);

    // Then
    assert_eq!(kind, ParseErrorKind::UnclosedString);
    assert_eq!(classifier.get_diagnostic_message(&kind), "Unclosed string literal");
    assert!(classifier
        .get_suggestion(&kind)
        .is_some_and(|s| s.contains("closing quote")));
    Ok(())
}

#[test]
fn given_statement_without_semicolon_when_classified_then_reports_missing_semicolon(
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let classifier = ErrorClassifier::new();
    let source = "my $x = 42\n";
    let error_node = error_node_at(source.len().saturating_sub(1));

    // When
    let kind = classifier.classify(&error_node, source);

    // Then
    assert_eq!(kind, ParseErrorKind::MissingSemicolon);
    assert!(classifier
        .get_suggestion(&kind)
        .is_some_and(|s| s.contains("semicolon")));
    Ok(())
}

#[test]
fn given_unexpected_semicolon_when_building_context_then_context_includes_fix_hint(
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "my $x = 1\nprint $x\n";
    let errors = vec![ParseError::unexpected("';'", "newline", 9)];

    // When
    let contexts = get_error_contexts(&errors, source);

    // Then
    assert_eq!(contexts.len(), 1);
    let ctx = &contexts[0];
    assert_eq!(ctx.line, 0);
    assert!(ctx.suggestion.as_deref().is_some_and(|s| s.contains("semicolon")));
    Ok(())
}

#[test]
fn given_strict_budget_when_recovery_attempts_hit_limit_then_recovery_is_blocked(
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let budget = ParseBudget::strict();
    let mut tracker = BudgetTracker::new();

    // When
    for _ in 0..budget.max_recoveries {
        assert!(tracker.begin_recovery(&budget));
    }
    let next_attempt = tracker.begin_recovery(&budget);

    // Then
    assert!(!next_attempt);
    assert!(tracker.recoveries_exhausted(&budget));
    Ok(())
}

#[test]
fn given_multiple_errors_when_building_parse_output_then_diagnostics_are_preserved(
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let diagnostics = vec![ParseError::syntax("missing expression", 3), ParseError::UnexpectedEof];

    // When
    let output = ParseOutput::with_errors(program_node(), diagnostics.clone());

    // Then
    assert!(output.has_errors());
    assert_eq!(output.error_count(), 2);
    assert_eq!(output.diagnostics, diagnostics);
    Ok(())
}

#[test]
fn given_parse_error_offset_when_building_context_then_line_and_column_are_derived(
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "first line\nsecond line\nthird line\n";

    // When
    let contexts = get_error_contexts(&[ParseError::syntax("boom", 14)], source);

    // Then
    assert_eq!(contexts.len(), 1);
    assert_eq!(contexts[0].line, 1);
    assert_eq!(contexts[0].column, 3);
    assert_eq!(contexts[0].source_line, "second line");
    Ok(())
}
