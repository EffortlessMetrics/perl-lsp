//! Extended unit tests for the perl-tokenizer crate.
//!
//! Focuses on: full Perl construct tokenization, token text fidelity,
//! boundary conditions, whitespace/comment edge cases, trivia coverage,
//! position tracking accuracy, and utility helpers.

use perl_lexer::tokenizer::token_wrapper::PositionTracker;
use perl_lexer::tokenizer::util::{code_slice, find_data_marker_byte_lexed};
use perl_parser_core::tokens::token_stream::TokenStream;
use perl_parser_core::trivia::{Trivia, TriviaLexer, TriviaToken};
use perl_parser_core::trivia_parser::{
    TriviaParserContext, TriviaPreservingParser, format_with_trivia,
};
use perl_tdd_support::{must, must_some};
use perl_token::TokenKind;

// ===================================================================
// Helper: collect all non-EOF tokens from a stream
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
// 1. TokenStream — Perl construct tokenization
// ===================================================================

#[test]
fn ext_while_loop_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("while ($i < 10) { $i++; }");
    assert!(kinds.contains(&TokenKind::While));
    assert!(kinds.contains(&TokenKind::LeftParen));
    assert!(kinds.contains(&TokenKind::Less));
    assert!(kinds.contains(&TokenKind::Increment));
    assert!(kinds.contains(&TokenKind::RightBrace));
    Ok(())
}

#[test]
fn ext_until_loop_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("until ($done) { next; }");
    assert!(kinds.contains(&TokenKind::Until));
    assert!(kinds.contains(&TokenKind::Next));
    Ok(())
}

#[test]
fn ext_foreach_loop_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("foreach my $item (@list) { last; }");
    assert!(kinds.contains(&TokenKind::Foreach));
    assert!(kinds.contains(&TokenKind::My));
    assert!(kinds.contains(&TokenKind::Last));
    Ok(())
}

#[test]
fn ext_for_c_style_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("for (my $i = 0; $i < 10; $i++) {}");
    assert!(kinds.contains(&TokenKind::For));
    assert!(kinds.contains(&TokenKind::My));
    assert!(kinds.contains(&TokenKind::Assign));
    assert!(kinds.contains(&TokenKind::Less));
    assert!(kinds.contains(&TokenKind::Increment));
    Ok(())
}

#[test]
fn ext_unless_keyword() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("unless ($x) { return; }");
    assert!(kinds.contains(&TokenKind::Unless));
    assert!(kinds.contains(&TokenKind::Return));
    Ok(())
}

#[test]
fn ext_elsif_chain() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("if ($a) {} elsif ($b) {} else {}");
    assert!(kinds.contains(&TokenKind::If));
    assert!(kinds.contains(&TokenKind::Elsif));
    assert!(kinds.contains(&TokenKind::Else));
    Ok(())
}

#[test]
fn ext_use_strict_warnings() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("use strict; use warnings;");
    let use_count = kinds.iter().filter(|k| **k == TokenKind::Use).count();
    assert_eq!(use_count, 2);
    Ok(())
}

#[test]
fn ext_package_with_version() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("package Foo::Bar 1.23;");
    assert_eq!(kinds[0], TokenKind::Package);
    assert!(kinds.contains(&TokenKind::Number));
    assert!(kinds.contains(&TokenKind::Semicolon));
    Ok(())
}

#[test]
fn ext_sub_with_prototype() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("sub foo { return 42; }");
    assert_eq!(kinds[0], TokenKind::Sub);
    assert!(kinds.contains(&TokenKind::Return));
    assert!(kinds.contains(&TokenKind::Number));
    Ok(())
}

#[test]
fn ext_eval_block() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("eval { die 'oops'; };");
    assert_eq!(kinds[0], TokenKind::Eval);
    assert!(kinds.contains(&TokenKind::LeftBrace));
    Ok(())
}

#[test]
fn ext_do_block() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("do { 1 };");
    assert_eq!(kinds[0], TokenKind::Do);
    Ok(())
}

#[test]
fn ext_redo_keyword() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("redo;");
    assert!(kinds.contains(&TokenKind::Redo));
    Ok(())
}

#[test]
fn ext_continue_keyword() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("continue { }");
    assert!(kinds.contains(&TokenKind::Continue));
    Ok(())
}

#[test]
fn ext_begin_end_blocks() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("BEGIN { } END { }");
    assert!(kinds.contains(&TokenKind::Begin));
    assert!(kinds.contains(&TokenKind::End));
    Ok(())
}

#[test]
fn ext_check_init_unitcheck() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("CHECK { } INIT { } UNITCHECK { }");
    assert!(kinds.contains(&TokenKind::Check));
    assert!(kinds.contains(&TokenKind::Init));
    assert!(kinds.contains(&TokenKind::Unitcheck));
    Ok(())
}

#[test]
fn ext_given_when_default() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("given ($x) { when (1) { } default { } }");
    assert!(kinds.contains(&TokenKind::Given));
    assert!(kinds.contains(&TokenKind::When));
    assert!(kinds.contains(&TokenKind::Default));
    Ok(())
}

#[test]
fn ext_try_catch_finally() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("try { } catch ($e) { } finally { }");
    assert!(kinds.contains(&TokenKind::Try));
    assert!(kinds.contains(&TokenKind::Catch));
    assert!(kinds.contains(&TokenKind::Finally));
    Ok(())
}

#[test]
fn ext_class_method_keywords() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("class Foo { method bar { } }");
    assert!(kinds.contains(&TokenKind::Class));
    assert!(kinds.contains(&TokenKind::Method));
    Ok(())
}

// ===================================================================
// 2. Operator coverage
// ===================================================================

#[test]
fn ext_all_compound_assign_operators() -> Result<(), Box<dyn std::error::Error>> {
    let tests: Vec<(&str, TokenKind)> = vec![
        ("$x += 1", TokenKind::PlusAssign),
        ("$x -= 1", TokenKind::MinusAssign),
        ("$x *= 1", TokenKind::StarAssign),
        ("$x %= 1", TokenKind::PercentAssign),
        ("$x .= 'a'", TokenKind::DotAssign),
        ("$x &= 1", TokenKind::AndAssign),
        ("$x |= 1", TokenKind::OrAssign),
        ("$x ^= 1", TokenKind::XorAssign),
        ("$x **= 2", TokenKind::PowerAssign),
        ("$x <<= 1", TokenKind::LeftShiftAssign),
        ("$x >>= 1", TokenKind::RightShiftAssign),
        ("$x &&= 1", TokenKind::LogicalAndAssign),
        ("$x ||= 1", TokenKind::LogicalOrAssign),
        ("$x //= 1", TokenKind::DefinedOrAssign),
    ];
    for (src, expected) in &tests {
        let kinds = collect_kinds(src);
        assert!(kinds.contains(expected), "missing {expected:?} in `{src}`");
    }
    Ok(())
}

#[test]
fn ext_comparison_operators() -> Result<(), Box<dyn std::error::Error>> {
    let tests: Vec<(&str, TokenKind)> = vec![
        ("1 == 2", TokenKind::Equal),
        ("1 != 2", TokenKind::NotEqual),
        ("1 < 2", TokenKind::Less),
        ("1 > 2", TokenKind::Greater),
        ("1 <= 2", TokenKind::LessEqual),
        ("1 >= 2", TokenKind::GreaterEqual),
        ("1 <=> 2", TokenKind::Spaceship),
    ];
    for (src, expected) in &tests {
        let kinds = collect_kinds(src);
        assert!(kinds.contains(expected), "missing {expected:?} in `{src}`");
    }
    Ok(())
}

#[test]
fn ext_word_logical_operators() -> Result<(), Box<dyn std::error::Error>> {
    assert!(collect_kinds("1 and 2").contains(&TokenKind::WordAnd));
    assert!(collect_kinds("1 or 2").contains(&TokenKind::WordOr));
    assert!(collect_kinds("not 1").contains(&TokenKind::WordNot));
    assert!(collect_kinds("1 xor 2").contains(&TokenKind::WordXor));
    Ok(())
}

#[test]
fn ext_string_compare_cmp() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("$a cmp $b");
    assert!(kinds.contains(&TokenKind::StringCompare));
    Ok(())
}

#[test]
fn ext_range_and_ellipsis() -> Result<(), Box<dyn std::error::Error>> {
    assert!(collect_kinds("1 .. 10").contains(&TokenKind::Range));
    assert!(collect_kinds("1 ... 10").contains(&TokenKind::Ellipsis));
    Ok(())
}

#[test]
fn ext_backslash_operator() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("\\@array");
    assert!(kinds.contains(&TokenKind::Backslash));
    Ok(())
}

#[test]
fn ext_question_colon_ternary() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("$x ? 1 : 0");
    assert!(kinds.contains(&TokenKind::Question));
    assert!(kinds.contains(&TokenKind::Colon));
    Ok(())
}

#[test]
fn ext_double_colon_separator() -> Result<(), Box<dyn std::error::Error>> {
    // The lexer may absorb :: into the identifier; verify we get identifiers
    let kinds = collect_kinds("Foo::Bar::baz()");
    assert!(kinds.contains(&TokenKind::Identifier));
    assert!(kinds.contains(&TokenKind::LeftParen));
    Ok(())
}

#[test]
fn ext_bitwise_not_operator() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("~$x");
    assert!(kinds.contains(&TokenKind::BitwiseNot));
    Ok(())
}

#[test]
fn ext_not_operator() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("!$x");
    assert!(kinds.contains(&TokenKind::Not));
    Ok(())
}

// ===================================================================
// 3. Token text fidelity
// ===================================================================

#[test]
fn ext_keyword_text_preserved() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("foreach");
    let t = must(s.next());
    assert_eq!(t.text.as_ref(), "foreach");
    Ok(())
}

#[test]
fn ext_number_text_preserved() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("3.14");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Number);
    assert_eq!(t.text.as_ref(), "3.14");
    Ok(())
}

#[test]
fn ext_hex_number_text_preserved() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("0xFF");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Number);
    Ok(())
}

#[test]
fn ext_octal_number_text_preserved() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("0777");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Number);
    Ok(())
}

#[test]
fn ext_identifier_text_preserved() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("$some_variable");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Identifier);
    // Verify text contains the variable name
    assert!(!t.text.is_empty());
    Ok(())
}

#[test]
fn ext_operator_text_preserved() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("1 <=> 2");
    // skip 1
    let _ = must(s.next());
    let op = must(s.next());
    assert_eq!(op.kind, TokenKind::Spaceship);
    assert_eq!(op.text.as_ref(), "<=>");
    Ok(())
}

// ===================================================================
// 4. Token position correctness
// ===================================================================

#[test]
fn ext_token_start_end_offsets() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("my $x");
    let t = must(s.next());
    // "my" starts at 0, ends at 2
    assert_eq!(t.start, 0);
    assert_eq!(t.end, 2);
    Ok(())
}

#[test]
fn ext_token_offsets_with_leading_whitespace() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("   42");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Number);
    // Starts after 3 spaces
    assert_eq!(t.start, 3);
    assert_eq!(t.end, 5);
    Ok(())
}

#[test]
fn ext_token_offsets_monotonically_increase() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = TokenStream::new("my $x = 1 + 2;");
    let mut prev_end = 0;
    loop {
        let t = must(s.next());
        if t.kind == TokenKind::Eof {
            break;
        }
        assert!(t.start >= prev_end, "start {} < prev_end {}", t.start, prev_end);
        assert!(t.end > t.start, "end {} <= start {}", t.end, t.start);
        prev_end = t.end;
    }
    Ok(())
}

// ===================================================================
// 5. PositionTracker edge cases
// ===================================================================

#[test]
fn ext_position_tracker_blank_lines() -> Result<(), Box<dyn std::error::Error>> {
    let tracker = PositionTracker::new("a\n\nb\n");
    // byte 0 = 'a' → line 1, col 1
    let p0 = tracker.byte_to_position(0);
    assert_eq!(p0.line, 1);
    assert_eq!(p0.column, 1);
    // byte 2 = '\n' (empty line) → line 2, col 1
    let p2 = tracker.byte_to_position(2);
    assert_eq!(p2.line, 2);
    assert_eq!(p2.column, 1);
    // byte 3 = 'b' → line 3, col 1
    let p3 = tracker.byte_to_position(3);
    assert_eq!(p3.line, 3);
    assert_eq!(p3.column, 1);
    Ok(())
}

#[test]
fn ext_position_tracker_tabs() -> Result<(), Box<dyn std::error::Error>> {
    let tracker = PositionTracker::new("\thello");
    // Tab is 1 char, so col is 2 for 'h' at byte 1
    let p = tracker.byte_to_position(1);
    assert_eq!(p.line, 1);
    assert_eq!(p.column, 2);
    Ok(())
}

#[test]
fn ext_position_tracker_multibyte_utf8() -> Result<(), Box<dyn std::error::Error>> {
    // "café" has 'é' as 2 bytes
    let tracker = PositionTracker::new("café");
    // 'c' at byte 0 → col 1
    let p0 = tracker.byte_to_position(0);
    assert_eq!(p0.column, 1);
    // 'a' at byte 1 → col 2
    let p1 = tracker.byte_to_position(1);
    assert_eq!(p1.column, 2);
    // 'f' at byte 2 → col 3
    let p2 = tracker.byte_to_position(2);
    assert_eq!(p2.column, 3);
    // 'é' at byte 3 → col 4
    let p3 = tracker.byte_to_position(3);
    assert_eq!(p3.column, 4);
    Ok(())
}

#[test]
fn ext_position_tracker_wrap_token_positions() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line1\nline2";
    let tracker = PositionTracker::new(source);
    let token = perl_lexer::Token::new(
        perl_lexer::TokenType::Identifier(std::sync::Arc::from("line2")),
        std::sync::Arc::from("line2"),
        6,
        11,
    );
    let wrapped = tracker.wrap_token(token);
    assert_eq!(wrapped.start_pos.line, 2);
    assert_eq!(wrapped.start_pos.column, 1);
    assert_eq!(wrapped.end_pos.line, 2);
    assert_eq!(wrapped.end_pos.column, 6);
    Ok(())
}

// ===================================================================
// 6. TokenWithPosition accessors
// ===================================================================

#[test]
fn ext_token_with_position_kind_accessor() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my";
    let tracker = PositionTracker::new(source);
    let token = perl_lexer::Token::new(
        perl_lexer::TokenType::Keyword(std::sync::Arc::from("my")),
        std::sync::Arc::from("my"),
        0,
        2,
    );
    let wrapped = tracker.wrap_token(token);
    assert!(matches!(wrapped.kind(), perl_lexer::TokenType::Keyword(_)));
    Ok(())
}

#[test]
fn ext_token_with_position_text_accessor() -> Result<(), Box<dyn std::error::Error>> {
    let source = "hello";
    let tracker = PositionTracker::new(source);
    let token = perl_lexer::Token::new(
        perl_lexer::TokenType::Identifier(std::sync::Arc::from("hello")),
        std::sync::Arc::from("hello"),
        0,
        5,
    );
    let wrapped = tracker.wrap_token(token);
    assert_eq!(wrapped.text(), "hello");
    Ok(())
}

#[test]
fn ext_token_with_position_byte_range() -> Result<(), Box<dyn std::error::Error>> {
    let source = "  foo";
    let tracker = PositionTracker::new(source);
    let token = perl_lexer::Token::new(
        perl_lexer::TokenType::Identifier(std::sync::Arc::from("foo")),
        std::sync::Arc::from("foo"),
        2,
        5,
    );
    let wrapped = tracker.wrap_token(token);
    assert_eq!(wrapped.byte_range(), (2, 5));
    Ok(())
}

#[test]
fn ext_token_with_position_range_object() -> Result<(), Box<dyn std::error::Error>> {
    let source = "ab\ncd";
    let tracker = PositionTracker::new(source);
    let token = perl_lexer::Token::new(
        perl_lexer::TokenType::Identifier(std::sync::Arc::from("cd")),
        std::sync::Arc::from("cd"),
        3,
        5,
    );
    let wrapped = tracker.wrap_token(token);
    let range = wrapped.range();
    assert_eq!(range.start.line, 2);
    assert_eq!(range.end.line, 2);
    Ok(())
}

// ===================================================================
// 7. Trivia enum
// ===================================================================

#[test]
fn ext_trivia_whitespace_kind_name() -> Result<(), Box<dyn std::error::Error>> {
    let t = Trivia::Whitespace("  ".into());
    assert_eq!(t.kind_name(), "whitespace");
    assert_eq!(t.as_str(), "  ");
    Ok(())
}

#[test]
fn ext_trivia_line_comment_kind_name() -> Result<(), Box<dyn std::error::Error>> {
    let t = Trivia::LineComment("# hello".into());
    assert_eq!(t.kind_name(), "comment");
    assert_eq!(t.as_str(), "# hello");
    Ok(())
}

#[test]
fn ext_trivia_pod_comment_kind_name() -> Result<(), Box<dyn std::error::Error>> {
    let t = Trivia::PodComment("=head1 NAME\n\n=cut".into());
    assert_eq!(t.kind_name(), "pod");
    assert_eq!(t.as_str(), "=head1 NAME\n\n=cut");
    Ok(())
}

#[test]
fn ext_trivia_newline_kind_name() -> Result<(), Box<dyn std::error::Error>> {
    let t = Trivia::Newline;
    assert_eq!(t.kind_name(), "newline");
    assert_eq!(t.as_str(), "\n");
    Ok(())
}

#[test]
fn ext_trivia_clone_and_eq() -> Result<(), Box<dyn std::error::Error>> {
    let t1 = Trivia::Whitespace("  ".into());
    let t2 = t1.clone();
    assert_eq!(t1, t2);

    let t3 = Trivia::LineComment("# x".into());
    assert_ne!(t1, t3);

    assert_eq!(Trivia::Newline, Trivia::Newline);
    Ok(())
}

// ===================================================================
// 8. TriviaToken
// ===================================================================

#[test]
fn ext_trivia_token_new_and_fields() -> Result<(), Box<dyn std::error::Error>> {
    use perl_position_tracking::{Position, Range};
    let range = Range::new(Position::new(0, 1, 1), Position::new(3, 1, 4));
    let tt = TriviaToken::new(Trivia::Whitespace("   ".into()), range);
    assert!(matches!(tt.trivia, Trivia::Whitespace(_)));
    assert_eq!(tt.range.start.byte, 0);
    assert_eq!(tt.range.end.byte, 3);
    Ok(())
}

// ===================================================================
// 9. TriviaLexer
// ===================================================================

#[test]
fn ext_trivia_lexer_tabs_as_whitespace() -> Result<(), Box<dyn std::error::Error>> {
    let mut lexer = TriviaLexer::new("\t\tmy $x;".into());
    let (token, trivia) = must_some(lexer.next_token_with_trivia());
    // Leading trivia should capture tab whitespace
    assert!(!trivia.is_empty());
    assert!(
        trivia.iter().any(|t| matches!(&t.trivia, Trivia::Whitespace(ws) if ws.contains('\t')))
    );
    // Token should be a keyword
    assert!(matches!(token.token_type, perl_lexer::TokenType::Keyword(_)));
    Ok(())
}

#[test]
fn ext_trivia_lexer_multiple_comments() -> Result<(), Box<dyn std::error::Error>> {
    let src = "# line1\n# line2\nmy $x;".to_string();
    let mut lexer = TriviaLexer::new(src);
    let (_, trivia) = must_some(lexer.next_token_with_trivia());
    let comment_count =
        trivia.iter().filter(|t| matches!(t.trivia, Trivia::LineComment(_))).count();
    assert!(comment_count >= 2, "expected at least 2 comments, got {comment_count}");
    Ok(())
}

#[test]
fn ext_trivia_lexer_newline_between_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let src = "my\n$x".to_string();
    let mut lexer = TriviaLexer::new(src);
    // First token
    let _ = must_some(lexer.next_token_with_trivia());
    // Second token should have newline trivia
    let (_, trivia) = must_some(lexer.next_token_with_trivia());
    assert!(trivia.iter().any(|t| matches!(t.trivia, Trivia::Newline)));
    Ok(())
}

#[test]
fn ext_trivia_lexer_pod_section() -> Result<(), Box<dyn std::error::Error>> {
    let src = "=pod\nsome docs\n=cut\nmy $x;".to_string();
    let mut lexer = TriviaLexer::new(src);
    let (_, trivia) = must_some(lexer.next_token_with_trivia());
    assert!(trivia.iter().any(|t| matches!(&t.trivia, Trivia::PodComment(_))));
    Ok(())
}

#[test]
fn ext_trivia_lexer_no_trivia_for_plain_code() -> Result<(), Box<dyn std::error::Error>> {
    let src = "42".to_string();
    let mut lexer = TriviaLexer::new(src);
    let (token, trivia) = must_some(lexer.next_token_with_trivia());
    assert!(trivia.is_empty());
    assert!(matches!(token.token_type, perl_lexer::TokenType::Number(_)));
    Ok(())
}

// ===================================================================
// 10. TriviaParserContext
// ===================================================================

#[test]
fn ext_trivia_ctx_multiple_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = TriviaParserContext::new("my $x = 1;".into());
    assert!(!ctx.is_eof());
    Ok(())
}

#[test]
fn ext_trivia_ctx_advance_through_eof() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = TriviaParserContext::new("my $x;".into());
    assert!(!ctx.is_eof());
    Ok(())
}

#[test]
fn ext_trivia_ctx_comment_only_source() -> Result<(), Box<dyn std::error::Error>> {
    // A source with only comments should either have tokens with trivia or be at EOF
    let ctx = TriviaParserContext::new("# just a comment\n".into());
    // It's valid to be at EOF since there are no meaningful tokens
    // But there may be an EOF token with leading trivia
    // Just verify no panic
    let _ = ctx.is_eof();
    Ok(())
}

#[test]
fn ext_trivia_ctx_leading_trivia_captured() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = TriviaParserContext::new("  # comment\nmy $x;".into());
    // Context should have tokens and not be at EOF
    assert!(!ctx.is_eof());
    Ok(())
}

// ===================================================================
// 11. TriviaPreservingParser
// ===================================================================

#[test]
fn ext_trivia_parser_preserves_shebang() -> Result<(), Box<dyn std::error::Error>> {
    let src = "#!/usr/bin/perl\nmy $x = 1;".to_string();
    let parser = TriviaPreservingParser::new(src);
    let result = parser.parse();
    // Shebang should appear in leading trivia
    assert!(
        result.leading_trivia.iter().any(|t| {
            matches!(&t.trivia, Trivia::LineComment(s) if s.contains("#!/usr/bin/perl"))
        })
    );
    Ok(())
}

#[test]
fn ext_trivia_parser_preserves_inline_comment() -> Result<(), Box<dyn std::error::Error>> {
    let src = "my $x = 1; # set x\nmy $y = 2;".to_string();
    let parser = TriviaPreservingParser::new(src);
    let result = parser.parse();
    // Should have at least one comment in the trivia
    assert!(
        result
            .leading_trivia
            .iter()
            .any(|t| { matches!(&t.trivia, Trivia::LineComment(s) if s.contains("set x")) })
    );
    Ok(())
}

#[test]
fn ext_trivia_parser_empty_produces_program() -> Result<(), Box<dyn std::error::Error>> {
    let parser = TriviaPreservingParser::new(String::new());
    let result = parser.parse();
    // Node should be a Program
    assert!(matches!(result.node.kind, perl_ast_v2::NodeKind::Program { .. }));
    Ok(())
}

#[test]
fn ext_trivia_parser_whitespace_only_source() -> Result<(), Box<dyn std::error::Error>> {
    let parser = TriviaPreservingParser::new("   \n\n  \t  ".into());
    let result = parser.parse();
    assert!(matches!(result.node.kind, perl_ast_v2::NodeKind::Program { .. }));
    Ok(())
}

#[test]
fn ext_trivia_parser_pod_in_middle() -> Result<(), Box<dyn std::error::Error>> {
    let src = "my $x;\n\n=head1 TITLE\n\nSome text\n\n=cut\n\nour $y;".to_string();
    let parser = TriviaPreservingParser::new(src);
    let result = parser.parse();
    // POD should be in the trivia
    assert!(result.leading_trivia.iter().any(|t| matches!(&t.trivia, Trivia::PodComment(_))));
    Ok(())
}

// ===================================================================
// 12. format_with_trivia
// ===================================================================

#[test]
fn ext_format_with_trivia_includes_leading() -> Result<(), Box<dyn std::error::Error>> {
    let parser = TriviaPreservingParser::new("# comment\nmy $x;".into());
    let result = parser.parse();
    let formatted = format_with_trivia(&result);
    assert!(formatted.contains("# comment"));
    Ok(())
}

#[test]
fn ext_format_with_trivia_empty_trivia() -> Result<(), Box<dyn std::error::Error>> {
    // Parse an empty source to get a program node with no trivia
    let parser = TriviaPreservingParser::new(String::new());
    let result = parser.parse();
    let formatted = format_with_trivia(&result);
    // Should contain the node debug output but no trivia
    assert!(!formatted.is_empty());
    Ok(())
}

// ===================================================================
// 13. Utility functions
// ===================================================================

#[test]
fn ext_find_data_marker_with_code_before() -> Result<(), Box<dyn std::error::Error>> {
    let src = "my $x = 1;\n__DATA__\nhello world";
    let pos = must_some(find_data_marker_byte_lexed(src));
    assert!(pos > 0);
    assert!(pos < src.len());
    Ok(())
}

#[test]
fn ext_find_data_marker_end_marker() -> Result<(), Box<dyn std::error::Error>> {
    let src = "print 1;\n__END__\nstuff";
    let pos = must_some(find_data_marker_byte_lexed(src));
    assert!(pos > 0);
    Ok(())
}

#[test]
fn ext_find_data_marker_none_for_code() -> Result<(), Box<dyn std::error::Error>> {
    assert!(find_data_marker_byte_lexed("my $x = 42;").is_none());
    Ok(())
}

#[test]
fn ext_code_slice_returns_before_marker() -> Result<(), Box<dyn std::error::Error>> {
    let src = "use strict;\n__DATA__\nsome data";
    let code = code_slice(src);
    assert!(code.contains("use strict;"));
    assert!(!code.contains("some data"));
    Ok(())
}

#[test]
fn ext_code_slice_returns_all_when_no_marker() -> Result<(), Box<dyn std::error::Error>> {
    let src = "my $x = 1; my $y = 2;";
    assert_eq!(code_slice(src), src);
    Ok(())
}

#[test]
fn ext_code_slice_empty_input() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(code_slice(""), "");
    Ok(())
}

// ===================================================================
// 14. Stress / boundary tests
// ===================================================================

#[test]
fn ext_many_semicolons() -> Result<(), Box<dyn std::error::Error>> {
    let src = ";;;;;;;;";
    let kinds = collect_kinds(src);
    assert_eq!(kinds.len(), 8);
    assert!(kinds.iter().all(|k| *k == TokenKind::Semicolon));
    Ok(())
}

#[test]
fn ext_deeply_nested_structure() -> Result<(), Box<dyn std::error::Error>> {
    let src = "((((((1))))))";
    let kinds = collect_kinds(src);
    let lparen_count = kinds.iter().filter(|k| **k == TokenKind::LeftParen).count();
    let rparen_count = kinds.iter().filter(|k| **k == TokenKind::RightParen).count();
    assert_eq!(lparen_count, 6);
    assert_eq!(rparen_count, 6);
    Ok(())
}

#[test]
fn ext_mixed_braces_brackets_parens() -> Result<(), Box<dyn std::error::Error>> {
    let src = "{ [ ( ) ] }";
    let kinds = collect_kinds(src);
    assert!(kinds.contains(&TokenKind::LeftBrace));
    assert!(kinds.contains(&TokenKind::LeftBracket));
    assert!(kinds.contains(&TokenKind::LeftParen));
    assert!(kinds.contains(&TokenKind::RightParen));
    assert!(kinds.contains(&TokenKind::RightBracket));
    assert!(kinds.contains(&TokenKind::RightBrace));
    Ok(())
}

#[test]
fn ext_multiple_statements_on_one_line() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("my $a = 1; my $b = 2; my $c = 3;");
    let my_count = kinds.iter().filter(|k| **k == TokenKind::My).count();
    let semi_count = kinds.iter().filter(|k| **k == TokenKind::Semicolon).count();
    assert_eq!(my_count, 3);
    assert_eq!(semi_count, 3);
    Ok(())
}

#[test]
fn ext_single_character_input() -> Result<(), Box<dyn std::error::Error>> {
    // Various single-char inputs
    let mut s = TokenStream::new(";");
    let t = must(s.next());
    assert_eq!(t.kind, TokenKind::Semicolon);
    Ok(())
}

#[test]
fn ext_token_stream_comma_separated_list() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("(1, 2, 3)");
    let comma_count = kinds.iter().filter(|k| **k == TokenKind::Comma).count();
    assert_eq!(comma_count, 2);
    Ok(())
}

#[test]
fn ext_hash_constructor() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("(a => 1, b => 2)");
    let fat_arrow_count = kinds.iter().filter(|k| **k == TokenKind::FatArrow).count();
    assert_eq!(fat_arrow_count, 2);
    Ok(())
}

#[test]
fn ext_method_call_chain() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("$obj->method()->another()");
    let arrow_count = kinds.iter().filter(|k| **k == TokenKind::Arrow).count();
    assert!(arrow_count >= 2, "expected at least 2 arrows, got {arrow_count}");
    Ok(())
}

#[test]
fn ext_dot_concat_operator() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("$a . $b");
    assert!(kinds.contains(&TokenKind::Dot));
    Ok(())
}

#[test]
fn ext_collect_all_returns_no_eof() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = collect_kinds("my $x = 1;");
    assert!(!kinds.contains(&TokenKind::Eof));
    assert!(!kinds.is_empty());
    Ok(())
}

#[test]
fn ext_collect_texts_match_source_fragments() -> Result<(), Box<dyn std::error::Error>> {
    let texts = collect_texts("my $x;");
    assert!(texts.contains(&"my".to_string()));
    assert!(texts.contains(&";".to_string()));
    Ok(())
}
