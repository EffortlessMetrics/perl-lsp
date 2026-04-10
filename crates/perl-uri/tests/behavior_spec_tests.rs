//! BDD-style behavior specification tests for `perl-uri`.
//!
//! These tests emphasize user-visible behavior at the protocol boundary:
//! callers can pass either `file://` URIs or filesystem paths and still get
//! stable, consistent normalization for cache keys and lookup surfaces.

use perl_tdd_support::{must, must_some};
use perl_uri::{is_file_uri, is_special_scheme, uri_extension, uri_key};

#[cfg(not(target_arch = "wasm32"))]
use perl_uri::{fs_path_to_uri, normalize_uri, uri_to_fs_path};

// ---------------------------------------------------------------------------
// uri_key behaviors
// ---------------------------------------------------------------------------

#[test]
fn when_uri_key_receives_windows_file_uri_then_drive_letter_is_canonicalized() {
    let key = uri_key("file:///C:/Workspace/lib.pm");
    assert_eq!(key, "file:///c:/Workspace/lib.pm");
}

#[test]
fn when_uri_key_receives_non_file_uri_then_uri_is_preserved() {
    let key = uri_key("https://example.com/docs/index.html");
    assert_eq!(key, "https://example.com/docs/index.html");
}

#[test]
fn when_uri_key_receives_invalid_uri_then_input_is_returned_as_is() {
    let key = uri_key("not a uri");
    assert_eq!(key, "not a uri");
}

// ---------------------------------------------------------------------------
// scheme classification behaviors
// ---------------------------------------------------------------------------

#[test]
fn when_scheme_is_file_then_is_file_uri_returns_true() {
    assert!(is_file_uri("file:///tmp/script.pl"));
}

#[test]
fn when_scheme_is_not_file_then_is_file_uri_returns_false() {
    assert!(!is_file_uri("untitled:Untitled-1"));
}

#[test]
fn when_scheme_is_editor_virtual_then_is_special_scheme_returns_true() {
    assert!(is_special_scheme("vscode-notebook:cell-id"));
    assert!(is_special_scheme("vscode-vfs://github/owner/repo/file.pl"));
}

#[test]
fn when_scheme_is_plain_path_then_is_special_scheme_returns_false() {
    assert!(!is_special_scheme("/tmp/plain/path.pl"));
}

// ---------------------------------------------------------------------------
// extension extraction behaviors
// ---------------------------------------------------------------------------

#[test]
fn when_uri_contains_query_or_fragment_then_extension_ignores_them() {
    assert_eq!(uri_extension("file:///tmp/module.pm?view=1#L10"), Some("pm"));
}

#[test]
fn when_uri_has_no_extension_then_extension_is_none() {
    assert_eq!(uri_extension("file:///tmp/Makefile"), None);
}

#[test]
fn when_uri_uses_multiple_dots_then_last_segment_is_extension() {
    assert_eq!(uri_extension("file:///tmp/archive.tar.gz"), Some("gz"));
}

// ---------------------------------------------------------------------------
// path <-> URI conversion behaviors (non-wasm)
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn when_absolute_path_is_converted_then_file_uri_is_produced() {
    let uri = must(fs_path_to_uri("/tmp/perl-uri-bdd/test.pl"));
    assert!(uri.starts_with("file:///"), "uri should be file URI: {uri}");
    assert!(uri.ends_with("test.pl"), "uri should contain filename: {uri}");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn when_relative_path_is_converted_then_result_is_absolute_file_uri() {
    let uri = must(fs_path_to_uri("relative/path/to/test.pm"));
    assert!(uri.starts_with("file:///"), "uri should be absolute file URI: {uri}");
    assert!(uri.contains("test.pm"), "uri should keep leaf file: {uri}");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn when_file_uri_contains_percent_encoding_then_path_is_decoded() {
    let path = must_some(uri_to_fs_path("file:///tmp/path%20with%20spaces/test.pl"));
    let lossy = path.to_string_lossy();
    assert!(lossy.contains("path with spaces"), "decoded path expected, got: {lossy}");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn when_uri_scheme_is_not_file_then_uri_to_fs_path_returns_none() {
    assert!(uri_to_fs_path("https://example.com/not-local.pl").is_none());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn when_uri_is_normalized_from_raw_path_then_output_is_file_uri() {
    let normalized = normalize_uri("/tmp/perl-uri-bdd/raw-path-script.pl");
    assert!(normalized.starts_with("file:///"), "expected file URI, got: {normalized}");
    assert!(normalized.ends_with("raw-path-script.pl"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn when_uri_is_special_scheme_then_normalize_uri_preserves_it() {
    let normalized = normalize_uri("untitled:Scratch-1");
    assert_eq!(normalized, "untitled:Scratch-1");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn when_path_roundtrips_through_uri_then_leaf_name_is_stable() {
    let original = "/tmp/perl-uri-bdd/roundtrip/demo.pl";
    let uri = must(fs_path_to_uri(original));
    let back = must_some(uri_to_fs_path(&uri));

    let name = back.file_name().and_then(|n| n.to_str()).map(ToOwned::to_owned);

    assert_eq!(name.as_deref(), Some("demo.pl"));
}
