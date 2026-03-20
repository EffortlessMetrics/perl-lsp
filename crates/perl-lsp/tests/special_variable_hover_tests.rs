mod support;

use perl_tdd_support::must_some;
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
fn test_hover_inc_array() -> TestResult {
    let doc = "push @INC, '/lib';\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///inc.pl", doc)?;
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///inc.pl"},
                "position": {"line": 0, "character": 6}
            }),
        )
        .unwrap_or(json!(null));
    let val = must_some(hover_value(&result));
    assert!(val.contains("Module Search Paths"), "got: {val}");
    Ok(())
}

#[test]
fn test_hover_inc_hash() -> TestResult {
    let doc = "print keys %INC;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///inc_hash.pl", doc)?;
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///inc_hash.pl"},
                "position": {"line": 0, "character": 12}
            }),
        )
        .unwrap_or(json!(null));
    let val = must_some(hover_value(&result));
    assert!(val.contains("Loaded Modules"), "got: {val}");
    Ok(())
}

#[test]
fn test_hover_env_hash() -> TestResult {
    let doc = "my @keys = keys %ENV;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///env.pl", doc)?;
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///env.pl"},
                "position": {"line": 0, "character": 17}
            }),
        )
        .unwrap_or(json!(null));
    let val = must_some(hover_value(&result));
    assert!(val.contains("Environment Variables"), "got: {val}");
    Ok(())
}

#[test]
fn test_hover_isa_undeclared() -> TestResult {
    let doc = "push @ISA, 'Base';\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///isa.pl", doc)?;
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///isa.pl"},
                "position": {"line": 0, "character": 6}
            }),
        )
        .unwrap_or(json!(null));
    let val = must_some(hover_value(&result));
    assert!(val.contains("Inheritance"), "got: {val}");
    Ok(())
}

#[test]
fn test_hover_default_variable() -> TestResult {
    let doc = "print $_;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///default.pl", doc)?;
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///default.pl"},
                "position": {"line": 0, "character": 7}
            }),
        )
        .unwrap_or(json!(null));
    if !result.is_null() {
        let val = hover_value(&result);
        assert!(val.is_some(), "Hover should have content");
    }
    Ok(())
}

#[test]
fn test_hover_special_variables_return_markdown() -> TestResult {
    let doc = "push @INC, '/lib';\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///md.pl", doc)?;
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///md.pl"},
                "position": {"line": 0, "character": 6}
            }),
        )
        .unwrap_or(json!(null));
    if !result.is_null() {
        let kind = result.get("contents").and_then(|c| c.get("kind")).and_then(|k| k.as_str());
        assert_eq!(kind, Some("markdown"), "Hover content should be markdown");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// New tests for issue #2347 – extended special variable hover coverage
// ---------------------------------------------------------------------------

#[test]
fn test_hover_child_process_status() -> TestResult {
    // $? is set after system(), backtick, or waitpid
    // "system('ls'); my $rc = $?;\n"
    //  0123456789012345678901234
    // $? is at byte offset 23
    let doc = "system('ls'); my $rc = $?;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///child_status2347.pl", doc)?;
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///child_status2347.pl"},
                "position": {"line": 0, "character": 23}
            }),
        )
        .unwrap_or(json!(null));
    let val = must_some(hover_value(&result));
    let lower = val.to_lowercase();
    assert!(
        lower.contains("child") || lower.contains("status") || lower.contains("exit"),
        "$? hover should mention child process status, got: {val}"
    );
    Ok(())
}

#[test]
fn test_hover_perl_version_variable() -> TestResult {
    // $^V is the Perl version object (v-string like v5.38.0)
    // "print $^V;\n"
    //  0123456789
    // $^V starts at offset 6
    let doc = "print $^V;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///version2347.pl", doc)?;
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///version2347.pl"},
                "position": {"line": 0, "character": 7}
            }),
        )
        .unwrap_or(json!(null));
    let val = must_some(hover_value(&result));
    let lower = val.to_lowercase();
    assert!(
        lower.contains("version") || lower.contains("perl"),
        "$^V hover should mention Perl version, got: {val}"
    );
    Ok(())
}

#[test]
fn test_hover_argv_array() -> TestResult {
    // @ARGV holds command-line arguments
    // "my $first = shift @ARGV;\n"
    //  0         1         2
    //  0123456789012345678901234
    // @ARGV starts at offset 18
    let doc = "my $first = shift @ARGV;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///argv2347.pl", doc)?;
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///argv2347.pl"},
                "position": {"line": 0, "character": 19}
            }),
        )
        .unwrap_or(json!(null));
    let val = must_some(hover_value(&result));
    let lower = val.to_lowercase();
    assert!(
        lower.contains("argv") || lower.contains("command") || lower.contains("argument"),
        "@ARGV hover should mention command-line arguments, got: {val}"
    );
    Ok(())
}
