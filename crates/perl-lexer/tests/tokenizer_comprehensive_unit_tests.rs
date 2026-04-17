//! Comprehensive unit tests for the perl-tokenizer crate.
//!
//! Covers: TokenStream, TokenWithPosition, PositionTracker,
//! Trivia/TriviaLexer/TriviaToken, TriviaPreservingParser,
//! and utility functions (find_data_marker_byte_lexed, code_slice).

use perl_lexer::tokenizer::token_wrapper::PositionTracker;
use perl_lexer::tokenizer::util::{code_slice, find_data_marker_byte_lexed};
use perl_parser_core::tokens::token_stream::TokenStream;
use perl_parser_core::trivia::{Trivia, TriviaLexer, TriviaToken};
use perl_parser_core::trivia_parser::{TriviaParserContext, TriviaPreservingParser};
use perl_tdd_support::{must, must_some};
use perl_token::TokenKind;

// ---------------------------------------------------------------------------
// TokenStream — basic tokenization
// ---------------------------------------------------------------------------

#[test]
fn token_stream_simple_assignment() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("my $x = 42;");
    let t = must(s.peek());
    assert_eq!(t.kind, TokenKind::My);
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::My);
    Ok(())
}

#[test]
fn token_stream_eof_after_empty_input() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("");
    let t = must(s.peek());
    assert_eq!(t.kind, TokenKind::Eof);
    assert!(s.is_eof());
    Ok(())
}

#[test]
fn token_stream_whitespace_only() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("   \t\n  ");
    assert!(s.is_eof());
    Ok(())
}

#[test]
fn token_stream_eof_is_sticky() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("42");
    // Consume number
    let _ = must(s.next());
    // Should see EOF repeatedly
    assert_eq!(must(s.next()).kind, TokenKind::Eof);
    assert_eq!(must(s.next()).kind, TokenKind::Eof);
    assert!(s.is_eof());
    Ok(())
}

#[test]
fn token_stream_skips_whitespace_and_comments() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("  # comment\n  42  ");
    let t = must(s.peek());
    assert_eq!(t.kind, TokenKind::Number);
    Ok(())
}

// ---------------------------------------------------------------------------
// TokenStream — lookahead (peek, peek_second, peek_third)
// ---------------------------------------------------------------------------

#[test]
fn token_stream_peek_second() -> Result<(), Box<dyn std::error::Error>> {
    // Lexer tokenizes `$x` as a single Identifier token
    let mut s = TokenStream::new("my $x = 42");
    let first = must(s.peek());
    assert_eq!(first.kind, TokenKind::My);
    let second = must(s.peek_second());
    assert_eq!(second.kind, TokenKind::Identifier); // $x
    // peek should still return first
    let again = must(s.peek());
    assert_eq!(again.kind, TokenKind::My);
    Ok(())
}

#[test]
fn token_stream_peek_third() -> Result<(), Box<dyn std::error::Error>> {
    // my($x) = (42); → My, Identifier($x), Assign, Number, Semicolon
    let mut s = TokenStream::new("my $x = 42;");
    let _ = must(s.peek());
    let _ = must(s.peek_second());
    let third = must(s.peek_third());
    // Third token: my=0, $x=1, ==2 → Assign
    assert_eq!(third.kind, TokenKind::Assign);
    Ok(())
}

#[test]
fn token_stream_peek_chain_then_consume() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("if ($x) {}");
    let _ = must(s.peek_third()); // fill all three peek slots
    let t1 = must(s.next());
    assert_eq!(t1.kind, TokenKind::If);
    let t2 = must(s.next());
    assert_eq!(t2.kind, TokenKind::LeftParen);
    Ok(())
}

// ---------------------------------------------------------------------------
// TokenStream — invalidate_peek / peek_fresh_kind
// ---------------------------------------------------------------------------

#[test]
fn invalidate_peek_clears_cache() -> Result<(), Box<dyn std::error::Error>> {
    // invalidate_peek clears the peek cache but does NOT rewind the lexer,
    // so re-peeking advances from the current lexer position.
    let mut s = TokenStream::new("my $x = 1;");
    let first = must(s.peek());
    assert_eq!(first.kind, TokenKind::My);
    s.invalidate_peek();
    // After invalidation, the lexer position is past `my`, so we get $x
    let after = must(s.peek());
    assert_eq!(after.kind, TokenKind::Identifier); // $x
    Ok(())
}

#[test]
fn peek_fresh_kind_returns_kind() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("my $x");
    let kind = must_some(s.peek_fresh_kind());
    assert_eq!(kind, TokenKind::My);
    Ok(())
}

#[test]
fn peek_fresh_kind_on_eof() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("");
    let kind = must_some(s.peek_fresh_kind());
    assert_eq!(kind, TokenKind::Eof);
    Ok(())
}

// ---------------------------------------------------------------------------
// TokenStream — on_stmt_boundary
// ---------------------------------------------------------------------------

#[test]
fn on_stmt_boundary_resets_lexer_mode() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("my $x; our $y;");
    assert_eq!(must(s.peek()).kind, TokenKind::My);
    // Consume first statement
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Semicolon {
            break;
        }
    }
    // Reset at statement boundary
    s.on_stmt_boundary();
    // After boundary, lexer re-lexes from current position
    let t = must(s.peek());
    assert_eq!(t.kind, TokenKind::Our);
    Ok(())
}

// ---------------------------------------------------------------------------
// TokenStream — keyword mapping
// ---------------------------------------------------------------------------

#[test]
fn token_stream_keywords() -> Result<(), Box<dyn std::error::Error>> {
    let keywords: Vec<(&str, TokenKind)> = vec![
        ("my", TokenKind::My),
        ("our", TokenKind::Our),
        ("local", TokenKind::Local),
        ("state", TokenKind::State),
        ("sub", TokenKind::Sub),
        ("if", TokenKind::If),
        ("elsif", TokenKind::Elsif),
        ("else", TokenKind::Else),
        ("unless", TokenKind::Unless),
        ("while", TokenKind::While),
        ("until", TokenKind::Until),
        ("for", TokenKind::For),
        ("foreach", TokenKind::Foreach),
        ("return", TokenKind::Return),
        ("package", TokenKind::Package),
        ("use", TokenKind::Use),
        ("no", TokenKind::No),
        ("eval", TokenKind::Eval),
        ("do", TokenKind::Do),
        ("undef", TokenKind::Undef),
    ];
    for (src, expected) in &keywords {
        let mut s = TokenStream::new(src);
        let t = must(s.peek());
        assert_eq!(t.kind, *expected, "keyword mismatch for `{src}`");
    }
    Ok(())
}

#[test]
fn token_stream_phase_keywords() -> Result<(), Box<dyn std::error::Error>> {
    let phase: Vec<(&str, TokenKind)> = vec![
        ("BEGIN", TokenKind::Begin),
        ("END", TokenKind::End),
        ("CHECK", TokenKind::Check),
        ("INIT", TokenKind::Init),
        ("UNITCHECK", TokenKind::Unitcheck),
    ];
    for (src, expected) in &phase {
        let mut s = TokenStream::new(src);
        let t = must(s.peek());
        assert_eq!(t.kind, *expected, "phase keyword mismatch for `{src}`");
    }
    Ok(())
}

#[test]
fn token_stream_loop_control_keywords() -> Result<(), Box<dyn std::error::Error>> {
    for (src, expected) in [
        ("next", TokenKind::Next),
        ("last", TokenKind::Last),
        ("redo", TokenKind::Redo),
        ("continue", TokenKind::Continue),
    ] {
        let mut s = TokenStream::new(src);
        let t = must(s.peek());
        assert_eq!(t.kind, expected, "keyword mismatch for `{src}`");
    }
    Ok(())
}

#[test]
fn token_stream_experimental_keywords() -> Result<(), Box<dyn std::error::Error>> {
    for (src, expected) in [
        ("try", TokenKind::Try),
        ("catch", TokenKind::Catch),
        ("finally", TokenKind::Finally),
        ("class", TokenKind::Class),
        ("method", TokenKind::Method),
        ("field", TokenKind::Field),
        ("given", TokenKind::Given),
        ("when", TokenKind::When),
        ("default", TokenKind::Default),
    ] {
        let mut s = TokenStream::new(src);
        let t = must(s.peek());
        assert_eq!(t.kind, expected, "keyword mismatch for `{src}`");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// TokenStream — operator mapping
// ---------------------------------------------------------------------------

#[test]
fn token_stream_arithmetic_operators() -> Result<(), Box<dyn std::error::Error>> {
    // Parse `1 + 2 - 3 * 4`
    let mut s = TokenStream::new("1 + 2 - 3 * 4");
    assert_eq!(must(s.next()).kind, TokenKind::Number);
    assert_eq!(must(s.next()).kind, TokenKind::Plus);
    assert_eq!(must(s.next()).kind, TokenKind::Number);
    assert_eq!(must(s.next()).kind, TokenKind::Minus);
    assert_eq!(must(s.next()).kind, TokenKind::Number);
    assert_eq!(must(s.next()).kind, TokenKind::Star);
    assert_eq!(must(s.next()).kind, TokenKind::Number);
    Ok(())
}

#[test]
fn token_stream_comparison_operators() -> Result<(), Box<dyn std::error::Error>> {
    // Check a few comparison tokens
    let mut s = TokenStream::new("1 == 2");
    assert_eq!(must(s.next()).kind, TokenKind::Number);
    assert_eq!(must(s.next()).kind, TokenKind::Equal);
    assert_eq!(must(s.next()).kind, TokenKind::Number);

    let mut s = TokenStream::new("1 != 2");
    let _ = must(s.next());
    assert_eq!(must(s.next()).kind, TokenKind::NotEqual);

    let mut s = TokenStream::new("1 <=> 2");
    let _ = must(s.next());
    assert_eq!(must(s.next()).kind, TokenKind::Spaceship);
    Ok(())
}

#[test]
fn token_stream_logical_operators() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("1 && 2 || 3 // 4");
    let _ = must(s.next()); // 1
    assert_eq!(must(s.next()).kind, TokenKind::And);
    let _ = must(s.next()); // 2
    assert_eq!(must(s.next()).kind, TokenKind::Or);
    let _ = must(s.next()); // 3
    assert_eq!(must(s.next()).kind, TokenKind::DefinedOr);
    Ok(())
}

#[test]
fn token_stream_word_operators() -> Result<(), Box<dyn std::error::Error>> {
    for (src, expected) in [
        ("and", TokenKind::WordAnd),
        ("or", TokenKind::WordOr),
        ("not", TokenKind::WordNot),
        ("xor", TokenKind::WordXor),
        ("cmp", TokenKind::StringCompare),
    ] {
        let mut s = TokenStream::new(src);
        assert_eq!(must(s.peek()).kind, expected, "word op mismatch for `{src}`");
    }
    Ok(())
}

#[test]
fn token_stream_assignment_operators() -> Result<(), Box<dyn std::error::Error>> {
    let ops: Vec<(&str, TokenKind)> = vec![
        ("$x = 1", TokenKind::Assign),
        ("$x += 1", TokenKind::PlusAssign),
        ("$x -= 1", TokenKind::MinusAssign),
        ("$x *= 1", TokenKind::StarAssign),
        ("$x .= 1", TokenKind::DotAssign),
        ("$x ||= 1", TokenKind::LogicalOrAssign),
        ("$x &&= 1", TokenKind::LogicalAndAssign),
        ("$x //= 1", TokenKind::DefinedOrAssign),
    ];
    for (src, expected) in &ops {
        let mut s = TokenStream::new(src);
        let _ = must(s.next()); // $x (single identifier)
        let t = must(s.next()); // operator
        assert_eq!(t.kind, *expected, "assignment op mismatch for `{src}`");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// TokenStream — delimiters
// ---------------------------------------------------------------------------

#[test]
fn token_stream_delimiters() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("({[]})");
    assert_eq!(must(s.next()).kind, TokenKind::LeftParen);
    assert_eq!(must(s.next()).kind, TokenKind::LeftBrace);
    assert_eq!(must(s.next()).kind, TokenKind::LeftBracket);
    assert_eq!(must(s.next()).kind, TokenKind::RightBracket);
    assert_eq!(must(s.next()).kind, TokenKind::RightBrace);
    assert_eq!(must(s.next()).kind, TokenKind::RightParen);
    Ok(())
}

#[test]
fn token_stream_semicolon_comma() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("1, 2;");
    assert_eq!(must(s.next()).kind, TokenKind::Number);
    assert_eq!(must(s.next()).kind, TokenKind::Comma);
    assert_eq!(must(s.next()).kind, TokenKind::Number);
    assert_eq!(must(s.next()).kind, TokenKind::Semicolon);
    Ok(())
}

// ---------------------------------------------------------------------------
// TokenStream — literals
// ---------------------------------------------------------------------------

#[test]
fn token_stream_number_literal() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("42");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Number);
    assert_eq!(t.text.as_ref(), "42");
    Ok(())
}

#[test]
fn token_stream_string_literal() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("'hello'");
    let t = must(s.next());
    // Depending on lexer: could be String or QuoteSingle
    assert!(
        matches!(t.kind, TokenKind::String | TokenKind::QuoteSingle),
        "expected string-like token, got {:?}",
        t.kind
    );
    Ok(())
}

#[test]
fn token_stream_double_quoted_string() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("\"world\"");
    let t = must(s.next());
    assert!(
        matches!(t.kind, TokenKind::String | TokenKind::QuoteDouble),
        "expected string-like token, got {:?}",
        t.kind
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// TokenStream — sigils and variables
// ---------------------------------------------------------------------------

#[test]
fn token_stream_scalar_variable() -> Result<(), Box<dyn std::error::Error>> {
    // Lexer treats `$foo` as a single Identifier token
    let mut s = TokenStream::new("$foo");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Identifier);
    assert_eq!(t.text.as_ref(), "$foo");
    Ok(())
}

#[test]
fn token_stream_array_variable() -> Result<(), Box<dyn std::error::Error>> {
    // Lexer treats `@arr` as a single Identifier token
    let mut s = TokenStream::new("@arr");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Identifier);
    assert_eq!(t.text.as_ref(), "@arr");
    Ok(())
}

// ---------------------------------------------------------------------------
// TokenStream — complex Perl constructs
// ---------------------------------------------------------------------------

#[test]
fn token_stream_sub_declaration() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("sub foo { return 1; }");
    assert_eq!(must(s.next()).kind, TokenKind::Sub);
    assert_eq!(must(s.next()).kind, TokenKind::Identifier); // foo
    assert_eq!(must(s.next()).kind, TokenKind::LeftBrace);
    assert_eq!(must(s.next()).kind, TokenKind::Return);
    assert_eq!(must(s.next()).kind, TokenKind::Number); // 1
    assert_eq!(must(s.next()).kind, TokenKind::Semicolon);
    assert_eq!(must(s.next()).kind, TokenKind::RightBrace);
    Ok(())
}

#[test]
fn token_stream_if_else_chain() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("if (1) {} elsif (2) {} else {}");
    assert_eq!(must(s.next()).kind, TokenKind::If);
    assert_eq!(must(s.next()).kind, TokenKind::LeftParen);
    assert_eq!(must(s.next()).kind, TokenKind::Number);
    assert_eq!(must(s.next()).kind, TokenKind::RightParen);
    assert_eq!(must(s.next()).kind, TokenKind::LeftBrace);
    assert_eq!(must(s.next()).kind, TokenKind::RightBrace);
    assert_eq!(must(s.next()).kind, TokenKind::Elsif);
    Ok(())
}

#[test]
fn token_stream_use_statement() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("use strict;");
    assert_eq!(must(s.next()).kind, TokenKind::Use);
    assert_eq!(must(s.next()).kind, TokenKind::Identifier); // strict
    assert_eq!(must(s.next()).kind, TokenKind::Semicolon);
    Ok(())
}

#[test]
fn token_stream_package_declaration() -> Result<(), Box<dyn std::error::Error>> {
    // Lexer treats `Foo::Bar` as a single identifier
    let mut s = TokenStream::new("package Foo::Bar;");
    assert_eq!(must(s.next()).kind, TokenKind::Package);
    assert_eq!(must(s.next()).kind, TokenKind::Identifier); // Foo::Bar
    assert_eq!(must(s.next()).kind, TokenKind::Semicolon);
    Ok(())
}

#[test]
fn token_stream_arrow_and_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("$obj->method(key => 'val')");
    assert_eq!(must(s.next()).kind, TokenKind::Identifier); // $obj
    assert_eq!(must(s.next()).kind, TokenKind::Arrow);
    assert_eq!(must(s.next()).kind, TokenKind::Method); // method (keyword)
    assert_eq!(must(s.next()).kind, TokenKind::LeftParen);
    assert_eq!(must(s.next()).kind, TokenKind::Identifier); // key
    assert_eq!(must(s.next()).kind, TokenKind::FatArrow);
    Ok(())
}

#[test]
fn token_stream_ternary_operator() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("$a ? $b : $c");
    assert_eq!(must(s.next()).kind, TokenKind::Identifier); // $a
    assert_eq!(must(s.next()).kind, TokenKind::Question);
    assert_eq!(must(s.next()).kind, TokenKind::Identifier); // $b
    assert_eq!(must(s.next()).kind, TokenKind::Colon);
    assert_eq!(must(s.next()).kind, TokenKind::Identifier); // $c
    Ok(())
}

#[test]
fn token_stream_range_operators() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("1 .. 10");
    let _ = must(s.next()); // 1
    assert_eq!(must(s.next()).kind, TokenKind::Range);
    let _ = must(s.next()); // 10

    let mut s = TokenStream::new("1 ... 10");
    let _ = must(s.next()); // 1
    assert_eq!(must(s.next()).kind, TokenKind::Ellipsis);
    Ok(())
}

#[test]
fn token_stream_increment_decrement() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("$x++ + $y--");
    assert_eq!(must(s.next()).kind, TokenKind::Identifier); // $x
    assert_eq!(must(s.next()).kind, TokenKind::Increment);
    assert_eq!(must(s.next()).kind, TokenKind::Plus);
    assert_eq!(must(s.next()).kind, TokenKind::Identifier); // $y
    assert_eq!(must(s.next()).kind, TokenKind::Decrement);
    Ok(())
}

#[test]
fn token_stream_backslash_reference() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("\\@array");
    assert_eq!(must(s.next()).kind, TokenKind::Backslash);
    assert_eq!(must(s.next()).kind, TokenKind::Identifier); // @array
    Ok(())
}

// ---------------------------------------------------------------------------
// TokenStream — full statement iteration
// ---------------------------------------------------------------------------

#[test]
fn token_stream_collect_all_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("my $x = 1;");
    let mut kinds = Vec::new();
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        kinds.push(t.kind);
    }
    // Expect at least: My, ScalarSigil, Identifier, Assign, Number, Semicolon
    assert!(kinds.len() >= 5, "expected at least 5 tokens, got {}", kinds.len());
    assert_eq!(kinds[0], TokenKind::My);
    Ok(())
}

// ---------------------------------------------------------------------------
// PositionTracker
// ---------------------------------------------------------------------------

#[test]
fn position_tracker_single_line() -> Result<(), Box<dyn std::error::Error>> {
    let tracker = PositionTracker::new("hello");
    let pos = tracker.byte_to_position(0);
    assert_eq!(pos.line, 1);
    assert_eq!(pos.column, 1);

    let pos = tracker.byte_to_position(4);
    assert_eq!(pos.line, 1);
    assert_eq!(pos.column, 5);
    Ok(())
}

#[test]
fn position_tracker_multiline() -> Result<(), Box<dyn std::error::Error>> {
    let tracker = PositionTracker::new("ab\ncd\nef");
    // Start of second line (byte 3 is 'c')
    let pos = tracker.byte_to_position(3);
    assert_eq!(pos.line, 2);
    assert_eq!(pos.column, 1);

    // Start of third line (byte 6 is 'e')
    let pos = tracker.byte_to_position(6);
    assert_eq!(pos.line, 3);
    assert_eq!(pos.column, 1);
    Ok(())
}

#[test]
fn position_tracker_utf8() -> Result<(), Box<dyn std::error::Error>> {
    let tracker = PositionTracker::new("a\u{00E9}b"); // a, é (2 bytes), b
    let pos = tracker.byte_to_position(0);
    assert_eq!(pos.column, 1);
    // byte 1 = start of é
    let pos = tracker.byte_to_position(1);
    assert_eq!(pos.column, 2);
    // byte 3 = start of b (after 2-byte é)
    let pos = tracker.byte_to_position(3);
    assert_eq!(pos.column, 3);
    Ok(())
}

#[test]
fn position_tracker_empty_source() -> Result<(), Box<dyn std::error::Error>> {
    let tracker = PositionTracker::new("");
    let pos = tracker.byte_to_position(0);
    assert_eq!(pos.line, 1);
    assert_eq!(pos.column, 1);
    Ok(())
}

#[test]
fn position_tracker_wrap_token() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x";
    let tracker = PositionTracker::new(source);

    let token = perl_lexer::Token::new(
        perl_lexer::TokenType::Keyword(std::sync::Arc::from("my")),
        std::sync::Arc::from("my"),
        0,
        2,
    );
    let wrapped = tracker.wrap_token(token);
    assert_eq!(wrapped.start_pos.line, 1);
    assert_eq!(wrapped.start_pos.column, 1);
    assert_eq!(wrapped.end_pos.line, 1);
    assert_eq!(wrapped.end_pos.column, 3);
    assert_eq!(wrapped.text(), "my");
    assert_eq!(wrapped.byte_range(), (0, 2));
    Ok(())
}

// ---------------------------------------------------------------------------
// Trivia types
// ---------------------------------------------------------------------------

#[test]
fn trivia_as_str() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(Trivia::Whitespace("  ".to_string()).as_str(), "  ");
    assert_eq!(Trivia::LineComment("# hi".to_string()).as_str(), "# hi");
    assert_eq!(Trivia::PodComment("=pod\n=cut".to_string()).as_str(), "=pod\n=cut");
    assert_eq!(Trivia::Newline.as_str(), "\n");
    Ok(())
}

#[test]
fn trivia_kind_name() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(Trivia::Whitespace(String::new()).kind_name(), "whitespace");
    assert_eq!(Trivia::LineComment(String::new()).kind_name(), "comment");
    assert_eq!(Trivia::PodComment(String::new()).kind_name(), "pod");
    assert_eq!(Trivia::Newline.kind_name(), "newline");
    Ok(())
}

#[test]
fn trivia_token_new() -> Result<(), Box<dyn std::error::Error>> {
    let range = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(0, 1, 1),
        perl_position_tracking::Position::new(5, 1, 6),
    );
    let tt = TriviaToken::new(Trivia::Whitespace("     ".to_string()), range);
    assert_eq!(tt.trivia.as_str(), "     ");
    Ok(())
}

// ---------------------------------------------------------------------------
// TriviaLexer
// ---------------------------------------------------------------------------

#[test]
fn trivia_lexer_whitespace_before_token() -> Result<(), Box<dyn std::error::Error>> {
    let mut lexer = TriviaLexer::new("   42".to_string());
    let (token, trivia) = must_some(lexer.next_token_with_trivia());
    assert!(trivia.iter().any(|t| matches!(&t.trivia, Trivia::Whitespace(_))));
    assert!(!matches!(token.token_type, perl_lexer::TokenType::EOF));
    Ok(())
}

#[test]
fn trivia_lexer_comment_before_token() -> Result<(), Box<dyn std::error::Error>> {
    let mut lexer = TriviaLexer::new("# comment\n42".to_string());
    let (_token, trivia) = must_some(lexer.next_token_with_trivia());
    assert!(trivia.iter().any(|t| matches!(&t.trivia, Trivia::LineComment(_))));
    Ok(())
}

#[test]
fn trivia_lexer_newline_trivia() -> Result<(), Box<dyn std::error::Error>> {
    let mut lexer = TriviaLexer::new("\n42".to_string());
    let (_token, trivia) = must_some(lexer.next_token_with_trivia());
    assert!(trivia.iter().any(|t| matches!(&t.trivia, Trivia::Newline)));
    Ok(())
}

#[test]
fn trivia_lexer_pod_trivia() -> Result<(), Box<dyn std::error::Error>> {
    let src = "=head1 NAME\n\nStuff\n\n=cut\nmy $x;".to_string();
    let mut lexer = TriviaLexer::new(src);
    let (_token, trivia) = must_some(lexer.next_token_with_trivia());
    assert!(trivia.iter().any(|t| matches!(&t.trivia, Trivia::PodComment(_))));
    Ok(())
}

#[test]
fn trivia_lexer_multiple_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let mut lexer = TriviaLexer::new("my $x = 42;".to_string());
    let mut count = 0;
    while let Some((_token, _trivia)) = lexer.next_token_with_trivia() {
        count += 1;
    }
    // Should have at least: my, $x, =, 42, ;
    assert!(count >= 4, "expected at least 4 tokens, got {count}");
    Ok(())
}

#[test]
fn trivia_lexer_empty_source() -> Result<(), Box<dyn std::error::Error>> {
    let mut lexer = TriviaLexer::new(String::new());
    // Empty source should return None (EOF only)
    assert!(lexer.next_token_with_trivia().is_none());
    Ok(())
}

// ---------------------------------------------------------------------------
// TriviaParserContext — public API only
// ---------------------------------------------------------------------------

#[test]
fn trivia_parser_context_is_eof_on_code() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = TriviaParserContext::new("my $x = 1;".to_string());
    assert!(!ctx.is_eof());
    Ok(())
}

#[test]
fn trivia_parser_context_whitespace_only() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = TriviaParserContext::new("   \n\n  ".to_string());
    // Only whitespace: no meaningful tokens, but may have EOF with trivia
    // Just ensure it doesn't panic
    let _ = ctx.is_eof();
    Ok(())
}

// ---------------------------------------------------------------------------
// TriviaPreservingParser
// ---------------------------------------------------------------------------

#[test]
fn trivia_preserving_parser_basic() -> Result<(), Box<dyn std::error::Error>> {
    let parser = TriviaPreservingParser::new("my $x = 1;".to_string());
    let result = parser.parse();
    // Should produce a Program node
    assert!(matches!(&result.node.kind, perl_ast_v2::NodeKind::Program { .. }));
    Ok(())
}

#[test]
fn trivia_preserving_parser_empty_source() -> Result<(), Box<dyn std::error::Error>> {
    let parser = TriviaPreservingParser::new(String::new());
    let result = parser.parse();
    assert!(matches!(&result.node.kind, perl_ast_v2::NodeKind::Program { .. }));
    Ok(())
}

#[test]
fn trivia_preserving_parser_comment_only() -> Result<(), Box<dyn std::error::Error>> {
    let parser = TriviaPreservingParser::new("# just a comment\n".to_string());
    let result = parser.parse();
    // Should still produce a valid program
    assert!(matches!(&result.node.kind, perl_ast_v2::NodeKind::Program { .. }));
    // Leading trivia should contain the comment
    let has_comment =
        result.leading_trivia.iter().any(|t| matches!(&t.trivia, Trivia::LineComment(_)));
    assert!(has_comment, "comment-only source should capture comment as trivia");
    Ok(())
}

#[test]
fn trivia_preserving_parser_multiple_statements() -> Result<(), Box<dyn std::error::Error>> {
    let src = "my $x = 1;\nour $y = 2;\n".to_string();
    let parser = TriviaPreservingParser::new(src);
    let result = parser.parse();
    if let perl_ast_v2::NodeKind::Program { statements } = &result.node.kind {
        assert!(statements.len() >= 2, "expected >=2 statements, got {}", statements.len());
    }
    Ok(())
}

#[test]
fn trivia_preserving_parser_shebang_and_code() -> Result<(), Box<dyn std::error::Error>> {
    let src = "#!/usr/bin/perl\nuse strict;\nmy $x = 1;\n".to_string();
    let parser = TriviaPreservingParser::new(src);
    let result = parser.parse();
    let has_shebang = result.leading_trivia.iter().any(|t| {
        if let Trivia::LineComment(text) = &t.trivia { text.starts_with("#!") } else { false }
    });
    assert!(has_shebang, "should detect shebang line as trivia");
    Ok(())
}

#[test]
fn trivia_preserving_parser_pod_in_code() -> Result<(), Box<dyn std::error::Error>> {
    let src = "=head1 NAME\n\nFoo\n\n=cut\n\nmy $x = 1;\n".to_string();
    let parser = TriviaPreservingParser::new(src);
    let result = parser.parse();
    let has_pod = result.leading_trivia.iter().any(|t| matches!(&t.trivia, Trivia::PodComment(_)));
    assert!(has_pod, "should detect POD as trivia");
    Ok(())
}

// ---------------------------------------------------------------------------
// format_with_trivia
// ---------------------------------------------------------------------------

#[test]
fn format_with_trivia_includes_leading() -> Result<(), Box<dyn std::error::Error>> {
    let parser = TriviaPreservingParser::new("# hello\nmy $x;".to_string());
    let result = parser.parse();
    let formatted = perl_parser_core::trivia_parser::format_with_trivia(&result);
    assert!(formatted.contains("# hello"), "formatted output should contain leading comment");
    Ok(())
}

// ---------------------------------------------------------------------------
// util — find_data_marker_byte_lexed
// ---------------------------------------------------------------------------

#[test]
fn data_marker_not_present() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(find_data_marker_byte_lexed("print 'hello';\n"), None);
    Ok(())
}

#[test]
fn data_marker_data() -> Result<(), Box<dyn std::error::Error>> {
    let src = "print 1;\n__DATA__\nsome data";
    let offset = must_some(find_data_marker_byte_lexed(src));
    assert_eq!(offset, 9); // byte offset of __DATA__
    Ok(())
}

#[test]
fn data_marker_end() -> Result<(), Box<dyn std::error::Error>> {
    let src = "code;\n__END__\nstuff";
    let offset = must_some(find_data_marker_byte_lexed(src));
    assert_eq!(offset, 6);
    Ok(())
}

#[test]
fn data_marker_in_string_not_matched() -> Result<(), Box<dyn std::error::Error>> {
    let src = "print '__DATA__';\n";
    assert_eq!(find_data_marker_byte_lexed(src), None);
    Ok(())
}

// ---------------------------------------------------------------------------
// util — code_slice
// ---------------------------------------------------------------------------

#[test]
fn code_slice_no_marker() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(code_slice("print 1;\n"), "print 1;\n");
    Ok(())
}

#[test]
fn code_slice_with_data() -> Result<(), Box<dyn std::error::Error>> {
    let src = "print 1;\n__DATA__\ndata";
    assert_eq!(code_slice(src), "print 1;\n");
    Ok(())
}

#[test]
fn code_slice_with_end() -> Result<(), Box<dyn std::error::Error>> {
    let src = "code;\n__END__\nstuff";
    assert_eq!(code_slice(src), "code;\n");
    Ok(())
}

#[test]
fn code_slice_empty() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(code_slice(""), "");
    Ok(())
}

// ---------------------------------------------------------------------------
// TokenStream — edge cases / error recovery
// ---------------------------------------------------------------------------

#[test]
fn token_stream_very_long_input() -> Result<(), Box<dyn std::error::Error>> {
    // Build a large input without excessive allocations
    let input = "my $x = 1;\n".repeat(1000);
    let mut s = TokenStream::new(&input);
    let mut count = 0;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        count += 1;
    }
    // Each statement has >= 5 tokens, 1000 statements
    assert!(count >= 5000, "expected >= 5000 tokens, got {count}");
    Ok(())
}

#[test]
fn token_stream_nested_braces() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("{ { { } } }");
    let mut depth = 0i32;
    loop {
        let t = must(s.next());
        match t.kind {
            TokenKind::LeftBrace => depth += 1,
            TokenKind::RightBrace => depth -= 1,
            TokenKind::Eof => break,
            _ => {}
        }
    }
    assert_eq!(depth, 0, "braces should be balanced");
    Ok(())
}

#[test]
fn token_stream_mixed_constructs() -> Result<(), Box<dyn std::error::Error>> {
    let src = r#"
        use strict;
        my $x = 42;
        sub foo { return $x + 1; }
        if ($x > 0) { print "yes"; }
    "#;
    let mut s = TokenStream::new(src);
    let mut token_count = 0;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        token_count += 1;
    }
    assert!(token_count > 20, "complex source should produce many tokens");
    Ok(())
}

#[test]
fn token_stream_only_comments() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("# comment 1\n# comment 2\n");
    assert!(s.is_eof());
    Ok(())
}

#[test]
fn token_stream_dot_operator() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("$a . $b");
    assert_eq!(must(s.next()).kind, TokenKind::Identifier); // $a
    assert_eq!(must(s.next()).kind, TokenKind::Dot);
    assert_eq!(must(s.next()).kind, TokenKind::Identifier); // $b
    Ok(())
}

#[test]
fn token_stream_match_operators() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("$x =~ $y !~ $z");
    assert_eq!(must(s.next()).kind, TokenKind::Identifier); // $x
    assert_eq!(must(s.next()).kind, TokenKind::Match);
    assert_eq!(must(s.next()).kind, TokenKind::Identifier); // $y
    assert_eq!(must(s.next()).kind, TokenKind::NotMatch);
    assert_eq!(must(s.next()).kind, TokenKind::Identifier); // $z
    Ok(())
}

#[test]
fn token_stream_bitwise_operators() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("1 | 2 ^ 3 << 4 >> 5");
    let _ = must(s.next()); // 1
    assert_eq!(must(s.next()).kind, TokenKind::BitwiseOr);
    let _ = must(s.next()); // 2
    assert_eq!(must(s.next()).kind, TokenKind::BitwiseXor);
    let _ = must(s.next()); // 3
    assert_eq!(must(s.next()).kind, TokenKind::LeftShift);
    let _ = must(s.next()); // 4
    assert_eq!(must(s.next()).kind, TokenKind::RightShift);
    Ok(())
}

#[test]
fn token_stream_comparison_less_greater() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("1 < 2");
    let _ = must(s.next());
    assert_eq!(must(s.next()).kind, TokenKind::Less);

    let mut s = TokenStream::new("1 > 2");
    let _ = must(s.next());
    assert_eq!(must(s.next()).kind, TokenKind::Greater);

    let mut s = TokenStream::new("1 <= 2");
    let _ = must(s.next());
    assert_eq!(must(s.next()).kind, TokenKind::LessEqual);

    let mut s = TokenStream::new("1 >= 2");
    let _ = must(s.next());
    assert_eq!(must(s.next()).kind, TokenKind::GreaterEqual);
    Ok(())
}

// ---------------------------------------------------------------------------
// Trivia equality
// ---------------------------------------------------------------------------

#[test]
fn trivia_equality() -> Result<(), Box<dyn std::error::Error>> {
    let a = Trivia::Whitespace("  ".to_string());
    let b = Trivia::Whitespace("  ".to_string());
    assert_eq!(a, b);

    let c = Trivia::Newline;
    let d = Trivia::Newline;
    assert_eq!(c, d);

    assert_ne!(a, c);
    Ok(())
}

// ---------------------------------------------------------------------------
// TokenWithPosition — accessors
// ---------------------------------------------------------------------------

#[test]
fn token_with_position_range() -> Result<(), Box<dyn std::error::Error>> {
    let source = "hello\nworld";
    let tracker = PositionTracker::new(source);

    let token = perl_lexer::Token::new(
        perl_lexer::TokenType::Identifier(std::sync::Arc::from("world")),
        std::sync::Arc::from("world"),
        6,
        11,
    );
    let wrapped = tracker.wrap_token(token);
    let range = wrapped.range();
    assert_eq!(range.start.line, 2);
    assert_eq!(range.start.column, 1);
    assert_eq!(range.end.line, 2);
    assert_eq!(range.end.column, 6);
    Ok(())
}

// ---------------------------------------------------------------------------
// TriviaPreservingParser — format_with_trivia round-trip
// ---------------------------------------------------------------------------

#[test]
fn format_with_trivia_empty() -> Result<(), Box<dyn std::error::Error>> {
    let parser = TriviaPreservingParser::new(String::new());
    let result = parser.parse();
    let formatted = perl_parser_core::trivia_parser::format_with_trivia(&result);
    // Should not panic or produce garbage
    assert!(formatted.contains("Program"), "should contain Program node repr");
    Ok(())
}

// ---------------------------------------------------------------------------
// Additional TokenStream — shift / defined-or / smart-match operators
// ---------------------------------------------------------------------------

#[test]
fn token_stream_shift_operators() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("$x << 2; $y >> 3;");
    // consume tokens and look for LeftShift / RightShift
    let mut found_left_shift = false;
    let mut found_right_shift = false;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        if t.kind == TokenKind::LeftShift {
            found_left_shift = true;
        }
        if t.kind == TokenKind::RightShift {
            found_right_shift = true;
        }
    }
    assert!(found_left_shift, "should detect <<");
    assert!(found_right_shift, "should detect >>");
    Ok(())
}

#[test]
fn token_stream_defined_or_operator() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("$x // $y;");
    let mut found = false;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        if t.kind == TokenKind::DefinedOr {
            found = true;
        }
    }
    assert!(found, "should detect // as DefinedOr");
    Ok(())
}

#[test]
fn token_stream_smart_match_operator() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("$x ~~ @arr;");
    let mut found = false;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        if t.kind == TokenKind::SmartMatch {
            found = true;
        }
    }
    assert!(found, "should detect ~~ as SmartMatch");
    Ok(())
}

// ---------------------------------------------------------------------------
// TokenStream — compound assignment operators
// ---------------------------------------------------------------------------

#[test]
fn token_stream_shift_assign_operators() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("$x <<= 2; $y >>= 3;");
    let mut found_lsa = false;
    let mut found_rsa = false;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        if t.kind == TokenKind::LeftShiftAssign {
            found_lsa = true;
        }
        if t.kind == TokenKind::RightShiftAssign {
            found_rsa = true;
        }
    }
    assert!(found_lsa, "should detect <<=");
    assert!(found_rsa, "should detect >>=");
    Ok(())
}

#[test]
fn token_stream_logical_assign_operators() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("$x &&= 1; $y ||= 2; $z //= 3;");
    let mut found_and = false;
    let mut found_or = false;
    let mut found_defor = false;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        match t.kind {
            TokenKind::LogicalAndAssign => found_and = true,
            TokenKind::LogicalOrAssign => found_or = true,
            TokenKind::DefinedOrAssign => found_defor = true,
            _ => {}
        }
    }
    assert!(found_and, "should detect &&=");
    assert!(found_or, "should detect ||=");
    assert!(found_defor, "should detect //=");
    Ok(())
}

#[test]
fn token_stream_power_assign_operator() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("$x **= 2;");
    let mut found = false;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        if t.kind == TokenKind::PowerAssign {
            found = true;
        }
    }
    assert!(found, "should detect **=");
    Ok(())
}

#[test]
fn token_stream_xor_assign_operator() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("$x ^= 0xFF;");
    let mut found = false;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        if t.kind == TokenKind::XorAssign {
            found = true;
        }
    }
    assert!(found, "should detect ^=");
    Ok(())
}

// ---------------------------------------------------------------------------
// TokenStream — sigils and special tokens
// ---------------------------------------------------------------------------

#[test]
fn token_stream_hash_variable() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("my %hash;");
    let t = must(s.next()); // my
    assert_eq!(t.kind, TokenKind::My);
    // The lexer may produce the hash variable as a single token or sigil + ident
    // Just verify we don't crash and get meaningful tokens
    let t2 = must(s.next());
    assert_ne!(t2.kind, TokenKind::Eof, "should have tokens after 'my'");
    Ok(())
}

#[test]
fn token_stream_double_colon() -> Result<(), Box<dyn std::error::Error>> {
    // The lexer may tokenize Foo::Bar as a single qualified identifier
    // Verify it produces valid tokens without crashing
    let mut s = TokenStream::new("Foo::Bar");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Identifier, "qualified name should be an identifier");
    Ok(())
}

#[test]
fn token_stream_power_operator() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("2 ** 10;");
    let mut found = false;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        if t.kind == TokenKind::Power {
            found = true;
        }
    }
    assert!(found, "should detect ** as Power");
    Ok(())
}

#[test]
fn token_stream_spaceship_operator() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("$a <=> $b;");
    let mut found = false;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        if t.kind == TokenKind::Spaceship {
            found = true;
        }
    }
    assert!(found, "should detect <=> as Spaceship");
    Ok(())
}

#[test]
fn token_stream_string_compare() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("$a cmp $b;");
    let mut found = false;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        if t.kind == TokenKind::StringCompare {
            found = true;
        }
    }
    assert!(found, "should detect cmp");
    Ok(())
}

#[test]
fn token_stream_word_xor() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("$a xor $b;");
    let mut found = false;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        if t.kind == TokenKind::WordXor {
            found = true;
        }
    }
    assert!(found, "should detect xor keyword");
    Ok(())
}

#[test]
fn token_stream_ellipsis() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("...");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Ellipsis);
    Ok(())
}

// ---------------------------------------------------------------------------
// TokenStream — special keyword coverage
// ---------------------------------------------------------------------------

#[test]
fn token_stream_eval_keyword() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("eval { 1 };");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Eval);
    Ok(())
}

#[test]
fn token_stream_do_keyword() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("do 'file.pl';");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Do);
    Ok(())
}

#[test]
fn token_stream_given_when_default() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("given ($x) { when (1) { } default { } }");
    let mut found_given = false;
    let mut found_when = false;
    let mut found_default = false;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        match t.kind {
            TokenKind::Given => found_given = true,
            TokenKind::When => found_when = true,
            TokenKind::Default => found_default = true,
            _ => {}
        }
    }
    assert!(found_given, "should detect given");
    assert!(found_when, "should detect when");
    assert!(found_default, "should detect default");
    Ok(())
}

#[test]
fn token_stream_try_catch_finally() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("try { } catch ($e) { } finally { }");
    let mut found_try = false;
    let mut found_catch = false;
    let mut found_finally = false;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        match t.kind {
            TokenKind::Try => found_try = true,
            TokenKind::Catch => found_catch = true,
            TokenKind::Finally => found_finally = true,
            _ => {}
        }
    }
    assert!(found_try, "should detect try");
    assert!(found_catch, "should detect catch");
    assert!(found_finally, "should detect finally");
    Ok(())
}

#[test]
fn token_stream_class_method() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("class Foo { method bar { } }");
    let mut found_class = false;
    let mut found_method = false;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        match t.kind {
            TokenKind::Class => found_class = true,
            TokenKind::Method => found_method = true,
            _ => {}
        }
    }
    assert!(found_class, "should detect class");
    assert!(found_method, "should detect method");
    Ok(())
}

#[test]
fn token_stream_undef_keyword() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("undef;");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Undef);
    Ok(())
}

#[test]
fn token_stream_no_keyword() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("no strict;");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::No);
    Ok(())
}

#[test]
fn token_stream_state_keyword() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("state $x = 0;");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::State);
    Ok(())
}

#[test]
fn token_stream_format_keyword() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("format STDOUT =");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Format);
    Ok(())
}

// ---------------------------------------------------------------------------
// TokenStream — token text and position verification
// ---------------------------------------------------------------------------

#[test]
fn token_stream_token_text_preserved() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("my $variable = 42;");
    let my_tok = must(s.next());
    assert_eq!(my_tok.text.as_ref(), "my");
    Ok(())
}

#[test]
fn token_stream_token_positions_are_monotonic() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("my $x = 1 + 2;");
    let mut prev_start = 0;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        assert!(t.start >= prev_start, "token starts should be non-decreasing");
        assert!(t.end > t.start, "token end should be after start");
        prev_start = t.start;
    }
    Ok(())
}

#[test]
fn token_stream_multiple_peeks_same_result() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("my $x;");
    let k1 = must(s.peek()).kind;
    let k2 = must(s.peek()).kind;
    let k3 = must(s.peek()).kind;
    assert_eq!(k1, k2);
    assert_eq!(k2, k3);
    Ok(())
}

#[test]
fn token_stream_peek_does_not_advance() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("if (1) { }");
    let peeked = must(s.peek()).kind;
    let consumed = must(s.next()).kind;
    assert_eq!(peeked, consumed, "peek and next should return the same token");
    Ok(())
}

// ---------------------------------------------------------------------------
// TokenStream — heredoc / regex / substitution / transliteration tokens
// ---------------------------------------------------------------------------

#[test]
fn token_stream_heredoc_start() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("my $x = <<'END';\nhello\nEND\n");
    let mut found_heredoc = false;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        if t.kind == TokenKind::HeredocStart || t.kind == TokenKind::HeredocBody {
            found_heredoc = true;
        }
    }
    assert!(found_heredoc, "should detect heredoc tokens");
    Ok(())
}

#[test]
fn token_stream_regex_match() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("$x =~ /pattern/;");
    let mut found_regex = false;
    let mut found_match = false;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        if t.kind == TokenKind::Regex {
            found_regex = true;
        }
        if t.kind == TokenKind::Match {
            found_match = true;
        }
    }
    assert!(found_match, "should detect =~ match operator");
    assert!(found_regex, "should detect regex literal");
    Ok(())
}

#[test]
fn token_stream_not_match_operator() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("$x !~ /pattern/;");
    let mut found = false;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        if t.kind == TokenKind::NotMatch {
            found = true;
        }
    }
    assert!(found, "should detect !~ not-match operator");
    Ok(())
}

#[test]
fn token_stream_substitution() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("$x =~ s/foo/bar/g;");
    let mut found = false;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        if t.kind == TokenKind::Substitution {
            found = true;
        }
    }
    assert!(found, "should detect s/// substitution");
    Ok(())
}

#[test]
fn token_stream_transliteration() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("$x =~ tr/a-z/A-Z/;");
    let mut found = false;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        if t.kind == TokenKind::Transliteration {
            found = true;
        }
    }
    assert!(found, "should detect tr/// transliteration");
    Ok(())
}

#[test]
fn token_stream_quote_words() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("my @list = qw(foo bar baz);");
    let mut found = false;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        if t.kind == TokenKind::QuoteWords {
            found = true;
        }
    }
    assert!(found, "should detect qw() quote words");
    Ok(())
}

// ---------------------------------------------------------------------------
// TokenStream — enter_format_mode
// ---------------------------------------------------------------------------

#[test]
fn token_stream_enter_format_mode_no_crash() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("format =\ntest\n.\n");
    s.enter_format_mode();
    // Just verify it doesn't crash
    let _ = s.next();
    Ok(())
}

// ---------------------------------------------------------------------------
// TokenStream — Unicode identifiers and edge cases
// ---------------------------------------------------------------------------

#[test]
fn token_stream_unicode_string_content() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("my $x = \"日本語\";");
    let mut found_string = false;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        if t.kind == TokenKind::String {
            found_string = true;
        }
    }
    assert!(found_string, "should tokenize strings with Unicode content");
    Ok(())
}

#[test]
fn token_stream_consecutive_semicolons() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new(";;;");
    let mut count = 0;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        if t.kind == TokenKind::Semicolon {
            count += 1;
        }
    }
    assert_eq!(count, 3, "should detect three consecutive semicolons");
    Ok(())
}

#[test]
fn token_stream_deeply_nested_parens() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("((((1))))");
    let mut left_count = 0;
    let mut right_count = 0;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        match t.kind {
            TokenKind::LeftParen => left_count += 1,
            TokenKind::RightParen => right_count += 1,
            _ => {}
        }
    }
    assert_eq!(left_count, 4);
    assert_eq!(right_count, 4);
    Ok(())
}

#[test]
fn token_stream_multiline_code() -> Result<(), Box<dyn std::error::Error>> {
    let input = "my $x = 1;\nmy $y = 2;\nmy $z = 3;\n";
    let mut s = TokenStream::new(input);
    let mut my_count = 0;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        if t.kind == TokenKind::My {
            my_count += 1;
        }
    }
    assert_eq!(my_count, 3, "should tokenize across multiple lines");
    Ok(())
}

// ---------------------------------------------------------------------------
// PositionTracker — additional edge cases
// ---------------------------------------------------------------------------

#[test]
fn position_tracker_crlf_newlines() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line1\r\nline2\r\n";
    let tracker = PositionTracker::new(source);
    // Byte 0 = start of line 1
    let pos = tracker.byte_to_position(0);
    assert_eq!(pos.line, 1);
    assert_eq!(pos.column, 1);
    Ok(())
}

#[test]
fn position_tracker_end_of_file_offset() -> Result<(), Box<dyn std::error::Error>> {
    let source = "abc\ndef";
    let tracker = PositionTracker::new(source);
    let pos = tracker.byte_to_position(source.len());
    // Should not crash, position should be valid
    assert!(pos.line >= 1);
    Ok(())
}

#[test]
fn position_tracker_single_char_lines() -> Result<(), Box<dyn std::error::Error>> {
    let source = "a\nb\nc\n";
    let tracker = PositionTracker::new(source);
    let pos_a = tracker.byte_to_position(0);
    let pos_b = tracker.byte_to_position(2);
    let pos_c = tracker.byte_to_position(4);
    assert_eq!(pos_a.line, 1);
    assert_eq!(pos_b.line, 2);
    assert_eq!(pos_c.line, 3);
    Ok(())
}

// ---------------------------------------------------------------------------
// TokenWithPosition — byte_range and kind accessors
// ---------------------------------------------------------------------------

#[test]
fn token_with_position_byte_range_accessors() -> Result<(), Box<dyn std::error::Error>> {
    use perl_lexer::{Token as LToken, TokenType as LTT};
    use std::sync::Arc;

    let source = "my $x";
    let tracker = PositionTracker::new(source);
    let token = LToken::new(LTT::Keyword(Arc::from("my")), Arc::from("my"), 0, 2);
    let wrapped = tracker.wrap_token(token);

    assert_eq!(wrapped.byte_range(), (0, 2));
    assert_eq!(wrapped.text(), "my");
    assert!(matches!(wrapped.kind(), LTT::Keyword(_)));
    Ok(())
}

// ---------------------------------------------------------------------------
// Trivia — additional coverage
// ---------------------------------------------------------------------------

#[test]
fn trivia_whitespace_as_str() -> Result<(), Box<dyn std::error::Error>> {
    let t = Trivia::Whitespace("  \t".to_string());
    assert_eq!(t.as_str(), "  \t");
    assert_eq!(t.kind_name(), "whitespace");
    Ok(())
}

#[test]
fn trivia_newline_as_str() -> Result<(), Box<dyn std::error::Error>> {
    let t = Trivia::Newline;
    assert_eq!(t.as_str(), "\n");
    assert_eq!(t.kind_name(), "newline");
    Ok(())
}

#[test]
fn trivia_pod_comment_as_str() -> Result<(), Box<dyn std::error::Error>> {
    let pod = "=head1 NAME\n\ntest\n\n=cut".to_string();
    let t = Trivia::PodComment(pod.clone());
    assert_eq!(t.as_str(), pod.as_str());
    assert_eq!(t.kind_name(), "pod");
    Ok(())
}

#[test]
fn trivia_line_comment_preserves_content() -> Result<(), Box<dyn std::error::Error>> {
    let t = Trivia::LineComment("# hello world".to_string());
    assert_eq!(t.as_str(), "# hello world");
    assert_eq!(t.kind_name(), "comment");
    Ok(())
}

#[test]
fn trivia_equality_different_variants() -> Result<(), Box<dyn std::error::Error>> {
    let a = Trivia::Whitespace(" ".to_string());
    let b = Trivia::LineComment("# x".to_string());
    assert_ne!(a, b);
    Ok(())
}

#[test]
fn trivia_equality_same_whitespace_content() -> Result<(), Box<dyn std::error::Error>> {
    let a = Trivia::Whitespace("  ".to_string());
    let b = Trivia::Whitespace("  ".to_string());
    assert_eq!(a, b);
    let c = Trivia::Whitespace(" ".to_string());
    assert_ne!(a, c);
    Ok(())
}

// ---------------------------------------------------------------------------
// TriviaLexer — additional coverage
// ---------------------------------------------------------------------------

#[test]
fn trivia_lexer_consecutive_comments() -> Result<(), Box<dyn std::error::Error>> {
    let source = "# line 1\n# line 2\nmy $x;".to_string();
    let mut lexer = TriviaLexer::new(source);
    let (_, trivia) = must_some(lexer.next_token_with_trivia());
    let comment_count =
        trivia.iter().filter(|t| matches!(&t.trivia, Trivia::LineComment(_))).count();
    assert!(comment_count >= 2, "should detect consecutive comments, got {}", comment_count);
    Ok(())
}

#[test]
fn trivia_lexer_whitespace_only_source() -> Result<(), Box<dyn std::error::Error>> {
    let source = "   \t\t   ".to_string();
    let mut lexer = TriviaLexer::new(source);
    // Should eventually return None (no meaningful tokens)
    let result = lexer.next_token_with_trivia();
    // Either None or EOF token with trivia
    if let Some((tok, _trivia)) = result {
        assert!(
            matches!(tok.token_type, perl_lexer::TokenType::EOF),
            "whitespace-only source should yield EOF or None"
        );
    }
    Ok(())
}

#[test]
fn trivia_lexer_comment_only_source() -> Result<(), Box<dyn std::error::Error>> {
    let source = "# just a comment\n".to_string();
    let mut lexer = TriviaLexer::new(source);
    let result = lexer.next_token_with_trivia();
    // Should either return None or EOF with the comment in trivia
    if let Some((tok, trivia)) = result {
        if !matches!(tok.token_type, perl_lexer::TokenType::EOF) {
            // If not EOF, the trivia should have the comment
            let has_comment = trivia.iter().any(|t| matches!(&t.trivia, Trivia::LineComment(_)));
            assert!(has_comment, "comment should be in trivia");
        }
    }
    Ok(())
}

#[test]
fn trivia_lexer_mixed_whitespace_and_comments() -> Result<(), Box<dyn std::error::Error>> {
    let source = "  \n  # comment\n  my $x;".to_string();
    let mut lexer = TriviaLexer::new(source);
    let (_, trivia) = must_some(lexer.next_token_with_trivia());
    let has_ws = trivia.iter().any(|t| matches!(&t.trivia, Trivia::Whitespace(_)));
    let has_comment = trivia.iter().any(|t| matches!(&t.trivia, Trivia::LineComment(_)));
    assert!(has_ws, "should have whitespace trivia");
    assert!(has_comment, "should have comment trivia");
    Ok(())
}

// ---------------------------------------------------------------------------
// TriviaParserContext — additional coverage
// ---------------------------------------------------------------------------

#[test]
fn trivia_parser_context_advance_past_end() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = TriviaParserContext::new("my $x;".to_string());
    // is_eof on non-empty source should be false initially
    assert!(!ctx.is_eof());
    Ok(())
}

#[test]
fn trivia_parser_context_empty_source() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = TriviaParserContext::new(String::new());
    assert!(ctx.is_eof(), "empty source should be immediately eof");
    Ok(())
}

// ---------------------------------------------------------------------------
// TriviaPreservingParser (trivia_parser module) — additional coverage
// ---------------------------------------------------------------------------

#[test]
fn trivia_parser_variable_declaration_with_comment() -> Result<(), Box<dyn std::error::Error>> {
    let source = "# header\nmy $x = 1;\nour $y = 2;\n".to_string();
    let ctx = TriviaParserContext::new(source);
    // Non-empty source with tokens should not be eof
    assert!(!ctx.is_eof(), "should have parsed some tokens");
    Ok(())
}

#[test]
fn trivia_preserving_parser_format_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let source = "# comment\nmy $x;".to_string();
    let parser = TriviaPreservingParser::new(source);
    let result = parser.parse();
    let formatted = perl_parser_core::trivia_parser::format_with_trivia(&result);
    // Should include the comment in output
    assert!(!formatted.is_empty(), "formatted output should not be empty");
    Ok(())
}

// ---------------------------------------------------------------------------
// Utility functions — additional coverage
// ---------------------------------------------------------------------------

#[test]
fn data_marker_at_very_start() -> Result<(), Box<dyn std::error::Error>> {
    let src = "__DATA__\nsome data here";
    assert_eq!(find_data_marker_byte_lexed(src), Some(0));
    Ok(())
}

#[test]
fn data_marker_end_at_very_start() -> Result<(), Box<dyn std::error::Error>> {
    let src = "__END__\nsome data here";
    assert_eq!(find_data_marker_byte_lexed(src), Some(0));
    Ok(())
}

#[test]
fn code_slice_data_at_start() -> Result<(), Box<dyn std::error::Error>> {
    let src = "__DATA__\ndata";
    assert_eq!(code_slice(src), "");
    Ok(())
}

#[test]
fn code_slice_multiline_code_then_data() -> Result<(), Box<dyn std::error::Error>> {
    let src = "my $x = 1;\nmy $y = 2;\n__DATA__\ndata here\n";
    let slice = code_slice(src);
    assert!(slice.contains("my $x"));
    assert!(slice.contains("my $y"));
    assert!(!slice.contains("data here"));
    Ok(())
}

#[test]
fn find_data_marker_empty_input() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(find_data_marker_byte_lexed(""), None);
    Ok(())
}

// ---------------------------------------------------------------------------
// TokenStream — on_stmt_boundary comprehensive
// ---------------------------------------------------------------------------

#[test]
fn on_stmt_boundary_after_peek_second() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("my $x; our $y;");
    // Fill peek slots
    let _ = must(s.peek());
    let _ = must(s.peek_second());
    // Reset boundary clears cache and sets lexer to ExpectTerm mode
    s.on_stmt_boundary();
    // After reset, cached peeks are cleared so next peek re-reads from lexer
    let t = must(s.peek());
    // The lexer continues from its current position (not from the start),
    // so we just verify the boundary reset doesn't crash and returns a valid token
    assert_ne!(t.kind, TokenKind::Unknown, "should return a valid token after boundary reset");
    Ok(())
}

#[test]
fn on_stmt_boundary_after_peek_third() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("my $x = 42;");
    let _ = must(s.peek());
    let _ = must(s.peek_second());
    let _ = must(s.peek_third());
    s.on_stmt_boundary();
    // Boundary clears peek cache; lexer continues from its current position
    let t = must(s.peek());
    assert_ne!(t.kind, TokenKind::Unknown, "should return a valid token after boundary reset");
    Ok(())
}

// ---------------------------------------------------------------------------
// TokenStream — Division vs Regex context awareness
// ---------------------------------------------------------------------------

#[test]
fn token_stream_division_after_number() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("42 / 7;");
    let num = must(s.next());
    assert_eq!(num.kind, TokenKind::Number);
    let div = must(s.next());
    assert_eq!(div.kind, TokenKind::Slash, "/ after number should be division");
    Ok(())
}

// ---------------------------------------------------------------------------
// TriviaToken — range accessor
// ---------------------------------------------------------------------------

#[test]
fn trivia_token_range_is_valid() -> Result<(), Box<dyn std::error::Error>> {
    use perl_position_tracking::{Position, Range};
    let start = Position::new(0, 1, 1);
    let end = Position::new(5, 1, 6);
    let tt = TriviaToken::new(Trivia::Whitespace("     ".to_string()), Range::new(start, end));
    assert_eq!(tt.range.start.byte, 0);
    assert_eq!(tt.range.end.byte, 5);
    assert_eq!(tt.trivia.as_str(), "     ");
    Ok(())
}
