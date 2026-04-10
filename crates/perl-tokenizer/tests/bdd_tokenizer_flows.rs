use perl_ast_v2::NodeKind;
use perl_lexer::PerlLexer;
use perl_tdd_support::must;
use perl_tokenizer::trivia_parser::TriviaPreservingParser;
use perl_tokenizer::util::code_slice;
use perl_tokenizer::{Token, TokenKind, TokenStream};

#[test]
fn given_trivia_heavy_source_when_iterating_token_stream_then_only_semantic_tokens_are_returned()
-> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("  # lead comment\nmy $x = 42;\n");

    let sequence = [
        TokenKind::My,
        TokenKind::Identifier,
        TokenKind::Assign,
        TokenKind::Number,
        TokenKind::Semicolon,
        TokenKind::Eof,
    ];

    for expected in sequence {
        let token = must(stream.next());
        assert_eq!(token.kind, expected);
    }

    Ok(())
}

#[test]
fn given_prelexed_tokens_when_stream_runs_out_then_eof_is_synthesized_and_sticky()
-> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::from_vec(vec![
        Token::new(TokenKind::My, "my", 0, 2),
        Token::new(TokenKind::Identifier, "$name", 3, 8),
    ]);

    assert_eq!(must(stream.next()).kind, TokenKind::My);
    assert_eq!(must(stream.next()).kind, TokenKind::Identifier);
    assert_eq!(must(stream.next()).kind, TokenKind::Eof);
    assert_eq!(must(stream.peek()).kind, TokenKind::Eof);

    Ok(())
}

#[test]
fn given_raw_lexer_tokens_when_converting_for_parser_then_trivia_tokens_are_filtered_out() {
    let mut lexer = PerlLexer::new("my $x = 1; # trailing\n");
    let mut raw = Vec::new();

    while let Some(token) = lexer.next_token() {
        raw.push(token.clone());
        if matches!(token.token_type, perl_lexer::TokenType::EOF) {
            break;
        }
    }

    let parser_tokens = TokenStream::lexer_tokens_to_parser_tokens(raw);
    let kinds: Vec<TokenKind> = parser_tokens.into_iter().map(|token| token.kind).collect();

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
fn given_variable_declarations_with_comments_when_using_trivia_parser_then_program_node_has_statements_and_trivia()
-> Result<(), Box<dyn std::error::Error>> {
    let source = "# lead\nmy $x = 1;\n# mid\nour $y = 2;\n".to_string();
    let parsed = TriviaPreservingParser::new(source).parse();

    match &parsed.node.kind {
        NodeKind::Program { statements } => assert_eq!(statements.len(), 2),
        _ => return Err("expected top-level program node".into()),
    }

    assert!(
        parsed
            .leading_trivia
            .iter()
            .any(|t| matches!(t.trivia, perl_tokenizer::Trivia::LineComment(_)))
    );

    Ok(())
}

#[test]
fn given_data_marker_when_slicing_executable_code_then_trailing_data_section_is_removed() {
    let source = "print qq/ok/;\n__DATA__\nname\nvalue\n";
    let code = code_slice(source);

    assert_eq!(code, "print qq/ok/;\n");
}
