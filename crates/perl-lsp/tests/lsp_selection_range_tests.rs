//! Tests for textDocument/selectionRange LSP feature
//!
//! Validates the selection range provider functionality including:
//! - Selection expansion on a variable
//! - Selection expansion on a subroutine body
//! - Nested parent chain (expanding outward)
//! - Empty file handling
//! - Multiple positions in a single request
//! - String content expansion chain
//! - Hash access expansion chain
//! - Function name / signature expansion chain

mod support;
use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Walk a JSON SelectionRange chain and collect (start_line, start_char,
/// end_line, end_char) tuples from innermost to outermost.
fn collect_chain(sel: &serde_json::Value) -> Vec<(u64, u64, u64, u64)> {
    let mut chain = Vec::new();
    let mut cur = sel;
    for _ in 0..50 {
        let r = &cur["range"];
        if !r.is_object() {
            break;
        }
        let sl = r["start"]["line"].as_u64().unwrap_or(0);
        let sc = r["start"]["character"].as_u64().unwrap_or(0);
        let el = r["end"]["line"].as_u64().unwrap_or(0);
        let ec = r["end"]["character"].as_u64().unwrap_or(0);
        chain.push((sl, sc, el, ec));
        if cur.get("parent").is_some() && !cur["parent"].is_null() {
            cur = &cur["parent"];
        } else {
            break;
        }
    }
    chain
}

/// Assert that every parent range in the chain strictly encompasses (or equals)
/// the child range.
fn assert_chain_monotonic(chain: &[(u64, u64, u64, u64)], label: &str) {
    for w in chain.windows(2) {
        let (is, ic, ie, iec) = w[0]; // inner
        let (os, oc, oe, oec) = w[1]; // outer
        // outer start must be <= inner start AND outer end must be >= inner end
        let start_ok = os < is || (os == is && oc <= ic);
        let end_ok = oe > ie || (oe == ie && oec >= iec);
        assert!(
            start_ok && end_ok,
            "{}: parent ({},{})..({},{}) does not encompass child ({},{})..({},{})",
            label,
            os,
            oc,
            oe,
            oec,
            is,
            ic,
            ie,
            iec,
        );
    }
}

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

// =========================================================================
// Scenario 1: Cursor inside a string expands through
//   string content -> full string -> expression -> statement -> block -> function -> file
// =========================================================================

/// Cursor inside a quoted string should produce a chain that grows from
/// the innermost node outward through to the file root.
#[test]
fn test_selection_range_string_expansion_chain() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_sel_string.pl";
    harness.open(
        doc_uri,
        // Line 0: sub greet {
        // Line 1:     my $msg = "hello world";
        // Line 2: }
        "sub greet {\n    my $msg = \"hello world\";\n}\n",
    )?;

    // Place cursor on the 'w' of "world" inside the string (line 1, col 22)
    let response = harness.request(
        "textDocument/selectionRange",
        json!({
            "textDocument": { "uri": doc_uri },
            "positions": [
                { "line": 1, "character": 22 }
            ]
        }),
    )?;

    let ranges = response.as_array().ok_or("not an array")?;
    assert_eq!(ranges.len(), 1);

    let chain = collect_chain(&ranges[0]);

    // The chain should have at least 3 levels (leaf, some intermediate, root)
    assert!(
        chain.len() >= 3,
        "String expansion should produce >= 3 levels, got {} levels: {:?}",
        chain.len(),
        chain,
    );

    // Verify monotonically growing containment
    assert_chain_monotonic(&chain, "string expansion");

    // The outermost range must start at line 0 (the file / sub start)
    let outermost = chain.last().ok_or("empty chain")?;
    assert_eq!(outermost.0, 0, "outermost range should start at line 0");

    Ok(())
}

// =========================================================================
// Scenario 2: Cursor inside a hash access expands through
//   key -> subscript {key} -> full expression $h{key}
// =========================================================================

/// Cursor on the key inside a hash access should expand through the
/// subscript and then the full hash-access expression.
#[test]
fn test_selection_range_hash_access_expansion() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_sel_hash.pl";
    harness.open(
        doc_uri,
        // Line 0: sub lookup {
        // Line 1:     my %data = (name => "alice");
        // Line 2:     return $data{name};
        // Line 3: }
        "sub lookup {\n    my %data = (name => \"alice\");\n    return $data{name};\n}\n",
    )?;

    // Place cursor on 'n' of the hash key `name` in $data{name}
    // Line 2: "    return $data{name};"
    //          0123456789012345678
    //                              ^--- 'name' starts at col 17
    let response = harness.request(
        "textDocument/selectionRange",
        json!({
            "textDocument": { "uri": doc_uri },
            "positions": [
                { "line": 2, "character": 17 }
            ]
        }),
    )?;

    let ranges = response.as_array().ok_or("not an array")?;
    assert_eq!(ranges.len(), 1);

    let chain = collect_chain(&ranges[0]);

    // Should have at least 3 levels
    assert!(
        chain.len() >= 3,
        "Hash access expansion should produce >= 3 levels, got {} levels: {:?}",
        chain.len(),
        chain,
    );

    assert_chain_monotonic(&chain, "hash access");

    // The innermost range should be on line 2
    let innermost = &chain[0];
    assert_eq!(innermost.0, 2, "innermost range should be on line 2");

    Ok(())
}

// =========================================================================
// Scenario 3: Cursor on function name expands through
//   name -> signature -> full sub definition
// =========================================================================

/// Cursor on the subroutine name should expand through the name, then
/// (if present) the signature, then the full sub definition, then the file.
#[test]
fn test_selection_range_function_name_expansion() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_sel_func.pl";
    harness.open(
        doc_uri,
        // Line 0: sub calculate {
        // Line 1:     my ($a, $b) = @_;
        // Line 2:     return $a + $b;
        // Line 3: }
        "sub calculate {\n    my ($a, $b) = @_;\n    return $a + $b;\n}\n",
    )?;

    // Place cursor on the 'c' of 'calculate' (line 0, col 4)
    let response = harness.request(
        "textDocument/selectionRange",
        json!({
            "textDocument": { "uri": doc_uri },
            "positions": [
                { "line": 0, "character": 4 }
            ]
        }),
    )?;

    let ranges = response.as_array().ok_or("not an array")?;
    assert_eq!(ranges.len(), 1);

    let chain = collect_chain(&ranges[0]);

    // Should have at least 2 levels: the name/identifier and the full sub/file
    assert!(
        chain.len() >= 2,
        "Function name expansion should produce >= 2 levels, got {} levels: {:?}",
        chain.len(),
        chain,
    );

    assert_chain_monotonic(&chain, "function name");

    // The innermost range should start on line 0
    let innermost = &chain[0];
    assert_eq!(innermost.0, 0, "innermost range should be on line 0");

    // The outermost range should cover the entire file (start at 0,0)
    let outermost = chain.last().ok_or("empty chain")?;
    assert_eq!(outermost.0, 0, "outermost start line");
    assert_eq!(outermost.1, 0, "outermost start char");

    Ok(())
}

/// Verify that selection range chains never produce duplicate consecutive ranges.
#[test]
fn test_selection_range_no_duplicate_ranges() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_sel_nodup.pl";
    harness.open(doc_uri, "sub foo {\n    my $x = 42;\n    print $x;\n}\n")?;

    let response = harness.request(
        "textDocument/selectionRange",
        json!({
            "textDocument": { "uri": doc_uri },
            "positions": [
                { "line": 1, "character": 11 }
            ]
        }),
    )?;

    let ranges = response.as_array().ok_or("not an array")?;
    assert_eq!(ranges.len(), 1);

    let chain = collect_chain(&ranges[0]);

    // No two consecutive levels should have the exact same range
    for w in chain.windows(2) {
        assert_ne!(w[0], w[1], "consecutive selection ranges should not be identical: {:?}", w[0]);
    }

    Ok(())
}

/// Verify deeply nested code produces a rich selection chain.
#[test]
fn test_selection_range_deep_nesting() -> TestResult {
    let mut harness = LspHarness::new();
    let _init = harness.initialize(None)?;

    let doc_uri = "file:///test_sel_deep.pl";
    harness.open(
        doc_uri,
        concat!(
            "sub outer {\n",
            "    if (1) {\n",
            "        while (1) {\n",
            "            my $deep = 1;\n",
            "        }\n",
            "    }\n",
            "}\n",
        ),
    )?;

    // Cursor on `$deep` (line 3, col 16)
    let response = harness.request(
        "textDocument/selectionRange",
        json!({
            "textDocument": { "uri": doc_uri },
            "positions": [
                { "line": 3, "character": 16 }
            ]
        }),
    )?;

    let ranges = response.as_array().ok_or("not an array")?;
    assert_eq!(ranges.len(), 1);

    let chain = collect_chain(&ranges[0]);

    // With deep nesting we should have many levels:
    // $deep -> VariableDeclaration -> Block(while) -> While ->
    // Block(if) -> If -> Block(sub) -> Subroutine -> Program
    assert!(
        chain.len() >= 4,
        "Deeply nested code should produce >= 4 levels, got {} levels: {:?}",
        chain.len(),
        chain,
    );

    assert_chain_monotonic(&chain, "deep nesting");

    Ok(())
}
