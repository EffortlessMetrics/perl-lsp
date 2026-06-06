// Test infrastructure needs skip/status messages when the external binary is absent.
#![allow(clippy::print_stderr)]
// Test assertions intentionally panic with UX-specific failure messages.
#![allow(clippy::panic)]

//! Scenario 07 — Multi-file workspace / cross-file navigation.
//!
//! Sets up a small Perl project (cpanfile, library modules, script).
//! Verifies multi-file open and go-to-definition work or degrade gracefully.
//!
//! Acceptance criteria:
//! - All files open without crashing.
//! - `textDocument/definition` MUST resolve the configured workspace module.
//! - Server remains responsive after workspace indexing.

use anyhow::Result;
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde_json::{Value, json};
use std::time::Duration;

#[test]
fn scenario_07_multi_file_workspace_opens_without_crash() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_07: perl-lsp binary not found");
        return Ok(());
    }

    let module_a = "package MyProject::Utils;\nuse strict;\nuse warnings;\n\n\
                    sub greet { my ($self, $name) = @_; return \"Hello, $name!\"; }\n1;\n";
    let module_b = "package MyProject::Config;\nuse strict;\nuse warnings;\n\n\
                    our $VERSION = '1.0';\nsub get_setting { return 'default'; }\n1;\n";
    let script = "#!/usr/bin/env perl\nuse strict;\nuse warnings;\n\n\
                  use MyProject::Utils;\nuse MyProject::Config;\n\n\
                  my $utils = MyProject::Utils->new();\nprint $utils->greet('World');\n";

    let harness = UxHarness::new(
        ScenarioConfig::default()
            .with_file("lib/MyProject/Utils.pm", module_a)
            .with_file("lib/MyProject/Config.pm", module_b)
            .with_file("script.pl", script)
            .with_file("cpanfile", "requires 'Moo', '2.0';\n"),
    )?;

    harness.open_file("lib/MyProject/Utils.pm", module_a)?;
    harness.open_file("lib/MyProject/Config.pm", module_b)?;
    harness.open_file("script.pl", script)?;

    harness.assert_no_crash();
    Ok(())
}

#[test]
fn scenario_07_definition_request_resolves_workspace_module() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_07: perl-lsp binary not found");
        return Ok(());
    }

    let module = "package Counter;\nuse strict;\nuse warnings;\n\n\
                  sub new { bless {count => 0}, shift }\n\
                  sub increment { $_[0]->{count}++ }\n\
                  sub value { $_[0]->{count} }\n1;\n";
    let script = "use strict;\nuse warnings;\n\nuse Counter;\n\n\
                  my $c = Counter->new();\n$c->increment();\nprint $c->value();\n";

    let harness = UxHarness::new(
        ScenarioConfig { timeout: Duration::from_secs(15), ..Default::default() }
            .with_file("lib/Counter.pm", module)
            .with_file("main.pl", script),
    )?;

    send_include_paths(&harness, &["lib"])?;

    harness.open_file("lib/Counter.pm", module)?;
    harness.open_file("main.pl", script)?;

    let defs = harness.definition_with_retry("main.pl", 3, 4, 5, Duration::from_millis(250))?;
    assert!(
        !defs.is_empty(),
        "expected go-to-definition on `use Counter` to resolve lib/Counter.pm, got empty result"
    );
    assert!(
        defs.iter().all(is_lsp_location_shape),
        "definition entries must be Location or LocationLink values: {defs:?}"
    );
    assert!(
        defs.iter().any(|entry| entry_uri_ends_with(entry, "lib/Counter.pm")),
        "expected at least one definition result to point at lib/Counter.pm, got: {defs:?}"
    );

    harness.assert_no_crash();
    Ok(())
}

fn send_include_paths(harness: &UxHarness, paths: &[&str]) -> Result<()> {
    let paths_json: Vec<Value> = paths.iter().map(|path| json!(*path)).collect();
    harness.client.notify(
        "workspace/didChangeConfiguration",
        json!({
            "settings": {
                "perl": {
                    "workspace": {
                        "includePaths": paths_json,
                        "useSystemInc": false
                    }
                }
            }
        }),
    )?;
    std::thread::sleep(Duration::from_millis(200));
    Ok(())
}

fn is_lsp_location_shape(entry: &Value) -> bool {
    let is_location = entry.get("uri").is_some() && entry.get("range").is_some();
    let is_location_link = entry.get("targetUri").is_some() && entry.get("targetRange").is_some();
    is_location || is_location_link
}

fn entry_uri_ends_with(entry: &Value, suffix: &str) -> bool {
    entry
        .get("uri")
        .or_else(|| entry.get("targetUri"))
        .and_then(Value::as_str)
        .is_some_and(|uri| uri.replace('\\', "/").ends_with(suffix))
}
