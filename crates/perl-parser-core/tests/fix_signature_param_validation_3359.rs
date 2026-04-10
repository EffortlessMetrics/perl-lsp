/// Tests for signature parameter ordering and rule validation (issue #3359).
///
/// Perl 5.20+ signatures enforce ordering rules:
/// 1. Slurpy parameters (`@array` or `%hash`) must come last.
/// 2. Can't have both `@` and `%` slurpy parameters.
/// 3. A mandatory parameter cannot follow an optional parameter.
///
/// The parser should emit diagnostics for these violations while still
/// producing a usable AST (error-recovery mode).
use perl_parser_core::Parser;

/// Helper: parse and return diagnostic messages as a Vec<String>
fn parse_and_collect_errors(source: &str) -> Vec<String> {
    let mut parser = Parser::new(source);
    let _ast = parser.parse().ok();
    parser.errors().iter().map(|e| e.to_string()).collect()
}

// ---------------------------------------------------------------------------
// Rule 1: Slurpy parameter must be last
// ---------------------------------------------------------------------------

#[test]
fn test_slurpy_not_last_emits_diagnostic() {
    // @rest must come after $x — ordering violation
    let errors = parse_and_collect_errors("sub foo (@rest, $x) { }");
    assert!(
        errors.iter().any(|e| e.to_lowercase().contains("slurpy")),
        "Expected a 'slurpy' diagnostic for @rest before $x, got: {:?}",
        errors
    );
}

#[test]
fn test_slurpy_hash_not_last_emits_diagnostic() {
    // %opts must come after $x — ordering violation
    let errors = parse_and_collect_errors("sub bar (%opts, $x) { }");
    assert!(
        errors.iter().any(|e| e.to_lowercase().contains("slurpy")),
        "Expected a 'slurpy' diagnostic for %opts before $x, got: {:?}",
        errors
    );
}

#[test]
fn test_slurpy_last_is_valid() {
    // @rest at end is correct; should produce no slurpy diagnostic
    let errors = parse_and_collect_errors("sub foo ($x, $y, @rest) { }");
    assert!(
        !errors.iter().any(|e| e.to_lowercase().contains("slurpy")),
        "Unexpected slurpy diagnostic for valid signature, got: {:?}",
        errors
    );
}

// ---------------------------------------------------------------------------
// Rule 2: Can't have both @ and % slurpy parameters
// ---------------------------------------------------------------------------

#[test]
fn test_both_array_and_hash_slurpy_emits_diagnostic() {
    // @arr and %hash — can't have both slurpy types
    let errors = parse_and_collect_errors("sub bar (@arr, %hash) { }");
    assert!(
        errors.iter().any(|e| {
            let lower = e.to_lowercase();
            lower.contains("slurpy") || lower.contains("multiple")
        }),
        "Expected a diagnostic for both @ and % slurpy params, got: {:?}",
        errors
    );
}

#[test]
fn test_single_array_slurpy_is_valid() {
    let errors = parse_and_collect_errors("sub foo ($x, @rest) { }");
    assert!(
        errors.is_empty(),
        "Unexpected diagnostics for valid single-slurpy signature, got: {:?}",
        errors
    );
}

#[test]
fn test_single_hash_slurpy_is_valid() {
    let errors = parse_and_collect_errors("sub foo ($x, %opts) { }");
    assert!(
        errors.is_empty(),
        "Unexpected diagnostics for valid single-slurpy signature, got: {:?}",
        errors
    );
}

// ---------------------------------------------------------------------------
// Rule 3: Mandatory parameter cannot follow optional parameter
// ---------------------------------------------------------------------------

#[test]
fn test_mandatory_after_optional_emits_diagnostic() {
    // $x has default, $y is mandatory — invalid ordering
    let errors = parse_and_collect_errors("sub baz ($x = 1, $y) { }");
    assert!(
        errors.iter().any(|e| {
            let lower = e.to_lowercase();
            lower.contains("mandatory") || lower.contains("optional") || lower.contains("required")
        }),
        "Expected a diagnostic for mandatory after optional, got: {:?}",
        errors
    );
}

#[test]
fn test_optional_before_optional_is_valid() {
    // Both optional — should be fine
    let errors = parse_and_collect_errors("sub foo ($x = 1, $y = 2) { }");
    assert!(
        errors.is_empty(),
        "Unexpected diagnostics for all-optional signature, got: {:?}",
        errors
    );
}

#[test]
fn test_mandatory_before_optional_is_valid() {
    // Mandatory then optional — valid
    let errors = parse_and_collect_errors("sub foo ($x, $y = 2) { }");
    assert!(
        errors.is_empty(),
        "Unexpected diagnostics for mandatory-then-optional signature, got: {:?}",
        errors
    );
}

// ---------------------------------------------------------------------------
// Combined violations
// ---------------------------------------------------------------------------

#[test]
fn test_slurpy_not_last_with_optional_before_mandatory() {
    // Multiple violations: @rest is not last AND $y is mandatory after $x = 1
    let errors = parse_and_collect_errors("sub multi (@rest, $x = 1, $y) { }");
    assert!(
        !errors.is_empty(),
        "Expected at least one diagnostic for multiple violations, got: {:?}",
        errors
    );
}

// ---------------------------------------------------------------------------
// Regression: valid signatures produce no spurious diagnostics
// ---------------------------------------------------------------------------

#[test]
fn test_all_mandatory_no_diagnostics() {
    let errors = parse_and_collect_errors("sub foo ($a, $b, $c) { }");
    assert!(
        errors.is_empty(),
        "Unexpected diagnostics for all-mandatory signature, got: {:?}",
        errors
    );
}

#[test]
fn test_empty_signature_no_diagnostics() {
    let errors = parse_and_collect_errors("sub foo () { }");
    assert!(errors.is_empty(), "Unexpected diagnostics for empty signature, got: {:?}", errors);
}
