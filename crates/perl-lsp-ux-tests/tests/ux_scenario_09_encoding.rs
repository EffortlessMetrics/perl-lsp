// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 09 — BOM and encoding edge cases.
//!
//! Tests UTF-8 BOM, Latin-1 characters, `use utf8` declarations, and
//! Unicode in comments.
//!
//! Acceptance criteria:
//! - Server MUST NOT crash for any of these inputs.
//! - No error-level `window/showMessage` for encoding issues.
//! - Hover and completion MUST NOT crash (empty results OK).

use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};

#[test]
fn scenario_09_utf8_bom_file_does_not_crash() {
    if !binary_available() {
        eprintln!("SKIP scenario_09: perl-lsp binary not found");
        return;
    }

    let bom = "\u{FEFF}";
    let source = format!("{bom}use strict;\nuse warnings;\nmy $x = 1;\n");
    let harness = UxHarness::new(ScenarioConfig::default()).expect("Failed to create UX harness");

    harness.open_file("bom.pl", &source).expect("didOpen should succeed with UTF-8 BOM");

    let hover = harness.hover("bom.pl", 2, 3);
    assert!(hover.is_ok(), "hover crashed on UTF-8 BOM file — UX regression: {:?}", hover);

    harness.assert_no_crash();
}

#[test]
fn scenario_09_unicode_in_strings_does_not_crash() {
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

    let hover = harness.hover("utf8_decl.pl", 4, 3);
    assert!(hover.is_ok(), "hover crashed on use utf8 file — UX regression: {:?}", hover);

    harness.assert_no_crash();
}

#[test]
fn scenario_09_high_codepoint_comments_do_not_crash() {
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

    let hover = harness.hover("smart_quotes.pl", 4, 5);
    assert!(hover.is_ok(), "hover crashed on smart-quote comment — UX regression: {:?}", hover);

    harness.assert_no_crash();
}

#[test]
fn scenario_09_latin1_extended_chars_in_comment() {
    if !binary_available() {
        eprintln!("SKIP scenario_09: perl-lsp binary not found");
        return;
    }

    // Latin-1 supplemental characters (U+00E0..U+00FF) in a comment.
    let source = "use strict;\nuse warnings;\n# café résumé naïve\nmy $x = 1;\n";
    let harness = UxHarness::new(ScenarioConfig::default()).expect("Failed to create UX harness");

    harness.open_file("latin1.pl", source).expect("didOpen should succeed");

    let hover = harness.hover("latin1.pl", 3, 3);
    assert!(hover.is_ok(), "hover crashed on Latin-1 comment — UX regression: {:?}", hover);

    harness.assert_no_crash();
}
