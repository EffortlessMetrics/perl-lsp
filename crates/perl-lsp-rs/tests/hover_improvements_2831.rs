//! Tests for hover documentation improvements — issue #2831 Phase 1.
//!
//! Covers:
//! 1. MetaCPAN link in module-not-found hover
//! 2. Builtin function examples (split, join, push, pop, shift, unshift, map, grep)

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

// ---------------------------------------------------------------------------
// MetaCPAN link — module not found locally
// ---------------------------------------------------------------------------

/// Module hover responses should include a MetaCPAN link so developers can
/// quickly jump to the online docs regardless of local resolution status.
#[test]
fn test_hover_unknown_module_includes_metacpan_link() -> TestResult {
    // Use a module name that is unlikely to resolve to a real file.
    // The module resolver always returns a path (falling back to workspace/lib/)
    // even when the file doesn't exist, so the hover shows the "found" branch.
    // The MetaCPAN link should appear in the hover in all cases.
    let doc = "use Frobnicator::Xyzzy::Does::Not::Exist;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///metacpan_2831.pl", doc)?;
    // Position 4 = 'F' of "Frobnicator..."
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///metacpan_2831.pl"},
                "position": {"line": 0, "character": 4}
            }),
        )
        .unwrap_or(json!(null));
    let val = hover_value(&result).ok_or("Expected hover content for module")?;
    assert!(
        val.contains("metacpan.org"),
        "hover for module should include MetaCPAN link, got: {val}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Builtin examples — split, join, push, pop, shift, unshift, map, grep
// ---------------------------------------------------------------------------

/// Hover on `split` should show a usage example in a Perl code fence.
#[test]
fn test_hover_builtin_split_includes_example() -> TestResult {
    let doc = "my @words = split /\\s+/, $line;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///split_2831.pl", doc)?;
    // 'split' starts at character 12
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///split_2831.pl"},
                "position": {"line": 0, "character": 12}
            }),
        )
        .unwrap_or(json!(null));
    let val = hover_value(&result).ok_or("Expected hover for split")?;
    // The description must contain a Perl code fence (```perl) with a usage example
    assert!(
        val.contains("```perl"),
        "split hover should include a ```perl code fence example, got: {val}"
    );
    Ok(())
}

/// Hover on `push` should show a usage example in a Perl code fence.
#[test]
fn test_hover_builtin_push_includes_example() -> TestResult {
    let doc = "push @list, $item;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///push_2831.pl", doc)?;
    // 'push' starts at character 0
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///push_2831.pl"},
                "position": {"line": 0, "character": 0}
            }),
        )
        .unwrap_or(json!(null));
    let val = hover_value(&result).ok_or("Expected hover for push")?;
    assert!(
        val.contains("```perl"),
        "push hover should include a ```perl code fence example, got: {val}"
    );
    Ok(())
}

/// Hover on `map` should show a usage example in a Perl code fence.
#[test]
fn test_hover_builtin_map_includes_example() -> TestResult {
    let doc = "my @doubled = map { $_ * 2 } @numbers;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///map_2831.pl", doc)?;
    // 'map' starts at character 14
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///map_2831.pl"},
                "position": {"line": 0, "character": 14}
            }),
        )
        .unwrap_or(json!(null));
    let val = hover_value(&result).ok_or("Expected hover for map")?;
    assert!(
        val.contains("```perl"),
        "map hover should include a ```perl code fence example, got: {val}"
    );
    Ok(())
}

/// Hover on `grep` should show a usage example in a Perl code fence.
#[test]
fn test_hover_builtin_grep_includes_example() -> TestResult {
    let doc = "my @evens = grep { $_ % 2 == 0 } @numbers;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///grep_2831.pl", doc)?;
    // 'grep' starts at character 12
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///grep_2831.pl"},
                "position": {"line": 0, "character": 12}
            }),
        )
        .unwrap_or(json!(null));
    let val = hover_value(&result).ok_or("Expected hover for grep")?;
    assert!(
        val.contains("```perl"),
        "grep hover should include a ```perl code fence example, got: {val}"
    );
    Ok(())
}

/// Hover on `join` should show a usage example in a Perl code fence.
#[test]
fn test_hover_builtin_join_includes_example() -> TestResult {
    let doc = "my $str = join(', ', @parts);\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///join_2831.pl", doc)?;
    // 'join' starts at character 10
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///join_2831.pl"},
                "position": {"line": 0, "character": 10}
            }),
        )
        .unwrap_or(json!(null));
    let val = hover_value(&result).ok_or("Expected hover for join")?;
    assert!(
        val.contains("```perl"),
        "join hover should include a ```perl code fence example, got: {val}"
    );
    Ok(())
}

/// Hover on `pop` should show a usage example in a Perl code fence.
#[test]
fn test_hover_builtin_pop_includes_example() -> TestResult {
    let doc = "my $last = pop @stack;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///pop_2831.pl", doc)?;
    // 'pop' starts at character 11
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///pop_2831.pl"},
                "position": {"line": 0, "character": 11}
            }),
        )
        .unwrap_or(json!(null));
    let val = hover_value(&result).ok_or("Expected hover for pop")?;
    assert!(
        val.contains("```perl"),
        "pop hover should include a ```perl code fence example, got: {val}"
    );
    Ok(())
}

/// Hover on `shift` should show a usage example in a Perl code fence.
#[test]
fn test_hover_builtin_shift_includes_example() -> TestResult {
    let doc = "my $first = shift @queue;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///shift_2831.pl", doc)?;
    // 'shift' starts at character 12
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///shift_2831.pl"},
                "position": {"line": 0, "character": 12}
            }),
        )
        .unwrap_or(json!(null));
    let val = hover_value(&result).ok_or("Expected hover for shift")?;
    assert!(
        val.contains("```perl"),
        "shift hover should include a ```perl code fence example, got: {val}"
    );
    Ok(())
}

/// Hover on `unshift` should show a usage example in a Perl code fence.
#[test]
fn test_hover_builtin_unshift_includes_example() -> TestResult {
    let doc = "unshift @list, 0;\n";
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///unshift_2831.pl", doc)?;
    // 'unshift' starts at character 0
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///unshift_2831.pl"},
                "position": {"line": 0, "character": 0}
            }),
        )
        .unwrap_or(json!(null));
    let val = hover_value(&result).ok_or("Expected hover for unshift")?;
    assert!(
        val.contains("```perl"),
        "unshift hover should include a ```perl code fence example, got: {val}"
    );
    Ok(())
}
