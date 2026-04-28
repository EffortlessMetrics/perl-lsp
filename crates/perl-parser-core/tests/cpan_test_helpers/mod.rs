#![allow(dead_code)]

use perl_parser_core::{Node, NodeKind, Parser};
use perl_tdd_support::must;

/// Parse the given source and return the top-level AST node.
/// Panics (via `must`) if the parser returns Err.
pub fn parse(source: &str) -> perl_parser_core::Node {
    let mut parser = Parser::new(source);
    must(parser.parse())
}

/// Parse the given source and return both the AST and any parser diagnostics.
fn parse_with_diagnostics(source: &str) -> (Node, String, bool) {
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    let diagnostics = format!("{:?}", parser.get_errors());
    let has_diagnostics = !parser.get_errors().is_empty();
    (ast, diagnostics, has_diagnostics)
}

/// Walk the AST recursively and return the kind_name of the first error or
/// missing node found, or `None` if the tree is clean.
fn find_first_error(node: &Node) -> Option<&'static str> {
    match &node.kind {
        NodeKind::Error { .. }
        | NodeKind::MissingExpression
        | NodeKind::MissingStatement
        | NodeKind::MissingIdentifier
        | NodeKind::MissingBlock => return Some(node.kind.kind_name()),
        _ => {}
    }
    for child in node.children() {
        if let Some(name) = find_first_error(child) {
            return Some(name);
        }
    }
    None
}

/// Assert that a parsed AST has no Error / Missing* nodes anywhere in the
/// tree. Uses AST node walking rather than sexp string matching to avoid
/// false-positives on valid Perl that contains "ERROR" as an identifier.
pub fn assert_clean_parse(source: &str) {
    let ast = parse(source);
    let error_kind = find_first_error(&ast);
    let sexp = ast.to_sexp();
    assert!(
        error_kind.is_none(),
        "Clean-parse assertion failed: found '{}' node in AST for source:\n{}\n\nsexp:\n{}",
        error_kind.unwrap_or(""),
        source,
        sexp,
    );
}

/// Assert that a parsed AST contains at least one Error or Missing* node
/// whose sexp representation contains the given `needle` substring.
///
/// This is the inverse of `assert_clean_parse` — it verifies that the parser
/// correctly reports an error for malformed input.
pub fn assert_has_error(source: &str, needle: &str) {
    let (ast, diagnostics, has_diagnostics) = parse_with_diagnostics(source);
    let sexp = ast.to_sexp();
    let sexp_lower = sexp.to_lowercase();
    let needle_lower = needle.to_lowercase();
    let diagnostics_lower = diagnostics.to_lowercase();

    // First verify there IS an error signal either in AST error nodes or
    // parser diagnostics from recovery paths (e.g., inserted closers).
    let has_any_error = find_first_error(&ast).is_some();
    assert!(
        has_any_error || has_diagnostics,
        "Expected an error signal for source:\n{}\n\nsexp:\n{}\n\ndiagnostics:\n{}",
        source,
        sexp,
        diagnostics,
    );

    // Then verify the needle appears (case-insensitive) in either AST output
    // or parser diagnostics.
    assert!(
        sexp_lower.contains(&needle_lower) || diagnostics_lower.contains(&needle_lower),
        "Expected error containing '{}' for source:\n{}\n\nsexp:\n{}\n\ndiagnostics:\n{}",
        needle,
        source,
        sexp,
        diagnostics,
    );
}

/// Extract top-level statement kinds from a Program node.
pub fn top_level_kinds(ast: &perl_parser_core::Node) -> Vec<&str> {
    match &ast.kind {
        NodeKind::Program { statements } => statements.iter().map(|s| s.kind.kind_name()).collect(),
        _ => vec![ast.kind.kind_name()],
    }
}
