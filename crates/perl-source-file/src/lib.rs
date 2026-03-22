//! Shared Perl source-file classification helpers.
//!
//! These helpers provide one canonical definition for what constitutes a Perl
//! source file across workspace discovery and runtime file operations.

use std::path::Path;

/// Canonical Perl source file extensions.
pub const PERL_SOURCE_EXTENSIONS: [&str; 5] = ["pl", "pm", "t", "psgi", "tt"];

/// Returns `true` if `extension` is a recognized Perl source extension.
///
/// Accepts values with or without a leading dot and matches
/// case-insensitively.
#[must_use]
pub fn is_perl_source_extension(extension: &str) -> bool {
    let ext = extension.strip_prefix('.').unwrap_or(extension);
    PERL_SOURCE_EXTENSIONS.iter().any(|candidate| candidate.eq_ignore_ascii_case(ext))
}

/// Returns `true` if `path` points to a recognized Perl source file.
#[must_use]
pub fn is_perl_source_path(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()).is_some_and(is_perl_source_extension)
}

/// Returns `true` if `uri` or path-like string points to a Perl source file.
///
/// Supports:
/// - Plain filesystem paths
/// - `file://` URIs
/// - Optional query/fragment suffixes
#[must_use]
pub fn is_perl_source_uri(uri: &str) -> bool {
    let without_fragment = uri.split('#').next().unwrap_or(uri);
    let without_query = without_fragment.split('?').next().unwrap_or(without_fragment);
    is_perl_source_path(Path::new(without_query))
}

#[cfg(test)]
mod tests {
    use super::{
        PERL_SOURCE_EXTENSIONS, is_perl_source_extension, is_perl_source_path, is_perl_source_uri,
    };
    use std::path::Path;

    #[test]
    fn exposes_expected_extension_set() {
        assert_eq!(PERL_SOURCE_EXTENSIONS, ["pl", "pm", "t", "psgi", "tt"]);
    }

    #[test]
    fn classifies_extensions_case_insensitively() {
        assert!(is_perl_source_extension("pl"));
        assert!(is_perl_source_extension(".pm"));
        assert!(is_perl_source_extension("T"));
        assert!(is_perl_source_extension("PsGi"));
        assert!(is_perl_source_extension("tt"));
        assert!(is_perl_source_extension("TT"));
        assert!(!is_perl_source_extension("txt"));
    }

    #[test]
    fn classifies_tt_template_files() {
        assert!(is_perl_source_path(Path::new("/templates/page.tt")));
        assert!(is_perl_source_uri("file:///templates/page.tt"));
        assert!(!is_perl_source_path(Path::new("/templates/page.html")));
    }

    #[test]
    fn classifies_filesystem_paths() {
        assert!(is_perl_source_path(Path::new("/workspace/script.pl")));
        assert!(is_perl_source_path(Path::new("/workspace/lib/Foo/Bar.PM")));
        assert!(is_perl_source_path(Path::new("/workspace/app.psgi")));
        assert!(!is_perl_source_path(Path::new("/workspace/README.md")));
        assert!(!is_perl_source_path(Path::new("/workspace/no_extension")));
    }

    #[test]
    fn classifies_uri_like_inputs() {
        assert!(is_perl_source_uri("file:///workspace/script.pl"));
        assert!(is_perl_source_uri("file:///workspace/lib/Foo/Bar.pm"));
        assert!(is_perl_source_uri("file:///workspace/app.psgi"));
        assert!(is_perl_source_uri("file:///workspace/app.psgi?version=1#section"));
        assert!(!is_perl_source_uri("file:///workspace/README.md"));
    }

    #[test]
    fn supports_windows_style_paths() {
        assert!(is_perl_source_uri(r"C:\workspace\script.pl"));
        assert!(is_perl_source_uri(r"file:///C:/workspace/lib/Foo.pm"));
        assert!(!is_perl_source_uri(r"C:\workspace\README.txt"));
    }
}
