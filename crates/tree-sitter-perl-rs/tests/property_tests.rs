//! Property-based tests for `tree-sitter-perl-rs` `PerlLanguage` descriptor.
//!
//! These tests verify invariants that must hold for ALL inputs, not just specific examples.
//!
//! ## Properties Tested
//!
//! 1. **Consistency**: `node_kind_is_named(k)` returns true iff `k` is in `node_kind_names()`
//! 2. **Count invariant**: `node_kind_count()` equals `node_kind_names().len()`
//! 3. **Alphabetical order**: `node_kind_names()` is sorted alphabetically
//! 4. **Empty string**: `node_kind_is_named("")` returns false
//! 5. **Non-zero**: `node_kind_count() > 0`
//! 6. **Singleton**: `language()` returns pointer-identical instance on every call
//! 7. **Default wiring**: `PerlLanguage::default()` equals `LANGUAGE`

use proptest::prelude::*;
use tree_sitter_perl_rs::{language, LANGUAGE, PerlLanguage};

/// Property 1: `node_kind_is_named(k)` is consistent with `node_kind_names().contains(&k)`
///
/// For any possible string, the `is_named` query must agree exactly with whether that
/// string appears in the kind names list. This is the fundamental correctness property
/// of the descriptor.
proptest! {
    #[test]
    fn node_kind_is_named_is_consistent_with_kind_names(kind: String) {
        let lang = language();
        let kind_str: &str = &kind;
        let is_named = lang.node_kind_is_named(kind_str);
        let is_present = lang.node_kind_names().contains(&kind_str);
        prop_assert_eq!(
            is_named, is_present,
            "node_kind_is_named(\"{}\") = {} but kind {} in node_kind_names() = {}",
            kind, is_named, if is_present { "is" } else { "is NOT" }, is_present
        );
    }
}

/// Property 2: `node_kind_count()` equals `node_kind_names().len()`
///
/// The count returned by `node_kind_count()` must be exactly the length of the slice
/// returned by `node_kind_names()`. This is an internal consistency requirement.
#[test]
fn node_kind_count_matches_names_length() {
    let lang = language();
    let count = lang.node_kind_count();
    let names_len = lang.node_kind_names().len();
    assert_eq!(
        count, names_len,
        "node_kind_count() = {} but node_kind_names().len() = {}",
        count, names_len
    );
}

/// Property 3: `node_kind_names()` is alphabetically sorted
///
/// The spec documents that kind names are "in alphabetical order". This property
/// verifies the ordering contract holds.
#[test]
fn node_kind_names_are_alphabetically_sorted() {
    let lang = language();
    let names = lang.node_kind_names();
    let mut sorted = names.to_vec();
    sorted.sort_unstable();
    assert_eq!(
        names, sorted.as_slice(),
        "node_kind_names() must be in alphabetical order"
    );
}

/// Property 4: `node_kind_is_named("")` returns false
///
/// The empty string is not a valid kind name and must not be found in the kind names.
#[test]
fn node_kind_is_named_empty_string_returns_false() {
    let lang = language();
    assert!(
        !lang.node_kind_is_named(""),
        "empty string must not be a named kind"
    );
}

/// Property 5: `node_kind_count()` is always non-zero
///
/// A language descriptor with zero kinds would be useless. The grammar must have
/// at least one node kind.
#[test]
fn node_kind_count_is_nonzero() {
    let lang = language();
    assert!(
        lang.node_kind_count() > 0,
        "node_kind_count() must be non-zero, got {}",
        lang.node_kind_count()
    );
}

/// Property 6: `language()` returns the same instance on every call (singleton)
///
/// The `language()` function must return a pointer-identical instance. This is
/// critical for static analysis tools that rely on language descriptor identity.
#[test]
fn language_function_returns_singleton() {
    let first = language();
    let second = language();
    // Pointer equality on the backing slice proves singleton behavior
    assert!(
        std::ptr::eq(first.node_kind_names().as_ptr(), second.node_kind_names().as_ptr()),
        "language() must return the same static instance on every call"
    );
}

/// Property 7: `PerlLanguage::default()` equals `LANGUAGE`
///
/// The Default implementation must wire up to the LANGUAGE constant.
#[test]
fn default_returns_language_constant() {
    assert_eq!(
        PerlLanguage::default(),
        LANGUAGE,
        "PerlLanguage::default() must return LANGUAGE"
    );
}

/// Property 8: All kinds in `node_kind_names()` return true for `node_kind_is_named`
///
/// If a kind name appears in `node_kind_names()`, then `node_kind_is_named` for that
/// kind must return true. This is the converse of Property 1 and catches
/// cases where the contains check would fail.
#[test]
fn all_named_kinds_return_true_for_is_named() {
    let lang = language();
    for kind in lang.node_kind_names() {
        assert!(
            lang.node_kind_is_named(kind),
            "kind '{}' is in node_kind_names() so node_kind_is_named(\"{}\") must be true",
            kind, kind
        );
    }
}

/// Property 9: Unknown strings return false for `node_kind_is_named`
///
/// Strings that are NOT in `node_kind_names()` must return false. We test this
/// with a variety of generated non-matching strings.
proptest! {
    #[test]
    fn nonexistent_kinds_return_false(kind: String) {
        let lang = language();
        let kind_str: &str = &kind;
        // Only check if the string is actually not in the list
        if !lang.node_kind_names().contains(&kind_str) {
            prop_assert!(
                !lang.node_kind_is_named(kind_str),
                "node_kind_is_named(\"{}\") must be false for unknown kinds",
                kind
            );
        }
    }
}

/// Property 10: Equality is consistent with pointer equality of backing slice
///
/// Two `PerlLanguage` instances are equal if and only if they have the same backing
/// slice pointer. Since LANGUAGE is a static, any copy of it should be equal.
#[test]
fn perl_language_equality_is_backed_by_slice_pointer() {
    let first = language();
    let second = language();
    assert_eq!(first, second, "language() returns must be equal");
    assert!(
        std::ptr::eq(first.node_kind_names().as_ptr(), second.node_kind_names().as_ptr()),
        "equal PerlLanguage instances must have same backing slice"
    );
}

/// Property 11: LANGUAGE constant is accessible and valid
///
/// The static LANGUAGE constant must be accessible and contain valid data.
#[test]
fn language_constant_is_accessible_and_valid() {
    // LANGUAGE must be Sync (required for static)
    fn assert_sync<T: Sync>() {}
    assert_sync::<PerlLanguage>();

    // LANGUAGE must have non-zero kind count
    assert!(LANGUAGE.node_kind_count() > 0);

    // LANGUAGE must have alphabetically sorted names
    let names = LANGUAGE.node_kind_names();
    let mut sorted = names.to_vec();
    sorted.sort_unstable();
    assert_eq!(names, sorted.as_slice());
}
