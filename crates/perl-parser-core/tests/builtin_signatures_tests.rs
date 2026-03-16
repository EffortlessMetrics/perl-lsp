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
fn builtin_signatures_contains_common_functions() -> Result<(), Box<dyn std::error::Error>> {
    let sigs = builtin_signatures::create_builtin_signatures();
    // Some well-known Perl builtins should be present
    assert!(sigs.contains_key("print"), "missing 'print'");
    assert!(sigs.contains_key("push"), "missing 'push'");
    assert!(sigs.contains_key("pop"), "missing 'pop'");
    assert!(sigs.contains_key("chomp"), "missing 'chomp'");
    assert!(sigs.contains_key("open"), "missing 'open'");
    assert!(sigs.contains_key("close"), "missing 'close'");
    Ok(())
}

#[test]
fn builtin_signatures_phf_contains_common_functions() -> Result<(), Box<dyn std::error::Error>> {
    let phf = &builtin_signatures_phf::BUILTIN_SIGS;
    assert!(phf.contains_key("print"), "missing 'print' in phf");
    assert!(phf.contains_key("push"), "missing 'push' in phf");
    assert!(phf.contains_key("chomp"), "missing 'chomp' in phf");
    Ok(())
}

#[test]
fn builtin_signatures_not_empty() -> Result<(), Box<dyn std::error::Error>> {
    let sigs = builtin_signatures::create_builtin_signatures();
    assert!(sigs.len() > 50, "expected >50 builtins, got {}", sigs.len());
    Ok(())
}

#[test]
fn phf_map_not_empty() -> Result<(), Box<dyn std::error::Error>> {
    let phf = &builtin_signatures_phf::BUILTIN_SIGS;
    assert!(!phf.is_empty());
    Ok(())
}
