//! Tests for textDocument/selectionRange LSP feature
//!
//! Validates the selection range provider functionality including:
//! - Selection expansion on a variable
//! - Selection expansion on a subroutine body
//! - Nested parent chain (expanding outward)
//! - Empty file handling
//! - Multiple positions in a single request

mod support;
use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Test selection range expansion on a variable reference
#[test]
fn test_selection_range_on_variable() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_sel_var.pl";
    harness.open(
        doc_uri,
        r#"sub process {
    my $data = "hello";
    print $data;
}
"#,
    )?;

    // Request selection range at the $data variable on line 1
    let response = harness.request(
        "textDocument/selectionRange",
        json!({
            "textDocument": { "uri": doc_uri },
            "positions": [
                { "line": 1, "character": 8 }
            ]
        }),
    )?;

    assert!(response.is_array(), "selectionRange should return an array, got: {:?}", response);

    let ranges = response.as_array().ok_or("response is not an array")?;
    assert_eq!(ranges.len(), 1, "Should return one SelectionRange for one position");

    let sel = &ranges[0];
    // The innermost range should cover the variable or its immediate context
    assert!(sel["range"].is_object(), "SelectionRange should have a 'range' field");
    assert!(sel["range"]["start"]["line"].is_number(), "Range start line should be a number");
    assert!(sel["range"]["end"]["line"].is_number(), "Range end line should be a number");

    Ok(())
}

/// Test selection range expansion on a subroutine body
#[test]
fn test_selection_range_on_sub_body() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_sel_sub.pl";
    harness.open(
        doc_uri,
        r#"sub outer {
    my $x = 1;
    my $y = 2;
    return $x + $y;
}
"#,
    )?;

    // Request selection range inside the sub body (on the return statement)
    let response = harness.request(
        "textDocument/selectionRange",
        json!({
            "textDocument": { "uri": doc_uri },
            "positions": [
                { "line": 3, "character": 4 }
            ]
        }),
    )?;

    let ranges = response.as_array().ok_or("response is not an array")?;
    assert_eq!(ranges.len(), 1, "Should return one SelectionRange");

    let sel = &ranges[0];
    assert!(sel["range"].is_object(), "SelectionRange should have a range");

    // Verify parent chain exists (expanding outward from statement -> block -> sub -> file)
    // The parent field is optional but if present should be a nested SelectionRange
    if sel.get("parent").is_some() && !sel["parent"].is_null() {
        let parent = &sel["parent"];
        assert!(parent["range"].is_object(), "Parent SelectionRange should also have a range");
        // Parent range should be at least as large as the inner range
        let inner_start = sel["range"]["start"]["line"].as_u64().unwrap_or(0);
        let inner_end = sel["range"]["end"]["line"].as_u64().unwrap_or(0);
        let parent_start = parent["range"]["start"]["line"].as_u64().unwrap_or(0);
        let parent_end = parent["range"]["end"]["line"].as_u64().unwrap_or(0);
        assert!(
            parent_start <= inner_start && parent_end >= inner_end,
            "Parent range ({}-{}) should encompass inner range ({}-{})",
            parent_start,
            parent_end,
            inner_start,
            inner_end
        );
    }

    Ok(())
}

/// Test nested selection range expansion (parent chain depth)
#[test]
fn test_selection_range_nested_expansion() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_sel_nested.pl";
    harness.open(
        doc_uri,
        r#"package MyModule;

sub method {
    if (1) {
        my $deep = "nested";
        print $deep;
    }
}

1;
"#,
    )?;

    // Request selection range deep inside nested blocks
    let response = harness.request(
        "textDocument/selectionRange",
        json!({
            "textDocument": { "uri": doc_uri },
            "positions": [
                { "line": 4, "character": 12 }
            ]
        }),
    )?;

    let ranges = response.as_array().ok_or("response is not an array")?;
    assert_eq!(ranges.len(), 1, "Should return one SelectionRange");

    // Walk the parent chain to count nesting depth
    let mut depth = 0;
    let mut current = &ranges[0];
    loop {
        depth += 1;
        if current.get("parent").is_some() && !current["parent"].is_null() {
            current = &current["parent"];
        } else {
            break;
        }
        // Safety limit to avoid infinite loop on malformed data
        if depth > 20 {
            break;
        }
    }

    // We expect at least 2 levels: the variable context and some outer scope
    assert!(depth >= 2, "Should have at least 2 levels of nesting, got {}", depth);

    Ok(())
}

/// Test selection range on an empty file returns empty array
#[test]
fn test_selection_range_empty_file() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///empty_sel.pl";
    harness.open(doc_uri, "")?;

    let response = harness.request(
        "textDocument/selectionRange",
        json!({
            "textDocument": { "uri": doc_uri },
            "positions": [
                { "line": 0, "character": 0 }
            ]
        }),
    )?;

    assert!(
        response.is_array(),
        "selectionRange should return an array for empty file, got: {:?}",
        response
    );

    let ranges = response.as_array().ok_or("response is not an array")?;
    // For an empty file, we might get an array with a single range covering [0,0]-[0,0]
    // or an empty array. Both are acceptable.
    if !ranges.is_empty() {
        let sel = &ranges[0];
        assert!(sel["range"].is_object(), "Even for empty file, range should be an object");
    }

    Ok(())
}

/// Test multiple positions in a single selectionRange request
#[test]
fn test_selection_range_multiple_positions() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_sel_multi.pl";
    harness.open(
        doc_uri,
        r#"my $first = 1;
my $second = 2;
my $third = 3;

sub total {
    return $first + $second + $third;
}
"#,
    )?;

    // Request selection ranges at multiple positions simultaneously
    let response = harness.request(
        "textDocument/selectionRange",
        json!({
            "textDocument": { "uri": doc_uri },
            "positions": [
                { "line": 0, "character": 4 },
                { "line": 1, "character": 4 },
                { "line": 5, "character": 11 }
            ]
        }),
    )?;

    let ranges = response.as_array().ok_or("response is not an array")?;
    assert_eq!(
        ranges.len(),
        3,
        "Should return one SelectionRange per position, got {}",
        ranges.len()
    );

    // Each result should have a valid range
    for (i, sel) in ranges.iter().enumerate() {
        assert!(sel["range"].is_object(), "SelectionRange at index {} should have a range", i);
    }

    Ok(())
}

/// Test that selectionRangeProvider capability is advertised
#[test]
fn test_selection_range_capability_advertised() -> TestResult {
    let mut harness = LspHarness::new();
    let init_response = harness.initialize(None)?;

    let capabilities = &init_response["capabilities"];
    let has_selection_range = capabilities.get("selectionRangeProvider").is_some();
    assert!(
        has_selection_range,
        "Server should advertise selectionRangeProvider capability. Capabilities: {:?}",
        capabilities
    );

    Ok(())
}
