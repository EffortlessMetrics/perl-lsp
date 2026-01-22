//! URI ↔ filesystem path conversion and normalization utilities.
//!
//! This crate provides consistent URI handling for the Perl LSP ecosystem,
//! including:
//!
//! - Converting between `file://` URIs and filesystem paths
//! - Windows drive-letter normalization
//! - Percent encoding/decoding
//! - Special scheme handling (`untitled:`, etc.)
//!
//! # Platform Support
//!
//! Most functions are not available on `wasm32` targets since they require
//! filesystem access.
//!
//! # Examples
//!
//! ```
//! # #[cfg(not(target_arch = "wasm32"))]
//! # fn main() {
//! use perl_uri::{uri_to_fs_path, fs_path_to_uri};
//!
//! // Convert a URI to a path
//! let path = uri_to_fs_path("file:///tmp/test.pl");
//! assert!(path.is_some());
//!
//! // Convert a path to a URI
//! let uri = fs_path_to_uri("/tmp/test.pl");
//! assert!(uri.is_ok());
//! # }
//! # #[cfg(target_arch = "wasm32")]
//! # fn main() {}
//! ```

use url::Url;

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
#[cfg(not(target_arch = "wasm32"))]
pub fn uri_to_fs_path(uri: &str) -> Option<std::path::PathBuf> {
    // Parse the URI
    let url = Url::parse(uri).ok()?;

    // Only handle file:// URIs
    if url.scheme() != "file" {
        return None;
    }

    // Convert to filesystem path using the url crate's built-in method
    url.to_file_path().ok()
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
/// # fn main() {
/// use perl_uri::fs_path_to_uri;
///
/// // Absolute path
/// let uri = fs_path_to_uri("/tmp/test.pl").unwrap();
/// assert!(uri.starts_with("file:///"));
///
/// // Path with spaces gets percent-encoded
/// let uri = fs_path_to_uri("/tmp/path with spaces/test.pl").unwrap();
/// assert!(uri.contains("%20"));
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
#[cfg(not(target_arch = "wasm32"))]
pub fn fs_path_to_uri<P: AsRef<std::path::Path>>(path: P) -> Result<String, String> {
    let path = path.as_ref();

    // Convert to absolute path if relative
    let abs_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| format!("Failed to get current directory: {}", e))?
            .join(path)
    };

    // Use the url crate's built-in method to create a proper file:// URI
    Url::from_file_path(&abs_path)
        .map(|url| url.to_string())
        .map_err(|_| format!("Failed to convert path to URI: {}", abs_path.display()))
}

/// Normalize a URI to a consistent form.
///
/// This function handles various URI formats and normalizes them:
/// - Valid URIs are parsed and re-serialized
/// - File paths are converted to `file://` URIs
/// - Malformed `file://` URIs are reconstructed
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
/// On `wasm32`, only URI parsing is performed without filesystem operations.
#[cfg(not(target_arch = "wasm32"))]
pub fn normalize_uri(uri: &str) -> String {
    // Try to parse as URL first
    if let Ok(url) = Url::parse(uri) {
        // Already a valid URI, return as-is
        return url.to_string();
    }

    // If not a valid URI, try to treat as a file path
    let path = std::path::Path::new(uri);

    // Try to convert path to URI using our helper function
    if let Ok(uri_string) = fs_path_to_uri(path) {
        return uri_string;
    }

    // Last resort: if it looks like a file:// URI but is malformed,
    // try to extract the path and reconstruct properly
    if uri.starts_with("file://")
        && let Some(fs_path) = uri_to_fs_path(uri)
        && let Ok(normalized) = fs_path_to_uri(&fs_path)
    {
        return normalized;
    }

    // Final fallback: return as-is for special URIs like untitled:
    uri.to_string()
}

/// Normalize a URI to a consistent form (wasm32 version - no filesystem).
#[cfg(target_arch = "wasm32")]
pub fn normalize_uri(uri: &str) -> String {
    // On wasm32, just try to parse as URL or return as-is
    if let Ok(url) = Url::parse(uri) { url.to_string() } else { uri.to_string() }
}

/// Normalize a URI to a consistent key for lookups.
///
/// This function handles platform-specific differences to ensure consistent
/// lookups across different systems, particularly for Windows drive letters.
///
/// # Windows Drive Letter Normalization
///
/// On Windows, drive letters in URIs may be uppercase or lowercase.
/// This function normalizes them to lowercase for consistent lookups:
/// - `file:///C:/foo` → `file:///c:/foo`
/// - `file:///D:/bar` → `file:///d:/bar`
///
/// # Examples
///
/// ```
/// use perl_uri::uri_key;
///
/// // Standard URI
/// let key = uri_key("file:///tmp/test.pl");
/// assert_eq!(key, "file:///tmp/test.pl");
///
/// // Windows URI with uppercase drive
/// let key = uri_key("file:///C:/Users/test.pl");
/// assert_eq!(key, "file:///c:/Users/test.pl");
///
/// // Invalid URI returned as-is
/// let key = uri_key("not-a-uri");
/// assert_eq!(key, "not-a-uri");
/// ```
pub fn uri_key(uri: &str) -> String {
    if let Ok(u) = Url::parse(uri) {
        let s = u.as_str().to_string();
        if let Some(rest) = s.strip_prefix("file:///") {
            // Check for Windows drive letter pattern: single letter followed by colon
            if rest.len() > 1
                && rest.as_bytes()[1] == b':'
                && rest.as_bytes()[0].is_ascii_alphabetic()
            {
                // Normalize drive letter to lowercase
                return format!("file:///{}{}", rest[0..1].to_ascii_lowercase(), &rest[1..]);
            }
        }
        s
    } else {
        uri.to_string()
    }
}

/// Check if a URI uses the `file://` scheme.
///
/// # Examples
///
/// ```
/// use perl_uri::is_file_uri;
///
/// assert!(is_file_uri("file:///tmp/test.pl"));
/// assert!(!is_file_uri("https://example.com"));
/// assert!(!is_file_uri("untitled:Untitled-1"));
/// ```
pub fn is_file_uri(uri: &str) -> bool {
    uri.starts_with("file://")
}

/// Check if a URI uses a special scheme (not `file://`).
///
/// Special schemes include:
/// - `untitled:` - Unsaved documents
/// - `vscode-notebook:` - VS Code notebooks
/// - `git:` - Git diff views
/// - etc.
///
/// # Examples
///
/// ```
/// use perl_uri::is_special_scheme;
///
/// assert!(is_special_scheme("untitled:Untitled-1"));
/// assert!(is_special_scheme("git:/foo/bar"));
/// assert!(!is_special_scheme("file:///tmp/test.pl"));
/// ```
pub fn is_special_scheme(uri: &str) -> bool {
    if let Ok(url) = Url::parse(uri) {
        url.scheme() != "file"
    } else {
        // If it can't be parsed as a URL, check for common special prefixes
        uri.starts_with("untitled:")
            || uri.starts_with("git:")
            || uri.starts_with("vscode-notebook:")
            || uri.starts_with("vscode-vfs:")
    }
}

/// Extract the file extension from a URI.
///
/// # Examples
///
/// ```
/// use perl_uri::uri_extension;
///
/// assert_eq!(uri_extension("file:///tmp/test.pl"), Some("pl"));
/// assert_eq!(uri_extension("file:///tmp/Module.pm"), Some("pm"));
/// assert_eq!(uri_extension("file:///tmp/no-extension"), None);
/// ```
pub fn uri_extension(uri: &str) -> Option<&str> {
    // Find the last path segment
    let path_part = uri.rsplit('/').next()?;
    // Remove query string and fragment
    let path_part = path_part.split('?').next()?;
    let path_part = path_part.split('#').next()?;
    // Find the extension
    let dot_pos = path_part.rfind('.')?;
    let ext = &path_part[dot_pos + 1..];
    if ext.is_empty() { None } else { Some(ext) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uri_key_basic() {
        assert_eq!(uri_key("file:///tmp/test.pl"), "file:///tmp/test.pl");
    }

    #[test]
    fn test_uri_key_windows_drive() {
        assert_eq!(uri_key("file:///C:/Users/test.pl"), "file:///c:/Users/test.pl");
        assert_eq!(uri_key("file:///D:/foo/bar.pm"), "file:///d:/foo/bar.pm");
    }

    #[test]
    fn test_uri_key_invalid() {
        assert_eq!(uri_key("not-a-uri"), "not-a-uri");
    }

    #[test]
    fn test_is_file_uri() {
        assert!(is_file_uri("file:///tmp/test.pl"));
        assert!(!is_file_uri("https://example.com"));
        assert!(!is_file_uri("untitled:Untitled-1"));
    }

    #[test]
    fn test_is_special_scheme() {
        assert!(is_special_scheme("untitled:Untitled-1"));
        assert!(!is_special_scheme("file:///tmp/test.pl"));
    }

    #[test]
    fn test_uri_extension() {
        assert_eq!(uri_extension("file:///tmp/test.pl"), Some("pl"));
        assert_eq!(uri_extension("file:///tmp/Module.pm"), Some("pm"));
        assert_eq!(uri_extension("file:///tmp/script.t"), Some("t"));
        assert_eq!(uri_extension("file:///tmp/no-extension"), None);
        assert_eq!(uri_extension("file:///tmp/file.pl?query=1"), Some("pl"));
    }

    #[cfg(not(target_arch = "wasm32"))]
    mod filesystem_tests {
        use super::*;

        #[test]
        fn test_uri_to_fs_path_basic() {
            let path = uri_to_fs_path("file:///tmp/test.pl");
            assert!(path.is_some());
            let path = path.unwrap();
            assert!(path.ends_with("test.pl"));
        }

        #[test]
        fn test_uri_to_fs_path_non_file() {
            assert!(uri_to_fs_path("https://example.com").is_none());
            assert!(uri_to_fs_path("untitled:Untitled-1").is_none());
        }

        #[test]
        fn test_uri_to_fs_path_with_spaces() {
            let path = uri_to_fs_path("file:///tmp/path%20with%20spaces/test.pl");
            assert!(path.is_some());
            let path = path.unwrap();
            let path_str = path.to_string_lossy();
            assert!(path_str.contains("path with spaces"));
        }

        #[test]
        fn test_fs_path_to_uri_basic() {
            let uri = fs_path_to_uri("/tmp/test.pl").unwrap();
            assert!(uri.starts_with("file:///"));
            assert!(uri.contains("test.pl"));
        }

        #[test]
        fn test_fs_path_to_uri_with_spaces() {
            let uri = fs_path_to_uri("/tmp/path with spaces/test.pl").unwrap();
            assert!(uri.contains("%20") || uri.contains("path with spaces"));
        }

        #[test]
        fn test_normalize_uri_valid() {
            let uri = normalize_uri("file:///tmp/test.pl");
            assert_eq!(uri, "file:///tmp/test.pl");
        }

        #[test]
        fn test_normalize_uri_special() {
            let uri = normalize_uri("untitled:Untitled-1");
            assert_eq!(uri, "untitled:Untitled-1");
        }

        #[test]
        fn test_roundtrip() {
            let original = "/tmp/roundtrip-test.pl";
            let uri = fs_path_to_uri(original).unwrap();
            let path = uri_to_fs_path(&uri).unwrap();
            assert!(path.ends_with("roundtrip-test.pl"));
        }
    }
}
