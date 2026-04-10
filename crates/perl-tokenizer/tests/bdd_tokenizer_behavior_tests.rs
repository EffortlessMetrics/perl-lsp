//! BDD-style behavior tests for `perl-tokenizer`.
//!
//! These scenarios focus on end-user parser-facing behavior rather than
//! implementation details.

use perl_lexer::{PerlLexer, TokenType};
use perl_tokenizer::token_stream::TokenStream;
use perl_tokenizer::{Token, TokenKind};

fn collect_kinds(stream: &mut TokenStream<'_>) -> Vec<TokenKind> {
    let mut kinds = Vec::new();
    while let Ok(token) = stream.next() {
        kinds.push(token.kind);
        if token.kind == TokenKind::Eof {
            break;
        }
    }
    kinds
}

#[test]
fn bdd_given_trivia_heavy_source_when_tokenized_then_only_syntactic_tokens_remain() {
    // Given: source containing comments, spaces, and newlines around a statement.
    let mut stream = TokenStream::new("  # pre-comment\n\tmy $x = 42;  # trailing\n");

    // When: parser-facing tokens are consumed from the stream.
    let kinds = collect_kinds(&mut stream);

    // Then: trivia is skipped and only statement tokens are visible.
    assert_eq!(
        kinds,
        vec![
            TokenKind::My,
            TokenKind::Identifier,
            TokenKind::Assign,
            TokenKind::Number,
            TokenKind::Semicolon,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn bdd_given_raw_lexer_output_when_converted_then_trivia_is_filtered_and_keywords_are_mapped() {
    // Given: raw lexer tokens including whitespace/comment trivia.
    let mut lexer = PerlLexer::new("my $x = 1; # comment\n");
    let mut raw_tokens = Vec::new();
    while let Some(token) = lexer.next_token() {
        if matches!(token.token_type, TokenType::EOF) {
            break;
        }
        raw_tokens.push(token);
    }

    // When: raw tokens are converted into parser tokens.
    let parser_tokens = TokenStream::lexer_tokens_to_parser_tokens(raw_tokens);

    // Then: only semantic tokens remain with parser-facing kinds.
    let kinds: Vec<TokenKind> = parser_tokens.iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::My,
            TokenKind::Identifier,
            TokenKind::Assign,
            TokenKind::Number,
            TokenKind::Semicolon,
        ]
    );
}

#[test]
fn bdd_given_buffered_tokens_when_cache_is_cleared_then_stream_continues_from_current_cursor() {
    // Given: a pre-lexed/buffered token stream.
    let tokens = vec![
        Token::new(TokenKind::My, "my", 0, 2),
        Token::new(TokenKind::Identifier, "$x", 3, 5),
        Token::new(TokenKind::Semicolon, ";", 5, 6),
    ];
    let mut stream = TokenStream::from_vec(tokens);

    // Prime the peek cache, then execute parser boundary actions.
    assert_eq!(stream.peek().expect("peek should succeed").kind, TokenKind::My);

    // When: parser lifecycle hooks are executed in buffered mode.
    stream.on_stmt_boundary();
    stream.relex_as_term();
    stream.enter_format_mode();

    // Then: cache invalidation does not rewind buffered input; consumption continues from the current cursor.
    let kinds = collect_kinds(&mut stream);
    assert_eq!(
        kinds,
        vec![TokenKind::Identifier, TokenKind::Semicolon, TokenKind::Eof]
    );
}

#[test]
fn bdd_given_no_explicit_eof_token_when_buffer_is_exhausted_then_stream_synthesizes_sticky_eof() {
    // Given: a pre-lexed stream without an explicit EOF token.
    let tokens = vec![Token::new(TokenKind::Number, "123", 0, 3)];
    let mut stream = TokenStream::from_vec(tokens);

    // When: all tokens are consumed and we keep reading.
    assert_eq!(stream.next().expect("number token").kind, TokenKind::Number);
    assert_eq!(stream.next().expect("first eof").kind, TokenKind::Eof);
    assert_eq!(stream.next().expect("sticky eof").kind, TokenKind::Eof);

    // Then: EOF remains stable for subsequent parser checks.
    assert!(stream.is_eof());
}
