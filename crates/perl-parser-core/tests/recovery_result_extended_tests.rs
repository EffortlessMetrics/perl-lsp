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
fn recovered_with_count() -> Result<(), Box<dyn std::error::Error>> {
    let r = RecoveryResult::Recovered(5);
    if let RecoveryResult::Recovered(n) = r {
        assert_eq!(n, 5);
    } else {
        return Err("expected Recovered".into());
    }
    Ok(())
}

#[test]
fn all_variants_debug() -> Result<(), Box<dyn std::error::Error>> {
    let variants = vec![
        RecoveryResult::Recovered(0),
        RecoveryResult::AtSyncPoint,
        RecoveryResult::BudgetExhausted,
        RecoveryResult::ReachedEof,
    ];
    for v in &variants {
        let dbg = format!("{:?}", v);
        assert!(!dbg.is_empty());
    }
    Ok(())
}
