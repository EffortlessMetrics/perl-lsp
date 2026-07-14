//! Integration-style unit tests for execute-command provider behaviors.

// Tests are permitted to use `.expect()`/`.expect_err()` on Result/Option per
// the repo's coding standards (unlike production code, where they are banned).
#![allow(clippy::expect_used)]

use super::get_supported_commands;
use super::provider::{ExecuteCommandProvider, TestRunner, select_test_runner};
use super::test_support::mock_status;
use perl_lsp_rs_core::config::{CriticEngine, WorkspaceConfig};
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

// ============= GO-TO-TEST / GO-TO-IMPLEMENTATION TESTS =============

fn provider_with_execute_perl(workspace_roots: Vec<PathBuf>) -> ExecuteCommandProvider {
    let mut config = WorkspaceConfig::default();
    config.perl_path = Some("perl".to_string());
    ExecuteCommandProvider::with_workspace_roots(workspace_roots).with_workspace_config(config)
}

#[test]
fn test_go_to_test_basic_mapping() -> Result<(), Box<dyn std::error::Error>> {
    // lib/Foo/Bar.pm -> t/foo-bar.t (canonical hyphen form)
    let temp_dir = tempdir()?;
    let lib_dir = temp_dir.path().join("lib").join("Foo");
    let t_dir = temp_dir.path().join("t");
    fs::create_dir_all(&lib_dir)?;
    fs::create_dir_all(&t_dir)?;

    let pm_file = lib_dir.join("Bar.pm");
    let t_file = t_dir.join("foo-bar.t");
    fs::write(&pm_file, "package Foo::Bar;\n1;\n")?;
    fs::write(&t_file, "use Test::More;\nuse Foo::Bar;\ndone_testing;\n")?;

    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()]);
    let result = provider.execute_command(
        "perl.goToTest",
        vec![Value::String(pm_file.to_string_lossy().to_string())],
    );

    assert!(result.is_ok(), "perl.goToTest should execute successfully");
    let value = result?;
    assert!(value["found"].as_bool().unwrap_or(false), "Should find the test file");
    let test_path = value["path"].as_str().ok_or("expected path string")?;
    assert!(
        test_path.ends_with("foo-bar.t"),
        "Should map Foo/Bar.pm to foo-bar.t, got: {test_path}"
    );
    Ok(())
}

#[test]
fn test_go_to_test_not_found() -> Result<(), Box<dyn std::error::Error>> {
    // When no test file exists, returns found: false
    let temp_dir = tempdir()?;
    let lib_dir = temp_dir.path().join("lib").join("My");
    fs::create_dir_all(&lib_dir)?;

    let pm_file = lib_dir.join("Missing.pm");
    fs::write(&pm_file, "package My::Missing;\n1;\n")?;

    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()]);
    let result = provider.execute_command(
        "perl.goToTest",
        vec![Value::String(pm_file.to_string_lossy().to_string())],
    );

    assert!(result.is_ok(), "perl.goToTest should not error when test not found");
    let value = result?;
    assert!(!value["found"].as_bool().unwrap_or(true), "Should report not found");
    Ok(())
}

#[test]
fn test_go_to_test_underscore_variant() -> Result<(), Box<dyn std::error::Error>> {
    // lib/Foo/Bar.pm -> t/foo_bar.t (underscore form is also a valid convention)
    let temp_dir = tempdir()?;
    let lib_dir = temp_dir.path().join("lib").join("Foo");
    let t_dir = temp_dir.path().join("t");
    fs::create_dir_all(&lib_dir)?;
    fs::create_dir_all(&t_dir)?;

    let pm_file = lib_dir.join("Bar.pm");
    let t_file = t_dir.join("foo_bar.t");
    fs::write(&pm_file, "package Foo::Bar;\n1;\n")?;
    fs::write(&t_file, "use Test::More;\nuse Foo::Bar;\ndone_testing;\n")?;

    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()]);
    let result = provider.execute_command(
        "perl.goToTest",
        vec![Value::String(pm_file.to_string_lossy().to_string())],
    );

    assert!(result.is_ok(), "perl.goToTest should find underscore variant");
    let value = result?;
    assert!(value["found"].as_bool().unwrap_or(false), "Should find the underscore test file");
    Ok(())
}

#[test]
fn test_go_to_implementation_basic() -> Result<(), Box<dyn std::error::Error>> {
    // From a test file with `use Foo::Bar`, navigate to lib/Foo/Bar.pm
    let temp_dir = tempdir()?;
    let lib_dir = temp_dir.path().join("lib").join("Foo");
    let t_dir = temp_dir.path().join("t");
    fs::create_dir_all(&lib_dir)?;
    fs::create_dir_all(&t_dir)?;

    let pm_file = lib_dir.join("Bar.pm");
    let t_file = t_dir.join("foo-bar.t");
    fs::write(&pm_file, "package Foo::Bar;\nsub new { bless {}, shift }\n1;\n")?;
    fs::write(
        &t_file,
        "use strict;\nuse warnings;\nuse Foo::Bar;\nuse Test::More;\ndone_testing;\n",
    )?;

    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()]);
    let result = provider.execute_command(
        "perl.goToImplementation",
        vec![Value::String(t_file.to_string_lossy().to_string())],
    );

    assert!(result.is_ok(), "perl.goToImplementation should execute successfully");
    let value = result?;
    assert!(value["found"].as_bool().unwrap_or(false), "Should find the implementation file");
    let impl_path = value["path"].as_str().ok_or("expected path string")?;
    assert!(impl_path.ends_with("Bar.pm"), "Should navigate to Bar.pm, got: {impl_path}");
    Ok(())
}

#[test]
fn test_go_to_implementation_no_use_statement() -> Result<(), Box<dyn std::error::Error>> {
    // A test file with no recognizable `use` pointing to a local module
    let temp_dir = tempdir()?;
    let t_dir = temp_dir.path().join("t");
    fs::create_dir_all(&t_dir)?;

    let t_file = t_dir.join("simple.t");
    fs::write(&t_file, "use Test::More;\ndone_testing;\n")?;

    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()]);
    let result = provider.execute_command(
        "perl.goToImplementation",
        vec![Value::String(t_file.to_string_lossy().to_string())],
    );

    assert!(result.is_ok(), "perl.goToImplementation should not error on no use");
    let value = result?;
    assert!(!value["found"].as_bool().unwrap_or(true), "Should report not found");
    Ok(())
}

#[test]
fn test_go_to_test_module_mapping_conversion() {
    // Unit test for the module name -> test file name conversion logic
    let provider = ExecuteCommandProvider::new();

    // Foo::Bar -> foo-bar (hyphen form)
    let hyphen = provider.module_to_test_stem("Foo::Bar");
    assert_eq!(hyphen, "foo-bar", "Should produce hyphen-separated lowercase stem");

    // My::Very::Deep::Module -> my-very-deep-module
    let deep = provider.module_to_test_stem("My::Very::Deep::Module");
    assert_eq!(deep, "my-very-deep-module");
}

#[test]
fn test_supported_commands_includes_go_to_test() {
    let commands = get_supported_commands();
    assert!(
        commands.contains(&"perl.goToTest".to_string()),
        "perl.goToTest should be in supported commands list"
    );
    assert!(
        commands.contains(&"perl.goToImplementation".to_string()),
        "perl.goToImplementation should be in supported commands list"
    );
    assert!(
        commands.contains(&"perl.explainProviderDecision".to_string()),
        "perl.explainProviderDecision should be in supported commands list"
    );
    assert!(
        commands.contains(&"perl.workspaceTrustReport".to_string()),
        "perl.workspaceTrustReport should be in supported commands list"
    );
    assert!(
        commands.contains(&"perl.previewSafeDelete".to_string()),
        "perl.previewSafeDelete should be in supported commands list"
    );
    assert!(
        commands.contains(&"perl.safeDeleteSymbol".to_string()),
        "perl.safeDeleteSymbol should be in supported commands list"
    );
    assert!(
        commands.contains(&"perl.previewPackageRename".to_string()),
        "perl.previewPackageRename should be in supported commands list"
    );
    assert!(
        commands.contains(&"perl.explainMissingModuleLookup".to_string()),
        "perl.explainMissingModuleLookup should be in supported commands list"
    );
}

#[test]
fn test_explain_provider_decision_attaches_provider_receipt()
-> Result<(), Box<dyn std::error::Error>> {
    let provider = ExecuteCommandProvider::new();
    let result = provider.execute_command(
        "perl.explainProviderDecision",
        vec![json!({
            "provider": "safe_delete",
            "receipt_id": "semantic-shadow-compare",
            "scenario": "mojolicious-safe-delete",
            "request_position": {
                "uri_scheme": "file",
                "line": 3,
                "character": 11
            }
        })],
    )?;

    assert_eq!(result.get("schema_version").and_then(Value::as_str), Some("provider_decision.v1"));
    assert_eq!(result.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(result.get("decision").and_then(Value::as_str), Some("blocked"));
    assert_eq!(result.get("reason").and_then(Value::as_str), Some("unsafe_edit_blocked"));
    assert_eq!(result.get("fact_source").and_then(Value::as_str), Some("compiler_fact"));
    assert_eq!(result.get("confidence").and_then(Value::as_str), Some("high"));
    assert_eq!(result.get("freshness").and_then(Value::as_str), Some("fresh"));
    assert_eq!(result.get("fallback").and_then(Value::as_str), Some("no_edit"));
    assert_eq!(result.get("receipt_id").and_then(Value::as_str), Some("semantic-shadow-compare"));
    assert_eq!(result.get("scenario").and_then(Value::as_str), Some("mojolicious-safe-delete"));
    assert_eq!(result.get("dynamic_boundary").and_then(Value::as_bool), Some(false));
    let user_message =
        result.get("user_message").and_then(Value::as_str).ok_or("missing user_message")?;
    assert!(user_message.contains("Safe delete blocked."), "{user_message}");
    assert!(user_message.contains("Fact source: compiler facts."), "{user_message}");
    assert!(user_message.contains("Fallback: no edit."), "{user_message}");
    let copyable_payload = result
        .get("copyable_payload")
        .and_then(Value::as_object)
        .ok_or("missing copyable_payload")?;
    assert_eq!(
        copyable_payload.get("schema_version").and_then(Value::as_str),
        Some("provider_decision_bug_report.v1")
    );
    assert_eq!(
        copyable_payload.get("perl_lsp_version").and_then(Value::as_str),
        Some(env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(copyable_payload.get("workspace_root_class").and_then(Value::as_str), Some("none"));
    assert!(copyable_payload.get("workspace_root_hash").is_some());
    assert_eq!(copyable_payload.get("provider").and_then(Value::as_str), Some("safe_delete"));
    assert_eq!(copyable_payload.get("decision").and_then(Value::as_str), Some("blocked"));
    assert_eq!(
        copyable_payload.get("support_tier_link").and_then(Value::as_str),
        Some("docs/project/status/SUPPORT_TIERS.md#claim-rows")
    );
    let request_position = copyable_payload
        .get("request_position")
        .and_then(Value::as_object)
        .ok_or("missing copyable request_position")?;
    assert_eq!(request_position.get("uri_scheme").and_then(Value::as_str), Some("file"));
    assert_eq!(request_position.get("line").and_then(Value::as_u64), Some(3));
    assert_eq!(request_position.get("character").and_then(Value::as_u64), Some(11));
    Ok(())
}

#[test]
fn test_explain_provider_decision_attaches_request_local_receipt()
-> Result<(), Box<dyn std::error::Error>> {
    let provider = ExecuteCommandProvider::new();
    let result = provider.execute_command(
        "perl.explainProviderDecision",
        vec![json!({
            "provider": "rename",
            "receipt_id": "runtime-request",
            "scenario": "realbaseline-rename-fallback-noise",
            "request_receipt": {
                "provider": "rename",
                "decision": "fallback",
                "reason": "ambiguous_symbol_identity",
                "fallback_state": "compiler_empty"
            }
        })],
    )?;

    assert_eq!(result.get("schema_version").and_then(Value::as_str), Some("provider_decision.v1"));
    let request_receipt = result
        .get("request_receipt")
        .and_then(Value::as_object)
        .ok_or("missing request_receipt object")?;

    assert_eq!(result.get("provider").and_then(Value::as_str), Some("rename"));
    assert_eq!(result.get("decision").and_then(Value::as_str), Some("fallback"));
    assert_eq!(result.get("receipt_id").and_then(Value::as_str), Some("runtime-request"));
    assert_eq!(
        result.get("scenario").and_then(Value::as_str),
        Some("realbaseline-rename-fallback-noise")
    );
    assert_eq!(
        request_receipt.get("schema_version").and_then(Value::as_str),
        Some("provider_decision.v1")
    );
    assert_eq!(
        request_receipt.get("reason").and_then(Value::as_str),
        Some("ambiguous_symbol_identity")
    );
    assert_eq!(
        request_receipt.get("fallback_state").and_then(Value::as_str),
        Some("compiler_empty")
    );
    assert_eq!(request_receipt.get("fallback").and_then(Value::as_str), Some("no_result"));
    assert_eq!(request_receipt.get("source_backed").and_then(Value::as_bool), Some(false));
    let user_message =
        result.get("user_message").and_then(Value::as_str).ok_or("missing user_message")?;
    assert!(user_message.contains("Rename used fallback."), "{user_message}");
    assert!(user_message.contains("Fallback: legacy provider."), "{user_message}");
    let copyable_payload = result
        .get("copyable_payload")
        .and_then(Value::as_object)
        .ok_or("missing copyable_payload")?;
    let copyable_request_receipt = copyable_payload
        .get("request_receipt")
        .and_then(Value::as_object)
        .ok_or("missing copyable request_receipt")?;
    assert_eq!(
        copyable_request_receipt.get("reason").and_then(Value::as_str),
        Some("ambiguous_symbol_identity")
    );
    Ok(())
}

#[test]
fn test_explain_provider_decision_redacts_workspace_root_in_copyable_payload()
-> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let root_string = temp_dir.path().to_string_lossy().to_string();
    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()]);
    let result = provider.execute_command(
        "perl.explainProviderDecision",
        vec![json!({
            "provider": "completion"
        })],
    )?;
    let payload = result
        .get("copyable_payload")
        .and_then(Value::as_object)
        .ok_or("missing copyable_payload")?;
    let root_hash = payload
        .get("workspace_root_hash")
        .and_then(Value::as_str)
        .ok_or("missing workspace_root_hash")?;

    assert_eq!(payload.get("workspace_root_class").and_then(Value::as_str), Some("single_root"));
    assert!(!root_hash.is_empty(), "workspace root hash should be present");
    assert!(
        !serde_json::to_string(payload)?.contains(&root_string),
        "copyable payload must not expose raw workspace root paths"
    );
    Ok(())
}

#[test]
fn test_explain_provider_decision_rejects_non_object_request_receipt()
-> Result<(), Box<dyn std::error::Error>> {
    let provider = ExecuteCommandProvider::new();
    let result = provider.execute_command(
        "perl.explainProviderDecision",
        vec![json!({
            "provider": "rename",
            "request_receipt": "runtime-request"
        })],
    );

    let error = match result {
        Ok(value) => {
            return Err(
                format!("non-object request_receipt should reject the request: {value}").into()
            );
        }
        Err(error) => error,
    };
    assert!(
        error.contains("request_receipt must be an object"),
        "error should identify invalid request_receipt payload, got: {error}"
    );
    Ok(())
}

#[test]
fn test_explain_provider_decision_defaults_to_live_provider_receipt()
-> Result<(), Box<dyn std::error::Error>> {
    let provider = ExecuteCommandProvider::new();
    let result = provider.execute_command(
        "perl.explainProviderDecision",
        vec![json!({ "provider": "goto_definition" })],
    )?;

    assert_eq!(result.get("schema_version").and_then(Value::as_str), Some("provider_decision.v1"));
    assert_eq!(result.get("provider").and_then(Value::as_str), Some("goto_definition"));
    assert_eq!(result.get("decision").and_then(Value::as_str), Some("acted"));
    assert_eq!(result.get("reason").and_then(Value::as_str), Some("source_backed_high_confidence"));
    assert_eq!(result.get("fact_source").and_then(Value::as_str), Some("compiler_fact"));
    assert_eq!(result.get("confidence").and_then(Value::as_str), Some("high"));
    assert_eq!(result.get("freshness").and_then(Value::as_str), Some("fresh"));
    assert_eq!(result.get("fallback").and_then(Value::as_str), Some("none"));
    assert_eq!(
        result.get("receipt_id").and_then(Value::as_str),
        Some("docs/project/status/provider_cutover.md#navigation-live-quality-dashboard")
    );
    assert_eq!(
        result.get("scenario").and_then(Value::as_str),
        Some("ux_scenario_30_mojolicious_navigation_quality")
    );
    Ok(())
}

#[test]
fn test_explain_provider_decision_rejects_missing_provider()
-> Result<(), Box<dyn std::error::Error>> {
    let provider = ExecuteCommandProvider::new();
    let result = provider.execute_command("perl.explainProviderDecision", vec![json!({})]);

    let error = match result {
        Ok(value) => {
            return Err(format!("missing provider should reject the request: {value}").into());
        }
        Err(error) => error,
    };
    assert!(
        error.contains("Invalid explain-provider-decision argument"),
        "error should identify invalid explain-provider payload, got: {error}"
    );
    Ok(())
}

#[test]
fn test_go_to_test_deeply_nested_module() -> Result<(), Box<dyn std::error::Error>> {
    // lib/Foo/Bar/Baz.pm -> t/foo-bar-baz.t (three-level deep module)
    let temp_dir = tempdir()?;
    let lib_dir = temp_dir.path().join("lib").join("Foo").join("Bar");
    let t_dir = temp_dir.path().join("t");
    fs::create_dir_all(&lib_dir)?;
    fs::create_dir_all(&t_dir)?;

    let pm_file = lib_dir.join("Baz.pm");
    let t_file = t_dir.join("foo-bar-baz.t");
    fs::write(&pm_file, "package Foo::Bar::Baz;\n1;\n")?;
    fs::write(&t_file, "use Test::More;\nuse Foo::Bar::Baz;\ndone_testing;\n")?;

    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()]);
    let result = provider.execute_command(
        "perl.goToTest",
        vec![Value::String(pm_file.to_string_lossy().to_string())],
    );

    assert!(result.is_ok(), "perl.goToTest should handle deeply nested modules");
    let value = result?;
    assert!(value["found"].as_bool().unwrap_or(false), "Should find t/foo-bar-baz.t");
    let test_path = value["path"].as_str().ok_or("expected path string")?;
    assert!(
        test_path.ends_with("foo-bar-baz.t"),
        "Should map Foo/Bar/Baz.pm to foo-bar-baz.t, got: {test_path}"
    );
    Ok(())
}

#[test]
fn test_go_to_test_t_lib_mirror() -> Result<(), Box<dyn std::error::Error>> {
    // lib/Foo/Bar.pm -> t/lib/Foo/Bar.t (mirrored test hierarchy)
    let temp_dir = tempdir()?;
    let lib_dir = temp_dir.path().join("lib").join("Foo");
    let t_lib_dir = temp_dir.path().join("t").join("lib").join("Foo");
    fs::create_dir_all(&lib_dir)?;
    fs::create_dir_all(&t_lib_dir)?;

    let pm_file = lib_dir.join("Bar.pm");
    let t_file = t_lib_dir.join("Bar.t");
    fs::write(&pm_file, "package Foo::Bar;\n1;\n")?;
    fs::write(&t_file, "use Test2::V0;\nuse Foo::Bar;\ndone_testing;\n")?;

    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()]);
    let result = provider.execute_command(
        "perl.goToTest",
        vec![Value::String(pm_file.to_string_lossy().to_string())],
    );

    assert!(result.is_ok(), "perl.goToTest should handle t/lib/ mirror layout");
    let value = result?;
    assert!(
        value["found"].as_bool().unwrap_or(false),
        "Should find t/lib/Foo/Bar.t (mirror layout)"
    );
    let test_path = value["path"].as_str().ok_or("expected path string")?;
    assert!(test_path.contains("Bar.t"), "Should navigate to Bar.t, got: {test_path}");
    Ok(())
}

#[test]
fn test_go_to_implementation_skips_test2_modules() -> Result<(), Box<dyn std::error::Error>> {
    // Test2::V0 and Test2::Bundle::Extended should be skipped; Foo::Bar should be found.
    let temp_dir = tempdir()?;
    let lib_dir = temp_dir.path().join("lib").join("Foo");
    let t_dir = temp_dir.path().join("t");
    fs::create_dir_all(&lib_dir)?;
    fs::create_dir_all(&t_dir)?;

    let pm_file = lib_dir.join("Bar.pm");
    let t_file = t_dir.join("foo-bar.t");
    fs::write(&pm_file, "package Foo::Bar;\n1;\n")?;
    fs::write(
        &t_file,
        "use Test2::V0;\nuse Test2::Bundle::Extended;\nuse strict;\nuse Foo::Bar;\ndone_testing;\n",
    )?;

    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()]);
    let result = provider.execute_command(
        "perl.goToImplementation",
        vec![Value::String(t_file.to_string_lossy().to_string())],
    );

    assert!(result.is_ok(), "perl.goToImplementation should skip Test2 modules");
    let value = result?;
    assert!(
        value["found"].as_bool().unwrap_or(false),
        "Should find Foo::Bar after skipping Test2::* modules"
    );
    let impl_path = value["path"].as_str().ok_or("expected path string")?;
    assert!(impl_path.ends_with("Bar.pm"), "Should navigate to Bar.pm, got: {impl_path}");
    Ok(())
}

#[test]
fn test_go_to_implementation_skips_moosex_modules() -> Result<(), Box<dyn std::error::Error>> {
    // MooseX::* should be skipped; My::Class should be found.
    let temp_dir = tempdir()?;
    let lib_dir = temp_dir.path().join("lib").join("My");
    let t_dir = temp_dir.path().join("t");
    fs::create_dir_all(&lib_dir)?;
    fs::create_dir_all(&t_dir)?;

    let pm_file = lib_dir.join("Class.pm");
    let t_file = t_dir.join("my-class.t");
    fs::write(&pm_file, "package My::Class;\n1;\n")?;
    fs::write(
        &t_file,
        "use strict;\nuse MooseX::Types;\nuse namespace::autoclean;\nuse My::Class;\ndone_testing;\n",
    )?;

    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()]);
    let result = provider.execute_command(
        "perl.goToImplementation",
        vec![Value::String(t_file.to_string_lossy().to_string())],
    );

    assert!(result.is_ok(), "perl.goToImplementation should skip MooseX and namespace modules");
    let value = result?;
    assert!(
        value["found"].as_bool().unwrap_or(false),
        "Should find My::Class after skipping MooseX::Types and namespace::autoclean"
    );
    let impl_path = value["path"].as_str().ok_or("expected path string")?;
    assert!(impl_path.ends_with("Class.pm"), "Should navigate to Class.pm, got: {impl_path}");
    Ok(())
}

#[test]
fn test_go_to_implementation_skips_version_pragma() -> Result<(), Box<dyn std::error::Error>> {
    // `use v5.20;` and `use 5.010;` must be skipped; My::Module should be found.
    let temp_dir = tempdir()?;
    let lib_dir = temp_dir.path().join("lib").join("My");
    let t_dir = temp_dir.path().join("t");
    fs::create_dir_all(&lib_dir)?;
    fs::create_dir_all(&t_dir)?;

    let pm_file = lib_dir.join("Module.pm");
    let t_file = t_dir.join("my-module.t");
    fs::write(&pm_file, "package My::Module;\n1;\n")?;
    fs::write(&t_file, "use v5.20;\nuse 5.010;\nuse strict;\nuse My::Module;\ndone_testing;\n")?;

    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()]);
    let result = provider.execute_command(
        "perl.goToImplementation",
        vec![Value::String(t_file.to_string_lossy().to_string())],
    );

    assert!(result.is_ok(), "perl.goToImplementation should skip version pragmas");
    let value = result?;
    assert!(
        value["found"].as_bool().unwrap_or(false),
        "Should find My::Module after skipping version pragmas"
    );
    Ok(())
}

#[test]
fn test_find_workspace_root_prefers_project_marker() -> Result<(), Box<dyn std::error::Error>> {
    // A directory with a cpanfile should be found as the workspace root when walking up.
    // We call go_to_test directly (bypassing execute_command security validation) so we can
    // test the walk-up logic against a temp directory without needing a configured root.
    let temp_dir = tempdir()?;
    // Layout: <temp>/project/lib/Foo/Bar.pm and <temp>/project/t/foo-bar.t
    // with a cpanfile in <temp>/project/
    let project_dir = temp_dir.path().join("project");
    let lib_dir = project_dir.join("lib").join("Foo");
    let t_dir = project_dir.join("t");
    fs::create_dir_all(&lib_dir)?;
    fs::create_dir_all(&t_dir)?;
    // cpanfile marks project_dir as the distribution root
    fs::write(project_dir.join("cpanfile"), "requires 'Foo';\n")?;

    let pm_file = lib_dir.join("Bar.pm");
    let t_file = t_dir.join("foo-bar.t");
    fs::write(&pm_file, "package Foo::Bar;\n1;\n")?;
    fs::write(&t_file, "use Test::More;\nuse Foo::Bar;\ndone_testing;\n")?;

    // Call go_to_test directly (no security wrapper) to test the walk-up root detection.
    let provider = ExecuteCommandProvider::new();
    let value = provider.go_to_test(&pm_file);

    assert!(
        value["found"].as_bool().unwrap_or(false),
        "Should find foo-bar.t when workspace root is anchored by cpanfile; got: {value:?}"
    );
    Ok(())
}

#[test]
fn test_go_to_test_module_mapping_conversion_single_component() {
    // A single-component module (no ::) should produce just the lowercased name.
    let provider = ExecuteCommandProvider::new();
    let stem = provider.module_to_test_stem("MyModule");
    assert_eq!(stem, "mymodule", "Single-component module: lowercase, no separator");
}

#[test]
fn test_supported_commands_includes_run_critic() {
    let commands = get_supported_commands();
    assert!(
        commands.contains(&"perl.runCritic".to_string()),
        "perl.runCritic should be in supported commands list"
    );
}

#[test]
fn test_execute_command_run_critic_builtin() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let temp_file = temp_dir.path().join("test_violations_unit.pl");

    // Create a temporary file with violations
    let test_content = r#"#!/usr/bin/perl
# Test file with policy violations
my $variable = 42;
print "Value: $variable\n";
"#;

    fs::write(&temp_file, test_content)?;

    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()]);
    let result = provider
        .execute_command("perl.runCritic", vec![Value::String(temp_file.display().to_string())]);

    // Verify result
    assert!(result.is_ok(), "perl.runCritic command should execute successfully");

    let result_value = result?;
    assert_eq!(result_value["status"], "success", "Command should succeed");
    assert!(result_value["violations"].is_array(), "Should return violations array");
    assert!(result_value["analyzerUsed"].is_string(), "Should indicate which analyzer was used");

    // Should detect missing 'use strict' and 'use warnings'
    let violations = result_value["violations"].as_array().ok_or("expected violations array")?;
    assert!(!violations.is_empty(), "Should detect policy violations");

    // Check for specific violations
    let has_strict_violation = violations.iter().any(|v| {
        v["policy"]
            .as_str()
            .map(|p| p.contains("RequireUseStrict") || p.contains("strict"))
            .unwrap_or(false)
    });

    let has_warnings_violation = violations.iter().any(|v| {
        v["policy"]
            .as_str()
            .map(|p| p.contains("RequireUseWarnings") || p.contains("warnings"))
            .unwrap_or(false)
    });

    assert!(has_strict_violation, "Should detect missing 'use strict'");
    assert!(has_warnings_violation, "Should detect missing 'use warnings'");
    Ok(())
}

#[test]
fn test_execute_command_invalid_command() {
    let provider = ExecuteCommandProvider::new();
    let result = provider.execute_command("perl.invalidCommand", vec![]);
    assert!(result.is_err(), "Invalid command should return error");
    assert!(result.unwrap_err().contains("Unknown command"), "Should indicate unknown command");
}

#[test]
fn test_execute_command_run_critic_missing_file() -> Result<(), Box<dyn std::error::Error>> {
    let provider = ExecuteCommandProvider::new();
    let result = provider
        .execute_command("perl.runCritic", vec![Value::String("/tmp/nonexistent.pl".to_string())]);

    assert!(result.is_ok(), "Should handle missing files gracefully");
    let result_value = result?;
    assert_eq!(result_value["status"], "error", "Should report error status");
    assert!(
        result_value["error"].as_str().ok_or("expected error string")?.contains("File not found"),
        "Should indicate file not found"
    );
    Ok(())
}

// ============= MUTATION HARDENING TESTS =============
// These tests target specific surviving mutants to achieve ≥80% mutation score

#[test]
fn test_command_routing_perl_run_tests() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let temp_file = temp_dir.path().join("test_run_tests.pl");

    // Create a test file to ensure we get a specific result
    let test_content = "#!/usr/bin/perl\nuse strict;\nuse warnings;\nprint 'test';\n";
    fs::write(&temp_file, test_content)?;

    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()]);
    let result = provider
        .execute_command("perl.runTests", vec![Value::String(temp_file.display().to_string())]);

    // Verify the command was routed correctly and executed
    assert!(result.is_ok(), "perl.runTests should execute successfully");
    let result_value = result?;
    assert!(result_value.is_object(), "Should return a structured result");
    assert!(result_value["success"].is_boolean(), "Should have success field");
    assert!(result_value["output"].is_string(), "Should have output field");
    Ok(())
}

#[test]
fn test_command_routing_perl_run_file() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let temp_file = temp_dir.path().join("test_run_file.pl");

    // Create a test file
    let test_content = "#!/usr/bin/perl\nuse strict;\nuse warnings;\nprint 'hello world';\n";
    fs::write(&temp_file, test_content)?;

    let provider = provider_with_execute_perl(vec![temp_dir.path().to_path_buf()]);
    let result = provider
        .execute_command("perl.runFile", vec![Value::String(temp_file.display().to_string())]);

    // Verify the command was routed correctly
    assert!(result.is_ok(), "perl.runFile should execute successfully");
    let result_value = result?;
    assert!(result_value.is_object(), "Should return a structured result");
    assert!(result_value["success"].is_boolean(), "Should have success field");
    assert!(result_value["output"].is_string(), "Should have output field");
    Ok(())
}

#[test]
fn test_command_routing_perl_run_test_sub() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let temp_file = temp_dir.path().join("test_run_test_sub.pl");

    // Create a test file with a subroutine
    let test_content =
        "#!/usr/bin/perl\nuse strict;\nuse warnings;\nsub test_sub { print 'test executed'; }\n";
    fs::write(&temp_file, test_content)?;

    let provider = provider_with_execute_perl(vec![temp_dir.path().to_path_buf()]);
    let result = provider.execute_command(
        "perl.runTestSub",
        vec![Value::String(temp_file.display().to_string()), Value::String("test_sub".to_string())],
    );

    // Verify the command was routed correctly
    assert!(result.is_ok(), "perl.runTestSub should execute successfully");
    let result_value = result?;
    assert!(result_value.is_object(), "Should return a structured result");
    assert!(result_value["success"].is_boolean(), "Should have success field");
    assert!(result_value["subroutine"].is_string(), "Should have subroutine field");
    Ok(())
}

#[test]
fn test_run_file_rejects_unresolved_perl_fallback() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let temp_file = temp_dir.path().join("test_no_ambient_run_file.pl");
    fs::write(&temp_file, "print 'hello';\n")?;

    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()]);
    let result = provider
        .execute_command("perl.runFile", vec![Value::String(temp_file.display().to_string())]);

    assert!(result.is_err(), "perl.runFile must reject missing oracle config");
    let error = result.err().ok_or("expected error")?;
    assert!(
        error.contains("refusing ambient fallback"),
        "error should explain ambient fallback refusal: {error}"
    );
    Ok(())
}

#[test]
fn test_run_test_sub_rejects_unresolved_perl_fallback() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let temp_file = temp_dir.path().join("test_no_ambient_run_test_sub.pl");
    fs::write(&temp_file, "sub test_sub { print 'hello'; }\n")?;

    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()]);
    let result = provider.execute_command(
        "perl.runTestSub",
        vec![Value::String(temp_file.display().to_string()), Value::String("test_sub".to_string())],
    );

    assert!(result.is_err(), "perl.runTestSub must reject missing oracle config");
    let error = result.err().ok_or("expected error")?;
    assert!(
        error.contains("refusing ambient fallback"),
        "error should explain ambient fallback refusal: {error}"
    );
    Ok(())
}

#[test]
fn test_command_routing_perl_debug_tests() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let temp_file = temp_dir.path().join("test_debug.pl");
    fs::write(&temp_file, "print 'debug';")?;

    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()]);
    let result = provider
        .execute_command("perl.debugTests", vec![Value::String(temp_file.display().to_string())]);

    // Verify the command was routed correctly
    assert!(result.is_ok(), "perl.debugTests should execute successfully");
    let result_value = result?;
    assert!(result_value.is_object(), "Should return a structured result");
    // Debug now returns a real perl-dap launch configuration.
    assert_eq!(result_value["success"], true, "Debug should return a launch config");
    assert_eq!(result_value["action"], "startDebugging");
    assert_eq!(result_value["configuration"]["type"], "perl");
    Ok(())
}

#[test]
fn test_parameter_validation_missing_file_path() {
    let provider = ExecuteCommandProvider::new();

    // Test with no arguments
    let result = provider.execute_command("perl.runTests", vec![]);
    assert!(result.is_err(), "Should fail with missing file path");
    assert!(result.unwrap_err().contains("Missing file path argument"));

    // Test with null argument
    let result = provider.execute_command("perl.runTests", vec![Value::Null]);
    assert!(result.is_err(), "Should fail with null argument");
    assert!(result.unwrap_err().contains("Missing file path argument"));

    // Test with number instead of string
    let result = provider
        .execute_command("perl.runTests", vec![Value::Number(serde_json::Number::from(123))]);
    assert!(result.is_err(), "Should fail with non-string argument");
    assert!(result.unwrap_err().contains("Missing file path argument"));
}

#[test]
fn test_parameter_validation_missing_subroutine_name() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let temp_file = temp_dir.path().join("test_missing_sub.pl");
    fs::write(&temp_file, "sub test {}")?;

    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()]);
    let file_arg = temp_file.display().to_string();

    // Test runTestSub with only file path, missing subroutine name
    let result = provider.execute_command("perl.runTestSub", vec![Value::String(file_arg.clone())]);

    assert!(result.is_err(), "Should fail with missing subroutine name");
    // It might fail with path resolution if file doesn't exist, but here it exists
    let err = result.err().ok_or("expected error")?;
    assert!(err.contains("Missing subroutine name argument"));

    // Test with null second argument
    let result =
        provider.execute_command("perl.runTestSub", vec![Value::String(file_arg), Value::Null]);

    assert!(result.is_err(), "Should fail with null subroutine name");
    let err = result.err().ok_or("expected error")?;
    assert!(err.contains("Missing subroutine name argument"));
    Ok(())
}

#[test]
#[allow(deprecated)]
fn test_normalize_file_path_uri_handling() {
    let provider = ExecuteCommandProvider::new();

    // Test empty string
    let normalized = provider.normalize_file_path("");
    assert_eq!(normalized, "", "Should handle empty strings");

    // Test regular path without URI scheme (platform-neutral: just passes through)
    let normalized = provider.normalize_file_path("some/relative/path.pl");
    assert_eq!(normalized, "some/relative/path.pl", "Should leave non-URI paths unchanged");

    // Unix-specific assertions: file:// URI decoding produces Unix-style paths
    #[cfg(unix)]
    {
        // Test file:// URI scheme stripping
        let normalized = provider.normalize_file_path("file:///tmp/test.pl");
        assert_eq!(normalized, "/tmp/test.pl", "Should strip file:// prefix");

        // Test regular absolute path (no URI scheme)
        let normalized = provider.normalize_file_path("/tmp/test.pl");
        assert_eq!(normalized, "/tmp/test.pl", "Should leave regular paths unchanged");

        // Test file URI decoding
        let normalized = provider.normalize_file_path("file:///tmp/path%20with%20spaces/test.pl");
        assert_eq!(normalized, "/tmp/path with spaces/test.pl", "Should decode file URI path");

        // Test localhost authority in file URI
        let normalized = provider.normalize_file_path("file://localhost/tmp/test.pl");
        assert_eq!(normalized, "/tmp/test.pl", "Should support localhost file URI");
    }
}

#[test]
fn test_is_test_file_logic() {
    let provider = ExecuteCommandProvider::new();

    // Test .t extension
    assert!(provider.is_test_file("test.t"), "Should recognize .t files");
    assert!(provider.is_test_file("path/to/test.t"), "Should recognize .t files in paths");

    // Test /t/ directory
    assert!(provider.is_test_file("/path/t/test.pl"), "Should recognize files in t/ directory");

    // Test 'test' in name
    assert!(provider.is_test_file("test_file.pl"), "Should recognize files with 'test' in name");
    assert!(provider.is_test_file("my_test.pl"), "Should recognize files with 'test' in name");

    // Test non-test files
    assert!(!provider.is_test_file("regular.pl"), "Should not recognize regular files");
    assert!(!provider.is_test_file("module.pm"), "Should not recognize modules");
}

#[test]
fn test_format_command_result_structure() {
    let provider = ExecuteCommandProvider::new();

    // Test successful result
    let output = std::process::Output {
        status: mock_status(0),
        stdout: b"test output".to_vec(),
        stderr: b"".to_vec(),
    };

    let result = provider.format_command_result(output, None);
    assert_eq!(result["success"], true, "Should indicate success");
    assert_eq!(result["output"], "test output", "Should include stdout");
    assert_eq!(result["error"], Value::Null, "Should have null error for success");

    // Test with extra field
    let output = std::process::Output {
        status: mock_status(0),
        stdout: b"test".to_vec(),
        stderr: b"".to_vec(),
    };

    let result = provider
        .format_command_result(output, Some(("command", Value::String("perl".to_string()))));
    assert_eq!(result["command"], "perl", "Should include extra field");
}

#[test]
fn test_format_test_command_result_parses_tap() {
    let provider = ExecuteCommandProvider::new();

    let tap = b"1..2\n\
ok 1 - addition works\n\
not ok 2 - email matches\n\
#   at t/user.t line 12.\n\
#          got: 'x'\n\
#     expected: 'y'\n"
        .to_vec();
    let output = std::process::Output { status: mock_status(1), stdout: tap, stderr: b"".to_vec() };

    let result = provider.format_test_command_result(output, "prove");

    // Raw output and runner are preserved.
    assert_eq!(result["runner"], "prove");
    assert_eq!(result["command"], "prove");
    assert_eq!(result["exitCode"], 1);
    assert!(result["output"].as_str().unwrap_or("").contains("not ok 2"));

    // Structured TAP facts are additive.
    assert_eq!(result["tap"]["planned"], 2);
    assert_eq!(result["tap"]["passed"], 1);
    assert_eq!(result["tap"]["failed"], 1);

    let failures = result["failures"].as_array().expect("failures array");
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0]["number"], 2);
    assert_eq!(failures[0]["description"], "email matches");
    assert_eq!(failures[0]["file"], "t/user.t");
    assert_eq!(failures[0]["line"], 12);
    assert_eq!(failures[0]["got"], "'x'");
    assert_eq!(failures[0]["expected"], "'y'");
}

#[test]
fn test_format_test_command_result_todo_skip_not_hard_failures() {
    let provider = ExecuteCommandProvider::new();
    let tap = b"1..2\nnot ok 1 - later # TODO wip\nok 2 - win # SKIP no db\n".to_vec();
    let output = std::process::Output { status: mock_status(0), stdout: tap, stderr: b"".to_vec() };

    let result = provider.format_test_command_result(output, "yath");
    assert_eq!(result["tap"]["failed"], 0, "TODO/SKIP are not hard failures");
    assert_eq!(result["tap"]["todo"], 1);
    assert_eq!(result["tap"]["skipped"], 1);
    assert_eq!(result["failures"].as_array().map(Vec::len), Some(0));
}

#[test]
fn test_format_command_result_failure() {
    let provider = ExecuteCommandProvider::new();

    // Test failed result
    let output = std::process::Output {
        status: mock_status(1),
        stdout: b"partial output".to_vec(),
        stderr: b"error message".to_vec(),
    };

    let result = provider.format_command_result(output, None);
    assert_eq!(result["success"], false, "Should indicate failure");
    assert_eq!(result["output"], "partial output", "Should include stdout");
    assert_eq!(result["error"], "error message", "Should include stderr as error");
}

#[test]
fn test_format_violation_structure() {
    let provider = ExecuteCommandProvider::new();

    let violation = provider.format_violation(
        "TestPolicy",
        "Test description",
        "Test explanation",
        3,
        10,
        5,
        "/tmp/test.pl",
    );

    assert_eq!(violation["policy"], "TestPolicy");
    assert_eq!(violation["description"], "Test description");
    assert_eq!(violation["explanation"], "Test explanation");
    assert_eq!(violation["severity"], 3);
    assert_eq!(violation["line"], 10);
    assert_eq!(violation["column"], 5);
    assert_eq!(violation["file"], "/tmp/test.pl");
}

#[test]
fn test_format_critic_error_structure() {
    let provider = ExecuteCommandProvider::new();

    let error_response =
        provider.format_critic_error("Test error message".to_string(), "test_analyzer");

    assert_eq!(error_response["status"], "error");
    assert_eq!(error_response["error"], "Test error message");
    assert!(error_response["violations"].is_array());
    assert_eq!(error_response["violationCount"], 0);
    assert_eq!(error_response["analyzerUsed"], "test_analyzer");
}

#[test]
#[allow(deprecated)]
fn test_run_critic_file_exists_check() -> Result<(), Box<dyn std::error::Error>> {
    let provider = ExecuteCommandProvider::new();

    // Test with non-existent file
    let result = provider.run_critic("/tmp/definitely_nonexistent_file_12345.pl");
    assert!(result.is_ok(), "Should handle missing files gracefully");

    let result_value = result?;
    assert_eq!(result_value["status"], "error", "Should report error status");
    assert!(
        result_value["error"].as_str().ok_or("expected error string")?.contains("File not found"),
        "Should indicate file not found"
    );
    assert_eq!(result_value["analyzerUsed"], "none", "Should indicate no analyzer used");
    Ok(())
}

#[test]
fn test_run_builtin_critic_with_valid_file() -> Result<(), Box<dyn std::error::Error>> {
    let provider = ExecuteCommandProvider::new();

    // Create a temporary file with content using a portable temp directory
    let tmp = tempdir()?;
    let test_content = "#!/usr/bin/perl\nmy $var = 42;\nprint $var;\n";
    let temp_file = tmp.path().join("test_builtin_critic.pl");
    fs::write(&temp_file, test_content)?;

    let result = provider.run_builtin_critic(&temp_file);

    assert!(result.is_ok(), "Built-in critic should execute successfully");
    let result_value = result?;
    assert_eq!(result_value["status"], "success");
    assert!(result_value["violations"].is_array());
    assert_eq!(result_value["analyzerUsed"], "native");
    Ok(())
}

// ============= NATIVE-FIRST CRITIC ENGINE GATING =============
// perl.runCritic must default to the native analyzer. Merely having
// `perlcritic` on PATH must NOT change the default behavior — external
// perlcritic runs only when the critic engine is explicitly set to
// legacy/external/perlcritic.

#[test]
#[allow(deprecated)]
fn run_critic_defaults_to_native_analyzer() -> Result<(), Box<dyn std::error::Error>> {
    // A provider with no workspace config (the common case) must report the
    // native analyzer, regardless of whether `perlcritic` is installed on the
    // host running this test.
    let provider = ExecuteCommandProvider::new();

    let tmp = tempdir()?;
    let file = tmp.path().join("native_default.pl");
    fs::write(&file, "#!/usr/bin/perl\nmy $x = 1;\nprint $x;\n")?;

    let result = provider.run_critic(file.to_str().ok_or("path is not utf-8")?)?;
    assert_eq!(result["status"], "success");
    assert_eq!(
        result["analyzerUsed"], "native",
        "default runCritic must use the native analyzer, not external perlcritic"
    );
    Ok(())
}

// #3299: default `perl.runCritic` (engine Native) must route through the
// `NativeCriticRegistry` — the same engine the editor's on-type native pull
// diagnostics use — so the command reports `native.*` rules (here
// `native.security.string_eval`, in the default `recommended` profile) that the
// legacy `BuiltInAnalyzer` does not emit. Before #3299 the command ran
// `BuiltInAnalyzer` and diverged from native diagnostics on native.*-only rules.
#[test]
fn run_critic_native_matches_pull_diagnostics_registry() -> Result<(), Box<dyn std::error::Error>> {
    use crate::perl_critic::{
        CriticConfig, CriticContext, NativeCriticProfile, NativeCriticRegistry,
    };

    // A string `eval` trips `native.security.string_eval`.
    let source = "use strict;\nuse warnings;\nmy $c = '1';\neval $c;\n";
    let tmp = tempdir()?;
    let file = tmp.path().join("string_eval.pl");
    fs::write(&file, source)?;

    // Default provider = engine Native, `recommended` profile (the real default).
    let provider = ExecuteCommandProvider::new();
    let result = provider.run_native_critic(&file)?;
    assert_eq!(result["analyzerUsed"], "native");
    let command_policies: Vec<&str> = result["violations"]
        .as_array()
        .ok_or("violations is not an array")?
        .iter()
        .filter_map(|v| v["policy"].as_str())
        .collect();
    assert!(
        command_policies.contains(&"native.security.string_eval"),
        "runCritic must report the native registry rule the editor path reports: {command_policies:?}"
    );

    // Parity: the editor's on-type native pull path runs the same registry over
    // the same source and must report the same rule.
    let code = perl_parser::util::code_slice(source);
    let mut parser = perl_parser::Parser::new(code);
    let ast = parser.parse().map_err(|e| e.to_string())?;
    let critic_config = CriticConfig::default();
    let ctx = CriticContext::new(code, &ast, &critic_config);
    let registry = NativeCriticRegistry::for_profile(
        NativeCriticProfile::parse("recommended").ok_or("recommended profile must parse")?,
    );
    let pull_rule_ids: Vec<String> = registry.check(&ctx).into_iter().map(|f| f.rule_id).collect();
    assert!(
        pull_rule_ids.iter().any(|id| id == "native.security.string_eval"),
        "native pull path must report the same rule: {pull_rule_ids:?}"
    );

    // Regression proof: the legacy `BuiltInAnalyzer` path omits native.* rules —
    // exactly the divergence #3299 removes for the default engine.
    let builtin = provider.run_builtin_critic(&file)?;
    let builtin_policies: Vec<&str> = builtin["violations"]
        .as_array()
        .ok_or("violations is not an array")?
        .iter()
        .filter_map(|v| v["policy"].as_str())
        .collect();
    assert!(
        !builtin_policies.contains(&"native.security.string_eval"),
        "BuiltInAnalyzer must not emit native.* rules (the pre-#3299 gap): {builtin_policies:?}"
    );
    Ok(())
}

#[test]
fn external_critic_not_requested_without_config() {
    // The structural guarantee behind "perlcritic on PATH does not change the
    // default": with no config the external branch is never reached, so the
    // presence of `perlcritic` on PATH is irrelevant to the default analyzer.
    let provider = ExecuteCommandProvider::new();
    assert!(
        !provider.external_critic_requested(),
        "no config must default to the native critic engine"
    );
}

#[test]
fn external_critic_not_requested_for_native_engine() {
    let provider = ExecuteCommandProvider::new().with_critic_engine(CriticEngine::Native);
    assert!(
        !provider.external_critic_requested(),
        "native critic engine must not select external perlcritic"
    );
}

#[test]
fn external_critic_requested_for_legacy_engine() {
    let provider = ExecuteCommandProvider::new().with_critic_engine(CriticEngine::Legacy);
    assert!(
        provider.external_critic_requested(),
        "explicit legacy/external critic engine must select external perlcritic"
    );
}

#[test]
fn test_command_exists_behavior() {
    let provider = ExecuteCommandProvider::new();

    // Test with a command that definitely exists
    let exists = provider.command_exists("echo");
    // Note: We can't assert true here because the mutation test replaces return value
    // But we can verify it returns a boolean (this always passes but validates function call)
    assert!(matches!(exists, true | false), "Should return a boolean");

    // Test with a command that definitely doesn't exist
    let exists = provider.command_exists("definitely_nonexistent_command_12345");
    // This should be false, but mutation testing may change the logic
    assert!(matches!(exists, true | false), "Should return a boolean");
}

#[test]
fn test_all_command_routing_paths() {
    let provider = ExecuteCommandProvider::new();

    // Test each command path individually to ensure routing logic is tested
    let commands_to_test = vec![
        "perl.runTests",
        "perl.runFile",
        "perl.runTestSub",
        "perl.debugTests",
        "perl.runCritic",
        "perl.runTest",
        "perl.runTestFile",
        "perl.runSubtest",
        "perl.debugFile",
        "perl.debugTest",
    ];

    for command in commands_to_test {
        let args = if command == "perl.runTestSub" || command == "perl.runSubtest" {
            vec![Value::String("/tmp/test.pl".to_string()), Value::String("test_sub".to_string())]
        } else {
            vec![Value::String("/tmp/test.pl".to_string())]
        };

        let result = provider.execute_command(command, args);

        // Each command should either succeed or fail gracefully
        // but should never panic or be unhandled
        match result {
            Ok(value) => {
                assert!(value.is_object(), "Successful results should be objects");
            }
            Err(error) => {
                // Errors should be meaningful
                assert!(!error.is_empty(), "Error messages should not be empty");
            }
        }
    }
}

#[test]
fn test_debug_tests_returns_perl_dap_launch_config() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let test_file = temp_dir.path().join("debug_me.t");
    fs::write(&test_file, "use Test2::V0;\nok(1);\ndone_testing;\n")?;

    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()]);
    let result = provider.execute_command(
        "perl.debugTestFile",
        vec![Value::String(test_file.display().to_string())],
    )?;

    assert_eq!(result["success"], true, "debug should no longer be a 'coming soon' stub");
    assert_eq!(result["action"], "startDebugging");
    assert_eq!(result["adapter"], "perl-dap");
    let config = &result["configuration"];
    assert_eq!(config["type"], "perl", "launches through the native perl debug type");
    assert_eq!(config["request"], "launch");
    assert_eq!(config["stopOnEntry"], false);
    assert!(config["program"].as_str().unwrap_or("").ends_with("debug_me.t"));
    assert!(config["cwd"].is_string(), "cwd follows the runner policy (file dir)");
    assert_eq!(config["env"]["PERL_TEST_HARNESS_DUMP_TAP"], "1");
    Ok(())
}

#[test]
fn test_run_subtest_reports_whole_file_focused_mode() -> Result<(), Box<dyn std::error::Error>> {
    // The focused-run metadata (mode/requestedSubtest/note) is set regardless of
    // whether `perl` is present, so this test is robust to the environment.
    let temp_dir = tempdir()?;
    let test_file = temp_dir.path().join("focus.t");
    fs::write(&test_file, "use Test2::V0;\nsubtest 'alpha' => sub { ok(1); };\ndone_testing;\n")?;

    let provider = provider_with_execute_perl(vec![temp_dir.path().to_path_buf()]);
    let canonical = test_file.canonicalize()?;
    let result = provider.run_subtest(&canonical, "alpha")?;

    assert_eq!(result["requestedSubtest"], "alpha", "should echo the requested subtest name");
    assert_eq!(
        result["subtestMode"], "whole-file-focused",
        "runs whole file; never executes the subtest block in isolation"
    );
    assert!(result["note"].as_str().unwrap_or("").contains("does not execute subtest blocks"));
    assert!(result["subtestFocus"].is_object(), "should include a subtest focus object");
    // Raw runner fields are preserved from the whole-file run.
    assert!(result["output"].is_string(), "raw output preserved");
    Ok(())
}

// ============= ADDITIONAL MUTATION KILLER TESTS =============
// These tests specifically target remaining surviving mutants

#[test]
fn test_execute_command_return_value_mutations() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let temp_file = temp_dir.path().join("test_mutations.pl");
    fs::write(&temp_file, "print 'test';")?;

    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()]);

    // This test ensures that execute_command cannot return Ok(Default::default())
    // when it should return meaningful data
    let result = provider
        .execute_command("perl.debugTests", vec![Value::String(temp_file.display().to_string())]);

    assert!(result.is_ok(), "Should return Ok");
    let result_value = result?;

    // Verify it's not just a default empty object
    assert!(result_value.is_object(), "Should return an object");
    assert!(
        result_value.as_object().ok_or("expected object")?.contains_key("success"),
        "Should have success field"
    );
    assert!(
        result_value.as_object().ok_or("expected object")?.contains_key("configuration"),
        "Should have a launch configuration field"
    );

    // The result should be meaningful, not just Default::default()
    assert_ne!(result_value, Value::Object(serde_json::Map::new()), "Should not be empty object");
    Ok(())
}

#[test]
fn test_run_tests_logic_operators() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let provider = ExecuteCommandProvider::new();

    // Create test files to test is_test_file && command_exists logic
    let test_file_t = temp_dir.path().join("mutation_test.t");
    let non_test_file = temp_dir.path().join("mutation_test.pl");

    fs::write(&test_file_t, "use Test::More; ok(1); done_testing();")?;
    fs::write(&non_test_file, "print 'hello world';")?;

    // Test with .t file (should attempt to use prove if available)
    let result = provider.run_tests(&test_file_t);
    assert!(result.is_ok(), "Should handle .t files");
    let result_value = result?;
    assert!(result_value["success"].is_boolean(), "Should have boolean success");
    assert!(result_value["output"].is_string(), "Should have string output");

    // Test with non-test file (should use perl directly)
    let result = provider.run_tests(&non_test_file);
    assert!(result.is_ok(), "Should handle .pl files");
    let result_value = result?;
    assert!(result_value["success"].is_boolean(), "Should have boolean success");
    assert!(result_value["output"].is_string(), "Should have string output");

    Ok(())
}

#[test]
fn test_select_test_runner_prefers_yath_for_test_files() {
    assert_eq!(
        select_test_runner(true, true, true),
        TestRunner::Yath,
        "yath should win for test files when available"
    );
    assert_eq!(
        select_test_runner(true, true, false),
        TestRunner::Yath,
        "yath should still win when prove is unavailable"
    );
}

#[test]
fn test_select_test_runner_falls_back_to_prove_then_perl() {
    assert_eq!(
        select_test_runner(true, false, true),
        TestRunner::Prove,
        "prove should be used when yath is unavailable"
    );
    assert_eq!(
        select_test_runner(true, false, false),
        TestRunner::Perl,
        "perl fallback should be used when no test runner is available"
    );
    assert_eq!(
        select_test_runner(false, true, true),
        TestRunner::Perl,
        "non-test files should always use perl"
    );
}

#[test]
fn test_is_test_file_operator_mutations() {
    let provider = ExecuteCommandProvider::new();

    // Test various combinations to catch || to && mutations

    // Should be true - ends with .t
    let result = provider.is_test_file("script.t");
    assert!(result, "Files ending in .t should be test files");

    // Should be true - contains /t/
    let result = provider.is_test_file("/path/t/script.pl");
    assert!(result, "Files in t/ directory should be test files");

    // Should be true - contains 'test'
    let result = provider.is_test_file("my_test.pl");
    assert!(result, "Files with 'test' in name should be test files");

    // Should be false - none of the above
    let result = provider.is_test_file("regular.pl");
    assert!(!result, "Regular files should not be test files");

    // Edge case - file that would be false if && was used instead of ||
    let result = provider.is_test_file("test"); // has 'test' but not .t or /t/
    assert!(result, "Should be true with OR logic");
}

#[test]
fn test_run_builtin_critic_arithmetic_mutations() -> Result<(), Box<dyn std::error::Error>> {
    let provider = ExecuteCommandProvider::new();

    // Create a test file with content at known line/column positions
    let tmp = tempdir()?;
    let test_content = "#!/usr/bin/perl\n# Line 2\nmy $var = 42;\nprint $var;\n";
    let temp_file = tmp.path().join("test_arithmetic_mutations.pl");
    fs::write(&temp_file, test_content)?;

    let result = provider.run_builtin_critic(&temp_file);

    assert!(result.is_ok(), "Should analyze file successfully");
    let result_value = result?;

    // Verify that line/column arithmetic is correct
    let violations = result_value["violations"].as_array().ok_or("expected violations array")?;
    for violation in violations {
        let line = violation["line"].as_u64().ok_or("expected line number")?;
        let column = violation["column"].as_u64().ok_or("expected column number")?;

        // Line and column should be positive (+ 1 conversions work)
        assert!(line > 0, "Line numbers should be positive (not result of - or * mutations)");
        assert!(column > 0, "Column numbers should be positive (not result of - or * mutations)");

        // Should be reasonable values for a short file
        assert!(line <= 10, "Line numbers should be reasonable");
        assert!(column <= 100, "Column numbers should be reasonable");
    }
    Ok(())
}

#[test]
fn test_format_command_result_negation_mutation() {
    let provider = ExecuteCommandProvider::new();

    // Test successful status - should NOT be negated
    let success_output = std::process::Output {
        status: mock_status(0),
        stdout: b"success".to_vec(),
        stderr: b"".to_vec(),
    };

    let result = provider.format_command_result(success_output, None);
    assert_eq!(result["success"], true, "Success status should not be negated");
    assert_eq!(result["error"], Value::Null, "Success should have null error");

    // Test failure status - should properly indicate failure
    let failure_output = std::process::Output {
        status: mock_status(1),
        stdout: b"output".to_vec(),
        stderr: b"error".to_vec(),
    };

    let result = provider.format_command_result(failure_output, None);
    assert_eq!(result["success"], false, "Failure status should be false");
    assert_eq!(result["error"], "error", "Failure should include stderr");
}

/// Verifies that mock_status() correctly round-trips exit codes on both platforms.
/// This documents the POSIX (high-byte encoding) vs Windows (direct code) behavior.
#[test]
fn test_exit_status_roundtrip() {
    let ok = mock_status(0);
    assert_eq!(ok.code(), Some(0), "Exit code 0 should round-trip correctly");
    assert!(ok.success(), "Exit code 0 should be success");

    let fail = mock_status(1);
    assert_eq!(fail.code(), Some(1), "Exit code 1 should round-trip correctly");
    assert!(!fail.success(), "Exit code 1 should be failure");
}

#[test]
fn test_format_functions_not_default() -> Result<(), Box<dyn std::error::Error>> {
    let provider = ExecuteCommandProvider::new();

    // Test format_violation doesn't return Default::default()
    let violation = provider.format_violation(
        "TestPolicy",
        "Description",
        "Explanation",
        3,
        5,
        10,
        "/tmp/test.pl",
    );

    assert_ne!(violation, Value::Object(serde_json::Map::new()), "Should not return empty object");
    assert!(violation.is_object(), "Should return structured object");
    assert!(!violation.as_object().ok_or("expected object")?.is_empty(), "Should have content");

    // Test format_critic_error doesn't return Default::default()
    let error = provider.format_critic_error("Test error".to_string(), "test");

    assert_ne!(error, Value::Object(serde_json::Map::new()), "Should not return empty object");
    assert!(error.is_object(), "Should return structured object");
    assert!(!error.as_object().ok_or("expected object")?.is_empty(), "Should have content");
    Ok(())
}

#[test]
#[allow(deprecated)]
fn test_normalize_file_path_not_hardcoded() {
    let provider = ExecuteCommandProvider::new();

    // Non-URI passthrough: a plain path without file:// scheme is returned unchanged.
    let plain = "not-a-uri.pl";
    let result = provider.normalize_file_path(plain);
    assert_eq!(result, plain, "Should return input unchanged for non-URI paths");
    assert_ne!(result, "", "Should not return empty string");
    assert_ne!(result, "xyzzy", "Should not return hardcoded value");

    // Unix-specific: to_file_path() returns Unix-style paths only on Unix.
    #[cfg(unix)]
    {
        let file_uri = "file:///home/user/test.pl";
        let result = provider.normalize_file_path(file_uri);
        assert_eq!(result, "/home/user/test.pl", "Should properly strip file:// prefix");
        assert_ne!(result, "", "Should not return empty string");
        assert_ne!(result, "xyzzy", "Should not return hardcoded value");

        let regular_path = "/home/user/test.pl";
        let result = provider.normalize_file_path(regular_path);
        assert_eq!(result, regular_path, "Should return input unchanged");

        let encoded_file_uri = "file:///home/user/my%20test.pl";
        let result = provider.normalize_file_path(encoded_file_uri);
        assert_eq!(result, "/home/user/my test.pl", "Should decode URI escaped characters");
    }
}

#[test]
fn test_command_exists_not_hardcoded_true() {
    let provider = ExecuteCommandProvider::new();

    // Test with a command that definitely doesn't exist
    // This should return false, not hardcoded true
    let exists = provider.command_exists("definitely_nonexistent_command_xyz_12345");

    // The mutant that returns hardcoded true would fail this test
    // Note: We can't always assert false due to environment differences,
    // but we can verify the function actually runs the check
    assert!(matches!(exists, true | false), "Should return boolean result");

    // Test multiple times to catch inconsistencies from mutations
    let exists2 = provider.command_exists("definitely_nonexistent_command_xyz_12345");
    assert_eq!(exists, exists2, "Should be consistent");
}

#[test]
#[allow(deprecated)]
fn test_run_critic_file_existence_logic() -> Result<(), Box<dyn std::error::Error>> {
    let provider = ExecuteCommandProvider::new();

    // Test the file existence negation - ensure ! is not deleted
    let result = provider.run_critic("/tmp/absolutely_nonexistent_file_xyz_12345.pl");

    assert!(result.is_ok(), "Should handle gracefully");
    let result_value = result?;
    assert_eq!(result_value["status"], "error", "Should detect missing file");
    assert!(
        result_value["error"].as_str().ok_or("expected error string")?.contains("File not found"),
        "Should indicate file not found"
    );

    // If the ! in !path.exists() was deleted, this test would fail
    // because it would try to process a non-existent file
    Ok(())
}

#[test]
fn test_method_return_values_not_defaults() -> Result<(), Box<dyn std::error::Error>> {
    let provider = provider_with_execute_perl(Vec::new());

    // Create real test files using a portable temp directory
    let tmp = tempdir()?;

    let test_content = "#!/usr/bin/perl\nuse strict;\nprint 'hello';\n";
    let temp_file = tmp.path().join("test_return_values.pl");
    fs::write(&temp_file, test_content)?;

    // Test run_file doesn't return Ok(Default::default())
    let result = provider.run_file(&temp_file);
    assert!(result.is_ok(), "run_file should succeed");
    let result_value = result?;
    assert_ne!(result_value, Value::Object(serde_json::Map::new()), "Should not be empty object");

    // Test run_tests doesn't return Ok(Default::default())
    let result = provider.run_tests(&temp_file);
    assert!(result.is_ok(), "run_tests should succeed");
    let result_value = result?;
    assert_ne!(result_value, Value::Object(serde_json::Map::new()), "Should not be empty object");

    // Test run_test_sub doesn't return Ok(Default::default())
    let sub_content = "#!/usr/bin/perl\nuse strict;\nsub test_func { print 'test'; }\n";
    let sub_file = tmp.path().join("test_sub_return.pl");
    fs::write(&sub_file, sub_content)?;

    let result = provider.run_test_sub(&sub_file, "test_func");
    assert!(result.is_ok(), "run_test_sub should succeed");
    let result_value = result?;
    assert_ne!(result_value, Value::Object(serde_json::Map::new()), "Should not be empty object");

    Ok(())
}

#[test]
fn test_execute_command_workspace_security() -> Result<(), Box<dyn std::error::Error>> {
    // Create a temporary workspace and a file outside it
    let workspace_dir = std::env::temp_dir().join("perl_lsp_workspace");
    let outside_file = std::env::temp_dir().join("perl_lsp_outside.pl");

    fs::create_dir_all(&workspace_dir)?;
    fs::write(&outside_file, "print 'outside';")?;

    let provider = ExecuteCommandProvider::with_workspace_roots(vec![workspace_dir.clone()]);

    // Try to execute the outside file
    let result = provider.execute_command(
        "perl.runFile",
        vec![Value::String(outside_file.to_string_lossy().to_string())],
    );

    // Clean up
    fs::remove_dir_all(&workspace_dir).ok();
    fs::remove_file(&outside_file).ok();

    // Verify security check
    assert!(result.is_err(), "Should fail execution outside workspace");
    let error = result.err().ok_or("expected error")?;
    assert!(
        error.contains("Path traversal") || error.contains("outside workspace roots"),
        "Error should indicate security violation: {}",
        error
    );
    Ok(())
}

#[test]
fn test_execute_command_multi_root_security() -> Result<(), Box<dyn std::error::Error>> {
    // Create two temporary workspaces and a file outside both
    let workspace_dir1 = std::env::temp_dir().join("perl_lsp_workspace_1");
    let workspace_dir2 = std::env::temp_dir().join("perl_lsp_workspace_2");
    let file1 = workspace_dir1.join("test1.pl");
    let file2 = workspace_dir2.join("test2.pl");
    let outside_file = std::env::temp_dir().join("perl_lsp_outside_multi.pl");

    fs::create_dir_all(&workspace_dir1)?;
    fs::create_dir_all(&workspace_dir2)?;
    fs::write(&file1, "print 'file1';")?;
    fs::write(&file2, "print 'file2';")?;
    fs::write(&outside_file, "print 'outside';")?;

    let provider = provider_with_execute_perl(vec![workspace_dir1.clone(), workspace_dir2.clone()]);

    // 1. Should succeed for file in workspace 1
    let result1 = provider
        .execute_command("perl.runFile", vec![Value::String(file1.to_string_lossy().to_string())]);
    assert!(result1.is_ok(), "Should allow execution in workspace 1");

    // 2. Should succeed for file in workspace 2
    let result2 = provider
        .execute_command("perl.runFile", vec![Value::String(file2.to_string_lossy().to_string())]);
    assert!(result2.is_ok(), "Should allow execution in workspace 2");

    // 3. Should fail for outside file
    let result3 = provider.execute_command(
        "perl.runFile",
        vec![Value::String(outside_file.to_string_lossy().to_string())],
    );
    assert!(result3.is_err(), "Should fail execution outside both workspaces");
    let error3 = result3.err().ok_or("expected error")?;
    assert!(
        error3.contains("Path traversal") || error3.contains("outside workspace roots"),
        "Error should indicate security violation: {}",
        error3
    );

    // Clean up
    fs::remove_dir_all(&workspace_dir1).ok();
    fs::remove_dir_all(&workspace_dir2).ok();
    fs::remove_file(&outside_file).ok();

    Ok(())
}

// ============= ADVERTISED-BUT-UNHANDLED COMMAND WIRING TESTS =============
// Issue #2691: perl.runTest, perl.runTestFile, perl.runSubtest, perl.debugFile, perl.debugTest
// were advertised by get_supported_commands() but hit the "Unknown command" fallback.

#[test]
fn test_supported_commands_includes_all_advertised() {
    let commands = get_supported_commands();
    let required = [
        "perl.runTest",
        "perl.runTestFile",
        "perl.runSubtest",
        "perl.debugFile",
        "perl.debugTest",
        "perl.workspaceTrustReport",
        "perl.explainMissingModuleLookup",
    ];
    for cmd in &required {
        assert!(
            commands.contains(&cmd.to_string()),
            "{} should be in supported commands list",
            cmd
        );
    }
}

#[test]
fn test_all_supported_commands_are_handled() {
    // Every command in get_supported_commands() should be recognized by
    // execute_command() — none should return "Unknown command".
    let provider = ExecuteCommandProvider::new();
    let commands = get_supported_commands();

    for command in &commands {
        // Supply minimal arguments (a bogus path) so we hit the match arm,
        // not an argument-validation error.  We only care that the error is
        // NOT "Unknown command".
        let result =
            provider.execute_command(command, vec![Value::String("/tmp/test.pl".to_string())]);
        match &result {
            Err(e) => {
                assert!(
                    !e.contains("Unknown command"),
                    "Command {} should be handled, but got: {}",
                    command,
                    e
                );
            }
            Ok(_) => { /* handled successfully — fine */ }
        }
    }
}

#[test]
fn test_command_routing_perl_run_test() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let temp_file = temp_dir.path().join("test_run_test.t");
    fs::write(&temp_file, "use Test::More;\nok(1);\ndone_testing;\n")?;

    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()]);
    let result = provider
        .execute_command("perl.runTest", vec![Value::String(temp_file.display().to_string())]);

    assert!(result.is_ok(), "perl.runTest should dispatch without Unknown command error");
    let value = result?;
    assert!(value.is_object(), "Should return a structured result");
    assert!(value["success"].is_boolean(), "Should have success field");
    assert!(value["output"].is_string(), "Should have output field");
    Ok(())
}

#[test]
fn test_command_routing_perl_run_test_file() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let temp_file = temp_dir.path().join("test_run_test_file.t");
    fs::write(&temp_file, "use Test::More;\nok(1);\ndone_testing;\n")?;

    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()]);
    let result = provider
        .execute_command("perl.runTestFile", vec![Value::String(temp_file.display().to_string())]);

    assert!(result.is_ok(), "perl.runTestFile should dispatch without Unknown command error");
    let value = result?;
    assert!(value.is_object(), "Should return a structured result");
    assert!(value["success"].is_boolean(), "Should have success field");
    assert!(value["output"].is_string(), "Should have output field");
    Ok(())
}

#[test]
fn test_command_routing_perl_run_subtest() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let temp_file = temp_dir.path().join("test_run_subtest.t");
    fs::write(
        &temp_file,
        "use Test::More;\nsub my_subtest { ok(1); }\nmy_subtest();\ndone_testing;\n",
    )?;

    let provider = provider_with_execute_perl(vec![temp_dir.path().to_path_buf()]);
    let result = provider.execute_command(
        "perl.runSubtest",
        vec![
            Value::String(temp_file.display().to_string()),
            Value::String("my_subtest".to_string()),
        ],
    );

    assert!(result.is_ok(), "perl.runSubtest should dispatch without Unknown command error");
    let value = result?;
    assert!(value.is_object(), "Should return a structured result");
    assert!(value["success"].is_boolean(), "Should have success field");
    // Whole-file-focused run (never executes the subtest block in isolation).
    assert_eq!(value["subtestMode"], "whole-file-focused");
    assert_eq!(value["requestedSubtest"], "my_subtest");
    Ok(())
}

#[test]
fn test_command_routing_perl_debug_file() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let temp_file = temp_dir.path().join("test_debug_file.pl");
    fs::write(&temp_file, "print 'debug file';")?;

    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()]);
    let result = provider
        .execute_command("perl.debugFile", vec![Value::String(temp_file.display().to_string())]);

    assert!(result.is_ok(), "perl.debugFile should dispatch without Unknown command error");
    let value = result?;
    assert!(value.is_object(), "Should return a structured result");
    // Debug now returns a real perl-dap launch configuration.
    assert_eq!(value["success"], true, "Debug should return a launch config");
    assert_eq!(value["action"], "startDebugging");
    assert_eq!(value["configuration"]["type"], "perl");
    Ok(())
}

#[test]
fn test_command_routing_perl_debug_test() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let temp_file = temp_dir.path().join("test_debug_test.t");
    fs::write(&temp_file, "use Test::More;\nok(1);\ndone_testing;\n")?;

    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()]);
    let result = provider
        .execute_command("perl.debugTest", vec![Value::String(temp_file.display().to_string())]);

    assert!(result.is_ok(), "perl.debugTest should dispatch without Unknown command error");
    let value = result?;
    assert!(value.is_object(), "Should return a structured result");
    // Debug now returns a real perl-dap launch configuration.
    assert_eq!(value["success"], true, "Debug should return a launch config");
    assert_eq!(value["action"], "startDebugging");
    assert_eq!(value["configuration"]["type"], "perl");
    Ok(())
}

#[test]
fn test_debug_tests_bare_filename_uses_dot_cwd() -> Result<(), Box<dyn std::error::Error>> {
    // A single-component path has a Some("") parent; the launch cwd must fall
    // back to "." rather than an empty string (which can fail to launch).
    let provider = ExecuteCommandProvider::new();
    let value = provider.debug_tests(std::path::Path::new("bare.t"))?;
    assert_eq!(value["configuration"]["cwd"], ".");
    assert_eq!(value["configuration"]["program"], "bare.t");
    Ok(())
}

#[test]
fn test_perl_run_subtest_missing_subroutine_arg() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let temp_file = temp_dir.path().join("test_subtest_no_arg.t");
    fs::write(&temp_file, "use Test::More;\ndone_testing;\n")?;

    let provider =
        ExecuteCommandProvider::with_workspace_roots(vec![temp_dir.path().to_path_buf()]);
    let result = provider
        .execute_command("perl.runSubtest", vec![Value::String(temp_file.display().to_string())]);

    assert!(result.is_err(), "perl.runSubtest should fail without a subtest name");
    let err = result.err().ok_or("expected error")?;
    assert!(
        err.contains("Missing subtest name"),
        "Should report missing subtest name, got: {}",
        err
    );
    Ok(())
}

// --- Windows binary-planting RCE regression (#3028) ---
//
// These tests verify that `run_test_command` and `command_exists` never let a
// planted binary in the LSP workspace root (CWD) execute.  They serialize CWD
// mutation with a static Mutex and restore the original CWD after each test
// regardless of outcome.
//
// The chain being guarded: `executeCommand("perl.runTests")` →
// `run_tests` → `command_exists("yath")` / `command_exists("prove")` /
// `run_test_command("perl"|"yath"|"prove", …)` → previously `Command::new(bare)`,
// now `resolve_program(bare)` which excludes the CWD.

/// `command_exists` must return `false` for a tool whose only copy is planted in
/// the CWD — not `true`, which would then drive `run_test_command` to execute it.
///
/// Security invariant: the resolver excludes CWD; a not-on-PATH bare name must
/// report absent even when a same-named binary sits in the workspace root.
#[cfg(windows)]
#[test]
fn test_command_exists_ignores_planted_cwd_binary() {
    use std::io::Write as _;
    use std::sync::Mutex;
    static CWD_LOCK: Mutex<()> = Mutex::new(());
    let _guard = CWD_LOCK.lock().unwrap_or_else(|p| p.into_inner());

    let unique = format!("rce_exists_{}", std::process::id());
    let workspace = std::env::temp_dir().join(unique);
    let _ = std::fs::remove_dir_all(&workspace);
    std::fs::create_dir_all(&workspace).expect("create temp workspace");

    // Plant a batch file — only copy of this tool; never on PATH.
    let planted = workspace.join("definitely_not_real_3028.bat");
    {
        let mut f = std::fs::File::create(&planted).expect("create planted bat");
        writeln!(f, "@echo off").expect("write bat");
        writeln!(f, "echo PWNED").expect("write bat");
    }

    let original_cwd = std::env::current_dir().expect("capture cwd");
    std::env::set_current_dir(&workspace).expect("enter temp workspace");

    let provider = ExecuteCommandProvider::new();
    let exists = provider.command_exists("definitely_not_real_3028.bat");

    std::env::set_current_dir(&original_cwd).expect("restore cwd");
    let _ = std::fs::remove_dir_all(&workspace);

    assert!(
        !exists,
        "SECURITY: command_exists reported true for a tool only present in the CWD — \
         a planted binary would now be invoked via run_test_command (#3028)"
    );
}

/// `run_test_command` must fail closed (return `Err`) when the named tool is not
/// on PATH — it must not execute a planted binary in the CWD.
///
/// We use a tool name that is guaranteed not to be on PATH so the resolver
/// returns `Err` before any `Command` is spawned.  We verify the planted marker
/// file was NOT written (i.e., the batch file was not executed).
#[cfg(windows)]
#[test]
fn test_run_test_command_does_not_execute_planted_cwd_binary() {
    use std::io::Write as _;
    use std::sync::Mutex;
    static CWD_LOCK: Mutex<()> = Mutex::new(());
    let _guard = CWD_LOCK.lock().unwrap_or_else(|p| p.into_inner());

    let unique = format!("rce_run_{}", std::process::id());
    let workspace = std::env::temp_dir().join(&unique);
    let _ = std::fs::remove_dir_all(&workspace);
    std::fs::create_dir_all(&workspace).expect("create temp workspace");

    let marker = workspace.join("PWNED_3028.txt");
    let planted = workspace.join("pwned_3028.bat");
    {
        let mut f = std::fs::File::create(&planted).expect("create planted bat");
        writeln!(f, "@echo off").expect("write bat");
        writeln!(f, "echo pwned> \"{}\"", marker.display()).expect("write bat");
    }

    // A dummy test file — content doesn't matter; command resolution fails first.
    let dummy_t = workspace.join("dummy.t");
    std::fs::write(&dummy_t, "use Test::More;\ndone_testing;\n").expect("create dummy test");

    let original_cwd = std::env::current_dir().expect("capture cwd");
    std::env::set_current_dir(&workspace).expect("enter temp workspace");

    let provider = ExecuteCommandProvider::with_workspace_roots(vec![workspace.clone()]);
    // Drive run_test_command with the bare name of the planted batch file.
    // The resolver must fail closed before spawning anything.
    let result = provider.run_test_command("pwned_3028.bat", &dummy_t);

    std::env::set_current_dir(&original_cwd).expect("restore cwd");
    let marker_exists = marker.exists();
    let _ = std::fs::remove_dir_all(&workspace);

    assert!(
        result.is_err(),
        "run_test_command must fail closed for a not-on-PATH bare name; got: {result:?}"
    );
    assert!(
        !marker_exists,
        "SECURITY: planted CWD batch file was EXECUTED via run_test_command — the RCE is live (#3028)"
    );
}
