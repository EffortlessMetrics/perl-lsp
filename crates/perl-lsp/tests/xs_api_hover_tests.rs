//! Tests for XS / Perl C API hover support.

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

#[test]
fn hover_shows_xs_api_docs_in_xs_sources() -> TestResult {
    let doc = concat!(
        "#include \"EXTERN.h\"\n",
        "#include \"perl.h\"\n",
        "#include \"XSUB.h\"\n",
        "MODULE = My::Module PACKAGE = My::Module\n",
        "PPCODE:\n",
        "    newSVpv\n"
    );
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///example.xs", doc)?;

    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///example.xs"},
                "position": {"line": 5, "character": 7}
            }),
        )
        .unwrap_or(json!(null));

    let val = hover_value(&result).ok_or("Expected hover content for XS API symbol")?;
    assert!(
        val.contains("XS / Perl C API"),
        "hover should identify the XS API docs, got: {val}"
    );
    assert!(
        val.contains("newSVpv(pv, len)"),
        "hover should include the XS API signature, got: {val}"
    );
    Ok(())
}

#[test]
fn completion_offers_xs_api_docs_in_astless_xs_sources() -> TestResult {
    let doc = concat!(
        "#include \"EXTERN.h\"\n",
        "#include \"perl.h\"\n",
        "#include \"XSUB.h\"\n",
        "MODULE = My::Module PACKAGE = My::Module\n",
        "PPCODE:\n",
        "    newS\n"
    );
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///example.xs", doc)?;

    let result = harness
        .request(
            "textDocument/completion",
            json!({
                "textDocument": {"uri": "file:///example.xs"},
                "position": {"line": 5, "character": 8}
            }),
        )
        .unwrap_or(json!(null));

    let labels: Vec<&str> = result
        .get("items")
        .and_then(|items| items.as_array())
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("label").and_then(|label| label.as_str()))
        .collect();

    assert!(
        labels.contains(&"newSVpv"),
        "AST-less XS completion should offer newSVpv, got: {labels:?}"
    );
    assert!(
        labels.contains(&"newSViv"),
        "AST-less XS completion should offer newSViv, got: {labels:?}"
    );
    Ok(())
}

#[test]
fn hover_does_not_show_xs_docs_in_normal_perl() -> TestResult {
    let doc = "package My::Module;\nnewSVpv\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///example.pl", doc)?;

    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///example.pl"},
                "position": {"line": 1, "character": 3}
            }),
        )
        .unwrap_or(json!(null));

    let val = hover_value(&result).ok_or("Expected hover content for fallback token")?;
    assert!(
        !val.contains("XS / Perl C API"),
        "normal Perl hover should not surface XS docs, got: {val}"
    );
    Ok(())
}
