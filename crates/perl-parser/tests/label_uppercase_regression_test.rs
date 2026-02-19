//! Regression tests for uppercase label parsing.
//!
//! Perl convention uses uppercase labels (`OUTER:`, `LINE:`, `LOOP:`), but the
//! parser's `is_label_start()` heuristic currently rejects uppercase identifiers.
//! These tests document the current behaviour and gate the fix.

use perl_parser::Parser;

mod nodekind_helpers;
use nodekind_helpers::has_node_kind;

/// Current behaviour: uppercase label is NOT recognised as `LabeledStatement`,
/// but LoopControl's `last OUTER` still works through recovery.
#[test]
fn test_uppercase_label_current_behavior() {
    let source = "OUTER: while (1) { last OUTER; }";

    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();

    // LoopControl should exist regardless
    assert!(
        has_node_kind(&output.ast, "LoopControl"),
        "Expected LoopControl node for `last OUTER`"
    );

    // Hard assertion: uppercase labels do NOT produce LabeledStatement (known gap).
    // This will fail loudly if the parser starts handling them, signalling the
    // ignored test below can be un-ignored.
    assert!(
        !has_node_kind(&output.ast, "LabeledStatement"),
        "LabeledStatement should NOT appear for uppercase labels (known gap). \
         If this fires, un-ignore test_uppercase_label_fixed."
    );
}

/// This test should pass once `is_label_start()` is fixed to accept uppercase
/// identifiers followed by `:` (but not `::`, which is a package separator).
#[test]
#[ignore = "uppercase labels not yet parsed as LabeledStatement — fix is_label_start()"]
fn test_uppercase_label_fixed() -> Result<(), Box<dyn std::error::Error>> {
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
