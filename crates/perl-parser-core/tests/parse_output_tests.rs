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
fn success_output_has_no_diagnostics() -> Result<(), Box<dyn std::error::Error>> {
    let ast = V1Node::new(
        V1NodeKind::Program { statements: vec![] },
        SourceLocation { start: 0, end: 0 },
    );
    let output = ParseOutput::success(ast);
    assert!(output.diagnostics.is_empty());
    assert!(!output.terminated_early);
    assert_eq!(output.budget_usage.errors_emitted, 0);
    Ok(())
}

#[test]
fn with_errors_output_tracks_diagnostics() -> Result<(), Box<dyn std::error::Error>> {
    let ast = V1Node::new(
        V1NodeKind::Program { statements: vec![] },
        SourceLocation { start: 0, end: 0 },
    );
    let diags = vec![CatastrophicParseError::UnexpectedEof];
    let output = ParseOutput::with_errors(ast, diags);
    assert_eq!(output.diagnostics.len(), 1);
    assert_eq!(output.budget_usage.errors_emitted, 1);
    Ok(())
}
