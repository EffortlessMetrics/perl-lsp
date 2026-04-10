//! BDD-style behavior specification tests for `perl-builtins`.
//!
//! These tests describe outcomes from an editor/LSP consumer perspective using
//! `Given/When/Then` naming.

use perl_builtins::builtin_signatures::create_builtin_signatures;
use perl_builtins::builtin_signatures_phf::{
    BUILTIN_FULL_SIGS, builtin_count, get_param_names, is_builtin,
};

fn has_hashmap_signature(name: &str, signature_fragment: &str) -> bool {
    create_builtin_signatures()
        .get(name)
        .is_some_and(|sig| sig.signatures.iter().any(|s| s.contains(signature_fragment)))
}

fn has_hashmap_doc(name: &str, doc_fragment: &str) -> bool {
    create_builtin_signatures()
        .get(name)
        .is_some_and(|sig| sig.documentation.contains(doc_fragment))
}

#[test]
fn given_common_io_builtin_when_queried_then_it_is_recognized() {
    for name in ["print", "open", "read", "syswrite"] {
        assert!(is_builtin(name), "{name} should be recognized as builtin");
    }
}

#[test]
fn given_non_builtin_word_when_queried_then_it_is_rejected() {
    for name in ["frobnicate", "my_custom_fn", "Print", " open"] {
        assert!(!is_builtin(name), "{name:?} should not be recognized as builtin");
    }
}

#[test]
fn given_case_or_whitespace_mismatch_when_lookup_then_no_params_are_returned() {
    assert!(get_param_names("Print").is_empty());
    assert!(get_param_names("print ").is_empty());
    assert!(get_param_names(" print").is_empty());
}

#[test]
fn given_known_builtin_when_param_names_requested_then_editor_facing_labels_are_stable() {
    assert_eq!(get_param_names("open"), ["FILEHANDLE", "MODE", "FILENAME"]);
    assert_eq!(get_param_names("substr"), ["EXPR", "OFFSET", "LENGTH", "REPLACEMENT"]);
    assert_eq!(get_param_names("socket"), ["SOCKET", "DOMAIN", "TYPE", "PROTOCOL"]);
}

#[test]
fn given_file_test_operator_when_param_names_requested_then_single_file_param_is_exposed() {
    for op in ["-e", "-f", "-d", "-r", "-w", "-x", "-T", "-B", "-M", "-C"] {
        assert_eq!(get_param_names(op), ["FILE"], "{op} should expose FILE as the only parameter");
    }
}

#[test]
fn given_zero_arg_builtin_when_param_names_requested_then_empty_slice_is_returned() {
    for name in ["fork", "time", "wait", "getuid", "wantarray"] {
        assert!(get_param_names(name).is_empty(), "{name} should expose zero params");
    }
}

#[test]
fn given_signature_help_target_when_full_signatures_requested_then_most_specific_variant_is_first()
{
    let print_sigs = BUILTIN_FULL_SIGS.get("print").copied().unwrap_or(&[]);
    assert!(!print_sigs.is_empty());
    assert_eq!(print_sigs[0], "print FILEHANDLE LIST");

    let split_sigs = BUILTIN_FULL_SIGS.get("split").copied().unwrap_or(&[]);
    assert!(!split_sigs.is_empty());
    assert_eq!(split_sigs[0], "split PATTERN, EXPR, LIMIT");
}

#[test]
fn given_hashmap_lookup_when_consulting_open_then_signatures_and_doc_support_hover() {
    assert!(has_hashmap_signature("open", "MODE"));
    assert!(has_hashmap_signature("open", "FILENAME"));
    assert!(has_hashmap_doc("open", "Opens a file"));
}

#[test]
fn given_hashmap_lookup_when_consulting_print_then_multiple_call_forms_are_available() {
    let signatures = create_builtin_signatures()
        .get("print")
        .map(|entry| entry.signatures.as_slice())
        .unwrap_or(&[]);

    assert!(signatures.iter().any(|s| *s == "print FILEHANDLE LIST"));
    assert!(signatures.iter().any(|s| *s == "print LIST"));
    assert!(signatures.iter().any(|s| *s == "print"));
}

#[test]
fn given_public_catalog_when_counted_then_it_stays_large_enough_for_core_perl_surface() {
    assert!(builtin_count() >= 200, "builtin catalog unexpectedly shrank");
}
