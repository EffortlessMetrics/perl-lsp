// Test infrastructure — allow test-friendly patterns.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Scenario 19 — incremental text-sync UX regression coverage.
//!
//! BDD workflow:
//! - Given an open Perl document with a parse error,
//! - When the user fixes the document and the editor emits didChange,
//! - Then diagnostics should recover and the server should keep serving requests.

use perl_lsp_ux_tests::{LspEvent, ScenarioConfig, UxHarness};
use std::time::{Duration, Instant};

const BROKEN_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
\n\
my $value = ;\n\
print $value;\n\
";

const FIXED_SOURCE: &str = "\
use strict;\n\
use warnings;\n\
\n\
my $value = 42;\n\
print $value;\n\
";

fn binary_available() -> bool {
    perl_lsp_ux_tests::resolve_binary().is_ok()
}

fn has_parse_like_diagnostic(diagnostics: &[serde_json::Value]) -> bool {
    diagnostics.iter().any(|diag| {
        let message = diag
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        message.contains("syntax")
            || message.contains("parse")
            || message.contains("unexpected")
            || message.contains("expected")
    })
}

fn wait_for_any_diagnostics_event(
    harness: &UxHarness,
    timeout: Duration,
) -> Option<Vec<serde_json::Value>> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        for event in harness.peek_notifications() {
            if let LspEvent::Diagnostics { diagnostics, .. } = event {
                return Some(diagnostics);
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    None
}

#[test]
fn scenario_19_didchange_recovers_after_parse_error_fix() {
    if !binary_available() {
        eprintln!("SKIP scenario_19: perl-lsp binary not found");
        return;
    }

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("sync.pl", BROKEN_SOURCE),
    )
    .expect("Failed to create UX harness");

    // Given: user opens a file that initially has parse issues.
    harness.open_file("sync.pl", BROKEN_SOURCE).expect("didOpen should succeed");
    let initial_diagnostics = harness.wait_for_diagnostics("sync.pl", Duration::from_secs(5));
    let saw_parse_issue_before_fix = has_parse_like_diagnostic(&initial_diagnostics);
    if !saw_parse_issue_before_fix {
        eprintln!(
            "INFO scenario_19: initial parse-like diagnostics were not observed; \
             continuing with non-crash recovery checks"
        );
    }

    // Clear previously buffered notifications so post-change assertions inspect fresh server output.
    let _ = harness.collect_notifications();

    // When: user fixes the file and the editor sends didChange full-text sync.
    harness.change_file_full("sync.pl", FIXED_SOURCE).expect("didChange should succeed");

    // Then: diagnostics should settle without parse-like errors and server remains responsive.
    let post_change_diagnostics = harness.wait_for_diagnostics("sync.pl", Duration::from_secs(5));
    assert!(
        !has_parse_like_diagnostic(&post_change_diagnostics),
        "expected parse-like diagnostics to clear after fixing file; got: {:?}",
        post_change_diagnostics
    );

    let hover_result = harness.hover("sync.pl", 4, 2);
    assert!(
        hover_result.is_ok(),
        "hover should stay available after didChange recovery flow: {:?}",
        hover_result
    );

    let diagnostics_event = wait_for_any_diagnostics_event(&harness, Duration::from_secs(1));
    assert!(
        diagnostics_event.is_some(),
        "expected at least one publishDiagnostics event during didChange recovery"
    );

    harness.assert_no_crash();
}
