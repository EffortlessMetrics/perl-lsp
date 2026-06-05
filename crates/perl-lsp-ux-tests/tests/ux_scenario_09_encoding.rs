// Test infrastructure allows assertion panics and skip-status stderr.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::print_stderr)]

//! Scenario 09 — BOM and encoding edge cases.
//!
//! Tests UTF-8 BOM, Latin-1 characters, `use utf8` declarations, and
//! Unicode in comments.
//!
//! Acceptance criteria:
//! - Server MUST NOT crash for any of these inputs.
//! - No error-level `window/showMessage` for encoding issues.
//! - Hover on symbols in encoded files returns non-empty contents.

use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde_json::Value;

fn hover_contents_text(contents: &Value) -> String {
    if let Some(value) = contents.get("value").and_then(Value::as_str) {
        return value.to_string();
    }

    if let Some(text) = contents.as_str() {
        return text.to_string();
    }

    if let Some(items) = contents.as_array() {
        return items.iter().map(hover_contents_text).collect::<Vec<_>>().join("\n");
    }

    String::new()
}

fn assert_hover_has_non_empty_contents(harness: &UxHarness, path: &str, line: u32, character: u32) {
    let hover = harness
        .hover(path, line, character)
        .unwrap_or_else(|error| panic!("hover must not error for encoded file {path}: {error:?}"));
    let hover =
        hover.unwrap_or_else(|| panic!("hover must not return null for encoded file {path}"));
    let contents = hover
        .get("contents")
        .unwrap_or_else(|| panic!("hover for encoded file {path} missing contents: {hover:?}"));
    assert!(
        !hover_contents_text(contents).trim().is_empty(),
        "hover for encoded file {path} must return non-empty contents; got {contents:?}"
    );
}

#[test]
fn scenario_09_utf8_bom_hover_returns_contents() {
    if !binary_available() {
        eprintln!("SKIP scenario_09: perl-lsp binary not found");
        return;
    }

    let bom = "\u{FEFF}";
    let source = format!("{bom}use strict;\nuse warnings;\nmy $x = 1;\n");
    let harness = UxHarness::new(ScenarioConfig::default()).expect("Failed to create UX harness");

    harness.open_file("bom.pl", &source).expect("didOpen should succeed with UTF-8 BOM");

    assert_hover_has_non_empty_contents(&harness, "bom.pl", 2, 3);

    harness.assert_no_crash();
}

#[test]
fn scenario_09_unicode_string_hover_returns_contents() {
    if !binary_available() {
        eprintln!("SKIP scenario_09: perl-lsp binary not found");
        return;
    }

    // Unicode characters in string literals (valid UTF-8, common in i18n Perl code).
    let source = "use utf8;\nuse strict;\nuse warnings;\n\n\
                  my $name = \"\u{4E16}\u{754C}\";\n\
                  print \"Hello, $name\\n\";\n";
    let harness = UxHarness::new(ScenarioConfig::default()).expect("Failed to create UX harness");

    harness.open_file("utf8_decl.pl", source).expect("didOpen should succeed with use utf8");

    assert_hover_has_non_empty_contents(&harness, "utf8_decl.pl", 4, 3);

    harness.assert_no_crash();
}

#[test]
fn scenario_09_high_codepoint_comment_hover_returns_contents() {
    if !binary_available() {
        eprintln!("SKIP scenario_09: perl-lsp binary not found");
        return;
    }

    // Em-dashes and smart-quotes in comments — common in legacy Perl codebases.
    let source = "#!/usr/bin/perl\n\
                  # This is a comment with \u{2014} an em-dash\n\
                  # And \u{201C}smart quotes\u{201D}\n\
                  use strict;\n\
                  my $x = 1; # \u{2013} some note\n";
    let harness = UxHarness::new(ScenarioConfig::default()).expect("Failed to create UX harness");

    harness.open_file("smart_quotes.pl", source).expect("didOpen should succeed");

    assert_hover_has_non_empty_contents(&harness, "smart_quotes.pl", 4, 5);

    harness.assert_no_crash();
}

#[test]
fn scenario_09_latin1_comment_hover_returns_contents() {
    if !binary_available() {
        eprintln!("SKIP scenario_09: perl-lsp binary not found");
        return;
    }

    // Latin-1 supplemental characters (U+00E0..U+00FF) in a comment.
    let source = "use strict;\nuse warnings;\n# café résumé naïve\nmy $x = 1;\n";
    let harness = UxHarness::new(ScenarioConfig::default()).expect("Failed to create UX harness");

    harness.open_file("latin1.pl", source).expect("didOpen should succeed");

    assert_hover_has_non_empty_contents(&harness, "latin1.pl", 3, 3);

    harness.assert_no_crash();
}
