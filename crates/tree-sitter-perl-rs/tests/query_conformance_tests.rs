#![cfg(feature = "queries")]

use std::error::Error;

use perl_tdd_support::must_some;
use tree_sitter_perl_rs::{Parser, Query, QueryCursor};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

#[test]
fn executes_a_supported_fragment_from_the_upstream_highlights_query() -> TestResult {
    let upstream = include_str!("../../../tree-sitter-perl/queries/highlights.scm");
    let fragment = "(number)";
    if !upstream.contains(fragment) {
        return Err("upstream highlights query no longer contains the conformance fragment".into());
    }

    let source = "my $value = 42;\n";
    let mut parser = Parser::new();
    let tree = must_some(parser.parse(source));
    let query_source = format!("{fragment} @number");
    let query = Query::new(&query_source)?;
    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node()).collect::<Vec<_>>();

    let query_match = must_some(matches.first());
    let capture = must_some(query_match.captures().first());
    assert_eq!(capture.name(), "number");
    assert_eq!(capture.node().kind(), "number");
    assert_eq!(capture.node().utf8_text(source.as_bytes())?, "42");
    Ok(())
}
