/// Tests for issue #751 (Bug 1): Subroutine signature default values must accept
/// full scalar expressions (calls, binops, ternary), not just primary expressions.
///
/// Before the fix, `sub f($x = foo()) {}` produced an ERROR node because the
/// default-value parser only called `parse_primary`, which rejects function calls
/// and binary operators.
mod cpan_test_helpers;
use cpan_test_helpers::{assert_clean_parse, parse};

/// Helper: walk the AST and count all nodes whose kind_name matches `target`.
fn count_nodes_by_kind(node: &perl_parser_core::Node, target: &str) -> usize {
    let mut count = if node.kind.kind_name() == target { 1 } else { 0 };
    for child in node.children() {
        count += count_nodes_by_kind(child, target);
    }
    count
}

/// Helper: walk the AST and find the first node by kind_name.
fn find_node_by_kind<'a>(
    node: &'a perl_parser_core::Node,
    target: &str,
) -> Option<&'a perl_parser_core::Node> {
    if node.kind.kind_name() == target {
        return Some(node);
    }
    for child in node.children() {
        if let Some(found) = find_node_by_kind(child, target) {
            return Some(found);
        }
    }
    None
}

// ------------------------------------------------------------------
// Bug cases — these FAIL before the fix (default only parsed primary)
// ------------------------------------------------------------------

/// `sub f($x = foo()) {}` — default is a function call.
/// Issue #751, Bug 1: parse_primary rejects the call, producing an ERROR node.
#[test]
fn test_sig_default_function_call_parses_cleanly() {
    assert_clean_parse("sub f($x = foo()) {}");
}

/// `sub f($x = $y * 2) {}` — default is a binary expression.
#[test]
fn test_sig_default_binop_parses_cleanly() {
    assert_clean_parse("sub f($x = $y * 2) {}");
}

/// `sub f($x = $a ? $b : $c) {}` — default is a ternary expression.
#[test]
fn test_sig_default_ternary_parses_cleanly() {
    assert_clean_parse("sub f($x = $a ? $b : $c) {}");
}

// ------------------------------------------------------------------
// Comma-separation guard — full-expr parse must NOT over-consume commas
// ------------------------------------------------------------------

/// `sub f($x = 1, $y = 2) {}` — must produce TWO OptionalParameter nodes.
/// This verifies the fixed parse_ternary/parse_assignment level still stops
/// at the comma delimiter between signature parameters.
#[test]
fn test_sig_two_defaulted_params_produces_two_optional_params()
-> Result<(), Box<dyn std::error::Error>> {
    let source = "sub f($x = 1, $y = 2) {}";
    assert_clean_parse(source);

    let ast = parse(source);
    let count = count_nodes_by_kind(&ast, "OptionalParameter");
    assert_eq!(
        count,
        2,
        "Expected 2 OptionalParameter nodes for '($x = 1, $y = 2)', got {}; sexp:\n{}",
        count,
        ast.to_sexp()
    );
    Ok(())
}

/// The default node for a call should be a `call` (or similar) node,
/// not an ERROR node.
#[test]
fn test_sig_default_call_node_kind() -> Result<(), Box<dyn std::error::Error>> {
    let source = "sub f($x = foo()) {}";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    // There must be exactly zero ERROR nodes.
    assert!(
        !sexp.to_lowercase().contains("error"),
        "Expected no ERROR node in sexp for '{}', got:\n{}",
        source,
        sexp
    );
    // There must be at least one OptionalParameter.
    let count = count_nodes_by_kind(&ast, "OptionalParameter");
    assert!(
        count >= 1,
        "Expected at least one OptionalParameter in AST for '{}', got {}; sexp:\n{}",
        source,
        count,
        sexp
    );
    Ok(())
}

/// The default node for `$y * 2` should be a Binary expression node.
/// Note: signature content is not included in `to_sexp()` output, so we
/// use `find_node_by_kind` (which traverses via `children()`) instead.
#[test]
fn test_sig_default_binop_node_kind() -> Result<(), Box<dyn std::error::Error>> {
    let source = "sub f($x = $y * 2) {}";
    let ast = parse(source);
    let sexp = ast.to_sexp();
    // No ERROR nodes at all.
    assert!(
        !sexp.to_lowercase().contains("error"),
        "Expected no ERROR node in sexp for '{}', got:\n{}",
        source,
        sexp
    );
    // The AST should contain a Binary node (from $y * 2) inside the signature.
    let binary_node = find_node_by_kind(&ast, "Binary");
    assert!(
        binary_node.is_some(),
        "Expected a Binary node in AST for '{}'; sexp:\n{}",
        source,
        sexp
    );
    Ok(())
}

// ------------------------------------------------------------------
// Regression guards — simple cases must still parse as before
// ------------------------------------------------------------------

/// `sub f($x = 1) {}` — single optional param with literal default.
#[test]
fn test_sig_default_literal_regression() -> Result<(), Box<dyn std::error::Error>> {
    let source = "sub f($x = 1) {}";
    assert_clean_parse(source);
    let ast = parse(source);
    let count = count_nodes_by_kind(&ast, "OptionalParameter");
    assert_eq!(count, 1, "Expected 1 OptionalParameter for '($x = 1)', got {}", count);
    Ok(())
}

/// `sub f($x) {}` — required parameter (no default).
#[test]
fn test_sig_required_param_regression() -> Result<(), Box<dyn std::error::Error>> {
    let source = "sub f($x) {}";
    assert_clean_parse(source);
    let ast = parse(source);
    let mandatory = count_nodes_by_kind(&ast, "MandatoryParameter");
    let optional = count_nodes_by_kind(&ast, "OptionalParameter");
    assert_eq!(mandatory, 1, "Expected 1 MandatoryParameter for '($x)', got {}", mandatory);
    assert_eq!(optional, 0, "Expected 0 OptionalParameter for '($x)', got {}", optional);
    Ok(())
}

/// `sub f($x, @rest) {}` — required + slurpy parameter.
#[test]
fn test_sig_slurpy_param_regression() -> Result<(), Box<dyn std::error::Error>> {
    let source = "sub f($x, @rest) {}";
    assert_clean_parse(source);
    let ast = parse(source);
    let mandatory = count_nodes_by_kind(&ast, "MandatoryParameter");
    let slurpy = count_nodes_by_kind(&ast, "SlurpyParameter");
    assert_eq!(mandatory, 1, "Expected 1 MandatoryParameter for '($x, @rest)', got {}", mandatory);
    assert_eq!(slurpy, 1, "Expected 1 SlurpyParameter for '($x, @rest)', got {}", slurpy);
    Ok(())
}

/// `sub f($x = 0, $y = 0, @rest) {}` — mixed params with defaults + slurpy.
#[test]
fn test_sig_mixed_params_regression() -> Result<(), Box<dyn std::error::Error>> {
    let source = "sub f($x = 0, $y = 0, @rest) {}";
    assert_clean_parse(source);
    let ast = parse(source);
    let optional = count_nodes_by_kind(&ast, "OptionalParameter");
    let slurpy = count_nodes_by_kind(&ast, "SlurpyParameter");
    assert_eq!(
        optional, 2,
        "Expected 2 OptionalParameter for '($x = 0, $y = 0, @rest)', got {}",
        optional
    );
    assert_eq!(
        slurpy, 1,
        "Expected 1 SlurpyParameter for '($x = 0, $y = 0, @rest)', got {}",
        slurpy
    );
    Ok(())
}
