//! Tests for regex hover explanation feature (issue #2048)
//!
//! When the user hovers over a Perl regex literal, the hover response should
//! include a human-readable breakdown of the regex metacharacters.

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn hover_value(result: &serde_json::Value) -> Option<String> {
    result
        .get("contents")
        .and_then(|c| c.get("value"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Hover on a simple regex returns a human-readable explanation.
///
/// The regex `/\d+/` should explain `\d` and `+`.
#[test]
fn test_hover_regex_simple_digit_plus() -> TestResult {
    // Regex starts at character 10 on line 0: `if ($x =~ /\d+/) {`
    let doc = "if ($x =~ /\\d+/) {\n}\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///regex_simple.pl", doc)?;

    // Position 11 is inside the `/\d+/` literal (on the `\d`)
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///regex_simple.pl"},
                "position": {"line": 0, "character": 11}
            }),
        )
        .unwrap_or(json!(null));

    let val = hover_value(&result).ok_or("Expected hover contents for regex")?;
    assert!(
        val.contains("digit") || val.contains("Digit"),
        "Hover should explain \\d as digit-related; got: {val}"
    );
    Ok(())
}

/// Hover on a regex with anchors and capture groups returns explanation.
#[test]
fn test_hover_regex_fat_arrow_pattern() -> TestResult {
    let doc = "my $re = /^(\\w+)\\s*=>\\s*(.*)$/;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///regex_fat_arrow.pl", doc)?;

    // Position 10 is inside the regex literal (on `^`)
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///regex_fat_arrow.pl"},
                "position": {"line": 0, "character": 10}
            }),
        )
        .unwrap_or(json!(null));

    let val = hover_value(&result).ok_or("Expected hover contents for regex")?;
    assert!(
        val.contains("Regex") || val.contains("regex") || val.contains("pattern"),
        "Hover should describe the regex pattern; got: {val}"
    );
    assert!(
        val.contains("Start") || val.contains("start") || val.contains("anchor"),
        "Hover should explain ^ anchor; got: {val}"
    );
    Ok(())
}

/// Hover on a regex explains capture groups.
#[test]
fn test_hover_regex_capture_group_explanation() -> TestResult {
    let doc = "my @m = ($str =~ /(\\w+)/);\\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///regex_capture.pl", doc)?;

    // Position 18 is inside the regex `/(\\w+)/` (on the opening paren)
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///regex_capture.pl"},
                "position": {"line": 0, "character": 18}
            }),
        )
        .unwrap_or(json!(null));

    let val = hover_value(&result).ok_or("Expected hover contents for regex")?;
    assert!(
        val.contains("Capture") || val.contains("capture") || val.contains("group"),
        "Hover should mention capture group; got: {val}"
    );
    Ok(())
}

/// Hover on a substitution regex (s///) also explains the pattern.
#[test]
fn test_hover_substitution_regex_explanation() -> TestResult {
    let doc = "$str =~ s/\\s+/ /g;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///regex_subst.pl", doc)?;

    // Position 11 is inside `s/\s+/ /g` (on `s` in `\s`)
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///regex_subst.pl"},
                "position": {"line": 0, "character": 11}
            }),
        )
        .unwrap_or(json!(null));

    let val = hover_value(&result).ok_or("Expected hover contents for substitution regex")?;
    assert!(
        val.contains("whitespace") || val.contains("Whitespace") || val.contains("\\s"),
        "Hover should explain \\s as whitespace; got: {val}"
    );
    Ok(())
}

/// Hover content for a regex is formatted as markdown.
#[test]
fn test_hover_regex_returns_markdown_kind() -> TestResult {
    let doc = "if ($x =~ /\\d+/) {\n}\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///regex_markdown.pl", doc)?;

    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///regex_markdown.pl"},
                "position": {"line": 0, "character": 11}
            }),
        )
        .unwrap_or(json!(null));

    if !result.is_null() {
        let kind = result.get("contents").and_then(|c| c.get("kind")).and_then(|k| k.as_str());
        assert_eq!(kind, Some("markdown"), "Hover kind should be 'markdown'; got: {kind:?}");
    }
    Ok(())
}
