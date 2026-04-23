//! Scenario 19: BDD coverage for live diagnostics refresh after in-editor fixes.
//!
//! User journey:
//! - Given a file with strict-mode diagnostics.
//! - When the user edits the file to fix the issue.
//! - Then diagnostics should refresh and clear for that file.

use anyhow::{Context, Result};
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use std::time::Duration;

const BROKEN_SOURCE: &str = r#"use strict;
use warnings;

my $value = undef;
print $missing_variable;
"#;

const FIXED_SOURCE: &str = r#"use strict;
use warnings;

my $value = 42;
print $value;
"#;

#[test]
fn scenario_19_diagnostics_refresh_after_fix_uses_did_change() -> Result<()> {
    // Given: a strict Perl file with a missing variable diagnostic.
    let harness = UxHarness::new(ScenarioConfig::default())
        .context("failed to create UX harness for scenario_19")?;
    harness.open_file("live_diagnostics.pl", BROKEN_SOURCE)?;

    let initial_diagnostics =
        harness.wait_for_diagnostics("live_diagnostics.pl", Duration::from_secs(5));
    assert!(
        !initial_diagnostics.is_empty(),
        "expected initial diagnostics for broken source before edit"
    );
    let _ = harness.collect_notifications();

    // When: the user fixes the file contents in-place.
    harness.change_file("live_diagnostics.pl", FIXED_SOURCE)?;

    // Then: diagnostics should refresh and settle to empty for the fixed file.
    let diagnostics_after_fix =
        harness.wait_for_diagnostics("live_diagnostics.pl", Duration::from_secs(6));
    assert!(
        diagnostics_after_fix.is_empty(),
        "expected diagnostics to clear after fix; got: {diagnostics_after_fix:?}"
    );
    harness.assert_no_crash();
    Ok(())
}
