//! Extended unit tests for the `perl-error` crate.
//!
//! These tests complement comprehensive_unit_tests.rs by covering:
//! - std::error::Error trait implementation
#![allow(clippy::panic)]
//! - ParseResult type alias usage
//! - Multi-step BudgetTracker workflows
//! - ParseOutput with complex diagnostics
//! - ErrorClassifier with tricky Perl source patterns
//! - Recovery module builder patterns and edge cases
//! - get_error_contexts edge cases
//! - ParseError PartialEq across variants
//! - ParseBudget arithmetic boundary conditions

use perl_error::classifier::{ErrorClassifier, ParseErrorKind};
use perl_error::recovery;
use perl_error::{
    BudgetTracker, ErrorContext, ParseBudget, ParseError, ParseOutput, ParseResult,
    get_error_contexts,
};
use perl_position_tracking::Position;

// ---------------------------------------------------------------------------
// std::error::Error trait
// ---------------------------------------------------------------------------

#[test]
fn error_trait_is_implemented() {
    let err: Box<dyn std::error::Error> = Box::new(ParseError::UnexpectedEof);
    let msg = format!("{err}");
    assert!(!msg.is_empty());
}

#[test]
fn error_trait_source_returns_none() {
    use std::error::Error;
    let err = ParseError::RecursionLimit;
    assert!(err.source().is_none());
}

#[test]
fn error_trait_debug_contains_variant() {
    let err = ParseError::InvalidString;
    let debug = format!("{err:?}");
    assert!(debug.contains("InvalidString"), "got: {debug}");
}

// ---------------------------------------------------------------------------
// ParseResult type alias
// ---------------------------------------------------------------------------

#[test]
fn parse_result_ok_variant() {
    let result: ParseResult<i32> = Ok(42);
    if let Ok(val) = result {
        assert_eq!(val, 42);
    }
}

#[test]
fn parse_result_err_variant() {
    let result: ParseResult<()> = Err(ParseError::UnexpectedEof);
    assert!(result.is_err());
}

#[test]
fn parse_result_question_mark_propagation() -> Result<(), Box<dyn std::error::Error>> {
    fn inner() -> ParseResult<u32> {
        Ok(100)
    }
    let val = inner()?;
    assert_eq!(val, 100);
    Ok(())
}

// ---------------------------------------------------------------------------
// ParseError PartialEq across variants
// ---------------------------------------------------------------------------

#[test]
fn parse_error_eq_same_variant_same_fields() {
    let a = ParseError::syntax("msg", 10);
    let b = ParseError::syntax("msg", 10);
    assert_eq!(a, b);
}

#[test]
fn parse_error_ne_same_variant_different_fields() {
    let a = ParseError::syntax("msg", 10);
    let b = ParseError::syntax("msg", 20);
    assert_ne!(a, b);
}

#[test]
fn parse_error_ne_different_variants() {
    let a = ParseError::UnexpectedEof;
    let b = ParseError::RecursionLimit;
    assert_ne!(a, b);
}

#[test]
fn parse_error_eq_unclosed_delimiter_same_char() {
    let a = ParseError::UnclosedDelimiter { delimiter: '(' };
    let b = ParseError::UnclosedDelimiter { delimiter: '(' };
    assert_eq!(a, b);
}

#[test]
fn parse_error_ne_unclosed_delimiter_different_char() {
    let a = ParseError::UnclosedDelimiter { delimiter: '(' };
    let b = ParseError::UnclosedDelimiter { delimiter: '{' };
    assert_ne!(a, b);
}

#[test]
fn parse_error_eq_nesting_too_deep() {
    let a = ParseError::NestingTooDeep { depth: 100, max_depth: 64 };
    let b = ParseError::NestingTooDeep { depth: 100, max_depth: 64 };
    assert_eq!(a, b);
}

#[test]
fn parse_error_ne_nesting_too_deep_different_values() {
    let a = ParseError::NestingTooDeep { depth: 100, max_depth: 64 };
    let b = ParseError::NestingTooDeep { depth: 99, max_depth: 64 };
    assert_ne!(a, b);
}

// ---------------------------------------------------------------------------
// ParseError::location exhaustive coverage
// ---------------------------------------------------------------------------

#[test]
fn location_none_for_eof() {
    assert!(ParseError::UnexpectedEof.location().is_none());
}

#[test]
fn location_none_for_recursion_limit() {
    assert!(ParseError::RecursionLimit.location().is_none());
}

#[test]
fn location_none_for_invalid_string() {
    assert!(ParseError::InvalidString.location().is_none());
}

#[test]
fn location_none_for_invalid_number() {
    let err = ParseError::InvalidNumber { literal: "0xGG".into() };
    assert!(err.location().is_none());
}

#[test]
fn location_none_for_unclosed_delimiter() {
    let err = ParseError::UnclosedDelimiter { delimiter: '"' };
    assert!(err.location().is_none());
}

#[test]
fn location_none_for_invalid_regex() {
    let err = ParseError::InvalidRegex { message: "bad".into() };
    assert!(err.location().is_none());
}

#[test]
fn location_none_for_lexer_error() {
    let err = ParseError::LexerError { message: "bad".into() };
    assert!(err.location().is_none());
}

#[test]
fn location_none_for_nesting_too_deep() {
    let err = ParseError::NestingTooDeep { depth: 5, max_depth: 3 };
    assert!(err.location().is_none());
}

#[test]
fn location_some_for_unexpected_token() {
    let err = ParseError::unexpected("a", "b", 77);
    assert_eq!(err.location(), Some(77));
}

#[test]
fn location_some_for_syntax_error() {
    let err = ParseError::syntax("oops", 33);
    assert_eq!(err.location(), Some(33));
}

// ---------------------------------------------------------------------------
// ParseError::suggestion exhaustive
// ---------------------------------------------------------------------------

#[test]
fn suggestion_none_for_eof() {
    assert!(ParseError::UnexpectedEof.suggestion().is_none());
}

#[test]
fn suggestion_none_for_recursion_limit() {
    assert!(ParseError::RecursionLimit.suggestion().is_none());
}

#[test]
fn suggestion_none_for_invalid_string() {
    assert!(ParseError::InvalidString.suggestion().is_none());
}

#[test]
fn suggestion_none_for_invalid_number() {
    let err = ParseError::InvalidNumber { literal: "abc".into() };
    assert!(err.suggestion().is_none());
}

#[test]
fn suggestion_none_for_invalid_regex() {
    let err = ParseError::InvalidRegex { message: "bad".into() };
    assert!(err.suggestion().is_none());
}

#[test]
fn suggestion_none_for_nesting_too_deep() {
    let err = ParseError::NestingTooDeep { depth: 100, max_depth: 50 };
    assert!(err.suggestion().is_none());
}

#[test]
fn suggestion_none_for_lexer_error() {
    let err = ParseError::LexerError { message: "x".into() };
    assert!(err.suggestion().is_none());
}

#[test]
fn suggestion_for_unclosed_delimiter_paren() {
    let err = ParseError::UnclosedDelimiter { delimiter: '(' };
    let sug = err.suggestion().unwrap_or_default();
    assert!(sug.contains("("), "got: {sug}");
}

#[test]
fn suggestion_for_unclosed_delimiter_brace() {
    let err = ParseError::UnclosedDelimiter { delimiter: '{' };
    let sug = err.suggestion().unwrap_or_default();
    assert!(sug.contains("{"), "got: {sug}");
}

// ---------------------------------------------------------------------------
// Display messages - deeper checks
// ---------------------------------------------------------------------------

#[test]
fn display_nesting_too_deep_includes_both_values() {
    let err = ParseError::NestingTooDeep { depth: 300, max_depth: 256 };
    let msg = format!("{err}");
    assert!(msg.contains("300"), "got: {msg}");
    assert!(msg.contains("256"), "got: {msg}");
}

#[test]
fn display_invalid_number_includes_literal() {
    let err = ParseError::InvalidNumber { literal: "0xZZZ".into() };
    let msg = format!("{err}");
    assert!(msg.contains("0xZZZ"), "got: {msg}");
}

#[test]
fn display_unclosed_delimiter_includes_char() {
    let err = ParseError::UnclosedDelimiter { delimiter: '[' };
    let msg = format!("{err}");
    assert!(msg.contains('['), "got: {msg}");
}

#[test]
fn display_invalid_regex_includes_message() {
    let err = ParseError::InvalidRegex { message: "unbalanced parens".into() };
    let msg = format!("{err}");
    assert!(msg.contains("unbalanced parens"), "got: {msg}");
}

#[test]
fn display_lexer_error_includes_message() {
    let err = ParseError::LexerError { message: "unexpected byte 0xFF".into() };
    let msg = format!("{err}");
    assert!(msg.contains("unexpected byte 0xFF"), "got: {msg}");
}

// ---------------------------------------------------------------------------
// ParseError clone preserves all data
// ---------------------------------------------------------------------------

#[test]
fn clone_unexpected_token_preserves_fields() {
    let err = ParseError::unexpected("semi", "brace", 99);
    let cloned = err.clone();
    assert_eq!(err, cloned);
    if let ParseError::UnexpectedToken { expected, found, location } = &cloned {
        assert_eq!(expected, "semi");
        assert_eq!(found, "brace");
        assert_eq!(*location, 99);
    }
}

#[test]
fn clone_nesting_too_deep_preserves_fields() {
    let err = ParseError::NestingTooDeep { depth: 500, max_depth: 256 };
    let cloned = err.clone();
    assert_eq!(err, cloned);
}

// ---------------------------------------------------------------------------
// BudgetTracker multi-step workflows
// ---------------------------------------------------------------------------

#[test]
fn budget_tracker_full_lifecycle() {
    let budget =
        ParseBudget { max_errors: 5, max_depth: 3, max_tokens_skipped: 20, max_recoveries: 4 };
    let mut t = BudgetTracker::new();

    // Depth tracking
    t.enter_depth();
    t.enter_depth();
    assert_eq!(t.current_depth, 2);
    assert_eq!(t.max_depth_reached, 2);
    t.exit_depth();
    assert_eq!(t.current_depth, 1);
    assert_eq!(t.max_depth_reached, 2); // max stays

    // Error tracking
    for _ in 0..4 {
        t.record_error();
    }
    assert!(!t.errors_exhausted(&budget));
    t.record_error();
    assert!(t.errors_exhausted(&budget));

    // Skip tracking
    t.record_skip(10);
    t.record_skip(10);
    assert!(!t.can_skip_more(&budget, 1));

    // Recovery tracking
    assert!(t.begin_recovery(&budget));
    assert!(t.begin_recovery(&budget));
    assert!(t.begin_recovery(&budget));
    assert!(t.begin_recovery(&budget));
    assert!(!t.begin_recovery(&budget));
}

#[test]
fn budget_tracker_max_depth_reached_tracks_peak() {
    let mut t = BudgetTracker::new();
    t.enter_depth();
    t.enter_depth();
    t.enter_depth();
    assert_eq!(t.max_depth_reached, 3);
    t.exit_depth();
    t.exit_depth();
    assert_eq!(t.max_depth_reached, 3);
    t.enter_depth();
    assert_eq!(t.max_depth_reached, 3); // didn't exceed previous peak
    t.enter_depth();
    t.enter_depth();
    assert_eq!(t.max_depth_reached, 4); // new peak
}

#[test]
fn budget_tracker_begin_recovery_increments_on_success() {
    let budget = ParseBudget { max_recoveries: 3, ..Default::default() };
    let mut t = BudgetTracker::new();

    assert!(t.begin_recovery(&budget));
    assert_eq!(t.recoveries_attempted, 1);
    assert!(t.begin_recovery(&budget));
    assert_eq!(t.recoveries_attempted, 2);
    assert!(t.begin_recovery(&budget));
    assert_eq!(t.recoveries_attempted, 3);
    // Now at limit
    assert!(!t.begin_recovery(&budget));
    assert_eq!(t.recoveries_attempted, 3); // unchanged
}

#[test]
fn budget_tracker_zero_budget_blocks_immediately() {
    let budget =
        ParseBudget { max_errors: 0, max_depth: 0, max_tokens_skipped: 0, max_recoveries: 0 };
    let t = BudgetTracker::new();

    assert!(t.errors_exhausted(&budget));
    assert!(t.depth_would_exceed(&budget));
    assert!(t.skip_would_exceed(&budget, 1));
    assert!(t.recoveries_exhausted(&budget));
    assert!(!t.can_skip_more(&budget, 1));
}

// ---------------------------------------------------------------------------
// ParseBudget constructors
// ---------------------------------------------------------------------------

#[test]
fn parse_budget_for_ide_equals_default() {
    assert_eq!(ParseBudget::for_ide(), ParseBudget::default());
}

#[test]
fn parse_budget_strict_all_smaller_than_default() {
    let strict = ParseBudget::strict();
    let default = ParseBudget::default();
    assert!(strict.max_errors < default.max_errors);
    assert!(strict.max_depth < default.max_depth);
    assert!(strict.max_tokens_skipped < default.max_tokens_skipped);
    assert!(strict.max_recoveries < default.max_recoveries);
}

#[test]
fn parse_budget_unlimited_all_usize_max() {
    let u = ParseBudget::unlimited();
    assert_eq!(u.max_errors, usize::MAX);
    assert_eq!(u.max_depth, usize::MAX);
    assert_eq!(u.max_tokens_skipped, usize::MAX);
    assert_eq!(u.max_recoveries, usize::MAX);
}

#[test]
fn parse_budget_copy_semantics() {
    let a = ParseBudget::default();
    let b = a; // Copy
    assert_eq!(a, b);
}

#[test]
fn parse_budget_debug_format() {
    let budget = ParseBudget::strict();
    let dbg = format!("{budget:?}");
    assert!(dbg.contains("ParseBudget"), "got: {dbg}");
}

// ---------------------------------------------------------------------------
// ParseOutput additional tests
// ---------------------------------------------------------------------------

#[test]
fn parse_output_success_has_zero_budget_usage() {
    use perl_ast::{Node, NodeKind, SourceLocation};
    let ast =
        Node::new(NodeKind::Program { statements: vec![] }, SourceLocation { start: 0, end: 0 });
    let output = ParseOutput::success(ast);
    assert_eq!(output.budget_usage.errors_emitted, 0);
    assert_eq!(output.budget_usage.tokens_skipped, 0);
    assert_eq!(output.budget_usage.recoveries_attempted, 0);
    assert_eq!(output.budget_usage.current_depth, 0);
    assert_eq!(output.budget_usage.max_depth_reached, 0);
}

#[test]
fn parse_output_with_errors_sets_error_count_in_tracker() {
    use perl_ast::{Node, NodeKind, SourceLocation};
    let ast =
        Node::new(NodeKind::Program { statements: vec![] }, SourceLocation { start: 0, end: 0 });
    let errs =
        vec![ParseError::UnexpectedEof, ParseError::RecursionLimit, ParseError::InvalidString];
    let output = ParseOutput::with_errors(ast, errs);
    assert_eq!(output.budget_usage.errors_emitted, 3);
    assert!(!output.terminated_early);
}

#[test]
fn parse_output_finish_with_terminated_early() {
    use perl_ast::{Node, NodeKind, SourceLocation};
    let ast =
        Node::new(NodeKind::Program { statements: vec![] }, SourceLocation { start: 0, end: 0 });
    let output = ParseOutput::finish(ast, vec![], BudgetTracker::new(), true);
    assert!(output.terminated_early);
    assert!(output.is_ok());
}

#[test]
fn parse_output_clone() {
    use perl_ast::{Node, NodeKind, SourceLocation};
    let ast =
        Node::new(NodeKind::Program { statements: vec![] }, SourceLocation { start: 0, end: 0 });
    let output = ParseOutput::with_errors(ast, vec![ParseError::InvalidString]);
    let cloned = output.clone();
    assert_eq!(cloned.error_count(), 1);
    assert!(cloned.has_errors());
}

// ---------------------------------------------------------------------------
// get_error_contexts edge cases
// ---------------------------------------------------------------------------

#[test]
fn error_contexts_empty_errors_returns_empty() {
    let contexts = get_error_contexts(&[], "some source");
    assert!(contexts.is_empty());
}

#[test]
fn error_contexts_location_beyond_source_clamps() {
    let source = "short";
    let errors = vec![ParseError::syntax("oops", 9999)];
    let contexts = get_error_contexts(&errors, source);
    assert_eq!(contexts.len(), 1);
    // Should not panic - location clamped to source.len()
}

#[test]
fn error_contexts_at_exact_end_of_source() {
    let source = "abc";
    let errors = vec![ParseError::syntax("at end", 3)];
    let contexts = get_error_contexts(&errors, source);
    assert_eq!(contexts.len(), 1);
}

#[test]
fn error_contexts_multiline_correct_line_numbers() {
    let source = "line0\nline1\nline2\nline3";
    // "line2" starts at offset 12
    let errors = vec![ParseError::syntax("err", 12)];
    let contexts = get_error_contexts(&errors, source);
    assert_eq!(contexts.len(), 1);
    assert_eq!(contexts[0].line, 2);
    assert_eq!(contexts[0].source_line, "line2");
}

#[test]
fn error_contexts_suggestion_propagated_from_error() {
    let source = "my $x = 42";
    let err = ParseError::UnclosedDelimiter { delimiter: '(' };
    let contexts = get_error_contexts(&[err], source);
    assert_eq!(contexts.len(), 1);
    assert!(contexts[0].suggestion.is_some());
}

#[test]
fn error_contexts_no_suggestion_for_generic_error() {
    let source = "something";
    let err = ParseError::UnexpectedEof;
    let contexts = get_error_contexts(&[err], source);
    assert_eq!(contexts.len(), 1);
    assert!(contexts[0].suggestion.is_none());
}

#[test]
fn error_context_column_is_correct() {
    let source = "abcd\nefgh";
    // 'g' is at offset 6 (5 for "abcd\n" + 1 for 'f')
    let errors = vec![ParseError::syntax("at g", 6)];
    let contexts = get_error_contexts(&errors, source);
    assert_eq!(contexts.len(), 1);
    assert_eq!(contexts[0].line, 1);
    assert_eq!(contexts[0].column, 1); // 0-indexed within line
}

// ---------------------------------------------------------------------------
// ErrorClassifier - complex Perl patterns
// ---------------------------------------------------------------------------

fn make_error_node(start: usize, end: usize) -> perl_ast::Node {
    perl_ast::Node::new(
        perl_ast::NodeKind::Error {
            message: "test".into(),
            expected: vec![],
            found: None,
            partial: None,
        },
        perl_ast::SourceLocation { start, end },
    )
}

#[test]
fn classifier_default_trait() {
    let c = ErrorClassifier;
    // Just verify it works the same as new()
    let source = "valid;";
    let node = make_error_node(0, 1);
    let _ = c.classify(&node, source);
}

#[test]
fn classifier_balanced_quotes_not_unclosed_string() {
    let c = ErrorClassifier::new();
    // Source has balanced double quotes
    let source = r#"my $x = "hello"; broken"#;
    let node = make_error_node(17, 23); // "broken" area
    let kind = c.classify(&node, source);
    assert_ne!(kind, ParseErrorKind::UnclosedString);
}

#[test]
fn classifier_unclosed_single_quote() {
    let c = ErrorClassifier::new();
    let source = "my $x = 'hello";
    let node = make_error_node(8, 14);
    let kind = c.classify(&node, source);
    assert_eq!(kind, ParseErrorKind::UnclosedString);
}

#[test]
fn classifier_unclosed_regex_pattern() {
    let c = ErrorClassifier::new();
    // Balanced quotes, error at regex area starting with /
    let source = "my $x = /pattern";
    let node = make_error_node(8, 16);
    let kind = c.classify(&node, source);
    assert_eq!(kind, ParseErrorKind::UnclosedRegex);
}

#[test]
fn classifier_missing_semicolon_our() {
    let c = ErrorClassifier::new();
    let source = "our $x = 42\n";
    let node = make_error_node(4, 11);
    let kind = c.classify(&node, source);
    assert_eq!(kind, ParseErrorKind::MissingSemicolon);
}

#[test]
fn classifier_missing_semicolon_local() {
    let c = ErrorClassifier::new();
    let source = "local $x = 42\n";
    let node = make_error_node(6, 13);
    let kind = c.classify(&node, source);
    assert_eq!(kind, ParseErrorKind::MissingSemicolon);
}

#[test]
fn classifier_unclosed_paren_on_line() {
    let c = ErrorClassifier::new();
    let source = "func(1, 2;\n";
    let node = make_error_node(4, 9);
    let kind = c.classify(&node, source);
    assert_eq!(kind, ParseErrorKind::UnclosedParenthesis);
}

#[test]
fn classifier_unclosed_bracket_on_line() {
    let c = ErrorClassifier::new();
    let source = "my @a = [1, 2;\n";
    let node = make_error_node(8, 13);
    let kind = c.classify(&node, source);
    assert_eq!(kind, ParseErrorKind::UnclosedBracket);
}

#[test]
fn classifier_unclosed_brace_on_line() {
    let c = ErrorClassifier::new();
    // not a statement pattern (no my/print/etc), balanced quotes
    let source = "sub foo { 1;\n";
    let node = make_error_node(8, 12);
    let kind = c.classify(&node, source);
    assert_eq!(kind, ParseErrorKind::UnclosedBrace);
}

#[test]
fn classifier_eof_at_last_byte() {
    let c = ErrorClassifier::new();
    let source = "x";
    let node = make_error_node(0, 1);
    let kind = c.classify(&node, source);
    assert_eq!(kind, ParseErrorKind::UnexpectedEof);
}

#[test]
fn classifier_empty_source_returns_unexpected_eof() {
    let c = ErrorClassifier::new();
    let source = " ";
    let node = make_error_node(0, 1);
    let kind = c.classify(&node, source);
    assert_eq!(kind, ParseErrorKind::UnexpectedEof);
}

// ---------------------------------------------------------------------------
// ErrorClassifier diagnostic messages
// ---------------------------------------------------------------------------

#[test]
fn diagnostic_message_unexpected_token_includes_both() {
    let c = ErrorClassifier::new();
    let msg = c.get_diagnostic_message(&ParseErrorKind::UnexpectedToken {
        expected: "IDENT".into(),
        found: "NUMBER".into(),
    });
    assert!(msg.contains("IDENT"), "got: {msg}");
    assert!(msg.contains("NUMBER"), "got: {msg}");
}

#[test]
fn diagnostic_message_all_variants_non_empty() {
    let c = ErrorClassifier::new();
    let variants = [
        ParseErrorKind::UnclosedString,
        ParseErrorKind::UnclosedRegex,
        ParseErrorKind::UnclosedBlock,
        ParseErrorKind::MissingSemicolon,
        ParseErrorKind::InvalidSyntax,
        ParseErrorKind::UnclosedParenthesis,
        ParseErrorKind::UnclosedBracket,
        ParseErrorKind::UnclosedBrace,
        ParseErrorKind::UnterminatedHeredoc,
        ParseErrorKind::InvalidVariableName,
        ParseErrorKind::InvalidSubroutineName,
        ParseErrorKind::MissingOperator,
        ParseErrorKind::MissingOperand,
        ParseErrorKind::UnexpectedEof,
    ];
    for v in &variants {
        let msg = c.get_diagnostic_message(v);
        assert!(!msg.is_empty(), "Empty message for {v:?}");
    }
}

// ---------------------------------------------------------------------------
// ErrorClassifier suggestions
// ---------------------------------------------------------------------------

#[test]
fn suggestion_invalid_syntax_returns_none() {
    let c = ErrorClassifier::new();
    assert!(c.get_suggestion(&ParseErrorKind::InvalidSyntax).is_none());
}

#[test]
fn suggestion_unexpected_token_references_expected() {
    let c = ErrorClassifier::new();
    let sug = c.get_suggestion(&ParseErrorKind::UnexpectedToken {
        expected: "semicolon".into(),
        found: "brace".into(),
    });
    let text = sug.unwrap_or_default();
    assert!(text.contains("semicolon"), "got: {text}");
}

#[test]
fn suggestion_all_non_invalid_syntax_return_some() {
    let c = ErrorClassifier::new();
    let variants_with_suggestions = [
        ParseErrorKind::UnclosedString,
        ParseErrorKind::UnclosedRegex,
        ParseErrorKind::UnclosedBlock,
        ParseErrorKind::MissingSemicolon,
        ParseErrorKind::UnclosedParenthesis,
        ParseErrorKind::UnclosedBracket,
        ParseErrorKind::UnclosedBrace,
        ParseErrorKind::UnterminatedHeredoc,
        ParseErrorKind::InvalidVariableName,
        ParseErrorKind::InvalidSubroutineName,
        ParseErrorKind::MissingOperator,
        ParseErrorKind::MissingOperand,
        ParseErrorKind::UnexpectedEof,
        ParseErrorKind::UnexpectedToken { expected: "x".into(), found: "y".into() },
    ];
    for v in &variants_with_suggestions {
        assert!(c.get_suggestion(v).is_some(), "No suggestion for {v:?}");
    }
}

// ---------------------------------------------------------------------------
// ErrorClassifier explanations
// ---------------------------------------------------------------------------

#[test]
fn explanation_missing_semicolon_mentions_perl() {
    let c = ErrorClassifier::new();
    let exp = c.get_explanation(&ParseErrorKind::MissingSemicolon).unwrap_or_default();
    assert!(exp.contains("Perl"), "got: {exp}");
}

#[test]
fn explanation_unclosed_string_mentions_quotes() {
    let c = ErrorClassifier::new();
    let exp = c.get_explanation(&ParseErrorKind::UnclosedString).unwrap_or_default();
    assert!(exp.contains("quote"), "got: {exp}");
}

#[test]
fn explanation_unclosed_regex_mentions_regex() {
    let c = ErrorClassifier::new();
    let exp = c.get_explanation(&ParseErrorKind::UnclosedRegex).unwrap_or_default();
    assert!(exp.contains("Regular expressions"), "got: {exp}");
}

#[test]
fn explanation_unterminated_heredoc_mentions_heredoc() {
    let c = ErrorClassifier::new();
    let exp = c.get_explanation(&ParseErrorKind::UnterminatedHeredoc).unwrap_or_default();
    assert!(exp.contains("heredoc") || exp.contains("Heredoc"), "got: {exp}");
}

#[test]
fn explanation_invalid_variable_name_mentions_identifier() {
    let c = ErrorClassifier::new();
    let exp = c.get_explanation(&ParseErrorKind::InvalidVariableName).unwrap_or_default();
    assert!(exp.contains("identifier"), "got: {exp}");
}

#[test]
fn explanation_unclosed_block_mentions_braces() {
    let c = ErrorClassifier::new();
    let exp = c.get_explanation(&ParseErrorKind::UnclosedBlock).unwrap_or_default();
    assert!(exp.contains("brace") || exp.contains("{"), "got: {exp}");
}

#[test]
fn explanation_none_for_missing_operator() {
    let c = ErrorClassifier::new();
    assert!(c.get_explanation(&ParseErrorKind::MissingOperator).is_none());
}

#[test]
fn explanation_none_for_missing_operand() {
    let c = ErrorClassifier::new();
    assert!(c.get_explanation(&ParseErrorKind::MissingOperand).is_none());
}

// ---------------------------------------------------------------------------
// ParseErrorKind traits
// ---------------------------------------------------------------------------

#[test]
fn parse_error_kind_clone_preserves_fields() {
    let kind = ParseErrorKind::UnexpectedToken { expected: "semi".into(), found: "ident".into() };
    let cloned = kind.clone();
    assert_eq!(kind, cloned);
}

#[test]
fn parse_error_kind_debug_format() {
    let kind = ParseErrorKind::MissingSemicolon;
    let dbg = format!("{kind:?}");
    assert!(dbg.contains("MissingSemicolon"), "got: {dbg}");
}

// ---------------------------------------------------------------------------
// Recovery module types
// ---------------------------------------------------------------------------

#[test]
fn recovery_parse_error_new_defaults() {
    use perl_position_tracking::Range;
    let err = recovery::ParseError::new("test message".into(), Range::empty(Position::start()));
    assert_eq!(err.message, "test message");
    assert!(err.expected.is_empty());
    assert!(err.found.is_empty());
    assert!(err.recovery_hint.is_none());
}

#[test]
fn recovery_parse_error_with_expected_sets_vec() {
    use perl_position_tracking::Range;
    let err = recovery::ParseError::new("msg".into(), Range::empty(Position::start()))
        .with_expected(vec!["semicolon".into(), "brace".into()]);
    assert_eq!(err.expected.len(), 2);
}

#[test]
fn recovery_parse_error_with_found_sets_string() {
    use perl_position_tracking::Range;
    let err = recovery::ParseError::new("msg".into(), Range::empty(Position::start()))
        .with_found("comma".into());
    assert_eq!(err.found, "comma");
}

#[test]
fn recovery_parse_error_with_hint_sets_option() {
    use perl_position_tracking::Range;
    let err = recovery::ParseError::new("msg".into(), Range::empty(Position::start()))
        .with_hint("try adding ;".into());
    assert_eq!(err.recovery_hint.as_deref(), Some("try adding ;"));
}

#[test]
fn recovery_parse_error_full_builder_chain() {
    use perl_position_tracking::Range;
    let err = recovery::ParseError::new("unexpected".into(), Range::empty(Position::start()))
        .with_expected(vec!["semi".into()])
        .with_found("eof".into())
        .with_hint("add semicolon".into());
    assert_eq!(err.message, "unexpected");
    assert_eq!(err.expected, vec!["semi".to_string()]);
    assert_eq!(err.found, "eof");
    assert_eq!(err.recovery_hint.as_deref(), Some("add semicolon"));
}

#[test]
fn recovery_parse_error_clone_deep_copy() {
    use perl_position_tracking::Range;
    let err = recovery::ParseError::new("msg".into(), Range::empty(Position::start()))
        .with_expected(vec!["a".into()])
        .with_found("b".into())
        .with_hint("c".into());
    let cloned = err.clone();
    assert_eq!(cloned.message, err.message);
    assert_eq!(cloned.expected, err.expected);
    assert_eq!(cloned.found, err.found);
    assert_eq!(cloned.recovery_hint, err.recovery_hint);
}

// ---------------------------------------------------------------------------
// SyncPoint
// ---------------------------------------------------------------------------

#[test]
fn sync_point_all_variants_distinct() {
    let variants = [
        recovery::SyncPoint::Semicolon,
        recovery::SyncPoint::CloseBrace,
        recovery::SyncPoint::Keyword,
        recovery::SyncPoint::Eof,
    ];
    for (i, a) in variants.iter().enumerate() {
        for (j, b) in variants.iter().enumerate() {
            if i == j {
                assert_eq!(a, b);
            } else {
                assert_ne!(a, b);
            }
        }
    }
}

#[test]
fn sync_point_copy_semantics() {
    let a = recovery::SyncPoint::Semicolon;
    let b = a; // Copy
    assert_eq!(a, b);
}

#[test]
fn sync_point_debug_format() {
    let sp = recovery::SyncPoint::CloseBrace;
    let dbg = format!("{sp:?}");
    assert!(dbg.contains("CloseBrace"), "got: {dbg}");
}

// ---------------------------------------------------------------------------
// RecoveryResult
// ---------------------------------------------------------------------------

#[test]
fn recovery_result_recovered_holds_count() {
    let r = recovery::RecoveryResult::Recovered(5);
    if let recovery::RecoveryResult::Recovered(n) = r {
        assert_eq!(n, 5);
    }
}

#[test]
fn recovery_result_all_variants_distinct() {
    let variants = [
        recovery::RecoveryResult::Recovered(0),
        recovery::RecoveryResult::AtSyncPoint,
        recovery::RecoveryResult::BudgetExhausted,
        recovery::RecoveryResult::ReachedEof,
    ];
    // AtSyncPoint != BudgetExhausted != ReachedEof
    assert_ne!(variants[1], variants[2]);
    assert_ne!(variants[2], variants[3]);
    assert_ne!(variants[1], variants[3]);
}

#[test]
fn recovery_result_copy_semantics() {
    let a = recovery::RecoveryResult::AtSyncPoint;
    let b = a; // Copy
    assert_eq!(a, b);
}

#[test]
fn recovery_result_recovered_different_counts_not_equal() {
    let a = recovery::RecoveryResult::Recovered(1);
    let b = recovery::RecoveryResult::Recovered(2);
    assert_ne!(a, b);
}

// ---------------------------------------------------------------------------
// From<RegexError> conversion
// ---------------------------------------------------------------------------

#[test]
fn from_regex_error_creates_syntax_error() {
    let regex_err =
        perl_regex::RegexError::Syntax { message: "unterminated group".into(), offset: 5 };
    let parse_err: ParseError = regex_err.into();
    assert!(matches!(
        parse_err,
        ParseError::SyntaxError { ref message, location }
            if message.contains("unterminated group") && location == 5
    ));
}

#[test]
fn from_regex_error_zero_offset() {
    let regex_err = perl_regex::RegexError::Syntax { message: "bad pattern".into(), offset: 0 };
    let parse_err: ParseError = regex_err.into();
    assert_eq!(parse_err.location(), Some(0));
}

// ---------------------------------------------------------------------------
// ErrorContext struct field access
// ---------------------------------------------------------------------------

#[test]
fn error_context_fields_accessible() {
    let ctx = ErrorContext {
        error: ParseError::InvalidString,
        line: 5,
        column: 10,
        source_line: "my $x = 'unclosed".into(),
        suggestion: Some("close the quote".into()),
    };
    assert_eq!(ctx.line, 5);
    assert_eq!(ctx.column, 10);
    assert_eq!(ctx.source_line, "my $x = 'unclosed");
    assert_eq!(ctx.suggestion.as_deref(), Some("close the quote"));
    assert_eq!(ctx.error, ParseError::InvalidString);
}
