//! Bridge coverage tests for perl-tokenizer.
//!
//! Tests the lexer-to-parser bridge functionality that is not covered
//! by existing test files. Focuses on:
//! - Token stream creation from various source inputs
//! - Trivia handling (whitespace, comments, kind_name, as_str)
//! - Token kind mapping for less-common lexer token types
//! - Edge cases: empty source, Unicode, very long lines, format_with_trivia

use perl_lexer::tokenizer::token_wrapper::PositionTracker;
use perl_lexer::tokenizer::util::{code_slice, find_data_marker_byte_lexed};
use perl_parser_core::tokens::token_stream::TokenStream;
use perl_parser_core::trivia::{Trivia, TriviaLexer, TriviaToken};
use perl_parser_core::trivia_parser::{
    TriviaParserContext, TriviaPreservingParser, format_with_trivia,
};
use perl_tdd_support::must;
use perl_token::TokenKind;

// ===================================================================
// Helper: collect all non-EOF token kinds from a stream
// ===================================================================

fn collect_kinds(src: &str) -> Vec<TokenKind> {
    let mut s = TokenStream::new(src);
    let mut kinds = Vec::new();
    while let Ok(t) = s.next() {
        if t.kind == TokenKind::Eof {
            break;
        }
        kinds.push(t.kind);
    }
    kinds
}

fn collect_texts(src: &str) -> Vec<String> {
    let mut s = TokenStream::new(src);
    let mut texts = Vec::new();
    while let Ok(t) = s.next() {
        if t.kind == TokenKind::Eof {
            break;
        }
        texts.push(t.text.to_string());
    }
    texts
}

// ===================================================================
// 1. Trivia enum methods
// ===================================================================

#[test]
fn trivia_whitespace_as_str_returns_content() -> Result<(), Box<dyn std::error::Error>> {
    let t = Trivia::Whitespace("  \t".to_string());
    assert_eq!(t.as_str(), "  \t");
    Ok(())
}

#[test]
fn trivia_line_comment_as_str_returns_content() -> Result<(), Box<dyn std::error::Error>> {
    let t = Trivia::LineComment("# hello".to_string());
    assert_eq!(t.as_str(), "# hello");
    Ok(())
}

#[test]
fn trivia_pod_comment_as_str_returns_content() -> Result<(), Box<dyn std::error::Error>> {
    let t = Trivia::PodComment("=head1 NAME\n\n=cut\n".to_string());
    assert_eq!(t.as_str(), "=head1 NAME\n\n=cut\n");
    Ok(())
}

#[test]
fn trivia_newline_as_str_returns_newline() -> Result<(), Box<dyn std::error::Error>> {
    let t = Trivia::Newline;
    assert_eq!(t.as_str(), "\n");
    Ok(())
}

#[test]
fn trivia_kind_name_whitespace() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        Trivia::Whitespace(" ".to_string()).kind_name(),
        "whitespace"
    );
    Ok(())
}

#[test]
fn trivia_kind_name_comment() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        Trivia::LineComment("# x".to_string()).kind_name(),
        "comment"
    );
    Ok(())
}

#[test]
fn trivia_kind_name_pod() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(Trivia::PodComment("=pod".to_string()).kind_name(), "pod");
    Ok(())
}

#[test]
fn trivia_kind_name_newline() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(Trivia::Newline.kind_name(), "newline");
    Ok(())
}

#[test]
fn trivia_equality_whitespace() -> Result<(), Box<dyn std::error::Error>> {
    let a = Trivia::Whitespace("  ".to_string());
    let b = Trivia::Whitespace("  ".to_string());
    assert_eq!(a, b);
    Ok(())
}

#[test]
fn trivia_inequality_different_variants() -> Result<(), Box<dyn std::error::Error>> {
    let ws = Trivia::Whitespace(" ".to_string());
    let nl = Trivia::Newline;
    assert_ne!(ws, nl);
    Ok(())
}

// ===================================================================
// 2. TriviaToken construction
// ===================================================================

#[test]
fn trivia_token_new_stores_trivia_and_range() -> Result<(), Box<dyn std::error::Error>> {
    let range = perl_position_tracking::Range::new(
        perl_position_tracking::Position::new(0, 1, 1),
        perl_position_tracking::Position::new(3, 1, 4),
    );
    let tt = TriviaToken::new(Trivia::Whitespace("   ".to_string()), range);
    assert!(matches!(&tt.trivia, Trivia::Whitespace(s) if s == "   "));
    assert_eq!(tt.range.start.byte, 0);
    assert_eq!(tt.range.end.byte, 3);
    Ok(())
}

// ===================================================================
// 3. TokenStream — token kind mapping for less-common token types
// ===================================================================

#[test]
fn token_stream_substitution_s_operator() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("s/foo/bar/g");
    assert!(
        kinds.contains(&TokenKind::Substitution),
        "Expected Substitution token in {:?}",
        kinds
    );
    Ok(())
}

#[test]
fn token_stream_transliteration_tr_operator() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("tr/a-z/A-Z/");
    assert!(
        kinds.contains(&TokenKind::Transliteration),
        "Expected Transliteration token in {:?}",
        kinds
    );
    Ok(())
}

#[test]
fn token_stream_transliteration_y_operator() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("y/a-z/A-Z/");
    assert!(
        kinds.contains(&TokenKind::Transliteration),
        "Expected Transliteration token from y/// in {:?}",
        kinds
    );
    Ok(())
}

#[test]
fn token_stream_qw_quote_words() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("qw(one two three)");
    assert!(
        kinds.contains(&TokenKind::QuoteWords),
        "Expected QuoteWords token in {:?}",
        kinds
    );
    Ok(())
}

#[test]
fn token_stream_qq_quote_double() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("qq{hello world}");
    // qq{} may lex as QuoteDouble or String depending on lexer
    let has_string_like = kinds
        .iter()
        .any(|k| matches!(k, TokenKind::QuoteDouble | TokenKind::String));
    assert!(
        has_string_like,
        "Expected QuoteDouble or String for qq in {:?}",
        kinds
    );
    Ok(())
}

#[test]
fn token_stream_q_quote_single() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("q{hello world}");
    let has_string_like = kinds
        .iter()
        .any(|k| matches!(k, TokenKind::QuoteSingle | TokenKind::String));
    assert!(
        has_string_like,
        "Expected QuoteSingle or String for q{{}} in {kinds:?}"
    );
    Ok(())
}

#[test]
fn token_stream_qx_quote_command() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("qx(ls -la)");
    let has_cmd = kinds
        .iter()
        .any(|k| matches!(k, TokenKind::QuoteCommand | TokenKind::String));
    assert!(
        has_cmd,
        "Expected QuoteCommand or String for qx() in {:?}",
        kinds
    );
    Ok(())
}

#[test]
fn token_stream_regex_match_operator() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("m/pattern/i");
    assert!(
        kinds.contains(&TokenKind::Regex),
        "Expected Regex token for m// in {:?}",
        kinds
    );
    Ok(())
}

#[test]
fn token_stream_qr_regex() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("qr/pattern/ix");
    assert!(
        kinds.contains(&TokenKind::Regex),
        "Expected Regex token for qr// in {:?}",
        kinds
    );
    Ok(())
}

#[test]
fn token_stream_data_marker() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("my $x = 1;\n__DATA__\nsome data here");
    assert!(
        kinds.contains(&TokenKind::DataMarker),
        "Expected DataMarker token in {:?}",
        kinds
    );
    Ok(())
}

#[test]
fn token_stream_end_marker() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("my $x = 1;\n__END__\nsome data here");
    assert!(
        kinds.contains(&TokenKind::DataMarker),
        "Expected DataMarker token for __END__ in {:?}",
        kinds
    );
    Ok(())
}

#[test]
fn token_stream_match_and_not_match_operators() -> Result<(), Box<dyn std::error::Error>> {
    // =~ operator
    let mut s = TokenStream::new("$x =~ /foo/");
    let _ = must(s.next()); // $x
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Match, "Expected Match for =~");

    // !~ operator
    let mut s = TokenStream::new("$x !~ /foo/");
    let _ = must(s.next()); // $x
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::NotMatch, "Expected NotMatch for !~");
    Ok(())
}

#[test]
fn token_stream_dot_operator() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("$a . $b");
    let _ = must(s.next()); // $a
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Dot, "Expected Dot for string concat");
    Ok(())
}

#[test]
fn token_stream_power_operator() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("2 ** 8");
    let _ = must(s.next()); // 2
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Power, "Expected Power for **");
    Ok(())
}

#[test]
fn token_stream_bitwise_operators() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("$a << 2");
    let _ = must(s.next()); // $a
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::LeftShift, "Expected LeftShift for <<");

    let mut s = TokenStream::new("$a >> 2");
    let _ = must(s.next()); // $a
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::RightShift, "Expected RightShift for >>");
    Ok(())
}

#[test]
fn token_stream_not_operator() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("!$x");
    assert!(
        kinds.contains(&TokenKind::Not),
        "Expected Not operator in {:?}",
        kinds
    );
    Ok(())
}

#[test]
fn token_stream_format_keyword() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("format STDOUT =");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Format, "Expected Format keyword");
    Ok(())
}

#[test]
fn token_stream_goto_keyword() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("goto LABEL");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Goto, "Expected Goto keyword");
    Ok(())
}

// ===================================================================
// 4. TokenStream — edge cases
// ===================================================================

#[test]
fn token_stream_very_long_line() -> Result<(), Box<dyn std::error::Error>> {
    // Generate a very long assignment: my $x = "aaa...aaa";
    let long_str: String = "a".repeat(10_000);
    let src = format!("my $x = \"{long_str}\";");
    let mut s = TokenStream::new(&src);
    assert_eq!(must(s.peek()).kind, TokenKind::My);
    // Consume all tokens without error
    let mut count = 0;
    while let Ok(t) = s.next() {
        if t.kind == TokenKind::Eof {
            break;
        }
        count += 1;
    }
    assert!(
        count >= 3,
        "Expected at least 3 tokens from long line, got {count}"
    );
    Ok(())
}

#[test]
fn token_stream_unicode_identifier() -> Result<(), Box<dyn std::error::Error>> {
    // Perl supports Unicode identifiers with utf8 pragma
    let kinds = collect_kinds("my $\u{00E9}l\u{00E8}ve = 1;");
    assert!(
        kinds.contains(&TokenKind::My),
        "Should parse 'my' before Unicode identifier"
    );
    Ok(())
}

#[test]
fn token_stream_unicode_string_content() -> Result<(), Box<dyn std::error::Error>> {
    let texts = collect_texts("\"\\x{1F600}\"");
    // The string should be captured as a token
    assert!(
        !texts.is_empty(),
        "Should produce tokens for Unicode escape string"
    );
    Ok(())
}

#[test]
fn token_stream_deeply_nested_delimiters() -> Result<(), Box<dyn std::error::Error>> {
    let src = "((([[[{{{1}}}]]])))";
    let kinds = collect_kinds(src);
    let left_parens = kinds.iter().filter(|k| **k == TokenKind::LeftParen).count();
    let right_parens = kinds
        .iter()
        .filter(|k| **k == TokenKind::RightParen)
        .count();
    assert_eq!(left_parens, 3, "Expected 3 left parens");
    assert_eq!(right_parens, 3, "Expected 3 right parens");
    assert!(kinds.contains(&TokenKind::Number));
    Ok(())
}

#[test]
fn token_stream_comment_only_source() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("# just a comment\n");
    assert!(s.is_eof(), "Comment-only source should be EOF");
    Ok(())
}

#[test]
fn token_stream_multiple_comments_only() -> Result<(), Box<dyn std::error::Error>> {
    let src = "# line 1\n# line 2\n# line 3\n";
    let mut s = TokenStream::new(src);
    assert!(s.is_eof(), "Multiple comments only should be EOF");
    Ok(())
}

#[test]
fn token_stream_newlines_only() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("\n\n\n\n\n");
    assert!(s.is_eof(), "Newlines-only source should be EOF");
    Ok(())
}

#[test]
fn token_stream_single_character_source() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new(";");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Semicolon);
    Ok(())
}

#[test]
fn token_stream_multiple_statements_same_line() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("my $x = 1; my $y = 2; my $z = 3;");
    let my_count = kinds.iter().filter(|k| **k == TokenKind::My).count();
    assert_eq!(my_count, 3, "Expected 3 'my' keywords on one line");
    let semi_count = kinds.iter().filter(|k| **k == TokenKind::Semicolon).count();
    assert_eq!(semi_count, 3, "Expected 3 semicolons");
    Ok(())
}

// ===================================================================
// 5. Token text fidelity for specific tokens
// ===================================================================

#[test]
fn token_text_preserves_keyword_text() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("foreach");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Foreach);
    assert_eq!(t.text.as_ref(), "foreach");
    Ok(())
}

#[test]
fn token_text_preserves_number_formats() -> Result<(), Box<dyn std::error::Error>> {
    // Integer
    let mut s = TokenStream::new("42");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Number);
    assert_eq!(t.text.as_ref(), "42");

    // Hex
    let mut s = TokenStream::new("0xFF");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Number);

    // Octal
    let mut s = TokenStream::new("0777");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Number);

    // Binary
    let mut s = TokenStream::new("0b1010");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Number);

    // Float
    let mut s = TokenStream::new("3.14");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Number);
    Ok(())
}

#[test]
fn token_text_preserves_operator_text() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("$a <=> $b");
    let _ = must(s.next()); // $a
    let t = must(s.next()); // <=>
    assert_eq!(t.kind, TokenKind::Spaceship);
    assert_eq!(t.text.as_ref(), "<=>");
    Ok(())
}

// ===================================================================
// 6. Token byte positions
// ===================================================================

#[test]
fn token_positions_are_correct_for_simple_source() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("my $x");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::My);
    assert_eq!(t.start, 0);
    assert_eq!(t.end, 2);
    Ok(())
}

#[test]
fn token_positions_account_for_whitespace() -> Result<(), Box<dyn std::error::Error>> {
    // "   42" -> number starts at byte 3
    let mut s = TokenStream::new("   42");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Number);
    assert_eq!(t.start, 3);
    assert_eq!(t.end, 5);
    Ok(())
}

#[test]
fn token_positions_account_for_comments() -> Result<(), Box<dyn std::error::Error>> {
    // Comment takes bytes 0..11, newline at 11, then "42" starts at 12
    let src = "# comment\n42";
    let mut s = TokenStream::new(src);
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Number);
    assert!(
        t.start >= 10,
        "Number should start after comment, start={}",
        t.start
    );
    Ok(())
}

// ===================================================================
// 7. PositionTracker — additional coverage
// ===================================================================

#[test]
fn position_tracker_wrap_token_multiline() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line1\nline2\nline3";
    let tracker = PositionTracker::new(source);

    let token = perl_lexer::Token::new(
        perl_lexer::TokenType::Identifier(std::sync::Arc::from("line3")),
        std::sync::Arc::from("line3"),
        12, // byte offset of "line3"
        17,
    );
    let wrapped = tracker.wrap_token(token);
    assert_eq!(wrapped.start_pos.line, 3);
    assert_eq!(wrapped.start_pos.column, 1);
    assert_eq!(wrapped.end_pos.line, 3);
    assert_eq!(wrapped.end_pos.column, 6);
    Ok(())
}

#[test]
fn position_tracker_crlf_lines() -> Result<(), Box<dyn std::error::Error>> {
    // CRLF: each line ending is 2 bytes
    let source = "ab\r\ncd\r\nef";
    let tracker = PositionTracker::new(source);

    // 'c' is at byte 4 (after "ab\r\n")
    let pos = tracker.byte_to_position(4);
    assert_eq!(pos.line, 2);
    assert_eq!(pos.column, 1);
    Ok(())
}

#[test]
fn position_tracker_empty_lines() -> Result<(), Box<dyn std::error::Error>> {
    let source = "a\n\n\nb";
    let tracker = PositionTracker::new(source);

    // 'b' is at byte 4 (a + \n + \n + \n)
    let pos = tracker.byte_to_position(4);
    assert_eq!(pos.line, 4);
    assert_eq!(pos.column, 1);
    Ok(())
}

#[test]
fn position_tracker_unicode_column_counting() -> Result<(), Box<dyn std::error::Error>> {
    // Japanese characters are 3 bytes each in UTF-8
    let source = "\u{65E5}\u{672C}\u{8A9E}b"; // 日本語b
    let tracker = PositionTracker::new(source);

    // 'b' is at byte 9 (3 * 3-byte chars)
    let pos = tracker.byte_to_position(9);
    assert_eq!(pos.line, 1);
    // Column should be 4 (after 3 Unicode chars), not 10
    assert_eq!(pos.column, 4);
    Ok(())
}

#[test]
fn position_tracker_emoji_column_counting() -> Result<(), Box<dyn std::error::Error>> {
    // Crab emoji is 4 bytes in UTF-8
    let source = "\u{1F980}x"; // crab emoji + x
    let tracker = PositionTracker::new(source);

    // 'x' is at byte 4
    let pos = tracker.byte_to_position(4);
    assert_eq!(pos.line, 1);
    assert_eq!(pos.column, 2);
    Ok(())
}

// ===================================================================
// 8. TokenWithPosition accessors
// ===================================================================

#[test]
fn token_with_position_kind_and_text() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 42;";
    let tracker = PositionTracker::new(source);

    let token = perl_lexer::Token::new(
        perl_lexer::TokenType::Keyword(std::sync::Arc::from("my")),
        std::sync::Arc::from("my"),
        0,
        2,
    );

    let wrapped = tracker.wrap_token(token);
    assert!(matches!(wrapped.kind(), perl_lexer::TokenType::Keyword(_)));
    assert_eq!(wrapped.text(), "my");
    assert_eq!(wrapped.byte_range(), (0, 2));
    Ok(())
}

#[test]
fn token_with_position_range() -> Result<(), Box<dyn std::error::Error>> {
    let source = "hello";
    let tracker = PositionTracker::new(source);

    let token = perl_lexer::Token::new(
        perl_lexer::TokenType::Identifier(std::sync::Arc::from("hello")),
        std::sync::Arc::from("hello"),
        0,
        5,
    );

    let wrapped = tracker.wrap_token(token);
    let range = wrapped.range();
    assert_eq!(range.start.line, 1);
    assert_eq!(range.start.column, 1);
    assert_eq!(range.end.line, 1);
    assert_eq!(range.end.column, 6);
    Ok(())
}

// ===================================================================
// 9. TriviaLexer — additional coverage
// ===================================================================

#[test]
fn trivia_lexer_empty_source() -> Result<(), Box<dyn std::error::Error>> {
    let mut lexer = TriviaLexer::new(String::new());
    // Empty source should produce no tokens (or just EOF)
    let result = lexer.next_token_with_trivia();
    // Either None or an EOF-like result
    if let Some((_token, _trivia)) = result {
        // If we get something, it should have empty or EOF characteristics
    }
    // The key is it doesn't crash
    Ok(())
}

#[test]
fn trivia_lexer_whitespace_only_source() -> Result<(), Box<dyn std::error::Error>> {
    let mut lexer = TriviaLexer::new("   \n\n  \t  ".to_string());
    // Should produce trivia but eventually exhaust tokens
    let mut total_trivia = Vec::new();
    while let Some((_token, trivia)) = lexer.next_token_with_trivia() {
        total_trivia.extend(trivia);
    }
    // Should have found some whitespace/newline trivia
    let has_ws = total_trivia
        .iter()
        .any(|t| matches!(&t.trivia, Trivia::Whitespace(_) | Trivia::Newline));
    assert!(
        has_ws,
        "Whitespace-only source should produce whitespace trivia"
    );
    Ok(())
}

#[test]
fn trivia_lexer_consecutive_comments() -> Result<(), Box<dyn std::error::Error>> {
    let source = "# comment 1\n# comment 2\n# comment 3\nmy $x;".to_string();
    let mut lexer = TriviaLexer::new(source);

    let mut all_trivia = Vec::new();
    while let Some((_token, trivia)) = lexer.next_token_with_trivia() {
        all_trivia.extend(trivia);
    }

    let comment_count = all_trivia
        .iter()
        .filter(|t| matches!(&t.trivia, Trivia::LineComment(_)))
        .count();
    assert!(
        comment_count >= 2,
        "Expected at least 2 comments, got {comment_count}"
    );
    Ok(())
}

#[test]
fn trivia_lexer_interleaved_code_and_comments() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x; # assign x\nmy $y; # assign y\n".to_string();
    let mut lexer = TriviaLexer::new(source);

    let mut tokens_with_trivia = Vec::new();
    while let Some((token, trivia)) = lexer.next_token_with_trivia() {
        tokens_with_trivia.push((token, trivia));
    }

    // Should have at least 2 meaningful tokens (from 'my $x;' and 'my $y;')
    assert!(
        tokens_with_trivia.len() >= 2,
        "Expected at least 2 tokens, got {}",
        tokens_with_trivia.len()
    );
    Ok(())
}

// ===================================================================
// 10. TriviaParserContext — additional coverage
// ===================================================================

#[test]
fn trivia_context_empty_source_is_eof() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = TriviaParserContext::new(String::new());
    assert!(ctx.is_eof(), "Empty source should be at EOF");
    Ok(())
}

#[test]
fn trivia_context_non_empty_source_is_not_eof() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = TriviaParserContext::new("42".to_string());
    assert!(
        !ctx.is_eof(),
        "Non-empty source should not be at EOF initially"
    );
    Ok(())
}

#[test]
fn trivia_context_whitespace_only_eof_behavior() -> Result<(), Box<dyn std::error::Error>> {
    // Whitespace-only source: all tokens are trivia, so the context may be at EOF
    // or may contain an EOF token with trivia attached
    let ctx = TriviaParserContext::new("   \n\n  ".to_string());
    // The key test is that it doesn't crash
    let _is_eof = ctx.is_eof();
    Ok(())
}

#[test]
fn trivia_context_preserves_leading_whitespace_via_parser() -> Result<(), Box<dyn std::error::Error>>
{
    let parser = TriviaPreservingParser::new("   42".to_string());
    let result = parser.parse();

    let has_ws = result
        .leading_trivia
        .iter()
        .any(|t| matches!(&t.trivia, Trivia::Whitespace(_)));
    assert!(has_ws, "Parser should preserve leading whitespace trivia");
    Ok(())
}

#[test]
fn trivia_context_preserves_leading_comment_via_parser() -> Result<(), Box<dyn std::error::Error>> {
    let parser = TriviaPreservingParser::new("# preamble\n42".to_string());
    let result = parser.parse();

    let has_comment = result
        .leading_trivia
        .iter()
        .any(|t| matches!(&t.trivia, Trivia::LineComment(_)));
    assert!(has_comment, "Parser should preserve leading comment trivia");
    Ok(())
}

// ===================================================================
// 11. TriviaPreservingParser — additional coverage
// ===================================================================

#[test]
fn trivia_parser_empty_source_produces_program_node() -> Result<(), Box<dyn std::error::Error>> {
    let parser = TriviaPreservingParser::new(String::new());
    let result = parser.parse();
    // Should produce a Program node (even if empty)
    assert!(
        matches!(&result.node.kind, perl_ast_v2::NodeKind::Program { .. }),
        "Expected Program node from empty source"
    );
    Ok(())
}

#[test]
fn trivia_parser_comment_only_preserves_trivia() -> Result<(), Box<dyn std::error::Error>> {
    let parser = TriviaPreservingParser::new("# just a comment\n".to_string());
    let result = parser.parse();

    // Should have comment as trivia
    let has_comment = result
        .leading_trivia
        .iter()
        .any(|t| matches!(&t.trivia, Trivia::LineComment(_)));
    assert!(
        has_comment,
        "Comment-only source should preserve comment as trivia"
    );
    Ok(())
}

// ===================================================================
// 12. format_with_trivia
// ===================================================================

#[test]
fn format_with_trivia_includes_leading_trivia() -> Result<(), Box<dyn std::error::Error>> {
    let parser = TriviaPreservingParser::new("# comment\nmy $x;".to_string());
    let result = parser.parse();
    let formatted = format_with_trivia(&result);

    // Should contain the comment text somewhere in the output
    assert!(
        formatted.contains("# comment"),
        "format_with_trivia should include leading comment trivia, got: {}",
        formatted
    );
    Ok(())
}

#[test]
fn format_with_trivia_includes_whitespace_trivia() -> Result<(), Box<dyn std::error::Error>> {
    let parser = TriviaPreservingParser::new("  my $x;".to_string());
    let result = parser.parse();
    let formatted = format_with_trivia(&result);

    // Should contain the whitespace prefix
    assert!(
        formatted.starts_with("  "),
        "format_with_trivia should include leading whitespace trivia, got: {:?}",
        formatted
    );
    Ok(())
}

// ===================================================================
// 13. Utility functions — additional edge cases
// ===================================================================

#[test]
fn find_data_marker_empty_source() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(find_data_marker_byte_lexed(""), None);
    Ok(())
}

#[test]
fn find_data_marker_no_marker_present() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(find_data_marker_byte_lexed("my $x = 1;\nprint $x;\n"), None);
    Ok(())
}

#[test]
fn find_data_marker_in_string_is_not_marker() -> Result<(), Box<dyn std::error::Error>> {
    // __DATA__ inside a string should not be found
    let src = "my $x = '__DATA__';\n";
    assert_eq!(find_data_marker_byte_lexed(src), None);
    Ok(())
}

#[test]
fn code_slice_empty_source() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(code_slice(""), "");
    Ok(())
}

#[test]
fn code_slice_no_marker_returns_full() -> Result<(), Box<dyn std::error::Error>> {
    let src = "print 'hello';\nprint 'world';\n";
    assert_eq!(code_slice(src), src);
    Ok(())
}

#[test]
fn code_slice_with_data_returns_code_portion() -> Result<(), Box<dyn std::error::Error>> {
    let src = "my $x = 1;\n__DATA__\nstuff";
    let result = code_slice(src);
    assert_eq!(result, "my $x = 1;\n");
    assert!(!result.contains("__DATA__"));
    Ok(())
}

#[test]
fn code_slice_with_end_returns_code_portion() -> Result<(), Box<dyn std::error::Error>> {
    let src = "my $x = 1;\n__END__\nstuff";
    let result = code_slice(src);
    assert_eq!(result, "my $x = 1;\n");
    assert!(!result.contains("__END__"));
    Ok(())
}

// ===================================================================
// 14. TokenStream — EOF stickiness edge cases
// ===================================================================

#[test]
fn token_stream_eof_sticky_after_peek() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("");
    // Peek sees EOF
    assert_eq!(must(s.peek()).kind, TokenKind::Eof);
    // next should also return EOF
    assert_eq!(must(s.next()).kind, TokenKind::Eof);
    // And again
    assert_eq!(must(s.next()).kind, TokenKind::Eof);
    // peek still sees EOF
    assert_eq!(must(s.peek()).kind, TokenKind::Eof);
    Ok(())
}

#[test]
fn token_stream_peek_second_at_eof_returns_error() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("42");
    let _ = must(s.next()); // 42
    // Now at EOF
    assert_eq!(must(s.peek()).kind, TokenKind::Eof);
    // peek_second tries to read past the sticky EOF peeked token
    // and gets UnexpectedEof from the exhausted lexer
    assert!(
        s.peek_second().is_err(),
        "peek_second at EOF should return Err"
    );
    Ok(())
}

#[test]
fn token_stream_peek_third_at_eof_returns_error() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("42");
    let _ = must(s.next()); // 42
    // peek_third also returns Err when the lexer is exhausted
    assert!(
        s.peek_third().is_err(),
        "peek_third at EOF should return Err"
    );
    Ok(())
}

// ===================================================================
// 15. TokenStream — stmt boundary with peek chain
// ===================================================================

#[test]
fn on_stmt_boundary_clears_all_peek_slots() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("my $x = 1; our $y = 2;");
    // Fill all peek slots
    let _ = must(s.peek());
    let _ = must(s.peek_second());
    let _ = must(s.peek_third());

    // Consume first statement
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Semicolon {
            break;
        }
    }

    // Reset at boundary
    s.on_stmt_boundary();

    // After boundary, should re-lex from current position
    let t = must(s.peek());
    assert_eq!(
        t.kind,
        TokenKind::Our,
        "After stmt boundary, should see 'our'"
    );
    Ok(())
}

// ===================================================================
// 16. Compound Perl constructs through the bridge
// ===================================================================

#[test]
fn token_stream_hash_slice() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("@hash{qw(a b c)}");
    assert!(
        kinds.contains(&TokenKind::Identifier),
        "Hash slice should contain identifier"
    );
    assert!(
        kinds.contains(&TokenKind::LeftBrace),
        "Hash slice should contain left brace"
    );
    Ok(())
}

#[test]
fn token_stream_chained_method_calls() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("$obj->foo->bar->baz");
    let arrow_count = kinds.iter().filter(|k| **k == TokenKind::Arrow).count();
    assert_eq!(arrow_count, 3, "Chained calls should have 3 arrows");
    Ok(())
}

#[test]
fn token_stream_complex_regex_with_modifiers() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("s/foo(bar)/baz$1/gixms");
    assert!(
        kinds.contains(&TokenKind::Substitution),
        "Complex substitution should produce Substitution token"
    );
    Ok(())
}

#[test]
fn token_stream_heredoc_in_expression() -> Result<(), Box<dyn std::error::Error>> {
    let src = "my $x = <<EOF . 'suffix';\nhello\nEOF\n";
    let kinds = collect_kinds(src);
    assert!(kinds.contains(&TokenKind::My), "Should see 'my' keyword");
    assert!(
        kinds.contains(&TokenKind::HeredocStart),
        "Should see HeredocStart"
    );
    Ok(())
}

#[test]
fn token_stream_anonymous_sub() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("my $fn = sub { return 42; };");
    assert!(kinds.contains(&TokenKind::My));
    assert!(kinds.contains(&TokenKind::Sub));
    assert!(kinds.contains(&TokenKind::Return));
    Ok(())
}

#[test]
fn token_stream_try_catch_finally() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("try { die 'oops'; } catch ($e) { warn $e; } finally { cleanup(); }");
    assert!(kinds.contains(&TokenKind::Try));
    assert!(kinds.contains(&TokenKind::Catch));
    assert!(kinds.contains(&TokenKind::Finally));
    Ok(())
}

#[test]
fn token_stream_class_declaration() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("class Foo { field $x; method bar { return $x; } }");
    assert!(kinds.contains(&TokenKind::Class));
    assert!(kinds.contains(&TokenKind::Field));
    assert!(kinds.contains(&TokenKind::Method));
    Ok(())
}
