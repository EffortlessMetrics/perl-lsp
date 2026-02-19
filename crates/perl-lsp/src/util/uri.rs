//! URI utilities for LSP

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

/// Helper function to parse a URI string into an lsp_types::Uri.
/// Falls back to a valid URI if parsing fails.
pub fn parse_uri(s: &str) -> Uri {
    match s.parse::<Uri>() {
        Ok(uri) => uri,
        Err(_) => fallback_uri(),
    }
}
