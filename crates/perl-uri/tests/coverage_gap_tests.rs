//! Targeted coverage tests for `perl-uri` gaps surfaced by `cargo llvm-cov`.
//!
//! Each test references the specific code path it exercises so that
//! future refactors can update both the implementation and its receipt
//! together.

use perl_uri::uri_key;

// ── classify::normalize_unc_path_to_key (UNC bare server/share) ──────
//
// `classify.rs:101-115` returns `Some("file://{server}/{share}")` when a
// UNC payload has no path segments after the share. The existing tests
// in `classify_boundary_cases.rs` exercise UNC paths with a trailing
// file segment but never a bare `\\server\share` form.

#[test]
fn uri_key_bare_unc_path_without_trailing_path() {
    // Pure UNC with no path past the share name.
    assert_eq!(uri_key(r"\\server\share"), "file://server/share");
}

#[test]
fn uri_key_bare_unc_path_via_legacy_two_slash_form() {
    assert_eq!(uri_key(r"file://\\server\share"), "file://server/share");
}

#[test]
fn uri_key_bare_unc_path_with_trailing_separator() {
    // Trailing separator collapses to an empty segment and is filtered out,
    // so this still lands in the "no rest" branch.
    assert_eq!(uri_key(r"\\server\share\"), "file://server/share");
}

// ── classify::normalize_windows_path_to_key (length guard) ──────────
//
// `classify.rs:120-127` early-returns `None` for path strings shorter
// than three bytes after slash stripping. Without an explicit test the
// guard registers as an unhit branch, and a future "drop the length
// check" refactor would not break any existing assertions.

#[test]
fn uri_key_drive_only_short_form_is_returned_as_is() {
    // Drive letter without separator is exactly two bytes after stripping —
    // `normalize_windows_path_to_key` rejects it, normalize_unc rejects it,
    // and the URL crate parses `C:` as scheme-only with empty path.
    let key = uri_key("C:");
    assert_eq!(key, "c:");
}

#[test]
fn uri_key_single_letter_input_falls_through() {
    // Too short for Windows path or UNC pre-pass; not a valid URL either.
    assert_eq!(uri_key("X"), "X");
}

#[test]
fn uri_key_empty_after_file_double_slash_falls_through() {
    // `file://` strips to empty, which is shorter than 3 chars — both
    // helpers refuse it, then Url::parse handles it cleanly.
    assert_eq!(uri_key("file://"), "file:///");
}

// ── classify::normalize_windows_path_to_key (missing separator) ─────
//
// `classify.rs:142-144` inserts a `/` after the drive colon when the
// caller supplied `C:foo` style paths (no separator). Without the
// insertion, downstream consumers would see `file:///c:foo`.

#[test]
fn uri_key_windows_path_missing_separator_after_colon() {
    // Forces the `normalized.insert(2, '/')` path.
    assert_eq!(uri_key("C:foo"), "file:///c:/foo");
}

#[test]
fn uri_key_windows_path_missing_separator_with_pipe_drive() {
    // Pipe-drive variant (`C|foo`) must also gain the separator after
    // the legacy `|` → `:` rewrite.
    assert_eq!(uri_key("C|foo"), "file:///c:/foo");
}

#[test]
fn uri_key_windows_path_missing_separator_via_two_slash_form() {
    // `file://C:foo` strips to `C:foo` and lands in the same insertion path.
    assert_eq!(uri_key("file://C:foo"), "file:///c:/foo");
}

// ── lib::windows_rooted_file_uri_to_path (non-Windows stub) ─────────
//
// On non-Windows targets the helper at `lib.rs:198-201` is a stub that
// always returns `None`. It is reachable from `uri_to_fs_path` only
// when `url::Url::to_file_path` itself fails, e.g. for file URIs with a
// non-localhost host (which the URL crate strips for `localhost`).

#[cfg(all(not(target_arch = "wasm32"), not(windows)))]
mod non_windows_stub {
    use perl_uri::uri_to_fs_path;

    #[test]
    fn non_localhost_file_host_returns_none() {
        // `Url::to_file_path()` returns `Err(())` for non-localhost hosts
        // on Unix; the fallback then exercises the non-Windows stub.
        assert!(uri_to_fs_path("file://example.com/foo.pl").is_none());
    }

    #[test]
    fn ipv4_file_host_maps_to_local_path() -> Result<(), String> {
        let path = uri_to_fs_path("file://127.0.0.1/foo.pl").ok_or("expected Some")?;
        if !path.ends_with("foo.pl") {
            return Err(format!("unexpected path: {}", path.display()));
        }
        Ok(())
    }

    #[test]
    fn explicit_server_share_file_uri_returns_none() {
        assert!(uri_to_fs_path("file://server/share/file.pl").is_none());
    }
}

// ── lib::repair_path_mojibake / repair_mojibake_text branches ───────
//
// `lib.rs:204-237` covers the mojibake repair pipeline. The existing
// test exercises the happy path where a percent-encoded mojibake
// double-encoding round-trips to readable UTF-8. The four lesser-trod
// branches are exercised below.

#[cfg(not(target_arch = "wasm32"))]
mod mojibake_branches {
    use perl_uri::uri_to_fs_path;

    #[cfg(unix)]
    #[test]
    fn non_utf8_path_bytes_pass_through_unchanged() -> Result<(), String> {
        // `%FF` decodes to a single 0xFF byte which is invalid UTF-8 in
        // a path. `path.to_str()` returns `None`, so the repair function
        // bails early and returns the path unchanged (`lib.rs:205-207`).
        let path = uri_to_fs_path("file:///tmp/%FF.pl").ok_or("expected Some")?;
        let bytes = path_bytes(&path);
        // The 0xFF byte survives the round-trip; we only assert the path
        // exists and contains the raw byte to keep this stable across
        // PathBuf debug formatting changes.
        if !bytes.contains(&0xFF) {
            return Err(format!("expected 0xFF in path bytes, got {bytes:?}"));
        }
        Ok(())
    }

    #[test]
    fn mojibake_marker_with_high_codepoint_bails() -> Result<(), String> {
        // `%C3%83%E4%B8%AD.pl` → "Ã中.pl". `Ã` trips `looks_like_mojibake`,
        // but `中` (U+4E2D) exceeds u8::MAX, so the byte-collection loop
        // returns early via `u8::try_from(...)` failure
        // (`lib.rs:221-224`).
        let path = uri_to_fs_path("file:///%C3%83%E4%B8%AD.pl").ok_or("expected Some")?;
        let display = path.to_string_lossy();
        // The original text must round-trip unchanged.
        if !display.contains('Ã') {
            return Err(format!("Ã missing in {display}"));
        }
        if !display.contains('中') {
            return Err(format!("中 missing in {display}"));
        }
        Ok(())
    }

    #[test]
    fn mojibake_text_producing_invalid_utf8_bytes_bails() -> Result<(), String> {
        // `%C3%83a.pl` → "Ãa.pl". `Ã` (0xC3) plus `a` (0x61) collapses to
        // bytes [0xC3, 0x61] which is invalid UTF-8 (`0x61` is not a
        // continuation byte). `String::from_utf8` errors and the text is
        // returned as-is (`lib.rs:228-230`).
        let path = uri_to_fs_path("file:///%C3%83a.pl").ok_or("expected Some")?;
        let display = path.to_string_lossy();
        if !display.contains('Ã') {
            return Err(format!("Ã missing in {display}"));
        }
        // The `a` after Ã must survive — the repair should not fire here.
        if !display.contains("Ãa") {
            return Err(format!("expected 'Ãa' in {display}"));
        }
        Ok(())
    }

    #[test]
    fn mojibake_text_that_does_not_reduce_markers_is_kept() -> Result<(), String> {
        // `%C3%83%C2%83.pl` → "Ã\u{83}.pl". Collapsing to bytes gives
        // [0xC3, 0x83, 0xC2, 0x83]:
        //   - 0xC3 0x83 → "Ã"  (marker)
        //   - 0xC2 0x83 → "\u{83}" (control char, not a marker)
        // The candidate "Ã\u{83}" therefore has the same marker count as
        // the original, so the function falls through to returning the
        // original text (`lib.rs:232-236`).
        let path = uri_to_fs_path("file:///%C3%83%C2%83.pl").ok_or("expected Some")?;
        let display = path.to_string_lossy();
        // The original mojibake marker must survive.
        if !display.contains('Ã') {
            return Err(format!("Ã missing in {display}"));
        }
        // And the trailing control byte must remain (not collapsed to a
        // single repaired Ã).
        if !display.contains('\u{83}') {
            return Err(format!("control byte missing in {display}"));
        }
        Ok(())
    }

    #[cfg(unix)]
    fn path_bytes(path: &std::path::Path) -> Vec<u8> {
        use std::os::unix::ffi::OsStrExt;
        path.as_os_str().as_bytes().to_vec()
    }
}

// ── lib::normalize_uri (non-URL relative path branch) ───────────────
//
// `lib.rs:309-313` handles the "input is not a valid URL but we can
// still treat it as a filesystem path" branch. The existing relative
// path test in `additional_unit_tests.rs` exercises this, but we add a
// deterministic assertion here so the branch's contract is locked in.

#[cfg(not(target_arch = "wasm32"))]
mod normalize_uri_relative_path {
    use perl_uri::normalize_uri;

    #[test]
    fn relative_path_segment_is_resolved_against_cwd() {
        // `Url::parse` rejects relative URLs without a base, so we fall
        // through to `fs_path_to_uri`, which joins with the current
        // working directory and emits a `file://` URI.
        let result = normalize_uri("subdir/module.pm");
        assert!(
            result.starts_with("file:///"),
            "expected file:// URI for relative path, got: {result}"
        );
        assert!(result.ends_with("subdir/module.pm"), "tail missing: {result}");
    }

    #[test]
    fn bare_filename_resolves_against_cwd() {
        let result = normalize_uri("README.md");
        assert!(result.starts_with("file:///"), "got: {result}");
        assert!(result.ends_with("README.md"), "tail missing: {result}");
    }

    #[test]
    fn non_url_unrecognised_scheme_string_stays_url_serialised() {
        // `Url::parse` accepts `foo:bar` (opaque) — so we exit through
        // the URL branch and the input is returned canonicalised.
        let result = normalize_uri("foo:bar");
        // The `url` crate normalises by re-serialising; either prefix is
        // acceptable so long as the opaque payload survives.
        assert!(result.contains("bar"), "lost payload: {result}");
    }
}
