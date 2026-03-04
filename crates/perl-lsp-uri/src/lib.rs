//! Panic-free `lsp_types::Uri` parsing helpers.

use lsp_types::Uri;

/// Build a fallback URI for parse failures.
///
/// This function intentionally guarantees returning a valid URI without panicking,
/// even if parser behavior changes in upstream dependencies.
pub fn fallback_uri() -> Uri {
    for candidate in ["file:///unknown", "file:///", "about:blank", "urn:perl-lsp:unknown"] {
        if let Ok(uri) = candidate.parse::<Uri>() {
            return uri;
        }
    }

    let mut suffix = 0usize;
    loop {
        let candidate = format!("http://localhost/{suffix}");
        if let Ok(uri) = candidate.parse::<Uri>() {
            return uri;
        }
        suffix = suffix.saturating_add(1);
    }
}

/// Parse a URI string into an `lsp_types::Uri`.
///
/// Returns [`fallback_uri`] when parsing fails.
pub fn parse_uri(input: &str) -> Uri {
    match input.parse::<Uri>() {
        Ok(uri) => uri,
        Err(_) => fallback_uri(),
    }
}

#[cfg(test)]
mod tests {
    use super::{fallback_uri, parse_uri};

    #[test]
    fn parse_uri_returns_parsed_value_when_valid() {
        let uri = parse_uri("file:///tmp/test.pl");
        assert_eq!(uri.as_str(), "file:///tmp/test.pl");
    }

    #[test]
    fn parse_uri_returns_fallback_when_invalid() {
        let parsed = parse_uri("not a uri");
        assert_eq!(parsed.as_str(), fallback_uri().as_str());
    }

    #[test]
    fn fallback_uri_is_valid_and_stable() {
        let uri = fallback_uri();
        assert_eq!(uri.as_str(), "file:///unknown");
    }
}
