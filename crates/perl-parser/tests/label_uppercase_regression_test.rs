//! Regression tests for uppercase label parsing.
//!
//! Perl convention uses uppercase labels (`OUTER:`, `LINE:`, `LOOP:`).
//! The parser correctly recognises these as `LabeledStatement` nodes
//! because `::` tokenizes as `DoubleColon`, making single-colon
//! `IDENTIFIER:` unambiguously a label.

use perl_parser::Parser;

mod nodekind_helpers;
use nodekind_helpers::has_node_kind;

/// Uppercase labels (OUTER:, LOOP:, LINE:) are idiomatic Perl and are now
/// correctly recognised as `LabeledStatement` by the parser.
#[test]
fn test_uppercase_label_parsed() -> Result<(), Box<dyn std::error::Error>> {
    let source = "OUTER: while (1) { last OUTER; }";

    let mut parser = Parser::new(source);
    let ast = parser.parse()?;

    assert!(parser.errors().is_empty(), "uppercase label should parse without errors");
    assert!(
        has_node_kind(&ast, "LabeledStatement"),
        "Expected LabeledStatement for `OUTER:` label"
    );
    assert!(has_node_kind(&ast, "LoopControl"), "Expected LoopControl for `last OUTER`");

    Ok(())
}
