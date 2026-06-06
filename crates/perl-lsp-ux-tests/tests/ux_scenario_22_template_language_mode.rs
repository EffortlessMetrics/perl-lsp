//! Scenario 22 — Template language mode should not poison Perl UX flows.
//!
//! Why this is high-impact:
//! - Mojolicious/TT template files are frequently opened in HTML mode.
//! - Regressions here can flood users with parse noise or break navigation in
//!   neighboring Perl files during the first few minutes of editor usage.
//!
//! Contract:
//! - Opening a template-like file (`*.html.ep`) in non-Perl language mode MUST
//!   not crash.
//! - Template diagnostics in this mode SHOULD stay empty (parse intentionally skipped).
//! - Core navigation in normal Perl files in the same workspace MUST still work.

use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::missing_binary_skip;
use perl_lsp_ux_tests::{ScenarioConfig, UxCiTier, UxComponent, UxHarness, run_ux_scenario};
use std::time::Duration;

const SCENARIO_FILE: &str = "ux_scenario_22_template_language_mode.rs";

const APP_SOURCE: &str = r#"use strict;
use warnings;

sub index {
    return helper();
}

sub helper {
    return 'ok';
}

index();
"#;

const TEMPLATE_SOURCE: &str = r#"% my $user = shift;
<h1><%= $user %></h1>
% if ($user) {
  <p>Welcome!</p>
% }
"#;

#[test]
fn scenario_22_template_in_html_mode_preserves_neighboring_perl_navigation() {
    run_ux_scenario(
        "template_language_mode_resilience",
        SCENARIO_FILE,
        "scenario_22_template_in_html_mode_preserves_neighboring_perl_navigation",
        UxCiTier::Pr,
        Some(UxComponent::GotoDefinition),
        |recorder| {
            if !binary_available() {
                return Err(missing_binary_skip().into());
            }

            let harness = UxHarness::new(
                ScenarioConfig::default()
                    .with_file("app.pl", APP_SOURCE)
                    .with_file("templates/index.html.ep", TEMPLATE_SOURCE),
            )?;

            harness.open_file_with_language_id(
                "templates/index.html.ep",
                TEMPLATE_SOURCE,
                "html",
            )?;
            harness.open_file("app.pl", APP_SOURCE)?;

            std::thread::sleep(Duration::from_millis(500));

            recorder.mark_request_start("template_diagnostics");
            let template_diags = harness
                .wait_for_diagnostics("templates/index.html.ep", Duration::from_millis(1200));
            recorder.mark_first_useful_result("template_diagnostics");
            recorder.check(
                "template opened as html skipped Perl parse diagnostics",
                template_diags.is_empty(),
            )?;

            recorder.mark_request_start("neighboring_perl_definition");
            let defs = harness.definition("app.pl", 4, 11)?;
            if !defs.is_empty() {
                recorder.mark_first_useful_result("neighboring_perl_definition");
            }
            recorder.check(
                "goto-definition in neighboring Perl file stayed functional",
                !defs.is_empty(),
            )?;

            harness.assert_no_crash();
            Ok(())
        },
    );
}
