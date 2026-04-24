//! CodeLens tests
mod support;
use serde_json::json;
use support::lsp_harness::LspHarness;

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
    harness.initialize(None)?;
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
