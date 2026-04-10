//! Tests for duplicate hash key detection diagnostic (PL408)
//!
//! These tests cover:
//! - Hash literals with duplicate keys (should warn)
//! - Hash literals with all unique keys (should not warn)
//! - Hash refs with duplicate keys (should warn)
//! - Dynamic / variable keys (should not warn — cannot be statically determined)

use std::sync::Arc;

use perl_lsp_diagnostics::{Diagnostic, DiagnosticsProvider};
use perl_parser::Parser;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let output = Parser::new(source).parse_with_recovery();
    let ast = Arc::new(output.ast);
    let provider = DiagnosticsProvider::new(&ast, source.to_string());
    provider.get_diagnostics(&ast, &output.diagnostics, source, None)
}

fn dup_key_diags(source: &str) -> Vec<Diagnostic> {
    diagnostics_for(source).into_iter().filter(|d| d.code.as_deref() == Some("PL408")).collect()
}

// --- Should fire ---

#[test]
fn hash_variable_fat_arrow_duplicate_key_fires_pl407() {
    // my %h = (a => 1, b => 2, a => 3);  -- duplicate key 'a'
    let diags = dup_key_diags("my %h = (a => 1, b => 2, a => 3);\n");
    assert_eq!(diags.len(), 1, "duplicate key 'a' should fire PL408 once");
    assert!(
        diags[0].message.contains("'a'"),
        "message should name the duplicate key: {}",
        diags[0].message
    );
}

#[test]
fn hash_ref_fat_arrow_duplicate_key_fires_pl407() {
    // my $h = { a => 1, b => 2, a => 3 };  -- duplicate key 'a'
    let diags = dup_key_diags("my $h = { a => 1, b => 2, a => 3 };\n");
    assert_eq!(diags.len(), 1, "duplicate key 'a' in hash ref should fire PL408 once");
    assert!(
        diags[0].message.contains("'a'"),
        "message should name the duplicate key: {}",
        diags[0].message
    );
}

#[test]
fn hash_variable_string_key_duplicate_fires_pl407() {
    // my %h = ('host' => 'localhost', 'port' => 3306, 'host' => '127.0.0.1');
    let diags =
        dup_key_diags("my %h = ('host' => 'localhost', 'port' => 3306, 'host' => '127.0.0.1');\n");
    assert_eq!(diags.len(), 1, "duplicate string key 'host' should fire PL408 once");
    assert!(
        diags[0].message.contains("host"),
        "message should name the duplicate key: {}",
        diags[0].message
    );
}

#[test]
fn hash_variable_multiple_duplicates_fires_pl407_multiple_times() {
    // my %h = (a => 1, b => 2, a => 3, b => 4);  -- both 'a' and 'b' duplicated
    let diags = dup_key_diags("my %h = (a => 1, b => 2, a => 3, b => 4);\n");
    assert_eq!(diags.len(), 2, "two distinct duplicate keys should fire PL408 twice");
}

#[test]
fn hash_triply_duplicated_key_fires_pl407_twice() {
    // my %h = (a => 1, a => 2, a => 3);  -- 'a' appears 3 times: 2 extra
    let diags = dup_key_diags("my %h = (a => 1, a => 2, a => 3);\n");
    // Each occurrence after the first should produce a diagnostic
    assert_eq!(diags.len(), 2, "triple key 'a' should fire PL408 twice (2nd and 3rd occurrences)");
}

// --- Should NOT fire ---

#[test]
fn hash_all_unique_keys_no_pl407() {
    let diags = dup_key_diags("my %h = (a => 1, b => 2, c => 3);\n");
    assert!(diags.is_empty(), "unique keys must not fire PL408");
}

#[test]
fn hash_ref_all_unique_no_pl407() {
    let diags = dup_key_diags("my $h = { a => 1, b => 2 };\n");
    assert!(diags.is_empty(), "unique hash ref keys must not fire PL408");
}

#[test]
fn empty_hash_no_pl407() {
    let diags = dup_key_diags("my %h = ();\n");
    assert!(diags.is_empty(), "empty hash must not fire PL408");
}

#[test]
fn single_pair_hash_no_pl407() {
    let diags = dup_key_diags("my %h = (a => 1);\n");
    assert!(diags.is_empty(), "single-pair hash must not fire PL408");
}

#[test]
fn hash_with_variable_keys_no_pl407() {
    // Variable keys cannot be statically compared — must not produce false positives
    let diags = dup_key_diags("my %h = ($key => 1, $other => 2);\n");
    assert!(diags.is_empty(), "variable keys must not fire PL408 (cannot be compared statically)");
}

#[test]
fn hash_with_mixed_static_variable_no_false_positive() {
    // One static key, one variable — no duplicate can be asserted
    let diags = dup_key_diags("my %h = (a => 1, $key => 2);\n");
    assert!(diags.is_empty(), "mixed static+variable keys must not fire PL408");
}
