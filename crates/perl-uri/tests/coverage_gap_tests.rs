//! Coverage-gap tests for `perl-uri` — branches missed in classify.rs and lib.rs.
//!
//! Targets the specific uncovered branches identified in issue #9034:
//!
//! **classify.rs**
//! - `normalize_unc_path_to_key`: bare UNC with no path past the share root (line 110-111)
//! - `normalize_windows_path_to_key`: short-path guard `path.len() < 3` (line 126)
//! - `normalize_windows_path_to_key`: missing-separator insertion `C:foo` → `C:/foo` (lines 143-145)
//!
//! **lib.rs** (`#[cfg(not(target_arch = "wasm32"))]`)
//! - `windows_rooted_file_uri_to_path`: non-Windows always-None stub (lines 198-201)
//! - `repair_mojibake_text`: high-code-point char exit — `u8::try_from` fails (lines 222-224)
//! - `repair_mojibake_text`: invalid-UTF-8-bytes exit — `String::from_utf8` fails (lines 228-230)
//! - `repair_mojibake_text`: repair-no-improvement exit (lines 232-236)
//! - `normalize_uri`: relative-path-via-cwd branch (lines 311-313)

use perl_uri::classify::uri_key;

#[cfg(not(target_arch = "wasm32"))]
use perl_tdd_support::must_some;

#[cfg(not(target_arch = "wasm32"))]
use perl_uri::{normalize_uri, uri_to_fs_path};

// ── classify::normalize_unc_path_to_key — bare share root ────────────────────

/// Backslash UNC path with server + share but no trailing path component.
///
/// Exercises the `if rest.is_empty() { Some(format!("file://{server}/{share}")) }`
/// branch at lines 110-111 of classify.rs.
#[test]
fn unc_bare_backslash_server_share_no_trailing_path() {
    assert_eq!(uri_key(r"\\server\share"), "file://server/share");
}

/// Forward-slash UNC form with server + share but no trailing path component.
///
/// The `//` prefix is stripped, yielding "server/share" — no further segments,
/// so the bare-root branch fires and produces `file://server/share`.
#[test]
fn unc_bare_forward_slash_server_share_no_trailing_path() {
    assert_eq!(uri_key("//fileserver/docs"), "file://fileserver/docs");
}

// ── classify::normalize_windows_path_to_key — length guard (len < 3) ─────────

/// `C:` has length 2 — `normalize_windows_path_to_key` returns `None` immediately.
///
/// Exercises the `if path.len() < 3 { return None; }` guard at line 126 of
/// classify.rs.  The input then falls through to `Url::parse`, which parses
/// `"C:"` as scheme `"c"` — not a `file:///` URI.
#[test]
fn windows_path_bare_drive_colon_too_short_returns_non_file_uri() {
    let key = uri_key("C:");
    assert!(
        !key.starts_with("file:///"),
        "bare 'C:' is too short for Windows-path normalization; got: {key}"
    );
}

/// Double-backslash alone (length 2) exercises the same guard indirectly.
///
/// `normalize_windows_path_to_key(r"\\")` → len 2 < 3 → None.
/// `normalize_unc_path_to_key(r"\\")` → after stripping `\\` the remaining
/// string is empty, yielding no server segment → None.
/// `Url::parse` also fails, so the input is returned as-is.
#[test]
fn double_backslash_alone_returned_unchanged() {
    assert_eq!(uri_key(r"\\"), r"\\");
}

// ── classify::normalize_windows_path_to_key — missing-separator insertion ────

/// `C:file.pl` — bytes[2] is `'f'` (not `'/'`) triggers the separator insertion
/// at lines 143-145 of classify.rs: `normalized.insert(2, '/')`.
#[test]
fn windows_path_missing_separator_after_colon_gets_slash_inserted() {
    assert_eq!(uri_key("C:file.pl"), "file:///c:/file.pl");
}

/// Same missing-separator case with a backslash path and uppercase drive letter.
#[test]
fn windows_path_backslash_no_leading_slash_separator_inserted() {
    assert_eq!(uri_key(r"D:subfolder\file.pl"), "file:///d:/subfolder/file.pl");
}

// ── lib::windows_rooted_file_uri_to_path — non-Windows always-None stub ──────

/// On non-Windows, `windows_rooted_file_uri_to_path` always returns `None`.
///
/// It is reached via `uri_to_fs_path` when `url.to_file_path()` fails.  On Unix,
/// that happens for `file://` URIs with a non-empty, non-localhost authority —
/// the url crate rejects them as non-local file paths.
#[cfg(not(target_arch = "wasm32"))]
#[cfg(not(windows))]
#[test]
fn uri_to_fs_path_file_uri_nonlocalhost_host_returns_none_on_non_windows() {
    // to_file_path() fails on Unix (non-localhost authority);
    // windows_rooted_file_uri_to_path (non-Windows stub) returns None.
    let result = uri_to_fs_path("file://buildserver/projects/app/lib.pm");
    assert!(result.is_none(), "non-localhost file URI host must yield None on non-Windows");
}

// ── lib::repair_mojibake_text — char-code > 255 exit (lines 222-224) ─────────

/// Decoded path contains the mojibake marker `'Ã'` (triggers repair) **and** the
/// character `'→'` (U+2192, code = 8594 > 255).
///
/// `u8::try_from(8594)` fails, so `repair_mojibake_text` returns the original
/// path string immediately at lines 222-224 of lib.rs.
///
/// URL encoding:
/// - `%C3%83` → `'Ã'` (U+00C3, a Latin-1–doubled mojibake marker)
/// - `%E2%86%92` → `'→'` (U+2192, code point 8594 — cannot be a Latin-1 byte)
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn uri_to_fs_path_mojibake_high_codepoint_char_returns_original_path() {
    let result = uri_to_fs_path("file:///tmp/%C3%83%E2%86%92.pl");
    let path = must_some(result);
    let path_str = path.to_string_lossy();
    // Repair is abandoned when '→' is encountered; the arrow is retained.
    assert!(
        path_str.contains('→'),
        "path should retain '→' when repair is abandoned at char code > 255: {path_str}"
    );
}

// ── lib::repair_mojibake_text — invalid-UTF-8-bytes exit (lines 228-230) ─────

/// Decoded path is `/tmp/ÃA.pl`.  The repair attempt collects bytes `[0xC3, 0x41]`
/// for `'Ã'` and `'A'`.  `String::from_utf8([…, 0xC3, 0x41, …])` fails because
/// `0x41` is not a UTF-8 continuation byte, so the original path is returned at
/// lines 228-230 of lib.rs.
///
/// URL encoding: `%C3%83` → `'Ã'`; `A` stays as-is.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn uri_to_fs_path_mojibake_repair_produces_invalid_utf8_returns_original() {
    let result = uri_to_fs_path("file:///tmp/%C3%83A.pl");
    let path = must_some(result);
    let path_str = path.to_string_lossy();
    // Repair byte sequence [0xC3, 0x41] is invalid UTF-8 → repair aborted → 'Ã' retained.
    assert!(
        path_str.contains('Ã'),
        "path should retain 'Ã' when repair bytes form invalid UTF-8: {path_str}"
    );
}

// ── lib::repair_mojibake_text — repair-no-improvement exit (lines 232-236) ───

/// Decoded path is `/tmp/Ã\u{0083}.pl`.
///
/// The repair collects bytes `[0xC3, 0x83]` (for `'Ã'` and `U+0083`), which form
/// the valid UTF-8 encoding of `'Ã'` again — so the candidate string also contains
/// the mojibake marker.  Since `marker_count(candidate) == marker_count(original)`,
/// the repair is not an improvement and the original path is returned at lines
/// 232-236 of lib.rs.
///
/// URL encoding:
/// - `%C3%83` → `'Ã'` (U+00C3)
/// - `%C2%83` → U+0083 (C1 control character, byte value 0x83 — a valid continuation byte)
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn uri_to_fs_path_mojibake_repair_no_marker_improvement_returns_original() {
    let result = uri_to_fs_path("file:///tmp/%C3%83%C2%83.pl");
    let path = must_some(result);
    let path_str = path.to_string_lossy();
    // U+0083 must still be present because repair gave no improvement.
    assert!(
        path_str.contains('\u{83}'),
        "U+0083 should remain (repair gave no improvement in mojibake marker count): {path_str}"
    );
}

// ── lib::normalize_uri — relative-path-via-cwd branch (lines 311-313) ────────

/// A relative path that is not a valid URL falls through URL parsing and is
/// resolved against the current working directory via `fs_path_to_uri`.
///
/// This exercises the `if let Ok(uri_string) = fs_path_to_uri(path)` success arm
/// at lines 311-313 of lib.rs.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn normalize_uri_relative_path_resolves_via_cwd_to_file_uri() {
    let result = normalize_uri("relative/dir/module.pm");
    assert!(
        result.starts_with("file:///"),
        "relative path should be resolved to a file:// URI: {result}"
    );
    assert!(
        result.ends_with("module.pm"),
        "result should end with the original filename: {result}"
    );
}

/// A deeply-nested relative path also exercises the CWD-resolution branch.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn normalize_uri_nested_relative_path_resolves_via_cwd() {
    let result = normalize_uri("lib/Local/Utils.pm");
    assert!(
        result.starts_with("file:///"),
        "nested relative path must resolve to file:// URI: {result}"
    );
    assert!(result.ends_with("Utils.pm"), "result should end with the leaf filename: {result}");
}
