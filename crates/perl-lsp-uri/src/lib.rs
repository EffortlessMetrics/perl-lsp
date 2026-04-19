#![warn(missing_docs)]
//! Typed URI parsing helpers for LSP components.

use lsp_types::Uri;

fn fallback_uri() -> Uri {
    for candidate in [
        "file:///unknown",
        "file:///",
        "about:blank",
        "urn:perl-lsp:unknown",
    ] {
        if let Ok(uri) = candidate.parse::<Uri>() {
            return uri;
        }
    }

    // Last-resort fallback that avoids panicking if URI parser behavior changes unexpectedly.
    let mut suffix = 0usize;
    loop {
        let candidate = format!("http://localhost/{suffix}");
        if let Ok(uri) = candidate.parse::<Uri>() {
            return uri;
        }
        suffix = suffix.saturating_add(1);
    }
}

/// Parse a URI string into [`lsp_types::Uri`].
///
/// Falls back to a guaranteed-valid URI if parsing fails.
#[must_use]
pub fn parse_uri(s: &str) -> Uri {
    match s.parse::<Uri>() {
        Ok(uri) => uri,
        Err(_) => fallback_uri(),
    }
}

#[cfg(test)]
mod tests {
    use super::{fallback_uri, parse_uri};

    // ── File URI parsing ──────────────────────────────────────────

    #[test]
    fn parse_uri_returns_original_for_valid_uri() {
        let uri = parse_uri("file:///tmp/test.pl");
        assert_eq!(uri.as_str(), "file:///tmp/test.pl");
    }

    #[test]
    fn parse_uri_unix_absolute_path() {
        let uri = parse_uri("file:///home/user/project/lib/Module.pm");
        assert_eq!(uri.as_str(), "file:///home/user/project/lib/Module.pm");
    }

    #[test]
    fn parse_uri_deeply_nested_path() {
        let uri = parse_uri("file:///a/b/c/d/e/f/g.pl");
        assert_eq!(uri.as_str(), "file:///a/b/c/d/e/f/g.pl");
    }

    #[test]
    fn parse_uri_file_root() {
        let uri = parse_uri("file:///");
        assert_eq!(uri.as_str(), "file:///");
    }

    // ── Windows paths ─────────────────────────────────────────────

    #[test]
    fn parse_uri_windows_drive_path() {
        let uri = parse_uri("file:///C:/Users/dev/project/file.pm");
        assert_eq!(uri.as_str(), "file:///C:/Users/dev/project/file.pm");
    }

    #[test]
    fn parse_uri_windows_lowercase_drive() {
        let uri = parse_uri("file:///c:/perl/lib/Module.pm");
        assert_eq!(uri.as_str(), "file:///c:/perl/lib/Module.pm");
    }

    #[test]
    fn parse_uri_windows_drive_root() {
        let uri = parse_uri("file:///D:/");
        assert_eq!(uri.as_str(), "file:///D:/");
    }

    // ── Percent-encoded URIs (spaces and special chars) ───────────

    #[test]
    fn parse_uri_percent_encoded_space() {
        let uri = parse_uri("file:///path/to/my%20module/Foo.pm");
        assert_eq!(uri.as_str(), "file:///path/to/my%20module/Foo.pm");
    }

    #[test]
    fn parse_uri_percent_encoded_special_chars() {
        let uri = parse_uri("file:///tmp/%E2%9C%93check.pl");
        assert_eq!(uri.as_str(), "file:///tmp/%E2%9C%93check.pl");
    }

    #[test]
    fn parse_uri_percent_encoded_hash() {
        // '#' must be encoded in paths
        let uri = parse_uri("file:///tmp/file%23name.pl");
        assert_eq!(uri.as_str(), "file:///tmp/file%23name.pl");
    }

    #[test]
    fn parse_uri_percent_encoded_windows_space() {
        let uri = parse_uri("file:///C:/My%20Documents/script.pl");
        assert_eq!(uri.as_str(), "file:///C:/My%20Documents/script.pl");
    }

    // ── Invalid URI fallback chain ────────────────────────────────

    #[test]
    fn parse_uri_falls_back_for_invalid_uri() {
        let uri = parse_uri("not a uri");
        assert!(!uri.as_str().is_empty());
    }

    #[test]
    fn parse_uri_empty_string_does_not_panic() {
        // An empty string may parse as a valid (empty) URI depending on the
        // URI library. The key invariant is that parse_uri never panics.
        let _uri = parse_uri("");
    }

    #[test]
    fn parse_uri_fallback_for_bare_path() {
        // A bare filesystem path (no scheme) may or may not parse depending on
        // the URI library. Either way parse_uri must not panic.
        let uri = parse_uri("/usr/local/lib/perl5/Foo.pm");
        assert!(!uri.as_str().is_empty());
    }

    #[test]
    fn parse_uri_fallback_for_whitespace_only() {
        let uri = parse_uri("   ");
        assert!(!uri.as_str().is_empty());
    }

    #[test]
    fn parse_uri_fallback_for_control_chars() {
        let uri = parse_uri("\x00\x01\x02");
        assert!(!uri.as_str().is_empty());
    }

    #[test]
    fn parse_uri_fallback_returns_valid_uri_string() {
        // The fallback must produce a string that starts with a known scheme
        let uri = parse_uri("definitely not valid %%% uri");
        let s = uri.as_str();
        assert!(
            s.starts_with("file:")
                || s.starts_with("about:")
                || s.starts_with("urn:")
                || s.starts_with("http:"),
            "fallback URI should have a recognized scheme, got: {s}"
        );
    }

    #[test]
    fn fallback_uri_returns_known_scheme() {
        let uri = fallback_uri();
        let s = uri.as_str();
        assert!(
            s.starts_with("file:")
                || s.starts_with("about:")
                || s.starts_with("urn:")
                || s.starts_with("http:"),
            "fallback_uri should produce a recognized scheme, got: {s}"
        );
    }

    #[test]
    fn fallback_uri_is_deterministic() {
        let a = fallback_uri();
        let b = fallback_uri();
        assert_eq!(a.as_str(), b.as_str());
    }

    // ── URI scheme preservation ───────────────────────────────────

    #[test]
    fn parse_uri_preserves_https_scheme() {
        let uri = parse_uri("https://example.com/docs/perl");
        assert_eq!(uri.as_str(), "https://example.com/docs/perl");
    }

    #[test]
    fn parse_uri_preserves_untitled_scheme() {
        // VS Code uses untitled: for unsaved buffers
        let uri = parse_uri("untitled:Untitled-1");
        assert_eq!(uri.as_str(), "untitled:Untitled-1");
    }

    // ── Round-trip: parse then as_str ─────────────────────────────

    #[test]
    fn parse_uri_round_trip_preserves_string() {
        let inputs = [
            "file:///tmp/test.pl",
            "file:///C:/Users/file.pm",
            "file:///path/with%20space/lib.pm",
            "https://example.com/resource",
        ];
        for input in inputs {
            let uri = parse_uri(input);
            assert_eq!(uri.as_str(), input, "round-trip failed for: {input}");
        }
    }

    // ── Edge cases ────────────────────────────────────────────────

    #[test]
    fn parse_uri_with_query_and_fragment() {
        let uri = parse_uri("file:///path/to/file.pm?line=10#L10");
        assert_eq!(uri.as_str(), "file:///path/to/file.pm?line=10#L10");
    }

    #[test]
    fn parse_uri_with_port() {
        let uri = parse_uri("http://localhost:8080/path");
        assert_eq!(uri.as_str(), "http://localhost:8080/path");
    }

    #[test]
    fn parse_uri_very_long_path() {
        let long_segment = "a".repeat(200);
        let input = format!("file:///{long_segment}/{long_segment}.pm");
        let uri = parse_uri(&input);
        assert_eq!(uri.as_str(), input);
    }
}
