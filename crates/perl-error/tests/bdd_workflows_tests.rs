//! BDD-style workflow tests for the `perl-error` crate.
//!
//! These tests emphasize end-to-end behavior with Given/When/Then structure
//! across error conversion, classification, enrichment, budgeting, and
//! parse-output aggregation.

use perl_ast::{Node, NodeKind, SourceLocation};
use perl_error::classifier::{ErrorClassifier, ParseErrorKind};
use perl_error::{BudgetTracker, ParseBudget, ParseError, ParseOutput, get_error_contexts};

fn make_program_node() -> Node {
    Node::new(NodeKind::Program { statements: vec![] }, SourceLocation { start: 0, end: 0 })
}

fn make_error_node(start: usize, end: usize) -> Node {
    Node::new(
        NodeKind::Error { message: "test".into(), expected: vec![], found: None, partial: None },
        SourceLocation { start, end },
    )
}

#[test]
fn given_regex_syntax_error_when_converted_then_message_and_offset_are_preserved()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let regex_error = perl_regex::RegexError::Syntax {
        message: "missing closing parenthesis".into(),
        offset: 17,
    };

    // When
    let parse_error: ParseError = regex_error.into();

    // Then
    match parse_error {
        ParseError::SyntaxError { message, location } => {
            assert_eq!(message, "missing closing parenthesis");
            assert_eq!(location, 17);
        }
        other => return Err(format!("expected SyntaxError, got {other:?}").into()),
    }

    Ok(())
}

#[test]
fn given_missing_semicolon_line_when_classified_then_user_guidance_is_actionable()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let classifier = ErrorClassifier::new();
    let source = "my $name = $value\n";
    let error_node = make_error_node(3, 8);

    // When
    let kind = classifier.classify(&error_node, source);
    let diagnostic = classifier.get_diagnostic_message(&kind);
    let suggestion =
        classifier.get_suggestion(&kind).ok_or("expected missing-semicolon suggestion")?;

    // Then
    assert_eq!(kind, ParseErrorKind::MissingSemicolon);
    assert!(diagnostic.to_lowercase().contains("semicolon"));
    assert!(suggestion.contains(';'));

    Ok(())
}

#[test]
fn given_mixed_diagnostics_when_contexts_are_collected_then_only_located_errors_are_enriched()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "my $x = 1\nmy $y = ;\n";
    let errors =
        vec![ParseError::unexpected("expression", "semicolon", 17), ParseError::UnexpectedEof];

    // When
    let contexts = get_error_contexts(&errors, source);

    // Then
    assert_eq!(contexts.len(), 2);
    assert_eq!(contexts[0].line, 1);
    assert!(contexts[0].source_line.contains("my $y = ;"));
    assert_eq!(contexts[0].error.location(), Some(17));
    assert!(contexts[0].suggestion.is_none());

    // Errors without explicit locations are mapped to EOF context.
    assert_eq!(contexts[1].error, ParseError::UnexpectedEof);
    assert!(contexts[1].line >= contexts[0].line);

    Ok(())
}

#[test]
fn given_parse_budget_limits_when_tracker_progresses_then_exhaustion_checks_match_budget()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let budget = ParseBudget::strict();
    let mut tracker = BudgetTracker::new();

    // When
    for _ in 0..budget.max_errors {
        tracker.record_error();
    }
    for _ in 0..budget.max_recoveries {
        let started = tracker.begin_recovery(&budget);
        assert!(started);
    }

    // Then
    assert!(tracker.errors_exhausted(&budget));
    assert!(tracker.recoveries_exhausted(&budget));
    assert!(!tracker.begin_recovery(&budget));
    assert!(tracker.can_skip_more(&budget, budget.max_tokens_skipped));
    assert!(tracker.skip_would_exceed(&budget, budget.max_tokens_skipped + 1));

    Ok(())
}

#[test]
fn given_recovered_and_fatal_errors_when_parse_output_finishes_then_recovered_count_is_precise()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let diagnostics = vec![
        ParseError::Recovered {
            site: perl_error::RecoverySite::ArgList,
            kind: perl_error::RecoveryKind::InsertedCloser,
            location: 9,
        },
        ParseError::Recovered {
            site: perl_error::RecoverySite::InfixRhs,
            kind: perl_error::RecoveryKind::MissingOperand,
            location: 13,
        },
        ParseError::UnexpectedEof,
    ];
    let mut tracker = BudgetTracker::new();
    tracker.errors_emitted = diagnostics.len();

    // When
    let output = ParseOutput::finish(make_program_node(), diagnostics, tracker, true);

    // Then
    assert_eq!(output.error_count(), 3);
    assert_eq!(output.recovered_count, 2);
    assert!(output.terminated_early);

    Ok(())
}
