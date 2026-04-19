//! Unit tests for context-sensitive builtin documentation strings.
//!
//! These tests guard the factual accuracy of the 11 builtin descriptions
//! updated in issue #2361. They call `get_builtin_documentation` directly
//! so failures are immediate and pinpoint the exact string, not a
//! hover-position-lookup that can silently pass when hover returns null.

use perl_semantic_analyzer::analysis::semantic::{
    get_builtin_documentation, get_operator_documentation,
};
use perl_tdd_support::must_some;

// ---------------------------------------------------------------------------
// splice
// ---------------------------------------------------------------------------

#[test]
fn test_splice_scalar_context_doc() {
    let doc = must_some(get_builtin_documentation("splice"));
    assert!(
        doc.description.contains("scalar context"),
        "splice description must mention scalar context behavior: {}",
        doc.description
    );
    assert!(
        doc.description.contains("last removed element"),
        "splice scalar context must clarify it returns the last removed element: {}",
        doc.description
    );
}

// ---------------------------------------------------------------------------
// sort
// ---------------------------------------------------------------------------

#[test]
fn test_sort_scalar_context_doc() {
    let doc = must_some(get_builtin_documentation("sort"));
    assert!(
        doc.description.contains("scalar context"),
        "sort description must warn about scalar context: {}",
        doc.description
    );
    // Must say undef, not "count" — sort in scalar context returns undef in Perl 5
    assert!(
        doc.description.contains("undef"),
        "sort scalar context must say it returns undef (not count): {}",
        doc.description
    );
    assert!(
        !doc.description.contains("or count"),
        "sort description must not claim scalar context returns a count: {}",
        doc.description
    );
}

// ---------------------------------------------------------------------------
// map
// ---------------------------------------------------------------------------

#[test]
fn test_map_scalar_context_doc() {
    let doc = must_some(get_builtin_documentation("map"));
    assert!(
        doc.description.contains("scalar context"),
        "map description must mention scalar context behavior: {}",
        doc.description
    );
}

// ---------------------------------------------------------------------------
// grep
// ---------------------------------------------------------------------------

#[test]
fn test_grep_scalar_context_doc() {
    let doc = must_some(get_builtin_documentation("grep"));
    assert!(
        doc.description.contains("scalar context"),
        "grep description must mention scalar context count behavior: {}",
        doc.description
    );
    assert!(
        doc.description.contains("number of matching"),
        "grep scalar context must explain it returns a count: {}",
        doc.description
    );
}

// ---------------------------------------------------------------------------
// wantarray
// ---------------------------------------------------------------------------

#[test]
fn test_wantarray_void_context_doc() {
    let doc = must_some(get_builtin_documentation("wantarray"));
    assert!(
        doc.description.contains("void context"),
        "wantarray description must mention void context returning undef: {}",
        doc.description
    );
    assert!(
        doc.description.contains("scalar context"),
        "wantarray description must mention scalar context (false): {}",
        doc.description
    );
    assert!(
        doc.description.contains("undef"),
        "wantarray description must mention undef for void context: {}",
        doc.description
    );
}

// ---------------------------------------------------------------------------
// file test operators
// ---------------------------------------------------------------------------

#[test]
fn test_file_test_operator_docs() {
    let cases = [
        ("-e", "exists"),
        ("-f", "plain file"),
        ("-d", "directory"),
        ("-r", "readable"),
        ("-w", "writable"),
        ("-M", "program start"),
        ("-A", "last access"),
        ("-C", "inode change"),
    ];

    for (op, expected) in cases {
        let doc = must_some(get_operator_documentation(op));
        assert!(
            doc.description.contains(expected),
            "{op} description should mention {expected}: {}",
            doc.description
        );
        assert!(
            !doc.signature.is_empty(),
            "{op} should have a non-empty signature"
        );
    }
}

// ---------------------------------------------------------------------------
// keys — factual accuracy: scalar keys returns COUNT, not bucket ratio
// ---------------------------------------------------------------------------

#[test]
fn test_keys_scalar_context_doc() {
    let doc = must_some(get_builtin_documentation("keys"));
    assert!(
        doc.description.contains("scalar context"),
        "keys description must mention scalar context: {}",
        doc.description
    );
    // Must not mention the old bucket-ratio behavior (Perl 5.26+ removed this)
    assert!(
        !doc.description.contains("3/8"),
        "keys description must not reference the old bucket-ratio format (Perl 5.26+ removed it): {}",
        doc.description
    );
    assert!(
        !doc.description.contains("used/total buckets"),
        "keys description must not mention used/total buckets (Perl 5.26+ fixed this): {}",
        doc.description
    );
    // Must say "number of keys" or equivalent count language
    assert!(
        doc.description.contains("number of keys")
            || doc.description.contains("integer count")
            || doc.description.contains("count"),
        "keys scalar context must describe returning a count: {}",
        doc.description
    );
}

// ---------------------------------------------------------------------------
// values
// ---------------------------------------------------------------------------

#[test]
fn test_values_scalar_context_doc() {
    let doc = must_some(get_builtin_documentation("values"));
    assert!(
        doc.description.contains("scalar context"),
        "values description must mention scalar context: {}",
        doc.description
    );
    assert!(
        doc.description.contains("number of"),
        "values scalar context must explain it returns a count: {}",
        doc.description
    );
}

// ---------------------------------------------------------------------------
// each — iterator reset must mention keys() as a reset trigger
// ---------------------------------------------------------------------------

#[test]
fn test_each_iterator_reset_doc() {
    let doc = must_some(get_builtin_documentation("each"));
    assert!(
        doc.description.contains("resets") || doc.description.contains("reset"),
        "each description must mention iterator reset behavior: {}",
        doc.description
    );
    // The critical Perl gotcha: calling keys() resets each's iterator
    assert!(
        doc.description.contains("keys()") || doc.description.contains("keys"),
        "each description must mention that calling keys() resets the iterator: {}",
        doc.description
    );
    assert!(
        doc.description.contains("while"),
        "each description must show the canonical while-loop usage pattern: {}",
        doc.description
    );
}

// ---------------------------------------------------------------------------
// stat
// ---------------------------------------------------------------------------

#[test]
fn test_stat_doc() {
    let doc = must_some(get_builtin_documentation("stat"));
    assert!(
        doc.description.contains("13"),
        "stat description must mention the 13-element return list: {}",
        doc.description
    );
    // Must mention failure case
    assert!(
        doc.description.contains("empty list")
            || doc.description.contains("failure")
            || doc.description.contains("on failure"),
        "stat description must mention empty list on failure: {}",
        doc.description
    );
}

// ---------------------------------------------------------------------------
// caller
// ---------------------------------------------------------------------------

#[test]
fn test_caller_scalar_context_doc() {
    let doc = must_some(get_builtin_documentation("caller"));
    assert!(
        doc.description.contains("scalar context"),
        "caller description must mention scalar context (returns package name): {}",
        doc.description
    );
    assert!(
        doc.description.contains("package"),
        "caller description must mention it returns the package name in scalar context: {}",
        doc.description
    );
}

// ---------------------------------------------------------------------------
// gmtime
// ---------------------------------------------------------------------------

#[test]
fn test_gmtime_context_doc() {
    let doc = must_some(get_builtin_documentation("gmtime"));
    assert!(
        doc.description.contains("scalar context"),
        "gmtime description must mention scalar context string form: {}",
        doc.description
    );
    assert!(
        doc.description.contains("list context"),
        "gmtime description must mention list context 9-element form: {}",
        doc.description
    );
    // Check for the ctime-style description
    assert!(
        doc.description.contains("ctime") || doc.description.contains("string"),
        "gmtime scalar context must mention ctime-style string: {}",
        doc.description
    );
}

// ---------------------------------------------------------------------------
// Cross-check: no builtin should have the old bucket-ratio wording
// ---------------------------------------------------------------------------

#[test]
fn test_no_builtin_has_stale_bucket_ratio_wording() {
    // The '3/8' bucket ratio format was true pre-5.26 for %hash in scalar context.
    // It has never been true for `keys` directly. Guard against it creeping back.
    for name in &["keys", "values", "each", "exists", "delete"] {
        if let Some(doc) = get_builtin_documentation(name) {
            assert!(
                !doc.description.contains("3/8"),
                "Builtin '{}' must not contain stale '3/8' bucket-ratio wording: {}",
                name,
                doc.description
            );
        }
    }
}
