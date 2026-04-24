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

    // Convert to filesystem path using the url crate's built-in method.
    // On Windows, accept rooted file URIs like file:///tmp/test.pl as \tmp\test.pl
    // so cross-platform tests and internal helpers stay permissive.
    let path = url.to_file_path().ok().or_else(|| windows_rooted_file_uri_to_path(&url))?;
    Some(repair_path_mojibake(path))
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
#[cfg(not(target_arch = "wasm32"))]
pub fn fs_path_to_uri<P: AsRef<std::path::Path>>(path: P) -> Result<String, String> {
    let path = normalize_filesystem_path(path.as_ref());

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

#[cfg(not(target_arch = "wasm32"))]
fn normalize_filesystem_path(path: &std::path::Path) -> std::path::PathBuf {
    #[cfg(windows)]
    {
        if let Some(path_str) = path.to_str() {
            if let Some(stripped) = path_str.strip_prefix(r"\\?\UNC\") {
                return std::path::PathBuf::from(format!(r"\\{}", stripped));
            }
            if let Some(stripped) = path_str.strip_prefix(r"\\?\") {
                return std::path::PathBuf::from(stripped);
            }
        }
    }

    path.to_path_buf()
}

#[cfg(all(not(target_arch = "wasm32"), windows))]
fn windows_rooted_file_uri_to_path(url: &Url) -> Option<std::path::PathBuf> {
    use percent_encoding::percent_decode_str;

    match url.host_str() {
        None | Some("localhost") => {}
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

    Some(std::path::PathBuf::from(native))
}

#[cfg(all(not(target_arch = "wasm32"), not(windows)))]
fn windows_rooted_file_uri_to_path(_url: &Url) -> Option<std::path::PathBuf> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn repair_path_mojibake(path: std::path::PathBuf) -> std::path::PathBuf {
    let Some(path_text) = path.to_str() else {
        return path;
    };

    let repaired = repair_mojibake_text(path_text);
    if repaired == path_text { path } else { std::path::PathBuf::from(repaired) }
}

#[cfg(not(target_arch = "wasm32"))]
fn repair_mojibake_text(text: &str) -> String {
    if !looks_like_mojibake(text) {
        return text.to_string();
    }

    let mut bytes = Vec::with_capacity(text.len());
    for ch in text.chars() {
        let code = u32::from(ch);
        let Ok(byte) = u8::try_from(code) else {
            return text.to_string();
        };
        bytes.push(byte);
    }

    let Ok(candidate) = String::from_utf8(bytes) else {
        return text.to_string();
    };

    if mojibake_marker_count(&candidate) < mojibake_marker_count(text) {
        candidate
    } else {
        text.to_string()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn looks_like_mojibake(text: &str) -> bool {
    mojibake_marker_count(text) > 0
}

#[cfg(not(target_arch = "wasm32"))]
fn mojibake_marker_count(text: &str) -> usize {
    text.chars().filter(|ch| matches!(ch, 'Ã' | 'Â' | 'â' | 'ð' | '�')).count()
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
    let path = std::path::Path::new(uri);

    // Raw absolute filesystem paths should normalize to file:// URIs before
    // URL parsing, especially on Windows where `C:\foo` can parse as `c:`.
    if path.is_absolute()
        && let Ok(uri_string) = fs_path_to_uri(path)
    {
        return uri_string;
    }

    // Try to parse as URL first
    if let Ok(url) = Url::parse(uri) {
        // Already a valid URI, return as-is
        return url.to_string();
    }

    // If not a valid URI, try to treat as a file path
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

/// URI classification and key normalization helpers (previously `perl-uri-classify`).
pub mod classify;
pub use classify::{is_file_uri, is_special_scheme, uri_extension, uri_key};

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
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
        use perl_tdd_support::{must, must_some};

        #[test]
        fn test_uri_to_fs_path_basic() {
            let path = uri_to_fs_path("file:///tmp/test.pl");
            assert!(path.is_some());
            let path = must_some(path);
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
            let path = must_some(path);
            let path_str = path.to_string_lossy();
            assert!(path_str.contains("path with spaces"));
        }

        #[test]
        fn test_uri_to_fs_path_repairs_common_mojibake() {
            let path = must_some(uri_to_fs_path("file:///tmp/caf%C3%83%C2%A9.pl"));
            let path_str = path.to_string_lossy();
            assert!(path_str.contains("café.pl"), "expected repaired UTF-8 path, got {path_str}");
        }

        #[test]
        fn test_fs_path_to_uri_basic() {
            let uri = must(fs_path_to_uri("/tmp/test.pl"));
            assert!(uri.starts_with("file:///"));
            assert!(uri.contains("test.pl"));
        }

        #[test]
        fn test_fs_path_to_uri_with_spaces() {
            let uri = must(fs_path_to_uri("/tmp/path with spaces/test.pl"));
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
        fn test_normalize_uri_absolute_path() {
            let path = std::env::temp_dir().join("normalize-uri-absolute.pl");
            let raw_path = path.to_string_lossy();
            let expected = must(fs_path_to_uri(&path));

            assert_eq!(normalize_uri(raw_path.as_ref()), expected);
        }

        #[test]
        fn test_roundtrip() {
            let original = "/tmp/roundtrip-test.pl";
            let uri = must(fs_path_to_uri(original));
            let path = must_some(uri_to_fs_path(&uri));
            assert!(path.ends_with("roundtrip-test.pl"));
        }
    }
}
