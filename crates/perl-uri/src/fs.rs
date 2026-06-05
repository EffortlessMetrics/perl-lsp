//! Filesystem path and `file://` URI conversion helpers.

use std::path::{Path, PathBuf};

use url::Url;

use crate::mojibake::repair_path_mojibake;

/// Convert a `file://` URI to a filesystem path.
///
/// Properly handles percent-encoding and works with spaces, Windows paths,
/// and non-ASCII characters. Returns `None` if the URI is not a valid `file://` URI.
///
/// # Examples
///
/// ```
/// # #[cfg(not(target_arch = "wasm32"))]
/// # fn main() {
/// use perl_uri::uri_to_fs_path;
///
/// // Basic file URI
/// let path = uri_to_fs_path("file:///tmp/test.pl");
/// assert!(path.is_some());
///
/// // URI with percent-encoded spaces
/// let path = uri_to_fs_path("file:///tmp/path%20with%20spaces/test.pl");
/// assert!(path.is_some());
///
/// // Non-file URIs return None
/// let path = uri_to_fs_path("https://example.com");
/// assert!(path.is_none());
/// # }
/// # #[cfg(target_arch = "wasm32")]
/// # fn main() {}
/// ```
///
/// # Platform Support
///
/// This function is not available on `wasm32` targets (no filesystem).
pub fn uri_to_fs_path(uri: &str) -> Option<PathBuf> {
    let normalized = crate::classify::normalize_legacy_windows_uri(uri);
    let parse_input = normalized.as_deref().unwrap_or(uri);
    let url = Url::parse(parse_input).ok()?;

    // Only handle file:// URIs
    if url.scheme() != "file" {
        return None;
    }

    // Convert to filesystem path using the url crate's built-in method.
    // On Windows, accept rooted file URIs like file:///tmp/test.pl as \tmp\test.pl
    // so cross-platform tests and internal helpers stay permissive.
    let path = url
        .to_file_path()
        .ok()
        .or_else(|| local_authority_file_uri_to_path(&url))
        .or_else(|| windows_rooted_file_uri_to_path(&url))?;
    Some(repair_path_mojibake(path))
}

/// Convert either a `file://` URI or absolute filesystem path to a source path.
///
/// This helper accepts:
/// - absolute native paths (including Windows drive paths)
/// - `file://` URIs that map to local filesystem paths
///
/// It returns `None` for non-file schemes, invalid inputs, and relative paths.
pub fn source_path_from_uri_or_path(input: &str) -> Option<PathBuf> {
    let trimmed = input.trim();
    let path = Path::new(trimmed);
    if path.is_absolute() {
        return Some(path.to_path_buf());
    }

    uri_to_fs_path(trimmed)
}

/// Convert a filesystem path to a `file://` URI.
///
/// Properly handles percent-encoding and works with spaces, Windows paths,
/// and non-ASCII characters.
///
/// # Examples
///
/// ```
/// # #[cfg(not(target_arch = "wasm32"))]
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use perl_uri::fs_path_to_uri;
///
/// // Absolute path
/// let uri = fs_path_to_uri("/tmp/test.pl")?;
/// assert!(uri.starts_with("file:///"));
///
/// // Path with spaces gets percent-encoded
/// let uri = fs_path_to_uri("/tmp/path with spaces/test.pl")?;
/// assert!(uri.contains("%20"));
/// # Ok(())
/// # }
/// # #[cfg(target_arch = "wasm32")]
/// # fn main() {}
/// ```
///
/// # Errors
///
/// Returns an error if the path cannot be converted to an absolute path
/// or if the conversion to a URI fails.
///
/// # Platform Support
///
/// This function is not available on `wasm32` targets (no filesystem).
pub fn fs_path_to_uri<P: AsRef<Path>>(path: P) -> Result<String, String> {
    let path = normalize_filesystem_path(path.as_ref());

    // Convert to absolute path if relative
    let abs_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| format!("Failed to get current directory: {e}"))?
            .join(path)
    };

    // Use the url crate's built-in method to create a proper file:// URI
    Url::from_file_path(&abs_path)
        .map(|url| url.to_string())
        .map_err(|_| format!("Failed to convert path to URI: {}", abs_path.display()))
}

fn normalize_filesystem_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(path_str) = path.to_str() {
            if let Some(stripped) = path_str.strip_prefix(r"\\?\UNC\") {
                return PathBuf::from(format!(r"\\{}", stripped));
            }
            if let Some(stripped) = path_str.strip_prefix(r"\\?\") {
                return PathBuf::from(stripped);
            }
        }
    }

    path.to_path_buf()
}

fn local_authority_file_uri_to_path(url: &Url) -> Option<PathBuf> {
    if !crate::classify::is_local_file_authority(url.host_str()) {
        return None;
    }

    let canonical = Url::parse(&format!("file://{}", url.path())).ok()?;
    canonical.to_file_path().ok()
}

#[cfg(windows)]
fn windows_rooted_file_uri_to_path(url: &Url) -> Option<PathBuf> {
    use percent_encoding::percent_decode_str;

    match url.host_str() {
        None => {}
        host if crate::classify::is_local_file_authority(host) => {}
        Some(_) => return None,
    }

    let decoded = percent_decode_str(url.path()).decode_utf8().ok()?;
    if decoded.is_empty() {
        return None;
    }

    let native = if decoded.len() > 3
        && decoded.starts_with('/')
        && decoded.as_bytes()[2] == b':'
        && decoded.as_bytes()[1].is_ascii_alphabetic()
    {
        decoded[1..].replace('/', "\\")
    } else {
        decoded.replace('/', "\\")
    };

    Some(PathBuf::from(native))
}

#[cfg(not(windows))]
fn windows_rooted_file_uri_to_path(_url: &Url) -> Option<PathBuf> {
    None
}
