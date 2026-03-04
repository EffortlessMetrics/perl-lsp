//! Comprehensive unit tests for the perl-tokenizer crate.
//!
//! Covers: TokenStream, TokenWithPosition, PositionTracker,
//! Trivia/TriviaLexer/TriviaToken, TriviaPreservingParser,
//! and utility functions (find_data_marker_byte_lexed, code_slice).

use perl_tdd_support::{must, must_some};
use perl_tokenizer::TokenKind;
use perl_tokenizer::token_stream::TokenStream;
use perl_tokenizer::token_wrapper::PositionTracker;
use perl_tokenizer::trivia::{Trivia, TriviaLexer, TriviaToken};
use perl_tokenizer::trivia_parser::{TriviaParserContext, TriviaPreservingParser};
use perl_tokenizer::util::{code_slice, find_data_marker_byte_lexed};

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
    assert!(matches!(&result.node.kind, perl_ast::v2::NodeKind::Program { .. }));
    Ok(())
}

#[test]
fn trivia_preserving_parser_empty_source() -> Result<(), Box<dyn std::error::Error>> {
    let parser = TriviaPreservingParser::new(String::new());
    let result = parser.parse();
    assert!(matches!(&result.node.kind, perl_ast::v2::NodeKind::Program { .. }));
    Ok(())
}

#[test]
fn trivia_preserving_parser_comment_only() -> Result<(), Box<dyn std::error::Error>> {
    let parser = TriviaPreservingParser::new("# just a comment\n".to_string());
    let result = parser.parse();
    // Should still produce a valid program
    assert!(matches!(&result.node.kind, perl_ast::v2::NodeKind::Program { .. }));
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
    if let perl_ast::v2::NodeKind::Program { statements } = &result.node.kind {
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
    let formatted = perl_tokenizer::trivia_parser::format_with_trivia(&result);
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
    let formatted = perl_tokenizer::trivia_parser::format_with_trivia(&result);
    // Should not panic or produce garbage
    assert!(formatted.contains("Program"), "should contain Program node repr");
    Ok(())
}
