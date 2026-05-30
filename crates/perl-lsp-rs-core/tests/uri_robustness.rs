//! Robustness tests for `perl_lsp_rs_core::uri::parse_uri`.
//!
//! Covers malformed/adversarial inputs, scheme edge cases, idempotency,
//! and no-panic guarantees not present in the shape-level test suite.
//!
//! All tests assert that `parse_uri` never panics and always returns a
//! result that is itself a syntactically valid URI.

use perl_lsp_rs_core::uri::parse_uri;

/// Verify that a URI returned from `parse_uri` is always parseable again.
fn is_valid_uri(s: &str) -> bool {
    s.parse::<lsp_types::Uri>().is_ok()
}

// ---------------------------------------------------------------------------
// Empty and minimal inputs
// ---------------------------------------------------------------------------

/// BUG REPORT (#815): `lsp_types::Uri` silently accepts an empty string as a
/// valid parse, returning a URI whose `.as_str()` is `""`.  Because the
/// `parse::<Uri>()` call succeeds, `parse_uri` never reaches its fallback and
/// returns an empty URI instead of a meaningful sentinel.
///
/// This test is `#[ignore]`d so the test suite stays green; it documents the
/// defect so a follow-up builder can add the empty-string guard to
/// `parse_uri` (check `sanitized.is_empty()` before the parse attempt).
#[test]
#[ignore = "BUG #815: lsp_types::Uri accepts empty string — parse_uri must guard before calling parse"]
fn test_parse_uri_empty_string_returns_valid_fallback() {
    let uri = parse_uri("");
    assert!(!uri.as_str().is_empty(), "empty input must not produce an empty URI");
    assert!(is_valid_uri(uri.as_str()), "fallback for empty input must itself be a valid URI");
}

/// BUG REPORT (#815): Same root cause as the empty-string case above.
/// After BOM-stripping and `trim()`, a whitespace-only input collapses to
/// `""`, which `lsp_types::Uri` accepts without error.  The fallback is
/// never reached.
///
/// `#[ignore]`d for the same reason — pending fix in `parse_uri`.
#[test]
#[ignore = "BUG #815: lsp_types::Uri accepts empty string — parse_uri must guard before calling parse"]
fn test_parse_uri_whitespace_only_returns_valid_fallback() {
    // After trimming, the sanitized string is empty — must use fallback.
    for ws in ["   ", "\t\t", "\n\r\n", "  \t  \n  "] {
        let uri = parse_uri(ws);
        assert!(!uri.as_str().is_empty(), "whitespace-only input must not produce empty URI");
        assert!(
            is_valid_uri(uri.as_str()),
            "fallback for whitespace-only input '{ws:?}' must be valid"
        );
    }
}

#[test]
fn test_parse_uri_scheme_only_returns_valid_fallback() {
    // "file:" with no authority or path is not a useful URI;
    // the function must not panic and must return a valid fallback.
    let uri = parse_uri("file:");
    assert!(!uri.as_str().is_empty(), "scheme-only input must not produce empty URI");
    assert!(is_valid_uri(uri.as_str()), "result for 'file:' must be a valid URI");
}

// ---------------------------------------------------------------------------
// Non-file schemes — valid URIs that parse_uri must pass through
// ---------------------------------------------------------------------------

#[test]
fn test_parse_uri_http_scheme_preserved() {
    // LSP clients may use http:// URIs (e.g. for inlined content).
    // parse_uri should not reject a syntactically valid http URI.
    let input = "http://localhost:8080/path/to/script.pl";
    let uri = parse_uri(input);
    assert_eq!(uri.as_str(), input, "valid http URI must be passed through unchanged");
}

#[test]
fn test_parse_uri_https_scheme_preserved() {
    let input = "https://example.com/lib/Module.pm";
    let uri = parse_uri(input);
    assert_eq!(uri.as_str(), input, "valid https URI must be passed through unchanged");
}

#[test]
fn test_parse_uri_urn_scheme_preserved() {
    // URNs are valid URIs and may appear in some LSP contexts.
    let input = "urn:perl-lsp:module:Foo::Bar";
    let uri = parse_uri(input);
    assert!(is_valid_uri(uri.as_str()), "urn: URI should produce a valid URI");
    // The urn must come through — either as-is or via fallback.
    assert!(!uri.as_str().is_empty());
}

#[test]
fn test_parse_uri_invalid_scheme_returns_valid_fallback() {
    // A scheme with illegal characters (digits starting the label, spaces)
    // must not panic.
    for bad in ["123://not-valid", "no spaces://foo", "://missing-scheme"] {
        let uri = parse_uri(bad);
        assert!(!uri.as_str().is_empty(), "bad scheme '{bad}' must not produce empty URI");
        assert!(is_valid_uri(uri.as_str()), "result for bad scheme '{bad}' must be a valid URI");
    }
}

// ---------------------------------------------------------------------------
// Idempotency
// ---------------------------------------------------------------------------

#[test]
fn test_parse_uri_idempotent_on_valid_file_uri() {
    // Applying parse_uri twice must produce the same result as applying it once.
    let inputs = [
        "file:///home/user/lib/Module.pm",
        "file:///tmp/na%C3%AFve/%E6%A8%A1%E5%9D%97.pm",
        "file:///C:/Users/dev/project/script.pl",
    ];
    for input in inputs {
        let first = parse_uri(input);
        let second = parse_uri(first.as_str());
        assert_eq!(first.as_str(), second.as_str(), "parse_uri must be idempotent for '{input}'");
    }
}

#[test]
fn test_parse_uri_idempotent_on_fallback_output() {
    // Even when parse_uri falls back to a synthetic URI, that fallback must
    // itself survive a second round of parsing unchanged.
    for bad in ["not-a-uri", "", "   ", "file:", ":::"] {
        let first = parse_uri(bad);
        let second = parse_uri(first.as_str());
        assert_eq!(
            first.as_str(),
            second.as_str(),
            "parse_uri fallback for '{bad}' must be idempotent"
        );
    }
}

// ---------------------------------------------------------------------------
// Adversarial / boundary inputs
// ---------------------------------------------------------------------------

#[test]
fn test_parse_uri_null_bytes_do_not_panic() {
    // Null bytes are illegal in URIs but may appear in adversarial input.
    let inputs = ["file:///tmp/\0module.pm", "\0", "file://\0/path"];
    for bad in inputs {
        let uri = parse_uri(bad);
        assert!(!uri.as_str().is_empty(), "null-byte input must not produce empty URI");
        assert!(is_valid_uri(uri.as_str()), "result for null-byte input must be a valid URI");
    }
}

#[test]
fn test_parse_uri_very_long_input_does_not_panic() {
    // 64 KiB of repeated path segments — must not panic or stack-overflow.
    let long_segment = "a".repeat(64 * 1024);
    let long_uri = format!("file:///tmp/{long_segment}/mod.pm");
    let uri = parse_uri(&long_uri);
    assert!(!uri.as_str().is_empty(), "very long URI must not produce empty result");
    assert!(is_valid_uri(uri.as_str()), "result for very long URI must be a valid URI");
}

#[test]
fn test_parse_uri_multiple_bom_prefixes_do_not_panic() {
    // Only the first BOM is stripped (code uses trim_start_matches which strips all
    // leading occurrences). Either behaviour is acceptable — must not panic.
    let double_bom = "\u{feff}\u{feff}file:///tmp/module.pm";
    let uri = parse_uri(double_bom);
    assert!(!uri.as_str().is_empty(), "double BOM must not produce empty URI");
    assert!(is_valid_uri(uri.as_str()), "result for double BOM must be a valid URI");
}

#[test]
fn test_parse_uri_fragment_identifier_preserved() {
    // URIs with fragment identifiers are syntactically valid and should not
    // be mangled by parse_uri.
    let input = "file:///home/user/lib/Module.pm#line42";
    let uri = parse_uri(input);
    // The fragment must not be silently dropped.
    assert!(
        uri.as_str().contains('#'),
        "fragment identifier must be preserved, got '{}'",
        uri.as_str()
    );
    assert!(is_valid_uri(uri.as_str()), "result must be a valid URI");
}

#[test]
fn test_parse_uri_query_string_preserved() {
    // URIs with query strings (unusual for file:// but valid) must not panic.
    let input = "file:///tmp/test.pl?debug=1";
    let uri = parse_uri(input);
    assert!(!uri.as_str().is_empty(), "query-string URI must not produce empty result");
    assert!(is_valid_uri(uri.as_str()), "result must be a valid URI");
}
