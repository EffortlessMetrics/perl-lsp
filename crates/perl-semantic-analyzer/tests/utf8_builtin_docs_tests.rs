//! Unit tests for utf8::* builtin function documentation.
//!
//! These tests verify that hover documentation is provided for the three
//! utf8 module functions that convert between Unicode strings and UTF-8
//! encoded bytes: utf8::encode, utf8::decode, and utf8::downgrade.
//!
//! Issue: #3371 - perl-lsp does not provide hover documentation for
//! utf8::encode() and utf8::decode()

use perl_semantic_analyzer::analysis::semantic::get_builtin_documentation;
use perl_tdd_support::must_some;

// ---------------------------------------------------------------------------
// utf8::encode
// ---------------------------------------------------------------------------

#[test]
fn test_utf8_encode_returns_documentation() {
    let doc = must_some(get_builtin_documentation("utf8::encode"));
    // utf8::encode converts a Unicode string to UTF-8 encoded bytes
    assert!(
        doc.description.contains("UTF-8") || doc.description.contains("UTF8"),
        "utf8::encode description should mention UTF-8 encoding: {}",
        doc.description
    );
    assert!(
        doc.description.contains("Unicode") || doc.description.contains("string"),
        "utf8::encode description should mention Unicode or string: {}",
        doc.description
    );
    assert!(
        doc.description.contains("convert") || doc.description.contains("bytes"),
        "utf8::encode description should mention conversion or bytes: {}",
        doc.description
    );
    assert!(
        !doc.signature.is_empty(),
        "utf8::encode should have a non-empty signature"
    );
}

#[test]
fn test_utf8_encode_signature_mentions_scalar() {
    let doc = must_some(get_builtin_documentation("utf8::encode"));
    // utf8::encode operates on a scalar in place
    assert!(
        doc.signature.contains("SCALAR") || doc.signature.contains("scalar"),
        "utf8::encode signature should mention SCALAR: {}",
        doc.signature
    );
}

// ---------------------------------------------------------------------------
// utf8::decode
// ---------------------------------------------------------------------------

#[test]
fn test_utf8_decode_returns_documentation() {
    let doc = must_some(get_builtin_documentation("utf8::decode"));
    // utf8::decode converts UTF-8 encoded bytes to a Unicode string
    assert!(
        doc.description.contains("UTF-8") || doc.description.contains("UTF8"),
        "utf8::decode description should mention UTF-8 encoding: {}",
        doc.description
    );
    assert!(
        doc.description.contains("Unicode") || doc.description.contains("string"),
        "utf8::decode description should mention Unicode or string: {}",
        doc.description
    );
    assert!(
        doc.description.contains("convert") || doc.description.contains("bytes"),
        "utf8::decode description should mention conversion or bytes: {}",
        doc.description
    );
    assert!(
        !doc.signature.is_empty(),
        "utf8::decode should have a non-empty signature"
    );
}

#[test]
fn test_utf8_decode_signature_mentions_scalar() {
    let doc = must_some(get_builtin_documentation("utf8::decode"));
    // utf8::decode operates on a scalar in place
    assert!(
        doc.signature.contains("SCALAR") || doc.signature.contains("scalar"),
        "utf8::decode signature should mention SCALAR: {}",
        doc.signature
    );
}

// ---------------------------------------------------------------------------
// utf8::downgrade
// ---------------------------------------------------------------------------

#[test]
fn test_utf8_downgrade_returns_documentation() {
    let doc = must_some(get_builtin_documentation("utf8::downgrade"));
    // utf8::downgrade attempts to convert a Unicode string to bytes
    // (fails if the string contains characters beyond U+00FF)
    assert!(
        doc.description.contains("Unicode") || doc.description.contains("string"),
        "utf8::downgrade description should mention Unicode or string: {}",
        doc.description
    );
    assert!(
        doc.description.contains("convert") || doc.description.contains("bytes"),
        "utf8::downgrade description should mention conversion or bytes: {}",
        doc.description
    );
    assert!(
        !doc.signature.is_empty(),
        "utf8::downgrade should have a non-empty signature"
    );
}

#[test]
fn test_utf8_downgrade_signature_mentions_scalar() {
    let doc = must_some(get_builtin_documentation("utf8::downgrade"));
    // utf8::downgrade operates on a scalar in place, with optional FAIL_OK
    assert!(
        doc.signature.contains("SCALAR") || doc.signature.contains("scalar"),
        "utf8::downgrade signature should mention SCALAR: {}",
        doc.signature
    );
}

#[test]
fn test_utf8_downgrade_mentions_failure_case() {
    let doc = must_some(get_builtin_documentation("utf8::downgrade"));
    // utf8::downgrade fails if the string contains characters beyond U+00FF
    // The description should mention this limitation
    assert!(
        doc.description.contains("fail") || doc.description.contains("U+00FF"),
        "utf8::downgrade description should mention failure case or U+00FF limitation: {}",
        doc.description
    );
}

// ---------------------------------------------------------------------------
// Regression: utf8 functions should NOT return None
// ---------------------------------------------------------------------------

#[test]
fn test_utf8_encode_not_none() {
    // This test documents the bug: get_builtin_documentation returns None
    // for utf8::encode when it should return Some(BuiltinDoc)
    let result = get_builtin_documentation("utf8::encode");
    assert!(
        result.is_some(),
        "get_builtin_documentation(\"utf8::encode\") should return Some, got None. \
         Issue #3371: perl-lsp does not provide hover documentation for utf8::encode"
    );
}

#[test]
fn test_utf8_decode_not_none() {
    // This test documents the bug: get_builtin_documentation returns None
    // for utf8::decode when it should return Some(BuiltinDoc)
    let result = get_builtin_documentation("utf8::decode");
    assert!(
        result.is_some(),
        "get_builtin_documentation(\"utf8::decode\") should return Some, got None. \
         Issue #3371: perl-lsp does not provide hover documentation for utf8::decode"
    );
}

#[test]
fn test_utf8_downgrade_not_none() {
    // This test documents the bug: get_builtin_documentation returns None
    // for utf8::downgrade when it should return Some(BuiltinDoc)
    let result = get_builtin_documentation("utf8::downgrade");
    assert!(
        result.is_some(),
        "get_builtin_documentation(\"utf8::downgrade\") should return Some, got None. \
         utf8::downgrade documentation is missing from get_builtin_documentation"
    );
}
