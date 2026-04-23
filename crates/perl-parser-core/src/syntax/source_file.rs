//! Shared Perl source-file classification helpers.
//!
//! These helpers provide one canonical definition for what constitutes a Perl
//! source file across workspace discovery and runtime file operations.

use std::path::Path;

/// Number of bytes to inspect for binary content detection.
///
/// 4 KB is enough to catch all common binary formats (ELF, PE, ZIP, PNG, …)
/// while being cheap to scan.
const BINARY_PROBE_BYTES: usize = 4096;

/// Returns `true` if `text` appears to contain binary (non-text) content.
///
/// The heuristic checks the first [`BINARY_PROBE_BYTES`] bytes for null bytes
/// (`\0`).  A single null byte is sufficient to classify the content as
/// binary: valid Perl (or any UTF-8 text) never contains null bytes outside of
/// raw string literals, and real-world binary formats (ELF, PE/COFF, ZIP,
/// PNG, …) all begin with or contain null bytes in their headers.
///
/// # Why null bytes?
///
/// - Fast: a single `memchr`-style scan of at most 4 KB.
/// - Low false-positive rate: Perl source virtually never contains `\0`.
/// - High true-positive rate: every common compiled binary contains `\0`.
#[must_use]
pub fn is_binary_content(text: &str) -> bool {
    text.bytes().take(BINARY_PROBE_BYTES).any(|b| b == 0)
}

/// Canonical Perl source file extensions.
///
/// Includes core Perl script and module extensions as well as common embedded
/// Perl template formats: `.ep` (Mojolicious), `.tt`/`.tt2` (Template Toolkit),
/// and `.mason` (Mason/HTML::Mason), plus `.claw` (OpenClaw templates).
pub const PERL_SOURCE_EXTENSIONS: [&str; 10] =
    ["pl", "pm", "t", "psgi", "cgi", "ep", "tt", "tt2", "mason", "claw"];

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
        BINARY_PROBE_BYTES, PERL_SOURCE_EXTENSIONS, is_binary_content, is_perl_source_extension,
        is_perl_source_path, is_perl_source_uri,
    };
    use std::path::Path;

    #[test]
    fn exposes_expected_extension_set() {
        assert_eq!(
            PERL_SOURCE_EXTENSIONS,
            ["pl", "pm", "t", "psgi", "cgi", "ep", "tt", "tt2", "mason", "claw"]
        );
    }

    #[test]
    fn classifies_extensions_case_insensitively() {
        assert!(is_perl_source_extension("pl"));
        assert!(is_perl_source_extension(".pm"));
        assert!(is_perl_source_extension("T"));
        assert!(is_perl_source_extension("PsGi"));
        assert!(is_perl_source_extension("cgi"));
        assert!(is_perl_source_extension(".CGI"));
        assert!(!is_perl_source_extension("txt"));
    }

    #[test]
    fn classifies_filesystem_paths() {
        assert!(is_perl_source_path(Path::new("/workspace/script.pl")));
        assert!(is_perl_source_path(Path::new("/workspace/lib/Foo/Bar.PM")));
        assert!(is_perl_source_path(Path::new("/workspace/app.psgi")));
        assert!(is_perl_source_path(Path::new("/var/www/cgi-bin/form.cgi")));
        assert!(is_perl_source_path(Path::new("/var/www/cgi-bin/upload.CGI")));
        assert!(is_perl_source_path(Path::new("/workspace/views/layout.claw")));
        assert!(!is_perl_source_path(Path::new("/workspace/README.md")));
        assert!(!is_perl_source_path(Path::new("/workspace/no_extension")));
    }

    #[test]
    fn classifies_uri_like_inputs() {
        assert!(is_perl_source_uri("file:///workspace/script.pl"));
        assert!(is_perl_source_uri("file:///workspace/lib/Foo/Bar.pm"));
        assert!(is_perl_source_uri("file:///workspace/app.psgi"));
        assert!(is_perl_source_uri("file:///workspace/app.psgi?version=1#section"));
        assert!(is_perl_source_uri("file:///var/www/cgi-bin/form.cgi"));
        assert!(is_perl_source_uri("file:///var/www/cgi-bin/search.cgi?q=perl#results"));
        assert!(is_perl_source_uri("file:///workspace/views/layout.claw"));
        assert!(!is_perl_source_uri("file:///workspace/README.md"));
    }

    #[test]
    fn cgi_and_psgi_are_recognized() {
        // CGI scripts (.cgi) — web projects, Apache/Nginx CGI handlers
        assert!(is_perl_source_extension("cgi"));
        assert!(is_perl_source_extension("CGI"));
        assert!(is_perl_source_path(Path::new("/var/www/cgi-bin/form.cgi")));
        assert!(is_perl_source_uri("file:///var/www/cgi-bin/form.cgi"));

        // PSGI apps (.psgi) — Plack/PSGI applications
        assert!(is_perl_source_extension("psgi"));
        assert!(is_perl_source_extension("PSGI"));
        assert!(is_perl_source_path(Path::new("/workspace/app.psgi")));
        assert!(is_perl_source_uri("file:///workspace/app.psgi"));

        // Non-Perl extensions remain unrecognized
        assert!(!is_perl_source_extension("sh"));
        assert!(!is_perl_source_extension("py"));
    }

    #[test]
    fn template_extensions_are_recognized() {
        // .ep — Mojolicious embedded Perl templates
        assert!(is_perl_source_extension("ep"));
        assert!(is_perl_source_extension("EP"));
        assert!(is_perl_source_path(Path::new("/app/templates/index.html.ep")));
        assert!(is_perl_source_uri("file:///app/templates/index.html.ep"));

        // .tt — Template Toolkit templates (version 2 default)
        assert!(is_perl_source_extension("tt"));
        assert!(is_perl_source_extension("TT"));
        assert!(is_perl_source_path(Path::new("/app/templates/page.tt")));
        assert!(is_perl_source_uri("file:///app/templates/page.tt"));

        // .tt2 — Template Toolkit 2 explicit extension
        assert!(is_perl_source_extension("tt2"));
        assert!(is_perl_source_extension("TT2"));
        assert!(is_perl_source_path(Path::new("/app/templates/layout.tt2")));
        assert!(is_perl_source_uri("file:///app/templates/layout.tt2"));

        // .mason — HTML::Mason / Mason2 templates
        assert!(is_perl_source_extension("mason"));
        assert!(is_perl_source_extension("MASON"));
        assert!(is_perl_source_path(Path::new("/app/comp/header.mason")));
        assert!(is_perl_source_uri("file:///app/comp/header.mason"));

        // .claw — OpenClaw embedded templates
        assert!(is_perl_source_extension("claw"));
        assert!(is_perl_source_extension("CLAW"));
        assert!(is_perl_source_path(Path::new("/app/views/home.claw")));
        assert!(is_perl_source_uri("file:///app/views/home.claw"));

        // Non-template extensions remain unrecognized
        assert!(!is_perl_source_extension("html"));
        assert!(!is_perl_source_extension("tmpl"));
    }

    #[test]
    fn supports_windows_style_paths() {
        assert!(is_perl_source_uri(r"C:\workspace\script.pl"));
        assert!(is_perl_source_uri(r"file:///C:/workspace/lib/Foo.pm"));
        assert!(!is_perl_source_uri(r"C:\workspace\README.txt"));
    }

    // ── is_binary_content ─────────────────────────────────────────────────

    #[test]
    fn binary_content_null_byte_is_detected() {
        // Simulate a binary file arriving as a string with embedded null bytes
        let binary = "PK\x00\x03some binary content\x00\x00\x00";
        assert!(is_binary_content(binary), "null bytes must trigger binary guard");
    }

    #[test]
    fn binary_content_single_null_byte_triggers_guard() {
        let text = "use strict;\x00\nuse warnings;\n";
        assert!(is_binary_content(text), "single null byte must trigger binary guard");
    }

    #[test]
    fn binary_content_clean_perl_is_not_binary() {
        let perl = "#!/usr/bin/perl\nuse strict;\nuse warnings;\n\nprint \"Hello, World!\\n\";\n";
        assert!(!is_binary_content(perl), "clean Perl source must not be classified as binary");
    }

    #[test]
    fn binary_content_empty_string_is_not_binary() {
        assert!(!is_binary_content(""), "empty string must not be classified as binary");
    }

    #[test]
    fn binary_content_unicode_text_is_not_binary() {
        // High-byte UTF-8 sequences must not trigger the guard
        let utf8 = "# Perl with Unicode: \u{00e9}t\u{00e9}\nprint \"caf\u{00e9}\\n\";\n";
        assert!(!is_binary_content(utf8), "UTF-8 text without null bytes must not be binary");
    }

    #[test]
    fn binary_content_only_scans_first_probe_window() {
        // A null byte beyond the probe window must NOT trigger the guard —
        // we only scan the first BINARY_PROBE_BYTES bytes.
        let safe_prefix = "a".repeat(BINARY_PROBE_BYTES);
        let text_with_late_null = format!("{safe_prefix}\x00trailing");
        assert!(
            !is_binary_content(&text_with_late_null),
            "null byte beyond probe window must not trigger the guard"
        );
    }

    #[test]
    fn binary_content_null_byte_at_probe_boundary() {
        // A null byte exactly at the last probe byte must still be detected
        let prefix = "a".repeat(BINARY_PROBE_BYTES - 1);
        let text = format!("{prefix}\x00rest");
        assert!(is_binary_content(&text), "null byte at probe boundary must trigger binary guard");
    }

    #[test]
    fn binary_content_elf_header_is_detected() {
        // ELF magic: \x7fELF followed by binary data
        let elf_like = "\x7fELF\x02\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        assert!(is_binary_content(elf_like), "ELF-like header with null bytes must be binary");
    }

    #[test]
    fn binary_content_zip_pk_header_is_detected() {
        // ZIP files start with PK\x03\x04
        let zip_like = "PK\x03\x04\x14\x00\x00\x00\x08\x00";
        assert!(is_binary_content(zip_like), "ZIP-like header with null bytes must be binary");
    }
}
