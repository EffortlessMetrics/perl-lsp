//! Behavior-driven integration tests for `perl-uri`.
//!
//! These tests focus on caller-observable workflows using a
//! Given/When/Then structure so future changes preserve user-facing behavior.

#[cfg(not(target_arch = "wasm32"))]
mod bdd {
    use perl_tdd_support::{must, must_some};
    use perl_uri::{
        fs_path_to_uri, is_file_uri, is_special_scheme, normalize_uri, uri_extension, uri_key,
        uri_to_fs_path,
    };

    #[test]
    fn given_unix_file_uri_when_converted_then_path_and_extension_are_preserved() {
        // Given
        let source_uri = "file:///tmp/perl-lsp/lib/Project.pm";

        // When
        let fs_path = must_some(uri_to_fs_path(source_uri));
        let ext = uri_extension(source_uri);

        // Then
        assert!(fs_path.ends_with("Project.pm"));
        assert_eq!(ext, Some("pm"));
        assert!(is_file_uri(source_uri));
        assert!(!is_special_scheme(source_uri));
    }

    #[test]
    fn given_filesystem_path_with_spaces_when_normalized_then_cache_key_is_stable() {
        // Given
        let source_path = "/tmp/perl lsp/workspace/main script.pl";

        // When
        let uri = must(fs_path_to_uri(source_path));
        let normalized = normalize_uri(&uri);
        let key_from_uri = uri_key(&uri);
        let key_from_normalized = uri_key(&normalized);

        // Then
        assert!(uri.starts_with("file:///"));
        assert!(uri.contains("%20"));
        assert_eq!(normalized, uri);
        assert_eq!(key_from_uri, key_from_normalized);
    }

    #[test]
    fn given_special_editor_resource_when_normalized_then_scheme_semantics_are_preserved() {
        // Given
        let uri = "untitled:Scratch-1";

        // When
        let normalized = normalize_uri(uri);

        // Then
        assert_eq!(normalized, uri);
        assert!(is_special_scheme(uri));
        assert!(!is_file_uri(uri));
        assert_eq!(uri_extension(uri), None);
    }

    #[test]
    fn given_windows_drive_file_uri_when_keyed_then_drive_letter_is_canonicalized() {
        // Given
        let uri = "file:///C:/Users/Dev/MyModule.pm";

        // When
        let key = uri_key(uri);
        let normalized = normalize_uri(uri);

        // Then
        assert_eq!(key, "file:///c:/Users/Dev/MyModule.pm");
        assert_eq!(normalized, uri);
        assert_eq!(uri_extension(&key), Some("pm"));
    }

    #[test]
    fn given_roundtrip_through_protocol_boundary_when_converted_then_original_path_returns() {
        // Given
        let original = "/tmp/perl-uri/bdd/roundtrip test.t";

        // When
        let uri = must(fs_path_to_uri(original));
        let back = must_some(uri_to_fs_path(&uri));

        // Then
        assert_eq!(back, std::path::PathBuf::from(original));
        assert!(is_file_uri(&uri));
        assert_eq!(uri_extension(&uri), Some("t"));
    }
}
