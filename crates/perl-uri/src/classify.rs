//! URI classification and key normalization helpers.
//!
//! This module centralizes URI helpers that are frequently reused by LSP-facing
//! crates while keeping filesystem URI conversion concerns in `perl-uri`.

use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};
use url::Url;

const URI_PATH_KEY_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}');

/// Normalize a URI to a consistent key for lookups.
///
/// This function handles platform-specific differences to ensure consistent
/// lookups across different systems, particularly for Windows drive letters.
///
/// In addition to standard canonical `file:///C:/...` URIs, this also handles
/// legacy forms emitted by Notepad++ and similar editors:
/// - `file://C:\path\file.pl` (two slashes, backslashes)
/// - `C:\path\file.pl` (bare Windows path, no scheme)
///
/// Both forms are converted to `file:///c:/path/file.pl`.
#[must_use]
pub fn uri_key(uri: &str) -> String {
    let trimmed = uri.trim();

    // Try to normalize legacy Windows path forms before URL parsing, since
    // `file://C:\...` and `C:\...` are not valid URLs and fall through to the
    // else branch as-is without this pre-pass.
    if let Some(normalized) = normalize_legacy_windows_uri(trimmed) {
        return normalized;
    }

    if let Ok(parsed) = Url::parse(trimmed) {
        let mut value = parsed.as_str().to_string();

        // Canonicalize localhost file authorities (file://localhost/...) to
        // the standard local form (file:///...) so equivalent URIs map to the
        // same key.
        if parsed.scheme() == "file"
            && parsed.host_str() == Some("localhost")
            && let Some(path) = value.strip_prefix("file://localhost")
        {
            value = format!("file://{path}");
        }

        if let Some(rest) = value.strip_prefix("file:///")
            && rest.len() > 1
            && (rest.as_bytes()[1] == b':' || rest.as_bytes()[1] == b'|')
            && rest.as_bytes()[0].is_ascii_alphabetic()
        {
            let separator = if rest.as_bytes()[1] == b'|' { ":" } else { &rest[1..2] };
            return format!(
                "file:///{}{}{}",
                rest[0..1].to_ascii_lowercase(),
                separator,
                &rest[2..]
            );
        }
        value
    } else {
        trimmed.to_string()
    }
}

/// Normalize legacy Windows URI forms to canonical `file:///c:/...` keys.
///
/// Handles:
/// - `file://C:\path\file.pl` → `file:///c:/path/file.pl`  (two-slash form)
/// - `C:\path\file.pl`        → `file:///c:/path/file.pl`  (bare path form)
///
/// Returns `None` for any URI that is not one of these legacy forms.
pub(crate) fn normalize_legacy_windows_uri(uri: &str) -> Option<String> {
    let trimmed = uri.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Strip the optional `file://` prefix (two slashes — not three).
    // A three-slash `file:///` form is already canonical and must not match here.
    let path = if let Some(rest) = strip_ascii_prefix(trimmed, "file://") {
        // Make sure we are not accidentally handling `file:///...` (three slashes).
        // After stripping `file://`, a canonical URI starts with `/` followed by
        // another `/` (the empty authority makes a third slash), so we skip it.
        if rest.starts_with('/') {
            return None;
        }
        rest
    } else {
        trimmed
    };

    // Accept malformed localhost authorities commonly emitted by some clients,
    // e.g. `file://localhost/C:\dir\file.pl`, `FILE://LOCALHOST/C:/dir/file.pl`,
    // and the backslash-delimited `file://localhost\C:\dir\file.pl` variant.
    let path = strip_localhost_authority(path).unwrap_or(path);

    normalize_windows_path_to_key(path).or_else(|| normalize_unc_path_to_key(path))
}

fn strip_ascii_prefix<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    let candidate = value.get(..prefix.len())?;
    if candidate.eq_ignore_ascii_case(prefix) { value.get(prefix.len()..) } else { None }
}

fn strip_localhost_authority(path: &str) -> Option<&str> {
    let rest = strip_ascii_prefix(path, "localhost")?;
    rest.strip_prefix('/').or_else(|| rest.strip_prefix('\\'))
}

/// Convert a UNC path into canonical `file://server/share/...` key.
///
/// Handles both bare UNC paths (e.g. `\\server\share\file.pl`) and
/// legacy two-slash URI payloads (e.g. `file://\\server\share\file.pl`).
fn normalize_unc_path_to_key(path: &str) -> Option<String> {
    let without_prefix = path.strip_prefix(r"\\").or_else(|| path.strip_prefix("//"))?;
    let replaced = without_prefix.replace('\\', "/");
    let mut parts = replaced.split('/').filter(|segment| !segment.is_empty());

    let server = parts.next()?;
    let share = parts.next()?;
    let share = encode_uri_path_key_component(share);
    let rest = parts.map(encode_uri_path_key_component).collect::<Vec<_>>().join("/");

    if rest.is_empty() {
        Some(format!("file://{server}/{share}"))
    } else {
        Some(format!("file://{server}/{share}/{rest}"))
    }
}

/// Convert a Windows-style path string (with or without a drive letter) into a
/// canonical `file:///c:/...` key.  Returns `None` if `path` does not look like
/// a Windows path (i.e. does not start with `<letter>:`).
fn normalize_windows_path_to_key(path: &str) -> Option<String> {
    // Strip any leading slashes (e.g. from a `file://` prefix that had an extra `/`).
    let path = path.trim_start_matches('/');

    // Must be at least `X:\` or `X:/` to be a Windows path.
    if path.len() < 3 {
        return None;
    }

    let bytes = path.as_bytes();
    if !bytes[0].is_ascii_alphabetic() || (bytes[1] != b':' && bytes[1] != b'|') {
        return None;
    }

    // Replace backslashes with forward slashes.
    let mut normalized = path.replace('\\', "/");

    // Convert legacy drive separators (`C|`) to `C:`.
    if normalized.as_bytes().get(1) == Some(&b'|') {
        normalized.replace_range(1..2, ":");
    }

    // Ensure there is a separator after the drive colon: `C:foo` → `C:/foo`.
    if normalized.as_bytes().get(2) != Some(&b'/') {
        normalized.insert(2, '/');
    }

    // Lowercase the drive letter.
    let drive = normalized[0..1].to_ascii_lowercase();
    let path =
        normalized[3..].split('/').map(encode_uri_path_key_component).collect::<Vec<_>>().join("/");

    if path.is_empty() {
        Some(format!("file:///{drive}:/"))
    } else {
        Some(format!("file:///{drive}:/{path}"))
    }
}

fn encode_uri_path_key_component(component: &str) -> String {
    utf8_percent_encode(component, URI_PATH_KEY_ENCODE_SET).to_string()
}

/// Check if a URI uses the `file://` scheme.
#[must_use]
pub fn is_file_uri(uri: &str) -> bool {
    uri.get(..7).is_some_and(|prefix| prefix.eq_ignore_ascii_case("file://"))
}

/// Check if a URI uses a special scheme (not `file://`).
#[must_use]
pub fn is_special_scheme(uri: &str) -> bool {
    if let Ok(url) = Url::parse(uri) {
        url.scheme() != "file"
    } else {
        uri.get(..9).is_some_and(|p| p.eq_ignore_ascii_case("untitled:"))
            || uri.get(..4).is_some_and(|p| p.eq_ignore_ascii_case("git:"))
            || uri.get(..16).is_some_and(|p| p.eq_ignore_ascii_case("vscode-notebook:"))
            || uri.get(..21).is_some_and(|p| p.eq_ignore_ascii_case("vscode-notebook-cell:"))
            || uri.get(..11).is_some_and(|p| p.eq_ignore_ascii_case("vscode-vfs:"))
    }
}

/// Extract the file extension from a URI-like string.
#[must_use]
pub fn uri_extension(uri: &str) -> Option<&str> {
    let path_without_query_or_fragment =
        uri.split_once(['?', '#']).map_or(uri, |(path_prefix, _)| path_prefix);
    let path_part = path_without_query_or_fragment.rsplit(['/', '\\']).next()?;
    let dot_pos = path_part.rfind('.')?;
    // A leading dot means a dotfile (e.g. `.bashrc`, `.gitignore`) — treat as
    // extensionless rather than returning the entire filename after the dot.
    if dot_pos == 0 {
        return None;
    }
    let ext = &path_part[dot_pos + 1..];
    if ext.is_empty() { None } else { Some(ext) }
}

#[cfg(test)]
mod tests {
    use super::{is_file_uri, is_special_scheme, uri_extension, uri_key};

    #[test]
    fn normalizes_uri_keys() {
        assert_eq!(uri_key("file:///tmp/test.pl"), "file:///tmp/test.pl");
        assert_eq!(uri_key("file:///C:/Users/test.pl"), "file:///c:/Users/test.pl");
    }

    #[test]
    fn normalizes_localhost_file_authority() {
        assert_eq!(uri_key("file://localhost/tmp/test.pl"), uri_key("file:///tmp/test.pl"));
        assert_eq!(
            uri_key("file://localhost/C:/Users/test.pl"),
            uri_key("file:///c:/Users/test.pl")
        );
    }

    #[test]
    fn preserves_non_local_file_authority() {
        assert_eq!(uri_key("file://server/share/test.pl"), "file://server/share/test.pl");
    }

    #[test]
    fn preserves_invalid_uri_values() {
        assert_eq!(uri_key("not-a-uri"), "not-a-uri");
    }

    #[test]
    fn normalizes_surrounding_whitespace_before_keying() {
        assert_eq!(uri_key("  file:///tmp/test.pl\n"), "file:///tmp/test.pl");
        assert_eq!(
            uri_key(" \tfile:///C:/Users/dev/trimmed.pl\n"),
            "file:///c:/Users/dev/trimmed.pl"
        );
        assert_eq!(uri_key("\tfile:///C:/Users/dev/test.pl  "), "file:///c:/Users/dev/test.pl");
        assert_eq!(uri_key("  not-a-uri  "), "not-a-uri");
    }

    #[test]
    fn normalizes_legacy_notepadpp_file_uri_two_slashes() {
        // Notepad++ LSP client emits `file://C:\...` (two slashes, backslashes).
        assert_eq!(uri_key(r"file://C:\Users\dev\example.pl"), "file:///c:/Users/dev/example.pl");
        assert_eq!(
            uri_key(r"file://D:\projects\MyApp\script.pl"),
            "file:///d:/projects/MyApp/script.pl"
        );
    }

    #[test]
    fn normalizes_bare_windows_path() {
        // Some editors send a bare `C:\...` path with no scheme at all.
        assert_eq!(uri_key(r"C:\Users\dev\plain_path.pl"), "file:///c:/Users/dev/plain_path.pl");
        assert_eq!(uri_key(r"c:\users\dev\lowercase.pl"), "file:///c:/users/dev/lowercase.pl");
    }

    #[test]
    fn normalizes_windows_drive_paths_without_directory_separator() {
        assert_eq!(uri_key(r"C:relative\script.pl"), "file:///c:/relative/script.pl");
        assert_eq!(uri_key("file://D:relative/script.pl"), "file:///d:/relative/script.pl");
    }

    #[test]
    fn normalizes_legacy_file_uri_two_slashes_forward_slash() {
        // Some clients emit `file://C:/...` (two slashes, forward slashes) instead of
        // the canonical three-slash form.  The pre-pass handles both backslash and
        // forward-slash variants of the two-slash form.
        assert_eq!(uri_key("file://C:/Users/dev/example.pl"), "file:///c:/Users/dev/example.pl");
        assert_eq!(
            uri_key("file://D:/projects/MyApp/script.pl"),
            "file:///d:/projects/MyApp/script.pl"
        );
    }

    #[test]
    fn normalizes_legacy_windows_drive_pipe_separator() {
        assert_eq!(uri_key("file:///C|/Users/dev/example.pl"), "file:///c:/Users/dev/example.pl");
        assert_eq!(
            uri_key(r"file://D|\projects\MyApp\script.pl"),
            "file:///d:/projects/MyApp/script.pl"
        );
    }

    #[test]
    fn normalizes_legacy_localhost_windows_uri_variants() {
        assert_eq!(
            uri_key(r"file://localhost/C:\Users\dev\example.pl"),
            "file:///c:/Users/dev/example.pl"
        );
        assert_eq!(
            uri_key("file://localhost/C:/Users/dev/example.pl"),
            "file:///c:/Users/dev/example.pl"
        );
        assert_eq!(
            uri_key(r"file://LOCALHOST/D:\projects\myapp\script.pl"),
            "file:///d:/projects/myapp/script.pl"
        );
    }

    #[test]
    fn normalizes_case_insensitive_legacy_file_scheme() {
        assert_eq!(uri_key(r"FILE://C:\Users\dev\example.pl"), "file:///c:/Users/dev/example.pl");
        assert_eq!(
            uri_key("File://D:/projects/MyApp/script.pl"),
            "file:///d:/projects/MyApp/script.pl"
        );
    }

    #[test]
    fn normalizes_backslash_delimited_localhost_authority() {
        assert_eq!(
            uri_key(r"file://localhost\C:\Users\dev\example.pl"),
            "file:///c:/Users/dev/example.pl"
        );
        assert_eq!(
            uri_key(r"FILE://LocalHost\D:\projects\myapp\script.pl"),
            "file:///d:/projects/myapp/script.pl"
        );
    }

    #[test]
    fn canonical_file_uri_three_slashes_unchanged_by_legacy_pass() {
        // Canonical `file:///c:/...` must NOT be double-processed by the legacy pass.
        assert_eq!(uri_key("file:///c:/Users/dev/example.pl"), "file:///c:/Users/dev/example.pl");
        assert_eq!(uri_key("file:///C:/Users/dev/example.pl"), "file:///c:/Users/dev/example.pl");
    }

    #[test]
    fn normalizes_legacy_unc_windows_path() {
        assert_eq!(uri_key(r"\\server\share\folder\file.pl"), "file://server/share/folder/file.pl");
        assert_eq!(
            uri_key(r"file://\\server\share\folder\file.pl"),
            "file://server/share/folder/file.pl"
        );
    }

    #[test]
    fn percent_encodes_legacy_windows_drive_key_segments() {
        assert_eq!(
            uri_key(r"C:\Users\dev\path with spaces\name#frag?.pl"),
            "file:///c:/Users/dev/path%20with%20spaces/name%23frag%3F.pl"
        );
        assert_eq!(
            uri_key("file://D:/projects/ümlaut/module%.pm"),
            "file:///d:/projects/%C3%BCmlaut/module%25.pm"
        );
    }

    #[test]
    fn percent_encodes_legacy_unc_key_segments() {
        assert_eq!(
            uri_key(r"\\server\shared docs\folder#1\file?.pl"),
            "file://server/shared%20docs/folder%231/file%3F.pl"
        );
    }

    #[test]
    fn bare_incomplete_unc_paths_remain_unmodified() {
        assert_eq!(uri_key(r"\\server"), r"\\server");
    }

    #[test]
    fn normalizes_legacy_unc_share_roots() {
        assert_eq!(uri_key(r"\\server\share"), "file://server/share");
        assert_eq!(uri_key(r"file://\\server\share"), "file://server/share");
    }

    #[test]
    fn linux_paths_not_treated_as_windows() {
        // Linux absolute paths like `/home/user/file.pl` must not be misidentified
        // as Windows paths (index-1 byte is not `:`).
        assert_eq!(uri_key("/home/user/file.pl"), "/home/user/file.pl");
        assert_eq!(uri_key("file:///home/user/file.pl"), "file:///home/user/file.pl");
    }

    #[test]
    fn detects_file_uris() {
        assert!(is_file_uri("file:///tmp/test.pl"));
        assert!(is_file_uri("file://localhost/tmp/test.pl"));
        assert!(is_file_uri("FILE:///tmp/test.pl"));
        assert!(!is_file_uri("file:test.pl"));
        assert!(!is_file_uri("https://example.com"));
    }

    #[test]
    fn detects_special_schemes() {
        assert!(is_special_scheme("untitled:Untitled-1"));
        assert!(is_special_scheme("git:/foo/bar"));
        assert!(is_special_scheme("vscode-notebook-cell:/nb.ipynb#cell-id"));
        assert!(!is_special_scheme("file:///tmp/test.pl"));
    }

    #[test]
    fn detects_special_schemes_case_insensitive_fallback() {
        // Invalid URIs can still be recognized by prefix fallback, regardless of case.
        assert!(is_special_scheme("UNTITLED:Untitled-1"));
        assert!(is_special_scheme("GIT:relative/path"));
        assert!(is_special_scheme("VSCODE-NOTEBOOK-CELL:bad uri"));
    }

    #[test]
    fn detects_all_special_scheme_fallback_prefixes() {
        assert!(is_special_scheme("VSCODE-NOTEBOOK:invalid notebook uri"));
        assert!(is_special_scheme("VSCODE-VFS:invalid vfs uri"));
    }

    #[test]
    fn extracts_extensions() {
        assert_eq!(uri_extension("file:///tmp/test.pl"), Some("pl"));
        assert_eq!(uri_extension("file:///tmp/archive.tar.gz"), Some("gz"));
        assert_eq!(uri_extension("file:///tmp.with.dots/test"), None);
        assert_eq!(uri_extension("file:///tmp/file.pl?query=1"), Some("pl"));
        assert_eq!(uri_extension("file:///tmp/file.pl#L10/permalink"), Some("pl"));
        assert_eq!(uri_extension(r"C:\tmp\file.pl"), Some("pl"));
        assert_eq!(uri_extension(r"C:\Users\.bashrc"), None);
        assert_eq!(uri_extension(r"C:\Users\.gitignore"), None);
        assert_eq!(uri_extension("file:///tmp/trailing-dot."), None);
        assert_eq!(uri_extension("file:///tmp/file.pm/"), None);
        assert_eq!(uri_extension("file:///tmp/no-extension"), None);
    }
}
