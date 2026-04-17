//! Behavior-driven tests for `perl-builtins`.
//!
//! These scenarios describe expected user-facing behavior for editor tooling
//! consumers (completion, signature help, and hover).

use std::ptr;

use perl_lexer::builtins::builtin_signatures::create_builtin_signatures;
use perl_lexer::builtins::phf_lookup::{BUILTIN_FULL_SIGS, get_param_names, is_builtin};
use perl_tdd_support::must_some;

#[test]
fn scenario_signature_help_for_print_builtin() -> Result<(), String> {
    // Given a user asks for signature help on a known builtin.
    let signatures = create_builtin_signatures();

    // When the builtin metadata is resolved for `print`.
    let print_signature = must_some(signatures.get("print"));
    let full_print_signatures = *must_some(BUILTIN_FULL_SIGS.get("print"));

    // Then signature variants and documentation are available to the client.
    if print_signature.signatures.len() < 2 {
        return Err("expected multiple signature variants for print".into());
    }
    if !print_signature.documentation.contains("Prints") {
        return Err("expected print documentation to describe printing behavior".into());
    }
    if full_print_signatures.is_empty() {
        return Err("expected full signatures for print".into());
    }

    Ok(())
}

#[test]
fn scenario_unknown_symbol_is_not_treated_as_builtin() -> Result<(), String> {
    // Given an arbitrary non-builtin symbol.
    let unknown = "definitely_not_a_perl_builtin";

    // When builtin lookup APIs are queried.
    let signatures = create_builtin_signatures();
    let builtin = is_builtin(unknown);
    let params = get_param_names(unknown);

    // Then all lookup APIs should report "unknown" consistently.
    if builtin {
        return Err("unknown symbol unexpectedly reported as builtin".into());
    }
    if !params.is_empty() {
        return Err("unknown symbol unexpectedly returned parameter names".into());
    }
    if signatures.contains_key(unknown) {
        return Err("unknown symbol unexpectedly present in signature map".into());
    }

    Ok(())
}

#[test]
fn scenario_file_test_operator_metadata_is_available() -> Result<(), String> {
    // Given a user writes a Perl file-test operator expression.
    let operator = "-e";

    // When the builtin metadata for the operator is queried.
    let signatures = create_builtin_signatures();
    let file_test_signature = must_some(signatures.get(operator));
    let params = get_param_names(operator);

    // Then the operator is recognized and carries file-centric parameter/docs.
    if !is_builtin(operator) {
        return Err("file-test operator should be recognized as builtin".into());
    }
    if params != ["FILE"] {
        return Err(format!("file-test operator should expose FILE parameter, got {params:?}"));
    }
    if !file_test_signature.documentation.contains("File") {
        return Err("file-test operator documentation should mention files".into());
    }

    Ok(())
}

#[test]
fn scenario_signature_store_is_singleton_backed() -> Result<(), String> {
    // Given two independent requests for builtin metadata.
    let first = create_builtin_signatures();
    let second = create_builtin_signatures();

    // When both maps are compared by address.
    let same_allocation = ptr::eq(first, second);

    // Then both references should point at the same OnceLock-backed map.
    if !same_allocation {
        return Err("create_builtin_signatures should return a singleton reference".into());
    }

    Ok(())
}
