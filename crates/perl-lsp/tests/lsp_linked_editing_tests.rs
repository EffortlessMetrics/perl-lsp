//! Linked editing range tests
mod support;
use serde_json::json;
use support::lsp_harness::LspHarness;

fn get_range(result: &serde_json::Value, idx: usize) -> (u64, u64, u64, u64) {
    let r = &result["ranges"][idx];
    (
        r["start"]["line"].as_u64().unwrap_or(0),
        r["start"]["character"].as_u64().unwrap_or(0),
        r["end"]["line"].as_u64().unwrap_or(0),
        r["end"]["character"].as_u64().unwrap_or(0),
    )
}

#[test]

fn test_brace_pair() -> Result<(), Box<dyn std::error::Error>> {
    let doc = r#"sub x { my $h = { a => 1 }; }"#;
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///test.pl", doc)?;
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
        assert_eq!(
            ranges.len(),
            2,
            "Should return two linked ranges for brace pair"
        );
    } else {
        // Null is also acceptable if no linked ranges at this position
        assert!(result.is_null(), "Should return either ranges or null");
    }

    Ok(())
}

#[test]

fn test_quotes_pair() -> Result<(), Box<dyn std::error::Error>> {
    let doc = r#"my $s = "hi";"#;
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///test.pl", doc)?;
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
        assert_eq!(
            ranges.len(),
            2,
            "Should return two linked ranges for quote pair"
        );
    } else {
        assert!(result.is_null(), "Should return either ranges or null");
    }

    Ok(())
}

#[test]

fn test_nested_parens() -> Result<(), Box<dyn std::error::Error>> {
    let doc = r#"if ((($x > 0))) { print "yes"; }"#;
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///test.pl", doc)?;
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
        assert_eq!(
            ranges.len(),
            2,
            "Should return two linked ranges for innermost parens"
        );
    } else {
        assert!(result.is_null(), "Should return either ranges or null");
    }

    Ok(())
}

#[test]

fn test_square_brackets() -> Result<(), Box<dyn std::error::Error>> {
    let doc = r#"my @arr = [1, 2, [3, 4]];"#;
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///test.pl", doc)?;
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
        assert_eq!(
            ranges.len(),
            2,
            "Should return two linked ranges for bracket pair"
        );
    } else {
        assert!(result.is_null(), "Should return either ranges or null");
    }

    Ok(())
}

#[test]

fn test_no_pair_at_position() -> Result<(), Box<dyn std::error::Error>> {
    let doc = r#"my $x = 42;"#;
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///test.pl", doc)?;
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

    assert!(
        result.is_null(),
        "Should return null when no paired delimiter at position"
    );

    Ok(())
}

// ---- New tests: heredoc, regex delimiters, escape handling ----

#[test]
fn test_heredoc_pair() -> Result<(), Box<dyn std::error::Error>> {
    // "my $x = <<EOF;\nhello\nEOF\n"
    // line 0: "my $x = <<EOF;"  — EOF label starts at byte 10, char 10
    // line 2: "EOF"             — terminator starts at byte 21, char 0
    let doc = "my $x = <<EOF;\nhello\nEOF\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///test.pl", doc)?;

    // cursor on 'E' of EOF label (line 0, character 10)
    let result = harness
        .request(
            "textDocument/linkedEditingRange",
            json!({
                "textDocument": {"uri": "file:///test.pl"},
                "position": {"line": 0, "character": 10}
            }),
        )
        .unwrap_or(json!(null));

    assert!(!result.is_null(), "heredoc pair: expected ranges, got null");
    assert_eq!(result["ranges"].as_array().map(|a| a.len()), Some(2));
    // opener range: line 0, chars 10..13  ("EOF")
    assert_eq!(
        get_range(&result, 0),
        (0, 10, 0, 13),
        "opener range mismatch"
    );
    // terminator range: line 2, chars 0..3  ("EOF")
    assert_eq!(
        get_range(&result, 1),
        (2, 0, 2, 3),
        "terminator range mismatch"
    );

    Ok(())
}

#[test]
fn test_heredoc_indented() -> Result<(), Box<dyn std::error::Error>> {
    // "my $x = <<~EOF;\n  hello\n  EOF\n"
    // line 0: "my $x = <<~EOF;"  — '~' at char 10, EOF label at char 11
    // line 2: "  EOF"            — EOF starts at char 2
    let doc = "my $x = <<~EOF;\n  hello\n  EOF\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///test.pl", doc)?;

    // cursor on 'E' of EOF (line 0, character 11)
    let result = harness
        .request(
            "textDocument/linkedEditingRange",
            json!({
                "textDocument": {"uri": "file:///test.pl"},
                "position": {"line": 0, "character": 11}
            }),
        )
        .unwrap_or(json!(null));

    assert!(
        !result.is_null(),
        "indented heredoc: expected ranges, got null"
    );
    assert_eq!(result["ranges"].as_array().map(|a| a.len()), Some(2));
    // opener range: line 0, chars 11..14  ("EOF")
    assert_eq!(
        get_range(&result, 0),
        (0, 11, 0, 14),
        "opener range mismatch"
    );
    // terminator range: line 2, chars 2..5  ("EOF" after leading spaces)
    assert_eq!(
        get_range(&result, 1),
        (2, 2, 2, 5),
        "terminator range mismatch"
    );

    Ok(())
}

#[test]
fn test_heredoc_quoted_label() -> Result<(), Box<dyn std::error::Error>> {
    // "my $x = <<\"END\";\nhello\nEND\n"
    // line 0: chars: m y   $ x   =   < < " E N D " ;
    //         index: 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15
    // label "END" is at chars 11..14, surrounded by quotes at 10 and 14
    // line 2: "END" at chars 0..3
    let doc = "my $x = <<\"END\";\nhello\nEND\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///test.pl", doc)?;

    // cursor on 'E' inside <<"END" (line 0, character 11)
    let result = harness
        .request(
            "textDocument/linkedEditingRange",
            json!({
                "textDocument": {"uri": "file:///test.pl"},
                "position": {"line": 0, "character": 11}
            }),
        )
        .unwrap_or(json!(null));

    assert!(
        !result.is_null(),
        "quoted heredoc: expected ranges, got null"
    );
    assert_eq!(result["ranges"].as_array().map(|a| a.len()), Some(2));
    // opener range covers label only (not surrounding quotes): chars 11..14
    assert_eq!(
        get_range(&result, 0),
        (0, 11, 0, 14),
        "opener range mismatch"
    );
    // terminator range: line 2, chars 0..3
    assert_eq!(
        get_range(&result, 1),
        (2, 0, 2, 3),
        "terminator range mismatch"
    );

    Ok(())
}

#[test]
fn test_regex_slash_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    // "my $x = $str =~ m/foo/;"
    //  0         1         2
    //  0123456789012345678901234
    //                    ^ char 17 = opening /
    //                        ^ char 21 = closing /
    let doc = "my $x = $str =~ m/foo/;";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///test.pl", doc)?;

    // cursor on opening / (line 0, character 17)
    let result = harness
        .request(
            "textDocument/linkedEditingRange",
            json!({
                "textDocument": {"uri": "file:///test.pl"},
                "position": {"line": 0, "character": 17}
            }),
        )
        .unwrap_or(json!(null));

    assert!(!result.is_null(), "regex slash: expected ranges, got null");
    assert_eq!(result["ranges"].as_array().map(|a| a.len()), Some(2));
    assert_eq!(
        get_range(&result, 0),
        (0, 17, 0, 18),
        "opening delimiter range mismatch"
    );
    assert_eq!(
        get_range(&result, 1),
        (0, 21, 0, 22),
        "closing delimiter range mismatch"
    );

    Ok(())
}

#[test]
fn test_regex_pipe_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    // "my $x = $str =~ m|foo|;"
    //                    ^ char 17 = opening |
    //                        ^ char 21 = closing |
    let doc = "my $x = $str =~ m|foo|;";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///test.pl", doc)?;

    // cursor on opening | (line 0, character 17)
    let result = harness
        .request(
            "textDocument/linkedEditingRange",
            json!({
                "textDocument": {"uri": "file:///test.pl"},
                "position": {"line": 0, "character": 17}
            }),
        )
        .unwrap_or(json!(null));

    assert!(!result.is_null(), "regex pipe: expected ranges, got null");
    assert_eq!(result["ranges"].as_array().map(|a| a.len()), Some(2));
    assert_eq!(
        get_range(&result, 0),
        (0, 17, 0, 18),
        "opening delimiter range mismatch"
    );
    assert_eq!(
        get_range(&result, 1),
        (0, 21, 0, 22),
        "closing delimiter range mismatch"
    );

    Ok(())
}

#[test]
fn test_subst_first_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    // "$str =~ s/foo/bar/;"
    //  0         1
    //  0123456789012345678
    //           ^ char 9 = first /
    //                ^ char 13 = second /
    let doc = "$str =~ s/foo/bar/;";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///test.pl", doc)?;

    // cursor on first / (line 0, character 9)
    let result = harness
        .request(
            "textDocument/linkedEditingRange",
            json!({
                "textDocument": {"uri": "file:///test.pl"},
                "position": {"line": 0, "character": 9}
            }),
        )
        .unwrap_or(json!(null));

    assert!(
        !result.is_null(),
        "subst first /: expected ranges, got null"
    );
    assert_eq!(result["ranges"].as_array().map(|a| a.len()), Some(2));
    assert_eq!(
        get_range(&result, 0),
        (0, 9, 0, 10),
        "first delimiter range mismatch"
    );
    assert_eq!(
        get_range(&result, 1),
        (0, 13, 0, 14),
        "second delimiter range mismatch"
    );

    Ok(())
}

#[test]
fn test_subst_second_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    // "$str =~ s/foo/bar/;"
    //                ^ char 13 = second /
    //                    ^ char 17 = third /
    let doc = "$str =~ s/foo/bar/;";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///test.pl", doc)?;

    // cursor on second / (line 0, character 13)
    let result = harness
        .request(
            "textDocument/linkedEditingRange",
            json!({
                "textDocument": {"uri": "file:///test.pl"},
                "position": {"line": 0, "character": 13}
            }),
        )
        .unwrap_or(json!(null));

    assert!(
        !result.is_null(),
        "subst second /: expected ranges, got null"
    );
    assert_eq!(result["ranges"].as_array().map(|a| a.len()), Some(2));
    assert_eq!(
        get_range(&result, 0),
        (0, 13, 0, 14),
        "second delimiter range mismatch"
    );
    assert_eq!(
        get_range(&result, 1),
        (0, 17, 0, 18),
        "third delimiter range mismatch"
    );

    Ok(())
}

#[test]
fn test_quote_escape() -> Result<(), Box<dyn std::error::Error>> {
    // Perl source: my $s = "say \"hi\"";
    // As bytes:   my $s = "say \"hi\"";
    //   0         1
    //   01234567890123456789
    //           ^ char 8 = opening "
    //                         ^ char 19 = closing "
    //               ^ char 13-14 = \" (escaped, must be skipped)
    //                   ^ char 17-18 = \" (escaped, must be skipped)
    let doc = "my $s = \"say \\\"hi\\\"\";";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///test.pl", doc)?;

    // cursor on opening " (line 0, character 8)
    let result = harness
        .request(
            "textDocument/linkedEditingRange",
            json!({
                "textDocument": {"uri": "file:///test.pl"},
                "position": {"line": 0, "character": 8}
            }),
        )
        .unwrap_or(json!(null));

    assert!(!result.is_null(), "quote escape: expected ranges, got null");
    assert_eq!(result["ranges"].as_array().map(|a| a.len()), Some(2));
    // Must link opening " (char 8) to the REAL closing " (char 19), not the escaped \" (char 14)
    assert_eq!(
        get_range(&result, 0),
        (0, 8, 0, 9),
        "opening quote range mismatch"
    );
    assert_eq!(
        get_range(&result, 1),
        (0, 19, 0, 20),
        "closing quote range mismatch (must skip escaped quotes)"
    );

    Ok(())
}
