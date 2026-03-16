use perl_parser_core::{
    BudgetTracker,
    LineEnding,
    // AST (v1) types used by Parser
    Node as V1Node,
    NodeKind as V1NodeKind,
    ParseBudget,
    // Error types and recovery
    ParseError as CatastrophicParseError,
    ParseOutput,
    // Parser
    Parser,
    // Position mapping
    PositionMapper,
    SourceLocation,
    // AST (v2) types used by RecoveryParser
    ast_v2::MissingKind,
    ast_v2::NodeKind as V2NodeKind,
    builtin_signatures,
    builtin_signatures_phf,
    error::recovery_parser::RecoveryParser,
    error_recovery::{ParseError as RecoveryParseError, RecoveryResult, SyncPoint},
    line_index::LineIndex,
    // ParserContext
    parser_context::ParserContext,
    // Token stream
    token_stream::TokenStream,
    // Trivia
    trivia::{NodeWithTrivia, Trivia, TriviaPreservingParser, TriviaToken},
    trivia_parser::format_with_trivia,
};
use perl_tdd_support::{must, must_some};

#[test]
fn empty_stream_is_eof() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("");
    assert!(stream.is_eof());
    Ok(())
}

#[test]
fn peek_returns_token() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("42");
    let token = must(stream.peek());
    assert!(!token.text.is_empty());
    Ok(())
}

#[test]
fn next_consumes_token() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("42");
    let token = must(stream.next());
    assert!(!token.text.is_empty());
    assert!(stream.is_eof());
    Ok(())
}

#[test]
fn peek_second_looks_ahead() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("my $x");
    let _first = must(stream.peek());
    let second = stream.peek_second();
    assert!(second.is_ok(), "should be able to peek second token");
    Ok(())
}

#[test]
fn peek_third_looks_further() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("my $x = 42;");
    let third = stream.peek_third();
    assert!(third.is_ok(), "should be able to peek third token");
    Ok(())
}

#[test]
fn stream_processes_multiple_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("my $x = 42;");
    let mut count = 0;
    while !stream.is_eof() {
        let _tok = must(stream.next());
        count += 1;
        if count > 100 {
            return Err("infinite loop in token stream".into());
        }
    }
    assert!(count >= 4, "should have at least 4 tokens, got {}", count);
    Ok(())
}
