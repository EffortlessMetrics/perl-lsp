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
