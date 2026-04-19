//! Tests for die/warn exception context documentation and classification.
//!
//! Covers hover docs for die/warn/eval and Carp functions,
//! plus the `is_exception_function` and `get_exception_context` helpers.

use perl_semantic_analyzer::analysis::semantic::{
    get_builtin_documentation, get_exception_context, is_exception_function,
};
use perl_tdd_support::must_some;

// ---------------------------------------------------------------------------
// Hover doc enrichment — die
// ---------------------------------------------------------------------------

#[test]
fn test_die_hover_mentions_carp() {
    let doc = get_builtin_documentation("die");
    assert!(doc.is_some(), "die must have documentation");
    let doc = must_some(doc);
    assert!(
        doc.description.contains("Carp"),
        "die hover should mention Carp upgrade path, got: {}",
        doc.description
    );
}

#[test]
fn test_die_hover_mentions_error_variable() {
    let doc = get_builtin_documentation("die");
    assert!(doc.is_some(), "die must have documentation");
    let doc = must_some(doc);
    assert!(
        doc.description.contains("$@"),
        "die hover should mention $@ error variable, got: {}",
        doc.description
    );
}

// ---------------------------------------------------------------------------
// Hover doc enrichment — warn
// ---------------------------------------------------------------------------

#[test]
fn test_warn_hover_mentions_carp() {
    let doc = get_builtin_documentation("warn");
    assert!(doc.is_some(), "warn must have documentation");
    let doc = must_some(doc);
    assert!(
        doc.description.contains("Carp"),
        "warn hover should mention Carp upgrade path, got: {}",
        doc.description
    );
}

// ---------------------------------------------------------------------------
// Hover doc enrichment — eval
// ---------------------------------------------------------------------------

#[test]
fn test_eval_hover_mentions_error_variable() {
    let doc = get_builtin_documentation("eval");
    assert!(doc.is_some(), "eval must have documentation");
    let doc = must_some(doc);
    assert!(
        doc.description.contains("$@"),
        "eval hover should mention $@ error variable, got: {}",
        doc.description
    );
}

// ---------------------------------------------------------------------------
// Carp function docs
// ---------------------------------------------------------------------------

#[test]
fn test_croak_has_docs() {
    let doc = get_builtin_documentation("croak");
    assert!(doc.is_some(), "croak must have documentation");
}

#[test]
fn test_carp_has_docs() {
    let doc = get_builtin_documentation("carp");
    assert!(doc.is_some(), "carp must have documentation");
}

#[test]
fn test_confess_has_docs() {
    let doc = get_builtin_documentation("confess");
    assert!(doc.is_some(), "confess must have documentation");
}

#[test]
fn test_cluck_has_docs() {
    let doc = get_builtin_documentation("cluck");
    assert!(doc.is_some(), "cluck must have documentation");
}

#[test]
fn test_confess_doc_mentions_stack_trace() {
    let doc = get_builtin_documentation("confess");
    assert!(doc.is_some(), "confess must have docs");
    let doc = must_some(doc);
    assert!(
        doc.description.contains("stack"),
        "confess description should mention stack trace, got: {}",
        doc.description
    );
}

#[test]
fn test_cluck_doc_mentions_stack_trace() {
    let doc = get_builtin_documentation("cluck");
    assert!(doc.is_some(), "cluck must have docs");
    let doc = must_some(doc);
    assert!(
        doc.description.contains("stack"),
        "cluck description should mention stack trace, got: {}",
        doc.description
    );
}

// ---------------------------------------------------------------------------
// is_exception_function
// ---------------------------------------------------------------------------

#[test]
fn test_die_is_exception_function() {
    assert!(
        is_exception_function("die"),
        "die must be an exception function"
    );
}

#[test]
fn test_warn_is_exception_function() {
    assert!(
        is_exception_function("warn"),
        "warn must be an exception function"
    );
}

#[test]
fn test_croak_is_exception_function() {
    assert!(
        is_exception_function("croak"),
        "croak must be an exception function"
    );
}

#[test]
fn test_carp_is_exception_function() {
    assert!(
        is_exception_function("carp"),
        "carp must be an exception function"
    );
}

#[test]
fn test_confess_is_exception_function() {
    assert!(
        is_exception_function("confess"),
        "confess must be an exception function"
    );
}

#[test]
fn test_cluck_is_exception_function() {
    assert!(
        is_exception_function("cluck"),
        "cluck must be an exception function"
    );
}

#[test]
fn test_print_is_not_exception_function() {
    assert!(
        !is_exception_function("print"),
        "print must not be an exception function"
    );
}

// ---------------------------------------------------------------------------
// get_exception_context
// ---------------------------------------------------------------------------

#[test]
fn test_die_exception_context_has_alternative() {
    let ctx = get_exception_context("die");
    assert!(ctx.is_some(), "die must have exception context");
    let ctx = must_some(ctx);
    assert!(
        ctx.preferred_alternative.is_some(),
        "die should have a preferred alternative (croak)"
    );
    let alt = ctx.preferred_alternative.as_deref().unwrap_or("");
    assert!(
        alt.contains("croak"),
        "die preferred alternative should be croak, got: {}",
        alt
    );
}

#[test]
fn test_die_exception_context_has_error_variable() {
    let ctx = get_exception_context("die");
    assert!(ctx.is_some(), "die must have exception context");
    let ctx = must_some(ctx);
    assert_eq!(
        ctx.error_variable.as_deref(),
        Some("$@"),
        "die error variable should be $@"
    );
}

#[test]
fn test_warn_exception_context_has_alternative() {
    let ctx = get_exception_context("warn");
    assert!(ctx.is_some(), "warn must have exception context");
    let ctx = must_some(ctx);
    assert!(
        ctx.preferred_alternative.is_some(),
        "warn should have a preferred alternative (carp)"
    );
    let alt = ctx.preferred_alternative.as_deref().unwrap_or("");
    assert!(
        alt.contains("carp"),
        "warn preferred alternative should be carp, got: {}",
        alt
    );
}

#[test]
fn test_croak_exception_context_no_alternative() {
    let ctx = get_exception_context("croak");
    assert!(ctx.is_some(), "croak must have exception context");
    let ctx = must_some(ctx);
    assert!(
        ctx.preferred_alternative.is_none(),
        "croak has no preferred alternative (already preferred), got: {:?}",
        ctx.preferred_alternative
    );
}

#[test]
fn test_confess_exception_context_present() {
    let ctx = get_exception_context("confess");
    assert!(ctx.is_some(), "confess must have exception context");
}

#[test]
fn test_get_exception_context_eval_returns_none() {
    let ctx = get_exception_context("eval");
    assert!(ctx.is_none(), "eval is not an exception function");
}

#[test]
fn test_get_exception_context_unknown_returns_none() {
    let ctx = get_exception_context("print");
    assert!(ctx.is_none(), "print must not have exception context");
}
