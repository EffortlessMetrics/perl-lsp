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

    // Document the known gap: LabeledStatement is missing for uppercase labels.
    if !has_node_kind(&output.ast, "LabeledStatement") {
        eprintln!(
            "KNOWN GAP: uppercase label `OUTER:` does not produce LabeledStatement. \
             See is_label_start() in statements.rs."
        );
    }
}

/// This test should pass once `is_label_start()` is fixed to accept uppercase
/// identifiers followed by `:` (but not `::`, which is a package separator).
#[test]
#[ignore = "uppercase labels not yet parsed as LabeledStatement — fix is_label_start()"]
fn test_uppercase_label_fixed() {
    let source = "OUTER: while (1) { last OUTER; }";

    let mut parser = Parser::new(source);
    let result = parser.parse();
    let ast = result.expect("uppercase label should parse cleanly once fixed");

    assert!(
        has_node_kind(&ast, "LabeledStatement"),
        "Expected LabeledStatement for `OUTER:` label"
    );
    assert!(has_node_kind(&ast, "LoopControl"), "Expected LoopControl for `last OUTER`");
}
