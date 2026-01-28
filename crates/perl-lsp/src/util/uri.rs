//! URI utilities for LSP

use once_cell::sync::Lazy;
use lsp_types::Uri;

/// Fallback URI for when parsing fails. "file:///invalid" is a valid URI by spec.
static FALLBACK_URI: Lazy<Option<Uri>> = Lazy::new(|| "file:///invalid".parse().ok());

/// Helper function to parse a URI string into an lsp_types::Uri
/// Falls back to a valid URI if parsing fails
pub fn parse_uri(s: &str) -> Uri {
    s.parse::<Uri>()
        .ok()
        .or_else(|| FALLBACK_URI.clone())
        .or_else(|| "file:///".parse().ok())
        // Final fallback - if all else fails, return the original parse result (which will fail)
        // This branch is structurally unreachable since "file:///" is always valid
        .unwrap_or_else(|| s.parse().unwrap_or_else(|_| {
            // Construct a minimal valid URI by re-attempting parse
            // This is only reached if the URL parser is broken
            #[allow(clippy::expect_used)]
            "file:".parse().expect("file: is always a valid URI scheme")
        }))
}
