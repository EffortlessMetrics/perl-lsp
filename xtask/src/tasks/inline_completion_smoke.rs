use color_eyre::eyre::{Result, bail, eyre};
use perl_lsp_ux_tests::{FakeWorkspace, ScenarioConfig, UxClient};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

pub fn run(binary: PathBuf) -> Result<()> {
    let binary = resolve_binary_path(binary)?;

    run_static_client(&binary)?;
    run_dynamic_client(&binary)?;
    run_disabled_client(&binary)?;

    println!("inline-completion stdio smoke OK: {}", binary.display());
    Ok(())
}

fn run_static_client(binary: &PathBuf) -> Result<()> {
    let workspace = ux(FakeWorkspace::new())?;
    let timeout = Duration::from_secs(30);
    let config = ScenarioConfig { timeout, ..ScenarioConfig::default() };
    let client = spawn_client(binary, &workspace, &config)?;

    let initialize = client.initialize_result();
    ensure_static_initialize_shape(&initialize)?;
    assert_no_inline_registration(&client, Duration::from_millis(300))?;
    assert_inline_completion_runtime(&client, "static", timeout)?;
    shutdown_exit(&client, timeout)
}

fn run_dynamic_client(binary: &PathBuf) -> Result<()> {
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
    let client = spawn_client(binary, &workspace, &config)?;

    let initialize = client.initialize_result();
    ensure_dynamic_initialize_shape(&initialize)?;
    let registration = wait_for_inline_registration(&client, timeout)?;
    ensure_inline_registration_shape(&registration)?;
    assert_inline_completion_runtime(&client, "dynamic", timeout)?;
    shutdown_exit(&client, timeout)
}

fn run_disabled_client(binary: &PathBuf) -> Result<()> {
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
        initialization_options: json!({
            "disabledFeatures": ["lsp.inline_completion"]
        }),
        ..ScenarioConfig::default()
    };
    let client = spawn_client(binary, &workspace, &config)?;

    let initialize = client.initialize_result();
    ensure_disabled_initialize_shape(&initialize)?;
    assert_no_inline_registration(&client, Duration::from_millis(300))?;

    let uri = "file:///release-inline-disabled.pl";
    ux(client.did_open(uri, "use "))?;
    let response = request_inline_completion_response(&client, uri, 0, 4, timeout)?;
    if response.get("error").is_none() {
        bail!("disabled inline completion unexpectedly succeeded: {}", response);
    }

    shutdown_exit(&client, timeout)
}

fn spawn_client(
    binary: &PathBuf,
    workspace: &FakeWorkspace,
    config: &ScenarioConfig,
) -> Result<UxClient> {
    let binary_path = binary.to_string_lossy().into_owned();
    ux(UxClient::spawn(&binary_path, workspace, config))
}

fn capabilities(initialize: &Value) -> Result<&Value> {
    initialize
        .pointer("/result/capabilities")
        .or_else(|| initialize.get("capabilities"))
        .ok_or_else(|| eyre!("initialize response missing capabilities: {}", initialize))
}

fn ensure_static_initialize_shape(initialize: &Value) -> Result<()> {
    let caps = capabilities(initialize)?;
    if caps.get("inlineCompletionProvider") != Some(&json!({})) {
        bail!("static client did not receive inlineCompletionProvider: {}", initialize);
    }
    ensure_standard_provider_not_experimental(caps)?;
    ensure_stream_extension_enabled(caps)
}

fn ensure_dynamic_initialize_shape(initialize: &Value) -> Result<()> {
    let caps = capabilities(initialize)?;
    if caps.get("inlineCompletionProvider").is_some() {
        bail!(
            "dynamic inline-completion client received duplicate static inlineCompletionProvider: {}",
            initialize
        );
    }
    ensure_standard_provider_not_experimental(caps)?;
    ensure_stream_extension_enabled(caps)
}

fn ensure_disabled_initialize_shape(initialize: &Value) -> Result<()> {
    let caps = capabilities(initialize)?;
    if caps.get("inlineCompletionProvider").is_some() {
        bail!("disabled inline completion still advertised inlineCompletionProvider: {initialize}");
    }
    ensure_standard_provider_not_experimental(caps)?;
    if caps.pointer("/experimental/perlInlineCompletionStream").is_some() {
        bail!(
            "disabled inline completion still advertised perlInlineCompletionStream: {initialize}"
        );
    }
    Ok(())
}

fn ensure_standard_provider_not_experimental(caps: &Value) -> Result<()> {
    if caps.pointer("/experimental/inlineCompletionProvider").is_some() {
        bail!("initialize response advertised experimental.inlineCompletionProvider: {caps}");
    }
    Ok(())
}

fn ensure_stream_extension_enabled(caps: &Value) -> Result<()> {
    if caps.pointer("/experimental/perlInlineCompletionStream") != Some(&json!(true)) {
        bail!("inline completion did not advertise vendor stream extension separately: {caps}");
    }
    Ok(())
}

fn wait_for_inline_registration(client: &UxClient, timeout: Duration) -> Result<Value> {
    let deadline = Instant::now() + timeout;
    loop {
        for event in client.peek_raw_events() {
            if let Some(registration) = inline_registration(&event) {
                let id = event.get("id").and_then(Value::as_i64).ok_or_else(|| {
                    eyre!("client/registerCapability missing integer JSON-RPC id: {}", event)
                })?;
                if !(1..=i64::from(i32::MAX)).contains(&id) {
                    bail!("client/registerCapability JSON-RPC id out of bounds: {id}");
                }
                return Ok(registration.clone());
            }
        }

        if Instant::now() >= deadline {
            bail!("dynamic client did not receive textDocument/inlineCompletion registration");
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn assert_no_inline_registration(client: &UxClient, timeout: Duration) -> Result<()> {
    let deadline = Instant::now() + timeout;
    loop {
        for event in client.peek_raw_events() {
            if inline_registration(&event).is_some() {
                bail!(
                    "client unexpectedly received inline-completion dynamic registration: {event}"
                );
            }
        }

        if Instant::now() >= deadline {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn inline_registration(event: &Value) -> Option<&Value> {
    if event.get("method").and_then(Value::as_str) != Some("client/registerCapability") {
        return None;
    }
    event.pointer("/params/registrations").and_then(Value::as_array)?.iter().find(|registration| {
        registration.get("method").and_then(Value::as_str) == Some("textDocument/inlineCompletion")
    })
}

fn ensure_inline_registration_shape(registration: &Value) -> Result<()> {
    if registration.get("id") != Some(&json!("perl-inlineCompletion")) {
        bail!("inline-completion registration id mismatch: {registration}");
    }
    if registration.get("method") != Some(&json!("textDocument/inlineCompletion")) {
        bail!("inline-completion registration method mismatch: {registration}");
    }

    let selectors = registration
        .pointer("/registerOptions/documentSelector")
        .and_then(Value::as_array)
        .ok_or_else(|| eyre!("inline-completion registration missing documentSelector"))?;
    for language in ["perl", "perl5"] {
        if !selectors.iter().any(|selector| selector.get("language") == Some(&json!(language))) {
            bail!("inline-completion registration missing {language} selector: {registration}");
        }
    }

    Ok(())
}

fn assert_inline_completion_runtime(
    client: &UxClient,
    label: &str,
    timeout: Duration,
) -> Result<()> {
    let uri = format!("file:///release-inline-{label}.pl");
    ux(client.did_open(&uri, "use "))?;
    let items = request_inline_completion_items(client, &uri, 0, 4, timeout)?;
    if !items.iter().any(|item| item.get("insertText") == Some(&json!("strict;"))) {
        bail!("inline-completion smoke expected insertText strict;, got: {}", Value::Array(items));
    }

    let neutral_uri = format!("file:///release-inline-neutral-{label}.pl");
    ux(client.did_open(&neutral_uri, "my $name = \"World\";"))?;
    let neutral_items = request_inline_completion_items(client, &neutral_uri, 0, 11, timeout)?;
    if !neutral_items.is_empty() {
        bail!(
            "inline-completion smoke expected empty unsupported-position result, got: {}",
            Value::Array(neutral_items)
        );
    }

    Ok(())
}

fn request_inline_completion_response(
    client: &UxClient,
    uri: &str,
    line: u32,
    character: u32,
    timeout: Duration,
) -> Result<Value> {
    ux(client.request(
        "textDocument/inlineCompletion",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character },
            "context": { "triggerKind": 2 }
        }),
        timeout,
    ))
}

fn request_inline_completion_items(
    client: &UxClient,
    uri: &str,
    line: u32,
    character: u32,
    timeout: Duration,
) -> Result<Vec<Value>> {
    let response = request_inline_completion_response(client, uri, line, character, timeout)?;

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

fn shutdown_exit(client: &UxClient, timeout: Duration) -> Result<()> {
    let shutdown = ux(client.request("shutdown", json!({}), timeout))?;
    if let Some(error) = shutdown.get("error") {
        bail!("shutdown returned error: {}", error);
    }
    ux(client.notify("exit", json!({})))
}

fn ux<T>(result: anyhow::Result<T>) -> Result<T> {
    result.map_err(|error| eyre!("{error:#}"))
}

fn resolve_binary_path(binary: PathBuf) -> Result<PathBuf> {
    if binary.is_file() {
        return Ok(binary);
    }

    #[cfg(windows)]
    {
        let has_extension = binary.extension().is_some();
        if !has_extension {
            let mut exe = binary.clone();
            exe.set_extension("exe");
            if exe.is_file() {
                return Ok(exe);
            }
        }
    }

    bail!("inline-completion smoke binary does not exist: {}", binary.display());
}
