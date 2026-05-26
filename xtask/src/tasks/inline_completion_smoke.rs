use color_eyre::eyre::{Result, bail, eyre};
use perl_lsp_ux_tests::{FakeWorkspace, ScenarioConfig, UxClient};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::time::Duration;

pub fn run(binary: PathBuf) -> Result<()> {
    if !binary.is_file() {
        bail!("inline-completion smoke binary does not exist: {}", binary.display());
    }

    let workspace = ux(FakeWorkspace::new())?;
    let timeout = Duration::from_secs(30);
    let config = ScenarioConfig {
        timeout,
        client_capability_overrides: json!({
            "textDocument": {
                "inlineCompletion": {
                    "dynamicRegistration": true
                }
            }
        }),
        ..ScenarioConfig::default()
    };

    let binary_path = binary.to_string_lossy().into_owned();
    let client = ux(UxClient::spawn(&binary_path, &workspace, &config))?;

    let initialize = client.initialize_result();
    ensure_dynamic_initialize_shape(&initialize)?;

    let uri = "file:///release-inline.pl";
    ux(client.did_open(uri, "use "))?;
    let items = request_inline_completion_items(&client, uri, 0, 4, timeout)?;
    if !items.iter().any(|item| item.get("insertText") == Some(&json!("strict;"))) {
        bail!("inline-completion smoke expected insertText strict;, got: {}", Value::Array(items));
    }

    let neutral_uri = "file:///release-inline-neutral.pl";
    ux(client.did_open(neutral_uri, "my $name = \"World\";"))?;
    let neutral_items = request_inline_completion_items(&client, neutral_uri, 0, 11, timeout)?;
    if !neutral_items.is_empty() {
        bail!(
            "inline-completion smoke expected empty unsupported-position result, got: {}",
            Value::Array(neutral_items)
        );
    }

    let shutdown = ux(client.request("shutdown", json!({}), timeout))?;
    if let Some(error) = shutdown.get("error") {
        bail!("shutdown returned error: {}", error);
    }
    ux(client.notify("exit", json!({})))?;

    println!("inline-completion stdio smoke OK: {}", binary.display());
    Ok(())
}

fn ensure_dynamic_initialize_shape(initialize: &Value) -> Result<()> {
    if initialize.pointer("/result/capabilities/inlineCompletionProvider").is_some()
        || initialize.pointer("/capabilities/inlineCompletionProvider").is_some()
    {
        bail!(
            "dynamic inline-completion client received duplicate static inlineCompletionProvider: {}",
            initialize
        );
    }

    if initialize.pointer("/result/capabilities/experimental/inlineCompletionProvider").is_some()
        || initialize.pointer("/capabilities/experimental/inlineCompletionProvider").is_some()
    {
        bail!(
            "initialize response advertised experimental.inlineCompletionProvider: {}",
            initialize
        );
    }

    Ok(())
}

fn request_inline_completion_items(
    client: &UxClient,
    uri: &str,
    line: u32,
    character: u32,
    timeout: Duration,
) -> Result<Vec<Value>> {
    let response = ux(client.request(
        "textDocument/inlineCompletion",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character },
            "context": { "triggerKind": 2 }
        }),
        timeout,
    ))?;

    if let Some(error) = response.get("error") {
        bail!("textDocument/inlineCompletion returned error: {}", error);
    }

    let result = response
        .get("result")
        .ok_or_else(|| eyre!("inline-completion response missing result: {}", response))?;

    match result {
        Value::Null => Ok(Vec::new()),
        Value::Array(items) => Ok(items.clone()),
        Value::Object(_) => {
            result.get("items").and_then(Value::as_array).cloned().ok_or_else(|| {
                eyre!("inline-completion response missing result.items array: {}", response)
            })
        }
        _ => bail!("inline-completion response had unexpected result shape: {}", response),
    }
}

fn ux<T>(result: anyhow::Result<T>) -> Result<T> {
    result.map_err(|error| eyre!("{error:#}"))
}
