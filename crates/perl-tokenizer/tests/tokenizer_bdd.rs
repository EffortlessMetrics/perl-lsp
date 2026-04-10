use perl_tdd_support::must;
use perl_tokenizer::TokenKind;
use perl_tokenizer::token_stream::TokenStream;
use perl_tokenizer::trivia::Trivia;
use perl_tokenizer::trivia_parser::TriviaPreservingParser;

fn collect_non_eof_kinds(source: &str) -> Vec<TokenKind> {
    let mut stream = TokenStream::new(source);
    let mut kinds = Vec::new();
    loop {
        let token = must(stream.next());
        if token.kind == TokenKind::Eof {
            break;
        }
        kinds.push(token.kind);
    }
    kinds
}

#[test]
fn given_a_variable_declaration_when_tokenized_then_keywords_and_operators_are_classified() {
    let kinds = collect_non_eof_kinds("my $value = 42;");

    assert_eq!(
        kinds,
        vec![
            TokenKind::My,
            TokenKind::Identifier,
            TokenKind::Assign,
            TokenKind::Number,
            TokenKind::Semicolon
        ]
    );
}

#[test]
fn given_statement_boundary_when_reset_then_next_statement_is_retokenized() {
    let mut stream = TokenStream::new("my $x = 1; our $y = 2;");

    loop {
        let token = must(stream.next());
        if token.kind == TokenKind::Semicolon {
            break;
        }
    }

    stream.on_stmt_boundary();

    assert_eq!(must(stream.peek()).kind, TokenKind::Our);
}

#[test]
fn given_prelexed_tokens_when_streamed_then_eof_is_synthesized() {
    let tokens = vec![
        perl_tokenizer::Token::new(TokenKind::My, "my", 0, 2),
        perl_tokenizer::Token::new(TokenKind::Identifier, "$x", 3, 5),
    ];

    let mut stream = TokenStream::from_vec(tokens);

    assert_eq!(must(stream.next()).kind, TokenKind::My);
    assert_eq!(must(stream.next()).kind, TokenKind::Identifier);
    assert_eq!(must(stream.next()).kind, TokenKind::Eof);
    assert_eq!(must(stream.next()).kind, TokenKind::Eof);
}

#[test]
fn given_source_with_comments_when_parsed_with_trivia_then_comment_is_preserved() {
    let parser = TriviaPreservingParser::new("# lead\nmy $x = 1;\n".to_string());
    let parsed = parser.parse();

    let found_comment = parsed
        .leading_trivia
        .iter()
        .any(|t| matches!(&t.trivia, Trivia::LineComment(text) if text == "# lead"));

    assert!(found_comment);
}

#[test]
fn given_source_with_pod_when_parsed_with_trivia_then_pod_is_preserved() {
    let source = "=head1 NAME\n\nDemo\n\n=cut\n\nmy $x = 1;\n".to_string();
    let parser = TriviaPreservingParser::new(source);
    let parsed = parser.parse();

    let found_pod = parsed
        .leading_trivia
        .iter()
        .any(|t| matches!(&t.trivia, Trivia::PodComment(text) if text.contains("=head1 NAME") && text.contains("=cut")));

    assert!(found_pod);
}

#[test]
fn given_windows_newlines_when_parsed_with_trivia_then_newline_trivia_is_retained() {
    let parser = TriviaPreservingParser::new("\r\nmy $x = 1;\r\n".to_string());
    let parsed = parser.parse();

    let newline_count =
        parsed.leading_trivia.iter().filter(|t| matches!(t.trivia, Trivia::Newline)).count();

    assert!(newline_count >= 1);
}
