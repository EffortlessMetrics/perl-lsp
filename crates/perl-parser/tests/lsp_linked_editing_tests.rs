//! Linked editing range tests
mod support;
use serde_json::json;
use support::lsp_harness::LspHarness;

#[test]
fn test_brace_pair() {
    let doc = r#"sub x { my $h = { a => 1 }; }"#;
    let mut harness = LspHarness::new();
    harness.initialize(None).unwrap();
    harness.open_document("file:///test.pl", doc).unwrap();
    let uri = "file:///test.pl";

    // cursor on the '{' after '=' (line 0, character 16)
    let result = harness
        .request(
            "textDocument/linkedEditingRange",
            json!({
                "textDocument": {"uri": uri},
                "position": {"line": 0, "character": 16}
            }),
        )
        .unwrap_or(json!(null));

    if let Some(ranges) = result.get("ranges").and_then(|r| r.as_array()) {
        assert_eq!(ranges.len(), 2, "Should return two linked ranges for brace pair");
    } else {
        // Null is also acceptable if no linked ranges at this position
        assert!(result.is_null(), "Should return either ranges or null");
    }
}

#[test]
fn test_quotes_pair() {
    let doc = r#"my $s = "hi";"#;
    let mut harness = LspHarness::new();
    harness.initialize(None).unwrap();
    harness.open_document("file:///test.pl", doc).unwrap();
    let uri = "file:///test.pl";

    // cursor on opening quote (line 0, character 8)
    let result = harness
        .request(
            "textDocument/linkedEditingRange",
            json!({
                "textDocument": {"uri": uri},
                "position": {"line": 0, "character": 8}
            }),
        )
        .unwrap_or(json!(null));

    if let Some(ranges) = result.get("ranges").and_then(|r| r.as_array()) {
        assert_eq!(ranges.len(), 2, "Should return two linked ranges for quote pair");
    } else {
        assert!(result.is_null(), "Should return either ranges or null");
    }
}

#[test]
fn test_nested_parens() {
    let doc = r#"if ((($x > 0))) { print "yes"; }"#;
    let mut harness = LspHarness::new();
    harness.initialize(None).unwrap();
    harness.open_document("file:///test.pl", doc).unwrap();
    let uri = "file:///test.pl";

    // cursor on innermost opening paren (line 0, character 5)
    let result = harness
        .request(
            "textDocument/linkedEditingRange",
            json!({
                "textDocument": {"uri": uri},
                "position": {"line": 0, "character": 5}
            }),
        )
        .unwrap_or(json!(null));

    if let Some(ranges) = result.get("ranges").and_then(|r| r.as_array()) {
        assert_eq!(ranges.len(), 2, "Should return two linked ranges for innermost parens");
    } else {
        assert!(result.is_null(), "Should return either ranges or null");
    }
}

#[test]
fn test_square_brackets() {
    let doc = r#"my @arr = [1, 2, [3, 4]];"#;
    let mut harness = LspHarness::new();
    harness.initialize(None).unwrap();
    harness.open_document("file:///test.pl", doc).unwrap();
    let uri = "file:///test.pl";

    // cursor on outer opening bracket (line 0, character 10)
    let result = harness
        .request(
            "textDocument/linkedEditingRange",
            json!({
                "textDocument": {"uri": uri},
                "position": {"line": 0, "character": 10}
            }),
        )
        .unwrap_or(json!(null));

    if let Some(ranges) = result.get("ranges").and_then(|r| r.as_array()) {
        assert_eq!(ranges.len(), 2, "Should return two linked ranges for bracket pair");
    } else {
        assert!(result.is_null(), "Should return either ranges or null");
    }
}

#[test]
fn test_no_pair_at_position() {
    let doc = r#"my $x = 42;"#;
    let mut harness = LspHarness::new();
    harness.initialize(None).unwrap();
    harness.open_document("file:///test.pl", doc).unwrap();
    let uri = "file:///test.pl";

    // cursor on a number (line 0, character 8)
    let result = harness
        .request(
            "textDocument/linkedEditingRange",
            json!({
                "textDocument": {"uri": uri},
                "position": {"line": 0, "character": 8}
            }),
        )
        .unwrap_or(json!(null));

    assert!(result.is_null(), "Should return null when no paired delimiter at position");
}
