//! CodeLens tests
mod support;
use serde_json::json;
use support::lsp_harness::LspHarness;

fn code_lens_resolve_capabilities(properties: &[&str]) -> serde_json::Value {
    json!({
        "textDocument": {
            "codeLens": {
                "resolveSupport": {
                    "properties": properties
                }
            }
        }
    })
}

fn code_lens_command_id(lens: &serde_json::Value) -> Option<&str> {
    lens.pointer("/command/command").and_then(serde_json::Value::as_str)
}

fn code_lens_command_tooltip(lens: &serde_json::Value) -> Option<&str> {
    lens.pointer("/command/tooltip").and_then(serde_json::Value::as_str)
}

fn has_unresolved_reference_lens(lens: &serde_json::Value) -> bool {
    lens.get("command").is_none()
        && lens.pointer("/data/kind").and_then(serde_json::Value::as_str).is_some()
}

fn command_tooltip_for<'a>(lenses: &'a [serde_json::Value], command: &str) -> Option<&'a str> {
    lenses
        .iter()
        .find(|lens| code_lens_command_id(lens) == Some(command))
        .and_then(code_lens_command_tooltip)
}

#[test]

fn test_shows_codelens_on_sub() -> Result<(), Box<dyn std::error::Error>> {
    let doc = r#"
sub add {
    my ($x, $y) = @_;
    return $x + $y;
}

my $z = add(1, 2);
"#;
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///test.pl", doc)?;
    let uri = "file:///test.pl";

    let result = harness
        .request(
            "textDocument/codeLens",
            json!({
                "textDocument": {"uri": uri}
            }),
        )
        .unwrap_or(json!(null));

    if let Some(lenses) = result.as_array() {
        assert!(!lenses.is_empty(), "Should have at least one code lens");

        // Check that at least one lens is for references
        let has_ref_lens = lenses.iter().any(|lens| {
            lens.get("data").is_some()
                || lens
                    .get("command")
                    .and_then(|c| c.get("title"))
                    .and_then(|t| t.as_str())
                    .map(|t| t.contains("ref"))
                    .unwrap_or(false)
        });

        assert!(has_ref_lens, "Should have a reference code lens");
    }

    Ok(())
}

#[test]

fn test_test_subroutine_gets_run_lens() -> Result<(), Box<dyn std::error::Error>> {
    let doc = r#"
sub test_addition {
    my $result = add(2, 3);
    is($result, 5, "2 + 3 = 5");
}

sub add {
    my ($x, $y) = @_;
    return $x + $y;
}
"#;
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///test.pl", doc)?;
    let uri = "file:///test.pl";

    let result = harness
        .request(
            "textDocument/codeLens",
            json!({
                "textDocument": {"uri": uri}
            }),
        )
        .unwrap_or(json!(null));

    if let Some(lenses) = result.as_array() {
        // Check for Run Test lens
        let has_run_test = lenses.iter().any(|lens| {
            lens.get("command")
                .and_then(|c| c.get("title"))
                .and_then(|t| t.as_str())
                .map(|t| t.contains("Run Test"))
                .unwrap_or(false)
        });

        assert!(has_run_test, "Test subroutine should have a 'Run Test' code lens");
    }

    Ok(())
}

#[test]

fn test_package_gets_references_lens() -> Result<(), Box<dyn std::error::Error>> {
    let doc = r#"
package MyModule;

sub new {
    my $class = shift;
    return bless {}, $class;
}

1;
"#;
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///test.pl", doc)?;
    let uri = "file:///test.pl";

    let result = harness
        .request(
            "textDocument/codeLens",
            json!({
                "textDocument": {"uri": uri}
            }),
        )
        .unwrap_or(json!(null));

    if let Some(lenses) = result.as_array() {
        // Should have lenses for both package and sub
        assert!(lenses.len() >= 2, "Should have code lenses for package and subroutine");
    }

    Ok(())
}

#[test]

fn test_codelens_resolve() -> Result<(), Box<dyn std::error::Error>> {
    let doc = r#"
sub helper {
    return 42;
}

my $x = helper();
my $y = helper();
"#;
    let mut harness = LspHarness::new();
    harness.initialize(Some(code_lens_resolve_capabilities(&["command"])))?;
    harness.open_document("file:///test.pl", doc)?;
    let uri = "file:///test.pl";

    // First get the code lenses
    let lenses_result = harness
        .request(
            "textDocument/codeLens",
            json!({
                "textDocument": {"uri": uri}
            }),
        )
        .unwrap_or(json!(null));

    if let Some(lenses) = lenses_result.as_array() {
        // Find a lens with data (unresolved references lens)
        if let Some(unresolved_lens) = lenses.iter().find(|l| l.get("data").is_some()) {
            // Try to resolve it
            let resolved =
                harness.request("codeLens/resolve", unresolved_lens.clone()).unwrap_or(json!(null));

            // After resolution, it should have a command
            assert!(resolved.get("command").is_some(), "Resolved lens should have a command");

            if let Some(command) = resolved.get("command") {
                let title = command.get("title").and_then(|t| t.as_str()).unwrap_or("");
                assert!(title.contains("ref"), "Command title should mention references");
            }
        }
    }

    Ok(())
}

#[test]
fn test_codelens_eager_without_resolve_support() -> Result<(), Box<dyn std::error::Error>> {
    let doc = r#"
sub helper {
    return 42;
}

my $x = helper();
my $y = helper();
"#;
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///test.pl", doc)?;

    let result = harness.request(
        "textDocument/codeLens",
        json!({
            "textDocument": {"uri": "file:///test.pl"}
        }),
    )?;
    let lenses = result.as_array().ok_or("Expected codeLens result array")?;

    assert!(
        !lenses.iter().any(has_unresolved_reference_lens),
        "clients without codeLens.resolveSupport.command must receive eager command lenses; got {lenses:?}"
    );
    assert!(
        lenses
            .iter()
            .any(|lens| code_lens_command_id(lens) == Some("editor.action.findReferences")),
        "expected eager findReferences command lens; got {lenses:?}"
    );

    Ok(())
}

#[test]
fn test_codelens_defers_command_when_resolve_support_allows_command()
-> Result<(), Box<dyn std::error::Error>> {
    let doc = r#"
sub helper {
    return 42;
}

my $x = helper();
my $y = helper();
"#;
    let mut harness = LspHarness::new();
    harness.initialize(Some(code_lens_resolve_capabilities(&["command"])))?;
    harness.open_document("file:///test.pl", doc)?;

    let result = harness.request(
        "textDocument/codeLens",
        json!({
            "textDocument": {"uri": "file:///test.pl"}
        }),
    )?;
    let lenses = result.as_array().ok_or("Expected codeLens result array")?;

    assert!(
        lenses.iter().any(has_unresolved_reference_lens),
        "clients that support resolving command may receive unresolved reference lenses; got {lenses:?}"
    );

    Ok(())
}

#[test]
fn test_codelens_eager_when_resolve_support_lacks_command() -> Result<(), Box<dyn std::error::Error>>
{
    let doc = r#"
sub helper {
    return 42;
}

my $x = helper();
my $y = helper();
"#;
    let mut harness = LspHarness::new();
    harness.initialize(Some(code_lens_resolve_capabilities(&["tooltip"])))?;
    harness.open_document("file:///test.pl", doc)?;

    let result = harness.request(
        "textDocument/codeLens",
        json!({
            "textDocument": {"uri": "file:///test.pl"}
        }),
    )?;
    let lenses = result.as_array().ok_or("Expected codeLens result array")?;

    assert!(
        !lenses.iter().any(has_unresolved_reference_lens),
        "clients that do not list command in resolveSupport must not receive unresolved command lenses; got {lenses:?}"
    );
    assert!(
        lenses
            .iter()
            .any(|lens| code_lens_command_id(lens) == Some("editor.action.findReferences")),
        "expected eager findReferences command lens; got {lenses:?}"
    );

    Ok(())
}

#[test]
fn test_codelens_commands_include_lsp_318_tooltips() -> Result<(), Box<dyn std::error::Error>> {
    let doc = r#"#!/usr/bin/env perl
use Test::More;

sub test_addition {
    ok(1, "addition");
}

sub helper {
    return 42;
}
"#;
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///tooltip.t", doc)?;

    let result = harness.request(
        "textDocument/codeLens",
        json!({
            "textDocument": {"uri": "file:///tooltip.t"}
        }),
    )?;
    let lenses = result.as_array().ok_or("Expected codeLens result array")?;

    assert_eq!(
        command_tooltip_for(lenses, "perl.runScript"),
        Some("Run this Perl script"),
        "run-script CodeLens command should carry a plain LSP 3.18 tooltip: {lenses:?}"
    );
    assert_eq!(
        command_tooltip_for(lenses, "perl.runTestFile"),
        Some("Run all Perl tests in this file"),
        "run-all-tests CodeLens command should carry a plain LSP 3.18 tooltip: {lenses:?}"
    );
    assert_eq!(
        command_tooltip_for(lenses, "perl.runTest"),
        Some("Run Perl test subroutine test_addition"),
        "run-test CodeLens command should carry a plain LSP 3.18 tooltip: {lenses:?}"
    );
    assert_eq!(
        command_tooltip_for(lenses, "perl.debugTest"),
        Some("Debug Perl test subroutine test_addition"),
        "debug-test CodeLens command should carry a plain LSP 3.18 tooltip: {lenses:?}"
    );
    assert_eq!(
        command_tooltip_for(lenses, "editor.action.findReferences"),
        Some("Show references for this Perl symbol"),
        "eager reference CodeLens command should carry a plain LSP 3.18 tooltip: {lenses:?}"
    );

    Ok(())
}

#[test]
fn test_codelens_resolve_adds_lsp_318_command_tooltip() -> Result<(), Box<dyn std::error::Error>> {
    let doc = r#"
sub helper {
    return 42;
}

my $x = helper();
my $y = helper();
"#;
    let mut harness = LspHarness::new();
    harness.initialize(Some(code_lens_resolve_capabilities(&["command"])))?;
    harness.open_document("file:///tooltip-resolve.pl", doc)?;

    let result = harness.request(
        "textDocument/codeLens",
        json!({
            "textDocument": {"uri": "file:///tooltip-resolve.pl"}
        }),
    )?;
    let lenses = result.as_array().ok_or("Expected codeLens result array")?;
    let unresolved = lenses
        .iter()
        .find(|lens| {
            lens.pointer("/data/name").and_then(serde_json::Value::as_str) == Some("helper")
        })
        .ok_or("expected unresolved helper CodeLens")?;

    let resolved = harness.request("codeLens/resolve", unresolved.clone())?;

    assert_eq!(code_lens_command_id(&resolved), Some("editor.action.findReferences"));
    assert_eq!(
        code_lens_command_tooltip(&resolved),
        Some("Show references for this Perl symbol"),
        "resolved CodeLens command should carry a plain LSP 3.18 tooltip: {resolved}"
    );

    Ok(())
}
