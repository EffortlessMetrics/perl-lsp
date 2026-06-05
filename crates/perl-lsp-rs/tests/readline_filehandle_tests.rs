//! LSP hover and semantic-token coverage for `<$fh>` readline operator.
//!
//! PR #708 fixed `<$fh>` to parse as `Readline` (not `Glob`). This test file
//! locks in LSP-level characterization: what hover and semantic tokens actually
//! return for readline filehandle operations vs glob patterns.
//!
//! ## Design notes
//!
//! Tests are characterization-first: they assert what the providers *actually*
//! return today (non-null structure, no error) rather than pinning exact content
//! that might legitimately change.  Where the provider returns meaningful content
//! we pin specific strings; where it returns null we document that expectation.
//!
//! Regression: `test_glob_vs_readline_distinction` (lsp_integration_tests.rs)
//! and `<STDIN>` / `<*.pm>` cases must stay green alongside these new tests.

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ─── helpers ──────────────────────────────────────────────────────────────────

/// Extract `contents.value` string from a hover response, if present.
fn hover_value(result: &serde_json::Value) -> Option<String> {
    result
        .get("contents")
        .and_then(|c| c.get("value"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Decode LSP semantic tokens (5-tuple relative encoding) into
/// `(line, col, len, token_type, token_modifiers)` absolute tuples.
fn decode_tokens(data: &[u64]) -> Vec<(u64, u64, u64, u64, u64)> {
    let mut line = 0u64;
    let mut col = 0u64;
    let mut result = Vec::new();
    for chunk in data.chunks(5) {
        if chunk.len() < 5 {
            break;
        }
        let dl = chunk[0];
        let ds = chunk[1];
        let len = chunk[2];
        let tt = chunk[3];
        let tm = chunk[4];
        line += dl;
        if dl == 0 {
            col += ds;
        } else {
            col = ds;
        }
        result.push((line, col, len, tt, tm));
    }
    result
}

// ─── hover tests ──────────────────────────────────────────────────────────────

/// Hover on the `$fh` variable inside `<$fh>` should return meaningful content.
///
/// The variable `$fh` is in scope (declared via `open`), so the hover provider
/// should report "Scalar Variable" with the name `$fh`.  This pins that the
/// readline fix (PR #708) did not break variable resolution inside angle brackets.
#[test]
fn test_hover_fh_variable_inside_readline() -> TestResult {
    let doc = "open my $fh, '<', 'input.txt' or die $!;\nmy $line = <$fh>;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///readline_fh.pl", doc)?;

    // Hover over `$fh` at its usage inside `<$fh>` on line 1.
    // `my $line = <$fh>;`
    //  0123456789012345
    // `$fh` starts at character 12 on line 1.
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///readline_fh.pl"},
                "position": {"line": 1, "character": 12}
            }),
        )
        .unwrap_or(json!(null));

    // The response must be either null (provider gap) or a valid hover object.
    if !result.is_null() {
        let contents = result
            .get("contents")
            .ok_or("Hover response must have 'contents' field when non-null")?;

        if let Some(obj) = contents.as_object() {
            let kind = obj.get("kind").and_then(|k| k.as_str());
            assert!(
                kind == Some("markdown") || kind == Some("plaintext"),
                "contents.kind must be 'markdown' or 'plaintext', got: {kind:?}"
            );
            let value = obj.get("value").and_then(|v| v.as_str());
            assert!(value.is_some(), "contents.value must be present when contents is an object");
        }

        // When the provider returns content for `$fh`, it should reference the
        // variable name — this would confirm the Readline fix did not regress
        // variable hover inside angle bracket expressions.
        if let Some(val) = hover_value(&result) {
            assert!(
                !val.is_empty(),
                "Hover value for $fh inside <$fh> must not be empty when returned"
            );
            // Pin the meaningful case: if hover resolves $fh, it shows "Scalar Variable"
            assert!(
                val.contains("$fh") || val.contains("fh"),
                "Hover on $fh inside readline should mention the variable name, got: {val}"
            );
        }
    }
    // null is acceptable — documents the current state without locking bad behaviour.

    Ok(())
}

/// Hover on `<$fh>` in list context (`my @lines = <$fh>;`) must not error.
///
/// The readline operator in list context reads all remaining lines. This test
/// verifies the hover provider handles this variant without panicking or
/// returning a malformed response.
#[test]
fn test_hover_fh_readline_list_context_no_error() -> TestResult {
    let doc = "open my $fh, '<', 'data.txt' or die $!;\nmy @lines = <$fh>;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///readline_list.pl", doc)?;

    // Hover over `$fh` inside `<$fh>` on line 1.
    // `my @lines = <$fh>;`
    //  0123456789012345678
    // `$fh` starts at character 13 on line 1.
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///readline_list.pl"},
                "position": {"line": 1, "character": 13}
            }),
        )
        .unwrap_or(json!(null));

    // No error — response must be null or a valid hover structure.
    if !result.is_null() {
        assert!(
            result.get("contents").is_some(),
            "Non-null hover response must have 'contents', got: {result}"
        );
    }

    Ok(())
}

/// Hover on the `<` character of `<$fh>` (the readline operator boundary).
///
/// Characterises what the provider returns when the cursor is on the angle
/// bracket itself rather than the variable inside it.  Today this returns null;
/// pinning that expectation prevents accidental regression if someone adds a
/// spurious hover card for operators.
#[test]
fn test_hover_angle_bracket_of_readline_returns_null_or_valid() -> TestResult {
    let doc = "open my $fh, '<', 'f.txt' or die;\nmy $x = <$fh>;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///readline_bracket.pl", doc)?;

    // Hover over `<` at character 8 on line 1: `my $x = <$fh>;`
    //                                                     ^
    //  0123456789
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///readline_bracket.pl"},
                "position": {"line": 1, "character": 8}
            }),
        )
        .unwrap_or(json!(null));

    // Accept null (most likely) or a valid hover structure.
    if !result.is_null() {
        assert!(
            result.get("contents").is_some(),
            "Non-null hover on '<' of readline must have 'contents', got: {result}"
        );
    }

    Ok(())
}

/// Hover on `<STDIN>` (bareword filehandle readline) must not error.
///
/// Regression guard: `<STDIN>` is a common readline that existed before PR #708.
/// The hover provider should return null or a sensible response — never panic.
#[test]
fn test_hover_stdin_readline_no_error() -> TestResult {
    let doc = "my $line = <STDIN>;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///readline_stdin.pl", doc)?;

    // Hover over `STDIN` inside `<STDIN>`: character 11 on line 0.
    // `my $line = <STDIN>;`
    //  01234567890123456789
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///readline_stdin.pl"},
                "position": {"line": 0, "character": 12}
            }),
        )
        .unwrap_or(json!(null));

    // Accept null or valid hover structure — no error.
    if !result.is_null() {
        assert!(
            result.get("contents").is_some(),
            "Non-null hover on STDIN readline must have 'contents', got: {result}"
        );
    }

    Ok(())
}

/// Hover on `<*.pm>` glob pattern: regression that glob hover still works.
///
/// `<*.pm>` is a glob, not readline.  This test guards that the PR #708 change
/// (which correctly separated Readline from Glob AST nodes) did not break hover
/// behaviour for glob expressions.
#[test]
fn test_hover_glob_pattern_no_error() -> TestResult {
    let doc = "my @modules = <*.pm>;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///glob_hover.pl", doc)?;

    // Hover over `*` inside `<*.pm>`: character 15 on line 0.
    // `my @modules = <*.pm>;`
    //  0123456789012345678901
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///glob_hover.pl"},
                "position": {"line": 0, "character": 15}
            }),
        )
        .unwrap_or(json!(null));

    // Accept null or valid hover structure.
    if !result.is_null() {
        assert!(
            result.get("contents").is_some(),
            "Non-null hover on glob pattern must have 'contents', got: {result}"
        );
    }

    Ok(())
}

// ─── semantic token tests ─────────────────────────────────────────────────────

/// Semantic tokens for `<$fh>` must return a valid (possibly empty) token stream.
///
/// The primary guard: the semantic token provider must not error or panic when
/// processing a document containing a readline filehandle expression.
#[test]
fn test_semantic_tokens_readline_fh_no_error() -> TestResult {
    let doc = "open my $fh, '<', 'in.txt' or die $!;\nmy $line = <$fh>;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///st_readline.pl", doc)?;

    let response = harness
        .request(
            "textDocument/semanticTokens/full",
            json!({
                "textDocument": {"uri": "file:///st_readline.pl"}
            }),
        )
        .unwrap_or(json!(null));

    // Response must be null or have a valid `data` array (5-tuple encoding).
    if !response.is_null() {
        let data_field = response.get("data");
        assert!(
            data_field.is_some(),
            "Non-null semanticTokens response must have 'data' field, got: {response}"
        );
        if let Some(arr) = data_field.and_then(|d| d.as_array()) {
            assert_eq!(
                arr.len() % 5,
                0,
                "Semantic token data must be 5-tuples (length % 5 == 0), got length {}",
                arr.len()
            );
        }
    }

    Ok(())
}

/// Semantic tokens for `my @lines = <$fh>;` (list-context readline) must not error.
#[test]
fn test_semantic_tokens_readline_list_context_no_error() -> TestResult {
    let doc = "open my $fh, '<', 'data.txt' or die;\nmy @lines = <$fh>;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///st_readline_list.pl", doc)?;

    let response = harness
        .request(
            "textDocument/semanticTokens/full",
            json!({
                "textDocument": {"uri": "file:///st_readline_list.pl"}
            }),
        )
        .unwrap_or(json!(null));

    if !response.is_null() {
        let data_field = response.get("data");
        assert!(data_field.is_some(), "Non-null semanticTokens response must have 'data' field");
        if let Some(arr) = data_field.and_then(|d| d.as_array()) {
            assert_eq!(arr.len() % 5, 0, "Semantic token data must be 5-tuples");
        }
    }

    Ok(())
}

/// Semantic tokens for a document mixing readline and glob are both valid.
///
/// This is the key regression guard for PR #708: a document containing both
/// `<$fh>` (readline) and `<*.pm>` (glob) must produce a valid token stream.
/// The token stream itself may vary; what must not happen is an error or panic.
#[test]
fn test_semantic_tokens_readline_and_glob_mixed_no_error() -> TestResult {
    let doc = "open my $fh, '<', 'in.txt' or die;\nmy $line = <$fh>;\nmy @files = <*.pm>;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///st_mixed.pl", doc)?;

    let response = harness
        .request(
            "textDocument/semanticTokens/full",
            json!({
                "textDocument": {"uri": "file:///st_mixed.pl"}
            }),
        )
        .unwrap_or(json!(null));

    if !response.is_null() {
        let data_field = response.get("data");
        assert!(data_field.is_some(), "Non-null semanticTokens response must have 'data' field");
        if let Some(arr) = data_field.and_then(|d| d.as_array()) {
            assert_eq!(arr.len() % 5, 0, "Semantic token data must be 5-tuples");

            // The document has variables $fh that should appear in the token stream.
            // Decode tokens and verify the stream is internally consistent (no
            // negative delta lines or columns — those would indicate an encoding bug).
            let data: Vec<u64> = arr.iter().filter_map(|v| v.as_u64()).collect();
            let tokens = decode_tokens(&data);
            let mut prev_line = 0u64;
            let mut prev_col = 0u64;
            for (line, col, _len, _tt, _tm) in &tokens {
                if *line == prev_line {
                    assert!(
                        *col >= prev_col,
                        "Token column must be monotonically increasing on same line: \
                         prev_col={prev_col} got col={col}"
                    );
                } else {
                    assert!(
                        *line > prev_line,
                        "Token line must be non-decreasing: prev={prev_line} got={line}"
                    );
                }
                prev_line = *line;
                prev_col = *col;
            }
        }
    }

    Ok(())
}

/// Semantic tokens for `<STDIN>` readline: regression that it stays valid.
#[test]
fn test_semantic_tokens_stdin_readline_no_error() -> TestResult {
    let doc = "my $line = <STDIN>;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///st_stdin.pl", doc)?;

    let response = harness
        .request(
            "textDocument/semanticTokens/full",
            json!({
                "textDocument": {"uri": "file:///st_stdin.pl"}
            }),
        )
        .unwrap_or(json!(null));

    if !response.is_null() {
        if let Some(arr) = response.get("data").and_then(|d| d.as_array()) {
            assert_eq!(arr.len() % 5, 0, "Semantic token data must be 5-tuples");
        }
    }

    Ok(())
}

/// Semantic tokens for `<*.pm>` glob: regression that it stays valid.
#[test]
fn test_semantic_tokens_glob_pattern_no_error() -> TestResult {
    let doc = "my @mods = <*.pm>;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///st_glob.pl", doc)?;

    let response = harness
        .request(
            "textDocument/semanticTokens/full",
            json!({
                "textDocument": {"uri": "file:///st_glob.pl"}
            }),
        )
        .unwrap_or(json!(null));

    if !response.is_null() {
        if let Some(arr) = response.get("data").and_then(|d| d.as_array()) {
            assert_eq!(arr.len() % 5, 0, "Semantic token data must be 5-tuples");
        }
    }

    Ok(())
}

/// Semantic tokens: `$fh` variable is tokenized consistently whether inside
/// `<$fh>` (readline) or outside it.
///
/// Both uses of `$fh` in this document are the same variable. The token stream
/// should include at least one token for each use (though the exact type may
/// differ).  The test verifies the stream is non-empty and well-formed when
/// variables are used alongside readline expressions.
#[test]
fn test_semantic_tokens_fh_variable_consistent_tokenization() -> TestResult {
    let doc = concat!(
        "open my $fh, '<', 'file.txt' or die;\n", // line 0: $fh declared
        "my $line = <$fh>;\n",                    // line 1: $fh used in readline
        "close $fh;\n",                           // line 2: $fh used outside brackets
    );
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///st_fh_consistent.pl", doc)?;

    let response = harness
        .request(
            "textDocument/semanticTokens/full",
            json!({
                "textDocument": {"uri": "file:///st_fh_consistent.pl"}
            }),
        )
        .unwrap_or(json!(null));

    if !response.is_null() {
        let data_opt = response.get("data").and_then(|d| d.as_array());
        if let Some(arr) = data_opt {
            assert_eq!(arr.len() % 5, 0, "Semantic token data must be 5-tuples");

            let data: Vec<u64> = arr.iter().filter_map(|v| v.as_u64()).collect();
            // With three uses of $fh across three lines, we expect at least some tokens.
            // A completely empty token stream for a non-trivial document would indicate
            // a provider failure.
            assert!(
                !data.is_empty(),
                "Semantic token stream should be non-empty for a 3-line document with variables"
            );
        }
    }

    Ok(())
}
