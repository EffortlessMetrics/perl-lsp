//! URI utilities for LSP

use lsp_types::Uri;
use std::sync::LazyLock;

/// Fallback URI for when parsing fails.
/// Invariant: "file:///unknown" is a valid URI per RFC 3986.
static FALLBACK_URI: LazyLock<Uri> = LazyLock::new(|| {
    "file:///unknown".parse().unwrap_or_else(|_| unreachable!("file:///unknown must be a valid URI"))
});

/// Helper function to parse a URI string into an lsp_types::Uri.
/// Falls back to a valid URI if parsing fails.
pub fn parse_uri(s: &str) -> Uri {
    match s.parse::<Uri>() {
        Ok(uri) => uri,
        Err(_) => FALLBACK_URI.clone(),
    }
}
