use perl_builtins_phf::{
    BUILTIN_FULL_SIGS, BUILTIN_SIGS, builtin_count, get_param_names, is_builtin,
};

#[test]
fn get_param_names_returns_empty_slice_for_unknown_builtin() {
    assert_eq!(get_param_names("not_a_builtin"), &[""; 0]);
}

#[test]
fn builtin_count_matches_primary_signature_map_length() {
    assert_eq!(builtin_count(), BUILTIN_SIGS.len());
    assert!(BUILTIN_FULL_SIGS.len() < builtin_count());
    assert!(BUILTIN_FULL_SIGS.len() >= 40);
}

#[test]
fn file_test_operators_are_available_in_primary_signature_map() {
    for operator in ["-e", "-f", "-d", "-r", "-w", "-x", "-T", "-B", "-M", "-C"] {
        assert!(is_builtin(operator), "expected {operator} to be recognized as a builtin");
        assert_eq!(get_param_names(operator), ["FILE"]);
    }
}

#[test]
fn zero_argument_builtins_are_preserved() {
    for name in ["fork", "wait", "time", "endhostent"] {
        assert!(is_builtin(name), "expected {name} to be recognized as a builtin");
        assert_eq!(get_param_names(name), &[""; 0], "expected {name} to remain zero-argument");
    }
}

#[test]
fn full_signatures_keep_multi_variant_lookup_entries() {
    assert_eq!(
        BUILTIN_FULL_SIGS.get("system").copied(),
        Some(&["system PROGRAM, LIST", "system PROGRAM"][..])
    );
    assert_eq!(
        BUILTIN_FULL_SIGS.get("open").copied(),
        Some(&["open FILEHANDLE, MODE, FILENAME", "open FILEHANDLE, EXPR", "open FILEHANDLE",][..])
    );
}

#[test]
fn builtin_lookup_is_case_sensitive() {
    assert!(is_builtin("print"));
    assert!(!is_builtin("Print"));
    assert!(BUILTIN_FULL_SIGS.get("Print").is_none());
}
