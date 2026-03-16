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
fn all_variants_exist() -> Result<(), Box<dyn std::error::Error>> {
    let variants =
        [SyncPoint::Semicolon, SyncPoint::CloseBrace, SyncPoint::Keyword, SyncPoint::Eof];
    // All are distinct
    for i in 0..variants.len() {
        for j in (i + 1)..variants.len() {
            assert_ne!(variants[i], variants[j]);
        }
    }
    Ok(())
}

#[test]
fn debug_format() -> Result<(), Box<dyn std::error::Error>> {
    assert!(format!("{:?}", SyncPoint::Semicolon).contains("Semicolon"));
    assert!(format!("{:?}", SyncPoint::CloseBrace).contains("CloseBrace"));
    assert!(format!("{:?}", SyncPoint::Keyword).contains("Keyword"));
    assert!(format!("{:?}", SyncPoint::Eof).contains("Eof"));
    Ok(())
}
