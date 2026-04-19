//! URI classification and key normalization helpers.
//!
//! This module centralizes URI helpers that are frequently reused by LSP-facing
//! crates while keeping filesystem URI conversion concerns in `perl-uri`.

#![warn(clippy::all)]

use url::Url;

/// Normalize a URI to a consistent key for lookups.
///
/// This function handles platform-specific differences to ensure consistent
/// lookups across different systems, particularly for Windows drive letters.
#[must_use]
pub fn uri_key(uri: &str) -> String {
    if let Ok(parsed) = Url::parse(uri) {
        let value = parsed.as_str().to_string();
        if let Some(rest) = value.strip_prefix("file:///")
            && rest.len() > 1
            && rest.as_bytes()[1] == b':'
            && rest.as_bytes()[0].is_ascii_alphabetic()
        {
            return format!("file:///{}{}", rest[0..1].to_ascii_lowercase(), &rest[1..]);
        }
        value
    } else {
        uri.to_string()
    }
}

/// Check if a URI uses the `file://` scheme.
#[must_use]
pub fn is_file_uri(uri: &str) -> bool {
    uri.starts_with("file://")
}

/// Check if a URI uses a special scheme (not `file://`).
#[must_use]
pub fn is_special_scheme(uri: &str) -> bool {
    if let Ok(url) = Url::parse(uri) {
        url.scheme() != "file"
    } else {
        uri.starts_with("untitled:")
            || uri.starts_with("git:")
            || uri.starts_with("vscode-notebook:")
            || uri.starts_with("vscode-vfs:")
    }
}

/// Extract the file extension from a URI-like string.
#[must_use]
pub fn uri_extension(uri: &str) -> Option<&str> {
    let path_part = uri.rsplit('/').next()?;
    let path_part = path_part.split('?').next()?;
    let path_part = path_part.split('#').next()?;
    let dot_pos = path_part.rfind('.')?;
    let ext = &path_part[dot_pos + 1..];
    if ext.is_empty() { None } else { Some(ext) }
}

#[cfg(test)]
mod tests {
    use super::{is_file_uri, is_special_scheme, uri_extension, uri_key};

    #[test]
    fn normalizes_uri_keys() {
        assert_eq!(uri_key("file:///tmp/test.pl"), "file:///tmp/test.pl");
        assert_eq!(
            uri_key("file:///C:/Users/test.pl"),
            "file:///c:/Users/test.pl"
        );
    }

    #[test]
    fn preserves_invalid_uri_values() {
        assert_eq!(uri_key("not-a-uri"), "not-a-uri");
    }

    #[test]
    fn detects_file_uris() {
        assert!(is_file_uri("file:///tmp/test.pl"));
        assert!(!is_file_uri("https://example.com"));
    }

    #[test]
    fn detects_special_schemes() {
        assert!(is_special_scheme("untitled:Untitled-1"));
        assert!(is_special_scheme("git:/foo/bar"));
        assert!(!is_special_scheme("file:///tmp/test.pl"));
    }

    #[test]
    fn extracts_extensions() {
        assert_eq!(uri_extension("file:///tmp/test.pl"), Some("pl"));
        assert_eq!(uri_extension("file:///tmp/file.pl?query=1"), Some("pl"));
        assert_eq!(uri_extension("file:///tmp/no-extension"), None);
    }
}
