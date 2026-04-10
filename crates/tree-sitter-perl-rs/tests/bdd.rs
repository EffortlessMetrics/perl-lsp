//! Behavior-driven integration tests for the `tree-sitter-perl-rs` facade.
//!
//! These scenarios capture user-visible guarantees in Given/When/Then form so
//! regressions are easy to reason about during refactors.

use perl_tdd_support::must_some;
use tree_sitter_perl_rs::Parser;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn given_valid_perl_when_parsing_then_root_is_program_and_sexp_is_source_file() -> TestResult {
    let source = "my $x = 42;";
    let mut parser = Parser::new();

    let tree = must_some(parser.parse(source));
    let root = tree.root_node();

    assert_eq!(root.kind(), "Program");
    assert!(
        root.to_sexp().starts_with("(source_file"),
        "expected tree-sitter-compatible source_file root in S-expression"
    );

    Ok(())
}

#[test]
fn given_syntax_errors_when_parsing_then_partial_tree_is_still_returned() -> TestResult {
    let source = "sub demo { my $x = ; if ( {";
    let mut parser = Parser::new();

    let tree = parser.parse(source);

    assert!(
        tree.is_some(),
        "error-tolerant parser should return a partial tree for malformed input"
    );

    Ok(())
}

#[test]
fn given_multiple_statements_when_iterating_children_then_count_matches_iterator_length()
-> TestResult {
    let source = "my $x = 1; my $y = 2; my $z = 3;";
    let mut parser = Parser::new();

    let tree = must_some(parser.parse(source));
    let root = tree.root_node();
    let child_count = root.child_count();
    let iterated_count = root.children().count();

    assert_eq!(iterated_count, child_count);

    Ok(())
}

#[test]
fn given_tree_and_source_when_requesting_utf8_text_then_original_source_is_returned() -> TestResult
{
    let source = "my $x = 'café';";
    let mut parser = Parser::new();

    let tree = must_some(parser.parse(source));
    let extracted = tree.root_node().utf8_text(source.as_bytes())?;

    assert_eq!(extracted, source);

    Ok(())
}

#[test]
fn given_tree_when_reading_out_of_range_child_then_none_is_returned() -> TestResult {
    let source = "my $x = 1;";
    let mut parser = Parser::new();

    let tree = must_some(parser.parse(source));
    let root = tree.root_node();

    assert!(root.child(usize::MAX).is_none());

    Ok(())
}

#[test]
fn given_parser_default_when_parsing_simple_source_then_tree_is_created() -> TestResult {
    let mut parser = Parser::default();

    let tree = parser.parse("1;");

    assert!(tree.is_some(), "default parser should parse simple source");

    Ok(())
}
