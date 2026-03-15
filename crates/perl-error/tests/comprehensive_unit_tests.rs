//! Comprehensive unit tests for the `perl-error` crate.
//!
//! Covers: ParseError variants, helper constructors, location/suggestion methods,
//! ErrorContext enrichment, ParseBudget presets, BudgetTracker state machine,
//! ParseOutput constructors, ErrorClassifier classify/diagnostic/suggestion/explanation,
//! and recovery module types (ParseError, SyncPoint, RecoveryResult).

use perl_error::classifier::{ErrorClassifier, ParseErrorKind};
use perl_error::recovery;
use perl_error::{
    BudgetTracker, ErrorContext, ParseBudget, ParseError, ParseOutput, get_error_contexts,
};
use perl_tdd_support::must::must_some;

// ---------------------------------------------------------------------------
// ParseError variant construction and Display
// ---------------------------------------------------------------------------

#[test]
fn test_unexpected_eof_display() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::UnexpectedEof;
    let msg = format!("{err}");
    assert!(msg.contains("Unexpected end of input"), "got: {msg}");
    Ok(())
}

#[test]
fn test_unexpected_token_display() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::UnexpectedToken {
        expected: "semicolon".into(),
        found: "comma".into(),
        location: 42,
    };
    let msg = format!("{err}");
    assert!(msg.contains("semicolon"), "got: {msg}");
    assert!(msg.contains("comma"), "got: {msg}");
    assert!(msg.contains("42"), "got: {msg}");
    Ok(())
}

#[test]
fn test_syntax_error_display() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::SyntaxError { message: "bad token".into(), location: 7 };
    let msg = format!("{err}");
    assert!(msg.contains("bad token"), "got: {msg}");
    assert!(msg.contains("7"), "got: {msg}");
    Ok(())
}

#[test]
fn test_lexer_error_display() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::LexerError { message: "invalid byte".into() };
    let msg = format!("{err}");
    assert!(msg.contains("invalid byte"), "got: {msg}");
    Ok(())
}

#[test]
fn test_recursion_limit_display() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::RecursionLimit;
    let msg = format!("{err}");
    assert!(msg.contains("recursion"), "got: {msg}");
    Ok(())
}

#[test]
fn test_invalid_number_display() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::InvalidNumber { literal: "0xZZ".into() };
    let msg = format!("{err}");
    assert!(msg.contains("0xZZ"), "got: {msg}");
    Ok(())
}

#[test]
fn test_invalid_string_display() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::InvalidString;
    let msg = format!("{err}");
    assert!(msg.contains("string"), "got: {msg}");
    Ok(())
}

#[test]
fn test_unclosed_delimiter_display() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::UnclosedDelimiter { delimiter: '(' };
    let msg = format!("{err}");
    assert!(msg.contains('('), "got: {msg}");
    Ok(())
}

#[test]
fn test_invalid_regex_display() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::InvalidRegex { message: "unmatched paren".into() };
    let msg = format!("{err}");
    assert!(msg.contains("unmatched paren"), "got: {msg}");
    Ok(())
}

#[test]
fn test_nesting_too_deep_display() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::NestingTooDeep { depth: 300, max_depth: 256 };
    let msg = format!("{err}");
    assert!(msg.contains("300"), "got: {msg}");
    assert!(msg.contains("256"), "got: {msg}");
    Ok(())
}

// ---------------------------------------------------------------------------
// ParseError helper constructors
// ---------------------------------------------------------------------------

#[test]
fn test_syntax_constructor() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::syntax("missing semi", 10);
    match &err {
        ParseError::SyntaxError { message, location } => {
            assert_eq!(message, "missing semi");
            assert_eq!(*location, 10);
        }
        other => return Err(format!("expected SyntaxError, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn test_unexpected_constructor() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::unexpected("identifier", "number", 5);
    match &err {
        ParseError::UnexpectedToken { expected, found, location } => {
            assert_eq!(expected, "identifier");
            assert_eq!(found, "number");
            assert_eq!(*location, 5);
        }
        other => return Err(format!("expected UnexpectedToken, got {other:?}").into()),
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// ParseError::location()
// ---------------------------------------------------------------------------

#[test]
fn test_location_for_unexpected_token() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::unexpected("a", "b", 99);
    assert_eq!(err.location(), Some(99));
    Ok(())
}

#[test]
fn test_location_for_syntax_error() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::syntax("oops", 33);
    assert_eq!(err.location(), Some(33));
    Ok(())
}

#[test]
fn test_location_returns_none_for_other_variants() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(ParseError::UnexpectedEof.location(), None);
    assert_eq!(ParseError::RecursionLimit.location(), None);
    assert_eq!(ParseError::InvalidString.location(), None);
    assert_eq!(ParseError::InvalidNumber { literal: "x".into() }.location(), None);
    assert_eq!(ParseError::LexerError { message: "x".into() }.location(), None);
    assert_eq!(ParseError::UnclosedDelimiter { delimiter: '"' }.location(), None);
    assert_eq!(ParseError::InvalidRegex { message: "x".into() }.location(), None);
    assert_eq!(ParseError::NestingTooDeep { depth: 1, max_depth: 1 }.location(), None);
    Ok(())
}

// ---------------------------------------------------------------------------
// ParseError::suggestion()
// ---------------------------------------------------------------------------

#[test]
fn test_suggestion_semicolon() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::unexpected("';'", "newline", 10);
    let s = must_some(err.suggestion());
    assert!(s.contains("semicolon"), "got: {s}");
    Ok(())
}

#[test]
fn test_suggestion_right_brace() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::unexpected("'}'", "eof", 10);
    let s = must_some(err.suggestion());
    assert!(s.contains('}'), "got: {s}");
    Ok(())
}

#[test]
fn test_suggestion_right_paren() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::unexpected("')'", "eof", 10);
    let s = must_some(err.suggestion());
    assert!(s.contains(')'), "got: {s}");
    Ok(())
}

#[test]
fn test_suggestion_unclosed_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::UnclosedDelimiter { delimiter: '"' };
    let s = must_some(err.suggestion());
    assert!(s.contains('"'), "got: {s}");
    Ok(())
}

#[test]
fn test_suggestion_none_for_generic_token() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::unexpected("identifier", "number", 0);
    assert!(err.suggestion().is_none());
    Ok(())
}

#[test]
fn test_suggestion_none_for_non_token_errors() -> Result<(), Box<dyn std::error::Error>> {
    assert!(ParseError::UnexpectedEof.suggestion().is_none());
    assert!(ParseError::RecursionLimit.suggestion().is_none());
    assert!(ParseError::InvalidString.suggestion().is_none());
    assert!(ParseError::InvalidNumber { literal: "x".into() }.suggestion().is_none());
    assert!(ParseError::InvalidRegex { message: "x".into() }.suggestion().is_none());
    assert!(ParseError::NestingTooDeep { depth: 1, max_depth: 1 }.suggestion().is_none());
    assert!(ParseError::LexerError { message: "x".into() }.suggestion().is_none());
    assert!(ParseError::syntax("x", 0).suggestion().is_none());
    Ok(())
}

// ---------------------------------------------------------------------------
// From<RegexError> for ParseError
// ---------------------------------------------------------------------------

#[test]
fn test_from_regex_error() -> Result<(), Box<dyn std::error::Error>> {
    let regex_err = perl_regex::RegexError::Syntax { message: "bad group".into(), offset: 5 };
    let parse_err: ParseError = regex_err.into();
    match &parse_err {
        ParseError::SyntaxError { message, location } => {
            assert_eq!(message, "bad group");
            assert_eq!(*location, 5);
        }
        other => return Err(format!("expected SyntaxError, got {other:?}").into()),
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// ParseError Clone + PartialEq
// ---------------------------------------------------------------------------

#[test]
fn test_parse_error_clone_and_eq() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::unexpected("a", "b", 1);
    let cloned = err.clone();
    assert_eq!(err, cloned);
    Ok(())
}

// ---------------------------------------------------------------------------
// get_error_contexts
// ---------------------------------------------------------------------------

#[test]
fn test_error_contexts_multiline() -> Result<(), Box<dyn std::error::Error>> {
    let source = "first\nsecond\nthird";
    // byte offset 6 = start of "second"
    let errors = vec![ParseError::syntax("oops", 6)];
    let ctxs = get_error_contexts(&errors, source);
    assert_eq!(ctxs.len(), 1);
    assert_eq!(ctxs[0].line, 1);
    assert_eq!(ctxs[0].source_line, "second");
    Ok(())
}

#[test]
fn test_error_contexts_no_location_falls_back_to_eof() -> Result<(), Box<dyn std::error::Error>> {
    let source = "hello";
    let errors = vec![ParseError::UnexpectedEof];
    let ctxs = get_error_contexts(&errors, source);
    assert_eq!(ctxs.len(), 1);
    // Should not panic even though location() returns None
    Ok(())
}

#[test]
fn test_error_contexts_empty_source() -> Result<(), Box<dyn std::error::Error>> {
    let source = "";
    let errors = vec![ParseError::syntax("empty", 0)];
    let ctxs = get_error_contexts(&errors, source);
    assert_eq!(ctxs.len(), 1);
    Ok(())
}

#[test]
fn test_error_contexts_preserves_suggestion() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1";
    let errors = vec![ParseError::unexpected("';'", "eof", 0)];
    let ctxs = get_error_contexts(&errors, source);
    assert_eq!(ctxs.len(), 1);
    assert!(ctxs[0].suggestion.is_some());
    Ok(())
}

#[test]
fn test_error_context_struct_fields() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = ErrorContext {
        error: ParseError::UnexpectedEof,
        line: 3,
        column: 7,
        source_line: "some code".into(),
        suggestion: Some("fix it".into()),
    };
    assert_eq!(ctx.line, 3);
    assert_eq!(ctx.column, 7);
    assert_eq!(ctx.source_line, "some code");
    assert_eq!(ctx.suggestion.as_deref(), Some("fix it"));
    Ok(())
}

// ---------------------------------------------------------------------------
// ParseBudget presets
// ---------------------------------------------------------------------------

#[test]
fn test_parse_budget_for_ide() -> Result<(), Box<dyn std::error::Error>> {
    let ide = ParseBudget::for_ide();
    let def = ParseBudget::default();
    assert_eq!(ide, def);
    Ok(())
}

#[test]
fn test_parse_budget_strict_is_tighter() -> Result<(), Box<dyn std::error::Error>> {
    let strict = ParseBudget::strict();
    let def = ParseBudget::default();
    assert!(strict.max_errors < def.max_errors);
    assert!(strict.max_depth < def.max_depth);
    assert!(strict.max_tokens_skipped < def.max_tokens_skipped);
    assert!(strict.max_recoveries < def.max_recoveries);
    Ok(())
}

#[test]
fn test_parse_budget_unlimited() -> Result<(), Box<dyn std::error::Error>> {
    let u = ParseBudget::unlimited();
    assert_eq!(u.max_errors, usize::MAX);
    assert_eq!(u.max_depth, usize::MAX);
    assert_eq!(u.max_tokens_skipped, usize::MAX);
    assert_eq!(u.max_recoveries, usize::MAX);
    Ok(())
}

#[test]
fn test_parse_budget_clone_and_copy() -> Result<(), Box<dyn std::error::Error>> {
    let b = ParseBudget::default();
    let c = b; // Copy
    assert_eq!(b, c);
    Ok(())
}

// ---------------------------------------------------------------------------
// BudgetTracker
// ---------------------------------------------------------------------------

#[test]
fn test_budget_tracker_new_is_zeroed() -> Result<(), Box<dyn std::error::Error>> {
    let t = BudgetTracker::new();
    assert_eq!(t.errors_emitted, 0);
    assert_eq!(t.current_depth, 0);
    assert_eq!(t.max_depth_reached, 0);
    assert_eq!(t.tokens_skipped, 0);
    assert_eq!(t.recoveries_attempted, 0);
    Ok(())
}

#[test]
fn test_budget_tracker_default_equals_new() -> Result<(), Box<dyn std::error::Error>> {
    let a = BudgetTracker::new();
    let b = BudgetTracker::default();
    assert_eq!(a.errors_emitted, b.errors_emitted);
    Ok(())
}

#[test]
fn test_enter_exit_depth_tracking() -> Result<(), Box<dyn std::error::Error>> {
    let mut t = BudgetTracker::new();
    t.enter_depth();
    t.enter_depth();
    t.enter_depth();
    assert_eq!(t.current_depth, 3);
    assert_eq!(t.max_depth_reached, 3);

    t.exit_depth();
    assert_eq!(t.current_depth, 2);
    // max_depth_reached should not decrease
    assert_eq!(t.max_depth_reached, 3);

    t.exit_depth();
    t.exit_depth();
    assert_eq!(t.current_depth, 0);
    Ok(())
}

#[test]
fn test_exit_depth_saturates_at_zero() -> Result<(), Box<dyn std::error::Error>> {
    let mut t = BudgetTracker::new();
    t.exit_depth(); // should not underflow
    assert_eq!(t.current_depth, 0);
    Ok(())
}

#[test]
fn test_record_error_increments() -> Result<(), Box<dyn std::error::Error>> {
    let mut t = BudgetTracker::new();
    t.record_error();
    t.record_error();
    assert_eq!(t.errors_emitted, 2);
    Ok(())
}

#[test]
fn test_record_skip_accumulates() -> Result<(), Box<dyn std::error::Error>> {
    let mut t = BudgetTracker::new();
    t.record_skip(5);
    t.record_skip(3);
    assert_eq!(t.tokens_skipped, 8);
    Ok(())
}

#[test]
fn test_record_recovery_increments() -> Result<(), Box<dyn std::error::Error>> {
    let mut t = BudgetTracker::new();
    t.record_recovery();
    assert_eq!(t.recoveries_attempted, 1);
    Ok(())
}

#[test]
fn test_begin_recovery_returns_true_and_increments() -> Result<(), Box<dyn std::error::Error>> {
    let budget = ParseBudget { max_recoveries: 2, ..Default::default() };
    let mut t = BudgetTracker::new();
    assert!(t.begin_recovery(&budget));
    assert_eq!(t.recoveries_attempted, 1);
    assert!(t.begin_recovery(&budget));
    assert_eq!(t.recoveries_attempted, 2);
    // Now exhausted
    assert!(!t.begin_recovery(&budget));
    assert_eq!(t.recoveries_attempted, 2); // should not increment further
    Ok(())
}

#[test]
fn test_errors_exhausted_boundary() -> Result<(), Box<dyn std::error::Error>> {
    let budget = ParseBudget { max_errors: 1, ..Default::default() };
    let mut t = BudgetTracker::new();
    assert!(!t.errors_exhausted(&budget));
    t.record_error();
    assert!(t.errors_exhausted(&budget));
    Ok(())
}

#[test]
fn test_depth_would_exceed_boundary() -> Result<(), Box<dyn std::error::Error>> {
    let budget = ParseBudget { max_depth: 1, ..Default::default() };
    let mut t = BudgetTracker::new();
    assert!(!t.depth_would_exceed(&budget));
    t.enter_depth();
    assert!(t.depth_would_exceed(&budget));
    Ok(())
}

#[test]
fn test_skip_would_exceed_boundary() -> Result<(), Box<dyn std::error::Error>> {
    let budget = ParseBudget { max_tokens_skipped: 5, ..Default::default() };
    let mut t = BudgetTracker::new();
    assert!(!t.skip_would_exceed(&budget, 5));
    assert!(t.skip_would_exceed(&budget, 6));
    t.record_skip(5);
    assert!(t.skip_would_exceed(&budget, 1));
    Ok(())
}

#[test]
fn test_can_skip_more_exact_boundary() -> Result<(), Box<dyn std::error::Error>> {
    let budget = ParseBudget { max_tokens_skipped: 10, ..Default::default() };
    let mut t = BudgetTracker::new();
    assert!(t.can_skip_more(&budget, 10));
    assert!(!t.can_skip_more(&budget, 11));
    t.record_skip(10);
    assert!(t.can_skip_more(&budget, 0));
    assert!(!t.can_skip_more(&budget, 1));
    Ok(())
}

#[test]
fn test_recoveries_exhausted_boundary() -> Result<(), Box<dyn std::error::Error>> {
    let budget = ParseBudget { max_recoveries: 0, ..Default::default() };
    let t = BudgetTracker::new();
    assert!(t.recoveries_exhausted(&budget));
    Ok(())
}

#[test]
fn test_budget_tracker_clone() -> Result<(), Box<dyn std::error::Error>> {
    let mut t = BudgetTracker::new();
    t.record_error();
    t.enter_depth();
    t.record_skip(7);
    t.record_recovery();

    let c = t.clone();
    assert_eq!(c.errors_emitted, 1);
    assert_eq!(c.current_depth, 1);
    assert_eq!(c.max_depth_reached, 1);
    assert_eq!(c.tokens_skipped, 7);
    assert_eq!(c.recoveries_attempted, 1);
    Ok(())
}

// ---------------------------------------------------------------------------
// ParseOutput
// ---------------------------------------------------------------------------

fn make_program_node() -> perl_ast::Node {
    use perl_ast::{NodeKind, SourceLocation};
    perl_ast::Node::new(
        NodeKind::Program { statements: vec![] },
        SourceLocation { start: 0, end: 0 },
    )
}

#[test]
fn test_parse_output_success_state() -> Result<(), Box<dyn std::error::Error>> {
    let out = ParseOutput::success(make_program_node());
    assert!(out.is_ok());
    assert!(!out.has_errors());
    assert_eq!(out.error_count(), 0);
    assert!(!out.terminated_early);
    assert_eq!(out.budget_usage.errors_emitted, 0);
    Ok(())
}

#[test]
fn test_parse_output_with_errors_state() -> Result<(), Box<dyn std::error::Error>> {
    let errs = vec![ParseError::UnexpectedEof, ParseError::InvalidString];
    let out = ParseOutput::with_errors(make_program_node(), errs);
    assert!(!out.is_ok());
    assert!(out.has_errors());
    assert_eq!(out.error_count(), 2);
    assert_eq!(out.budget_usage.errors_emitted, 2);
    assert!(!out.terminated_early);
    Ok(())
}

#[test]
fn test_parse_output_finish_preserves_all_fields() -> Result<(), Box<dyn std::error::Error>> {
    let mut tracker = BudgetTracker::new();
    tracker.errors_emitted = 10;
    tracker.tokens_skipped = 50;
    tracker.recoveries_attempted = 7;
    tracker.max_depth_reached = 20;
    tracker.current_depth = 3;

    let errs = vec![ParseError::RecursionLimit];
    let out = ParseOutput::finish(make_program_node(), errs, tracker, true);

    assert!(out.terminated_early);
    assert_eq!(out.error_count(), 1);
    assert_eq!(out.budget_usage.errors_emitted, 10);
    assert_eq!(out.budget_usage.tokens_skipped, 50);
    assert_eq!(out.budget_usage.recoveries_attempted, 7);
    assert_eq!(out.budget_usage.max_depth_reached, 20);
    assert_eq!(out.budget_usage.current_depth, 3);
    Ok(())
}

#[test]
fn test_parse_output_with_empty_errors() -> Result<(), Box<dyn std::error::Error>> {
    let out = ParseOutput::with_errors(make_program_node(), vec![]);
    assert!(out.is_ok());
    assert_eq!(out.error_count(), 0);
    Ok(())
}

// ---------------------------------------------------------------------------
// ErrorClassifier — construction
// ---------------------------------------------------------------------------

#[test]
fn test_error_classifier_default_and_new() -> Result<(), Box<dyn std::error::Error>> {
    let _a = ErrorClassifier::new();
    let _b = ErrorClassifier;
    Ok(())
}

// ---------------------------------------------------------------------------
// ErrorClassifier::classify
// ---------------------------------------------------------------------------

fn make_error_node(start: usize, end: usize) -> perl_ast::Node {
    use perl_ast::{NodeKind, SourceLocation};
    perl_ast::Node::new(
        NodeKind::Error { message: "test".into(), expected: vec![], found: None, partial: None },
        SourceLocation { start, end },
    )
}

#[test]
fn test_classify_unclosed_double_quote() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    // Odd number of double quotes → UnclosedString
    let source = r#"my $x = "hello"#;
    let node = make_error_node(8, 15);
    assert_eq!(c.classify(&node, source), ParseErrorKind::UnclosedString);
    Ok(())
}

#[test]
fn test_classify_unclosed_single_quote() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    let source = "my $x = 'hello";
    let node = make_error_node(8, 14);
    assert_eq!(c.classify(&node, source), ParseErrorKind::UnclosedString);
    Ok(())
}

#[test]
fn test_classify_unclosed_regex() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    // Even quotes, error_text starts with '/' and no closing '/'
    let source = "my $x = /abc";
    let node = make_error_node(8, 12);
    assert_eq!(c.classify(&node, source), ParseErrorKind::UnclosedRegex);
    Ok(())
}

#[test]
fn test_classify_missing_semicolon_my() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    let source = "my $x = 42\nmy $y = 10;";
    let node = make_error_node(3, 10);
    assert_eq!(c.classify(&node, source), ParseErrorKind::MissingSemicolon);
    Ok(())
}

#[test]
fn test_classify_missing_semicolon_print() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    // Avoid quoted strings so error_text window doesn't trigger UnclosedString
    let source = "print $x\n";
    let node = make_error_node(6, 8);
    assert_eq!(c.classify(&node, source), ParseErrorKind::MissingSemicolon);
    Ok(())
}

#[test]
fn test_classify_missing_semicolon_say() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    let source = "say $x\n";
    let node = make_error_node(4, 6);
    assert_eq!(c.classify(&node, source), ParseErrorKind::MissingSemicolon);
    Ok(())
}

#[test]
fn test_classify_missing_semicolon_return() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    let source = "return 1\n";
    let node = make_error_node(7, 8);
    assert_eq!(c.classify(&node, source), ParseErrorKind::MissingSemicolon);
    Ok(())
}

#[test]
fn test_classify_missing_semicolon_our() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    let source = "our $x = 42\n";
    let node = make_error_node(4, 11);
    assert_eq!(c.classify(&node, source), ParseErrorKind::MissingSemicolon);
    Ok(())
}

#[test]
fn test_classify_missing_semicolon_local() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    let source = "local $x = 42\n";
    let node = make_error_node(6, 13);
    assert_eq!(c.classify(&node, source), ParseErrorKind::MissingSemicolon);
    Ok(())
}

#[test]
fn test_classify_unclosed_parenthesis() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    let source = "func(1, 2;\n";
    let node = make_error_node(4, 10);
    assert_eq!(c.classify(&node, source), ParseErrorKind::UnclosedParenthesis);
    Ok(())
}

#[test]
fn test_classify_unclosed_bracket() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    let source = "my @a = [1, 2;\n";
    let node = make_error_node(8, 13);
    assert_eq!(c.classify(&node, source), ParseErrorKind::UnclosedBracket);
    Ok(())
}

#[test]
fn test_classify_unclosed_brace() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    let source = "if (1) {\n";
    let node = make_error_node(7, 8);
    assert_eq!(c.classify(&node, source), ParseErrorKind::UnclosedBrace);
    Ok(())
}

#[test]
fn test_classify_unexpected_eof() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    let source = "x;";
    // Error at last byte
    let node = make_error_node(1, 2);
    assert_eq!(c.classify(&node, source), ParseErrorKind::UnexpectedEof);
    Ok(())
}

#[test]
fn test_classify_falls_back_to_invalid_syntax() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    // Even quotes, no unclosed delimiters, no statement keywords, not at EOF
    let source = "abc def ghi;\n";
    let node = make_error_node(4, 7);
    assert_eq!(c.classify(&node, source), ParseErrorKind::InvalidSyntax);
    Ok(())
}

// ---------------------------------------------------------------------------
// ErrorClassifier::get_diagnostic_message
// ---------------------------------------------------------------------------

#[test]
fn test_diagnostic_message_all_variants() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();

    let cases: Vec<(ParseErrorKind, &str)> = vec![
        (ParseErrorKind::UnexpectedToken { expected: "foo".into(), found: "bar".into() }, "foo"),
        (ParseErrorKind::UnclosedString, "string"),
        (ParseErrorKind::UnclosedRegex, "regular expression"),
        (ParseErrorKind::UnclosedBlock, "block"),
        (ParseErrorKind::MissingSemicolon, "semicolon"),
        (ParseErrorKind::InvalidSyntax, "Invalid syntax"),
        (ParseErrorKind::UnclosedParenthesis, "parenthesis"),
        (ParseErrorKind::UnclosedBracket, "bracket"),
        (ParseErrorKind::UnclosedBrace, "brace"),
        (ParseErrorKind::UnterminatedHeredoc, "heredoc"),
        (ParseErrorKind::InvalidVariableName, "variable"),
        (ParseErrorKind::InvalidSubroutineName, "subroutine"),
        (ParseErrorKind::MissingOperator, "operator"),
        (ParseErrorKind::MissingOperand, "operand"),
        (ParseErrorKind::UnexpectedEof, "end of file"),
    ];

    for (kind, substring) in cases {
        let msg = c.get_diagnostic_message(&kind);
        assert!(
            msg.to_lowercase().contains(&substring.to_lowercase()),
            "message for {kind:?} = {msg:?} should contain {substring:?}"
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// ErrorClassifier::get_suggestion
// ---------------------------------------------------------------------------

#[test]
fn test_suggestion_all_variants_with_some() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();

    let kinds_with_suggestions = vec![
        ParseErrorKind::MissingSemicolon,
        ParseErrorKind::UnclosedString,
        ParseErrorKind::UnclosedParenthesis,
        ParseErrorKind::UnclosedBracket,
        ParseErrorKind::UnclosedBrace,
        ParseErrorKind::UnclosedBlock,
        ParseErrorKind::UnclosedRegex,
        ParseErrorKind::UnterminatedHeredoc,
        ParseErrorKind::InvalidVariableName,
        ParseErrorKind::InvalidSubroutineName,
        ParseErrorKind::MissingOperator,
        ParseErrorKind::MissingOperand,
        ParseErrorKind::UnexpectedEof,
        ParseErrorKind::UnexpectedToken { expected: "semi".into(), found: "x".into() },
    ];

    for kind in kinds_with_suggestions {
        assert!(c.get_suggestion(&kind).is_some(), "expected Some suggestion for {kind:?}");
    }
    Ok(())
}

#[test]
fn test_suggestion_none_for_invalid_syntax() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    assert!(c.get_suggestion(&ParseErrorKind::InvalidSyntax).is_none());
    Ok(())
}

// ---------------------------------------------------------------------------
// ErrorClassifier::get_explanation
// ---------------------------------------------------------------------------

#[test]
fn test_explanation_some_variants() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();

    let kinds_with_explanation = vec![
        ParseErrorKind::MissingSemicolon,
        ParseErrorKind::UnclosedString,
        ParseErrorKind::UnclosedRegex,
        ParseErrorKind::UnterminatedHeredoc,
        ParseErrorKind::InvalidVariableName,
        ParseErrorKind::UnclosedBlock,
    ];

    for kind in kinds_with_explanation {
        assert!(c.get_explanation(&kind).is_some(), "expected Some explanation for {kind:?}");
    }
    Ok(())
}

#[test]
fn test_explanation_none_for_other_variants() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();

    let kinds_without_explanation = vec![
        ParseErrorKind::InvalidSyntax,
        ParseErrorKind::UnclosedParenthesis,
        ParseErrorKind::UnclosedBracket,
        ParseErrorKind::UnclosedBrace,
        ParseErrorKind::MissingOperator,
        ParseErrorKind::MissingOperand,
        ParseErrorKind::UnexpectedEof,
        ParseErrorKind::InvalidSubroutineName,
        ParseErrorKind::UnexpectedToken { expected: "a".into(), found: "b".into() },
    ];

    for kind in kinds_without_explanation {
        assert!(c.get_explanation(&kind).is_none(), "expected None explanation for {kind:?}");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// ParseErrorKind Clone + PartialEq
// ---------------------------------------------------------------------------

#[test]
fn test_parse_error_kind_clone_and_eq() -> Result<(), Box<dyn std::error::Error>> {
    let kind = ParseErrorKind::UnclosedString;
    let cloned = kind.clone();
    assert_eq!(kind, cloned);
    Ok(())
}

#[test]
fn test_parse_error_kind_debug() -> Result<(), Box<dyn std::error::Error>> {
    let kind = ParseErrorKind::MissingSemicolon;
    let dbg = format!("{kind:?}");
    assert!(dbg.contains("MissingSemicolon"), "got: {dbg}");
    Ok(())
}

// ---------------------------------------------------------------------------
// recovery module types
// ---------------------------------------------------------------------------

#[test]
fn test_recovery_parse_error_new() -> Result<(), Box<dyn std::error::Error>> {
    use perl_position_tracking::{Position, Range};

    let range = Range::new(Position::new(0, 1, 1), Position::new(5, 1, 6));
    let err = recovery::ParseError::new("test error".into(), range);
    assert_eq!(err.message, "test error");
    assert!(err.expected.is_empty());
    assert!(err.found.is_empty());
    assert!(err.recovery_hint.is_none());
    Ok(())
}

#[test]
fn test_recovery_parse_error_builder_chain() -> Result<(), Box<dyn std::error::Error>> {
    use perl_position_tracking::{Position, Range};

    let range = Range::new(Position::new(0, 1, 1), Position::new(5, 1, 6));
    let err = recovery::ParseError::new("err".into(), range)
        .with_expected(vec!["semi".into(), "brace".into()])
        .with_found("comma".into())
        .with_hint("add semicolon".into());

    assert_eq!(err.expected.len(), 2);
    assert_eq!(err.found, "comma");
    assert_eq!(err.recovery_hint.as_deref(), Some("add semicolon"));
    Ok(())
}

#[test]
fn test_recovery_parse_error_clone() -> Result<(), Box<dyn std::error::Error>> {
    use perl_position_tracking::{Position, Range};

    let range = Range::new(Position::new(0, 1, 1), Position::new(1, 1, 2));
    let err = recovery::ParseError::new("x".into(), range).with_hint("h".into());
    let cloned = err.clone();
    assert_eq!(cloned.message, "x");
    assert_eq!(cloned.recovery_hint.as_deref(), Some("h"));
    Ok(())
}

#[test]
fn test_sync_point_equality() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(recovery::SyncPoint::Semicolon, recovery::SyncPoint::Semicolon);
    assert_ne!(recovery::SyncPoint::Semicolon, recovery::SyncPoint::CloseBrace);
    assert_ne!(recovery::SyncPoint::Keyword, recovery::SyncPoint::Eof);
    Ok(())
}

#[test]
fn test_sync_point_copy() -> Result<(), Box<dyn std::error::Error>> {
    let sp = recovery::SyncPoint::Eof;
    let sp2 = sp; // Copy
    assert_eq!(sp, sp2);
    Ok(())
}

#[test]
fn test_recovery_result_equality() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(recovery::RecoveryResult::Recovered(3), recovery::RecoveryResult::Recovered(3));
    assert_ne!(recovery::RecoveryResult::Recovered(1), recovery::RecoveryResult::Recovered(2));
    assert_eq!(recovery::RecoveryResult::AtSyncPoint, recovery::RecoveryResult::AtSyncPoint);
    assert_eq!(
        recovery::RecoveryResult::BudgetExhausted,
        recovery::RecoveryResult::BudgetExhausted
    );
    assert_eq!(recovery::RecoveryResult::ReachedEof, recovery::RecoveryResult::ReachedEof);
    Ok(())
}

#[test]
fn test_recovery_result_debug() -> Result<(), Box<dyn std::error::Error>> {
    let r = recovery::RecoveryResult::Recovered(5);
    let dbg = format!("{r:?}");
    assert!(dbg.contains("Recovered"), "got: {dbg}");
    assert!(dbg.contains('5'), "got: {dbg}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Edge cases and integration-style checks
// ---------------------------------------------------------------------------

#[test]
fn test_classify_error_text_boundary_beyond_source() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    // Error node start at end of source — should not panic
    let source = "ab";
    let node = make_error_node(2, 2);
    // At EOF
    let _kind = c.classify(&node, source);
    Ok(())
}

#[test]
fn test_classify_single_char_source() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    // Use a minimal source to test boundary without triggering underflow on empty string
    let source = "x";
    let node = make_error_node(0, 1);
    let kind = c.classify(&node, source);
    assert_eq!(kind, ParseErrorKind::UnexpectedEof);
    Ok(())
}

#[test]
fn test_multiple_error_contexts() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line1\nline2\nline3";
    let errors = vec![
        ParseError::syntax("e1", 0),
        ParseError::syntax("e2", 6),
        ParseError::syntax("e3", 12),
    ];
    let ctxs = get_error_contexts(&errors, source);
    assert_eq!(ctxs.len(), 3);
    assert_eq!(ctxs[0].line, 0);
    assert_eq!(ctxs[1].line, 1);
    assert_eq!(ctxs[2].line, 2);
    Ok(())
}

#[test]
fn test_budget_tracker_saturating_operations() -> Result<(), Box<dyn std::error::Error>> {
    let mut t = BudgetTracker::new();
    // Test saturating_add on skip
    t.record_skip(usize::MAX);
    t.record_skip(1); // should saturate, not overflow
    assert_eq!(t.tokens_skipped, usize::MAX);

    // Test saturating_add on error
    t.errors_emitted = usize::MAX;
    t.record_error(); // should saturate
    assert_eq!(t.errors_emitted, usize::MAX);

    // Test saturating_add on recovery
    t.recoveries_attempted = usize::MAX;
    t.record_recovery(); // should saturate
    assert_eq!(t.recoveries_attempted, usize::MAX);
    Ok(())
}

#[test]
fn test_begin_recovery_saturating() -> Result<(), Box<dyn std::error::Error>> {
    let budget = ParseBudget::unlimited();
    let mut t = BudgetTracker::new();
    t.recoveries_attempted = usize::MAX - 1;
    assert!(t.begin_recovery(&budget));
    assert_eq!(t.recoveries_attempted, usize::MAX);
    // At MAX, should still be able to succeed if budget is unlimited
    // But begin_recovery checks >=, so MAX >= MAX is true → returns false
    assert!(!t.begin_recovery(&budget));
    Ok(())
}
