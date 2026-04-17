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
fn scenario_utf8_encode_decode_signatures_are_available() -> Result<(), String> {
    // Given a user writes utf8::encode($str) or utf8::decode($str) — core Perl
    // functions for explicit UTF-8 encoding control (perldoc utf8).
    let signatures = create_builtin_signatures();

    // When the builtin metadata for each function is resolved.
    let encode = must_some(signatures.get("utf8::encode"));
    let decode = must_some(signatures.get("utf8::decode"));

    // Then both functions are recognized as builtins with documentation
    // covering their effect on the scalar's encoding state.
    if !is_builtin("utf8::encode") {
        return Err("utf8::encode should be recognized as builtin".into());
    }
    if !is_builtin("utf8::decode") {
        return Err("utf8::decode should be recognized as builtin".into());
    }
    if encode.signatures != vec!["utf8::encode SCALAR"] {
        return Err(format!("utf8::encode signature mismatch: {:?}", encode.signatures));
    }
    if decode.signatures != vec!["utf8::decode SCALAR"] {
        return Err(format!("utf8::decode signature mismatch: {:?}", decode.signatures));
    }
    if !encode.documentation.contains("UTF-8") {
        return Err("utf8::encode documentation should mention UTF-8".into());
    }
    if !decode.documentation.contains("UTF-8") {
        return Err("utf8::decode documentation should mention UTF-8".into());
    }

    // And parameter names are exposed via the PHF fast path.
    if get_param_names("utf8::encode") != ["SCALAR"] {
        return Err("utf8::encode should take one SCALAR parameter".into());
    }
    if get_param_names("utf8::decode") != ["SCALAR"] {
        return Err("utf8::decode should take one SCALAR parameter".into());
    }
    Ok(())
}

#[test]
fn scenario_utf8_downgrade_exposes_multi_variant_signature() -> Result<(), String> {
    // utf8::downgrade can be called with an optional FAIL_OK second argument;
    // both call shapes should be discoverable via signature help.
    let signatures = create_builtin_signatures();
    let downgrade = must_some(signatures.get("utf8::downgrade"));

    if downgrade.signatures.len() < 2 {
        return Err(format!(
            "utf8::downgrade should expose both one- and two-arg forms, got {:?}",
            downgrade.signatures
        ));
    }
    if !downgrade.signatures.iter().any(|s| s.contains("FAIL_OK")) {
        return Err("utf8::downgrade should advertise FAIL_OK form".into());
    }
    if get_param_names("utf8::downgrade") != ["SCALAR", "FAIL_OK"] {
        return Err("utf8::downgrade should take SCALAR plus optional FAIL_OK".into());
    }

    // The full-signature PHF map should also contain both variants so that
    // signature help surfaces them both.
    let full = *must_some(BUILTIN_FULL_SIGS.get("utf8::downgrade"));
    if full.len() != 2 {
        return Err(format!(
            "utf8::downgrade should have 2 full signature variants, got {:?}",
            full
        ));
    }
    Ok(())
}

#[test]
fn scenario_utf8_namespace_functions_all_registered() -> Result<(), String> {
    // The perldoc utf8 surface that perl-lsp supports: encode, decode,
    // is_utf8, valid, upgrade, downgrade, and the native/unicode converters.
    let signatures = create_builtin_signatures();
    let utf8_fns = [
        "utf8::encode",
        "utf8::decode",
        "utf8::is_utf8",
        "utf8::valid",
        "utf8::upgrade",
        "utf8::downgrade",
        "utf8::native_to_unicode",
        "utf8::unicode_to_native",
    ];
    for name in &utf8_fns {
        if !signatures.contains_key(name) {
            return Err(format!("signatures map missing {name}"));
        }
        if !is_builtin(name) {
            return Err(format!("PHF is_builtin missing {name}"));
        }
        let full = *must_some(BUILTIN_FULL_SIGS.get(name));
        if full.is_empty() {
            return Err(format!("full signatures missing for {name}"));
        }
        // Every full signature variant starts with the function name so that
        // signature help renders qualified call syntax.
        if !full.iter().all(|s| s.starts_with(name)) {
            return Err(format!("full signatures for {name} should start with the name: {full:?}"));
        }
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
