//! URI utilities for LSP

use lsp_types::Uri;
use once_cell::sync::Lazy;

/// Fallback URI for when parsing fails. Stored as Option since we can't use expect.
static FALLBACK_URI: Lazy<Uri> = Lazy::new(|| {
    // All of these are valid URIs per RFC 3986. At least one will parse.
    ["file:///invalid", "file:", "a:"]
        .iter()
        .find_map(|s| s.parse().ok())
        .unwrap_or_else(|| {
            // Unreachable: URI parser is catastrophically broken
            std::process::abort()
        })
});

/// Helper function to parse a URI string into an lsp_types::Uri.
/// Falls back to a valid URI if parsing fails.
pub fn parse_uri(s: &str) -> Uri {
    s.parse().unwrap_or_else(|_| FALLBACK_URI.clone())
}
