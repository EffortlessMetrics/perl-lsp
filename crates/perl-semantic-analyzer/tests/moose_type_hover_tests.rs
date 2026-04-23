//! Tests for Moose/Moo type constraint documentation on hover.
//!
//! Covers `get_moose_type_documentation` for built-in Moose types:
//! simple types (Str, Int, Bool, etc.), parametrized types (ArrayRef[Int]),
//! and Maybe types.  Also covers `get_attribute_documentation` which is
//! exercised by the sibling `attribute_introspection_tests` file.

use perl_semantic_analyzer::analysis::semantic::{
    get_attribute_documentation, get_moose_type_documentation,
};
use perl_tdd_support::must_some;

// ---------------------------------------------------------------------------
// get_moose_type_documentation — simple base types
// ---------------------------------------------------------------------------

#[test]
fn moose_type_doc_str_has_description() {
    let doc = get_moose_type_documentation("Str");
    assert!(doc.is_some(), "Str should have documentation");
    let doc = must_some(doc);
    assert!(
        doc.description.to_lowercase().contains("string")
            || doc.description.to_lowercase().contains("str"),
        "Str description should mention string, got: {}",
        doc.description
    );
}

#[test]
fn moose_type_doc_int_has_description() {
    let doc = get_moose_type_documentation("Int");
    assert!(doc.is_some(), "Int should have documentation");
    let doc = must_some(doc);
    assert!(
        doc.description.to_lowercase().contains("integer")
            || doc.description.to_lowercase().contains("int"),
        "Int description should mention integer, got: {}",
        doc.description
    );
}

#[test]
fn moose_type_doc_bool_has_description() {
    let doc = get_moose_type_documentation("Bool");
    assert!(doc.is_some(), "Bool should have documentation");
    let doc = must_some(doc);
    assert!(
        doc.description.to_lowercase().contains("bool")
            || doc.description.to_lowercase().contains("true")
            || doc.description.to_lowercase().contains("false"),
        "Bool description should mention boolean values, got: {}",
        doc.description
    );
}

#[test]
fn moose_type_doc_arrayref_has_description() {
    let doc = get_moose_type_documentation("ArrayRef");
    assert!(doc.is_some(), "ArrayRef should have documentation");
    let doc = must_some(doc);
    assert!(
        doc.description.to_lowercase().contains("array")
            || doc.description.to_lowercase().contains("ref"),
        "ArrayRef description should mention array reference, got: {}",
        doc.description
    );
}

#[test]
fn moose_type_doc_hashref_has_description() {
    let doc = get_moose_type_documentation("HashRef");
    assert!(doc.is_some(), "HashRef should have documentation");
    let doc = must_some(doc);
    assert!(
        doc.description.to_lowercase().contains("hash")
            || doc.description.to_lowercase().contains("ref"),
        "HashRef description should mention hash reference, got: {}",
        doc.description
    );
}

#[test]
fn moose_type_doc_maybe_has_description() {
    let doc = get_moose_type_documentation("Maybe");
    assert!(doc.is_some(), "Maybe should have documentation");
    let doc = must_some(doc);
    assert!(
        doc.description.to_lowercase().contains("undef")
            || doc.description.to_lowercase().contains("maybe")
            || doc.description.to_lowercase().contains("optional"),
        "Maybe description should mention undef/optional, got: {}",
        doc.description
    );
}

#[test]
fn moose_type_doc_num_has_description() {
    let doc = get_moose_type_documentation("Num");
    assert!(doc.is_some(), "Num should have documentation");
    let doc = must_some(doc);
    assert!(
        doc.description.to_lowercase().contains("num")
            || doc.description.to_lowercase().contains("float")
            || doc.description.to_lowercase().contains("number"),
        "Num description should mention number, got: {}",
        doc.description
    );
}

#[test]
fn moose_type_doc_coderef_has_description() {
    let doc = get_moose_type_documentation("CodeRef");
    assert!(doc.is_some(), "CodeRef should have documentation");
    let doc = must_some(doc);
    assert!(
        doc.description.to_lowercase().contains("code")
            || doc.description.to_lowercase().contains("subroutine")
            || doc.description.to_lowercase().contains("sub"),
        "CodeRef description should mention code/subroutine, got: {}",
        doc.description
    );
}

#[test]
fn moose_type_doc_unknown_returns_none() {
    let doc = get_moose_type_documentation("NonExistentTypeXyz");
    assert!(doc.is_none(), "unknown Moose type should return None");
}

// ---------------------------------------------------------------------------
// Parametrized type lookup — strip parameter and resolve base type
// ---------------------------------------------------------------------------

#[test]
fn moose_type_doc_arrayref_int_resolves_base() {
    // "ArrayRef[Int]" should resolve to the ArrayRef documentation
    let doc = get_moose_type_documentation("ArrayRef[Int]");
    assert!(doc.is_some(), "ArrayRef[Int] should resolve to ArrayRef documentation");
    let doc = must_some(doc);
    assert!(
        doc.description.to_lowercase().contains("array")
            || doc.description.to_lowercase().contains("ref"),
        "ArrayRef[Int] description should describe array reference, got: {}",
        doc.description
    );
}

#[test]
fn moose_type_doc_maybe_str_resolves_base() {
    // "Maybe[Str]" should resolve to Maybe documentation
    let doc = get_moose_type_documentation("Maybe[Str]");
    assert!(doc.is_some(), "Maybe[Str] should resolve to Maybe documentation");
    let doc = must_some(doc);
    assert!(
        doc.description.to_lowercase().contains("undef")
            || doc.description.to_lowercase().contains("maybe")
            || doc.description.to_lowercase().contains("optional"),
        "Maybe[Str] description should mention undef/optional, got: {}",
        doc.description
    );
}

#[test]
fn moose_type_doc_hashref_str_str_resolves_base() {
    // "HashRef[Str]" should resolve to HashRef documentation
    let doc = get_moose_type_documentation("HashRef[Str]");
    assert!(doc.is_some(), "HashRef[Str] should resolve to HashRef documentation");
}

// ---------------------------------------------------------------------------
// Signature format validation
// ---------------------------------------------------------------------------

#[test]
fn moose_type_doc_str_signature_not_empty() {
    let doc = get_moose_type_documentation("Str");
    let doc = must_some(doc);
    assert!(!doc.signature.is_empty(), "Str signature should not be empty");
}

#[test]
fn moose_type_doc_covers_core_types() {
    let core_types = [
        "Str",
        "Int",
        "Num",
        "Bool",
        "ArrayRef",
        "HashRef",
        "CodeRef",
        "RegexpRef",
        "ScalarRef",
        "Object",
        "Defined",
        "Value",
        "Ref",
        "Maybe",
        "Undef",
        "Any",
        "Item",
    ];
    for type_name in &core_types {
        let doc = get_moose_type_documentation(type_name);
        assert!(
            doc.is_some(),
            "Expected documentation for Moose type '{}' but got None",
            type_name
        );
        let doc = must_some(doc);
        assert!(
            !doc.description.is_empty(),
            "Documentation for Moose type '{}' has empty description",
            type_name
        );
    }
}

// ---------------------------------------------------------------------------
// get_attribute_documentation — these tests duplicate the existing
// attribute_introspection_tests but keep them here for completeness
// and to confirm the function is exported correctly from the same path.
// ---------------------------------------------------------------------------

#[test]
fn attribute_doc_exported_lvalue() {
    // Confirm get_attribute_documentation is exported from analysis::semantic
    let doc = get_attribute_documentation("lvalue");
    assert!(doc.is_some(), "lvalue attribute should have documentation");
}
