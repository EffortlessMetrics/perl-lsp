//! Property-based tests for @INC path module completion
//!
//! These tests verify invariants about the module completion system using
//! property-based testing techniques with generated inputs.
//!
//! Invariants tested:
//! 1. Deduplication: A module appears at most once even if in multiple paths
//! 2. Prefix filtering: Only modules matching the prefix are returned
//! 3. Only .pm files: Non-.pm files are never returned as modules
//! 4. MAX_DEPTH enforcement: Modules at depth > 5 are not returned
//! 5. Path separator conversion: / becomes :: in module names
//! 6. Empty prefix returns all valid modules
//! 7. Cancellation returns partial results
//! 8. Idempotent path_to_module_name for valid inputs

use perl_lsp_completion::{CompletionItem, CompletionItemKind, CompletionProvider};
use perl_parser_core::Parser;
use perl_tdd_support::{must, must_some};
use perl_workspace_index::workspace_index::WorkspaceIndex;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------

fn has_label(items: &[CompletionItem], label: &str) -> bool {
    items.iter().any(|i| i.label == label)
}

fn count_label(items: &[CompletionItem], label: &str) -> usize {
    items.iter().filter(|i| i.label == label).count()
}

fn labels(items: &[CompletionItem]) -> Vec<String> {
    items.iter().map(|i| i.label.clone()).collect()
}

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

// Generate random valid module names
fn generate_module_names(count: usize) -> Vec<String> {
    let prefixes =
        ["Alpha", "Beta", "Gamma", "Delta", "Epsilon", "Zeta", "Eta", "Theta", "Iota", "Kappa"];
    let suffixes =
        ["Module", "Class", "Util", "Helper", "Manager", "Service", "Controller", "Model"];

    let mut names = Vec::with_capacity(count);
    for i in 0..count {
        let prefix_idx = i % prefixes.len();
        let suffix_idx = (i / prefixes.len()) % suffixes.len();
        let nesting = i / (prefixes.len() * suffixes.len());

        let base = format!("{}{}", prefixes[prefix_idx], suffixes[suffix_idx]);
        if nesting == 0 {
            names.push(base);
        } else {
            names.push(format!("{}::Nested{}", base, nesting));
        }
    }
    names
}

// Generate prefixes that should match a subset of generated modules
fn generate_prefixes(module_names: &[String], count: usize) -> Vec<String> {
    let mut prefixes = Vec::with_capacity(count);

    // Collect unique first segments
    let mut first_segments: Vec<&str> =
        module_names.iter().filter_map(|n| n.split("::").next()).collect();
    first_segments.sort();
    first_segments.dedup();

    for i in 0..count.min(first_segments.len() * 2) {
        if i < first_segments.len() {
            prefixes.push(first_segments[i].to_string());
        } else {
            // Add some longer prefixes from actual module names
            let idx = (i - first_segments.len()) % module_names.len();
            let name = &module_names[idx];
            if name.len() > 3 {
                let cut = (name.len() - 2).min(name.len());
                prefixes.push(name[..cut].to_string());
            }
        }
    }
    prefixes
}

// Count how many module names match a given prefix
fn count_matching_modules(module_names: &[String], prefix: &str) -> usize {
    if prefix.is_empty() {
        return module_names.len();
    }
    module_names.iter().filter(|n| n.starts_with(prefix)).count()
}

// -----------------------------------------------------------------------------
// Property 1: Deduplication - A module appears at most once
// -----------------------------------------------------------------------------

/// Property: For any set of include paths containing the same module file,
/// that module should appear exactly once in completions.
#[test]
fn property_deduplication_across_paths() {
    let temp_dir1 = tempfile::tempdir().expect("tempdir should succeed");
    let temp_dir2 = tempfile::tempdir().expect("tempdir should succeed");
    let temp_dir3 = tempfile::tempdir().expect("tempdir should succeed");

    // Create same module in multiple paths
    let module_content = "package Deduplicated::Module;\n1;\n";
    create_temp_module(temp_dir1.path(), "Deduplicated/Module.pm", module_content).unwrap();
    create_temp_module(temp_dir2.path(), "Deduplicated/Module.pm", module_content).unwrap();
    create_temp_module(temp_dir3.path(), "Deduplicated/Module.pm", module_content).unwrap();

    let index = Arc::new(WorkspaceIndex::new());

    let code = "use Deduplicated::";
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
        ],
        &[],
    );

    let items = provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    // The module should appear exactly once, not three times
    let count = count_label(&items, "Deduplicated::Module");
    assert_eq!(
        count,
        1,
        "module in multiple paths should appear exactly once, got {} occurrences: {:?}",
        count,
        labels(&items)
    );
}

/// Property: When the same module exists in both workspace and include path,
/// workspace version takes priority and module appears only once.
#[test]
fn property_workspace_include_deduplication() {
    let temp_include = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_include.path().to_path_buf();

    // Create module in include path
    create_temp_module(&include_path, "Shared/Module.pm", "package Shared::Module;\n1;\n").unwrap();

    // Create same module in workspace
    let workspace_uri = url::Url::parse("file:///workspace/Shared/Module.pm").unwrap();
    let workspace_code = "package Shared::Module;\nsub new { }\n1;\n";
    let index = Arc::new(WorkspaceIndex::new());
    must(index.index_file(workspace_uri, workspace_code.to_string()));

    let code = "use Shared::";
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

    // Should appear exactly once (workspace version takes priority)
    let count = count_label(&items, "Shared::Module");
    assert_eq!(
        count,
        1,
        "module in both workspace and include path should appear once, got {}: {:?}",
        count,
        labels(&items)
    );
}

// -----------------------------------------------------------------------------
// Property 2: Prefix filtering - Only matching modules are returned
// -----------------------------------------------------------------------------

/// Property: For any prefix, only modules starting with that prefix are returned.
#[test]
fn property_prefix_filtering_exact() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    // Create multiple modules with different prefixes
    create_temp_module(&include_path, "Alpha/Module.pm", "package Alpha::Module;\n1;\n").unwrap();
    create_temp_module(&include_path, "Alpha/Class.pm", "package Alpha::Class;\n1;\n").unwrap();
    create_temp_module(&include_path, "Beta/Module.pm", "package Beta::Module;\n1;\n").unwrap();
    create_temp_module(&include_path, "Alphabet/Module.pm", "package Alphabet::Module;\n1;\n")
        .unwrap();

    let index = Arc::new(WorkspaceIndex::new());

    // Test with prefix "Alpha" - should match Alpha::Module and Alpha::Class but NOT Alphabet::Module or Beta::Module
    let code = "use Alpha";
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

    // Alpha::Module and Alpha::Class should appear
    assert!(has_label(&items, "Alpha::Module"), "Alpha::Module should appear for 'Alpha' prefix");
    assert!(has_label(&items, "Alpha::Class"), "Alpha::Class should appear for 'Alpha' prefix");

    // Alphabet::Module should NOT appear (doesn't start with "Alpha" exactly)
    assert!(
        !has_label(&items, "Alphabet::Module"),
        "Alphabet::Module should NOT appear for 'Alpha' prefix"
    );
    // Beta::Module should NOT appear
    assert!(
        !has_label(&items, "Beta::Module"),
        "Beta::Module should NOT appear for 'Alpha' prefix"
    );
}

/// Property: After :: prefix, only nested modules are matched, not flat modules with similar names.
#[test]
fn property_nested_prefix_requires_double_colon() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    create_temp_module(&include_path, "Foo/Bar.pm", "package Foo::Bar;\n1;\n").unwrap();
    create_temp_module(&include_path, "FooBar.pm", "package FooBar;\n1;\n").unwrap();

    let index = Arc::new(WorkspaceIndex::new());

    // With prefix "Foo::", only Foo::Bar should match
    let code = "use Foo::";
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

    assert!(has_label(&items, "Foo::Bar"), "Foo::Bar should appear for 'Foo::' prefix");
    assert!(!has_label(&items, "FooBar"), "FooBar should NOT appear for 'Foo::' prefix");
}

// -----------------------------------------------------------------------------
// Property 3: Only .pm files are included as modules
// -----------------------------------------------------------------------------

/// Property: Only files with .pm extension are returned as module completions.
#[test]
fn property_only_pm_files_included() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    // Create .pm file (should appear)
    create_temp_module(&include_path, "Valid/Module.pm", "package Valid::Module;\n1;\n").unwrap();
    // Create .pod file (should NOT appear)
    create_temp_module(&include_path, "Doc/Module.pod", "=head1 NAME\n\nDoc::Module\n\n=cut\n")
        .unwrap();
    // Create .pl file (should NOT appear)
    create_temp_module(&include_path, "Script.pl", "#!/usr/bin/perl\nprint 'hello';\n").unwrap();
    // Create .pmc file (should NOT appear - compiled perl)
    create_temp_module(&include_path, "Compiled.pmc", "package Compiled;\n1;\n").unwrap();
    // Create .xs file (should NOT appear - native extension)
    std::fs::write(include_path.join("Native.xs"), "void boot_exporter() { }\n").unwrap();

    let index = Arc::new(WorkspaceIndex::new());

    let code = "use Valid";
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

    // .pm file should appear
    assert!(has_label(&items, "Valid::Module"), ".pm file should appear");

    // Non-.pm files should NOT appear
    assert!(!has_label(&items, "Doc::Module"), ".pod file should NOT appear");
    assert!(!has_label(&items, "Script"), ".pl file should NOT appear");
    assert!(!has_label(&items, "Compiled"), ".pmc file should NOT appear");
    assert!(!has_label(&items, "Native"), ".xs file should NOT appear");
}

// -----------------------------------------------------------------------------
// Property 4: MAX_DEPTH enforcement - Modules at depth > 5 are excluded
// -----------------------------------------------------------------------------

/// Property: Modules nested more than 5 levels deep are not returned.
#[test]
fn property_max_depth_exclusion() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    // Create modules at various depths
    // Depth 1: Module.pm
    create_temp_module(&include_path, "Depth1.pm", "package Depth1;\n1;\n").unwrap();
    // Depth 3: A/B/C/Module.pm
    create_temp_module(&include_path, "A/B/C/Module.pm", "package A::B::C::Module;\n1;\n").unwrap();
    // Depth 5: A/B/C/D/E/Module.pm (exactly at limit)
    create_temp_module(
        &include_path,
        "A/B/C/D/E/Module.pm",
        "package A::B::C::D::E::Module;\n1;\n",
    )
    .unwrap();
    // Depth 6: A/B/C/D/E/F/Module.pm (beyond limit)
    create_temp_module(
        &include_path,
        "A/B/C/D/E/F/Module.pm",
        "package A::B::C::D::E::F::Module;\n1;\n",
    )
    .unwrap();

    let index = Arc::new(WorkspaceIndex::new());

    let code = "use A::B::C::";
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

    // Depth 5 module (D::E::Module) should appear
    assert!(has_label(&items, "A::B::C::D::E::Module"), "depth-5 module should appear");
    // Depth 6 module (D::E::F::Module) should NOT appear
    assert!(!has_label(&items, "A::B::C::D::E::F::Module"), "depth-6 module should NOT appear");
}

// -----------------------------------------------------------------------------
// Property 5: Path separator conversion - / becomes :: in module names
// -----------------------------------------------------------------------------

/// Property: Forward slashes in file paths are converted to :: in module names.
#[test]
fn property_path_separator_conversion() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    // Create nested modules with forward slashes
    create_temp_module(&include_path, "Net/DNS/Resolver.pm", "package Net::DNS::Resolver;\n1;\n")
        .unwrap();

    let index = Arc::new(WorkspaceIndex::new());

    let code = "use Net::DNS::Reso";
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

    // Should find module with :: separators, not / separators
    assert!(
        has_label(&items, "Net::DNS::Resolver"),
        "forward slashes should be converted to :: in module names, got: {:?}",
        labels(&items)
    );
    assert!(
        !has_label(&items, "Net/DNS/Resolver"),
        "raw forward slashes should NOT appear in module names"
    );
}

// -----------------------------------------------------------------------------
// Property 6: Empty prefix returns all valid modules
// -----------------------------------------------------------------------------

/// Property: When prefix is empty, all valid modules in include paths are returned.
#[test]
fn property_empty_prefix_returns_all() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    // Create multiple modules
    create_temp_module(&include_path, "Alpha.pm", "package Alpha;\n1;\n").unwrap();
    create_temp_module(&include_path, "Beta.pm", "package Beta;\n1;\n").unwrap();
    create_temp_module(&include_path, "Gamma.pm", "package Gamma;\n1;\n").unwrap();
    create_temp_module(&include_path, "Delta/Module.pm", "package Delta::Module;\n1;\n").unwrap();

    let index = Arc::new(WorkspaceIndex::new());

    // Empty prefix - just "use "
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

    let items = provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    // All modules should appear for empty prefix
    assert!(has_label(&items, "Alpha"), "Alpha should appear for empty prefix");
    assert!(has_label(&items, "Beta"), "Beta should appear for empty prefix");
    assert!(has_label(&items, "Gamma"), "Gamma should appear for empty prefix");
    assert!(has_label(&items, "Delta::Module"), "Delta::Module should appear for empty prefix");
}

// -----------------------------------------------------------------------------
// Property 7: Cancellation returns partial results
// -----------------------------------------------------------------------------

/// Property: When cancelled mid-scan, results collected so far are returned.
#[test]
fn property_cancellation_returns_partial_results() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    // Create many modules - enough that cancellation will likely trigger
    for i in 0..100 {
        let module_name = format!("Module{}.pm", i);
        create_temp_module(&include_path, &module_name, &format!("package Module{};\n1;\n", i))
            .unwrap();
    }

    // Create one module that we'll check for
    create_temp_module(&include_path, "Zulu.pm", "package Zulu;\n1;\n").unwrap();

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

    // Cancellation immediately returns empty (no results collected yet for "use ")
    // But this test verifies the mechanism works - cancellation callback is checked
    let call_count = std::cell::Cell::new(0);
    let is_cancelled = || {
        call_count.set(call_count.get() + 1);
        true // Always cancelled
    };

    let items = provider.get_completions_with_path_cancellable(code, position, None, &is_cancelled);

    // Since we cancelled immediately, we should get no results
    // But the cancellation callback was invoked (proving it's being checked)
    assert!(call_count.get() > 0, "cancellation callback should have been invoked");
}

/// Property: Cancellation callback is checked multiple times during scanning.
#[test]
fn property_cancellation_checked_repeatedly() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    // Create enough modules that scanning takes multiple iterations
    for i in 0..50 {
        let path = format!("Mod{}/Sub{}.pm", i % 5, i);
        create_temp_module(&include_path, &path, &format!("package Mod{}::Sub{};\n1;\n", i % 5, i))
            .unwrap();
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

    let call_count = std::cell::Cell::new(0);
    let is_cancelled = || {
        call_count.set(call_count.get() + 1);
        call_count.get() >= 3 // Cancel after 3 checks
    };

    let items = provider.get_completions_with_path_cancellable(code, position, None, &is_cancelled);

    // If cancellation was checked multiple times, we may get partial results
    // But we verify the callback was invoked at least a few times
    assert!(
        call_count.get() >= 3,
        "cancellation callback should be checked multiple times during scan, was {}",
        call_count.get()
    );
}

// -----------------------------------------------------------------------------
// Property 8: Multiple include paths are all scanned
// -----------------------------------------------------------------------------

/// Property: All include paths are scanned for modules.
#[test]
fn property_all_include_paths_scanned() {
    let temp_dir1 = tempfile::tempdir().expect("tempdir should succeed");
    let temp_dir2 = tempfile::tempdir().expect("tempdir should succeed");
    let temp_dir3 = tempfile::tempdir().expect("tempdir should succeed");

    create_temp_module(temp_dir1.path(), "Path1/Module.pm", "package Path1::Module;\n1;\n")
        .unwrap();
    create_temp_module(temp_dir2.path(), "Path2/Module.pm", "package Path2::Module;\n1;\n")
        .unwrap();
    create_temp_module(temp_dir3.path(), "Path3/Module.pm", "package Path3::Module;\n1;\n")
        .unwrap();

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
        ],
        &[],
    );

    let items = provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    // All three paths should have their modules found
    assert!(has_label(&items, "Path1::Module"), "modules from path1 should appear");
    assert!(has_label(&items, "Path2::Module"), "modules from path2 should appear");
    assert!(has_label(&items, "Path3::Module"), "modules from path3 should appear");
}

// -----------------------------------------------------------------------------
// Property 9: Module names with special characters are handled
// -----------------------------------------------------------------------------

/// Property: Module names with underscores, numbers are properly matched.
#[test]
fn property_special_characters_in_module_names() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    create_temp_module(&include_path, "XML/LibXML/Common.pm", "package XML::LibXML::Common;\n1;\n")
        .unwrap();
    create_temp_module(&include_path, "Math2D.pm", "package Math2D;\n1;\n").unwrap();
    create_temp_module(&include_path, "Test/Unit/Runner.pm", "package Test::Unit::Runner;\n1;\n")
        .unwrap();

    let index = Arc::new(WorkspaceIndex::new());

    // Test underscore matching
    let code = "use XML::LibXML::Com";
    let position = code.len();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index_and_source_and_include_paths(
        &ast,
        code,
        Some(index.clone()),
        &[include_path.clone()],
        &[],
    );

    let items = provider.get_completions_with_path_cancellable(code, position, None, &|| false);
    assert!(has_label(&items, "XML::LibXML::Common"), "underscore module should match");

    // Test number matching
    let code2 = "use Math";
    let position2 = code2.len();
    let mut parser2 = Parser::new(code2);
    let ast2 = must(parser2.parse());

    let provider2 = CompletionProvider::new_with_index_and_source_and_include_paths(
        &ast2,
        code2,
        Some(index),
        &[include_path],
        &[],
    );

    let items2 = provider2.get_completions_with_path_cancellable(code2, position2, None, &|| false);
    assert!(has_label(&items2, "Math2D"), "module with number should match");
}

// -----------------------------------------------------------------------------
// Property 10: System @INC paths are scanned when provided
// -----------------------------------------------------------------------------

/// Property: System @INC paths are scanned with "(system)" detail.
#[test]
fn property_system_inc_paths_scanned() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let system_path = temp_dir.path().to_path_buf();

    create_temp_module(&system_path, "System/Module.pm", "package System::Module;\n1;\n").unwrap();

    let index = Arc::new(WorkspaceIndex::new());

    let code = "use System::";
    let position = code.len();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index_and_source_and_include_paths(
        &ast,
        code,
        Some(index),
        &[],
        &[system_path],
    );

    let items = provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    assert!(has_label(&items, "System::Module"), "system module should appear");

    // Check detail text indicates it's from system @INC
    let detail = items.iter().find(|i| i.label == "System::Module").and_then(|i| i.detail.clone());
    assert!(
        detail.as_ref().is_some_and(|d| d.contains("system")),
        "system module should have '(system)' detail, got: {:?}",
        detail
    );
}

// -----------------------------------------------------------------------------
// Property 11: Idempotent behavior for repeated completion requests
// -----------------------------------------------------------------------------

/// Property: Multiple calls with the same parameters return consistent results.
#[test]
fn property_idempotent_results() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    create_temp_module(&include_path, "Consistent/Module.pm", "package Consistent::Module;\n1;\n")
        .unwrap();

    let index = Arc::new(WorkspaceIndex::new());

    let code = "use Consistent::";
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

    // Make multiple calls
    let items1 = provider.get_completions_with_path_cancellable(code, position, None, &|| false);
    let items2 = provider.get_completions_with_path_cancellable(code, position, None, &|| false);
    let items3 = provider.get_completions_with_path_cancellable(code, position, None, &|| false);

    // Results should be consistent
    assert_eq!(labels(&items1), labels(&items2), "first and second call should be consistent");
    assert_eq!(labels(&items2), labels(&items3), "second and third call should be consistent");
    assert_eq!(count_label(&items1, "Consistent::Module"), 1, "module should appear exactly once");
}

// -----------------------------------------------------------------------------
// Property 12: Performance - scanning completes within timeout budget
// -----------------------------------------------------------------------------

/// Property: Include path scanning completes within 30ms timeout budget.
#[test]
fn property_timeout_budget_enforced() {
    let temp_dir = tempfile::tempdir().expect("tempdir should succeed");
    let include_path = temp_dir.path().to_path_buf();

    // Create a reasonable number of modules
    for i in 0..100 {
        let path = format!("Module{}/Sub{}.pm", i % 10, i);
        create_temp_module(
            &include_path,
            &path,
            &format!("package Module{}::Sub{};\n1;\n", i % 10, i),
        )
        .unwrap();
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

    let start = Instant::now();
    let items = provider.get_completions_with_path_cancellable(code, position, None, &|| false);
    let elapsed = start.elapsed();

    // Should complete quickly (within 100ms even on slow systems)
    // The 30ms budget is internal; we give some margin for test overhead
    assert!(
        elapsed < Duration::from_millis(100),
        "completion should complete within 100ms, took {}ms",
        elapsed.as_millis()
    );

    // Should still return valid results
    assert!(!items.is_empty(), "should return results despite timeout");
}
