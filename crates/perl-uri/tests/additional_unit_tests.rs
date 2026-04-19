//! Additional unit tests for the `perl-uri` crate.
//!
//! Covers: deeper edge cases, boundary conditions, cross-function
//! consistency, and platform-specific behavior not exercised by the
//! existing test suite.

use perl_uri::{is_file_uri, is_special_scheme, uri_extension, uri_key};

// ── uri_key: additional edge cases ──────────────────────────────────

#[test]
fn uri_key_empty_path_after_file_prefix() {
    // file:/// with nothing after the triple slash
    let key = uri_key("file:///");
    assert_eq!(key, "file:///");
}

#[test]
fn uri_key_single_char_path() {
    let key = uri_key("file:///x");
    assert_eq!(key, "file:///x");
}

#[test]
fn uri_key_windows_drive_preserves_rest_of_path() {
    let key = uri_key("file:///E:/deep/nested/path/module.pm");
    assert_eq!(key, "file:///e:/deep/nested/path/module.pm");
}

#[test]
fn uri_key_windows_drive_with_query_and_fragment() {
    let key = uri_key("file:///Z:/dir/file.pl?v=2#L99");
    assert!(key.starts_with("file:///z:"));
    assert!(key.contains("v=2"));
    assert!(key.contains("#L99"));
}

#[test]
fn uri_key_already_lowercase_windows_drive() {
    let key = uri_key("file:///d:/foo/bar.pm");
    assert_eq!(key, "file:///d:/foo/bar.pm");
}

#[test]
fn uri_key_non_drive_colon_not_lowered() {
    // Two-letter directory that happens to have a colon later
    let key = uri_key("file:///ab/c:/file.pl");
    assert_eq!(key, "file:///ab/c:/file.pl");
}

#[test]
fn uri_key_data_uri_passthrough() {
    let input = "data:text/plain;base64,SGVsbG8=";
    let key = uri_key(input);
    assert!(key.starts_with("data:"));
}

#[test]
fn uri_key_ftp_scheme() {
    let key = uri_key("ftp://host/path/file.pl");
    assert!(key.starts_with("ftp://"));
}

#[test]
fn uri_key_with_port() {
    let key = uri_key("http://localhost:8080/index.pl");
    assert!(key.contains(":8080"));
}

#[test]
fn uri_key_with_userinfo() {
    let key = uri_key("http://user:pass@host/path");
    // Url::parse may strip or retain userinfo; just ensure no panic
    assert!(!key.is_empty());
}

// ── is_file_uri: boundary inputs ────────────────────────────────────

#[test]
fn is_file_uri_bare_scheme() {
    assert!(is_file_uri("file://"));
}

#[test]
fn is_file_uri_with_localhost_authority() {
    assert!(is_file_uri("file://localhost/tmp/test.pl"));
}

#[test]
fn is_file_uri_with_extra_slashes() {
    assert!(is_file_uri("file:////tmp/test.pl"));
}

#[test]
fn is_file_uri_with_query() {
    assert!(is_file_uri("file:///tmp/test.pl?v=1"));
}

#[test]
fn is_file_uri_with_fragment() {
    assert!(is_file_uri("file:///tmp/test.pl#L10"));
}

#[test]
fn is_file_uri_just_file_colon_no_slashes() {
    // "file:" without "//" is not matched by starts_with("file://")
    assert!(!is_file_uri("file:test.pl"));
}

// ── is_special_scheme: additional schemes ───────────────────────────

#[test]
fn is_special_scheme_ftp() {
    assert!(is_special_scheme("ftp://host/file.pl"));
}

#[test]
fn is_special_scheme_ssh() {
    assert!(is_special_scheme("ssh://host/file.pl"));
}

#[test]
fn is_special_scheme_data_uri() {
    assert!(is_special_scheme("data:text/plain,hello"));
}

#[test]
fn is_special_scheme_empty_string() {
    // Empty string is not parseable and doesn't match any known prefix
    assert!(!is_special_scheme(""));
}

#[test]
fn is_special_scheme_plain_path() {
    // A plain path like "/tmp/foo" is not parseable as URL and doesn't
    // start with any known special prefix
    assert!(!is_special_scheme("/tmp/foo.pl"));
}

#[test]
fn is_special_scheme_vscode_vfs_with_path() {
    assert!(is_special_scheme("vscode-vfs://github/owner/repo/file.pl"));
}

#[test]
fn is_special_scheme_mailto() {
    assert!(is_special_scheme("mailto:user@example.com"));
}

// ── uri_extension: more patterns ────────────────────────────────────

#[test]
fn uri_extension_single_char_ext() {
    assert_eq!(uri_extension("file:///tmp/test.t"), Some("t"));
    assert_eq!(uri_extension("file:///tmp/test.a"), Some("a"));
}

#[test]
fn uri_extension_long_ext() {
    assert_eq!(uri_extension("file:///tmp/file.psgi"), Some("psgi"));
    assert_eq!(uri_extension("file:///tmp/file.cgi"), Some("cgi"));
}

#[test]
fn uri_extension_double_extension() {
    // Returns last extension only
    assert_eq!(uri_extension("file:///tmp/file.cpan.pm"), Some("pm"));
}

#[test]
fn uri_extension_only_dot_in_filename() {
    // A filename that is just "." — rfind('.') returns 0, ext is ""
    assert_eq!(uri_extension("file:///tmp/."), None);
}

#[test]
fn uri_extension_hidden_file_with_ext() {
    assert_eq!(uri_extension("file:///tmp/.bashrc.bak"), Some("bak"));
}

#[test]
fn uri_extension_numeric_extension() {
    assert_eq!(uri_extension("file:///tmp/file.123"), Some("123"));
}

#[test]
fn uri_extension_hyphen_in_extension() {
    assert_eq!(uri_extension("file:///tmp/file.tar-gz"), Some("tar-gz"));
}

#[test]
fn uri_extension_underscore_in_ext() {
    assert_eq!(uri_extension("file:///tmp/file.my_ext"), Some("my_ext"));
}

#[test]
fn uri_extension_slash_only() {
    // URI with trailing slash — last segment is empty
    assert_eq!(uri_extension("file:///tmp/dir/"), None);
}

#[test]
fn uri_extension_just_scheme() {
    assert_eq!(uri_extension("file:///"), None);
}

#[test]
fn uri_extension_bare_filename() {
    // No slashes at all
    assert_eq!(uri_extension("test.pl"), Some("pl"));
}

#[test]
fn uri_extension_with_both_query_and_fragment_complex() {
    assert_eq!(
        uri_extension("file:///tmp/test.pm?a=1&b=2#section"),
        Some("pm")
    );
}

#[test]
fn uri_extension_percent_encoded_extension() {
    // Extension itself is percent-encoded
    assert_eq!(uri_extension("file:///tmp/test.%70%6C"), Some("%70%6C"));
}

#[test]
fn uri_extension_windows_style_path_in_uri() {
    assert_eq!(uri_extension("file:///C:/Users/dev/test.pl"), Some("pl"));
}

// ── uri_to_fs_path: additional cases ────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
mod extra_uri_to_fs_path {
    use perl_uri::uri_to_fs_path;

    #[test]
    fn root_uri() -> Result<(), String> {
        let path = uri_to_fs_path("file:///").ok_or("expected Some for root")?;
        #[cfg(windows)]
        if !path.has_root() {
            return Err(format!("expected rooted path, got: {}", path.display()));
        }
        #[cfg(not(windows))]
        if !path.to_string_lossy().starts_with('/') {
            return Err(format!("expected root path, got: {}", path.display()));
        }
        Ok(())
    }

    #[test]
    fn uri_with_localhost_authority() {
        let result = uri_to_fs_path("file://localhost/tmp/test.pl");
        // May return Some on Unix, depends on url crate behavior
        let _ = result;
    }

    #[test]
    fn percent_encoded_directory_names() -> Result<(), String> {
        let path =
            uri_to_fs_path("file:///tmp/dir%20one/dir%20two/test.pl").ok_or("expected Some")?;
        let s = path.to_string_lossy();
        if !s.contains("dir one") || !s.contains("dir two") {
            return Err(format!("encoded dirs not decoded: {s}"));
        }
        Ok(())
    }

    #[test]
    fn uri_with_tilde() -> Result<(), String> {
        let path = uri_to_fs_path("file:///home/~user/test.pl").ok_or("expected Some")?;
        let s = path.to_string_lossy();
        if !s.contains("~user") {
            return Err(format!("tilde not preserved: {s}"));
        }
        Ok(())
    }

    #[test]
    fn uri_with_dot_segments() {
        // "." and ".." segments in the URI
        let path = uri_to_fs_path("file:///tmp/a/../b/./c.pl");
        if let Some(p) = path {
            // The url crate may or may not normalize dot segments
            let s = p.to_string_lossy();
            assert!(s.contains("c.pl"));
        }
    }

    #[test]
    fn uri_with_empty_segments() -> Result<(), String> {
        // Double slashes create empty segments
        let path = uri_to_fs_path("file:///tmp//double//slash.pl").ok_or("expected Some")?;
        let s = path.to_string_lossy();
        if !s.contains("slash.pl") {
            return Err(format!("unexpected path: {s}"));
        }
        Ok(())
    }

    #[test]
    fn uri_with_plus_sign() -> Result<(), String> {
        // '+' in URIs is NOT space; it should remain a literal '+'
        let path = uri_to_fs_path("file:///tmp/a+b.pl").ok_or("expected Some")?;
        let s = path.to_string_lossy();
        if !s.contains("a+b") {
            return Err(format!("plus sign not preserved: {s}"));
        }
        Ok(())
    }

    #[test]
    fn uri_with_parentheses() -> Result<(), String> {
        let path = uri_to_fs_path("file:///tmp/dir(1)/test.pl").ok_or("expected Some")?;
        let s = path.to_string_lossy();
        if !s.contains("dir(1)") {
            return Err(format!("parens not preserved: {s}"));
        }
        Ok(())
    }

    #[test]
    fn uri_with_at_sign() -> Result<(), String> {
        let path = uri_to_fs_path("file:///tmp/user%40host/test.pl").ok_or("expected Some")?;
        let s = path.to_string_lossy();
        if !s.contains("user@host") {
            return Err(format!("at sign not decoded: {s}"));
        }
        Ok(())
    }

    #[test]
    fn uri_very_long_path() -> Result<(), String> {
        let segment = "a".repeat(50);
        let path_str = format!("file:///{seg}/{seg}/{seg}/{seg}/test.pl", seg = segment);
        let path = uri_to_fs_path(&path_str).ok_or("expected Some for long path")?;
        if !path.to_string_lossy().ends_with("test.pl") {
            return Err(format!("long path truncated: {}", path.display()));
        }
        Ok(())
    }
}

// ── fs_path_to_uri: additional cases ────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
mod extra_fs_path_to_uri {
    use perl_uri::fs_path_to_uri;

    #[test]
    fn path_with_parentheses() -> Result<(), String> {
        let uri = fs_path_to_uri("/tmp/dir (copy)/test.pl")?;
        if !uri.starts_with("file:///") {
            return Err(format!("unexpected: {uri}"));
        }
        // Parens may or may not be encoded
        assert!(uri.contains("test.pl"));
        Ok(())
    }

    #[test]
    fn path_with_tilde() -> Result<(), String> {
        let uri = fs_path_to_uri("/home/~user/test.pl")?;
        assert!(uri.contains("~user") || uri.contains("%7E"));
        Ok(())
    }

    #[test]
    fn path_with_plus() -> Result<(), String> {
        let uri = fs_path_to_uri("/tmp/a+b/test.pl")?;
        assert!(uri.contains("a+b") || uri.contains("a%2Bb"));
        Ok(())
    }

    #[test]
    fn path_with_at_sign() -> Result<(), String> {
        let uri = fs_path_to_uri("/tmp/user@host/test.pl")?;
        // '@' may be encoded as %40
        assert!(uri.contains("user") && uri.contains("host"));
        Ok(())
    }

    #[test]
    fn path_with_percent_literal() -> Result<(), String> {
        let uri = fs_path_to_uri("/tmp/100%25/test.pl")?;
        // The literal '%' in the path should be double-encoded
        assert!(uri.starts_with("file:///"));
        Ok(())
    }

    #[test]
    fn path_single_file_at_root() -> Result<(), String> {
        let uri = fs_path_to_uri("/test.pl")?;
        assert!(uri.starts_with("file:///"));
        assert!(uri.ends_with("test.pl"));
        Ok(())
    }

    #[test]
    fn path_with_consecutive_dots() -> Result<(), String> {
        let uri = fs_path_to_uri("/tmp/.../test.pl")?;
        assert!(uri.starts_with("file:///"));
        Ok(())
    }

    #[test]
    fn path_with_exclamation_mark() -> Result<(), String> {
        let uri = fs_path_to_uri("/tmp/important!/test.pl")?;
        assert!(uri.starts_with("file:///"));
        assert!(uri.contains("test.pl"));
        Ok(())
    }

    #[test]
    fn path_with_brackets() -> Result<(), String> {
        let uri = fs_path_to_uri("/tmp/[version]/test.pl")?;
        assert!(uri.starts_with("file:///"));
        Ok(())
    }

    #[test]
    fn path_with_unicode_emoji() -> Result<(), String> {
        let uri = fs_path_to_uri("/tmp/📁/test.pl")?;
        assert!(uri.starts_with("file:///"));
        Ok(())
    }
}

// ── normalize_uri: additional cases ─────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
mod extra_normalize_uri {
    use perl_uri::normalize_uri;

    #[test]
    fn normalize_data_uri() {
        let result = normalize_uri("data:text/plain,hello");
        assert!(result.starts_with("data:"));
    }

    #[test]
    fn normalize_mailto_uri() {
        let result = normalize_uri("mailto:test@example.com");
        assert!(result.starts_with("mailto:"));
    }

    #[test]
    fn normalize_with_double_encoding() {
        // %2520 = double-encoded space (% → %25, then 20)
        let result = normalize_uri("file:///tmp/path%2520name/test.pl");
        assert!(result.contains("test.pl"));
    }

    #[test]
    fn normalize_preserves_case_in_path() {
        let result = normalize_uri("file:///tmp/MyModule/Test.PM");
        assert!(result.contains("MyModule"));
        assert!(result.contains("Test.PM"));
    }

    #[test]
    fn normalize_relative_path() {
        // A relative path that's not a valid URI
        let result = normalize_uri("relative/path.pl");
        // Should either become file:// URI or returned as-is
        assert!(
            result.starts_with("file:///") || result == "relative/path.pl",
            "unexpected: {result}"
        );
    }

    #[test]
    fn normalize_ssh_uri() {
        let result = normalize_uri("ssh://host/path/file.pl");
        assert!(result.starts_with("ssh://"));
    }

    #[test]
    fn normalize_triple_slash_no_path() {
        let result = normalize_uri("file:///");
        assert_eq!(result, "file:///");
    }

    #[test]
    fn normalize_custom_scheme() {
        let result = normalize_uri("custom-scheme:some-value");
        assert!(result.starts_with("custom-scheme:"));
    }
}

// ── roundtrip: additional patterns ──────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
mod extra_roundtrip {
    use perl_uri::{fs_path_to_uri, uri_to_fs_path};
    use std::path::Path;

    fn assert_roundtrip_matches(back: &Path, original: &str) -> Result<(), String> {
        #[cfg(windows)]
        if let Some(rootless) = original.strip_prefix('/') {
            let expected_suffix = rootless.replace('/', "\\");
            if back.ends_with(Path::new(&expected_suffix)) {
                return Ok(());
            }
            return Err(format!("mismatch: {} vs {}", back.display(), original));
        }

        if back == Path::new(original) {
            Ok(())
        } else {
            Err(format!("mismatch: {} vs {}", back.display(), original))
        }
    }

    #[test]
    fn roundtrip_path_with_hash() -> Result<(), String> {
        let original = "/tmp/file#name.pl";
        let uri = fs_path_to_uri(original)?;
        let back = uri_to_fs_path(&uri).ok_or("roundtrip failed")?;
        assert_roundtrip_matches(&back, original)
    }

    #[test]
    fn roundtrip_path_with_question_mark() -> Result<(), String> {
        let original = "/tmp/file?name.pl";
        let uri = fs_path_to_uri(original)?;
        let back = uri_to_fs_path(&uri).ok_or("roundtrip failed")?;
        assert_roundtrip_matches(&back, original)
    }

    #[test]
    fn roundtrip_path_with_multiple_spaces() -> Result<(), String> {
        let original = "/tmp/a  b   c/test.pl";
        let uri = fs_path_to_uri(original)?;
        let back = uri_to_fs_path(&uri).ok_or("roundtrip failed")?;
        assert_roundtrip_matches(&back, original)
    }

    #[test]
    fn roundtrip_many_extensions() -> Result<(), String> {
        let original = "/tmp/archive.tar.gz.bak";
        let uri = fs_path_to_uri(original)?;
        let back = uri_to_fs_path(&uri).ok_or("roundtrip failed")?;
        assert_roundtrip_matches(&back, original)
    }

    #[test]
    fn roundtrip_hidden_file() -> Result<(), String> {
        let original = "/tmp/.hidden_file";
        let uri = fs_path_to_uri(original)?;
        let back = uri_to_fs_path(&uri).ok_or("roundtrip failed")?;
        assert_roundtrip_matches(&back, original)
    }

    #[test]
    fn roundtrip_preserves_normalize_key_consistency() -> Result<(), String> {
        let path = "/tmp/consistent_key_test.pl";
        let uri = fs_path_to_uri(path)?;
        let key = perl_uri::uri_key(&uri);
        let normalized = perl_uri::normalize_uri(&uri);
        let key_of_normalized = perl_uri::uri_key(&normalized);
        if key != key_of_normalized {
            return Err(format!("keys differ: {key} vs {key_of_normalized}"));
        }
        Ok(())
    }
}

// ── cross-function consistency ──────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
mod consistency_tests {
    use perl_uri::{
        fs_path_to_uri, is_file_uri, is_special_scheme, normalize_uri, uri_extension, uri_key,
        uri_to_fs_path,
    };

    #[test]
    fn file_uri_is_not_special_and_vice_versa() {
        let test_uris = [
            ("file:///tmp/test.pl", true, false),
            ("https://example.com", false, true),
            ("untitled:Doc-1", false, true),
            ("git:/foo", false, true),
        ];
        for (uri, expect_file, expect_special) in &test_uris {
            assert_eq!(
                is_file_uri(uri),
                *expect_file,
                "is_file_uri({uri}) unexpected"
            );
            assert_eq!(
                is_special_scheme(uri),
                *expect_special,
                "is_special_scheme({uri}) unexpected"
            );
        }
    }

    #[test]
    fn normalize_then_extension() -> Result<(), String> {
        let normalized = normalize_uri("file:///tmp/module.pm");
        let ext = uri_extension(&normalized);
        if ext != Some("pm") {
            return Err(format!("expected pm, got: {ext:?}"));
        }
        Ok(())
    }

    #[test]
    fn normalize_then_key_then_extension() -> Result<(), String> {
        let uri = "file:///tmp/deep/path/Module.pm";
        let normalized = normalize_uri(uri);
        let key = uri_key(&normalized);
        let ext = uri_extension(&key);
        if ext != Some("pm") {
            return Err(format!("expected pm, got: {ext:?}"));
        }
        Ok(())
    }

    #[test]
    fn fs_to_uri_then_is_file() -> Result<(), String> {
        let uri = fs_path_to_uri("/tmp/test.pl")?;
        if !is_file_uri(&uri) {
            return Err(format!("fs_path_to_uri result is not a file URI: {uri}"));
        }
        if is_special_scheme(&uri) {
            return Err(format!(
                "fs_path_to_uri result should not be special: {uri}"
            ));
        }
        Ok(())
    }

    #[test]
    fn uri_key_idempotent() {
        let uris = [
            "file:///tmp/test.pl",
            "file:///C:/Users/test.pl",
            "https://example.com",
            "not-a-uri",
        ];
        for uri in &uris {
            let once = uri_key(uri);
            let twice = uri_key(&once);
            assert_eq!(once, twice, "uri_key not idempotent for: {uri}");
        }
    }

    #[test]
    fn normalize_idempotent_on_path() {
        let path = "/tmp/idempotent_test.pl";
        let once = normalize_uri(path);
        let twice = normalize_uri(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn all_perl_extensions_detected() {
        let extensions = [
            ("test.pl", "pl"),
            ("module.pm", "pm"),
            ("test.t", "t"),
            ("script.cgi", "cgi"),
            ("app.psgi", "psgi"),
            ("Makefile.PL", "PL"),
        ];
        for (filename, ext) in &extensions {
            let uri = format!("file:///tmp/{filename}");
            assert_eq!(uri_extension(&uri), Some(*ext), "failed for {filename}");
        }
    }

    #[test]
    fn roundtrip_then_extension() -> Result<(), String> {
        let path = "/tmp/deep/nested/Module.pm";
        let uri = fs_path_to_uri(path)?;
        let back = uri_to_fs_path(&uri).ok_or("roundtrip failed")?;
        let uri2 = fs_path_to_uri(&back)?;
        let ext = uri_extension(&uri2);
        if ext != Some("pm") {
            return Err(format!("expected pm, got: {ext:?}"));
        }
        Ok(())
    }

    #[test]
    fn tempdir_with_multiple_files() -> Result<(), String> {
        let dir = tempfile::tempdir().map_err(|e| format!("tempdir: {e}"))?;
        let files = ["one.pl", "two.pm", "three.t"];
        for name in &files {
            let p = dir.path().join(name);
            std::fs::write(&p, "# perl").map_err(|e| format!("write: {e}"))?;
            let uri = fs_path_to_uri(&p)?;
            let back = uri_to_fs_path(&uri).ok_or("roundtrip None")?;
            if back != p {
                return Err(format!(
                    "mismatch for {name}: {} vs {}",
                    back.display(),
                    p.display()
                ));
            }
        }
        Ok(())
    }
}
