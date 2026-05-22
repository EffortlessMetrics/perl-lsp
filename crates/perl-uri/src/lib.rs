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
mod fs;
#[cfg(not(target_arch = "wasm32"))]
mod mojibake;
mod normalize;

#[cfg(not(target_arch = "wasm32"))]
pub use fs::{fs_path_to_uri, source_path_from_uri_or_path, uri_to_fs_path};
pub use normalize::normalize_uri;

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
        fn test_normalize_uri_canonicalizes_localhost_authority() {
            assert_eq!(normalize_uri("file://localhost/tmp/test.pl"), "file:///tmp/test.pl");
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
            let expected = uri_key(&must(fs_path_to_uri(&path)));

            assert_eq!(normalize_uri(raw_path.as_ref()), expected);
        }

        #[test]
        fn test_roundtrip() {
            let original = "/tmp/roundtrip-test.pl";
            let uri = must(fs_path_to_uri(original));
            let path = must_some(uri_to_fs_path(&uri));
            assert!(path.ends_with("roundtrip-test.pl"));
        }

        #[test]
        fn test_source_path_from_uri_or_path_rejects_non_file_scheme() {
            assert!(source_path_from_uri_or_path("https://example.com/a.pl").is_none());
        }

        #[test]
        fn test_source_path_from_uri_or_path_rejects_relative_path() {
            assert!(source_path_from_uri_or_path("lib/Foo.pm").is_none());
        }

        #[test]
        fn test_source_path_from_uri_or_path_accepts_file_uri_and_absolute_path() {
            let from_uri = must_some(source_path_from_uri_or_path("file:///tmp/from-uri.pl"));
            assert!(from_uri.ends_with("from-uri.pl"));

            let absolute_path = std::env::temp_dir().join("from-path.pl");
            let from_path =
                must_some(source_path_from_uri_or_path(absolute_path.to_string_lossy().as_ref()));
            assert!(from_path.ends_with("from-path.pl"));
        }

        #[test]
        fn test_source_path_from_uri_or_path_localhost_file_uri() {
            let path = must_some(source_path_from_uri_or_path("file://localhost/tmp/localhost.pl"));
            assert!(path.ends_with("localhost.pl"));
        }

        #[test]
        fn test_source_path_from_uri_or_path_trims_surrounding_whitespace() {
            let from_uri =
                must_some(source_path_from_uri_or_path(" \nfile:///tmp/trimmed-uri.pl\t"));
            assert!(from_uri.ends_with("trimmed-uri.pl"));

            let absolute_path = std::env::temp_dir().join("trimmed-path.pl");
            let input = format!("  {}  ", absolute_path.display());
            let from_path = must_some(source_path_from_uri_or_path(&input));
            assert!(from_path.ends_with("trimmed-path.pl"));
        }

        #[cfg(windows)]
        #[test]
        fn test_source_path_from_uri_or_path_accepts_windows_drive_path() {
            let path = must_some(source_path_from_uri_or_path("C:\\tmp\\drive-path.pl"));
            assert!(path.ends_with("drive-path.pl"));
        }
    }
}
