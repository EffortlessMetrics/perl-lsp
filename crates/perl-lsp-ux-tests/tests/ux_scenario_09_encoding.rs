// Test infrastructure needs skip/status messages when the external binary is absent.
#![allow(clippy::print_stderr)]
// Test assertions intentionally panic with UX-specific failure messages.
#![allow(clippy::panic)]

//! Scenario 09 — BOM and encoding edge cases.
//!
//! Tests UTF-8 BOM, Latin-1 characters, `use utf8` declarations, and
//! Unicode in comments.
//!
//! Acceptance criteria:
//! - Server MUST NOT crash for any of these inputs.
//! - No error-level `window/showMessage` for encoding issues.
//! - UTF-8 BOM files MUST still expose normal Perl document symbols.
//! - Hover MUST NOT crash.

use anyhow::Result;
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde_json::Value;
use std::time::Duration;

const BOM_SYMBOL_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
\n\
sub decode_payload {\n\
    return 1;\n\
}\n\
\n\
decode_payload();\n\
";

#[test]
fn scenario_09_utf8_bom_file_preserves_document_symbols() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_09: perl-lsp binary not found");
        return Ok(());
    }

    let source = format!("\u{FEFF}{BOM_SYMBOL_SOURCE}");
    let harness = UxHarness::new(ScenarioConfig::default().with_file("bom.pl", &source))?;

    harness.open_file("bom.pl", &source)?;

    std::thread::sleep(Duration::from_millis(300));

    let symbols = harness.document_symbols("bom.pl")?;
    assert!(
        symbols.iter().any(|symbol| symbol_name_matches(symbol, "decode_payload")),
        "expected UTF-8 BOM file to expose decode_payload document symbol, got: {symbols:?}"
    );

    harness.hover("bom.pl", 7, 0)?;

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_09_unicode_in_strings_does_not_crash() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_09: perl-lsp binary not found");
        return Ok(());
    }

    // Unicode characters in string literals (valid UTF-8, common in i18n Perl code).
    let source = "use utf8;\nuse strict;\nuse warnings;\n\n\
                  my $name = \"\u{4E16}\u{754C}\";\n\
                  print \"Hello, $name\\n\";\n";
    let harness = UxHarness::new(ScenarioConfig::default())?;

    harness.open_file("utf8_decl.pl", source)?;

    harness.hover("utf8_decl.pl", 4, 3)?;

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_09_high_codepoint_comments_do_not_crash() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_09: perl-lsp binary not found");
        return Ok(());
    }

    // Em-dashes and smart-quotes in comments — common in legacy Perl codebases.
    let source = "#!/usr/bin/perl\n\
                  # This is a comment with \u{2014} an em-dash\n\
                  # And \u{201C}smart quotes\u{201D}\n\
                  use strict;\n\
                  my $x = 1; # \u{2013} some note\n";
    let harness = UxHarness::new(ScenarioConfig::default())?;

    harness.open_file("smart_quotes.pl", source)?;

    harness.hover("smart_quotes.pl", 4, 5)?;

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_09_latin1_extended_chars_in_comment() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_09: perl-lsp binary not found");
        return Ok(());
    }

    // Latin-1 supplemental characters (U+00E0..U+00FF) in a comment.
    let source = "use strict;\nuse warnings;\n# café résumé naïve\nmy $x = 1;\n";
    let harness = UxHarness::new(ScenarioConfig::default())?;

    harness.open_file("latin1.pl", source)?;

    harness.hover("latin1.pl", 3, 3)?;

    harness.assert_no_crash();
    Ok(())
}

fn symbol_name_matches(symbol: &Value, expected: &str) -> bool {
    symbol.get("name").and_then(Value::as_str) == Some(expected)
}
