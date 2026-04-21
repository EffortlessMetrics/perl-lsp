//! Integration test: `perl-lsp-uri` public API reachable via `perl_lsp_rs_core::uri`.

use perl_lsp_rs_core::uri::*;

#[test]
fn uri_module_exposes_parse_uri_function() {
    // Verify that parse_uri() is accessible post-absorption
    let uri = parse_uri("file:///tmp/test.pl");
    assert!(uri.as_str().contains("test.pl"), "parse_uri should preserve valid URIs");
}

#[test]
fn uri_module_parse_uri_handles_windows_paths() {
    // Verify that parse_uri handles Windows paths correctly post-absorption
    let uri = parse_uri("file:///C:/Users/dev/test.pm");
    assert!(uri.as_str().contains("test.pm"), "parse_uri should handle Windows paths");
}

#[test]
fn uri_module_parse_uri_handles_invalid_input() {
    // Verify that parse_uri gracefully handles invalid input post-absorption
    let uri = parse_uri("not a uri");
    assert!(!uri.as_str().is_empty(), "parse_uri should never panic on invalid input");
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
