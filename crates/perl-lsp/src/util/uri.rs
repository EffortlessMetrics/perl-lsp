//! URI utilities for LSP

use lsp_types::Uri;
use lazy_static::lazy_static;

lazy_static! {
    static ref FALLBACK_URI: Uri = "file:///invalid".parse().unwrap_or_else(|e| panic!("Invalid fallback URI: {e}"));
}

/// Helper function to parse a URI string into an lsp_types::Uri
/// Falls back to a valid URI if parsing fails
pub fn parse_uri(s: &str) -> Uri {
    s.parse::<Uri>().unwrap_or_else(|_| FALLBACK_URI.clone())
}
