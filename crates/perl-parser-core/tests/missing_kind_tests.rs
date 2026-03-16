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
    let variants = vec![
        MissingKind::Expression,
        MissingKind::Statement,
        MissingKind::Identifier,
        MissingKind::Block,
        MissingKind::ClosingDelimiter(')'),
        MissingKind::ClosingDelimiter('}'),
        MissingKind::ClosingDelimiter(']'),
        MissingKind::Semicolon,
        MissingKind::Condition,
        MissingKind::Argument,
        MissingKind::Operator,
    ];
    // Each is distinct
    for (i, a) in variants.iter().enumerate() {
        for (j, b) in variants.iter().enumerate() {
            if i != j {
                assert_ne!(a, b);
            }
        }
    }
    Ok(())
}

#[test]
fn debug_format() -> Result<(), Box<dyn std::error::Error>> {
    let kind = MissingKind::Expression;
    let dbg = format!("{:?}", kind);
    assert!(dbg.contains("Expression"));
    Ok(())
}

#[test]
fn clone_and_copy() -> Result<(), Box<dyn std::error::Error>> {
    let kind = MissingKind::Semicolon;
    let cloned = kind;
    assert_eq!(kind, cloned);
    Ok(())
}

#[test]
fn closing_delimiter_variants() -> Result<(), Box<dyn std::error::Error>> {
    let paren = MissingKind::ClosingDelimiter(')');
    let brace = MissingKind::ClosingDelimiter('}');
    let bracket = MissingKind::ClosingDelimiter(']');
    assert_ne!(paren, brace);
    assert_ne!(brace, bracket);
    assert_ne!(paren, bracket);
    Ok(())
}
