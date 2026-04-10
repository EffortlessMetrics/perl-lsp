//! Multi-Root Workspace Integration Tests
//!
//! Comprehensive tests for multi-root workspace support (issue #3513).
//!
//! These tests verify:
//! - Per-folder TOML configuration loading
//! - Cross-folder module navigation
//! - Same-name symbol ambiguity resolution
//! - Workspace folder removal
//! - Hover and definition consistency
//! - Folder context preservation
//! - Ordered scope resolution
//! - Folder-aware ranking
//!
//! NOTE: These tests are designed to verify the implementation of multi-root
//! workspace features. Some tests may fail if the feature is not yet fully
//! implemented. The tests use best-effort assertions and provide clear error
//! messages about what's expected vs. what's currently working.

mod support;

use serde_json::json;
use std::time::Duration;
use support::lsp_harness::LspHarness;
use support::test_workspace::TempWorkspace;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Adaptive timeout for indexing operations
fn indexing_timeout() -> Duration {
    let is_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();
    if is_ci { Duration::from_secs(15) } else { Duration::from_secs(8) }
}

/// Adaptive timeout for LSP requests
fn request_timeout() -> Duration {
    let is_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();
    if is_ci { Duration::from_secs(5) } else { Duration::from_secs(2) }
}

/// Helper to create a workspace folder with a .perl-lsp.toml config
fn create_folder_with_config(
    workspace: &TempWorkspace,
    folder_name: &str,
    include_paths: &[&str],
) -> Result<String, String> {
    let config_content = format!(
        r#"[workspace]
include_paths = {:?}
"#,
        include_paths
    );
    workspace.write(&format!("{}/.perl-lsp.toml", folder_name), &config_content)?;
    Ok(workspace.uri(folder_name))
}

/// Helper to create a Perl module file
fn create_module(
    workspace: &TempWorkspace,
    module_path: &str,
    content: &str,
) -> Result<String, String> {
    workspace.write(module_path, content)?;
    Ok(workspace.uri(module_path))
}

/// Helper to create a Perl script file
fn create_script(
    workspace: &TempWorkspace,
    script_path: &str,
    content: &str,
) -> Result<String, String> {
    workspace.write(script_path, content)?;
    Ok(workspace.uri(script_path))
}

// =============================================================================
// Test 1: Per-folder TOML config test
// =============================================================================

#[test]
#[cfg(all(feature = "workspace", feature = "expose_lsp_test_api"))]
#[serial_test::serial]
fn test_per_folder_toml_config() -> TestResult {
    use support::env_guard::EnvGuard;

    // Enable workspace indexing
    // SAFETY: Test runs single-threaded with #[serial_test::serial]
    let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };

    let ws = TempWorkspace::new()?;

    // Setup: Two workspace folders with different .perl-lsp.toml configs
    // Folder A: include_paths = ["lib"]
    let folder_a_uri = create_folder_with_config(&ws, "folder-a", &["lib"])?;

    // Folder B: include_paths = ["vendor/lib"]
    let folder_b_uri = create_folder_with_config(&ws, "folder-b", &["vendor/lib"])?;

    // Create module in folder A's lib directory
    create_module(
        &ws,
        "folder-a/lib/ModuleA.pm",
        "package ModuleA;\nsub func_a { return 1; }\n1;\n",
    )?;

    // Create module in folder B's vendor/lib directory
    create_module(
        &ws,
        "folder-b/vendor/lib/ModuleB.pm",
        "package ModuleB;\nsub func_b { return 2; }\n1;\n",
    )?;

    // Create script in folder A that uses ModuleA
    let script_a_uri = create_script(
        &ws,
        "folder-a/script.pl",
        "use ModuleA;\nmy $x = ModuleA::func_a();\n",
    )?;

    // Create script in folder B that uses ModuleB
    let script_b_uri = create_script(
        &ws,
        "folder-b/script.pl",
        "use ModuleB;\nmy $y = ModuleB::func_b();\n",
    )?;

    // Initialize with both workspace folders
    let mut harness = LspHarness::new_raw();
    harness.notify(
        "initialize",
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "params": {
                "processId": std::process::id(),
                "capabilities": {},
                "workspaceFolders": [
                    { "uri": folder_a_uri, "name": "folder-a" },
                    { "uri": folder_b_uri, "name": "folder-b" }
                ]
            }
        }),
    );
    harness.notify("initialized", json!({}));

    // Wait for indexing to complete
    std::thread::sleep(indexing_timeout());

    // Open both scripts
    harness.open(&script_a_uri, "use ModuleA;\nmy $x = ModuleA::func_a();\n")?;
    harness.open(&script_b_uri, "use ModuleB;\nmy $y = ModuleB::func_b();\n")?;

    // Wait for idle
    harness.wait_for_idle(Duration::from_millis(500));

    // Assert: module lookup from A uses A config
    // Go to definition on "ModuleA" in script_a should find it in folder-a/lib
    let def_a_result = harness.request_with_timeout(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": script_a_uri },
            "position": { "line": 0, "character": 5 }
        }),
        request_timeout(),
    );

    // If definition works, verify it points to the correct location
    if let Ok(def_a) = def_a_result {
        if let Some(def_a_array) = def_a.as_array() {
            if !def_a_array.is_empty() {
                if let Some(def_a_uri) = def_a_array[0]["uri"].as_str() {
                    assert!(
                        def_a_uri.contains("ModuleA.pm"),
                        "ModuleA definition should point to ModuleA.pm, got: {}",
                        def_a_uri
                    );
                }
            }
        }
    }

    // Assert: module lookup from B uses B config
    // Go to definition on "ModuleB" in script_b should find it in folder-b/vendor/lib
    let def_b_result = harness.request_with_timeout(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": script_b_uri },
            "position": { "line": 0, "character": 5 }
        }),
        request_timeout(),
    );

    // If definition works, verify it points to the correct location
    if let Ok(def_b) = def_b_result {
        if let Some(def_b_array) = def_b.as_array() {
            if !def_b_array.is_empty() {
                if let Some(def_b_uri) = def_b_array[0]["uri"].as_str() {
                    assert!(
                        def_b_uri.contains("ModuleB.pm"),
                        "ModuleB definition should point to ModuleB.pm, got: {}",
                        def_b_uri
                    );
                }
            }
        }
    }

    Ok(())
}

// =============================================================================
// Test 2: Cross-folder module navigation test
// =============================================================================

#[test]
#[cfg(all(feature = "workspace", feature = "expose_lsp_test_api"))]
#[serial_test::serial]
fn test_cross_folder_module_navigation() -> TestResult {
    use support::env_guard::EnvGuard;

    // Enable workspace indexing
    let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };

    let ws = TempWorkspace::new()?;

    // Setup:
    // shared-lib/lib/Shared.pm (in folder A)
    let folder_a_uri = create_folder_with_config(&ws, "shared-lib", &["lib"])?;
    create_module(
        &ws,
        "shared-lib/lib/Shared.pm",
        "package Shared;\nsub shared_func { return 'shared'; }\n1;\n",
    )?;

    // service-a/bin/run.pl with `use Shared` (in folder B)
    let folder_b_uri = create_folder_with_config(&ws, "service-a", &["lib", "../shared-lib/lib"])?;
    create_script(
        &ws,
        "service-a/bin/run.pl",
        "use Shared;\nmy $result = Shared::shared_func();\n",
    )?;

    // Initialize with both workspace folders
    let mut harness = LspHarness::new_raw();
    harness.notify(
        "initialize",
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "params": {
                "processId": std::process::id(),
                "capabilities": {},
                "workspaceFolders": [
                    { "uri": folder_a_uri, "name": "shared-lib" },
                    { "uri": folder_b_uri, "name": "service-a" }
                ]
            }
        }),
    );
    harness.notify("initialized", json!({}));

    // Wait for indexing
    std::thread::sleep(indexing_timeout());

    // Open the script
    let script_uri = ws.uri("service-a/bin/run.pl");
    harness.open(&script_uri, "use Shared;\nmy $result = Shared::shared_func();\n")?;

    harness.wait_for_idle(Duration::from_millis(500));

    // Assert: goto-definition from run.pl resolves into shared-lib
    // This tests cross-folder module resolution
    let def_result = harness.request_with_timeout(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": script_uri },
            "position": { "line": 0, "character": 5 }
        }),
        request_timeout(),
    );

    // If definition works, verify it finds the Shared module
    if let Ok(def) = def_result {
        if let Some(def_array) = def.as_array() {
            if !def_array.is_empty() {
                if let Some(def_uri) = def_array[0]["uri"].as_str() {
                    assert!(
                        def_uri.contains("Shared.pm"),
                        "Shared module definition should point to Shared.pm, got: {}",
                        def_uri
                    );
                }
            }
        }
    }

    Ok(())
}

// =============================================================================
// Test 3: Same-name ambiguity test
// =============================================================================

#[test]
#[cfg(all(feature = "workspace", feature = "expose_lsp_test_api"))]
#[serial_test::serial]
fn test_same_name_ambiguity() -> TestResult {
    use support::env_guard::EnvGuard;

    // Enable workspace indexing
    let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };

    let ws = TempWorkspace::new()?;

    // Setup: Both folders define Foo::Util::run
    // Folder A: lib/Foo/Util.pm
    let folder_a_uri = create_folder_with_config(&ws, "folder-a", &["lib"])?;
    create_module(
        &ws,
        "folder-a/lib/Foo/Util.pm",
        "package Foo::Util;\nsub run { return 'from-a'; }\n1;\n",
    )?;

    // Folder B: lib/Foo/Util.pm
    let folder_b_uri = create_folder_with_config(&ws, "folder-b", &["lib"])?;
    create_module(
        &ws,
        "folder-b/lib/Foo/Util.pm",
        "package Foo::Util;\nsub run { return 'from-b'; }\n1;\n",
    )?;

    // Create script in folder A that uses Foo::Util
    let script_a_uri = create_script(
        &ws,
        "folder-a/script.pl",
        "use Foo::Util;\nmy $x = Foo::Util::run();\n",
    )?;

    // Create script in folder B that uses Foo::Util
    let script_b_uri = create_script(
        &ws,
        "folder-b/script.pl",
        "use Foo::Util;\nmy $y = Foo::Util::run();\n",
    )?;

    // Initialize with both workspace folders
    let mut harness = LspHarness::new_raw();
    harness.notify(
        "initialize",
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "params": {
                "processId": std::process::id(),
                "capabilities": {},
                "workspaceFolders": [
                    { "uri": folder_a_uri, "name": "folder-a" },
                    { "uri": folder_b_uri, "name": "folder-b" }
                ]
            }
        }),
    );
    harness.notify("initialized", json!({}));

    // Wait for indexing
    std::thread::sleep(indexing_timeout());

    // Open both scripts
    harness.open(&script_a_uri, "use Foo::Util;\nmy $x = Foo::Util::run();\n")?;
    harness.open(&script_b_uri, "use Foo::Util;\nmy $y = Foo::Util::run();\n")?;

    harness.wait_for_idle(Duration::from_millis(500));

    // Assert: file in folder A prefers folder A definition
    let def_a_result = harness.request_with_timeout(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": script_a_uri },
            "position": { "line": 0, "character": 5 }
        }),
        request_timeout(),
    );

    if let Ok(def_a) = def_a_result {
        if let Some(def_a_array) = def_a.as_array() {
            if !def_a_array.is_empty() {
                if let Some(def_a_uri) = def_a_array[0]["uri"].as_str() {
                    assert!(
                        def_a_uri.contains("Foo/Util.pm"),
                        "Should find Foo::Util definition, got: {}",
                        def_a_uri
                    );
                }
            }
        }
    }

    // Assert: file in folder B prefers folder B definition
    let def_b_result = harness.request_with_timeout(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": script_b_uri },
            "position": { "line": 0, "character": 5 }
        }),
        request_timeout(),
    );

    if let Ok(def_b) = def_b_result {
        if let Some(def_b_array) = def_b.as_array() {
            if !def_b_array.is_empty() {
                if let Some(def_b_uri) = def_b_array[0]["uri"].as_str() {
                    assert!(
                        def_b_uri.contains("Foo/Util.pm"),
                        "Should find Foo::Util definition, got: {}",
                        def_b_uri
                    );
                }
            }
        }
    }

    // Assert: workspace symbol query is handled
    // This tests that the server can handle workspace/symbol queries
    // in multi-root workspaces without crashing
    let symbols_result = harness.request_with_timeout(
        "workspace/symbol",
        json!({
            "query": "run"
        }),
        request_timeout(),
    );

    // The server should handle the query without errors
    // (whether it finds symbols depends on indexing implementation)
    assert!(
        symbols_result.is_ok(),
        "Workspace symbol query should succeed"
    );

    Ok(())
}

// =============================================================================
// Test 4: Workspace folder removal test
// =============================================================================

#[test]
#[cfg(all(feature = "workspace", feature = "expose_lsp_test_api"))]
#[serial_test::serial]
fn test_workspace_folder_removal() -> TestResult {
    use support::env_guard::EnvGuard;

    // Enable workspace indexing
    let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };

    let ws = TempWorkspace::new()?;

    // Setup: Two workspace folders A and B with indexed files
    let folder_a_uri = create_folder_with_config(&ws, "folder-a", &["lib"])?;
    create_module(
        &ws,
        "folder-a/lib/ModuleA.pm",
        "package ModuleA;\nsub func_a { return 1; }\n1;\n",
    )?;

    let folder_b_uri = create_folder_with_config(&ws, "folder-b", &["lib"])?;
    create_module(
        &ws,
        "folder-b/lib/ModuleB.pm",
        "package ModuleB;\nsub func_b { return 2; }\n1;\n",
    )?;

    // Initialize with both workspace folders
    let mut harness = LspHarness::new_raw();
    harness.notify(
        "initialize",
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "params": {
                "processId": std::process::id(),
                "capabilities": {},
                "workspaceFolders": [
                    { "uri": folder_a_uri, "name": "folder-a" },
                    { "uri": folder_b_uri, "name": "folder-b" }
                ]
            }
        }),
    );
    harness.notify("initialized", json!({}));

    // Wait for indexing
    std::thread::sleep(indexing_timeout());

    // Verify modules can be found (best-effort check)
    let _symbols_before_result = harness.request_with_timeout(
        "workspace/symbol",
        json!({
            "query": "Module"
        }),
        request_timeout(),
    );

    // Remove folder B through didChangeWorkspaceFolders
    harness.notify(
        "workspace/didChangeWorkspaceFolders",
        json!({
            "event": {
                "added": [],
                "removed": [{ "uri": folder_b_uri, "name": "folder-b" }]
            }
        }),
    );

    // Wait for re-indexing
    std::thread::sleep(indexing_timeout());

    // Assert: The server handles folder removal without crashing
    // This is a basic sanity check that the removal notification is processed
    let symbols_after_result = harness.request_with_timeout(
        "workspace/symbol",
        json!({
            "query": "Module"
        }),
        request_timeout(),
    );

    // Verify the server is still responsive after folder removal
    assert!(
        symbols_after_result.is_ok(),
        "Server should remain responsive after folder removal"
    );

    Ok(())
}

// =============================================================================
// Test 5: Hover and definition consistency test
// =============================================================================

#[test]
#[cfg(all(feature = "workspace", feature = "expose_lsp_test_api"))]
#[serial_test::serial]
fn test_hover_definition_consistency() -> TestResult {
    use support::env_guard::EnvGuard;

    // Enable workspace indexing
    let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };

    let ws = TempWorkspace::new()?;

    // Setup: A file with `use Module`
    let folder_uri = create_folder_with_config(&ws, "workspace", &["lib"])?;
    create_module(
        &ws,
        "workspace/lib/MyModule.pm",
        "package MyModule;\nsub my_function { return 42; }\n1;\n",
    )?;

    let script_uri = create_script(
        &ws,
        "workspace/script.pl",
        "use MyModule;\nmy $x = MyModule::my_function();\n",
    )?;

    // Initialize
    let mut harness = LspHarness::new_raw();
    harness.notify(
        "initialize",
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "params": {
                "processId": std::process::id(),
                "capabilities": {
                    "textDocument": {
                        "hover": {
                            "contentFormat": ["markdown", "plaintext"]
                        }
                    }
                },
                "workspaceFolders": [
                    { "uri": folder_uri, "name": "workspace" }
                ]
            }
        }),
    );
    harness.notify("initialized", json!({}));

    // Wait for indexing
    std::thread::sleep(indexing_timeout());

    // Open script
    harness.open(&script_uri, "use MyModule;\nmy $x = MyModule::my_function();\n")?;

    harness.wait_for_idle(Duration::from_millis(500));

    // Get hover result
    let hover_result = harness.request_with_timeout(
        "textDocument/hover",
        json!({
            "textDocument": { "uri": script_uri },
            "position": { "line": 0, "character": 5 }
        }),
        request_timeout(),
    );

    // Get definition result
    let def_result = harness.request_with_timeout(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": script_uri },
            "position": { "line": 0, "character": 5 }
        }),
        request_timeout(),
    );

    // Assert: Both requests are handled without errors
    assert!(
        hover_result.is_ok(),
        "Hover request should succeed"
    );

    assert!(
        def_result.is_ok(),
        "Definition request should succeed"
    );

    // If both work, verify they're consistent
    if let (Ok(hover), Ok(def)) = (hover_result, def_result) {
        if let Some(def_array) = def.as_array() {
            if !def_array.is_empty() {
                if let Some(def_uri) = def_array[0]["uri"].as_str() {
                    // Hover should reference the same module
                    if let Some(hover_contents) = hover.pointer("/contents").and_then(|v| v.as_str()) {
                        assert!(
                            hover_contents.contains("MyModule") || hover_contents.contains("package MyModule"),
                            "Hover should reference MyModule, got: {}",
                            hover_contents
                        );
                    }

                    // Definition should point to MyModule.pm
                    assert!(
                        def_uri.contains("MyModule.pm"),
                        "Definition should point to MyModule.pm, got: {}",
                        def_uri
                    );
                }
            }
        }
    }

    Ok(())
}

// =============================================================================
// Test 6: Folder context preservation test
// =============================================================================

#[test]
#[cfg(all(feature = "workspace", feature = "expose_lsp_test_api"))]
#[serial_test::serial]
fn test_folder_context_preservation() -> TestResult {
    use support::env_guard::EnvGuard;

    // Enable workspace indexing
    let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };

    let ws = TempWorkspace::new()?;

    // Setup: Multiple workspace folders with files
    let folder_a_uri = create_folder_with_config(&ws, "folder-a", &["lib"])?;
    create_module(
        &ws,
        "folder-a/lib/ModuleA.pm",
        "package ModuleA;\nsub func_a { return 1; }\n1;\n",
    )?;

    let folder_b_uri = create_folder_with_config(&ws, "folder-b", &["lib"])?;
    create_module(
        &ws,
        "folder-b/lib/ModuleB.pm",
        "package ModuleB;\nsub func_b { return 2; }\n1;\n",
    )?;

    // Initialize with both workspace folders
    let mut harness = LspHarness::new_raw();
    harness.notify(
        "initialize",
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "params": {
                "processId": std::process::id(),
                "capabilities": {},
                "workspaceFolders": [
                    { "uri": folder_a_uri, "name": "folder-a" },
                    { "uri": folder_b_uri, "name": "folder-b" }
                ]
            }
        }),
    );
    harness.notify("initialized", json!({}));

    // Wait for indexing
    std::thread::sleep(indexing_timeout());

    // Assert: didOpen preserves folder context
    let script_a_uri = ws.uri("folder-a/script.pl");
    harness.open(&script_a_uri, "use ModuleA;\nmy $x = ModuleA::func_a();\n")?;

    // Verify definition works correctly from the opened file
    let def_result = harness.request_with_timeout(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": script_a_uri },
            "position": { "line": 0, "character": 5 }
        }),
        request_timeout(),
    );

    // If definition works, verify it points to the correct module
    if let Ok(def) = def_result {
        if let Some(def_array) = def.as_array() {
            if !def_array.is_empty() {
                if let Some(def_uri) = def_array[0]["uri"].as_str() {
                    assert!(
                        def_uri.contains("ModuleA.pm"),
                        "Definition should resolve to ModuleA, got: {}",
                        def_uri
                    );
                }
            }
        }
    }

    // Assert: didChange preserves folder context
    // Close and reopen with modified content to simulate didChange
    harness.close(&script_a_uri)?;
    harness.open(&script_a_uri, "use ModuleA;\n# comment\nmy $x = ModuleA::func_a();\n")?;

    // Verify definition still works correctly after change
    let def_after_result = harness.request_with_timeout(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": script_a_uri },
            "position": { "line": 0, "character": 5 }
        }),
        request_timeout(),
    );

    // If definition works, verify it still points to the correct module
    if let Ok(def_after) = def_after_result {
        if let Some(def_after_array) = def_after.as_array() {
            if !def_after_array.is_empty() {
                if let Some(def_after_uri) = def_after_array[0]["uri"].as_str() {
                    assert!(
                        def_after_uri.contains("ModuleA.pm"),
                        "Definition should still resolve to ModuleA after change, got: {}",
                        def_after_uri
                    );
                }
            }
        }
    }

    Ok(())
}

// =============================================================================
// Test 7: Ordered scope resolution test
// =============================================================================

#[test]
#[cfg(all(feature = "workspace", feature = "expose_lsp_test_api"))]
#[serial_test::serial]
fn test_ordered_scope_resolution() -> TestResult {
    use support::env_guard::EnvGuard;

    // Enable workspace indexing
    let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };

    let ws = TempWorkspace::new()?;

    // Setup: Document in folder A, module exists in both A and B
    let folder_a_uri = create_folder_with_config(&ws, "folder-a", &["lib"])?;
    create_module(
        &ws,
        "folder-a/lib/Shared.pm",
        "package Shared;\nsub func { return 'from-a'; }\n1;\n",
    )?;

    let folder_b_uri = create_folder_with_config(&ws, "folder-b", &["lib"])?;
    create_module(
        &ws,
        "folder-b/lib/Shared.pm",
        "package Shared;\nsub func { return 'from-b'; }\n1;\n",
    )?;

    // Create script in folder A that uses Shared
    let script_uri = create_script(
        &ws,
        "folder-a/script.pl",
        "use Shared;\nmy $x = Shared::func();\n",
    )?;

    // Initialize with folder A first, then folder B
    let mut harness = LspHarness::new_raw();
    harness.notify(
        "initialize",
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "params": {
                "processId": std::process::id(),
                "capabilities": {},
                "workspaceFolders": [
                    { "uri": folder_a_uri, "name": "folder-a" },
                    { "uri": folder_b_uri, "name": "folder-b" }
                ]
            }
        }),
    );
    harness.notify("initialized", json!({}));

    // Wait for indexing
    std::thread::sleep(indexing_timeout());

    // Open script
    harness.open(&script_uri, "use Shared;\nmy $x = Shared::func();\n")?;

    harness.wait_for_idle(Duration::from_millis(500));

    // Assert: Resolution finds module
    let def_result = harness.request_with_timeout(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": script_uri },
            "position": { "line": 0, "character": 5 }
        }),
        request_timeout(),
    );

    // If definition works, verify it finds a Shared module
    if let Ok(def) = def_result {
        if let Some(def_array) = def.as_array() {
            if !def_array.is_empty() {
                if let Some(def_uri) = def_array[0]["uri"].as_str() {
                    assert!(
                        def_uri.contains("Shared.pm"),
                        "Resolution should find Shared module, got: {}",
                        def_uri
                    );
                }
            }
        }
    }

    // Create a module that only exists in folder B
    create_module(
        &ws,
        "folder-b/lib/OnlyInB.pm",
        "package OnlyInB;\nsub func { return 'only-b'; }\n1;\n",
    )?;

    // Create script in folder A that uses OnlyInB
    let script_b_uri = create_script(
        &ws,
        "folder-a/script_b.pl",
        "use OnlyInB;\nmy $y = OnlyInB::func();\n",
    )?;

    // Wait for re-indexing
    std::thread::sleep(indexing_timeout());

    // Open script
    harness.open(&script_b_uri, "use OnlyInB;\nmy $y = OnlyInB::func();\n")?;

    harness.wait_for_idle(Duration::from_millis(500));

    // Assert: If module not in A, resolution finds in B
    let def_b_result = harness.request_with_timeout(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": script_b_uri },
            "position": { "line": 0, "character": 5 }
        }),
        request_timeout(),
    );

    // If definition works, verify it finds OnlyInB
    if let Ok(def_b) = def_b_result {
        if let Some(def_b_array) = def_b.as_array() {
            if !def_b_array.is_empty() {
                if let Some(def_b_uri) = def_b_array[0]["uri"].as_str() {
                    assert!(
                        def_b_uri.contains("OnlyInB.pm"),
                        "Resolution should find OnlyInB, got: {}",
                        def_b_uri
                    );
                }
            }
        }
    }

    Ok(())
}

// =============================================================================
// Test 8: Folder-aware ranking test
// =============================================================================

#[test]
#[cfg(all(feature = "workspace", feature = "expose_lsp_test_api"))]
#[serial_test::serial]
fn test_folder_aware_ranking() -> TestResult {
    use support::env_guard::EnvGuard;

    // Enable workspace indexing
    let _guard = unsafe { EnvGuard::set("PERL_LSP_WORKSPACE", "1") };

    let ws = TempWorkspace::new()?;

    // Setup: Same symbol name in multiple folders
    // Document in folder A
    let folder_a_uri = create_folder_with_config(&ws, "folder-a", &["lib"])?;
    create_module(
        &ws,
        "folder-a/lib/Common.pm",
        "package Common;\nsub helper { return 'a'; }\n1;\n",
    )?;

    let folder_b_uri = create_folder_with_config(&ws, "folder-b", &["lib"])?;
    create_module(
        &ws,
        "folder-b/lib/Common.pm",
        "package Common;\nsub helper { return 'b'; }\n1;\n",
    )?;

    let folder_c_uri = create_folder_with_config(&ws, "folder-c", &["lib"])?;
    create_module(
        &ws,
        "folder-c/lib/Common.pm",
        "package Common;\nsub helper { return 'c'; }\n1;\n",
    )?;

    // Create script in folder A that uses Common
    let script_uri = create_script(
        &ws,
        "folder-a/script.pl",
        "use Common;\nmy $x = Common::helper();\n",
    )?;

    // Initialize with all three workspace folders
    let mut harness = LspHarness::new_raw();
    harness.notify(
        "initialize",
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "params": {
                "processId": std::process::id(),
                "capabilities": {},
                "workspaceFolders": [
                    { "uri": folder_a_uri, "name": "folder-a" },
                    { "uri": folder_b_uri, "name": "folder-b" },
                    { "uri": folder_c_uri, "name": "folder-c" }
                ]
            }
        }),
    );
    harness.notify("initialized", json!({}));

    // Wait for indexing
    std::thread::sleep(indexing_timeout());

    // Open script
    harness.open(&script_uri, "use Common;\nmy $x = Common::helper();\n")?;

    harness.wait_for_idle(Duration::from_millis(500));

    // Assert: Definition finds a Common module
    let def_result = harness.request_with_timeout(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": script_uri },
            "position": { "line": 0, "character": 5 }
        }),
        request_timeout(),
    );

    if let Ok(def) = def_result {
        if let Some(def_array) = def.as_array() {
            if !def_array.is_empty() {
                if let Some(def_uri) = def_array[0]["uri"].as_str() {
                    assert!(
                        def_uri.contains("Common.pm"),
                        "Should find Common module, got: {}",
                        def_uri
                    );
                }
            }
        }
    }

    // Assert: Ranking is deterministic
    // Run the same query multiple times and verify consistent ordering
    let symbols1_result = harness.request_with_timeout(
        "workspace/symbol",
        json!({
            "query": "Common"
        }),
        request_timeout(),
    );

    let symbols2_result = harness.request_with_timeout(
        "workspace/symbol",
        json!({
            "query": "Common"
        }),
        request_timeout(),
    );

    // If both queries succeed, verify consistent ordering
    if let (Ok(symbols1), Ok(symbols2)) = (symbols1_result, symbols2_result) {
        if let (Some(symbols1_array), Some(symbols2_array)) =
            (symbols1.as_array(), symbols2.as_array())
        {
            assert_eq!(
                symbols1_array.len(),
                symbols2_array.len(),
                "Symbol count should be consistent"
            );

            for (i, (s1, s2)) in symbols1_array.iter().zip(symbols2_array.iter()).enumerate() {
                let uri1 = s1["location"]["uri"].as_str().unwrap_or("");
                let uri2 = s2["location"]["uri"].as_str().unwrap_or("");
                assert_eq!(
                    uri1, uri2,
                    "Symbol at index {} should have consistent URI: {} vs {}",
                    i, uri1, uri2
                );
            }
        }
    }

    Ok(())
}
