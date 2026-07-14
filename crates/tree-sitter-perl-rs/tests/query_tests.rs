#![cfg(feature = "queries")]

use std::error::Error;

use perl_tdd_support::must_some;
use tree_sitter_perl_rs::{Parser, Query, QueryCursor, QueryError};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn parse(source: &str) -> tree_sitter_perl_rs::Tree {
    let mut parser = Parser::new();
    must_some(parser.parse(source))
}

#[test]
fn matches_node_kinds_and_named_captures() -> TestResult {
    let source = "sub one { 1 }\nsub two { 2 }\n";
    let tree = parse(source);
    let query = Query::new("(sub) @definition")?;
    let mut cursor = QueryCursor::new();

    let matches = cursor.matches(&query, tree.root_node()).collect::<Vec<_>>();

    assert_eq!(matches.len(), 2);
    for query_match in matches {
        assert_eq!(query_match.pattern_index(), 0);
        let capture = must_some(query_match.captures().first());
        assert_eq!(capture.name(), "definition");
        assert_eq!(capture.node().kind(), "sub");
    }
    Ok(())
}

#[test]
fn matches_nested_children_with_named_fields() -> TestResult {
    let tree = parse("if ($x) { $y; }\n");
    let query = Query::new("(if condition: (variable) @condition then_branch: (block) @body)")?;
    let mut cursor = QueryCursor::new();

    let matches = cursor.matches(&query, tree.root_node()).collect::<Vec<_>>();

    assert_eq!(matches.len(), 1);
    let query_match = must_some(matches.first());
    assert_eq!(query_match.captures().len(), 2);
    assert_eq!(query_match.captures()[0].name(), "condition");
    assert_eq!(query_match.captures()[0].node().kind(), "variable");
    assert_eq!(query_match.captures()[1].name(), "body");
    assert_eq!(query_match.captures()[1].node().kind(), "block");
    Ok(())
}

#[test]
fn matches_wildcards_and_multiple_top_level_patterns() -> TestResult {
    let tree = parse("sub one { 1 }\nif ($x) { $x; }\n");
    let query = Query::new("(_) @any (sub)")?;
    assert_eq!(query.pattern_count(), 2);
    let mut cursor = QueryCursor::new();

    let matches = cursor.matches(&query, tree.root_node()).collect::<Vec<_>>();

    assert!(matches.iter().any(|query_match| query_match.pattern_index() == 0));
    assert!(matches.iter().any(|query_match| query_match.pattern_index() == 1));
    Ok(())
}

#[test]
fn restricts_matches_to_the_requested_byte_range() -> TestResult {
    let source = "sub one { 1 }\nsub two { 2 }\n";
    let tree = parse(source);
    let query = Query::new("(sub)")?;
    let second_start = source.find("sub two").ok_or("second subroutine missing")?;
    let mut cursor = QueryCursor::new();
    cursor.set_byte_range(second_start..source.len());

    let matches = cursor.matches(&query, tree.root_node()).collect::<Vec<_>>();

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].captures().len(), 0);
    Ok(())
}

#[test]
fn rejects_predicates_in_phase_2a() {
    let result = Query::new("(identifier) @name (#eq? @name \"foo\")");

    assert!(matches!(result, Err(QueryError::UnsupportedSyntax { .. })));
}

#[test]
fn sibling_patterns_are_ordered_and_distinct() -> TestResult {
    let tree = parse("sub one { 1 }\nif ($x) { $x; }\n");

    let ordered = Query::new("(source_file (sub) (if))")?;
    let mut cursor = QueryCursor::new();
    assert_eq!(cursor.matches(&ordered, tree.root_node()).count(), 1);

    let reversed = Query::new("(source_file (if) (sub))")?;
    assert_eq!(cursor.matches(&reversed, tree.root_node()).count(), 0);

    let reused = Query::new("(source_file (sub) (sub))")?;
    assert_eq!(cursor.matches(&reused, tree.root_node()).count(), 0);
    Ok(())
}

#[test]
fn rejects_unsupported_metacharacters() {
    assert!(matches!(Query::new("(sub|if)"), Err(QueryError::UnsupportedSyntax { .. })));
    assert!(matches!(Query::new("(sub*)"), Err(QueryError::UnsupportedSyntax { .. })));
}

#[test]
fn accepts_operator_derived_kind_names() -> TestResult {
    let tree = parse("1 + 2;\n");
    let query = Query::new("(binary_+) @operator")?;
    let mut cursor = QueryCursor::new();

    let matches = cursor.matches(&query, tree.root_node()).collect::<Vec<_>>();

    assert_eq!(matches.len(), 1);
    assert_eq!(must_some(matches[0].captures().first()).node().kind(), "binary_+");
    Ok(())
}
