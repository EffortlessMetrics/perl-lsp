use perl_parser_core::position::Range;
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
fn builder_chain_all_methods() -> Result<(), Box<dyn std::error::Error>> {
    let range = Range::new(
        perl_parser_core::position::Position::new(0, 1, 1),
        perl_parser_core::position::Position::new(5, 1, 6),
    );
    let err = RecoveryParseError::new("test error".to_string(), range)
        .with_expected(vec!["semicolon".to_string(), "brace".to_string()])
        .with_found("comma".to_string())
        .with_hint("try adding a semicolon".to_string());

    assert_eq!(err.message, "test error");
    assert_eq!(err.expected.len(), 2);
    assert_eq!(err.found, "comma");
    assert_eq!(err.recovery_hint.as_deref().unwrap_or(""), "try adding a semicolon");
    Ok(())
}

#[test]
fn new_sets_defaults() -> Result<(), Box<dyn std::error::Error>> {
    let range = Range::new(
        perl_parser_core::position::Position::new(0, 1, 1),
        perl_parser_core::position::Position::new(0, 1, 1),
    );
    let err = RecoveryParseError::new("msg".to_string(), range);
    assert!(err.expected.is_empty());
    assert!(err.found.is_empty());
    assert!(err.recovery_hint.is_none());
    Ok(())
}
