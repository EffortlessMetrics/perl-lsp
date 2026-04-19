//! Mutation-killing tests for perl-lsp-document-links: is_pragma coverage.
//!
//! The existing inline tests cover: strict, warnings, feature, parent, base.
//! The is_pragma function has 28 arms — the other 23 are never tested.
//!
//! These tests verify that every pragma arm returns no link via compute_links,
//! killing any mutant that removes one of the arms from the pragma list.

use perl_lsp_document_links::compute_links;

const URI: &str = "file:///test.pl";

// ---------------------------------------------------------------------------
// is_pragma: all arms must suppress document links
// ---------------------------------------------------------------------------

#[test]
fn pragma_utf8_emits_no_link() {
    let links = compute_links(URI, "use utf8;\n", &[]);
    assert!(links.is_empty(), "utf8 is a pragma");
}

#[test]
fn pragma_bytes_emits_no_link() {
    let links = compute_links(URI, "use bytes;\n", &[]);
    assert!(links.is_empty(), "bytes is a pragma");
}

#[test]
fn pragma_integer_emits_no_link() {
    let links = compute_links(URI, "use integer;\n", &[]);
    assert!(links.is_empty(), "integer is a pragma");
}

#[test]
fn pragma_constant_emits_no_link() {
    let links = compute_links(URI, "use constant PI => 3.14;\n", &[]);
    assert!(links.is_empty(), "constant is a pragma");
}

#[test]
fn pragma_lib_emits_no_link() {
    let links = compute_links(URI, "use lib './lib';\n", &[]);
    assert!(links.is_empty(), "lib is a pragma");
}

#[test]
fn pragma_vars_emits_no_link() {
    let links = compute_links(URI, "use vars qw($x);\n", &[]);
    assert!(links.is_empty(), "vars is a pragma");
}

#[test]
fn pragma_subs_emits_no_link() {
    let links = compute_links(URI, "use subs qw(foo);\n", &[]);
    assert!(links.is_empty(), "subs is a pragma");
}

#[test]
fn pragma_overload_emits_no_link() {
    let links = compute_links(URI, "use overload '+' => \\&add;\n", &[]);
    assert!(links.is_empty(), "overload is a pragma");
}

#[test]
fn pragma_fields_emits_no_link() {
    let links = compute_links(URI, "use fields qw(name age);\n", &[]);
    assert!(links.is_empty(), "fields is a pragma");
}

#[test]
fn pragma_if_emits_no_link() {
    let links = compute_links(URI, "use if $] >= 5.016, 'feature', 'say';\n", &[]);
    assert!(links.is_empty(), "if is a pragma");
}

#[test]
fn pragma_attributes_emits_no_link() {
    let links = compute_links(URI, "use attributes;\n", &[]);
    assert!(links.is_empty(), "attributes is a pragma");
}

#[test]
fn pragma_autouse_emits_no_link() {
    let links = compute_links(URI, "use autouse;\n", &[]);
    assert!(links.is_empty(), "autouse is a pragma");
}

#[test]
fn pragma_autodie_emits_no_link() {
    let links = compute_links(URI, "use autodie;\n", &[]);
    assert!(links.is_empty(), "autodie is a pragma");
}

#[test]
fn pragma_bigint_emits_no_link() {
    let links = compute_links(URI, "use bigint;\n", &[]);
    assert!(links.is_empty(), "bigint is a pragma");
}

#[test]
fn pragma_bignum_emits_no_link() {
    let links = compute_links(URI, "use bignum;\n", &[]);
    assert!(links.is_empty(), "bignum is a pragma");
}

#[test]
fn pragma_bigrat_emits_no_link() {
    let links = compute_links(URI, "use bigrat;\n", &[]);
    assert!(links.is_empty(), "bigrat is a pragma");
}

#[test]
fn pragma_blib_emits_no_link() {
    let links = compute_links(URI, "use blib;\n", &[]);
    assert!(links.is_empty(), "blib is a pragma");
}

#[test]
fn pragma_charnames_emits_no_link() {
    let links = compute_links(URI, "use charnames ':full';\n", &[]);
    assert!(links.is_empty(), "charnames is a pragma");
}

#[test]
fn pragma_diagnostics_emits_no_link() {
    let links = compute_links(URI, "use diagnostics;\n", &[]);
    assert!(links.is_empty(), "diagnostics is a pragma");
}

#[test]
fn pragma_encoding_emits_no_link() {
    let links = compute_links(URI, "use encoding 'utf8';\n", &[]);
    assert!(links.is_empty(), "encoding is a pragma");
}

#[test]
fn pragma_filetest_emits_no_link() {
    let links = compute_links(URI, "use filetest 'access';\n", &[]);
    assert!(links.is_empty(), "filetest is a pragma");
}

#[test]
fn pragma_locale_emits_no_link() {
    let links = compute_links(URI, "use locale;\n", &[]);
    assert!(links.is_empty(), "locale is a pragma");
}

#[test]
fn pragma_open_emits_no_link() {
    let links = compute_links(URI, "use open ':utf8';\n", &[]);
    assert!(links.is_empty(), "open is a pragma");
}

#[test]
fn pragma_ops_emits_no_link() {
    let links = compute_links(URI, "use ops;\n", &[]);
    assert!(links.is_empty(), "ops is a pragma");
}

#[test]
fn pragma_re_emits_no_link() {
    let links = compute_links(URI, "use re 'strict';\n", &[]);
    assert!(links.is_empty(), "re is a pragma");
}

#[test]
fn pragma_sigtrap_emits_no_link() {
    let links = compute_links(URI, "use sigtrap;\n", &[]);
    assert!(links.is_empty(), "sigtrap is a pragma");
}

#[test]
fn pragma_sort_emits_no_link() {
    let links = compute_links(URI, "use sort 'stable';\n", &[]);
    assert!(links.is_empty(), "sort is a pragma");
}

#[test]
fn pragma_threads_emits_no_link() {
    let links = compute_links(URI, "use threads;\n", &[]);
    assert!(links.is_empty(), "threads is a pragma");
}

#[test]
fn pragma_vmsish_emits_no_link() {
    let links = compute_links(URI, "use vmsish;\n", &[]);
    assert!(links.is_empty(), "vmsish is a pragma");
}

// ---------------------------------------------------------------------------
// Non-pragma use statements that look similar: verify they DO emit links
// ---------------------------------------------------------------------------

#[test]
fn non_pragma_module_emits_link() {
    // "Strict" (capital S) is NOT the pragma "strict" → must emit a link
    let links = compute_links(URI, "use Strict::More;\n", &[]);
    assert!(
        !links.is_empty(),
        "Strict::More is not a pragma and must emit a link"
    );
}

#[test]
fn empty_document_emits_no_links() {
    let links = compute_links(URI, "", &[]);
    assert!(links.is_empty(), "empty document must produce no links");
}

#[test]
fn document_with_only_code_emits_no_links() {
    let links = compute_links(URI, "my $x = 1;\nprint $x;\n", &[]);
    assert!(
        links.is_empty(),
        "document with no imports must produce no links"
    );
}
