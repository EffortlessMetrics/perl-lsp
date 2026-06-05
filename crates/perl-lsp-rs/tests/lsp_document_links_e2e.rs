//! End-to-end stdio coverage for document links and deferred resolution.
//!
//! The in-process document-link tests validate provider behavior. This test
//! drives the same workflow through a spawned `perl-lsp` process using real
//! JSON-RPC framing so editor-facing module and file links stay covered at the
//! process boundary.

mod support;

use serde_json::{Value, json};
use support::lsp_client::LspClient;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn response_result_array(response: &Value) -> Result<&Vec<Value>, Box<dyn std::error::Error>> {
    let result = response.get("result").ok_or("response missing result")?;
    Ok(result.as_array().ok_or("documentLink result should be an array")?)
}

fn data_field<'a>(link: &'a Value, field: &str) -> Option<&'a str> {
    link.pointer(&format!("/data/{field}")).and_then(Value::as_str)
}

#[test]
fn document_links_resolve_modules_and_relative_files_over_stdio() -> TestResult {
    let bin = env!("CARGO_BIN_EXE_perl-lsp");
    let mut client = LspClient::spawn(bin)?;
    let document_path = std::env::temp_dir().join("lsp_document_links_e2e").join("main.pl");
    let document_url = url::Url::from_file_path(&document_path)
        .map_err(|()| format!("failed to build file URI for {}", document_path.display()))?;
    let uri = document_url.as_str();
    let expected_file_target = document_url.join("lib/Local.pl")?.to_string();
    let source = r#"use strict;
use warnings;
use Data::Dumper;
require "lib/Local.pl";
"#;

    client.did_open(uri, "perl", source)?;

    let links_response = client.request(
        "textDocument/documentLink",
        json!({
            "textDocument": { "uri": uri }
        }),
    )?;
    assert!(links_response.get("error").is_none(), "documentLink failed: {links_response:#}");

    let links = response_result_array(&links_response)?;
    assert_eq!(links.len(), 2, "expected one module link and one file link: {links:#?}");
    assert!(
        links.iter().all(|link| link.get("target").is_none()),
        "documentLink should return deferred links without eager targets: {links:#?}"
    );

    let module_link = links
        .iter()
        .find(|link| data_field(link, "module") == Some("Data::Dumper"))
        .ok_or("expected deferred module link for Data::Dumper")?;
    assert_eq!(data_field(module_link, "type"), Some("module"));

    let file_link = links
        .iter()
        .find(|link| data_field(link, "path") == Some("lib/Local.pl"))
        .ok_or("expected deferred file link for lib/Local.pl")?;
    assert_eq!(data_field(file_link, "type"), Some("file"));

    let resolved_module = client.request("documentLink/resolve", module_link.clone())?;
    assert!(
        resolved_module.get("error").is_none(),
        "documentLink/resolve failed for module link: {resolved_module:#}"
    );
    let module_target = resolved_module
        .pointer("/result/target")
        .and_then(Value::as_str)
        .ok_or("resolved module link should include a target")?;
    assert!(
        module_target.starts_with("file:")
            || module_target == "https://metacpan.org/pod/Data::Dumper",
        "module link should resolve to a local file or MetaCPAN fallback, got {module_target:?}"
    );

    let resolved_file = client.request("documentLink/resolve", file_link.clone())?;
    assert!(
        resolved_file.get("error").is_none(),
        "documentLink/resolve failed for file link: {resolved_file:#}"
    );
    let file_target = resolved_file
        .pointer("/result/target")
        .and_then(Value::as_str)
        .ok_or("resolved file link should include a target")?;
    assert_eq!(
        file_target, expected_file_target,
        "relative require path should resolve against the opened document URI"
    );

    client.shutdown()?;
    Ok(())
}
