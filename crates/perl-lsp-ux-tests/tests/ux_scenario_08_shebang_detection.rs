// Test infrastructure needs skip/status messages when the external binary is absent.
#![allow(clippy::print_stderr)]
// Test assertions intentionally panic with UX-specific failure messages.
#![allow(clippy::panic)]

//! Scenario 08 — Shebang detection / non-standard extensions.
//!
//! Files with `#!/usr/bin/env perl` shebang but no `.pl`/`.pm` extension.
//!
//! Acceptance criteria:
//! - Server MUST accept `didOpen` with any URI when languageId is "perl".
//! - Shebang files without extensions MUST expose Perl document symbols.
//! - Hover and completion MUST NOT crash.

use anyhow::Result;
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde_json::Value;
use std::time::Duration;

const SHEBANG_DEPLOY_SOURCE: &str = r#"#!/usr/bin/env perl
use strict;
use warnings;

sub deploy_task {
    return 42;
}

deploy_task();
"#;

#[test]
fn scenario_08_shebang_file_without_pl_extension_has_document_symbols() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_08: perl-lsp binary not found");
        return Ok(());
    }

    let harness = UxHarness::new(
        ScenarioConfig::default().with_file("deploy_script", SHEBANG_DEPLOY_SOURCE),
    )?;

    harness.open_file("deploy_script", SHEBANG_DEPLOY_SOURCE)?;

    std::thread::sleep(Duration::from_millis(300));

    let symbols = harness.document_symbols("deploy_script")?;
    assert!(
        symbols.iter().any(|symbol| symbol_name_matches(symbol, "deploy_task")),
        "expected no-extension shebang file to expose deploy_task document symbol, got: {symbols:?}"
    );

    harness.hover("deploy_script", 8, 0)?;

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_08_no_extension_file_completion_does_not_crash() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_08: perl-lsp binary not found");
        return Ok(());
    }

    let source = "#!/usr/bin/perl\nmy $va\n";
    let harness = UxHarness::new(ScenarioConfig::default())?;

    harness.open_file("run_tests", source)?;

    harness.completion("run_tests", 1, 7)?;
    Ok(())
}

#[test]
fn scenario_08_test_file_t_extension() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_08: perl-lsp binary not found");
        return Ok(());
    }

    let source = "use Test::More;\nuse strict;\n\nok(1, 'basic');\ndone_testing();\n";
    let harness = UxHarness::new(ScenarioConfig::default())?;

    harness.open_file("basic.t", source)?;

    harness.hover("basic.t", 3, 1)?;
    Ok(())
}

fn symbol_name_matches(symbol: &Value, expected: &str) -> bool {
    symbol.get("name").and_then(Value::as_str) == Some(expected)
}
