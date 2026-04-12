//! Tests for hover richness on package names — issue #4282 Win 1.
//!
//! Covers:
//! - `File::Path` in a method-call context (token fallback path) returns richer hover
//!   than the bare `**Perl**: \`File::Path\`` fallback.

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

/// Hovering on `File::Path` in a method-call context should return richer content
/// than the bare-token fallback `**Perl**: \`File::Path\``.
///
/// The hover must include the module name as a header AND a MetaCPAN link — not
/// just the raw token with no context.
#[test]
fn test_hover_package_name_file_path_richer_than_bare_token() -> TestResult {
    // File::Path is used in a method-call position — NOT inside a `use` statement,
    // so it goes through the token fallback path rather than UseModule.
    let doc = "File::Path->make_path('/tmp/test');\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///pkg_hover_4282.pl", doc)?;
    // Position 0 = 'F' of "File::Path"
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///pkg_hover_4282.pl"},
                "position": {"line": 0, "character": 0}
            }),
        )
        .unwrap_or(json!(null));
    let val = hover_value(&result).ok_or("Expected hover content for File::Path")?;
    assert!(
        !val.starts_with("**Perl**: `"),
        "hover for File::Path should not be bare token fallback, got: {val}"
    );
    assert!(val.contains("File::Path"), "hover should mention the package name, got: {val}");
    assert!(val.contains("metacpan.org"), "hover should include MetaCPAN link, got: {val}");
    Ok(())
}
