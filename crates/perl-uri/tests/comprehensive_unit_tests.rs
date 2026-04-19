//! Comprehensive unit tests for the `perl-uri` crate.
//!
//! Covers: URI parsing, normalization, file path conversion,
//! extension extraction, scheme detection, and edge cases.

use perl_uri::{is_file_uri, is_special_scheme, uri_extension, uri_key};

// ── uri_key ─────────────────────────────────────────────────────────

#[test]
fn uri_key_preserves_unix_file_uri() {
    assert_eq!(uri_key("file:///tmp/test.pl"), "file:///tmp/test.pl");
}

#[test]
fn uri_key_lowercases_windows_drive_letter() {
    assert_eq!(
        uri_key("file:///C:/Users/test.pl"),
        "file:///c:/Users/test.pl"
    );
}

#[test]
fn uri_key_lowercases_various_drive_letters() {
    for letter in b'A'..=b'Z' {
        let upper = letter as char;
        let lower = upper.to_ascii_lowercase();
        let input = format!("file:///{upper}:/foo.pl");
        let expected = format!("file:///{lower}:/foo.pl");
        assert_eq!(uri_key(&input), expected);
    }
}

#[test]
fn uri_key_already_lowercase_drive_is_noop() {
    assert_eq!(
        uri_key("file:///c:/Users/test.pl"),
        "file:///c:/Users/test.pl"
    );
}

#[test]
fn uri_key_returns_invalid_uri_as_is() {
    assert_eq!(uri_key("not-a-uri"), "not-a-uri");
    assert_eq!(uri_key(""), "");
}

#[test]
fn uri_key_preserves_non_file_schemes() {
    let https = uri_key("https://example.com/path");
    assert!(https.starts_with("https://"));
}

#[test]
fn uri_key_preserves_query_and_fragment() {
    let key = uri_key("file:///tmp/test.pl?v=1#line=5");
    assert!(key.contains("v=1"));
    assert!(key.contains("#line=5"));
}

#[test]
fn uri_key_preserves_percent_encoding() {
    let key = uri_key("file:///tmp/path%20with%20spaces/test.pl");
    assert!(key.contains("path%20with%20spaces") || key.contains("path+with+spaces"));
}

// ── is_file_uri ─────────────────────────────────────────────────────

#[test]
fn is_file_uri_true_for_file_scheme() {
    assert!(is_file_uri("file:///tmp/test.pl"));
    assert!(is_file_uri("file:///C:/Users/test.pl"));
    assert!(is_file_uri("file://localhost/tmp/test.pl"));
}

#[test]
fn is_file_uri_false_for_other_schemes() {
    assert!(!is_file_uri("https://example.com"));
    assert!(!is_file_uri("http://example.com"));
    assert!(!is_file_uri("untitled:Untitled-1"));
    assert!(!is_file_uri("git:/foo/bar"));
    assert!(!is_file_uri("vscode-notebook:cell"));
}

#[test]
fn is_file_uri_false_for_empty_string() {
    assert!(!is_file_uri(""));
}

#[test]
fn is_file_uri_false_for_plain_path() {
    assert!(!is_file_uri("/tmp/test.pl"));
}

#[test]
fn is_file_uri_case_sensitive() {
    // file:// check is case-sensitive via starts_with
    assert!(!is_file_uri("FILE:///tmp/test.pl"));
    assert!(!is_file_uri("File:///tmp/test.pl"));
}

// ── is_special_scheme ───────────────────────────────────────────────

#[test]
fn is_special_scheme_detects_untitled() {
    assert!(is_special_scheme("untitled:Untitled-1"));
    assert!(is_special_scheme("untitled:some-doc"));
}

#[test]
fn is_special_scheme_detects_git() {
    assert!(is_special_scheme("git:/foo/bar"));
}

#[test]
fn is_special_scheme_detects_vscode_notebook() {
    assert!(is_special_scheme("vscode-notebook:cell-id"));
}

#[test]
fn is_special_scheme_detects_vscode_vfs() {
    assert!(is_special_scheme("vscode-vfs://github/repo/file.pl"));
}

#[test]
fn is_special_scheme_false_for_file() {
    assert!(!is_special_scheme("file:///tmp/test.pl"));
}

#[test]
fn is_special_scheme_detects_https() {
    assert!(is_special_scheme("https://example.com"));
    assert!(is_special_scheme("http://example.com"));
}

#[test]
fn is_special_scheme_handles_unparseable_with_known_prefix() {
    // Unparseable as URL but has a known prefix — fallback branch
    // These have colons but no valid authority, so Url::parse may succeed
    // or fall through to the prefix check depending on format
    let result = is_special_scheme("untitled:");
    assert!(result);
}

// ── uri_extension ───────────────────────────────────────────────────

#[test]
fn uri_extension_perl_extensions() {
    assert_eq!(uri_extension("file:///tmp/test.pl"), Some("pl"));
    assert_eq!(uri_extension("file:///tmp/Module.pm"), Some("pm"));
    assert_eq!(uri_extension("file:///tmp/script.t"), Some("t"));
    assert_eq!(uri_extension("file:///tmp/Makefile.PL"), Some("PL"));
}

#[test]
fn uri_extension_none_when_absent() {
    assert_eq!(uri_extension("file:///tmp/Makefile"), None);
    assert_eq!(uri_extension("file:///tmp/no-extension"), None);
}

#[test]
fn uri_extension_strips_query_string() {
    assert_eq!(uri_extension("file:///tmp/test.pl?query=1"), Some("pl"));
    assert_eq!(uri_extension("file:///tmp/test.pl?a=1&b=2"), Some("pl"));
}

#[test]
fn uri_extension_strips_fragment() {
    assert_eq!(uri_extension("file:///tmp/test.pl#line=10"), Some("pl"));
}

#[test]
fn uri_extension_handles_multiple_dots() {
    assert_eq!(uri_extension("file:///tmp/archive.tar.gz"), Some("gz"));
    assert_eq!(uri_extension("file:///tmp/My.Module.pm"), Some("pm"));
}

#[test]
fn uri_extension_hidden_file_no_ext() {
    // .gitignore → extension is "gitignore" (after the single dot)
    assert_eq!(uri_extension("file:///tmp/.gitignore"), Some("gitignore"));
}

#[test]
fn uri_extension_trailing_dot_returns_none() {
    assert_eq!(uri_extension("file:///tmp/file."), None);
}

#[test]
fn uri_extension_empty_string() {
    assert_eq!(uri_extension(""), None);
}

#[test]
fn uri_extension_non_file_uri() {
    assert_eq!(uri_extension("https://example.com/test.pl"), Some("pl"));
    assert_eq!(uri_extension("untitled:Untitled-1.pl"), Some("pl"));
}

#[test]
fn uri_extension_with_query_and_fragment() {
    assert_eq!(uri_extension("file:///tmp/test.pm?v=1#L5"), Some("pm"));
}

#[test]
fn uri_extension_percent_encoded_dot() {
    // %2E is percent-encoded dot — not treated as a real dot by uri_extension
    // since it operates on the raw string
    assert_eq!(uri_extension("file:///tmp/test%2Epl"), None);
}

// ── uri_to_fs_path (non-wasm) ───────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
mod uri_to_fs_path_tests {
    use perl_uri::uri_to_fs_path;

    #[test]
    fn basic_file_uri() -> Result<(), String> {
        let path = uri_to_fs_path("file:///tmp/test.pl").ok_or("expected Some")?;
        let s = path.to_string_lossy();
        if !s.ends_with("test.pl") {
            return Err(format!("unexpected path: {s}"));
        }
        Ok(())
    }

    #[test]
    fn percent_encoded_spaces() -> Result<(), String> {
        let path =
            uri_to_fs_path("file:///tmp/path%20with%20spaces/test.pl").ok_or("expected Some")?;
        let s = path.to_string_lossy();
        if !s.contains("path with spaces") {
            return Err(format!("spaces not decoded: {s}"));
        }
        Ok(())
    }

    #[test]
    fn non_file_schemes_return_none() {
        assert!(uri_to_fs_path("https://example.com").is_none());
        assert!(uri_to_fs_path("untitled:Untitled-1").is_none());
        assert!(uri_to_fs_path("git:/foo/bar").is_none());
        assert!(uri_to_fs_path("ftp://host/file").is_none());
    }

    #[test]
    fn invalid_uri_returns_none() {
        assert!(uri_to_fs_path("").is_none());
        assert!(uri_to_fs_path("not a uri at all").is_none());
        assert!(uri_to_fs_path(":::").is_none());
    }

    #[test]
    fn deeply_nested_path() -> Result<(), String> {
        let path = uri_to_fs_path("file:///a/b/c/d/e/f/g.pl").ok_or("expected Some")?;
        if !path.ends_with("g.pl") {
            return Err(format!("unexpected: {}", path.display()));
        }
        Ok(())
    }

    #[test]
    fn percent_encoded_special_chars() -> Result<(), String> {
        // %23 = '#', %3F = '?'
        let path = uri_to_fs_path("file:///tmp/file%23name.pl").ok_or("expected Some")?;
        let s = path.to_string_lossy();
        if !s.contains("file#name.pl") {
            return Err(format!("special char not decoded: {s}"));
        }
        Ok(())
    }

    #[test]
    fn unicode_path() -> Result<(), String> {
        let path =
            uri_to_fs_path("file:///tmp/%E4%B8%AD%E6%96%87/test.pl").ok_or("expected Some")?;
        let s = path.to_string_lossy();
        if !s.contains("中文") {
            return Err(format!("unicode not decoded: {s}"));
        }
        Ok(())
    }

    #[test]
    fn uri_with_fragment_still_parses() {
        // Url::parse will include fragments, but to_file_path ignores them
        let result = uri_to_fs_path("file:///tmp/test.pl#L10");
        assert!(result.is_some());
    }

    #[test]
    fn uri_with_query_still_parses() {
        let result = uri_to_fs_path("file:///tmp/test.pl?v=1");
        assert!(result.is_some());
    }
}

// ── fs_path_to_uri (non-wasm) ───────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
mod fs_path_to_uri_tests {
    use perl_uri::fs_path_to_uri;

    #[test]
    fn absolute_path() -> Result<(), String> {
        let uri = fs_path_to_uri("/tmp/test.pl")?;
        if !uri.starts_with("file:///") {
            return Err(format!("expected file:// prefix, got: {uri}"));
        }
        if !uri.contains("test.pl") {
            return Err(format!("expected test.pl in URI: {uri}"));
        }
        Ok(())
    }

    #[test]
    fn path_with_spaces_is_encoded() -> Result<(), String> {
        let uri = fs_path_to_uri("/tmp/path with spaces/test.pl")?;
        if !uri.contains("%20") {
            return Err(format!("expected percent-encoded spaces: {uri}"));
        }
        Ok(())
    }

    #[test]
    fn relative_path_becomes_absolute() -> Result<(), String> {
        let uri = fs_path_to_uri("relative/path.pl")?;
        if !uri.starts_with("file:///") {
            return Err(format!("relative path not made absolute: {uri}"));
        }
        Ok(())
    }

    #[test]
    fn path_with_special_characters() -> Result<(), String> {
        let uri = fs_path_to_uri("/tmp/file#name.pl")?;
        // '#' should be percent-encoded
        if !uri.contains("%23") {
            return Err(format!("expected encoded '#': {uri}"));
        }
        Ok(())
    }

    #[test]
    fn deeply_nested_path() -> Result<(), String> {
        let uri = fs_path_to_uri("/a/b/c/d/e/f/g.pl")?;
        if !uri.ends_with("g.pl") {
            return Err(format!("unexpected URI: {uri}"));
        }
        Ok(())
    }

    #[test]
    fn unicode_path() -> Result<(), String> {
        let uri = fs_path_to_uri("/tmp/中文/test.pl")?;
        if !uri.starts_with("file:///") {
            return Err(format!("expected file:// URI: {uri}"));
        }
        Ok(())
    }

    #[test]
    fn root_path() -> Result<(), String> {
        let uri = fs_path_to_uri("/")?;
        if !uri.starts_with("file:///") {
            return Err(format!("unexpected root URI: {uri}"));
        }
        Ok(())
    }
}

// ── roundtrip (non-wasm) ────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
mod roundtrip_tests {
    use perl_uri::{fs_path_to_uri, uri_to_fs_path};
    use std::path::Path;

    fn assert_roundtrip_matches(back: &Path, original: &str) -> Result<(), String> {
        #[cfg(windows)]
        if let Some(rootless) = original.strip_prefix('/') {
            let expected_suffix = rootless.replace('/', "\\");
            if back.ends_with(Path::new(&expected_suffix)) {
                return Ok(());
            }
            return Err(format!(
                "roundtrip mismatch: {} vs {}",
                back.display(),
                original
            ));
        }

        if back == Path::new(original) {
            Ok(())
        } else {
            Err(format!(
                "roundtrip mismatch: {} vs {}",
                back.display(),
                original
            ))
        }
    }

    #[test]
    fn path_to_uri_and_back() -> Result<(), String> {
        let original = "/tmp/roundtrip.pl";
        let uri = fs_path_to_uri(original)?;
        let path = uri_to_fs_path(&uri).ok_or("roundtrip failed: uri_to_fs_path returned None")?;
        assert_roundtrip_matches(&path, original)
    }

    #[test]
    fn roundtrip_with_spaces() -> Result<(), String> {
        let original = "/tmp/has spaces/file.pl";
        let uri = fs_path_to_uri(original)?;
        let path = uri_to_fs_path(&uri).ok_or("roundtrip failed")?;
        assert_roundtrip_matches(&path, original)
    }

    #[test]
    fn roundtrip_deeply_nested() -> Result<(), String> {
        let original = "/a/b/c/d/e/f/g.pm";
        let uri = fs_path_to_uri(original)?;
        let path = uri_to_fs_path(&uri).ok_or("roundtrip failed")?;
        assert_roundtrip_matches(&path, original)
    }

    #[test]
    fn roundtrip_unicode_path() -> Result<(), String> {
        let original = "/tmp/日本語/テスト.pl";
        let uri = fs_path_to_uri(original)?;
        let path = uri_to_fs_path(&uri).ok_or("roundtrip failed")?;
        assert_roundtrip_matches(&path, original)
    }
}

// ── normalize_uri (non-wasm) ────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
mod normalize_uri_tests {
    use perl_uri::normalize_uri;

    #[test]
    fn already_valid_file_uri() {
        assert_eq!(normalize_uri("file:///tmp/test.pl"), "file:///tmp/test.pl");
    }

    #[test]
    fn special_scheme_preserved() {
        assert_eq!(normalize_uri("untitled:Untitled-1"), "untitled:Untitled-1");
    }

    #[test]
    fn https_uri_preserved() {
        let result = normalize_uri("https://example.com/path");
        assert!(result.starts_with("https://"));
    }

    #[test]
    fn absolute_path_becomes_file_uri() {
        let result = normalize_uri("/tmp/test.pl");
        assert!(
            result.starts_with("file:///"),
            "expected file:// URI, got: {result}"
        );
        assert!(result.contains("test.pl"));
    }

    #[test]
    fn uri_with_percent_encoding_preserved() {
        let input = "file:///tmp/path%20with%20spaces/test.pl";
        let result = normalize_uri(input);
        assert!(result.contains("test.pl"));
    }

    #[test]
    fn git_scheme_preserved() {
        let result = normalize_uri("git:/foo/bar.pl");
        assert!(result.starts_with("git:"));
    }

    #[test]
    fn vscode_notebook_scheme_preserved() {
        let result = normalize_uri("vscode-notebook:cell-scheme-123");
        assert!(result.starts_with("vscode-notebook:"));
    }

    #[test]
    fn empty_string_is_returned() {
        // Empty string is not a valid URI and not a valid path;
        // normalize_uri falls through to returning as-is or converting
        let result = normalize_uri("");
        // Should not panic; exact value depends on implementation
        assert!(result.is_empty() || result.starts_with("file:///"));
    }

    #[test]
    fn idempotent_on_valid_uri() {
        let uri = "file:///tmp/test.pl";
        let once = normalize_uri(uri);
        let twice = normalize_uri(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn idempotent_on_special_scheme() {
        let uri = "untitled:Untitled-1";
        let once = normalize_uri(uri);
        let twice = normalize_uri(&once);
        assert_eq!(once, twice);
    }
}

// ── edge cases & integration ────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
mod edge_case_tests {
    use perl_uri::{
        fs_path_to_uri, is_file_uri, is_special_scheme, normalize_uri, uri_extension, uri_key,
        uri_to_fs_path,
    };

    #[test]
    fn uri_key_consistent_with_normalize() -> Result<(), String> {
        let path = "/tmp/consistent.pl";
        let uri = fs_path_to_uri(path)?;
        let normalized = normalize_uri(&uri);
        let key1 = uri_key(&uri);
        let key2 = uri_key(&normalized);
        if key1 != key2 {
            return Err(format!("keys differ: {key1} vs {key2}"));
        }
        Ok(())
    }

    #[test]
    fn is_file_uri_agrees_with_uri_to_fs_path() {
        let file = "file:///tmp/test.pl";
        let non_file = "https://example.com";
        assert!(is_file_uri(file));
        assert!(uri_to_fs_path(file).is_some());
        assert!(!is_file_uri(non_file));
        assert!(uri_to_fs_path(non_file).is_none());
    }

    #[test]
    fn extension_from_converted_uri() -> Result<(), String> {
        let uri = fs_path_to_uri("/tmp/module.pm")?;
        let ext = uri_extension(&uri);
        if ext != Some("pm") {
            return Err(format!("expected pm, got: {ext:?}"));
        }
        Ok(())
    }

    #[test]
    fn special_scheme_is_not_file_uri() {
        let uris = [
            "untitled:Untitled-1",
            "git:/foo/bar",
            "vscode-notebook:cell",
            "https://example.com",
        ];
        for uri in &uris {
            assert!(!is_file_uri(uri), "{uri} should not be a file URI");
            assert!(is_special_scheme(uri), "{uri} should be a special scheme");
        }
    }

    #[test]
    fn file_uri_is_not_special_scheme() {
        assert!(!is_special_scheme("file:///tmp/test.pl"));
        assert!(is_file_uri("file:///tmp/test.pl"));
    }

    #[test]
    fn tempfile_roundtrip() -> Result<(), String> {
        let dir = tempfile::tempdir().map_err(|e| format!("failed to create tempdir: {e}"))?;
        let file_path = dir.path().join("test_file.pl");
        std::fs::write(&file_path, "# perl").map_err(|e| format!("write failed: {e}"))?;

        let uri = fs_path_to_uri(&file_path)?;
        assert!(is_file_uri(&uri));

        let back = uri_to_fs_path(&uri).ok_or("uri_to_fs_path returned None")?;
        if back != file_path {
            return Err(format!(
                "tempfile roundtrip mismatch: {} vs {}",
                back.display(),
                file_path.display()
            ));
        }

        let ext = uri_extension(&uri);
        if ext != Some("pl") {
            return Err(format!("expected .pl extension, got: {ext:?}"));
        }

        Ok(())
    }

    #[test]
    fn uri_key_with_no_drive_letter() {
        // Unix-style path after file:/// — no drive letter normalization
        let key = uri_key("file:///usr/local/lib/perl5/Module.pm");
        assert_eq!(key, "file:///usr/local/lib/perl5/Module.pm");
    }

    #[test]
    fn normalize_uri_with_trailing_slash() {
        let result = normalize_uri("file:///tmp/dir/");
        assert!(result.starts_with("file:///"));
    }

    #[test]
    fn percent_encoded_slash_in_uri() {
        // %2F is an encoded forward slash — Url::parse may normalize it
        let path = uri_to_fs_path("file:///tmp/a%2Fb.pl");
        // May or may not succeed depending on platform; should not panic
        let _ = path;
    }
}
