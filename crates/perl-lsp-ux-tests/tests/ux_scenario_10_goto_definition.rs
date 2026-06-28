//! Scenario 10 — Go-to-definition UX workflow coverage.
//!
//! Focus area: `textDocument/definition` behavior for first-editing-session UX.
//! This suite uses a compact BDD-style helper so each test describes intent in
//! Given/When/Then language and avoids duplicated harness boilerplate.

use anyhow::Result;
use perl_lsp_ux_tests::binary_available;
use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
use serde_json::Value;
use std::time::Duration;

const SAME_FILE_FIXTURE: &str = r#"use strict;
use warnings;

sub greet {
    my ($name) = @_;
    return "Hello, $name!";
}

greet('World');
"#;

const CROSS_FILE_MODULE: &str = r#"package Counter;
use strict;
use warnings;

sub increment {
    my ($class, $n) = @_;
    return $n + 1;
}

1;
"#;

const CROSS_FILE_SCRIPT: &str = r#"use strict;
use warnings;
use lib 'lib';
use Counter;

my $value = Counter->increment(3);
print "$value\n";
"#;

struct DefinitionScenario {
    harness: UxHarness,
}

impl DefinitionScenario {
    fn single_file() -> Result<Self> {
        let harness = UxHarness::new(
            ScenarioConfig::default()
                .with_file("greet.pl", SAME_FILE_FIXTURE)
                .with_file("lib/Counter.pm", CROSS_FILE_MODULE)
                .with_file("main.pl", CROSS_FILE_SCRIPT),
        )?;
        Ok(Self { harness })
    }

    fn given_file_is_open(&self, path: &str, content: &str) -> Result<()> {
        self.harness.open_file(path, content)
    }

    fn when_requesting_definition_with_retry(
        &self,
        path: &str,
        line: u32,
        character: u32,
    ) -> Result<Vec<Value>> {
        self.harness.definition_with_retry(path, line, character, 5, Duration::from_millis(200))
    }

    fn then_no_crash_signals_exist(&self) {
        self.harness.assert_no_crash();
    }
}

fn is_lsp_location_shape(entry: &Value) -> bool {
    let is_location = entry.get("uri").is_some() && entry.get("range").is_some();
    let is_location_link = entry.get("targetUri").is_some() && entry.get("targetRange").is_some();
    is_location || is_location_link
}

fn entry_uri(entry: &Value) -> Option<&str> {
    entry.get("uri").or_else(|| entry.get("targetUri")).and_then(Value::as_str)
}

#[test]
fn scenario_10_definition_same_file_call_site_resolves() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_10: perl-lsp binary not found");
        return Ok(());
    }

    let scenario = DefinitionScenario::single_file()?;

    // Given a script with an in-file sub definition and call site.
    scenario.given_file_is_open("greet.pl", SAME_FILE_FIXTURE)?;

    // When go-to-definition is requested on `greet('World')`.
    let definitions = scenario.when_requesting_definition_with_retry("greet.pl", 8, 0)?;

    // Then the server must return at least one result — the `sub greet`
    // declaration is literally six lines above the call site. An empty list
    // after retry indicates goto-definition is broken, not degraded UX.
    assert!(
        !definitions.is_empty(),
        "expected at least one definition location for same-file `greet()` call site \
         (sub defined on line 3) but got empty list after retries"
    );
    for entry in &definitions {
        assert!(
            is_lsp_location_shape(entry),
            "definition entry must be a Location or LocationLink: {entry:?}"
        );
    }
    // And at least one location must point back to greet.pl — otherwise the
    // server resolved the call to some unrelated file (real regression).
    let points_to_source = definitions
        .iter()
        .any(|entry| entry_uri(entry).map(|uri| uri.ends_with("greet.pl")).unwrap_or(false));
    assert!(
        points_to_source,
        "expected at least one definition result to point back to greet.pl, got: {definitions:?}"
    );

    scenario.then_no_crash_signals_exist();
    Ok(())
}

#[test]
fn scenario_10_definition_cross_file_module_symbol_points_to_module() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_10: perl-lsp binary not found");
        return Ok(());
    }

    let scenario = DefinitionScenario::single_file()?;

    // Given a module and script opened in the same workspace.
    scenario.given_file_is_open("lib/Counter.pm", CROSS_FILE_MODULE)?;
    scenario.given_file_is_open("main.pl", CROSS_FILE_SCRIPT)?;

    // When go-to-definition is requested on `increment` in `Counter->increment`.
    let definitions = scenario.when_requesting_definition_with_retry("main.pl", 5, 23)?;

    // Then results are shape-valid. Cross-file module resolution may degrade
    // if `use lib` handling is incomplete; keep the empty path tolerated but
    // log it so we notice when it happens in CI.
    for entry in &definitions {
        assert!(
            is_lsp_location_shape(entry),
            "definition entry must be a Location or LocationLink: {entry:?}"
        );
    }

    if definitions.is_empty() {
        eprintln!(
            "INFO scenario_10: cross-file definition returned empty — tolerated \
             degraded path (cross-file indexing may not have settled)"
        );
    } else {
        let points_to_module = definitions
            .iter()
            .any(|entry| entry_uri(entry).map(|uri| uri.ends_with("Counter.pm")).unwrap_or(false));
        assert!(
            points_to_module,
            "non-empty cross-file definition results must include Counter.pm but did not: \
             {definitions:?}"
        );
        // And they must never resolve to an unrelated file (e.g. leaking the
        // call-site file as the definition would be a real regression).
        let resolves_outside_workspace = definitions.iter().any(|entry| {
            entry_uri(entry)
                .map(|uri| !uri.ends_with("Counter.pm") && !uri.ends_with("main.pl"))
                .unwrap_or(false)
        });
        assert!(
            !resolves_outside_workspace,
            "cross-file definition resolved to an unrelated file: {definitions:?}"
        );
    }

    scenario.then_no_crash_signals_exist();
    Ok(())
}

#[test]
fn scenario_10_definition_unknown_position_is_stable() -> Result<()> {
    if !binary_available() {
        eprintln!("SKIP scenario_10: perl-lsp binary not found");
        return Ok(());
    }

    let scenario = DefinitionScenario::single_file()?;

    // Given a tiny script with no resolvable symbol at the cursor position.
    let unknown_fixture = "use strict;\nmy $x = 1;\n";
    scenario.given_file_is_open("unknown.pl", unknown_fixture)?;

    // When go-to-definition is requested over `strict`.
    let definitions = scenario.when_requesting_definition_with_retry("unknown.pl", 0, 5)?;

    // Then the response is either empty or contains shape-valid locations.
    for entry in &definitions {
        assert!(
            is_lsp_location_shape(entry),
            "definition entry must be a Location or LocationLink: {entry:?}"
        );
    }

    scenario.then_no_crash_signals_exist();
    Ok(())
}
