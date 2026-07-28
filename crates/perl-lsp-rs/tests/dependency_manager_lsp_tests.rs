//! Request-level coverage for marker-based dependency-manager include roots.

mod support;

use serde_json::{Value, json};
use support::lsp_harness::{LspHarness, TempWorkspace};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn labels(result: &Value) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let items = result
        .as_array()
        .or_else(|| result.get("items").and_then(Value::as_array))
        .ok_or_else(|| std::io::Error::other(format!("expected completion list, got {result}")))?;
    Ok(items
        .iter()
        .filter_map(|item| item.get("label").and_then(Value::as_str).map(ToOwned::to_owned))
        .collect())
}

fn initialize_with_project_paths(
    workspace: &TempWorkspace,
    files: &[(&str, &str)],
) -> Result<(LspHarness, String), String> {
    workspace.write(".perl-lsp.toml", "[perl]\ninclude_paths = [\"lib\"]\n")?;
    for (path, content) in files {
        workspace.write(path, content)?;
    }

    let mut harness = LspHarness::new_raw();
    harness.initialize_ready(&workspace.root_uri, None)?;
    Ok((harness, workspace.uri("main.pl")))
}

#[test]
fn carton_root_reaches_module_completion() -> TestResult {
    let workspace = TempWorkspace::new()?;
    let (mut harness, main_uri) = initialize_with_project_paths(
        &workspace,
        &[
            ("cpanfile", "# Carton project marker\n"),
            ("carton.lock", "snapshot\n"),
            ("local/lib/perl5/Carton/Only.pm", "package Carton::Only;\n1;\n"),
        ],
    )?;
    let source = "use Carton::O";
    harness.open(&main_uri, source)?;
    harness.barrier();

    let result = harness.completion_at(&main_uri, 0, source.len() as u32)?;
    let labels = labels(&result)?;
    assert!(
        labels.iter().any(|label| label == "Carton::Only"),
        "Carton-detected local module should be offered by completion, got {labels:?}"
    );
    Ok(())
}

#[test]
fn carmel_root_reaches_module_definition() -> TestResult {
    let workspace = TempWorkspace::new()?;
    let (mut harness, main_uri) = initialize_with_project_paths(
        &workspace,
        &[
            ("cpanfile", "requires 'Carmel::Only';\n"),
            ("vendor/lib/perl5/Carmel/Only.pm", "package Carmel::Only;\n1;\n"),
        ],
    )?;
    let source = "use Carmel::Only;\n";
    harness.open(&main_uri, source)?;
    harness.barrier();

    let position = source.find("Carmel::Only").ok_or("test source is missing Carmel::Only")? as u32;
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": main_uri },
            "position": { "line": 0, "character": position }
        }),
    )?;
    let locations = result
        .as_array()
        .ok_or_else(|| std::io::Error::other(format!("expected definition array, got {result}")))?;
    let expected_uri = workspace.uri("vendor/lib/perl5/Carmel/Only.pm");
    assert!(
        locations.iter().any(|location| {
            location.get("uri").and_then(Value::as_str) == Some(expected_uri.as_str())
        }),
        "Carmel-detected vendor module should resolve to {expected_uri}, got {locations:?}"
    );
    Ok(())
}

#[test]
fn carmel_root_does_not_activate_without_cpanfile() -> TestResult {
    let workspace = TempWorkspace::new()?;
    let (mut harness, main_uri) = initialize_with_project_paths(
        &workspace,
        &[("vendor/lib/perl5/Carmel/Only.pm", "package Carmel::Only;\n1;\n")],
    )?;
    let source = "use Carmel::Only;\n";
    harness.open(&main_uri, source)?;
    harness.barrier();

    let position = source.find("Carmel::Only").ok_or("test source is missing Carmel::Only")? as u32;
    let result = harness.request(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": main_uri },
            "position": { "line": 0, "character": position }
        }),
    )?;
    assert!(
        result.as_array().is_some_and(Vec::is_empty),
        "Carmel vendor path must stay inactive without cpanfile, got {result}"
    );
    Ok(())
}
