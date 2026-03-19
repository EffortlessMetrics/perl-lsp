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

/// Error markers used by both `assert_clean_parse` and `assert_has_error`.
const ERROR_MARKERS: &[&str] = &[
    "(error ",
    "(Error ",
    "(ERROR ",
    "(missing_expression",
    "(missing_statement",
    "(missing_identifier",
    "(missing_block",
    "MissingExpression",
    "MissingStatement",
    "MissingIdentifier",
    "MissingBlock",
];

/// Assert that a parsed AST contains at least one Error or Missing* node
/// whose sexp representation contains the given `needle` substring.
///
/// This is the inverse of `assert_clean_parse` — it verifies that the parser
/// correctly reports an error for malformed input.
pub fn assert_has_error(source: &str, needle: &str) {
    let ast = parse(source);
    let sexp = ast.to_sexp();
    let sexp_lower = sexp.to_lowercase();
    let needle_lower = needle.to_lowercase();

    // First verify there IS an error node somewhere.
    let has_any_error = ERROR_MARKERS.iter().any(|marker| sexp.contains(marker));
    assert!(has_any_error, "Expected an error node for source:\n{}\n\nsexp:\n{}", source, sexp,);

    // Then verify the needle appears (case-insensitive) in the sexp.
    assert!(
        sexp_lower.contains(&needle_lower),
        "Expected error containing '{}' for source:\n{}\n\nsexp:\n{}",
        needle,
        source,
        sexp,
    );
}

/// Extract top-level statement kinds from a Program node.
pub fn top_level_kinds(ast: &perl_parser_core::Node) -> Vec<&str> {
    match &ast.kind {
        NodeKind::Program { statements } => statements.iter().map(|s| s.kind.kind_name()).collect(),
        _ => vec![ast.kind.kind_name()],
    }
}
