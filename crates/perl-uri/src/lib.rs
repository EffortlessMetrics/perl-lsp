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

#[cfg(not(target_arch = "wasm32"))]
mod mojibake;
mod normalize;
#[cfg(not(target_arch = "wasm32"))]
mod path;

pub use normalize::normalize_uri;
#[cfg(not(target_arch = "wasm32"))]
pub use path::{fs_path_to_uri, source_path_from_uri_or_path, uri_to_fs_path};

/// URI classification and key normalization helpers (previously `perl-uri-classify`).
pub mod classify;
pub use classify::{is_file_uri, is_special_scheme, uri_extension, uri_key};

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
}
