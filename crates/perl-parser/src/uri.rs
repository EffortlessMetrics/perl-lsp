//! URI utilities for LSP

use lsp_types::Uri;

/// Helper function to parse a URI string into an lsp_types::Uri
/// Falls back to a valid URI if parsing fails
pub fn parse_uri(s: &str) -> Uri {
    s.parse::<Uri>().unwrap_or_else(|_| {
        // This is a guaranteed valid URI as a fallback
        "file:///invalid".parse::<Uri>()
            .unwrap_or_else(|_| panic!("hardcoded fallback URI is invalid - this is a bug in the LSP URI parser"))
    })
}
