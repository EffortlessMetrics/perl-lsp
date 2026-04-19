//! Snapshot tests for `PerlLanguage` descriptor outputs.
//!
//! These tests capture the structured output of `PerlLanguage` methods:
//! - `node_kind_names()` returns the complete alphabetically-sorted list of kind names
//! - `node_kind_count()` returns the count of kinds
//!
//! Run `cargo insta review` to update snapshots when the output changes intentionally.

use tree_sitter_perl_rs::{language, LANGUAGE};

/// Snapshot the complete list of node kind names.
/// This is the primary structured output of `PerlLanguage::node_kind_names()`.
#[test]
fn snapshot_perl_language_node_kind_names() {
    let lang = language();
    let names = lang.node_kind_names();
    // Join with newlines for human-readable snapshot output
    let joined = names.join("\n");
    insta::assert_snapshot!("perl_language_node_kind_names", joined);
}

/// Snapshot the node kind count value.
#[test]
fn snapshot_perl_language_node_kind_count() {
    let lang = language();
    let count = lang.node_kind_count();
    insta::assert_snapshot!("perl_language_node_kind_count", count);
}

/// Snapshot `node_kind_names()` from the LANGUAGE constant.
#[test]
fn snapshot_language_constant_node_kind_names() {
    let names = LANGUAGE.node_kind_names();
    let joined = names.join("\n");
    insta::assert_snapshot!("language_constant_node_kind_names", joined);
}

/// Snapshot `node_kind_count()` from the LANGUAGE constant.
#[test]
fn snapshot_language_constant_node_kind_count() {
    let count = LANGUAGE.node_kind_count();
    insta::assert_snapshot!("language_constant_node_kind_count", count);
}

/// Snapshot `node_kind_is_named()` results for known kinds.
/// This captures the boolean query output for specific kind names.
#[test]
fn snapshot_perl_language_is_named_results() {
    let lang = language();
    let results = [
        ("Program", lang.node_kind_is_named("Program")),
        ("Subroutine", lang.node_kind_is_named("Subroutine")),
        ("Variable", lang.node_kind_is_named("Variable")),
        ("source_file", lang.node_kind_is_named("source_file")),
        ("__nonexistent__", lang.node_kind_is_named("__nonexistent__")),
        ("", lang.node_kind_is_named("")),
    ];
    let formatted = results
        .iter()
        .map(|(kind, is_named)| format!("{}: {}", kind, is_named))
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!("perl_language_is_named_results", formatted);
}
