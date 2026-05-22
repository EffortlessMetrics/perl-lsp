//! Typed URI parsing helpers for LSP components.
//!
//! Previously the standalone `perl-lsp-uri` crate; absorbed into
//! `perl-lsp-rs-core::uri` in Wave G3 (#4535).

use lsp_types::Uri;
use url::Url;

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
    let sanitized = s.trim_start_matches('\u{feff}').trim();

    match sanitized.parse::<Uri>() {
        Ok(uri) => uri,
        Err(_) => Url::parse(sanitized)
            .ok()
            .and_then(|url| url.as_str().parse::<Uri>().ok())
            .unwrap_or_else(fallback_uri),
    }
}
