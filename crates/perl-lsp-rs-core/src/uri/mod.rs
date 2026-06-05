//! Typed URI parsing helpers for LSP components.
//!
//! Previously the standalone `perl-lsp-uri` crate; absorbed into
//! `perl-lsp-rs-core::uri` in Wave G3 (#4535).

use lsp_types::Uri;
use std::path::Path;
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

fn file_path_uri(s: &str) -> Option<Uri> {
    let url = Url::from_file_path(Path::new(s)).ok()?;
    url.as_str().parse::<Uri>().ok()
}

fn windows_file_path_uri(s: &str) -> Option<Uri> {
    let mut chars = s.chars();
    let drive = chars.next()?;
    if !drive.is_ascii_alphabetic() || chars.next()? != ':' {
        return None;
    }
    let separator = chars.next()?;
    if separator != '\\' && separator != '/' {
        return None;
    }

    let normalized = s.replace('\\', "/");
    let url = Url::parse(&format!("file:///{normalized}")).ok()?;
    url.as_str().parse::<Uri>().ok()
}

/// Parse a URI string into [`lsp_types::Uri`].
///
/// Accepts valid URI strings and absolute local file paths. Falls back to a
/// guaranteed-valid URI if parsing fails.
#[must_use]
pub fn parse_uri(s: &str) -> Uri {
    let sanitized = s.trim_start_matches('\u{feff}').trim();

    if sanitized.is_empty() {
        return fallback_uri();
    }

    if let Some(uri) = file_path_uri(sanitized).or_else(|| windows_file_path_uri(sanitized)) {
        return uri;
    }

    match sanitized.parse::<Uri>() {
        Ok(uri) => uri,
        Err(_) => Url::parse(sanitized)
            .ok()
            .and_then(|url| url.as_str().parse::<Uri>().ok())
            .unwrap_or_else(fallback_uri),
    }
}
