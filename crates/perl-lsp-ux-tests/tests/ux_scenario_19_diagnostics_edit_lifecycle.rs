//! Scenario 19 — BDD diagnostics lifecycle under live editing.
//!
//! User journey:
//! - Given a file with a syntax error, diagnostics should appear.
//! - When the user fixes the file via `didChange`, diagnostics should clear.
//! - Then the server remains responsive for follow-up hover requests.

use anyhow::{Result, anyhow};
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use std::time::Duration;

const BROKEN_SOURCE: &str = "my $value = ;\nprint $value;\n";
const FIXED_SOURCE: &str = "my $value = 42;\nprint $value;\n";

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

#[test]
fn scenario_19_given_syntax_error_when_fixed_then_diagnostics_clear_and_hover_still_works()
-> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_19: perl-lsp binary not found");
        return Ok(());
    }

    let harness =
        UxHarness::new(ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() })?;

    harness.open_file("edit_cycle.pl", BROKEN_SOURCE)?;

    let initial_diagnostics = harness
        .wait_for_diagnostics_minimum("edit_cycle.pl", 1, Duration::from_secs(8))
        .ok_or_else(|| anyhow!("expected at least one diagnostic for broken source"))?;

    assert!(
        initial_diagnostics
            .iter()
            .all(|diag| { diag.get("range").is_some() && diag.get("message").is_some() }),
        "diagnostics must have LSP range/message shape"
    );

    harness.replace_file("edit_cycle.pl", FIXED_SOURCE, 2)?;

    let deadline = std::time::Instant::now() + Duration::from_secs(8);
    let mut syntax_error_cleared = false;
    while std::time::Instant::now() < deadline {
        if let Some(diagnostics) = harness.latest_diagnostics("edit_cycle.pl") {
            let contains_syntax_error = diagnostics.iter().any(|diag| {
                diag.get("message")
                    .and_then(|m| m.as_str())
                    .map(|m| {
                        let lower = m.to_ascii_lowercase();
                        lower.contains("syntax") || lower.contains("parse")
                    })
                    .unwrap_or(false)
            });
            if !contains_syntax_error {
                syntax_error_cleared = true;
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(syntax_error_cleared, "expected syntax diagnostics to clear after fixing source");

    let _hover = harness.hover("edit_cycle.pl", 1, 7)?;

    harness.assert_no_crash();
    Ok(())
}
