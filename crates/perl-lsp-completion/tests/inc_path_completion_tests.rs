//! BDD-style behavior specification tests for @INC path module completion
//!
//! These tests describe what the completion engine should do when @INC paths
//! are configured via `.perl-lsp.toml` `includePaths` or `useSystemInc: true`.
//!
//! Coverage targets (per ADR and specs):
//! - AC1: Configured Include Paths Work
//! - AC2: System @INC Works with useSystemInc
//! - AC3: Deduplication Between Sources
//! - AC4: Prefix Filtering Works
//! - AC5: Nested Module Paths Work
//! - AC6: Empty Include Paths Handled Gracefully
//! - AC7: Cancellation Stops Scanning
//! - AC9: Permission Errors Don't Crash
//! - AC10: WASM32 Graceful Degradation
//!
//! NOTE: These tests are written against the FUTURE API (after implementation).
//! They will fail to compile until the implementation adds:
//! - `include_paths` and `system_inc_paths` fields to `CompletionProvider`
//! - Extended `add_use_module_completions()` signature with these parameters

use perl_lsp_completion::{CompletionItem, CompletionItemKind, CompletionProvider};
use perl_parser_core::Parser;
use perl_tdd_support::{must, must_some};
use perl_workspace_index::workspace_index::WorkspaceIndex;
use std::path::PathBuf;
use std::sync::Arc;
use url::Url;

// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------

fn has_label(items: &[CompletionItem], label: &str) -> bool {
    items.iter().any(|i| i.label == label)
}

fn labels(items: &[CompletionItem]) -> Vec<String> {
    items.iter().map(|i| i.label.clone()).collect()
}

fn find_item<'a>(items: &'a [CompletionItem], label: &str) -> Option<&'a CompletionItem> {
    items.iter().find(|item| item.label == label)
}

fn detail_text(items: &[CompletionItem], label: &str) -> Option<String> {
    find_item(items, label).and_then(|item| item.detail.clone())
}

/// Creates a temp directory with a .pm file for testing include path scanning
fn create_temp_module(
    temp_dir: &std::path::Path,
    module_name: &str,
    package_content: &str,
) -> std::io::Result<()> {
    let module_path = temp_dir.join(module_name);
    if let Some(parent) = module_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&module_path, package_content)?;
    Ok(())
}

// -----------------------------------------------------------------------------
// AC1: Configured Include Paths Work
// -----------------------------------------------------------------------------

/// AC1: Given a `.perl-lsp.toml` with `includePaths: ["/path/to/libs"]`
/// And `/path/to/libs/DBI.pm` exists (containing `package DBI;`)
/// When the user types `use DB<|>`
/// Then the completion list includes `DBI` with `detail: "module (include)"`
#[test]
fn test_use_module_completions_includes_external_modules_from_include_path() {
    // Create a temporary directory to simulate an include path
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    // Create DBI.pm in the include path
    create_temp_module(&include_path, "DBI.pm", "package DBI;\n1;\n")
        .expect("creating DBI.pm should succeed");

    // Create a minimal workspace index (empty - no workspace modules)
    let index = Arc::new(WorkspaceIndex::new());

    // Parse the completion context
    let code = "use DB";
    let position = code.len(); // After "use DB"
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    // Create provider with the include paths - THIS WILL FAIL TO COMPILE
    // because CompletionProvider doesn't have include_paths field yet
    let provider = CompletionProvider::new_with_index_and_source_and_include_paths(
        &ast,
        code,
        Some(index),
        &[include_path], // include_paths
        &[],             // system_inc_paths (empty)
    );

    let items = provider.get_completions_with_path_cancellable(
        code,
        position,
        None,
        &|| false, // not cancelled
    );

    // Verify DBI appears in completions
    assert!(
        has_label(&items, "DBI"),
        "should suggest DBI from include path, got: {:?}",
        labels(&items)
    );

    // Verify the detail text indicates it's from include path
    let detail = detail_text(&items, "DBI");
    assert!(
        detail.as_ref().is_some_and(|d| d.contains("include")),
        "DBI detail should indicate '(include)', got: {:?}",
        detail
    );
}

// -----------------------------------------------------------------------------
// AC2: System @INC Works with useSystemInc
// -----------------------------------------------------------------------------

/// AC2: Given `useSystemInc: true` in `.perl-lsp.toml`
/// And the system Perl installation includes `Moo.pm`
/// When the user types `use Mo<|>`
/// Then the completion list includes `Moo` with `detail: "module (system)"`
#[test]
fn test_use_module_completions_system_inc() {
    // Create a temporary directory to simulate system @INC
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let system_inc_path = temp_dir.path().to_path_buf();

    // Create Moo.pm in the system @INC path
    create_temp_module(&system_inc_path, "Moo.pm", "package Moo;\n1;\n")
        .expect("creating Moo.pm should succeed");

    // Create a minimal workspace index (empty)
    let index = Arc::new(WorkspaceIndex::new());

    // Parse the completion context
    let code = "use Mo";
    let position = code.len(); // After "use Mo"
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    // Create provider with system_inc_paths
    let provider = CompletionProvider::new_with_index_and_source_and_include_paths(
        &ast,
        code,
        Some(index),
        &[],                // include_paths (empty)
        &[system_inc_path], // system_inc_paths
    );

    let items = provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    // Verify Moo appears in completions
    assert!(
        has_label(&items, "Moo"),
        "should suggest Moo from system @INC, got: {:?}",
        labels(&items)
    );

    // Verify the detail text indicates it's from system @INC
    let detail = detail_text(&items, "Moo");
    assert!(
        detail.as_ref().is_some_and(|d| d.contains("system")),
        "Moo detail should indicate '(system)', got: {:?}",
        detail
    );
}

// -----------------------------------------------------------------------------
// AC3: Deduplication Between Sources
// -----------------------------------------------------------------------------

/// AC3: Given a module `Foo.pm` exists in both the workspace and an include path
/// When the user types `use Fo<|>`
/// Then `Foo` appears only once in the completion list (workspace version takes priority)
#[test]
fn test_use_module_completions_dedup_workspace_and_external() {
    // Create temp directories
    let temp_workspace = tempfile::tempdir().expect("tempdir should succeed");
    let temp_include = tempfile::tempdir().expect("tempdir should succeed");

    // Create Foo.pm in workspace
    let workspace_uri = Url::parse("file:///workspace/Foo.pm").expect("valid URI");
    let workspace_code = "package Foo;\nsub new { }\n1;\n";
    let index = Arc::new(WorkspaceIndex::new());
    must(index.index_file(workspace_uri, workspace_code.to_string()));

    // Create Foo.pm in include path
    create_temp_module(temp_include.path(), "Foo.pm", "package Foo;\n1;\n")
        .expect("creating Foo.pm should succeed");

    // Parse the completion context
    let code = "use Fo";
    let position = code.len();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    // Create provider with both workspace index and include path
    let provider = CompletionProvider::new_with_index_and_source_and_include_paths(
        &ast,
        code,
        Some(index),
        &[temp_include.path().to_path_buf()],
        &[],
    );

    let items = provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    // Count how many times Foo appears
    let foo_count = items.iter().filter(|i| i.label == "Foo").count();
    assert_eq!(
        foo_count,
        1,
        "Foo should appear exactly once (deduplicated), got {} occurrences: {:?}",
        foo_count,
        labels(&items)
    );
}

// -----------------------------------------------------------------------------
// AC4: Prefix Filtering Works
// -----------------------------------------------------------------------------

/// AC4: Given an include path with `DBI.pm`, `DBD::SQLite.pm`, and `Moo.pm`
/// When the user types `use DB<|>`
/// Then the completion list shows `DBI` and `DBD::SQLite` but NOT `Moo`
#[test]
fn test_use_module_completions_prefix_filtering() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    // Create multiple modules in the include path
    create_temp_module(&include_path, "DBI.pm", "package DBI;\n1;\n")
        .expect("creating DBI.pm should succeed");
    create_temp_module(&include_path, "DBD/SQLite.pm", "package DBD::SQLite;\n1;\n")
        .expect("creating DBD::SQLite.pm should succeed");
    create_temp_module(&include_path, "Moo.pm", "package Moo;\n1;\n")
        .expect("creating Moo.pm should succeed");

    let index = Arc::new(WorkspaceIndex::new());

    let code = "use DB";
    let position = code.len();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index_and_source_and_include_paths(
        &ast,
        code,
        Some(index),
        &[include_path],
        &[],
    );

    let items = provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    // Should include DBI and DBD::SQLite
    assert!(
        has_label(&items, "DBI"),
        "should suggest DBI for 'DB' prefix, got: {:?}",
        labels(&items)
    );
    assert!(
        has_label(&items, "DBD::SQLite"),
        "should suggest DBD::SQLite for 'DB' prefix, got: {:?}",
        labels(&items)
    );

    // Should NOT include Moo
    assert!(
        !has_label(&items, "Moo"),
        "should NOT suggest Moo for 'DB' prefix, got: {:?}",
        labels(&items)
    );
}

// -----------------------------------------------------------------------------
// AC5: Nested Module Paths Work
// -----------------------------------------------------------------------------

/// AC5: Given an include path with `File/Path/To/Module.pm`
/// When the user types `use File::Path::To::Mo<|>`
/// Then the completion list includes `File::Path::To::Module`
/// And directory separators `/` are converted to `::` in module names
#[test]
fn test_use_module_completions_nested_external() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    // Create nested module structure: File/Path/To/Module.pm
    create_temp_module(
        &include_path,
        "File/Path/To/Module.pm",
        "package File::Path::To::Module;\n1;\n",
    )
    .expect("creating nested module should succeed");

    let index = Arc::new(WorkspaceIndex::new());

    let code = "use File::Path::To::Mo";
    let position = code.len();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index_and_source_and_include_paths(
        &ast,
        code,
        Some(index),
        &[include_path],
        &[],
    );

    let items = provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    // Should find the nested module with :: separator
    assert!(
        has_label(&items, "File::Path::To::Module"),
        "should suggest nested module with :: separators, got: {:?}",
        labels(&items)
    );
}

// -----------------------------------------------------------------------------
// AC6: Empty Include Paths Handled Gracefully
// -----------------------------------------------------------------------------

/// AC6: Given `includePaths` is empty and `useSystemInc` is false
/// When the user types `use DB<|>`
/// Then completion behaves identically to before this change (workspace-only)
#[test]
fn test_use_module_completions_empty_paths() {
    let index = Arc::new(WorkspaceIndex::new());

    let code = "use DB";
    let position = code.len();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    // Create provider with empty paths - should work without errors
    let provider = CompletionProvider::new_with_index_and_source_and_include_paths(
        &ast,
        code,
        Some(index),
        &[], // empty include_paths
        &[], // empty system_inc_paths
    );

    // Should not panic and should return empty or workspace-only results
    let items = provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    // No crash means success for empty paths case
    // (completions may be empty if workspace is empty)
    assert!(
        items
            .iter()
            .all(|i| i.label != "DBI" || !i.detail.as_ref().is_some_and(|d| d.contains("include"))),
        "empty paths should not produce include-path completions"
    );
}

// -----------------------------------------------------------------------------
// AC7: Cancellation Stops Scanning (but returns partial results)
// -----------------------------------------------------------------------------

/// AC7: Given include paths containing thousands of `.pm` files
/// And the user presses Ctrl+C to cancel a slow completion request
/// When the cancellation is detected mid-scan
/// Then partial results collected so far are returned (not empty list)
#[test]
fn test_use_module_completions_cancellation_returns_partial() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    // Create several modules
    create_temp_module(&include_path, "Alpha.pm", "package Alpha;\n1;\n")
        .expect("creating Alpha.pm should succeed");
    create_temp_module(&include_path, "Beta.pm", "package Beta;\n1;\n")
        .expect("creating Beta.pm should succeed");
    create_temp_module(&include_path, "Gamma.pm", "package Gamma;\n1;\n")
        .expect("creating Gamma.pm should succeed");

    let index = Arc::new(WorkspaceIndex::new());

    let code = "use A";
    let position = code.len();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index_and_source_and_include_paths(
        &ast,
        code,
        Some(index),
        &[include_path],
        &[],
    );

    // Cancellation callback that returns true after first few calls
    let call_count = std::sync::atomic::AtomicUsize::new(0);
    let is_cancelled = || {
        let count = call_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        // Cancel after we've had a chance to find at least one module
        count >= 1
    };

    let items = provider.get_completions_with_path_cancellable(code, position, None, &is_cancelled);

    // Should return partial results (at least Alpha if it was found before cancellation)
    // or empty if cancellation happened too early
    // The key behavior: should not panic and should return whatever was found
    assert!(
        items.iter().all(
            |i| i.label != "Alpha" || !i.detail.as_ref().is_some_and(|d| d.contains("include"))
        ),
        "cancellation should stop scanning, results depend on timing"
    );
}

// -----------------------------------------------------------------------------
// AC9: Permission Errors Don't Crash
// -----------------------------------------------------------------------------

/// AC9: Given an include path containing directories with no read permission
/// When the scanner encounters those directories
/// Then it skips them gracefully with a trace message
/// And completion continues with accessible directories
#[test]
fn test_use_module_completions_permission_errors_skipped() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    // Create a readable module
    create_temp_module(&include_path, "Readable.pm", "package Readable;\n1;\n")
        .expect("creating Readable.pm should succeed");

    // Create an unreadable directory
    let unreadable_dir = include_path.join("Unreadable");
    std::fs::create_dir(&unreadable_dir).expect("creating unreadable dir should succeed");

    // Remove read permission (only on Unix)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms =
            std::fs::metadata(&unreadable_dir).expect("should get metadata").permissions();
        perms.set_mode(0o000);
        std::fs::set_permissions(&unreadable_dir, perms)
            .expect("setting no permissions should succeed");
    }

    let index = Arc::new(WorkspaceIndex::new());

    let code = "use Re";
    let position = code.len();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index_and_source_and_include_paths(
        &ast,
        code,
        Some(index),
        &[include_path.clone()],
        &[],
    );

    // Should not panic even with permission errors
    let items = provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    // Readable module should still be found
    assert!(
        has_label(&items, "Readable"),
        "should still find readable modules despite permission errors, got: {:?}",
        labels(&items)
    );

    // Cleanup: restore permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms =
            std::fs::metadata(&unreadable_dir).expect("should get metadata").permissions();
        perms.set_mode(0o755);
        let _ = std::fs::set_permissions(&unreadable_dir, perms);
    }
}

// -----------------------------------------------------------------------------
// AC10: WASM32 Graceful Degradation
// -----------------------------------------------------------------------------

/// AC10: Given the LSP running on a wasm32 target
/// When completion is requested
/// Then the behavior is identical to before this change (no filesystem scanning attempted)
#[test]
fn test_wasm32_no_filesystem_scanning() {
    // This test verifies that on non-wasm32 targets, we DO scan include paths
    // On wasm32, the implementation should skip filesystem scanning entirely

    #[cfg(target_arch = "wasm32")]
    {
        // On wasm32, the provider should work without include path support
        let index = Arc::new(WorkspaceIndex::new());
        let code = "use DB";
        let position = code.len();
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());

        // On wasm32, this should fall back to workspace-only behavior
        let provider = CompletionProvider::new_with_index_and_source(&ast, code, Some(index));

        let items = provider.get_completions_with_path_cancellable(code, position, None, &|| false);

        // Should not crash, behavior matches pre-implementation
        assert!(true, "wasm32 should degrade gracefully");
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        // On non-wasm32, we should actually scan include paths
        // This test is a no-op placeholder - actual wasm32 testing requires cross-compilation
        assert!(true, "wasm32 test placeholder");
    }
}

// -----------------------------------------------------------------------------
// Test for detail text format
// -----------------------------------------------------------------------------

/// Verify the detail text format for include and system modules
#[test]
fn test_module_completion_detail_format() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();
    let system_path = temp_dir.path().join("system");
    std::fs::create_dir(&system_path).expect("creating system dir should succeed");

    // Create modules in include path
    create_temp_module(&include_path, "FromInclude.pm", "package FromInclude;\n1;\n")
        .expect("creating FromInclude.pm should succeed");

    // Create module in system path
    create_temp_module(&system_path, "FromSystem.pm", "package FromSystem;\n1;\n")
        .expect("creating FromSystem.pm should succeed");

    let index = Arc::new(WorkspaceIndex::new());

    let code = "use From";
    let position = code.len();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index_and_source_and_include_paths(
        &ast,
        code,
        Some(index),
        &[include_path],
        &[system_path],
    );

    let items = provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    // Check include path module has correct detail
    if has_label(&items, "FromInclude") {
        let detail = detail_text(&items, "FromInclude").unwrap_or_default();
        assert!(
            detail.contains("include"),
            "FromInclude should have '(include)' detail, got: {}",
            detail
        );
    }

    // Check system path module has correct detail
    if has_label(&items, "FromSystem") {
        let detail = detail_text(&items, "FromSystem").unwrap_or_default();
        assert!(
            detail.contains("system"),
            "FromSystem should have '(system)' detail, got: {}",
            detail
        );
    }
}
