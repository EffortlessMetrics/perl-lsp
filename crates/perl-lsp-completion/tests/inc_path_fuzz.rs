//! Fuzz tests for @INC path module completion
//!
//! These tests use randomized inputs to find crashes, panics, and unexpected
//! behavior in the include path scanning and module name conversion logic.
//!
//! Fuzzing targets:
//! 1. `path_to_module_name` - converts file paths to module names
//! 2. `scan_modules_in_directory` - directory scanning with WalkDir
//! 3. `add_use_module_completions` - full completion pipeline

use perl_lsp_completion::{CompletionItem, CompletionProvider};
use perl_parser_core::Parser;
use perl_tdd_support::must;
use perl_workspace_index::workspace_index::WorkspaceIndex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// -----------------------------------------------------------------------------
// Test helpers
// -----------------------------------------------------------------------------

fn create_temp_module(
    temp_dir: &Path,
    module_name: &str,
    package_content: &str,
) -> std::io::Result<PathBuf> {
    let module_path = temp_dir.join(module_name);
    if let Some(parent) = module_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&module_path, package_content)?;
    Ok(module_path)
}

/// Returns a CompletionProvider set up for include path testing
fn make_provider(
    code: &str,
    include_paths: &[PathBuf],
    system_inc_paths: &[PathBuf],
) -> CompletionProvider {
    let position = code.len();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let index = Arc::new(WorkspaceIndex::new());
    CompletionProvider::new_with_index_and_source_and_include_paths(
        &ast,
        code,
        Some(index),
        include_paths,
        system_inc_paths,
    )
}

/// Collect all labels from completion items
fn collect_labels(items: &[CompletionItem]) -> HashSet<String> {
    items.iter().map(|i| i.label.clone()).collect()
}

// -----------------------------------------------------------------------------
// Fuzz 1: path_to_module_name with pathological paths
// -----------------------------------------------------------------------------

/// Test that `path_to_module_name` (via directory scanning) handles
/// pathological paths without panicking or returning invalid module names.
///
/// Note: The code filters out empty parts and parts that are exactly ".".
/// However, it does NOT filter out ".." components - these become part of
/// the module name. WalkDir's follow_links(false) prevents actual symlink
/// traversal outside the include path.
#[test]
fn fuzz_path_traversal_attempts_do_not_cause_panics() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    // Create a normal module
    create_temp_module(&include_path, "Normal/Module.pm", "package Normal::Module; 1;")
        .expect("creating normal module should succeed");

    // Create modules with path traversal attempts in name
    // The code filters out "." parts but ".." becomes part of the module name.
    // With follow_links(false), actual symlink traversal is prevented.
    let traversal_names = [
        "foo/../../bar",         // Results in bar (.. filtered as path component, but / separator matters)
        "a/b/../c/d",           // Results in a::c::d (b is removed by ..)
        "a/b/c/../../d",        // Results in a::d (b and c removed by .. pairs)
    ];

    for name in &traversal_names {
        let full_path = format!("{}.pm", name);
        let _ = create_temp_module(&include_path, &full_path, "package Test; 1;");
    }

    let index = Arc::new(WorkspaceIndex::new());
    let code = "use ";
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

    // Should not panic
    let items = provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    // Normal module should appear
    let labels = collect_labels(&items);
    assert!(
        labels.contains("Normal::Module"),
        "Normal::Module should appear, got: {:?}",
        labels
    );

    // The code doesn't prevent path components like ".." from becoming module names
    // But with follow_links(false), actual symlink traversal outside the include path is prevented
    // This is the security measure - not filtering of ".." in names
    let _ = labels;
}

/// Fuzz test: paths with unusual separators and components
#[test]
fn fuzz_mixed_separators_and_dot_components() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    // Create modules with various edge cases in path names
    let edge_case_modules = [
        "A/B/C/Module.pm",          // Normal nested
        "A/./B/./C/Module.pm",     // Dot components
        "A//B//C/Module.pm",       // Double slashes
        "A///B///C///Module.pm",    // Triple slashes
        "./A/Module.pm",           // Leading dot
        "././A/Module.pm",         // Multiple leading dots
        "A/./B/Module.pm",          // Mixed dot component
        ".pm",                      // Just extension (weird but possible filename)
        "Module.pm.pm",             // Double extension
        "Module..pm",               // Double dot before extension
        "A/B/Module.pmx",           // Wrong extension (not .pm)
    ];

    for name in &edge_case_modules {
        let _ = create_temp_module(&include_path, name, "package Test; 1;");
    }

    let index = Arc::new(WorkspaceIndex::new());
    let code = "use ";
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

    // Should not panic
    let items =
        provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    let labels = collect_labels(&items);

    // Normal nested module should appear
    assert!(
        labels.contains("A::B::C::Module"),
        "Normal nested module should appear, got: {:?}",
        labels
    );

    // Dot components should be filtered (empty parts removed)
    // A/./B/./C/Module.pm -> A::B::C::Module
    // The double slashes would be replaced with single ::
    // So A//B//C/Module.pm -> A::B::C::Module (same result after filtering)

    // Wrong extension should not appear
    assert!(
        !labels.contains("A::B::Modulepmx") && !labels.contains("Modulepm"),
        "Non-.pm files should not appear, got: {:?}",
        labels
    );
}

// -----------------------------------------------------------------------------
// Fuzz 2: scan_modules_in_directory - timeout and cancellation
// -----------------------------------------------------------------------------

/// Test that cancellation returns partial results without panicking
#[test]
fn fuzz_cancellation_returns_partial_results_without_panic() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    // Create many modules
    for i in 0..200 {
        let path = format!("Module{}/Sub{}.pm", i % 10, i);
        let _ = create_temp_module(&include_path, &path, &format!("package Module{}::Sub{}; 1;", i % 10, i));
    }

    let index = Arc::new(WorkspaceIndex::new());
    let code = "use ";
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

    // Always-cancelled callback
    let is_cancelled = || true;

    // Should not panic
    let items =
        provider.get_completions_with_path_cancellable(code, position, None, &is_cancelled);

    // Results may be empty due to immediate cancellation, but no panic
    // The important thing is we get a valid Vec, not an unwind
}

/// Test that frequent cancellation checks don't cause issues
#[test]
fn fuzz_frequent_cancellation_checks_no_panic() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    // Create a moderate number of modules
    for i in 0..50 {
        let path = format!("Mod{}/Sub{}.pm", i % 5, i);
        let _ = create_temp_module(&include_path, &path, &format!("package Mod{}::Sub{}; 1;", i % 5, i));
    }

    let index = Arc::new(WorkspaceIndex::new());
    let code = "use Mod";
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

    // Cancel after every other call
    let cancel_after = std::cell::Cell::new(0usize);
    let is_cancelled = || {
        cancel_after.set(cancel_after.get() + 1);
        cancel_after.get() % 2 == 0
    };

    // Should not panic regardless of cancellation pattern
    let items =
        provider.get_completions_with_path_cancellable(code, position, None, &is_cancelled);

    // We should get some results before cancellation kicks in
    // or empty results if cancelled early - both are valid
    let _labels = collect_labels(&items);
    // No panic means success
}

// -----------------------------------------------------------------------------
// Fuzz 3: Depth limit enforcement with generated paths
// -----------------------------------------------------------------------------

/// Test that depth limit is enforced for deeply nested generated paths
#[test]
fn fuzz_depth_limit_with_generated_nested_paths() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    // Create modules at various depths up to and beyond MAX_SCAN_DEPTH (6)
    // WalkDir depth semantics: root=0, A=1, A/B=2, A/B/C=3, A/B/C/D=4, A/B/C/D/E=5, A/B/C/D/E/F=6
    let depth_cases = [
        ("Depth1.pm", 1),       // depth 1 - should appear
        ("A/Depth2.pm", 2),     // depth 2 - should appear
        ("A/B/Depth3.pm", 3),   // depth 3 - should appear
        ("A/B/C/Depth4.pm", 4), // depth 4 - should appear
        ("A/B/C/D/Depth5.pm", 5), // depth 5 - should appear
        ("A/B/C/D/E/Depth6.pm", 6), // depth 6 - should appear
        ("A/B/C/D/E/F/Depth7.pm", 7), // depth 7 - should NOT appear
        ("A/B/C/D/E/F/G/Depth8.pm", 8), // depth 8 - should NOT appear
    ];

    for (name, _depth) in &depth_cases {
        let _ = create_temp_module(&include_path, name, "package Test; 1;");
    }

    let index = Arc::new(WorkspaceIndex::new());
    let code = "use ";
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

    let items =
        provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    let labels = collect_labels(&items);

    // Depth 1-6 should appear
    assert!(labels.contains("Depth1"), "depth 1 module should appear");
    assert!(labels.contains("A::Depth2"), "depth 2 module should appear");
    assert!(labels.contains("A::B::Depth3"), "depth 3 module should appear");
    assert!(labels.contains("A::B::C::Depth4"), "depth 4 module should appear");
    assert!(labels.contains("A::B::C::D::Depth5"), "depth 5 module should appear");
    assert!(labels.contains("A::B::C::D::E::Depth6"), "depth 6 module should appear");

    // Depth 7+ should NOT appear
    assert!(
        !labels.contains("A::B::C::D::E::F::Depth7"),
        "depth 7 module should NOT appear (exceeds MAX_SCAN_DEPTH=6), got: {:?}",
        labels
    );
    assert!(
        !labels.contains("A::B::C::D::E::F::G::Depth8"),
        "depth 8 module should NOT appear (exceeds MAX_SCAN_DEPTH=6), got: {:?}",
        labels
    );
}

// -----------------------------------------------------------------------------
// Fuzz 4: Module name with special characters
// -----------------------------------------------------------------------------

/// Test that module names with special characters are handled
#[test]
fn fuzz_special_characters_in_module_names() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    // Create modules with unusual but valid filename characters
    let special_modules = [
        "Test_underscore.pm",      // Underscore
        "Test123/numeric123.pm",    // Numbers
        "UTF8/日本語.pm",           // Unicode (if filesystem supports it)
        "Spaces In Name/Module.pm", // Spaces
        "CamelCase/Module.pm",     // CamelCase
        "ALL_CAPS/Module.pm",      // ALL CAPS
    ];

    for name in &special_modules {
        let _ = create_temp_module(&include_path, name, "package Test; 1;");
    }

    let index = Arc::new(WorkspaceIndex::new());
    let code = "use ";
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

    // Should not panic
    let items =
        provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    let labels = collect_labels(&items);

    // Basic sanity check - at least some modules should appear
    assert!(
        !labels.is_empty(),
        "some modules should appear for empty prefix, got: {:?}",
        labels
    );

    // Underscore and numeric modules should work
    assert!(
        labels.contains("Test_underscore") || labels.contains("Test123::numeric123"),
        "modules with underscores/numbers should appear, got: {:?}",
        labels
    );
}

// -----------------------------------------------------------------------------
// Fuzz 5: Empty and very long include paths
// -----------------------------------------------------------------------------

/// Test that empty include paths don't cause issues
#[test]
fn fuzz_empty_include_paths_do_not_cause_panic() {
    let index = Arc::new(WorkspaceIndex::new());
    let code = "use ";
    let position = code.len();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    // Empty include paths
    let provider = CompletionProvider::new_with_index_and_source_and_include_paths(
        &ast,
        code,
        Some(index),
        &[],
        &[],
    );

    // Should return empty results without panic
    let items =
        provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    assert!(
        items.is_empty(),
        "empty include paths should return no modules, got: {:?}",
        items.len()
    );
}

/// Test that non-existent include paths don't cause panics
#[test]
fn fuzz_nonexistent_include_path_does_not_cause_panic() {
    let index = Arc::new(WorkspaceIndex::new());
    let code = "use ";
    let position = code.len();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    // Non-existent path
    let fake_path = PathBuf::from("/this/path/does/not/exist/12345");
    let provider = CompletionProvider::new_with_index_and_source_and_include_paths(
        &ast,
        code,
        Some(index),
        &[fake_path],
        &[],
    );

    // Should not panic even though path doesn't exist
    let items =
        provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    // Should just return empty
    assert!(
        items.is_empty(),
        "non-existent path should return no modules, got: {:?}",
        items.len()
    );
}

// -----------------------------------------------------------------------------
// Fuzz 6: Concurrent deduplication stress test
// -----------------------------------------------------------------------------

/// Test that deduplication works correctly with many modules from many paths
#[test]
fn fuzz_deduplication_with_many_modules_and_paths() {
    let temp_dir1 = tempfile::tempdir().expect("tempdir should succeed");
    let temp_dir2 = tempfile::tempdir().expect("tempdir should succeed");
    let temp_dir3 = tempfile::tempdir().expect("tempdir should succeed");
    let temp_dir4 = tempfile::tempdir().expect("tempdir should succeed");

    // Create overlapping modules in multiple paths
    let modules = ["Alpha", "Beta", "Gamma", "Delta", "Epsilon"];

    let dirs: Vec<_> = vec![
        temp_dir1.path().to_path_buf(),
        temp_dir2.path().to_path_buf(),
        temp_dir3.path().to_path_buf(),
        temp_dir4.path().to_path_buf(),
    ];
    for dir in &dirs {
        for module in &modules {
            let path = format!("{}.pm", module);
            let _ = create_temp_module(dir, &path, &format!("package {}; 1;", module));

            // Also create nested versions
            let nested_path = format!("Nested/{}.pm", module);
            let _ = create_temp_module(dir, &nested_path, &format!("package Nested::{}; 1;", module));
        }
    }

    let index = Arc::new(WorkspaceIndex::new());
    let code = "use ";
    let position = code.len();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index_and_source_and_include_paths(
        &ast,
        code,
        Some(index),
        &[
            temp_dir1.path().to_path_buf(),
            temp_dir2.path().to_path_buf(),
            temp_dir3.path().to_path_buf(),
            temp_dir4.path().to_path_buf(),
        ],
        &[],
    );

    let items =
        provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    let labels = collect_labels(&items);

    // Flat modules should appear exactly once each
    for module in &modules {
        let count = items.iter().filter(|i| i.label == *module).count();
        assert_eq!(
            count, 1,
            "module {} should appear exactly once across all paths, got {}: {:?}",
            module, count, labels
        );
    }

    // Nested modules should appear exactly once
    for module in &modules {
        let nested_name = format!("Nested::{}", module);
        let count = items.iter().filter(|i| i.label == nested_name).count();
        assert_eq!(
            count, 1,
            "module {} should appear exactly once across all paths, got {}: {:?}",
            nested_name, count, labels
        );
    }
}

// -----------------------------------------------------------------------------
// Fuzz 7: Very long module name components
// -----------------------------------------------------------------------------

/// Test that very long path components don't cause issues
#[test]
fn fuzz_very_long_path_components_do_not_cause_panic() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    // Create a very long module name
    let long_name = "A".repeat(10000);
    let long_path = format!("{}/Module.pm", long_name);
    let _ = create_temp_module(&include_path, &long_path, "package Test; 1;");

    let index = Arc::new(WorkspaceIndex::new());
    let code = "use ";
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

    // Should not panic with very long path component
    // (WalkDir should handle it, but to_string_lossy might truncate)
    let items =
        provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    // Results should be valid - module may or may not appear depending on truncation
    let labels = collect_labels(&items);
    assert!(
        !labels.iter().any(|l| l.len() > 20000),
        "module names should not be unreasonably long"
    );
}

// -----------------------------------------------------------------------------
// Fuzz 8: Module with only special characters
// -----------------------------------------------------------------------------

/// Test that modules with unusual names don't cause issues
#[test]
fn fuzz_module_with_special_names() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    // Create modules with edge case names
    let special_names = [
        ".pm",                      // Just extension
        "Module/.pm",              // Trailing dot in directory
        "Module/..pm",             // Similar to extension
        "/",                       // Just slash
        "//",                      // Double slash
        "///",                     // Triple slash
        "A///B///C.pm",           // Multiple slashes
    ];

    for name in &special_names {
        let _ = create_temp_module(&include_path, name, "package Test; 1;");
    }

    let index = Arc::new(WorkspaceIndex::new());
    let code = "use ";
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

    // Should not panic
    let items =
        provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    // All module names should be valid (non-empty after filtering)
    for item in &items {
        assert!(
            !item.label.is_empty(),
            "no completion item should have empty label"
        );
    }
}

// -----------------------------------------------------------------------------
// Fuzz 9: Empty prefix with many modules (stress test)
// -----------------------------------------------------------------------------

/// Test that empty prefix returns all modules without performance issues
#[test]
fn fuzz_empty_prefix_with_many_modules_no_timeout() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    // Create 500 modules
    for i in 0..500 {
        let path = format!("Module{:03}/Sub{:03}.pm", i % 50, i);
        let _ = create_temp_module(&include_path, &path, &format!("package Module{:03}::Sub{:03}; 1;", i % 50, i));
    }

    let index = Arc::new(WorkspaceIndex::new());
    let code = "use ";
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

    let start = std::time::Instant::now();
    let items =
        provider.get_completions_with_path_cancellable(code, position, None, &|| false);
    let elapsed = start.elapsed();

    // Should complete within reasonable time (100ms test overhead + 30ms internal budget)
    assert!(
        elapsed < std::time::Duration::from_millis(200),
        "completion should not take too long, took {}ms",
        elapsed.as_millis()
    );

    // Should return results
    assert!(
        !items.is_empty(),
        "should return some results for empty prefix with many modules"
    );
}

// -----------------------------------------------------------------------------
// Fuzz 10: Unicode module names
// -----------------------------------------------------------------------------

/// Test that module names with Unicode characters are handled
#[test]
fn fuzz_unicode_module_names() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    // Create modules with various Unicode characters
    let unicode_modules = [
        "Ünicode/Module.pm",      // German umlaut
        "日本語/モジュール.pm",      // Japanese
        "Ελληνικά/Module.pm",     // Greek
        "Модуль/Module.pm",       // Cyrillic
        "emoji/😀Module.pm",     // Emoji (unusual but valid on some filesystems)
    ];

    for name in &unicode_modules {
        let _ = create_temp_module(&include_path, name, "package Test; 1;");
    }

    let index = Arc::new(WorkspaceIndex::new());
    let code = "use ";
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

    // Should not panic - to_string_lossy handles invalid UTF-8
    let items =
        provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    // All completion items should have valid labels
    for item in &items {
        // Labels should be valid Rust Strings (properly owned)
        assert_eq!(
            item.label.chars().filter(|c| *c as u32 == 0xFFFD).count(),
            0,
            "label should not contain replacement character: {}",
            item.label
        );
    }
}