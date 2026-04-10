//! Behavior-driven tests for the `perl-uri` crate.
//!
//! Each scenario follows a Given/When/Then structure so the intended
//! user-visible behavior is easy to read and maintain.

#[cfg(not(target_arch = "wasm32"))]
mod bdd {
    use perl_uri::{
        fs_path_to_uri, is_file_uri, is_special_scheme, normalize_uri, uri_extension,
        uri_to_fs_path,
    };
    use std::path::Path;

    fn given_file_uri_with_spaces() -> &'static str {
        "file:///tmp/path%20with%20spaces/script.pl"
    }

    fn given_non_file_uri() -> &'static str {
        "https://example.com/script.pl"
    }

    #[test]
    fn scenario_convert_file_uri_to_filesystem_path() -> Result<(), String> {
        // Given a file:// URI with percent-encoded spaces
        let uri = given_file_uri_with_spaces();

        // When the URI is converted to a filesystem path
        let path = uri_to_fs_path(uri).ok_or("expected file URI to convert to path")?;

        // Then spaces are decoded and the script filename is preserved
        let text = path.to_string_lossy();
        if !text.contains("path with spaces") {
            return Err(format!("expected decoded spaces, got: {text}"));
        }
        if !text.ends_with("script.pl") {
            return Err(format!("expected script.pl suffix, got: {text}"));
        }

        Ok(())
    }

    #[test]
    fn scenario_reject_non_file_uri_for_path_conversion() {
        // Given a non-file URI
        let uri = given_non_file_uri();

        // When conversion is requested
        let path = uri_to_fs_path(uri);

        // Then conversion is rejected
        assert!(path.is_none());
    }

    #[test]
    fn scenario_convert_relative_path_to_absolute_file_uri() -> Result<(), String> {
        // Given a relative filesystem path
        let relative = Path::new("relative/path/module.pm");

        // When the path is converted to a URI
        let uri = fs_path_to_uri(relative)?;

        // Then an absolute file:// URI is returned with the extension preserved
        if !uri.starts_with("file:///") {
            return Err(format!("expected absolute file URI, got: {uri}"));
        }
        if uri_extension(&uri) != Some("pm") {
            return Err(format!("expected pm extension in URI, got: {uri}"));
        }

        Ok(())
    }

    #[test]
    fn scenario_normalize_absolute_path_input() -> Result<(), String> {
        // Given an absolute filesystem path
        let original = std::env::temp_dir().join("bdd-normalize.pl");

        // When normalized through the URI normalizer
        let normalized = normalize_uri(&original.to_string_lossy());

        // Then the result is the same as direct path-to-URI conversion
        let expected = fs_path_to_uri(&original)?;
        if normalized != expected {
            return Err(format!("expected {expected}, got {normalized}"));
        }

        Ok(())
    }

    #[test]
    fn scenario_roundtrip_path_uri_path() -> Result<(), String> {
        // Given an absolute path with unicode and spaces
        let original = Path::new("/tmp/bdd 日本語/round trip.t");

        // When the path is converted to a URI and then back to a path
        let uri = fs_path_to_uri(original)?;
        let roundtrip = uri_to_fs_path(&uri).ok_or("expected URI to roundtrip to path")?;

        // Then the resulting path matches the original input
        if roundtrip != original {
            return Err(format!(
                "roundtrip mismatch: {} vs {}",
                roundtrip.display(),
                original.display()
            ));
        }

        Ok(())
    }

    #[test]
    fn scenario_distinguish_file_and_special_schemes() {
        // Given one file URI and one editor-specific URI
        let file_uri = "file:///tmp/lib/Module.pm";
        let special_uri = "untitled:Untitled-1";

        // When each URI is classified
        let file_is_file = is_file_uri(file_uri);
        let file_is_special = is_special_scheme(file_uri);
        let special_is_file = is_file_uri(special_uri);
        let special_is_special = is_special_scheme(special_uri);

        // Then only the file URI is recognized as file:// and only the
        // editor URI is recognized as a special scheme
        assert!(file_is_file);
        assert!(!file_is_special);
        assert!(!special_is_file);
        assert!(special_is_special);
    }
}
