//! High-impact integration coverage for multi-file navigation user story.
//!
//! This test exercises the full LSP request flow against the in-process harness
//! to ensure cross-file definition/references/symbol lookup work together.

mod support;

use serde_json::json;
use std::time::Duration;
use support::lsp_harness::LspHarness;
use support::test_workspace::TempWorkspace;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
#[serial_test::serial]
fn user_story_multi_file_navigation_end_to_end() -> TestResult {
    let workspace = TempWorkspace::new()?;

    let main_uri = workspace.uri("main.pl");
    workspace.write(
        "main.pl",
        r#"use strict;
use warnings;
use lib 'lib';

use MyApp::Database;

my $db = MyApp::Database->new();
my $user = $db->fetch_user_by_email('test@example.com');
print $user->{name};
"#,
    )?;

    workspace.write(
        "lib/MyApp/Database.pm",
        r#"package MyApp::Database;
use strict;
use warnings;

sub new {
    my ($class) = @_;
    return bless {}, $class;
}

sub fetch_user_by_email {
    my ($self, $email) = @_;
    return { name => $email };
}

1;
"#,
    )?;

    let mut harness = LspHarness::new();
    harness.initialize_ready(&workspace.root_uri, None)?;
    harness.open(&main_uri, &std::fs::read_to_string(workspace.dir.path().join("main.pl"))?)?;
    harness.wait_for_idle(Duration::from_millis(250));

    let definition = harness.request_with_timeout(
        "textDocument/definition",
        json!({
            "textDocument": {"uri": main_uri},
            "position": {"line": 4, "character": 7}
        }),
        Duration::from_secs(2),
    )?;

    let definition_locations =
        definition.as_array().ok_or("expected array result for definition")?;
    let first_definition =
        definition_locations.first().ok_or("expected at least one definition location")?;
    let definition_uri =
        first_definition["uri"].as_str().ok_or("definition uri should be a string")?;
    assert!(
        definition_uri.ends_with("/lib/MyApp/Database.pm"),
        "definition should resolve to Database.pm, got: {definition_uri}"
    );

    let references = harness.request_with_timeout(
        "textDocument/references",
        json!({
            "textDocument": {"uri": main_uri},
            "position": {"line": 6, "character": 25},
            "context": {"includeDeclaration": true}
        }),
        Duration::from_secs(2),
    )?;

    let reference_locations =
        references.as_array().ok_or("expected array result for references")?;
    assert!(
        reference_locations.len() >= 2,
        "expected declaration + call references for fetch_user_by_email, got {}",
        reference_locations.len()
    );

    let symbol_results = harness.request_with_timeout(
        "workspace/symbol",
        json!({"query": "fetch_user_by_email"}),
        Duration::from_secs(2),
    )?;
    let symbols = symbol_results.as_array().ok_or("expected workspace/symbol array response")?;
    assert!(
        symbols.iter().any(|symbol| {
            symbol["name"].as_str() == Some("fetch_user_by_email")
                && symbol["location"]["uri"]
                    .as_str()
                    .unwrap_or_default()
                    .ends_with("/lib/MyApp/Database.pm")
        }),
        "workspace/symbol should include fetch_user_by_email in Database.pm; got: {symbols:?}"
    );

    Ok(())
}
