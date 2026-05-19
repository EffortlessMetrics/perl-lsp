use super::*;
use perl_parser::Parser;
use perl_tdd_support::{must, must_some};

fn dup_key_diags(source: &str) -> Vec<Diagnostic> {
    let ast = must(Parser::new(source).parse());
    let mut diags = Vec::new();
    check_duplicate_hash_keys(&ast, &mut diags);
    diags
}

fn has_pl408(diags: &[Diagnostic]) -> bool {
    diags.iter().any(|d| d.code.as_deref() == Some("PL408"))
}

#[test]
fn duplicate_string_key_is_flagged() {
    let diags = dup_key_diags(r#"my %h = (foo => 1, foo => 2);"#);
    assert!(has_pl408(&diags), "duplicate string key 'foo' should be flagged as PL408: {diags:?}");
}

#[test]
fn unique_keys_not_flagged() {
    let diags = dup_key_diags(r#"my %h = (foo => 1, bar => 2, baz => 3);"#);
    assert!(!has_pl408(&diags), "unique keys should not be flagged: {diags:?}");
}

#[test]
fn three_occurrences_two_diagnostics() {
    let diags = dup_key_diags(r#"my %h = (x => 1, x => 2, x => 3);"#);
    let count = diags.iter().filter(|d| d.code.as_deref() == Some("PL408")).count();
    assert_eq!(
        count, 2,
        "three occurrences of same key should produce two PL408 diagnostics: {diags:?}"
    );
}

#[test]
fn duplicate_numeric_key_is_flagged() {
    let diags = dup_key_diags(r#"my %h = (1 => "a", 1 => "b");"#);
    assert!(has_pl408(&diags), "duplicate numeric key should be flagged: {diags:?}");
}

#[test]
fn dynamic_variable_key_not_flagged() {
    let diags = dup_key_diags(r#"my $k = "foo"; my %h = ($k => 1, $k => 2);"#);
    assert!(!has_pl408(&diags), "dynamic variable keys should not be flagged: {diags:?}");
}

#[test]
fn duplicate_message_names_the_key() {
    let diags = dup_key_diags(r#"my %h = (alpha => 1, alpha => 2);"#);
    let diag = must_some(diags.iter().find(|d| d.code.as_deref() == Some("PL408")));
    assert!(
        diag.message.contains("alpha"),
        "PL408 message should name the duplicate key: {}",
        diag.message
    );
}

#[test]
fn duplicate_diagnostic_has_related_info_for_first_occurrence() {
    let diags = dup_key_diags(r#"my %h = (name => "Alice", name => "Bob");"#);
    let diag = must_some(diags.iter().find(|d| d.code.as_deref() == Some("PL408")));
    assert!(
        !diag.related_information.is_empty(),
        "PL408 should include related information pointing to first occurrence"
    );
}

#[test]
fn nested_hash_inner_duplicate_flagged() {
    let diags = dup_key_diags(r#"my %outer = (inner => { x => 1, x => 2 });"#);
    assert!(has_pl408(&diags), "duplicate key inside nested hash ref should be flagged: {diags:?}");
}

#[test]
fn empty_hash_not_flagged() {
    let diags = dup_key_diags(r#"my %h = ();"#);
    assert!(!has_pl408(&diags), "empty hash should not be flagged: {diags:?}");
}

#[test]
fn single_pair_hash_not_flagged() {
    let diags = dup_key_diags(r#"my %h = (key => "value");"#);
    assert!(!has_pl408(&diags), "single-pair hash should not be flagged: {diags:?}");
}
