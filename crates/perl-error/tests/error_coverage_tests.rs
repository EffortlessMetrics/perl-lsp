//! Additional test coverage for the `perl-error` crate.
//!
//! Focuses on gaps not covered by existing test files:
//! - ParseError::suggestion() for bracket, fat-arrow, and arrow patterns
//! - ParseError as std::error::Error trait object (dyn dispatch, downcasting)
//! - ParseResult with ? propagation through error conversion
//! - ErrorContext Clone and Debug traits
//! - ParseOutput Debug trait
//! - get_error_contexts with mixed location/no-location errors
//! - ErrorClassifier classify edge cases (regex with //, error_text boundary)
//! - BudgetTracker saturating arithmetic at usize::MAX boundaries
//! - Recovery module ParseError Debug formatting
//! - ParseError::syntax with various Into<String> argument types

use perl_error::classifier::{ErrorClassifier, ParseErrorKind};
use perl_error::recovery;
use perl_error::{
    BudgetTracker, ErrorContext, ParseBudget, ParseError, ParseOutput, ParseResult,
    get_error_contexts,
};
use perl_tdd_support::must_some;

// ---------------------------------------------------------------------------
// ParseError::suggestion() — bracket, fat-arrow, arrow patterns
// ---------------------------------------------------------------------------

#[test]
fn suggestion_for_expected_closing_bracket() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::unexpected("']'", "eof", 10);
    let sug = err.suggestion();
    assert!(sug.is_some(), "expected suggestion for ']'");
    let text = sug.unwrap_or_default();
    assert!(text.contains(']'), "suggestion should mention ']', got: {text}");
    Ok(())
}

#[test]
fn suggestion_for_fat_arrow_where_expression_expected() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::unexpected("expression", "=>", 10);
    let sug = err.suggestion();
    assert!(sug.is_some(), "expected suggestion for fat arrow");
    let text = sug.unwrap_or_default();
    assert!(text.contains("=>"), "suggestion should mention '=>', got: {text}");
    Ok(())
}

#[test]
fn suggestion_for_arrow_where_expression_expected() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::unexpected("expression", "->", 10);
    let sug = err.suggestion();
    assert!(sug.is_some(), "expected suggestion for arrow");
    let text = sug.unwrap_or_default();
    assert!(text.contains("->"), "suggestion should mention '->', got: {text}");
    Ok(())
}

#[test]
fn suggestion_fat_arrow_not_triggered_without_expression() -> Result<(), Box<dyn std::error::Error>>
{
    // expected does not contain "expression", so fat arrow should not trigger
    let err = ParseError::unexpected("statement", "=>", 0);
    assert!(err.suggestion().is_none());
    Ok(())
}

#[test]
fn suggestion_arrow_not_triggered_without_expression() -> Result<(), Box<dyn std::error::Error>> {
    // expected does not contain "expression", so arrow should not trigger
    let err = ParseError::unexpected("statement", "->", 0);
    assert!(err.suggestion().is_none());
    Ok(())
}

#[test]
fn suggestion_for_variable_expected_mentions_perl_sigil_forms()
-> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::unexpected("variable", "identifier", 4);
    let sug = must_some(err.suggestion());

    assert!(sug.contains("$foo"), "got: {sug}");
    assert!(sug.contains("@bar"), "got: {sug}");
    assert!(sug.contains("%hash"), "got: {sug}");
    Ok(())
}

// ---------------------------------------------------------------------------
// ParseError as dyn std::error::Error — trait object dispatch
// ---------------------------------------------------------------------------

#[test]
fn parse_error_is_send_and_sync() -> Result<(), Box<dyn std::error::Error>> {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ParseError>();
    Ok(())
}

#[test]
fn parse_error_as_boxed_error() -> Result<(), Box<dyn std::error::Error>> {
    let err: Box<dyn std::error::Error> = Box::new(ParseError::InvalidString);
    let display = format!("{err}");
    assert!(!display.is_empty());
    Ok(())
}

#[test]
fn parse_error_all_variants_as_dyn_error() -> Result<(), Box<dyn std::error::Error>> {
    let errors: Vec<Box<dyn std::error::Error>> = vec![
        Box::new(ParseError::UnexpectedEof),
        Box::new(ParseError::unexpected("a", "b", 0)),
        Box::new(ParseError::syntax("msg", 0)),
        Box::new(ParseError::LexerError { message: "lex".into() }),
        Box::new(ParseError::RecursionLimit),
        Box::new(ParseError::InvalidNumber { literal: "x".into() }),
        Box::new(ParseError::InvalidString),
        Box::new(ParseError::UnclosedDelimiter { delimiter: '(' }),
        Box::new(ParseError::InvalidRegex { message: "re".into() }),
        Box::new(ParseError::NestingTooDeep { depth: 1, max_depth: 1 }),
    ];
    for err in &errors {
        let msg = format!("{err}");
        assert!(!msg.is_empty(), "Display should not be empty for {err:?}");
    }
    Ok(())
}

#[test]
fn parse_error_source_is_none_for_all_variants() -> Result<(), Box<dyn std::error::Error>> {
    use std::error::Error;
    let variants: Vec<ParseError> = vec![
        ParseError::UnexpectedEof,
        ParseError::unexpected("a", "b", 0),
        ParseError::syntax("msg", 0),
        ParseError::LexerError { message: "lex".into() },
        ParseError::RecursionLimit,
        ParseError::InvalidNumber { literal: "x".into() },
        ParseError::InvalidString,
        ParseError::UnclosedDelimiter { delimiter: '(' },
        ParseError::InvalidRegex { message: "re".into() },
        ParseError::NestingTooDeep { depth: 1, max_depth: 1 },
    ];
    for v in &variants {
        assert!(v.source().is_none(), "source() should be None for {v:?}");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// ParseResult with ? propagation through From conversion
// ---------------------------------------------------------------------------

#[test]
fn parse_result_question_mark_propagates_regex_error() -> Result<(), Box<dyn std::error::Error>> {
    fn inner() -> ParseResult<()> {
        let regex_err = perl_regex::RegexError::Syntax { message: "bad group".into(), offset: 3 };
        Err(regex_err.into())
    }
    let result = inner();
    assert!(result.is_err());
    if let Err(ParseError::SyntaxError { message, location }) = result {
        assert_eq!(message, "bad group");
        assert_eq!(location, 3);
    } else {
        return Err("expected SyntaxError from RegexError conversion".into());
    }
    Ok(())
}

#[test]
fn parse_result_question_mark_short_circuit() -> Result<(), Box<dyn std::error::Error>> {
    fn step1() -> ParseResult<u32> {
        Err(ParseError::RecursionLimit)
    }
    fn step2() -> ParseResult<u32> {
        let val = step1()?;
        Ok(val + 1)
    }
    let result = step2();
    assert!(result.is_err());
    if let Err(ref e) = result {
        assert_eq!(*e, ParseError::RecursionLimit);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// ParseError::syntax with various Into<String> argument types
// ---------------------------------------------------------------------------

#[test]
fn syntax_constructor_with_string() -> Result<(), Box<dyn std::error::Error>> {
    let owned = String::from("owned message");
    let err = ParseError::syntax(owned, 5);
    if let ParseError::SyntaxError { message, location } = &err {
        assert_eq!(message, "owned message");
        assert_eq!(*location, 5);
    } else {
        return Err("expected SyntaxError".into());
    }
    Ok(())
}

#[test]
fn syntax_constructor_with_str_ref() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::syntax("str ref message", 0);
    if let ParseError::SyntaxError { message, .. } = &err {
        assert_eq!(message, "str ref message");
    } else {
        return Err("expected SyntaxError".into());
    }
    Ok(())
}

#[test]
fn unexpected_constructor_with_string() -> Result<(), Box<dyn std::error::Error>> {
    let expected_str = String::from("semicolon");
    let found_str = String::from("brace");
    let err = ParseError::unexpected(expected_str, found_str, 42);
    if let ParseError::UnexpectedToken { expected, found, location } = &err {
        assert_eq!(expected, "semicolon");
        assert_eq!(found, "brace");
        assert_eq!(*location, 42);
    } else {
        return Err("expected UnexpectedToken".into());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// ErrorContext Clone and Debug traits
// ---------------------------------------------------------------------------

#[test]
fn error_context_clone_preserves_all_fields() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = ErrorContext {
        error: ParseError::syntax("test error", 10),
        line: 5,
        column: 20,
        source_line: "my $x = 42;".into(),
        suggestion: Some("add semicolon".into()),
    };
    let cloned = ctx.clone();
    assert_eq!(cloned.error, ctx.error);
    assert_eq!(cloned.line, ctx.line);
    assert_eq!(cloned.column, ctx.column);
    assert_eq!(cloned.source_line, ctx.source_line);
    assert_eq!(cloned.suggestion, ctx.suggestion);
    Ok(())
}

#[test]
fn error_context_clone_with_none_suggestion() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = ErrorContext {
        error: ParseError::UnexpectedEof,
        line: 0,
        column: 0,
        source_line: String::new(),
        suggestion: None,
    };
    let cloned = ctx.clone();
    assert!(cloned.suggestion.is_none());
    assert_eq!(cloned.error, ParseError::UnexpectedEof);
    Ok(())
}

#[test]
fn error_context_debug_format_contains_fields() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = ErrorContext {
        error: ParseError::InvalidString,
        line: 3,
        column: 7,
        source_line: "code here".into(),
        suggestion: Some("fix it".into()),
    };
    let dbg = format!("{ctx:?}");
    assert!(dbg.contains("ErrorContext"), "got: {dbg}");
    assert!(dbg.contains("line"), "got: {dbg}");
    assert!(dbg.contains("column"), "got: {dbg}");
    Ok(())
}

// ---------------------------------------------------------------------------
// ParseOutput Debug trait
// ---------------------------------------------------------------------------

#[test]
fn parse_output_debug_format() -> Result<(), Box<dyn std::error::Error>> {
    use perl_ast::{Node, NodeKind, SourceLocation};
    let ast =
        Node::new(NodeKind::Program { statements: vec![] }, SourceLocation { start: 0, end: 0 });
    let output = ParseOutput::success(ast);
    let dbg = format!("{output:?}");
    assert!(dbg.contains("ParseOutput"), "got: {dbg}");
    assert!(dbg.contains("terminated_early"), "got: {dbg}");
    Ok(())
}

#[test]
fn parse_output_debug_with_errors() -> Result<(), Box<dyn std::error::Error>> {
    use perl_ast::{Node, NodeKind, SourceLocation};
    let ast =
        Node::new(NodeKind::Program { statements: vec![] }, SourceLocation { start: 0, end: 0 });
    let errors = vec![ParseError::UnexpectedEof, ParseError::InvalidString];
    let output = ParseOutput::with_errors(ast, errors);
    let dbg = format!("{output:?}");
    assert!(dbg.contains("diagnostics"), "got: {dbg}");
    Ok(())
}

// ---------------------------------------------------------------------------
// get_error_contexts — mixed location/no-location errors
// ---------------------------------------------------------------------------

#[test]
fn error_contexts_mixed_located_and_unlocated_errors() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line0\nline1\nline2";
    let errors = vec![
        ParseError::syntax("located at line1", 6), // byte 6 = start of "line1"
        ParseError::UnexpectedEof,                 // no location
        ParseError::unexpected("a", "b", 12),      // byte 12 = start of "line2"
        ParseError::RecursionLimit,                // no location
    ];
    let ctxs = get_error_contexts(&errors, source);
    assert_eq!(ctxs.len(), 4);

    // Located error at line1
    assert_eq!(ctxs[0].line, 1);
    assert_eq!(ctxs[0].source_line, "line1");

    // UnexpectedEof falls back to source.len() which is end of "line2"
    // Just verify no panic
    assert!(ctxs[1].line <= 2);

    // Located error at line2
    assert_eq!(ctxs[2].line, 2);
    assert_eq!(ctxs[2].source_line, "line2");

    // RecursionLimit falls back to source.len()
    assert!(ctxs[3].line <= 2);
    Ok(())
}

#[test]
fn error_contexts_single_line_source_multiple_errors() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1 + 2";
    let errors = vec![
        ParseError::syntax("e1", 0),
        ParseError::syntax("e2", 5),
        ParseError::syntax("e3", 10),
    ];
    let ctxs = get_error_contexts(&errors, source);
    assert_eq!(ctxs.len(), 3);
    for ctx in &ctxs {
        assert_eq!(ctx.line, 0);
        assert_eq!(ctx.source_line, "my $x = 1 + 2");
    }
    Ok(())
}

#[test]
fn error_contexts_error_at_newline_boundary() -> Result<(), Box<dyn std::error::Error>> {
    let source = "abc\ndef";
    // offset 3 is the newline character
    let errors = vec![ParseError::syntax("at newline", 3)];
    let ctxs = get_error_contexts(&errors, source);
    assert_eq!(ctxs.len(), 1);
    // The newline is at the end of line 0
    assert_eq!(ctxs[0].line, 0);
    Ok(())
}

// ---------------------------------------------------------------------------
// ErrorClassifier edge cases
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
fn classifier_regex_with_double_slash_not_detected_as_unclosed()
-> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    // Source has "//" which is a comment-like pattern; should NOT be classified as UnclosedRegex
    let source = "my $x = //abc;";
    let node = make_error_node(8, 14);
    let kind = c.classify(&node, source);
    // The // check in classifier skips the regex check
    assert_ne!(kind, ParseErrorKind::UnclosedRegex);
    Ok(())
}

#[test]
fn classifier_error_at_position_zero_with_long_source() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    let source = "abcdefghijklmnop;"; // longer than 10 chars, balanced quotes
    let node = make_error_node(0, 5);
    // Just verify no panic and returns a valid classification
    let kind = c.classify(&node, source);
    // Valid line has ';' at end, no statement keywords, balanced delimiters,
    // position 0 is not near EOF. Should get InvalidSyntax.
    assert_eq!(kind, ParseErrorKind::InvalidSyntax);
    Ok(())
}

#[test]
fn classifier_error_text_window_clamps_to_source_end() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    // Source is 5 chars, error starts at 3, so error_text window of 10 would go past end
    let source = "ab;cd";
    let node = make_error_node(3, 5);
    // Should not panic; error_text is "cd" (2 chars, not 10)
    let _kind = c.classify(&node, source);
    Ok(())
}

#[test]
fn classifier_error_at_start_equals_end_of_source() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    let source = "abc;";
    // Error node starts at source.len() (past end)
    let node = make_error_node(4, 4);
    // At EOF
    let kind = c.classify(&node, source);
    assert_eq!(kind, ParseErrorKind::UnexpectedEof);
    Ok(())
}

#[test]
fn classifier_closed_regex_not_detected_as_unclosed() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    // Error text starts with / but has a closing /
    let source = "my $x = /abc/;";
    let node = make_error_node(8, 14);
    let kind = c.classify(&node, source);
    assert_ne!(kind, ParseErrorKind::UnclosedRegex);
    Ok(())
}

#[test]
fn classifier_error_text_starts_with_double_quote_ends_with_double_quote()
-> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    // Error text starts and ends with " — NOT unclosed via error_text check.
    // But the quote parity check on the full source matters.
    // Source: even number of double quotes -> not UnclosedString.
    let source = "my $v = \"ok\"; bad";
    let node = make_error_node(14, 17); // "bad"
    let kind = c.classify(&node, source);
    // Even quotes, error_text is "bad" (not starting with "), no unclosed patterns
    assert_ne!(kind, ParseErrorKind::UnclosedString);
    Ok(())
}

#[test]
fn classifier_line_ending_with_brace_not_missing_semicolon()
-> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    // Line ends with { — should not be classified as MissingSemicolon
    let source = "if (1) {\n";
    let node = make_error_node(7, 8);
    let kind = c.classify(&node, source);
    // The brace check should catch it as UnclosedBrace (not MissingSemicolon)
    assert_ne!(kind, ParseErrorKind::MissingSemicolon);
    Ok(())
}

// ---------------------------------------------------------------------------
// BudgetTracker — enter_depth tracking max_depth_reached precisely
// ---------------------------------------------------------------------------

#[test]
fn budget_tracker_depth_alternating_enter_exit() -> Result<(), Box<dyn std::error::Error>> {
    let mut t = BudgetTracker::new();

    // Enter to depth 1
    t.enter_depth();
    assert_eq!(t.current_depth, 1);
    assert_eq!(t.max_depth_reached, 1);

    // Back to 0
    t.exit_depth();
    assert_eq!(t.current_depth, 0);
    assert_eq!(t.max_depth_reached, 1);

    // Enter to depth 1 again (does not exceed max)
    t.enter_depth();
    assert_eq!(t.current_depth, 1);
    assert_eq!(t.max_depth_reached, 1);

    // Enter to depth 2 (new max)
    t.enter_depth();
    assert_eq!(t.current_depth, 2);
    assert_eq!(t.max_depth_reached, 2);

    // Exit all the way
    t.exit_depth();
    t.exit_depth();
    assert_eq!(t.current_depth, 0);
    assert_eq!(t.max_depth_reached, 2);
    Ok(())
}

#[test]
fn budget_tracker_multiple_exit_at_zero_is_safe() -> Result<(), Box<dyn std::error::Error>> {
    let mut t = BudgetTracker::new();
    // Multiple exits at zero should all saturate at 0
    for _ in 0..10 {
        t.exit_depth();
    }
    assert_eq!(t.current_depth, 0);
    assert_eq!(t.max_depth_reached, 0);
    Ok(())
}

// ---------------------------------------------------------------------------
// BudgetTracker — saturating edge cases
// ---------------------------------------------------------------------------

#[test]
fn budget_tracker_depth_enter_at_usize_max_saturates() -> Result<(), Box<dyn std::error::Error>> {
    let mut t = BudgetTracker::new();
    t.current_depth = usize::MAX;
    t.enter_depth(); // should saturate, not overflow
    assert_eq!(t.current_depth, usize::MAX);
    assert_eq!(t.max_depth_reached, usize::MAX);
    Ok(())
}

#[test]
fn budget_tracker_skip_would_exceed_with_saturating_add() -> Result<(), Box<dyn std::error::Error>>
{
    let budget = ParseBudget { max_tokens_skipped: usize::MAX, ..Default::default() };
    let mut t = BudgetTracker::new();
    t.tokens_skipped = usize::MAX - 1;
    // Adding 2 saturates to usize::MAX which is NOT > usize::MAX
    assert!(!t.skip_would_exceed(&budget, 2));
    // Adding 1 brings us exactly to MAX which is also NOT > MAX
    assert!(!t.skip_would_exceed(&budget, 1));

    // Now test with a smaller budget where overflow matters
    let budget2 = ParseBudget { max_tokens_skipped: usize::MAX - 1, ..Default::default() };
    // tokens_skipped = MAX-1, adding 1 saturates to MAX which IS > MAX-1
    assert!(t.skip_would_exceed(&budget2, 1));
    Ok(())
}

#[test]
fn budget_tracker_can_skip_more_at_max() -> Result<(), Box<dyn std::error::Error>> {
    let budget = ParseBudget::unlimited();
    let mut t = BudgetTracker::new();
    t.tokens_skipped = usize::MAX;
    // At MAX, adding 0 still works (MAX <= MAX)
    assert!(t.can_skip_more(&budget, 0));
    // Adding 1 saturates to MAX which is still <= MAX
    assert!(t.can_skip_more(&budget, 1));
    Ok(())
}

// ---------------------------------------------------------------------------
// ParseBudget — custom construction
// ---------------------------------------------------------------------------

#[test]
fn parse_budget_custom_fields() -> Result<(), Box<dyn std::error::Error>> {
    let budget =
        ParseBudget { max_errors: 42, max_depth: 10, max_tokens_skipped: 500, max_recoveries: 25 };
    assert_eq!(budget.max_errors, 42);
    assert_eq!(budget.max_depth, 10);
    assert_eq!(budget.max_tokens_skipped, 500);
    assert_eq!(budget.max_recoveries, 25);
    Ok(())
}

#[test]
fn parse_budget_partial_override_with_default() -> Result<(), Box<dyn std::error::Error>> {
    let budget = ParseBudget { max_errors: 5, ..ParseBudget::default() };
    assert_eq!(budget.max_errors, 5);
    assert_eq!(budget.max_depth, 256);
    assert_eq!(budget.max_tokens_skipped, 1000);
    assert_eq!(budget.max_recoveries, 500);
    Ok(())
}

#[test]
fn parse_budget_eq_and_ne() -> Result<(), Box<dyn std::error::Error>> {
    let a = ParseBudget::default();
    let b = ParseBudget::default();
    let c = ParseBudget::strict();
    assert_eq!(a, b);
    assert_ne!(a, c);
    Ok(())
}

// ---------------------------------------------------------------------------
// Recovery module — ParseError Debug formatting
// ---------------------------------------------------------------------------

#[test]
fn recovery_parse_error_debug_format() -> Result<(), Box<dyn std::error::Error>> {
    use perl_position_tracking::{Position, Range};
    let range = Range::new(Position::new(0, 1, 1), Position::new(5, 1, 6));
    let err = recovery::ParseError::new("test error".into(), range)
        .with_expected(vec!["semi".into()])
        .with_found("comma".into())
        .with_hint("add ;".into());
    let dbg = format!("{err:?}");
    assert!(dbg.contains("ParseError"), "got: {dbg}");
    assert!(dbg.contains("test error"), "got: {dbg}");
    assert!(dbg.contains("semi"), "got: {dbg}");
    assert!(dbg.contains("comma"), "got: {dbg}");
    Ok(())
}

#[test]
fn recovery_parse_error_range_preserved() -> Result<(), Box<dyn std::error::Error>> {
    use perl_position_tracking::{Position, Range};
    let start = Position::new(10, 2, 5);
    let end = Position::new(20, 2, 15);
    let range = Range::new(start, end);
    let err = recovery::ParseError::new("msg".into(), range);
    assert_eq!(err.range.start.byte, 10);
    assert_eq!(err.range.end.byte, 20);
    Ok(())
}

// ---------------------------------------------------------------------------
// Recovery module — SyncPoint Debug
// ---------------------------------------------------------------------------

#[test]
fn sync_point_all_variants_debug() -> Result<(), Box<dyn std::error::Error>> {
    let variants = [
        (recovery::SyncPoint::Semicolon, "Semicolon"),
        (recovery::SyncPoint::CloseBrace, "CloseBrace"),
        (recovery::SyncPoint::Keyword, "Keyword"),
        (recovery::SyncPoint::Eof, "Eof"),
    ];
    for (sp, expected) in &variants {
        let dbg = format!("{sp:?}");
        assert!(dbg.contains(expected), "got: {dbg}");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Recovery module — RecoveryResult Debug
// ---------------------------------------------------------------------------

#[test]
fn recovery_result_all_variants_debug() -> Result<(), Box<dyn std::error::Error>> {
    let variants: Vec<(recovery::RecoveryResult, &str)> = vec![
        (recovery::RecoveryResult::Recovered(0), "Recovered"),
        (recovery::RecoveryResult::AtSyncPoint, "AtSyncPoint"),
        (recovery::RecoveryResult::BudgetExhausted, "BudgetExhausted"),
        (recovery::RecoveryResult::ReachedEof, "ReachedEof"),
    ];
    for (rr, expected) in &variants {
        let dbg = format!("{rr:?}");
        assert!(dbg.contains(expected), "got: {dbg}");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// ParseError Display — exact format verification
// ---------------------------------------------------------------------------

#[test]
fn display_unexpected_eof_exact() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::UnexpectedEof;
    assert_eq!(format!("{err}"), "Unexpected end of input");
    Ok(())
}

#[test]
fn display_recursion_limit_exact() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::RecursionLimit;
    assert_eq!(format!("{err}"), "Maximum recursion depth exceeded");
    Ok(())
}

#[test]
fn display_invalid_string_exact() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::InvalidString;
    assert_eq!(format!("{err}"), "Invalid string literal");
    Ok(())
}

#[test]
fn display_unexpected_token_format() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::unexpected("semicolon", "comma", 42);
    let msg = format!("{err}");
    assert_eq!(msg, "expected semicolon, found comma at position 42");
    Ok(())
}

#[test]
fn display_syntax_error_format() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::syntax("Missing semicolon", 10);
    let msg = format!("{err}");
    assert_eq!(msg, "Invalid syntax at position 10: Missing semicolon");
    Ok(())
}

#[test]
fn display_lexer_error_format() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::LexerError { message: "bad byte".into() };
    let msg = format!("{err}");
    assert_eq!(msg, "Lexer error: bad byte");
    Ok(())
}

#[test]
fn display_invalid_number_format() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::InvalidNumber { literal: "0xGG".into() };
    let msg = format!("{err}");
    assert_eq!(msg, "Invalid number literal: 0xGG");
    Ok(())
}

#[test]
fn display_unclosed_delimiter_format() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::UnclosedDelimiter { delimiter: '{' };
    let msg = format!("{err}");
    assert_eq!(msg, "Unclosed delimiter: {");
    Ok(())
}

#[test]
fn display_invalid_regex_format() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::InvalidRegex { message: "unmatched paren".into() };
    let msg = format!("{err}");
    assert_eq!(msg, "Invalid regex: unmatched paren");
    Ok(())
}

#[test]
fn display_nesting_too_deep_format() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::NestingTooDeep { depth: 300, max_depth: 256 };
    let msg = format!("{err}");
    assert_eq!(msg, "Nesting depth limit exceeded: 300 > 256");
    Ok(())
}

// ---------------------------------------------------------------------------
// ParseError PartialEq — cross-variant inequality
// ---------------------------------------------------------------------------

#[test]
fn parse_error_ne_across_all_variant_pairs() -> Result<(), Box<dyn std::error::Error>> {
    let variants: Vec<ParseError> = vec![
        ParseError::UnexpectedEof,
        ParseError::unexpected("a", "b", 0),
        ParseError::syntax("msg", 0),
        ParseError::LexerError { message: "lex".into() },
        ParseError::RecursionLimit,
        ParseError::InvalidNumber { literal: "x".into() },
        ParseError::InvalidString,
        ParseError::UnclosedDelimiter { delimiter: '(' },
        ParseError::InvalidRegex { message: "re".into() },
        ParseError::NestingTooDeep { depth: 1, max_depth: 1 },
    ];
    for i in 0..variants.len() {
        for j in (i + 1)..variants.len() {
            assert_ne!(
                variants[i], variants[j],
                "variants[{i}] ({:?}) should != variants[{j}] ({:?})",
                variants[i], variants[j]
            );
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// From<RegexError> — edge cases
// ---------------------------------------------------------------------------

#[test]
fn from_regex_error_with_empty_message() -> Result<(), Box<dyn std::error::Error>> {
    let regex_err = perl_regex::RegexError::Syntax { message: String::new(), offset: 0 };
    let parse_err: ParseError = regex_err.into();
    if let ParseError::SyntaxError { message, location } = &parse_err {
        assert!(message.is_empty());
        assert_eq!(*location, 0);
    } else {
        return Err("expected SyntaxError".into());
    }
    Ok(())
}

#[test]
fn from_regex_error_with_large_offset() -> Result<(), Box<dyn std::error::Error>> {
    let regex_err = perl_regex::RegexError::Syntax { message: "err".into(), offset: usize::MAX };
    let parse_err: ParseError = regex_err.into();
    assert_eq!(parse_err.location(), Some(usize::MAX));
    Ok(())
}

// ---------------------------------------------------------------------------
// ParseOutput — edge cases
// ---------------------------------------------------------------------------

#[test]
fn parse_output_with_errors_zero_errors_is_ok() -> Result<(), Box<dyn std::error::Error>> {
    use perl_ast::{Node, NodeKind, SourceLocation};
    let ast =
        Node::new(NodeKind::Program { statements: vec![] }, SourceLocation { start: 0, end: 0 });
    let output = ParseOutput::with_errors(ast, vec![]);
    assert!(output.is_ok());
    assert!(!output.has_errors());
    assert_eq!(output.error_count(), 0);
    assert_eq!(output.budget_usage.errors_emitted, 0);
    Ok(())
}

#[test]
fn parse_output_finish_not_terminated_with_errors() -> Result<(), Box<dyn std::error::Error>> {
    use perl_ast::{Node, NodeKind, SourceLocation};
    let ast =
        Node::new(NodeKind::Program { statements: vec![] }, SourceLocation { start: 0, end: 0 });
    let mut tracker = BudgetTracker::new();
    tracker.errors_emitted = 3;
    let errors = vec![ParseError::UnexpectedEof, ParseError::InvalidString];
    let output = ParseOutput::finish(ast, errors, tracker, false);
    assert!(!output.terminated_early);
    assert!(output.has_errors());
    assert_eq!(output.error_count(), 2);
    // budget_usage.errors_emitted is from the tracker, not the diagnostics vec
    assert_eq!(output.budget_usage.errors_emitted, 3);
    Ok(())
}

// ---------------------------------------------------------------------------
// ErrorClassifier — diagnostic message content verification
// ---------------------------------------------------------------------------

#[test]
fn classifier_diagnostic_unexpected_token_format() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();
    let msg = c.get_diagnostic_message(&ParseErrorKind::UnexpectedToken {
        expected: "semicolon".into(),
        found: "brace".into(),
    });
    assert!(msg.contains("Expected"), "got: {msg}");
    assert!(msg.contains("semicolon"), "got: {msg}");
    assert!(msg.contains("brace"), "got: {msg}");
    Ok(())
}

#[test]
fn classifier_suggestion_content_verification() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();

    // MissingSemicolon suggestion mentions semicolon
    let sug = c.get_suggestion(&ParseErrorKind::MissingSemicolon).unwrap_or_default();
    assert!(sug.to_lowercase().contains("semicolon"), "got: {sug}");

    // UnclosedString suggestion mentions quote
    let sug = c.get_suggestion(&ParseErrorKind::UnclosedString).unwrap_or_default();
    assert!(sug.to_lowercase().contains("quote"), "got: {sug}");

    // UnclosedParenthesis suggestion mentions ')'
    let sug = c.get_suggestion(&ParseErrorKind::UnclosedParenthesis).unwrap_or_default();
    assert!(sug.contains(')'), "got: {sug}");

    // UnclosedBracket suggestion mentions ']'
    let sug = c.get_suggestion(&ParseErrorKind::UnclosedBracket).unwrap_or_default();
    assert!(sug.contains(']'), "got: {sug}");

    // UnclosedBrace suggestion mentions '}'
    let sug = c.get_suggestion(&ParseErrorKind::UnclosedBrace).unwrap_or_default();
    assert!(sug.contains('}'), "got: {sug}");
    Ok(())
}

// ---------------------------------------------------------------------------
// ErrorClassifier — explanation content verification
// ---------------------------------------------------------------------------

#[test]
fn classifier_explanation_content_depth() -> Result<(), Box<dyn std::error::Error>> {
    let c = ErrorClassifier::new();

    // MissingSemicolon explanation mentions exceptions
    let exp = c.get_explanation(&ParseErrorKind::MissingSemicolon).unwrap_or_default();
    assert!(exp.contains("exception"), "got: {exp}");

    // UnclosedString explanation mentions double or single quotes
    let exp = c.get_explanation(&ParseErrorKind::UnclosedString).unwrap_or_default();
    assert!(exp.contains("double") || exp.contains("single"), "got: {exp}");

    // UnclosedRegex explanation mentions m// or s///
    let exp = c.get_explanation(&ParseErrorKind::UnclosedRegex).unwrap_or_default();
    assert!(exp.contains("m/") || exp.contains("s/"), "got: {exp}");

    // UnterminatedHeredoc explanation mentions <<
    let exp = c.get_explanation(&ParseErrorKind::UnterminatedHeredoc).unwrap_or_default();
    assert!(exp.contains("<<"), "got: {exp}");
    Ok(())
}

// ---------------------------------------------------------------------------
// ParseErrorKind — inequality across all variant pairs
// ---------------------------------------------------------------------------

#[test]
fn parse_error_kind_all_simple_variants_distinct() -> Result<(), Box<dyn std::error::Error>> {
    let variants: Vec<ParseErrorKind> = vec![
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
    for i in 0..variants.len() {
        for j in (i + 1)..variants.len() {
            assert_ne!(
                variants[i], variants[j],
                "variants[{i}] ({:?}) should != variants[{j}] ({:?})",
                variants[i], variants[j]
            );
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Recovery module — empty builder fields
// ---------------------------------------------------------------------------

#[test]
fn recovery_parse_error_with_empty_expected() -> Result<(), Box<dyn std::error::Error>> {
    use perl_position_tracking::{Position, Range};
    let range = Range::new(Position::new(0, 1, 1), Position::new(1, 1, 2));
    let err = recovery::ParseError::new("msg".into(), range).with_expected(vec![]);
    assert!(err.expected.is_empty());
    Ok(())
}

#[test]
fn recovery_parse_error_with_empty_found() -> Result<(), Box<dyn std::error::Error>> {
    use perl_position_tracking::{Position, Range};
    let range = Range::new(Position::new(0, 1, 1), Position::new(1, 1, 2));
    let err = recovery::ParseError::new("msg".into(), range).with_found(String::new());
    assert!(err.found.is_empty());
    Ok(())
}

#[test]
fn recovery_parse_error_with_empty_hint() -> Result<(), Box<dyn std::error::Error>> {
    use perl_position_tracking::{Position, Range};
    let range = Range::new(Position::new(0, 1, 1), Position::new(1, 1, 2));
    let err = recovery::ParseError::new("msg".into(), range).with_hint(String::new());
    assert_eq!(err.recovery_hint.as_deref(), Some(""));
    Ok(())
}

// ---------------------------------------------------------------------------
// RecoveryResult — Recovered(0) is valid
// ---------------------------------------------------------------------------

#[test]
fn recovery_result_recovered_zero() -> Result<(), Box<dyn std::error::Error>> {
    let r = recovery::RecoveryResult::Recovered(0);
    if let recovery::RecoveryResult::Recovered(n) = r {
        assert_eq!(n, 0);
    } else {
        return Err("expected Recovered(0)".into());
    }
    Ok(())
}

#[test]
fn recovery_result_recovered_usize_max() -> Result<(), Box<dyn std::error::Error>> {
    let r = recovery::RecoveryResult::Recovered(usize::MAX);
    if let recovery::RecoveryResult::Recovered(n) = r {
        assert_eq!(n, usize::MAX);
    } else {
        return Err("expected Recovered(usize::MAX)".into());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// ParseError::location at boundary offset 0
// ---------------------------------------------------------------------------

#[test]
fn location_zero_for_unexpected_token() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::unexpected("a", "b", 0);
    assert_eq!(err.location(), Some(0));
    Ok(())
}

#[test]
fn location_zero_for_syntax_error() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::syntax("at start", 0);
    assert_eq!(err.location(), Some(0));
    Ok(())
}

#[test]
fn location_usize_max_for_syntax_error() -> Result<(), Box<dyn std::error::Error>> {
    let err = ParseError::syntax("far away", usize::MAX);
    assert_eq!(err.location(), Some(usize::MAX));
    Ok(())
}
