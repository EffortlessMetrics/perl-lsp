//! Tests for LSP execute command functionality
use perl_lsp::{JsonRpcRequest, LspServer};
use serde_json::{Value, json};
use std::fs;
use tempfile::TempDir;
use url::Url;

fn setup_server(root_path: Option<String>) -> LspServer {
    let server = LspServer::new();

    // Initialize the server
    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "initialize".to_string(),
        params: Some(json!({
            "processId": null,
            "rootPath": root_path,
            "capabilities": {}
        })),
        id: Some(json!(1)),
    };

    let _response = server.handle_request(init_request);

    // Send the initialized notification to complete the handshake
    let initialized_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "initialized".to_string(),
        params: Some(json!({})),
        id: None,
    };

    let _initialized_response = server.handle_request(initialized_request);
    server
}

fn test_error(message: impl Into<String>) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message.into())
}

fn sorted_object_keys_at(
    value: &Value,
    pointer: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let object = value
        .pointer(pointer)
        .and_then(Value::as_object)
        .ok_or_else(|| test_error(format!("expected object at JSON pointer {pointer}")))?;
    let mut keys: Vec<String> = object.keys().cloned().collect();
    keys.sort();
    Ok(keys)
}

fn expected_keys(keys: &[&str]) -> Vec<String> {
    let mut expected: Vec<String> = keys.iter().map(|key| (*key).to_string()).collect();
    expected.sort();
    expected
}

fn workspace_trust_report_schema() -> Result<Value, Box<dyn std::error::Error>> {
    let schema_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("schemas")
        .join("workspace_trust_report.v1.schema.json");
    let schema_text = std::fs::read_to_string(&schema_path).map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("failed to read {}: {error}", schema_path.display()),
        )
    })?;
    Ok(serde_json::from_str(&schema_text)?)
}

fn schema_required_fields(
    schema: &Value,
    pointer: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let fields = schema.pointer(pointer).and_then(Value::as_array).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("schema missing required array at {pointer}"),
        )
    })?;
    let mut required = Vec::with_capacity(fields.len());
    for field in fields {
        let Some(name) = field.as_str() else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("schema required array at {pointer} contains non-string item: {field}"),
            )
            .into());
        };
        required.push(name.to_string());
    }
    Ok(required)
}

fn assert_schema_required_fields_present(
    value: &Value,
    schema: &Value,
    required_pointer: &str,
    context: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    for field in schema_required_fields(schema, required_pointer)? {
        if value.get(&field).is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("{context} missing schema-required field {field}: {value}"),
            )
            .into());
        }
    }
    Ok(())
}

#[test]
fn test_execute_command_run_file() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let root_path = temp_dir.path().to_string_lossy().to_string();
    let server = setup_server(Some(root_path.clone()));

    // Create a test file
    let test_content = r#"#!/usr/bin/perl
use strict;
use warnings;

print "Hello, World!\n";
"#;

    let file_path = temp_dir.path().join("test.pl");
    fs::write(&file_path, test_content)?;
    let file_path_str = file_path.to_string_lossy().to_string();

    let uri = format!("file://{}", file_path_str);
    let open_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": test_content
            }
        })),
        id: None,
    };

    // Send the notification
    let _ = server.handle_request(open_request);

    // Execute the run file command
    let execute_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "workspace/executeCommand".to_string(),
        params: Some(json!({
            "command": "perl.runFile",
            "arguments": [file_path_str]
        })),
        id: Some(json!(2)),
    };

    let response =
        server.handle_request(execute_request).ok_or("No response from execute command")?;
    let result = response.result.ok_or("No result in response")?;

    // Check that we got a response (even if the command might fail due to perl not installed/env issues)
    assert!(result.is_object());
    assert!(result.get("success").is_some());
    // output or error should be present
    assert!(result.get("output").is_some() || result.get("error").is_some());

    Ok(())
}

#[test]
fn test_execute_command_run_tests() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let root_path = temp_dir.path().to_string_lossy().to_string();
    let server = setup_server(Some(root_path.clone()));

    // Create a test file with Test::More
    let test_content = r#"#!/usr/bin/perl
use strict;
use warnings;
use Test::More tests => 2;

ok(1, "First test");
is(1 + 1, 2, "Math works");
"#;

    let file_path = temp_dir.path().join("test.t");
    fs::write(&file_path, test_content)?;
    let file_path_str = file_path.to_string_lossy().to_string();

    let uri = format!("file://{}", file_path_str);
    let open_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "perl",
                "version": 1,
                "text": test_content
            }
        })),
        id: None,
    };

    // Send the notification
    let _ = server.handle_request(open_request);

    // Execute the run tests command
    let execute_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "workspace/executeCommand".to_string(),
        params: Some(json!({
            "command": "perl.runTests",
            "arguments": [file_path_str]
        })),
        id: Some(json!(2)),
    };

    let response =
        server.handle_request(execute_request).ok_or("No response from execute command")?;
    let result = response.result.ok_or("No result in response")?;

    // Check response structure
    assert!(result.is_object());
    assert!(result.get("success").is_some());
    assert!(result.get("output").is_some());

    // Check that it recognized this as a test file
    if result.get("command").is_some() {
        let command = result
            .get("command")
            .ok_or("No command in result")?
            .as_str()
            .ok_or("Command is not a string")?;
        // If prove is available, it should use prove for .t files
        assert!(command == "prove" || command == "perl");
    }

    Ok(())
}

#[test]
fn test_execute_command_unknown() -> Result<(), Box<dyn std::error::Error>> {
    let server = setup_server(None);

    // Try an unknown command
    let execute_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "workspace/executeCommand".to_string(),
        params: Some(json!({
            "command": "perl.unknownCommand",
            "arguments": []
        })),
        id: Some(json!(2)),
    };

    let response = server.handle_request(execute_request);

    // Should return an error
    assert!(response.is_some());
    let response = response.ok_or("Expected a response for unknown command")?;
    assert!(response.error.is_some());

    Ok(())
}

#[test]
fn test_execute_command_capabilities() -> Result<(), Box<dyn std::error::Error>> {
    let server = LspServer::new();

    // Initialize and check capabilities
    let init_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "initialize".to_string(),
        params: Some(json!({
            "processId": null,
            "rootPath": "/test",
            "capabilities": {}
        })),
        id: Some(json!(10)),
    };

    let response = server.handle_request(init_request).ok_or("No response from initialize")?;
    let result = response.result.ok_or("No result in initialize response")?;
    let capabilities = result.get("capabilities").ok_or("No capabilities in result")?;
    let execute_command = capabilities
        .get("executeCommandProvider")
        .ok_or("No executeCommandProvider in capabilities")?;
    let commands = execute_command
        .get("commands")
        .ok_or("No commands in executeCommandProvider")?
        .as_array()
        .ok_or("Commands is not an array")?;

    // Check that our new commands are advertised
    let command_strs: Vec<&str> = commands.iter().filter_map(|v| v.as_str()).collect();

    assert!(command_strs.contains(&"perl.runTests"));
    assert!(command_strs.contains(&"perl.runFile"));
    assert!(command_strs.contains(&"perl.runTestSub"));
    assert!(command_strs.contains(&"perl.runCritic"));
    assert!(command_strs.contains(&"perl.explainProviderDecision"));
    assert!(command_strs.contains(&"perl.workspaceTrustReport"));
    assert!(command_strs.contains(&"perl.previewSafeDelete"));
    assert!(command_strs.contains(&"perl.safeDeleteSymbol"));
    assert!(command_strs.contains(&"perl.previewPackageRename"));
    assert!(command_strs.contains(&"perl.explainMissingModuleLookup"));

    Ok(())
}

#[test]
fn test_execute_command_workspace_trust_report() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let root_path = temp_dir.path().to_string_lossy().to_string();
    let server = setup_server(Some(root_path));

    let execute_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "workspace/executeCommand".to_string(),
        params: Some(json!({
            "command": "perl.workspaceTrustReport",
            "arguments": [{
                "client_runtime_state": {
                    "source": "vscode-extension",
                    "perldoc": {
                        "status": "client_surface_registered",
                        "uri_scheme": "perldoc",
                        "client_surface": "virtual_document"
                    },
                    "dap": {
                        "status": "client_state_reported",
                        "adapter_registered": true,
                        "active_perl_debug_session": false,
                        "managed_adapter_exists": true,
                        "launch_json_workspace_count": 1,
                        "workspace_folder_count": 1,
                        "launch_configuration": {
                            "status": "client_launch_config_reported",
                            "configuration_count": 3,
                            "perl_configuration_count": 2,
                            "launch_request_count": 1,
                            "attach_request_count": 1,
                            "perl_path_configured_count": 1,
                            "include_paths_configured_count": 2,
                            "include_path_entry_count": 3,
                            "non_string_include_path_count": 1,
                            "program_configured_count": 1,
                            "cwd_configured_count": 1,
                            "include_path_kind_counts": {
                                "workspace_variable": 1,
                                "relative": 2,
                                "raw_path_value": 99
                            },
                            "perl_path_kind_counts": {
                                "absolute": 1
                            },
                            "program_path_kind_counts": {
                                "workspace_variable": 1
                            },
                            "cwd_path_kind_counts": {
                                "relative": 1
                            },
                            "raw_include_paths": ["secret/lib"],
                            "claim_boundary": "Launch configuration state reports counts and path classes only."
                        }
                    },
                    "ignored": "not copied"
                }
            }]
        })),
        id: Some(json!(2)),
    };

    let response = server
        .handle_request(execute_request)
        .ok_or("No response from workspace-trust-report command")?;
    let result = response.result.ok_or("No result in workspace-trust-report response")?;

    assert_eq!(
        result.get("schema_version").and_then(|value| value.as_str()),
        Some("workspace_trust_report.v1")
    );
    assert_eq!(
        result.get("command").and_then(|value| value.as_str()),
        Some("perl.workspaceTrustReport")
    );
    assert!(
        result
            .get("claim_boundary")
            .and_then(|value| value.as_str())
            .is_some_and(|claim| claim.contains("does not scan files")),
        "report must state its no-scan claim boundary"
    );
    assert!(
        result.get("workspace").and_then(|value| value.as_object()).is_some(),
        "report should include workspace state"
    );
    assert!(
        result
            .get("module_resolution")
            .and_then(|value| value.get("global_workspace_config"))
            .is_some(),
        "report should include module-resolution config state"
    );
    assert_eq!(
        result.pointer("/setup_hints/perl_binary/version_status").and_then(|value| value.as_str()),
        Some("not_probed_by_report")
    );
    assert!(
        result
            .pointer("/setup_hints/claim_boundary")
            .and_then(|value| value.as_str())
            .is_some_and(|claim| claim.contains("do not resolve Perl")),
        "setup hints should preserve the no-probe boundary"
    );
    assert_eq!(
        result.pointer("/setup_hints/perldoc/status").and_then(|value| value.as_str()),
        Some("oracle_contract_reported_not_run")
    );
    assert_eq!(
        result.pointer("/setup_hints/perldoc/run_status").and_then(|value| value.as_str()),
        Some("not_run_by_report")
    );
    assert_eq!(
        result.pointer("/setup_hints/dap/status").and_then(|value| value.as_str()),
        Some("not_probed_by_lsp_workspace_report")
    );
    assert_eq!(
        result.pointer("/client_runtime_state/source").and_then(|value| value.as_str()),
        Some("vscode-extension")
    );
    assert_eq!(
        result.pointer("/client_runtime_state/perldoc/uri_scheme").and_then(|value| value.as_str()),
        Some("perldoc")
    );
    assert_eq!(
        result
            .pointer("/client_runtime_state/dap/managed_adapter_exists")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
    assert_eq!(
        result
            .pointer("/client_runtime_state/dap/launch_configuration/status")
            .and_then(|value| value.as_str()),
        Some("client_launch_config_reported")
    );
    assert_eq!(
        result
            .pointer("/client_runtime_state/dap/launch_configuration/include_path_entry_count")
            .and_then(|value| value.as_u64()),
        Some(3)
    );
    assert_eq!(
        result
            .pointer(
                "/client_runtime_state/dap/launch_configuration/include_path_kind_counts/workspace_variable",
            )
            .and_then(|value| value.as_u64()),
        Some(1)
    );
    assert!(
        result
            .pointer("/client_runtime_state/dap/launch_configuration/include_path_kind_counts/raw_path_value")
            .is_none(),
        "launch path class counts should be sanitized to known classes"
    );
    assert!(
        result
            .pointer("/client_runtime_state/dap/launch_configuration/raw_include_paths")
            .is_none(),
        "raw launch include paths should not be copied into the report"
    );
    assert!(
        result.pointer("/client_runtime_state/ignored").is_none(),
        "client runtime state should be sanitized to known fields"
    );
    assert!(
        result.get("index").and_then(|value| value.as_object()).is_some(),
        "report should include index state"
    );
    assert_eq!(
        result
            .get("providers")
            .and_then(|value| value.get("support_tiers"))
            .and_then(|value| value.get("completion"))
            .and_then(|value| value.as_str()),
        Some("partial-live-with-fallback")
    );

    Ok(())
}

#[test]
fn test_execute_command_workspace_trust_report_schema_snapshot()
-> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let root_path = temp_dir.path().to_string_lossy().to_string();
    let server = setup_server(Some(root_path.clone()));

    let execute_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "workspace/executeCommand".to_string(),
        params: Some(json!({
            "command": "perl.workspaceTrustReport",
            "arguments": [{
                "client_runtime_state": {
                    "source": "vscode-extension",
                    "perldoc": {
                        "status": "client_surface_registered",
                        "uri_scheme": "perldoc",
                        "client_surface": "virtual_document",
                        "raw_secret": "must-not-copy"
                    },
                    "dap": {
                        "status": "client_state_reported",
                        "adapter_registered": true,
                        "active_perl_debug_session": false,
                        "managed_adapter_exists": true,
                        "launch_json_workspace_count": 1,
                        "workspace_folder_count": 1,
                        "launch_configuration": {
                            "status": "client_launch_config_reported",
                            "configuration_count": 2,
                            "perl_configuration_count": 1,
                            "launch_request_count": 1,
                            "attach_request_count": 0,
                            "perl_path_configured_count": 1,
                            "include_paths_configured_count": 1,
                            "include_path_entry_count": 2,
                            "non_string_include_path_count": 0,
                            "program_configured_count": 1,
                            "cwd_configured_count": 1,
                            "include_path_kind_counts": {
                                "workspace_variable": 1,
                                "relative": 1,
                                "raw_path_value": 1
                            },
                            "perl_path_kind_counts": {
                                "absolute": 1
                            },
                            "program_path_kind_counts": {
                                "workspace_variable": 1
                            },
                            "cwd_path_kind_counts": {
                                "relative": 1
                            },
                            "raw_include_paths": ["secret/lib"],
                            "raw_perl_path": "/opt/private/perl",
                            "claim_boundary": "Launch configuration state reports counts and path classes only."
                        }
                    },
                    "ignored": "must-not-copy"
                }
            }]
        })),
        id: Some(json!(2)),
    };

    let response = server
        .handle_request(execute_request)
        .ok_or("No response from workspace-trust-report command")?;
    let result = response.result.ok_or("No result in workspace-trust-report response")?;
    let schema = workspace_trust_report_schema()?;

    assert_schema_required_fields_present(&result, &schema, "/required", "workspace trust report")?;
    assert_schema_required_fields_present(
        result.get("workspace").ok_or("missing workspace report")?,
        &schema,
        "/$defs/workspace/required",
        "workspace report",
    )?;
    assert_schema_required_fields_present(
        result.get("module_resolution").ok_or("missing module_resolution report")?,
        &schema,
        "/$defs/module_resolution/required",
        "module-resolution report",
    )?;
    assert_schema_required_fields_present(
        result
            .pointer("/module_resolution/global_workspace_config")
            .ok_or("missing global_workspace_config")?,
        &schema,
        "/$defs/workspace_config/required",
        "global workspace config report",
    )?;
    assert_schema_required_fields_present(
        result.get("setup_hints").ok_or("missing setup_hints report")?,
        &schema,
        "/$defs/setup_hints/required",
        "setup hints report",
    )?;
    assert_schema_required_fields_present(
        result.pointer("/setup_hints/perl_binary").ok_or("missing setup_hints perl_binary")?,
        &schema,
        "/$defs/setup_hints_perl_binary/required",
        "setup hints perl binary report",
    )?;
    assert_schema_required_fields_present(
        result.pointer("/setup_hints/perldoc").ok_or("missing setup_hints perldoc")?,
        &schema,
        "/$defs/setup_hints_perldoc/required",
        "setup hints perldoc report",
    )?;
    assert_schema_required_fields_present(
        result.pointer("/setup_hints/dap").ok_or("missing setup_hints dap")?,
        &schema,
        "/$defs/setup_hints_dap/required",
        "setup hints DAP report",
    )?;
    assert_schema_required_fields_present(
        result.get("client_runtime_state").ok_or("missing client_runtime_state")?,
        &schema,
        "/$defs/client_runtime_state/required",
        "client runtime state report",
    )?;
    assert_schema_required_fields_present(
        result.pointer("/client_runtime_state/dap").ok_or("missing client runtime DAP")?,
        &schema,
        "/$defs/client_runtime_dap/required",
        "client runtime DAP report",
    )?;
    assert_schema_required_fields_present(
        result
            .pointer("/client_runtime_state/dap/launch_configuration")
            .ok_or("missing launch configuration report")?,
        &schema,
        "/$defs/launch_configuration/required",
        "launch configuration report",
    )?;
    assert_schema_required_fields_present(
        result.get("index").ok_or("missing index report")?,
        &schema,
        "/$defs/index/required",
        "index report",
    )?;
    assert_schema_required_fields_present(
        result.get("providers").ok_or("missing providers report")?,
        &schema,
        "/$defs/providers/required",
        "providers report",
    )?;
    assert_schema_required_fields_present(
        result.get("dynamic_boundaries").ok_or("missing dynamic_boundaries report")?,
        &schema,
        "/$defs/dynamic_boundaries/required",
        "dynamic boundaries report",
    )?;
    assert_schema_required_fields_present(
        result.get("copyable_payload").ok_or("missing copyable_payload")?,
        &schema,
        "/$defs/copyable_payload/required",
        "copyable workspace trust payload",
    )?;

    assert_eq!(
        sorted_object_keys_at(&result, "")?,
        expected_keys(&[
            "claim_boundary",
            "client_runtime_state",
            "command",
            "copyable_payload",
            "dynamic_boundaries",
            "index",
            "module_resolution",
            "providers",
            "schema_version",
            "setup_hints",
            "user_message",
            "workspace",
        ])
    );
    assert_eq!(
        sorted_object_keys_at(&result, "/workspace")?,
        expected_keys(&["folders", "open_document_count", "root_path", "workspace_folder_count"])
    );
    assert_eq!(
        sorted_object_keys_at(&result, "/module_resolution")?,
        expected_keys(&["global_workspace_config", "policy"])
    );
    assert_eq!(
        sorted_object_keys_at(&result, "/module_resolution/global_workspace_config")?,
        expected_keys(&[
            "effective_include_paths",
            "include_paths",
            "perl5lib_entry_count",
            "perl5lib_precedence",
            "perl_args_count",
            "perl_path",
            "resolution_timeout_ms",
            "system_inc_status",
            "use_perl5lib",
            "use_system_inc",
        ])
    );
    assert_eq!(
        sorted_object_keys_at(&result, "/setup_hints")?,
        expected_keys(&[
            "claim_boundary",
            "dap",
            "hints",
            "hint_count",
            "perldoc",
            "perl_binary",
            "status"
        ])
    );
    assert_eq!(
        sorted_object_keys_at(&result, "/setup_hints/perl_binary")?,
        expected_keys(&["args_count", "configured_path", "resolution_status", "version_status"])
    );
    assert_eq!(
        sorted_object_keys_at(&result, "/setup_hints/perldoc")?,
        expected_keys(&[
            "allow_local_lib",
            "allow_perl5lib",
            "allow_perl5opt",
            "argv_policy",
            "binary_source",
            "lc_all",
            "policy",
            "run_status",
            "status",
            "timeout_ms",
        ])
    );
    assert_eq!(
        sorted_object_keys_at(&result, "/client_runtime_state")?,
        expected_keys(&["claim_boundary", "dap", "perldoc", "schema_version", "source"])
    );
    assert_eq!(
        sorted_object_keys_at(&result, "/client_runtime_state/perldoc")?,
        expected_keys(&["client_surface", "status", "uri_scheme"])
    );
    assert_eq!(
        sorted_object_keys_at(&result, "/client_runtime_state/dap")?,
        expected_keys(&[
            "active_perl_debug_session",
            "adapter_registered",
            "launch_configuration",
            "launch_json_workspace_count",
            "managed_adapter_exists",
            "status",
            "workspace_folder_count",
        ])
    );
    assert_eq!(
        sorted_object_keys_at(&result, "/client_runtime_state/dap/launch_configuration")?,
        expected_keys(&[
            "attach_request_count",
            "claim_boundary",
            "configuration_count",
            "cwd_configured_count",
            "cwd_path_kind_counts",
            "include_path_entry_count",
            "include_path_kind_counts",
            "include_paths_configured_count",
            "launch_request_count",
            "non_string_include_path_count",
            "perl_configuration_count",
            "perl_path_configured_count",
            "perl_path_kind_counts",
            "program_configured_count",
            "program_path_kind_counts",
            "status",
        ])
    );
    assert_eq!(
        sorted_object_keys_at(
            &result,
            "/client_runtime_state/dap/launch_configuration/include_path_kind_counts"
        )?,
        expected_keys(&["relative", "workspace_variable"])
    );
    assert_eq!(
        sorted_object_keys_at(&result, "/index")?,
        expected_keys(&[
            "availability",
            "file_count",
            "indexed_file_count",
            "indexed_symbol_count",
            "indexing_in_progress",
            "pending_index_tasks",
            "reason",
            "state",
            "symbol_count",
        ])
    );
    assert_eq!(
        sorted_object_keys_at(&result, "/providers")?,
        expected_keys(&["decision_trace_count", "decision_trace_keys", "support_tiers"])
    );
    let support_tier_keys = sorted_object_keys_at(&result, "/providers/support_tiers")?;
    assert_eq!(
        sorted_object_keys_at(&result, "/copyable_payload")?,
        expected_keys(&[
            "claim_boundary",
            "client_runtime_schema_version",
            "client_runtime_source",
            "command",
            "configured_include_path_count",
            "dap_status",
            "decision_trace_count",
            "dynamic_boundary_policy",
            "effective_include_path_count",
            "launch_configuration_status",
            "open_document_count",
            "perl5lib_entry_count",
            "perl5lib_precedence",
            "perl_binary_resolution_status",
            "perl_lsp_version",
            "perldoc_run_status",
            "perldoc_status",
            "provider",
            "provider_support_tiers",
            "schema_version",
            "support_tier_link",
            "system_inc_status",
            "use_perl5lib",
            "use_system_inc",
            "workspace_folder_count",
            "workspace_root_class",
            "workspace_root_hash",
        ])
    );
    assert_eq!(
        sorted_object_keys_at(&result, "/copyable_payload/provider_support_tiers")?,
        support_tier_keys
    );

    for required_tier in [
        "completion",
        "diagnostics",
        "provider_decision_explanations",
        "real_workspace_baseline",
        "rename",
        "safe_delete",
        "semantic_tokens",
        "workspace_symbols",
        "workspace_trust_report",
    ] {
        assert!(
            support_tier_keys.iter().any(|key| key == required_tier),
            "workspace trust report support tiers should include {required_tier}"
        );
    }

    assert_eq!(
        result.get("schema_version").and_then(Value::as_str),
        Some("workspace_trust_report.v1")
    );
    assert_eq!(
        result.pointer("/client_runtime_state/schema_version").and_then(Value::as_str),
        Some("workspace_trust_client_runtime.v1")
    );
    assert_eq!(
        result.pointer("/copyable_payload/schema_version").and_then(Value::as_str),
        Some("workspace_trust_report_copyable.v1")
    );
    assert_eq!(
        result.pointer("/copyable_payload/provider").and_then(Value::as_str),
        Some("workspace_trust_report")
    );
    assert_eq!(
        result.pointer("/copyable_payload/command").and_then(Value::as_str),
        Some("perl.workspaceTrustReport")
    );
    assert_eq!(
        result.pointer("/copyable_payload/workspace_root_class").and_then(Value::as_str),
        Some("single_root")
    );
    assert!(
        result.pointer("/copyable_payload/workspace_root_hash").and_then(Value::as_str).is_some(),
        "copyable payload should use a workspace root hash instead of relying on raw paths"
    );
    let copyable_payload_text =
        serde_json::to_string(result.get("copyable_payload").ok_or("missing copyable_payload")?)?;
    assert!(
        !copyable_payload_text.contains(&root_path),
        "copyable payload must not include the raw workspace root"
    );
    assert_eq!(
        result.pointer("/copyable_payload/client_runtime_source").and_then(Value::as_str),
        Some("vscode-extension")
    );
    assert_eq!(
        result.pointer("/copyable_payload/client_runtime_schema_version").and_then(Value::as_str),
        Some("workspace_trust_client_runtime.v1")
    );
    assert_eq!(
        result.pointer("/copyable_payload/launch_configuration_status").and_then(Value::as_str),
        Some("client_launch_config_reported")
    );
    assert_eq!(
        result
            .pointer("/copyable_payload/provider_support_tiers/workspace_trust_report")
            .and_then(Value::as_str),
        Some("partial-live-with-fallback")
    );
    assert_eq!(
        result.pointer("/copyable_payload/support_tier_link").and_then(Value::as_str),
        Some("docs/project/status/SUPPORT_TIERS.md#claim-rows")
    );
    assert!(
        result.pointer("/copyable_payload/claim_boundary").and_then(Value::as_str).is_some_and(
            |claim| claim.contains("does not scan files")
                && claim.contains("probe Perl")
                && claim.contains("promote support tiers")
        ),
        "copyable payload must preserve the report-only claim boundary"
    );
    assert!(
        result.get("claim_boundary").and_then(Value::as_str).is_some_and(|claim| claim
            .contains("does not scan files")
            && claim.contains("probe Perl")),
        "top-level claim boundary must keep the report read-only"
    );
    assert!(
        result.pointer("/setup_hints/claim_boundary").and_then(Value::as_str).is_some_and(
            |claim| claim.contains("do not resolve Perl") && claim.contains("run perldoc")
        ),
        "setup hints must preserve no-probe/no-perldoc boundaries"
    );
    assert!(
        result
            .pointer("/client_runtime_state/claim_boundary")
            .and_then(Value::as_str)
            .is_some_and(|claim| claim.contains("sanitized to known fields")),
        "client runtime state must document sanitization"
    );

    let report_text = serde_json::to_string(&result)?;
    assert!(!report_text.contains("secret/lib"), "raw include paths must not be copied");
    assert!(!report_text.contains("/opt/private/perl"), "raw Perl paths must not be copied");
    assert!(!report_text.contains("raw_path_value"), "unknown path classes must not be copied");
    assert!(!report_text.contains("must-not-copy"), "unknown client fields must not be copied");

    Ok(())
}

#[test]
fn test_execute_command_explain_missing_module_lookup() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let root_path = temp_dir.path().to_string_lossy().to_string();
    let server = setup_server(Some(root_path));

    let script_path = temp_dir.path().join("script.pl");
    let script_content = "use Missing::Payload;\n";
    fs::write(&script_path, script_content)?;
    let script_uri = Url::from_file_path(&script_path)
        .map_err(|_| "failed to convert script path to file URI")?
        .to_string();

    let open_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "textDocument/didOpen".to_string(),
        params: Some(json!({
            "textDocument": {
                "uri": script_uri,
                "languageId": "perl",
                "version": 1,
                "text": script_content
            }
        })),
        id: None,
    };
    let _ = server.handle_request(open_request);

    let execute_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "workspace/executeCommand".to_string(),
        params: Some(json!({
            "command": "perl.explainMissingModuleLookup",
            "arguments": [{
                "module": "Missing::Payload",
                "textDocument": {"uri": script_uri},
                "position": {"line": 0, "character": 4}
            }]
        })),
        id: Some(json!(2)),
    };

    let response = server
        .handle_request(execute_request)
        .ok_or("No response from explain-missing-module-lookup command")?;
    let result = response.result.ok_or("No result in explain-missing-module-lookup response")?;

    assert_eq!(
        result.get("schema_version").and_then(|value| value.as_str()),
        Some("missing_module_lookup_explanation.v1")
    );
    assert_eq!(
        result.get("command").and_then(|value| value.as_str()),
        Some("perl.explainMissingModuleLookup")
    );
    assert_eq!(
        result.get("requested_module").and_then(|value| value.as_str()),
        Some("Missing::Payload")
    );
    assert_eq!(
        result.get("expected_relative_path").and_then(|value| value.as_str()),
        Some("Missing/Payload.pm")
    );
    assert_eq!(result.get("document_open").and_then(|value| value.as_bool()), Some(true));
    assert!(
        result
            .get("claim_boundary")
            .and_then(|value| value.as_str())
            .is_some_and(|claim| claim.contains("no workspace scan")),
        "missing-module explanation must state its explanation-only claim boundary"
    );

    let module_resolution =
        result.get("module_resolution").ok_or("missing module_resolution payload")?;
    assert_eq!(
        module_resolution
            .get("result")
            .and_then(|value| value.get("status"))
            .and_then(|value| value.as_str()),
        Some("not_found")
    );
    assert_eq!(
        module_resolution.get("perl5lib_policy").and_then(|value| value.as_str()),
        Some("enabled_but_environment_empty")
    );
    let include_paths = module_resolution
        .get("effective_include_paths")
        .and_then(|value| value.as_array())
        .ok_or("missing effective_include_paths")?;
    assert!(
        include_paths.iter().any(|entry| {
            entry.get("source").and_then(|value| value.as_str()) == Some("workspace includePaths")
                && entry.get("candidate_paths").and_then(|value| value.as_array()).is_some_and(
                    |candidates| {
                        candidates.iter().any(|candidate| {
                            candidate.get("path").and_then(|value| value.as_str()).is_some_and(
                                |path| path.contains("Missing") && path.contains("Payload.pm"),
                            )
                        })
                    },
                )
        }),
        "workspace includePaths candidate should include Missing/Payload.pm: {include_paths:?}"
    );

    let copyable_payload = result.get("copyable_payload").ok_or("missing copyable_payload")?;
    assert_eq!(
        copyable_payload.get("provider").and_then(|value| value.as_str()),
        Some("module_resolution")
    );
    assert_eq!(copyable_payload.get("result").and_then(|value| value.as_str()), Some("not_found"));
    assert_eq!(
        copyable_payload.get("support_tier_link").and_then(|value| value.as_str()),
        Some("docs/project/status/SUPPORT_TIERS.md#claim-rows")
    );

    Ok(())
}

#[test]
fn test_execute_command_explain_provider_decision() -> Result<(), Box<dyn std::error::Error>> {
    let server = setup_server(None);

    let execute_request = JsonRpcRequest {
        _jsonrpc: "2.0".to_string(),
        method: "workspace/executeCommand".to_string(),
        params: Some(json!({
            "command": "perl.explainProviderDecision",
            "arguments": [{
                "provider": "goto_definition",
                "receipt_id": "semantic-shadow-compare",
                "scenario": "mojolicious-navigation",
                "request_receipt": {
                    "provider": "goto_definition",
                    "decision": "acted",
                    "fact_source": "compiler_fact",
                    "confidence": "high",
                    "freshness": "fresh"
                },
                "request_position": {
                    "uri_scheme": "file",
                    "line": 7,
                    "character": 2
                }
            }]
        })),
        id: Some(json!(2)),
    };

    let response = server
        .handle_request(execute_request)
        .ok_or("No response from explain-provider-decision command")?;
    let result = response.result.ok_or("No result in explain-provider-decision response")?;

    assert_eq!(result.get("provider").and_then(|value| value.as_str()), Some("goto_definition"));
    assert_eq!(result.get("decision").and_then(|value| value.as_str()), Some("acted"));
    assert_eq!(
        result.get("reason").and_then(|value| value.as_str()),
        Some("source_backed_high_confidence")
    );
    assert_eq!(result.get("fact_source").and_then(|value| value.as_str()), Some("compiler_fact"));
    assert_eq!(result.get("confidence").and_then(|value| value.as_str()), Some("high"));
    assert_eq!(result.get("freshness").and_then(|value| value.as_str()), Some("fresh"));
    assert_eq!(result.get("fallback").and_then(|value| value.as_str()), Some("none"));
    assert_eq!(
        result.get("receipt_id").and_then(|value| value.as_str()),
        Some("semantic-shadow-compare")
    );
    assert_eq!(
        result.get("scenario").and_then(|value| value.as_str()),
        Some("mojolicious-navigation")
    );
    let request_receipt = result
        .get("request_receipt")
        .and_then(|value| value.as_object())
        .ok_or("missing request_receipt")?;
    assert_eq!(
        request_receipt.get("provider").and_then(|value| value.as_str()),
        Some("goto_definition")
    );
    assert_eq!(
        request_receipt.get("fact_source").and_then(|value| value.as_str()),
        Some("compiler_fact")
    );
    assert_eq!(result.get("dynamic_boundary").and_then(|value| value.as_bool()), Some(false));
    let copyable_payload = result
        .get("copyable_payload")
        .and_then(|value| value.as_object())
        .ok_or("missing copyable_payload")?;
    assert_eq!(
        copyable_payload.get("schema_version").and_then(|value| value.as_str()),
        Some("provider_decision_bug_report.v1")
    );
    assert_eq!(
        copyable_payload.get("perl_lsp_version").and_then(|value| value.as_str()),
        Some(env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(
        copyable_payload.get("provider").and_then(|value| value.as_str()),
        Some("goto_definition")
    );
    assert_eq!(
        copyable_payload.get("support_tier_link").and_then(|value| value.as_str()),
        Some("docs/project/status/SUPPORT_TIERS.md#claim-rows")
    );
    let copyable_position = copyable_payload
        .get("request_position")
        .and_then(|value| value.as_object())
        .ok_or("missing copyable request_position")?;
    assert_eq!(copyable_position.get("uri_scheme").and_then(|value| value.as_str()), Some("file"));
    assert_eq!(copyable_position.get("line").and_then(|value| value.as_u64()), Some(7));
    assert_eq!(copyable_position.get("character").and_then(|value| value.as_u64()), Some(2));

    Ok(())
}
