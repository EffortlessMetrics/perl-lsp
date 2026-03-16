#![allow(dead_code)]

use perl_parser_core::{NodeKind, Parser};
use perl_tdd_support::must;

/// Parse the given source and return the top-level AST node.
/// Panics (via `must`) if the parser returns Err.
pub fn parse(source: &str) -> perl_parser_core::Node {
    let mut parser = Parser::new(source);
    must(parser.parse())
}

/// Assert that a parsed AST has no Error / Missing* nodes anywhere in the
/// S-expression representation. This is a conservative "clean parse" check.
pub fn assert_clean_parse(source: &str) {
    let ast = parse(source);
    let sexp = ast.to_sexp();

    // Check for error sentinels in the sexp output.
    // We look for the node-kind markers, not arbitrary substrings.
    let error_markers = [
        "(error ",
        "(Error ",
        "(missing_expression",
        "(missing_statement",
        "(missing_identifier",
        "(missing_block",
        "MissingExpression",
        "MissingStatement",
        "MissingIdentifier",
        "MissingBlock",
    ];

    for marker in &error_markers {
        assert!(
            !sexp.contains(marker),
            "Clean-parse assertion failed: found '{}' in sexp for source:\n{}\n\nsexp:\n{}",
            marker,
            source,
            sexp,
        );
    }
}

/// Extract top-level statement kinds from a Program node.
pub fn top_level_kinds(ast: &perl_parser_core::Node) -> Vec<&str> {
    match &ast.kind {
        NodeKind::Program { statements } => statements.iter().map(|s| s.kind.kind_name()).collect(),
        _ => vec![ast.kind.kind_name()],
    }
}
