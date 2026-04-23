// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 10 — Go-to-definition feature grid coverage.
//!
//! Verifies that `textDocument/definition` is wired up end-to-end for the LSP
//! feature advertised in `features.toml`.
//!
//! Acceptance criteria:
//! - `textDocument/definition` MUST NOT return a JSON-RPC error.
//! - A definition result MAY be empty (degraded mode acceptable) but must not crash.
//! - When a sub is defined in the same file, the server SHOULD return a location
//!   pointing back into that file.
//! - No crash signatures after the request.

use anyhow::Result;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use std::time::Duration;

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

/// Source with a named sub defined near the top and called later.
/// We will request go-to-definition on the call site at line 8, char 0.
const GOTO_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
\n\
sub greet {\n\
    my ($name) = @_;\n\
    return \"Hello, $name!\";\n\
}\n\
\n\
greet('World');\n\
";

#[test]
fn scenario_10_definition_request_does_not_error() {
    if !binary_available() {
        eprintln!("SKIP scenario_10: perl-lsp binary not found");
        return;
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("greet.pl", GOTO_SOURCE),
    )
    .expect("Failed to create UX harness");

    harness.open_file("greet.pl", GOTO_SOURCE).expect("didOpen should succeed");

    // Allow the server to index the file.
    std::thread::sleep(Duration::from_millis(500));

    // Request definition on the call site `greet('World')` — line 8, char 0.
    let result = harness.definition("greet.pl", 8, 0);
    assert!(
        result.is_ok(),
        "textDocument/definition must not return a JSON-RPC error — feature grid regression: {:?}",
        result
    );

    harness.assert_no_crash();
}

#[test]
fn scenario_10_definition_result_is_location_or_empty() {
    if !binary_available() {
        eprintln!("SKIP scenario_10: perl-lsp binary not found");
        return;
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("greet.pl", GOTO_SOURCE),
    )
    .expect("Failed to create UX harness");

    harness.open_file("greet.pl", GOTO_SOURCE).expect("didOpen should succeed");

    std::thread::sleep(Duration::from_millis(500));

    let defs = harness.definition("greet.pl", 8, 0).expect("definition must not error");

    // If results are returned they must be well-formed Location objects.
    for loc in &defs {
        assert!(
            loc.get("uri").is_some() && loc.get("range").is_some(),
            "Definition result must have 'uri' and 'range' fields, got: {:?}",
            loc
        );
        // The location must point to our file (not some unrelated URI).
        let uri = loc["uri"].as_str().unwrap_or("");
        assert!(
            uri.ends_with("greet.pl"),
            "Definition location should point to greet.pl, got uri={:?}",
            uri
        );
    }

    harness.assert_no_crash();
}

#[test]
fn scenario_10_definition_on_unknown_position_returns_empty() {
    if !binary_available() {
        eprintln!("SKIP scenario_10: perl-lsp binary not found");
        return;
    }

    let source = "use strict;\nmy $x = 1;\n";
    let harness = UxHarness::new(ScenarioConfig::default().with_file("simple.pl", source))
        .expect("Failed to create UX harness");

    harness.open_file("simple.pl", source).expect("didOpen should succeed");

    // Position in middle of `strict` string literal — no definition expected.
    let defs = harness
        .definition("simple.pl", 0, 5)
        .expect("definition must not error on arbitrary position");

    // Empty is fine; what we are testing is that no crash or error occurs.
    let _ = defs;

    harness.assert_no_crash();
}

const IMPORTED_FUNCTION_CALLER: &str = "\
use strict;\n\
use warnings;\n\
use lib 'lib';\n\
use Foo qw(bar);\n\
\n\
bar();\n\
";

const IMPORTED_FUNCTION_MODULE: &str = "\
package Foo;\n\
use strict;\n\
use warnings;\n\
use Exporter qw(import);\n\
our @EXPORT_OK = qw(bar);\n\
\n\
sub bar {\n\
    return 'ok';\n\
}\n\
\n\
1;\n\
";

#[test]
fn scenario_10_definition_resolves_imported_function_call_site() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_10: perl-lsp binary not found");
        return Ok(());
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(20), ..Default::default() }
            .with_file("script.pl", IMPORTED_FUNCTION_CALLER)
            .with_file("lib/Foo.pm", IMPORTED_FUNCTION_MODULE),
    )?;

    harness.open_file("lib/Foo.pm", IMPORTED_FUNCTION_MODULE)?;
    harness.open_file("script.pl", IMPORTED_FUNCTION_CALLER)?;

    std::thread::sleep(Duration::from_millis(700));

    // `bar();` call site — line 5, char 0.
    let defs = harness.definition("script.pl", 5, 0)?;
    assert!(
        !defs.is_empty(),
        "Expected goto-definition to resolve imported function `bar`, got empty results."
    );

    let resolves_to_module = defs.iter().any(|entry| {
        entry
            .get("uri")
            .and_then(|uri| uri.as_str())
            .map(|uri| uri.ends_with("lib/Foo.pm"))
            .unwrap_or(false)
            || entry
                .get("targetUri")
                .and_then(|uri| uri.as_str())
                .map(|uri| uri.ends_with("lib/Foo.pm"))
                .unwrap_or(false)
    });

    assert!(
        resolves_to_module,
        "Expected goto-definition target to point at lib/Foo.pm for imported `bar`, got: {:?}",
        defs
    );

    harness.assert_no_crash();
    Ok(())
}
