//! Scenario 19 — Template language mode should not poison Perl UX flows.
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

use anyhow::Result;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use std::time::Duration;

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
fn scenario_19_template_in_html_mode_preserves_neighboring_perl_navigation() -> Result<()> {
    let harness = UxHarness::new(
        ScenarioConfig::default()
            .with_file("app.pl", APP_SOURCE)
            .with_file("templates/index.html.ep", TEMPLATE_SOURCE),
    )?;

    harness.open_file_with_language_id("templates/index.html.ep", TEMPLATE_SOURCE, "html")?;
    harness.open_file("app.pl", APP_SOURCE)?;

    std::thread::sleep(Duration::from_millis(500));

    let template_diags =
        harness.wait_for_diagnostics("templates/index.html.ep", Duration::from_millis(1200));
    assert!(
        template_diags.is_empty(),
        "template opened as html should skip Perl parse diagnostics, got: {template_diags:?}"
    );

    // `helper` call in `return helper();` (line 4, character 11).
    let defs = harness.definition("app.pl", 4, 11)?;
    assert!(
        !defs.is_empty(),
        "expected goto-definition in neighboring Perl file to keep working after template open"
    );

    harness.assert_no_crash();
    Ok(())
}
