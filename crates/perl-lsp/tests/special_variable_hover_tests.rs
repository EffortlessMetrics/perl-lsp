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
    let val = hover_value(&result).ok_or("Expected hover content for $_")?;
    let lower = val.to_lowercase();
    assert!(
        lower.contains("default") || lower.contains("$_"),
        "$_ hover should mention 'default' or '$_', got: {val}"
    );
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
        let kind = result
            .get("contents")
            .and_then(|c| c.get("kind"))
            .and_then(|k| k.as_str());
        assert_eq!(kind, Some("markdown"), "Hover content should be markdown");
    }
    Ok(())
}

#[test]
fn test_hover_internal_pl_sv_special_variables() -> TestResult {
    let doc = "print $PL_sv_yes; print $PL_sv_no; print $PL_sv_undef;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///pl_sv_internal.pl", doc)?;

    for (needle, expected) in [
        ("$PL_sv_yes", "true scalar"),
        ("$PL_sv_no", "false scalar"),
        ("$PL_sv_undef", "undefined scalar"),
    ] {
        let character = doc.find(needle).ok_or("needle not found")?;
        let result = harness
            .request(
                "textDocument/hover",
                json!({
                    "textDocument": {"uri": "file:///pl_sv_internal.pl"},
                    "position": {"line": 0, "character": character}
                }),
            )
            .unwrap_or(json!(null));
        let val = hover_value(&result).ok_or("Expected hover content for PL_sv_*")?;
        let lower = val.to_lowercase();
        assert!(
            lower.contains(expected) || lower.contains("internal special variable"),
            "{needle} hover should mention {expected}, got: {val}"
        );
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

#[test]
fn test_hover_sig_hash() -> TestResult {
    // %SIG maps signal names to handlers
    // "$SIG{INT} = 'IGNORE';\n"
    //  0123456789
    // %SIG-like access at $SIG — position 0 is '$', character 1 is 'S'
    // Use the %SIG form directly:
    // "my %h = %SIG;\n"  — %SIG starts at offset 8
    let doc = "my %h = %SIG;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///sig2347.pl", doc)?;
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///sig2347.pl"},
                "position": {"line": 0, "character": 9}
            }),
        )
        .unwrap_or(json!(null));
    let val = hover_value(&result).ok_or("Expected hover for %SIG")?;
    let lower = val.to_lowercase();
    assert!(
        lower.contains("signal") || lower.contains("sig") || lower.contains("handler"),
        "%SIG hover should mention signal handlers, got: {val}"
    );
    Ok(())
}

#[test]
fn test_hover_process_id() -> TestResult {
    // $$ is the process ID
    // "print $$;\n"  — $$ starts at offset 6
    let doc = "print $$;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///pid2347.pl", doc)?;
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///pid2347.pl"},
                "position": {"line": 0, "character": 6}
            }),
        )
        .unwrap_or(json!(null));
    let val = hover_value(&result).ok_or("Expected hover for $$")?;
    let lower = val.to_lowercase();
    assert!(
        lower.contains("pid") || lower.contains("process"),
        "$$ hover should mention process ID, got: {val}"
    );
    Ok(())
}

#[test]
fn test_hover_warning_flag() -> TestResult {
    // $^W is the global warning flag
    // "local $^W = 1;\n"  — $^W starts at offset 6
    let doc = "local $^W = 1;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///warn2347.pl", doc)?;
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///warn2347.pl"},
                "position": {"line": 0, "character": 7}
            }),
        )
        .unwrap_or(json!(null));
    let val = hover_value(&result).ok_or("Expected hover for $^W")?;
    let lower = val.to_lowercase();
    assert!(
        lower.contains("warn") || lower.contains("flag"),
        "$^W hover should mention warning flag, got: {val}"
    );
    Ok(())
}

#[test]
fn test_hover_last_bracket_matched() -> TestResult {
    // $+ is the last successful capture group bracket
    // "\"foo\" =~ /(o+)/; print $+;\n"
    //  0         1         2
    //  0123456789012345678901234567
    // $+ starts at offset 24
    let doc = "\"foo\" =~ /(o+)/; print $+;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///lastbracket2347.pl", doc)?;
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///lastbracket2347.pl"},
                "position": {"line": 0, "character": 23}
            }),
        )
        .unwrap_or(json!(null));
    let val = hover_value(&result).ok_or("Expected hover for $+")?;
    let lower = val.to_lowercase();
    assert!(
        lower.contains("bracket") || lower.contains("capture") || lower.contains("last"),
        "$+ hover should mention last bracket matched, got: {val}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests for issue #2831 – Phase 1 hover doc improvements
// ---------------------------------------------------------------------------

/// Hovering on $1 (first capture group) should show capture group documentation.
#[test]
fn test_hover_capture_group_dollar_1() -> TestResult {
    // Use a multi-line doc so the hover is on a plain $1 reference
    // without any nearby regex literal to confuse the regex hover path.
    // Line 0: m{...} match
    // Line 1: print $1;   <- hover here at character 6
    let doc = "m{hello};\nprint $1;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///capture1_2831.pl", doc)?;
    // line 1: "print $1;" — character 7 = '1' (the digit after '$')
    // get_token_at_position expands backward to include the '$' sigil
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///capture1_2831.pl"},
                "position": {"line": 1, "character": 7}
            }),
        )
        .unwrap_or(json!(null));
    let val = must_some(hover_value(&result));
    let lower = val.to_lowercase();
    // Must specifically mention capture group semantics
    assert!(
        lower.contains("capture") || lower.contains("match"),
        "$1 hover should mention capture group semantics, got: {val}"
    );
    Ok(())
}

/// Hovering on $9 (last supported capture group) should show capture group documentation.
#[test]
fn test_hover_capture_group_dollar_9() -> TestResult {
    // Line 0: m{...} match
    // Line 1: print $9;   <- hover here at character 6
    let doc = "m{abcdefghi};\nprint $9;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///capture9_2831.pl", doc)?;
    // line 1: "print $9;" — character 7 = '9' (the digit after '$')
    // get_token_at_position expands backward to include the '$' sigil
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///capture9_2831.pl"},
                "position": {"line": 1, "character": 7}
            }),
        )
        .unwrap_or(json!(null));
    let val = must_some(hover_value(&result));
    let lower = val.to_lowercase();
    // Must specifically mention capture group semantics
    assert!(
        lower.contains("capture") || lower.contains("match"),
        "$9 hover should mention capture group semantics, got: {val}"
    );
    Ok(())
}

/// Hovering on $| should show output autoflush documentation.
#[test]
fn test_hover_output_autoflush() -> TestResult {
    // "$| = 1;  # enable autoflush\n"
    //  0123456789
    // $| starts at offset 0
    let doc = "$| = 1;  # enable autoflush\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///autoflush_2831.pl", doc)?;
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///autoflush_2831.pl"},
                "position": {"line": 0, "character": 0}
            }),
        )
        .unwrap_or(json!(null));
    let val = must_some(hover_value(&result));
    let lower = val.to_lowercase();
    assert!(
        lower.contains("autoflush") || lower.contains("flush") || lower.contains("buffer"),
        "$| hover should mention autoflush/buffer, got: {val}"
    );
    Ok(())
}

/// $0 is the program name, NOT a capture group variable.
/// The capture group handler must only match $1–$9 (b'1'..=b'9'), never $0.
#[test]
fn test_hover_dollar_zero_is_not_capture_group() -> TestResult {
    // $0 holds the script name in Perl — it must not show capture group docs.
    let doc = "print $0;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///dollar_zero_2831.pl", doc)?;
    // $0 starts at character 6; hover on the '0'
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///dollar_zero_2831.pl"},
                "position": {"line": 0, "character": 7}
            }),
        )
        .unwrap_or(json!(null));
    // $0 may or may not have hover docs, but if it does, they must NOT
    // say "capture group" — that would be wrong Perl semantics.
    if let Some(val) = hover_value(&result) {
        let lower = val.to_lowercase();
        assert!(
            !lower.contains("capture group"),
            "$0 hover must NOT claim it is a capture group (it is the program name), got: {val}"
        );
    }
    Ok(())
}
