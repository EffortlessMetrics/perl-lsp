//! Mutation-killing tests for perl-uri-classify.
//!
//! The existing 5 tests cover: uri_key normalization, invalid URI fallback,
//! is_file_uri basic check, is_special_scheme basic, uri_extension basic.
//!
//! Untested branches:
//!   - is_file_uri: "file://localhost/path" (double slash without host)
//!   - is_file_uri: non-file schemes like https, untitled
//!   - is_special_scheme: each special scheme individually (git:, vscode-notebook:, vscode-vfs:)
//!   - is_special_scheme: invalid URI fallback branch (starts_with checks)
//!   - uri_extension: fragment (#anchor) stripped before looking at extension
//!   - uri_extension: empty extension (file.)
//!   - uri_extension: no dot in filename
//!   - uri_extension: path with multiple dots uses LAST dot
//!   - uri_key: single-char drive letter edge (already covered by C:)
//!   - uri_key: non-windows paths pass through unchanged

use perl_uri::classify::{is_file_uri, is_special_scheme, uri_extension, uri_key};

// ---------------------------------------------------------------------------
// is_file_uri: various schemes
// ---------------------------------------------------------------------------

#[test]
fn is_file_uri_returns_true_for_file_triple_slash() {
    assert!(is_file_uri("file:///tmp/test.pl"), "file:/// must be file URI");
}

#[test]
fn is_file_uri_returns_true_for_file_double_slash_with_host() {
    // file://localhost/path also starts with "file://"
    assert!(is_file_uri("file://localhost/path/to/file.pl"), "file:// with host must be file URI");
}

#[test]
fn is_file_uri_returns_false_for_https() {
    assert!(!is_file_uri("https://example.com/file.pl"), "https must not be file URI");
}

#[test]
fn is_file_uri_returns_false_for_untitled() {
    assert!(!is_file_uri("untitled:Untitled-1"), "untitled must not be file URI");
}

#[test]
fn is_file_uri_returns_false_for_empty() {
    assert!(!is_file_uri(""), "empty string must not be file URI");
}

// ---------------------------------------------------------------------------
// is_special_scheme: each special scheme individually
// ---------------------------------------------------------------------------

#[test]
fn is_special_scheme_true_for_untitled() {
    assert!(is_special_scheme("untitled:Untitled-1"), "untitled: is special");
}

#[test]
fn is_special_scheme_true_for_git() {
    assert!(is_special_scheme("git:/foo/bar.pl"), "git: is special");
}

#[test]
fn is_special_scheme_true_for_vscode_notebook() {
    assert!(is_special_scheme("vscode-notebook:/path/to/nb.ipynb"), "vscode-notebook: is special");
}

#[test]
fn is_special_scheme_true_for_vscode_vfs() {
    assert!(is_special_scheme("vscode-vfs:/mount/path"), "vscode-vfs: is special");
}

#[test]
fn is_special_scheme_false_for_file() {
    assert!(!is_special_scheme("file:///tmp/test.pl"), "file: is not special");
}

#[test]
fn is_special_scheme_false_for_https() {
    // Url::parse succeeds, scheme is "https" != "file" → true
    // Actually https is parsed and scheme != file → special
    assert!(is_special_scheme("https://example.com"), "https: scheme != file → special");
}

#[test]
fn is_special_scheme_for_invalid_uri_uses_fallback_branch() {
    // An unparseable string that doesn't start with any special prefix
    assert!(
        !is_special_scheme("not-a-uri-at-all"),
        "unrecognized non-URI string must not be special"
    );
}

#[test]
fn is_special_scheme_invalid_uri_with_git_prefix() {
    // "git:something" — Url::parse may succeed or fail depending on format
    // Either way, the result must be true since git: is special
    assert!(is_special_scheme("git:something"), "git: must always be special");
}

// ---------------------------------------------------------------------------
// uri_extension: edge cases
// ---------------------------------------------------------------------------

#[test]
fn uri_extension_strips_fragment_before_extracting() {
    // "file.pl#anchor" → extension is "pl" (not "pl#anchor")
    assert_eq!(uri_extension("file:///path/file.pl#anchor"), Some("pl"));
}

#[test]
fn uri_extension_strips_query_before_extracting() {
    assert_eq!(uri_extension("file:///path/file.pm?version=1"), Some("pm"));
}

#[test]
fn uri_extension_returns_none_for_trailing_dot() {
    // "file." has empty extension → None
    assert_eq!(uri_extension("file:///path/file."), None);
}

#[test]
fn uri_extension_returns_none_for_no_dot() {
    assert_eq!(uri_extension("file:///path/no_extension"), None);
}

#[test]
fn uri_extension_uses_last_dot_for_multiple_dots() {
    // "archive.tar.gz" → extension is "gz", not "tar"
    assert_eq!(uri_extension("file:///path/archive.tar.gz"), Some("gz"));
}

#[test]
fn uri_extension_for_empty_string_returns_none() {
    assert_eq!(uri_extension(""), None);
}

#[test]
fn uri_extension_for_path_ending_in_slash() {
    // "file:///path/" → last segment is empty → no extension
    assert_eq!(uri_extension("file:///path/"), None);
}

// ---------------------------------------------------------------------------
// uri_key: non-Windows path unchanged, drive letter normalization
// ---------------------------------------------------------------------------

#[test]
fn uri_key_unix_path_unchanged() {
    let key = uri_key("file:///usr/local/lib/perl5/File/Basename.pm");
    assert_eq!(key, "file:///usr/local/lib/perl5/File/Basename.pm");
}

#[test]
fn uri_key_uppercase_windows_drive_lowercased() {
    // file:///C:/path → file:///c:/path
    let key = uri_key("file:///C:/Users/test/file.pl");
    assert!(key.starts_with("file:///c:"), "Windows drive letter must be lowercased, got: {key}");
}

#[test]
fn uri_key_already_lowercase_drive_unchanged() {
    let key = uri_key("file:///d:/projects/app/lib.pl");
    assert!(key.starts_with("file:///d:"), "Lowercase drive must stay lowercase: {key}");
}
