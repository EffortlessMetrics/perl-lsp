//! Unit-level invariant tests for `TokenStream`.
//!
//! These tests exercise the individual stream guarantees — lookahead consistency,
//! EOF stickiness, mode boundary behaviour, and construction mode equivalence —
//! rather than integration-level parsing scenarios covered by the other
//! token_stream_* test files.

use perl_parser_core::token_stream::{Token, TokenKind, TokenStream};
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Lookahead consistency
// ---------------------------------------------------------------------------

/// `peek` and `peek_second` do not advance the stream.
/// Calling both and then `next` must return the same token `peek` showed.
#[test]
fn peek_does_not_advance_stream() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("my $x;");

    let peeked_kind = must(stream.peek()).kind;
    // Reading peek_second must not disturb what peek promises.
    let _second_kind = must(stream.peek_second()).kind;
    // peek again — must still return the same token.
    let peeked_again_kind = must(stream.peek()).kind;
    assert_eq!(peeked_kind, peeked_again_kind, "peek must be idempotent");

    // next() must return exactly what peek() showed.
    let consumed = must(stream.next());
    assert_eq!(consumed.kind, peeked_kind, "next() must return the token that peek() promised");
    Ok(())
}

/// `peek_second` then `peek` does not skip the first token.
/// After consuming with `next`, the previously-peeked second token becomes first.
#[test]
fn peek_second_then_peek_preserves_ordering() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("my $x;");

    let first_kind = must(stream.peek()).kind;
    let second_kind = must(stream.peek_second()).kind;

    // Consume the first token.
    let consumed_first = must(stream.next());
    assert_eq!(consumed_first.kind, first_kind, "consumed token must match first peek");

    // After consuming first, peek() should now show what was peek_second before.
    let new_first_kind = must(stream.peek()).kind;
    assert_eq!(
        new_first_kind, second_kind,
        "after consuming first, peek() must return the former peek_second"
    );
    Ok(())
}

/// `peek_third` then repeated peeks do not advance the stream.
#[test]
fn three_token_lookahead_is_consistent() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("my $x = 42;");

    let first = must(stream.peek()).kind;
    let second = must(stream.peek_second()).kind;
    let third = must(stream.peek_third()).kind;

    // All three slots must be stable — re-read them.
    assert_eq!(must(stream.peek()).kind, first, "peek() must be stable after peek_third");
    assert_eq!(
        must(stream.peek_second()).kind,
        second,
        "peek_second() must be stable after peek_third"
    );
    assert_eq!(
        must(stream.peek_third()).kind,
        third,
        "peek_third() must be stable after re-reading"
    );

    // Consuming first shifts the window.
    must(stream.next());
    assert_eq!(
        must(stream.peek()).kind,
        second,
        "after one next(), peek() must equal former peek_second()"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// EOF stickiness
// ---------------------------------------------------------------------------

/// Once `is_eof` returns true, repeated `next()` calls must keep returning EOF.
#[test]
fn eof_is_sticky_after_exhaustion() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("x");

    // Drain the single real token.
    while !stream.is_eof() {
        must(stream.next());
    }

    // Now we are at EOF. Call next() several more times — must always return Eof.
    for _ in 0..5 {
        let tok = must(stream.next());
        assert_eq!(tok.kind, TokenKind::Eof, "next() after EOF must return Eof sentinel");
    }
    Ok(())
}

/// `is_eof` on an empty live-lexer stream is immediately true.
#[test]
fn empty_live_stream_is_eof_immediately() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("");
    assert!(stream.is_eof(), "empty input must be immediately EOF");
    Ok(())
}

/// `is_eof` on an empty `from_vec` stream (no tokens at all) is immediately true.
#[test]
fn empty_buffered_stream_is_eof_immediately() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::from_vec(vec![]);
    assert!(stream.is_eof(), "empty from_vec must be immediately EOF");
    Ok(())
}

// ---------------------------------------------------------------------------
// Single-token streams
// ---------------------------------------------------------------------------

/// A buffered stream with only one real token: lookahead beyond it returns Eof cleanly.
///
/// Note: the live-lexer mode uses `lexer.next_token()` which returns `None` after
/// EOF has been emitted; at that point `next_token_from_lexer` yields
/// `Err(UnexpectedEof)` rather than a second Eof token. This is a known live-lexer
/// limitation. The buffered (`from_vec`) path synthesises Eof correctly.
#[test]
fn single_token_buffered_lookahead_beyond_gives_eof_extended()
-> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::from_vec(vec![Token::new(TokenKind::Number, "42", 0, 2)]);

    let first_kind = must(stream.peek()).kind;
    assert_eq!(first_kind, TokenKind::Number, "first token must be Number");

    // peek_second must return Eof for a single-token buffered stream.
    let second_kind = must(stream.peek_second()).kind;
    assert_eq!(
        second_kind,
        TokenKind::Eof,
        "peek_second on single-token buffered stream must be Eof"
    );

    // peek_third must also return Eof cleanly.
    let third_kind = must(stream.peek_third()).kind;
    assert_eq!(
        third_kind,
        TokenKind::Eof,
        "peek_third on single-token buffered stream must be Eof"
    );
    Ok(())
}

/// In live-lexer mode, is_eof() works correctly on single-token input after consuming it.
/// This validates the EOF stickiness path through peek() rather than peek_second().
#[test]
fn single_token_live_stream_is_eof_after_consuming() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("42");

    let first_kind = must(stream.peek()).kind;
    assert_ne!(first_kind, TokenKind::Eof, "first token must not be Eof");

    // Consuming the single token must leave the stream at Eof.
    must(stream.next());
    assert!(stream.is_eof(), "stream must be EOF after consuming the single token");
    Ok(())
}

// ---------------------------------------------------------------------------
// on_stmt_boundary
// ---------------------------------------------------------------------------

/// `on_stmt_boundary` clears the peek cache. In buffered mode, the cleared cache
/// means tokens that were in the lookahead slots are dropped; tokens not yet
/// fetched into the lookahead buffer are still available. This tests the invariant
/// that `on_stmt_boundary` in buffered mode leaves the stream readable after the
/// already-buffered window, and that the stream reaches EOF eventually.
#[test]
fn on_stmt_boundary_in_buffered_mode_clears_cache_stream_remains_usable()
-> Result<(), Box<dyn std::error::Error>> {
    // Use a multi-token buffered stream.
    let tokens = vec![
        Token::new(TokenKind::My, "my", 0, 2),
        Token::new(TokenKind::Identifier, "x", 3, 4),
        Token::new(TokenKind::Semicolon, ";", 4, 5),
    ];
    let mut stream = TokenStream::from_vec(tokens);

    // Prime peek — this pops "my" and "x" from the buffer into slots.
    let first_kind = must(stream.peek()).kind;
    assert_eq!(first_kind, TokenKind::My, "first peek must be My");
    let second_kind = must(stream.peek_second()).kind;
    assert_eq!(second_kind, TokenKind::Identifier, "second peek must be Identifier");

    // Clearing the cache in buffered mode discards what was pre-fetched.
    stream.on_stmt_boundary();

    // The stream must remain usable — it should eventually reach EOF.
    let mut count = 0;
    while !stream.is_eof() {
        must(stream.next());
        count += 1;
        if count > 20 {
            return Err("on_stmt_boundary left stream in an infinite loop".into());
        }
    }
    // After boundary clear, only un-fetched tokens remain (\";\" was not in a peek slot).
    assert_eq!(count, 1, "only the un-fetched Semicolon should remain after boundary clear");
    Ok(())
}

/// `on_stmt_boundary` in live-lexer mode clears peek cache and resets lexer mode.
/// After consuming some tokens manually (before priming any peek), boundary is safe.
#[test]
fn on_stmt_boundary_after_consuming_is_safe() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("my $x; my $y;");

    // Consume "my" and "$x" manually.
    must(stream.next());
    must(stream.next());

    // At the semicolon boundary: calling on_stmt_boundary with no peek primed is safe.
    stream.on_stmt_boundary();

    // The stream must still be functional.
    assert!(!stream.is_eof(), "stream must not be EOF after on_stmt_boundary mid-input");
    Ok(())
}

// ---------------------------------------------------------------------------
// invalidate_peek
// ---------------------------------------------------------------------------

/// `invalidate_peek` clears all cached lookahead slots without rolling back the
/// underlying lexer position. After invalidation, `peek()` re-fetches from the
/// current lexer position (the token AFTER the pre-fetched window), and the stream
/// must reach EOF without errors.
///
/// This is the intended contract: `invalidate_peek` is paired with `relex_as_term`
/// (which restores the lexer position) for context-sensitive re-lexing. Used alone,
/// it simply drops the cached window.
#[test]
fn invalidate_peek_clears_cache_stream_reaches_eof() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("my $x = 1;");

    // Prime all three lookahead slots — this pre-fetches "my", "$x", "=".
    let _ = must(stream.peek()).kind;
    let _ = must(stream.peek_second()).kind;
    let _ = must(stream.peek_third()).kind;

    // Invalidate.
    stream.invalidate_peek();

    // After invalidation, peek() re-fetches from the current lexer position.
    // The peeked window ("my", "$x", "=") is gone; the next token from the
    // lexer is whatever follows them.
    let kind_after = must(stream.peek()).kind;
    assert_ne!(
        kind_after,
        TokenKind::Eof,
        "invalidate_peek on a mid-stream input must not immediately yield EOF"
    );

    // The stream must be fully drainable without errors after invalidation.
    let mut count = 0;
    while !stream.is_eof() {
        must(stream.next());
        count += 1;
        if count > 50 {
            return Err("invalidate_peek left stream in an infinite loop".into());
        }
    }
    assert!(count > 0, "stream must have remaining tokens after invalidate_peek");
    Ok(())
}

/// `invalidate_peek` on a buffered stream clears the cache window.
/// Tokens not yet fetched into lookahead slots remain available.
#[test]
fn invalidate_peek_on_buffered_stream_drops_cached_window() -> Result<(), Box<dyn std::error::Error>>
{
    let tokens = vec![
        Token::new(TokenKind::My, "my", 0, 2),
        Token::new(TokenKind::Identifier, "x", 3, 4),
        Token::new(TokenKind::Semicolon, ";", 4, 5),
    ];
    let mut stream = TokenStream::from_vec(tokens);

    // Prime peek — "my" goes into peeked slot, "x" into peeked_second.
    let _ = must(stream.peek()).kind;
    let _ = must(stream.peek_second()).kind;

    // Invalidate drops the cache; ";" is still in the buffer (not yet fetched).
    stream.invalidate_peek();

    // Next peek should return ";" (the first un-fetched token).
    let kind_after = must(stream.peek()).kind;
    assert_eq!(
        kind_after,
        TokenKind::Semicolon,
        "after invalidate_peek, peek must return the next un-fetched token"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// from_vec vs. live-lex equivalence
// ---------------------------------------------------------------------------

/// `from_vec` and `TokenStream::new` must produce identical token kind sequences
/// for simple input that does not rely on context-sensitive re-lexing.
#[test]
fn from_vec_and_live_lex_produce_identical_sequences() -> Result<(), Box<dyn std::error::Error>> {
    let input = "my $x = 42;";

    // Collect from live lexer.
    let mut live = TokenStream::new(input);
    let mut live_kinds: Vec<TokenKind> = Vec::new();
    while !live.is_eof() {
        live_kinds.push(must(live.next()).kind);
    }

    // Build the equivalent pre-lexed token list using the conversion helper.
    use perl_lexer::{PerlLexer, TokenType};
    let mut raw_lexer = PerlLexer::new(input);
    let mut raw_tokens = Vec::new();
    while let Some(t) = raw_lexer.next_token() {
        if matches!(t.token_type, TokenType::EOF) {
            break;
        }
        raw_tokens.push(t);
    }
    let parser_tokens = TokenStream::lexer_tokens_to_parser_tokens(raw_tokens);
    let mut buffered = TokenStream::from_vec(parser_tokens);
    let mut buffered_kinds: Vec<TokenKind> = Vec::new();
    while !buffered.is_eof() {
        buffered_kinds.push(must(buffered.next()).kind);
    }

    assert_eq!(
        live_kinds, buffered_kinds,
        "live-lex and from_vec must produce identical token kind sequences"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// enter_format_mode (no-op in buffered mode)
// ---------------------------------------------------------------------------

/// `enter_format_mode` on a buffered stream must be a no-op:
/// the stream must continue producing the same tokens unchanged.
#[test]
fn enter_format_mode_is_noop_in_buffered_mode() -> Result<(), Box<dyn std::error::Error>> {
    let tokens =
        vec![Token::new(TokenKind::My, "my", 0, 2), Token::new(TokenKind::Identifier, "x", 3, 4)];
    let mut stream = TokenStream::from_vec(tokens);

    // This should be a no-op without panicking.
    stream.enter_format_mode();

    let first = must(stream.next());
    assert_eq!(first.kind, TokenKind::My, "first token must be My after enter_format_mode no-op");

    let second = must(stream.next());
    assert_eq!(
        second.kind,
        TokenKind::Identifier,
        "second token must be Identifier after enter_format_mode no-op"
    );
    Ok(())
}
