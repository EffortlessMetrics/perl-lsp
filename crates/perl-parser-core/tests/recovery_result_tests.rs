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
fn recovery_result_variants_distinct() -> Result<(), Box<dyn std::error::Error>> {
    let recovered = RecoveryResult::Recovered(3);
    let at_sync = RecoveryResult::AtSyncPoint;
    let exhausted = RecoveryResult::BudgetExhausted;
    let eof = RecoveryResult::ReachedEof;

    assert_ne!(recovered, at_sync);
    assert_ne!(at_sync, exhausted);
    assert_ne!(exhausted, eof);
    assert_ne!(eof, recovered);
    Ok(())
}

#[test]
fn recovery_result_clone_eq() -> Result<(), Box<dyn std::error::Error>> {
    let original = RecoveryResult::Recovered(5);
    let cloned = original;
    assert_eq!(original, cloned);
    Ok(())
}
