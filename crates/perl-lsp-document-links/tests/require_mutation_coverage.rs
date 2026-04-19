//! Mutation-hardening tests for `require` parsing branches in document link extraction.
//!
//! These tests target boolean branch mutations in `compute_links` for:
//! - module-form `require` filtering
//! - quoted file `require` extraction
//! - malformed quote handling

use perl_lsp_document_links::compute_links;
use serde_json::Value;

const URI: &str = "file:///test.pl";

fn data_type(link: &Value) -> Option<&str> {
    link.pointer("/data/type").and_then(Value::as_str)
}

#[test]
fn require_double_quoted_module_path_emits_only_file_link() {
    let links = compute_links(URI, r#"require \"Foo/Bar.pm\";"#, &[]);
    assert_eq!(
        links.len(),
        1,
        "quoted require should produce exactly one file link"
    );
    assert_eq!(data_type(&links[0]), Some("file"));
}

#[test]
fn require_single_quoted_value_with_colons_emits_only_file_link() {
    let links = compute_links(URI, "require 'Foo::Bar';", &[]);
    assert_eq!(
        links.len(),
        1,
        "quoted require should not be interpreted as module-form require"
    );
    assert_eq!(data_type(&links[0]), Some("file"));
}

#[test]
fn require_module_form_with_colons_emits_module_link() {
    let links = compute_links(URI, "require Foo::Bar::Baz;", &[]);
    assert_eq!(links.len(), 1, "module-form require should emit one link");
    assert_eq!(data_type(&links[0]), Some("module"));
    assert_eq!(
        links[0].pointer("/data/module").and_then(Value::as_str),
        Some("Foo::Bar::Baz")
    );
}

#[test]
fn require_quoted_without_closing_quote_emits_no_link() {
    let links = compute_links(URI, "require 'Foo/Bar.pm;", &[]);
    assert!(
        links.is_empty(),
        "unterminated quoted require must not emit a partial link"
    );
}

#[test]
fn require_numeric_version_emits_no_link() {
    let links = compute_links(URI, "require 5.038;", &[]);
    assert!(
        links.is_empty(),
        "version-only require should not emit module or file links"
    );
}
