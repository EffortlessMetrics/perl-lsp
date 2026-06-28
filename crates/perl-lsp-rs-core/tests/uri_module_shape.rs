//! Integration test: `perl-lsp-uri` public API reachable via `perl_lsp_rs_core::uri`.

use perl_lsp_rs_core::uri::*;

#[test]
fn uri_module_exposes_parse_uri_function() {
    // Verify that parse_uri() is accessible post-absorption
    let uri = parse_uri("file:///tmp/test.pl");
    assert_eq!(
        uri.as_str(),
        "file:///tmp/test.pl",
        "parse_uri should preserve valid URIs verbatim"
    );
}

/// Ported from EffortlessMetrics/perl-lsp#9903.
///
/// Unix bare file paths must be converted to `file://` URIs automatically.
/// Only runs on Unix because `Url::from_file_path` requires POSIX path semantics.
#[cfg(unix)]
#[test]
fn uri_module_parse_uri_accepts_unix_file_paths() -> Result<(), Box<dyn std::error::Error>> {
    let uri = parse_uri("/tmp/perl-lsp/lib/PlainPath.pm");
    assert_eq!(uri.as_str(), "file:///tmp/perl-lsp/lib/PlainPath.pm");
    Ok(())
}

/// Ported from EffortlessMetrics/perl-lsp#9903.
///
/// Windows bare file paths (drive-letter + backslash) must be normalised to
/// `file:///DRIVE:/...` URIs automatically — even on non-Windows hosts.
#[test]
fn uri_module_parse_uri_accepts_windows_file_paths() -> Result<(), Box<dyn std::error::Error>> {
    let uri = parse_uri(r"C:\Users\dev\lib\PlainPath.pm");
    assert_eq!(uri.as_str(), "file:///C:/Users/dev/lib/PlainPath.pm");
    Ok(())
}

/// Ported from EffortlessMetrics/perl-lsp#9903.
///
/// Windows bare file paths with forward-slash separators (e.g. `C:/foo/bar.pm`)
/// must also be accepted and normalised — some editors send forward-slash
/// Windows paths even on Windows hosts.
#[test]
fn uri_module_parse_uri_accepts_windows_forward_slash_paths()
-> Result<(), Box<dyn std::error::Error>> {
    let uri = parse_uri("C:/Users/dev/lib/PlainPath.pm");
    assert_eq!(uri.as_str(), "file:///C:/Users/dev/lib/PlainPath.pm");
    Ok(())
}

#[test]
fn uri_module_parse_uri_handles_windows_paths() {
    // Verify that parse_uri handles Windows paths correctly post-absorption
    let input = "file:///C:/Users/dev/test.pm";
    let uri = parse_uri(input);
    assert_eq!(uri.as_str(), input, "parse_uri should preserve Windows paths verbatim");
}

#[test]
fn uri_module_parse_uri_handles_invalid_input() {
    // Verify that parse_uri gracefully handles invalid input post-absorption
    let uri = parse_uri("not a uri");
    assert!(!uri.as_str().is_empty(), "parse_uri should never panic on invalid input");
    // Fallback must itself be a valid URI — round-trip proves that.
    assert!(
        uri.as_str().parse::<lsp_types::Uri>().is_ok(),
        "fallback URI must itself round-trip parse"
    );
}

#[test]
fn uri_module_parse_uri_round_trip_preserves_valid_uri() {
    // Verify that parse_uri -> as_str round-trip preserves input for valid URIs
    let input = "file:///home/user/lib/Module.pm";
    let uri = parse_uri(input);
    assert!(uri.as_str() == input, "parse_uri should preserve valid URIs on round-trip");
}

#[test]
fn uri_module_parse_uri_handles_percent_encoding() {
    // Verify that parse_uri preserves percent-encoded paths
    let input = "file:///path/to/my%20module/Foo.pm";
    let uri = parse_uri(input);
    assert!(uri.as_str() == input, "parse_uri should preserve percent-encoding");
}

#[test]
fn uri_module_parse_uri_handles_utf8_file_path() {
    let input = "file:///tmp/naïve/模块.pm";
    let uri = parse_uri(input);
    assert_eq!(uri.as_str(), "file:///tmp/na%C3%AFve/%E6%A8%A1%E5%9D%97.pm");
}

#[test]
fn uri_module_parse_uri_preserves_encoded_utf8_path() {
    let input = "file:///tmp/na%C3%AFve/%E6%A8%A1%E5%9D%97.pm";
    let uri = parse_uri(input);
    assert_eq!(uri.as_str(), input, "parse_uri should preserve valid UTF-8 percent encoding");
}

#[test]
fn uri_module_parse_uri_handles_utf8_bom_prefix() {
    // A UTF-8 BOM (U+FEFF, encoded as 0xEF 0xBB 0xBF) at the front of the input
    // is not a legal URI character. Even when encoded as %EF%BB%BF, the path
    // segment must remain intact rather than drop the bytes.
    let input = "file:///tmp/%EF%BB%BFmodule.pm";
    let uri = parse_uri(input);
    assert_eq!(
        uri.as_str(),
        input,
        "parse_uri should preserve percent-encoded BOM bytes within a path"
    );

    // A raw BOM prefix on the URI itself must not panic. The exact fallback
    // behaviour doesn't matter, but the result must be a valid URI.
    let raw_bom_input = "\u{feff}file:///tmp/module.pm";
    let uri = parse_uri(raw_bom_input);
    assert!(!uri.as_str().is_empty(), "parse_uri must tolerate a raw BOM prefix");
    assert!(
        uri.as_str().parse::<lsp_types::Uri>().is_ok(),
        "fallback URI must itself round-trip parse"
    );
}

#[test]
fn uri_module_parse_uri_tolerates_invalid_percent_escape() {
    // An invalid percent-escape sequence (e.g. `%ZZ`, or a lone `%` at EOL)
    // must not panic. The result must be a valid URI — either the input
    // re-parsed by `url::Url` or the canonical fallback.
    for bad in [
        "file:///tmp/%ZZ.pm",
        "file:///tmp/half%",
        "file:///tmp/%C3%28.pm", // 0xC3 0x28 is an invalid UTF-8 byte sequence.
    ] {
        let uri = parse_uri(bad);
        assert!(!uri.as_str().is_empty(), "parse_uri({bad}) should not produce empty URI");
        assert!(
            uri.as_str().parse::<lsp_types::Uri>().is_ok(),
            "parse_uri({bad}) output must itself be a valid URI"
        );
    }
}

#[test]
fn uri_module_parse_uri_handles_supplementary_plane_codepoints() {
    // Emoji sit in the supplementary plane (surrogate-pair territory for UTF-16)
    // and encode to 4-byte UTF-8 sequences. A naive byte-at-a-time decoder that
    // slices mid-codepoint would corrupt these.
    let input = "file:///tmp/emoji_\u{1F600}/mod.pm"; // U+1F600 grinning face
    let uri = parse_uri(input);
    assert_eq!(
        uri.as_str(),
        "file:///tmp/emoji_%F0%9F%98%80/mod.pm",
        "parse_uri must encode 4-byte UTF-8 codepoints intact"
    );
}

#[test]
fn uri_module_parse_uri_ignores_ascii_whitespace_around_uri() {
    let uri = parse_uri("  file:///tmp/trimmed.pm\n");
    assert_eq!(
        uri.as_str(),
        "file:///tmp/trimmed.pm",
        "parse_uri should trim surrounding ASCII whitespace before parsing"
    );
}

#[test]
fn uri_module_parse_uri_ignores_leading_bom_before_whitespace() {
    let uri = parse_uri("\u{feff}  file:///tmp/trimmed.pm\n");
    assert_eq!(
        uri.as_str(),
        "file:///tmp/trimmed.pm",
        "parse_uri should strip a leading BOM before trimming whitespace"
    );
}
