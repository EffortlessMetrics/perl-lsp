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
fn on_stmt_boundary_resets_peek() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("my $x; my $y;");
    // Prime the peek
    let _ = stream.peek();
    // on_stmt_boundary should invalidate cached peeks
    stream.on_stmt_boundary();
    // After reset, peek should still work
    if let Ok(token) = stream.peek() {
        // We should get a valid token (the reparsed first token)
        let _ = format!("{:?}", token.kind);
    }
    Ok(())
}

#[test]
fn invalidate_peek_clears_cache() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("1 + 2");
    let _ = stream.peek();
    let _ = stream.peek_second();
    // Invalidate all cached peeks
    stream.invalidate_peek();
    // Should still work after invalidation
    if let Ok(token) = stream.peek() {
        let _ = format!("{:?}", token.kind);
    }
    Ok(())
}

#[test]
fn peek_fresh_kind_on_empty() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("");
    let kind = stream.peek_fresh_kind();
    // Should return Some(Eof) or similar
    assert!(kind.is_some());
    Ok(())
}

#[test]
fn peek_fresh_kind_on_valid_input() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("my $x;");
    let kind = stream.peek_fresh_kind();
    assert!(kind.is_some());
    Ok(())
}

#[test]
fn enter_format_mode_does_not_crash() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("format STDOUT =\nsome text\n.\n");
    stream.enter_format_mode();
    // Should still be able to get tokens
    if let Ok(token) = stream.peek() {
        let _ = format!("{:?}", token.kind);
    }
    Ok(())
}
