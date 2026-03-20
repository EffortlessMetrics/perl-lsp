//! Tests for die/warn exception context — issue #2359.
//!
//! Verifies that:
//! - die and warn hover docs include exception semantics ($@ context).
//! - Carp functions (croak, carp, confess, cluck) are documented.
//! - `is_exception_function` classifies the full exception family.
//! - `get_exception_context` returns structured upgrade advice for die.

use perl_semantic_analyzer::analysis::semantic::{
    get_builtin_documentation, get_exception_context, is_exception_function,
};
use perl_tdd_support::must_some;

// ---------------------------------------------------------------------------
// die / warn enhanced hover docs
// ---------------------------------------------------------------------------

#[test]
fn test_die_doc_mentions_exception_variable() {
    let doc = must_some(get_builtin_documentation("die"));
    // The description must tell the developer that $@ captures the error.
    assert!(
        doc.description.contains("$@"),
        "die description should mention $@ variable, got: {}",
        doc.description
    );
}

#[test]
fn test_die_doc_mentions_croak_alternative() {
    let doc = must_some(get_builtin_documentation("die"));
    assert!(
        doc.description.contains("croak") || doc.description.contains("Carp"),
        "die description should suggest Carp::croak as a module-safe alternative, got: {}",
        doc.description
    );
}

#[test]
fn test_warn_doc_mentions_stderr() {
    let doc = must_some(get_builtin_documentation("warn"));
    assert!(
        doc.description.contains("STDERR"),
        "warn description should mention STDERR, got: {}",
        doc.description
    );
}

#[test]
fn test_warn_doc_mentions_carp_alternative() {
    let doc = must_some(get_builtin_documentation("warn"));
    assert!(
        doc.description.contains("carp") || doc.description.contains("Carp"),
        "warn description should suggest Carp::carp as a module-safe alternative, got: {}",
        doc.description
    );
}

// ---------------------------------------------------------------------------
// Carp functions are documented
// ---------------------------------------------------------------------------

#[test]
fn test_croak_has_docs() {
    let doc = get_builtin_documentation("croak");
    assert!(doc.is_some(), "croak (Carp) should have hover documentation");
}

#[test]
fn test_croak_doc_mentions_caller() {
    let doc = must_some(get_builtin_documentation("croak"));
    assert!(
        doc.description.contains("caller") || doc.description.contains("stack"),
        "croak description should explain caller-perspective errors, got: {}",
        doc.description
    );
}

#[test]
fn test_carp_has_docs() {
    let doc = get_builtin_documentation("carp");
    assert!(doc.is_some(), "carp (Carp) should have hover documentation");
}

#[test]
fn test_confess_has_docs() {
    let doc = get_builtin_documentation("confess");
    assert!(doc.is_some(), "confess (Carp) should have hover documentation");
}

#[test]
fn test_cluck_has_docs() {
    let doc = get_builtin_documentation("cluck");
    assert!(doc.is_some(), "cluck (Carp) should have hover documentation");
}

// ---------------------------------------------------------------------------
// is_exception_function
// ---------------------------------------------------------------------------

#[test]
fn test_is_exception_function_die() {
    assert!(is_exception_function("die"), "die must be an exception function");
}

#[test]
fn test_is_exception_function_warn() {
    assert!(is_exception_function("warn"), "warn must be an exception function");
}

#[test]
fn test_is_exception_function_croak() {
    assert!(is_exception_function("croak"), "croak must be an exception function");
}

#[test]
fn test_is_exception_function_carp() {
    assert!(is_exception_function("carp"), "carp must be an exception function");
}

#[test]
fn test_is_exception_function_confess() {
    assert!(is_exception_function("confess"), "confess must be an exception function");
}

#[test]
fn test_is_exception_function_cluck() {
    assert!(is_exception_function("cluck"), "cluck must be an exception function");
}

#[test]
fn test_is_exception_function_rejects_print() {
    assert!(!is_exception_function("print"), "print must not be an exception function");
}

// ---------------------------------------------------------------------------
// get_exception_context
// ---------------------------------------------------------------------------

#[test]
fn test_get_exception_context_die_suggests_croak() {
    let ctx = must_some(get_exception_context("die"));
    assert!(
        ctx.preferred_alternative.is_some(),
        "die context should suggest a preferred alternative"
    );
    let alt = must_some(ctx.preferred_alternative);
    assert_eq!(alt, "Carp::croak", "die preferred alternative should be Carp::croak, got: {}", alt);
}

#[test]
fn test_get_exception_context_warn_suggests_carp() {
    let ctx = must_some(get_exception_context("warn"));
    assert!(
        ctx.preferred_alternative.is_some(),
        "warn context should suggest a preferred alternative"
    );
    let alt = must_some(ctx.preferred_alternative);
    assert_eq!(alt, "Carp::carp", "warn preferred alternative should be Carp::carp, got: {}", alt);
}

#[test]
fn test_get_exception_context_die_sets_error_var() {
    let ctx = must_some(get_exception_context("die"));
    assert_eq!(
        ctx.error_variable,
        Some("$@".to_string()),
        "die context should identify $@ as the error variable"
    );
}

#[test]
fn test_get_exception_context_croak_no_alternative() {
    // croak is already the preferred form — no further upgrade needed
    let ctx = must_some(get_exception_context("croak"));
    assert!(
        ctx.preferred_alternative.is_none(),
        "croak should have no preferred alternative (it is already preferred)"
    );
}

#[test]
fn test_get_exception_context_unknown_returns_none() {
    let ctx = get_exception_context("print");
    assert!(ctx.is_none(), "non-exception function should return None from get_exception_context");
}
