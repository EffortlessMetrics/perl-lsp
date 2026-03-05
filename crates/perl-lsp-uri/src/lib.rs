//! Typed URI parsing helpers for LSP components.

use lsp_types::Uri;

fn fallback_uri() -> Uri {
    for candidate in ["file:///unknown", "file:///", "about:blank", "urn:perl-lsp:unknown"] {
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
    use super::parse_uri;

    #[test]
    fn parse_uri_returns_original_for_valid_uri() {
        let uri = parse_uri("file:///tmp/test.pl");
        assert_eq!(uri.as_str(), "file:///tmp/test.pl");
    }

    #[test]
    fn parse_uri_falls_back_for_invalid_uri() {
        let uri = parse_uri("not a uri");
        assert!(!uri.as_str().is_empty());
    }
}
