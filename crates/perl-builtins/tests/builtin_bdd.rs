use perl_builtins::builtin_signatures::create_builtin_signatures;
use perl_builtins::builtin_signatures_phf::{BUILTIN_FULL_SIGS, get_param_names, is_builtin};
use perl_tdd_support::must_some;

#[test]
fn given_known_core_builtin_when_checking_membership_then_it_is_reported_as_builtin() {
    assert!(is_builtin("print"));
    assert!(is_builtin("open"));
    assert!(is_builtin("substr"));
}

#[test]
fn given_unknown_symbol_when_checking_membership_then_it_is_not_reported_as_builtin() {
    assert!(!is_builtin("totally_not_a_builtin"));
}

#[test]
fn given_unknown_symbol_when_requesting_param_names_then_an_empty_slice_is_returned() {
    assert!(get_param_names("totally_not_a_builtin").is_empty());
}

#[test]
fn given_open_builtin_when_requesting_param_names_then_the_expected_three_arg_shape_is_exposed() {
    assert_eq!(get_param_names("open"), &["FILEHANDLE", "MODE", "FILENAME"]);
}

#[test]
fn given_file_test_operator_when_requesting_param_names_then_single_file_argument_is_returned() {
    assert_eq!(get_param_names("-e"), &["FILE"]);
}

#[test]
fn given_print_builtin_when_requesting_full_signatures_then_a_filehandle_variant_is_available() {
    let sigs = must_some(BUILTIN_FULL_SIGS.get("print"));
    assert!(!sigs.is_empty());
    assert!(sigs.iter().any(|sig| sig.contains("FILEHANDLE") && sig.contains("LIST")));
}

#[test]
fn given_signatures_catalog_when_querying_open_then_variants_and_docs_are_present() {
    let signatures = create_builtin_signatures();
    let open = must_some(signatures.get("open"));

    assert!(!open.signatures.is_empty());
    assert!(open.signatures.iter().any(|sig| sig.contains("MODE") && sig.contains("FILENAME")));
    assert!(!open.documentation.trim().is_empty());
}

#[test]
fn given_signatures_catalog_when_constructed_twice_then_the_same_cached_map_is_reused() {
    let first = create_builtin_signatures();
    let second = create_builtin_signatures();

    assert!(std::ptr::eq(first, second));
}
