//! URI normalization entry points.

use url::Url;

/// Normalize a URI to a consistent form.
///
/// This function handles various URI formats and normalizes them:
/// - Valid URIs are parsed and re-serialized
/// - File paths are converted to `file://` URIs
/// - Malformed `file://` URIs are reconstructed
/// - Legacy Windows `file://C:\...` forms are canonicalized to `file:///c:/...`
/// - Special URIs (e.g., `untitled:`) are preserved as-is
///
/// # Examples
///
/// ```
/// # #[cfg(not(target_arch = "wasm32"))]
/// # fn main() {
/// use perl_uri::normalize_uri;
///
/// // Already valid URI
/// let uri = normalize_uri("file:///tmp/test.pl");
/// assert_eq!(uri, "file:///tmp/test.pl");
///
/// // Special schemes preserved
/// let uri = normalize_uri("untitled:Untitled-1");
/// assert_eq!(uri, "untitled:Untitled-1");
/// # }
/// # #[cfg(target_arch = "wasm32")]
/// # fn main() {}
/// ```
///
/// # Platform Support
///
/// The full implementation is only available on non-`wasm32` targets.
/// On `wasm32`, legacy Windows URI normalization and URI parsing are performed
/// without filesystem operations.
#[cfg(not(target_arch = "wasm32"))]
pub fn normalize_uri(uri: &str) -> String {
    let trimmed = uri.trim();

    if trimmed.is_empty() {
        return String::new();
    }

    if let Some(normalized) = crate::classify::normalize_legacy_windows_uri(trimmed) {
        return normalized;
    }

    let path = std::path::Path::new(trimmed);

    // Raw absolute filesystem paths should normalize to file:// URIs before
    // URL parsing, especially on Windows where `C:\foo` can parse as `c:`.
    if path.is_absolute()
        && let Ok(uri_string) = crate::fs_path_to_uri(path)
    {
        return uri_string;
    }

    // Try to parse as URL first
    if let Ok(url) = Url::parse(trimmed) {
        // Canonicalize local file URIs through filesystem conversion so legacy
        // forms like `file://C:/...` normalize to `file:///c:/...` on Windows
        // and `file:///tmp/...` on Unix while preserving non-local authorities.
        if url.scheme() == "file"
            && url.host_str() == Some("localhost")
            && let Some(fs_path) = crate::uri_to_fs_path(trimmed)
            && let Ok(normalized) = crate::fs_path_to_uri(&fs_path)
        {
            return normalized;
        }

        // Already a valid non-file URI, return as-is.
        return url.to_string();
    }

    // If not a valid URI, try to treat as a file path
    // Try to convert path to URI using our helper function
    if let Ok(uri_string) = crate::fs_path_to_uri(path) {
        return uri_string;
    }

    // Last resort: if it looks like a file:// URI but is malformed,
    // try to extract the path and reconstruct properly
    if trimmed.starts_with("file://")
        && let Some(fs_path) = crate::uri_to_fs_path(trimmed)
        && let Ok(normalized) = crate::fs_path_to_uri(&fs_path)
    {
        return normalized;
    }

    // Final fallback: return as-is for special URIs like untitled:
    trimmed.to_string()
}

/// Normalize a URI to a consistent form (wasm32 version - no filesystem).
#[cfg(target_arch = "wasm32")]
pub fn normalize_uri(uri: &str) -> String {
    let trimmed = uri.trim();

    if trimmed.is_empty() {
        return String::new();
    }

    if let Some(normalized) = crate::classify::normalize_legacy_windows_uri(trimmed) {
        return normalized;
    }

    // On wasm32, just try to parse as URL or return as-is
    if let Ok(url) = Url::parse(trimmed) { url.to_string() } else { trimmed.to_string() }
}
