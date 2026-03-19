use perl_parser_core::{
    ParseBudget,
    error_recovery::ParseError as RecoveryParseError,
    // ParserContext
    parser_context::ParserContext,
};
use perl_tdd_support::must_some;

#[test]
fn context_from_empty_source() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = ParserContext::new(String::new());
    assert!(ctx.is_eof(), "empty source should be immediately at EOF");
    assert!(ctx.current_token().is_none());
    Ok(())
}

#[test]
fn advance_through_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = ParserContext::new("my $x = 42;".to_string());
    assert!(!ctx.is_eof());

    // Advance until EOF
    let mut count = 0;
    while !ctx.is_eof() {
        ctx.advance();
        count += 1;
        // Safety bound
        if count > 100 {
            return Err("infinite loop detected in token advancement".into());
        }
    }
    assert!(count > 0, "should have advanced through some tokens");
    assert!(ctx.is_eof());
    Ok(())
}

#[test]
fn peek_token_offset() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = ParserContext::new("my $x = 42;".to_string());
    // peek(0) should be same as current_token
    let current = must_some(ctx.current_token());
    let peeked = must_some(ctx.peek_token(0));
    assert_eq!(current.range().start.byte, peeked.range().start.byte);

    // peek(1) should be the next token
    let next = ctx.peek_token(1);
    assert!(next.is_some(), "should be able to peek ahead");
    Ok(())
}

#[test]
fn save_and_restore_index() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = ParserContext::new("my $x = 42;".to_string());

    let saved = ctx.current_index();
    ctx.advance();
    ctx.advance();
    assert!(ctx.current_index() > saved, "index should have advanced");

    ctx.set_index(saved);
    assert_eq!(ctx.current_index(), saved, "should be restored");
    Ok(())
}

#[test]
fn set_index_clamped_to_token_count() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = ParserContext::new("42".to_string());
    // set_index beyond token count should clamp
    ctx.set_index(9999);
    assert!(ctx.is_eof());
    Ok(())
}

#[test]
fn check_and_consume_token_type() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = ParserContext::new("42;".to_string());

    // Skip the number token
    ctx.advance();

    // Now should be at semicolon
    assert!(ctx.check(&perl_lexer::TokenType::Semicolon));
    assert!(ctx.consume(&perl_lexer::TokenType::Semicolon));
    // After consuming, should no longer match
    assert!(!ctx.check(&perl_lexer::TokenType::Semicolon));
    Ok(())
}

#[test]
fn consume_returns_false_on_mismatch() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = ParserContext::new("42".to_string());
    assert!(!ctx.consume(&perl_lexer::TokenType::Semicolon));
    Ok(())
}

#[test]
fn expect_returns_error_on_mismatch() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = ParserContext::new("42".to_string());
    let result = ctx.expect(perl_lexer::TokenType::Semicolon);
    assert!(result.is_err(), "expect should fail when token doesn't match");

    let err = result.err();
    assert!(err.is_some());
    Ok(())
}

#[test]
fn expect_eof_gives_error() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = ParserContext::new(String::new());
    let result = ctx.expect(perl_lexer::TokenType::Semicolon);
    assert!(result.is_err(), "expect at EOF should fail");
    Ok(())
}

#[test]
fn error_accumulation_and_take() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = ParserContext::new("test".to_string());

    let e1 = RecoveryParseError::new("err1".to_string(), ctx.current_position_range());
    let e2 = RecoveryParseError::new("err2".to_string(), ctx.current_position_range());
    ctx.add_error(e1);
    ctx.add_error(e2);

    let errors = ctx.take_errors();
    assert_eq!(errors.len(), 2);
    assert_eq!(errors[0].message, "err1");
    assert_eq!(errors[1].message, "err2");

    // After take, errors should be empty
    let errors_after = ctx.take_errors();
    assert!(errors_after.is_empty());
    Ok(())
}

#[test]
fn add_error_unchecked_always_adds() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = ParserContext::new("test".to_string());
    let e = RecoveryParseError::new("critical".to_string(), ctx.current_position_range());
    ctx.add_error_unchecked(e);

    let errors = ctx.take_errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].message, "critical");
    Ok(())
}

#[test]
fn source_slice_extracts_text() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = ParserContext::new("my $x = 42;".to_string());
    let token = must_some(ctx.current_token());
    let range = token.range();
    let slice = ctx.source_slice(&range);
    assert!(!slice.is_empty(), "source slice should not be empty");
    Ok(())
}

#[test]
fn current_position_at_eof_uses_last_token() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = ParserContext::new("42".to_string());
    // Advance past the only token
    ctx.advance();
    assert!(ctx.is_eof());

    let pos = ctx.current_position();
    // Should use end of last token, not zero
    assert!(pos.byte > 0, "at EOF, position should be at end of last token");
    Ok(())
}

#[test]
fn current_position_empty_source() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = ParserContext::new(String::new());
    let pos = ctx.current_position();
    assert_eq!(pos.byte, 0);
    assert_eq!(pos.line, 1);
    assert_eq!(pos.column, 1);
    Ok(())
}

#[test]
fn with_budget_sets_custom_budget() -> Result<(), Box<dyn std::error::Error>> {
    let budget = ParseBudget::strict();
    let ctx = ParserContext::with_budget("my $x;".to_string(), budget);
    assert_eq!(ctx.budget().max_errors, budget.max_errors);
    Ok(())
}

#[test]
fn depth_tracking() -> Result<(), Box<dyn std::error::Error>> {
    let mut ctx = ParserContext::new("test".to_string());

    assert!(!ctx.depth_would_exceed(), "fresh context should not exceed depth");
    assert!(ctx.enter_depth(), "should be able to enter depth");
    ctx.exit_depth();
    Ok(())
}

#[test]
fn errors_exhausted_respects_budget() -> Result<(), Box<dyn std::error::Error>> {
    let budget = ParseBudget { max_errors: 1, ..ParseBudget::default() };
    let mut ctx = ParserContext::with_budget("test".to_string(), budget);

    assert!(!ctx.errors_exhausted());

    let e = RecoveryParseError::new("err".to_string(), ctx.current_position_range());
    ctx.add_error(e);

    assert!(ctx.errors_exhausted(), "should be exhausted after max_errors reached");
    Ok(())
}

#[test]
fn add_error_returns_false_when_budget_exhausted() -> Result<(), Box<dyn std::error::Error>> {
    let budget = ParseBudget { max_errors: 1, ..ParseBudget::default() };
    let mut ctx = ParserContext::with_budget("test".to_string(), budget);

    let e1 = RecoveryParseError::new("err1".to_string(), ctx.current_position_range());
    assert!(ctx.add_error(e1), "first error should be added");

    let e2 = RecoveryParseError::new("err2".to_string(), ctx.current_position_range());
    assert!(!ctx.add_error(e2), "second error should be rejected (budget exhausted)");
    Ok(())
}
