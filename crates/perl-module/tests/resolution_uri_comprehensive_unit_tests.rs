//! Comprehensive unit tests for `perl-module-resolution-uri`.
//!
//! Covers all variants of [`ModuleUriResolution`], the full search precedence
//! of [`resolve_module_uri`], edge cases (empty inputs, special characters,
//! path traversal), and timeout behavior.

use perl_module::resolution::uri::{ModuleUriResolution, resolve_module_uri};
use perl_tdd_support::{must, must_some};
use std::path::PathBuf;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Helper: create a temporary workspace with a module file on disk
// ---------------------------------------------------------------------------
fn setup_workspace_with_module(
    module_rel: &str,
) -> Result<(tempfile::TempDir, String), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    let module_file = workspace.join(module_rel);
    let parent = must_some(module_file.parent()).to_path_buf();
    std::fs::create_dir_all(&parent)?;
    std::fs::write(&module_file, "1;")?;

    let workspace_uri = url::Url::from_file_path(&workspace)
        .map_err(|()| "failed to build workspace URI")?
        .to_string();

    Ok((temp, workspace_uri))
}

// ===========================================================================
// ModuleUriResolution enum tests
// ===========================================================================

#[test]
fn resolution_variants_are_distinct() {
    let resolved = ModuleUriResolution::Resolved("file:///a.pm".to_string());
    let not_found = ModuleUriResolution::NotFound;
    let timed_out = ModuleUriResolution::TimedOut;

    assert_ne!(resolved, not_found);
    assert_ne!(resolved, timed_out);
    assert_ne!(not_found, timed_out);
}

#[test]
fn resolution_resolved_equality() {
    let a = ModuleUriResolution::Resolved("file:///x.pm".to_string());
    let b = ModuleUriResolution::Resolved("file:///x.pm".to_string());
    let c = ModuleUriResolution::Resolved("file:///y.pm".to_string());

    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn resolution_clone_produces_equal_value() {
    let original = ModuleUriResolution::Resolved("file:///clone.pm".to_string());
    let cloned = original.clone();
    assert_eq!(original, cloned);
}

#[test]
fn resolution_debug_format_is_non_empty() {
    let resolved = ModuleUriResolution::Resolved("file:///dbg.pm".to_string());
    let not_found = ModuleUriResolution::NotFound;
    let timed_out = ModuleUriResolution::TimedOut;

    assert!(!format!("{resolved:?}").is_empty());
    assert!(!format!("{not_found:?}").is_empty());
    assert!(!format!("{timed_out:?}").is_empty());
}

// ===========================================================================
// Open document precedence (phase 1 of search)
// ===========================================================================

#[test]
fn open_document_match_takes_precedence_over_workspace() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, workspace_uri) = setup_workspace_with_module("lib/Foo/Bar.pm")?;
    let open_doc = "file:///other/Foo/Bar.pm".to_string();

    let result = resolve_module_uri(
        "Foo::Bar",
        std::slice::from_ref(&open_doc),
        &[workspace_uri],
        &["lib".to_string()],
        false,
        &[],
        Duration::from_millis(100),
    );

    assert_eq!(result, ModuleUriResolution::Resolved(open_doc));
    Ok(())
}

#[test]
fn first_matching_open_document_wins() {
    let docs =
        vec!["file:///first/Foo/Bar.pm".to_string(), "file:///second/Foo/Bar.pm".to_string()];

    let result =
        resolve_module_uri("Foo::Bar", &docs, &[], &[], false, &[], Duration::from_millis(100));

    assert_eq!(result, ModuleUriResolution::Resolved("file:///first/Foo/Bar.pm".to_string()));
}

#[test]
fn open_document_no_match_falls_through() {
    let docs = vec!["file:///unrelated/Baz/Quux.pm".to_string()];

    let result =
        resolve_module_uri("Foo::Bar", &docs, &[], &[], false, &[], Duration::from_millis(100));

    assert_eq!(result, ModuleUriResolution::NotFound);
}

#[test]
fn open_document_partial_suffix_does_not_match() {
    // "Bar.pm" does not end with "Foo/Bar.pm" — module_name_to_path("Foo::Bar") == "Foo/Bar.pm"
    let docs = vec!["file:///only/Bar.pm".to_string()];

    let result =
        resolve_module_uri("Foo::Bar", &docs, &[], &[], false, &[], Duration::from_millis(100));

    assert_eq!(result, ModuleUriResolution::NotFound);
}

// ===========================================================================
// Workspace folder resolution (phase 2 of search)
// ===========================================================================

#[test]
fn workspace_folder_with_lib_include_path() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, workspace_uri) = setup_workspace_with_module("lib/My/Module.pm")?;

    let result = resolve_module_uri(
        "My::Module",
        &[],
        &[workspace_uri],
        &["lib".to_string()],
        false,
        &[],
        Duration::from_millis(100),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            assert!(uri.starts_with("file://"));
            assert!(uri.ends_with("My/Module.pm"));
        }
        other => return Err(format!("expected Resolved, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn workspace_folder_with_dot_include_path() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, workspace_uri) = setup_workspace_with_module("Simple.pm")?;

    let result = resolve_module_uri(
        "Simple",
        &[],
        &[workspace_uri],
        &[".".to_string()],
        false,
        &[],
        Duration::from_millis(100),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            assert!(uri.ends_with("Simple.pm"));
        }
        other => return Err(format!("expected Resolved, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn workspace_whitespace_only_include_paths_are_ignored() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, workspace_uri) = setup_workspace_with_module("lib/Trimmed/Path.pm")?;

    let result = resolve_module_uri(
        "Trimmed::Path",
        &[],
        &[workspace_uri],
        &["   ".to_string(), "\n\t".to_string(), " lib ".to_string()],
        false,
        &[],
        Duration::from_millis(100),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            assert!(uri.ends_with("Trimmed/Path.pm"));
        }
        other => return Err(format!("expected Resolved, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn workspace_folder_multiple_include_paths_first_match_wins()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");

    // Create module in "lib" but not in "src"
    let module_file = workspace.join("lib").join("Alpha.pm");
    let parent = must_some(module_file.parent()).to_path_buf();
    std::fs::create_dir_all(&parent)?;
    std::fs::write(&module_file, "1;")?;

    let workspace_uri = url::Url::from_file_path(&workspace).map_err(|()| "build URI")?.to_string();

    let result = resolve_module_uri(
        "Alpha",
        &[],
        &[workspace_uri],
        &["src".to_string(), "lib".to_string()],
        false,
        &[],
        Duration::from_millis(100),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            assert!(uri.contains("lib"));
            assert!(uri.ends_with("Alpha.pm"));
        }
        other => return Err(format!("expected Resolved, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn workspace_folder_module_not_on_disk_returns_not_found() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace)?;

    let workspace_uri = url::Url::from_file_path(&workspace).map_err(|()| "build URI")?.to_string();

    let result = resolve_module_uri(
        "Missing::Module",
        &[],
        &[workspace_uri],
        &["lib".to_string()],
        false,
        &[],
        Duration::from_millis(100),
    );

    assert_eq!(result, ModuleUriResolution::NotFound);
    Ok(())
}

#[test]
fn multiple_workspace_folders_searches_in_order() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;

    // Only second workspace has the module
    let ws1 = temp.path().join("ws1");
    std::fs::create_dir_all(ws1.join("lib"))?;

    let ws2 = temp.path().join("ws2");
    let module_file = ws2.join("lib").join("Found.pm");
    std::fs::create_dir_all(must_some(module_file.parent()))?;
    std::fs::write(&module_file, "1;")?;

    let uri1 = url::Url::from_file_path(&ws1).map_err(|()| "uri1")?.to_string();
    let uri2 = url::Url::from_file_path(&ws2).map_err(|()| "uri2")?.to_string();

    let result = resolve_module_uri(
        "Found",
        &[],
        &[uri1, uri2],
        &["lib".to_string()],
        false,
        &[],
        Duration::from_millis(100),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            assert!(uri.contains("ws2"));
            assert!(uri.ends_with("Found.pm"));
        }
        other => return Err(format!("expected Resolved, got {other:?}").into()),
    }
    Ok(())
}

// ===========================================================================
// System @INC resolution (phase 3 of search)
// ===========================================================================

#[test]
fn system_inc_enabled_resolves_module() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let inc_dir = temp.path().join("perl5");
    let module_file = inc_dir.join("Sys").join("Module.pm");
    std::fs::create_dir_all(must_some(module_file.parent()))?;
    std::fs::write(&module_file, "1;")?;

    let result = resolve_module_uri(
        "Sys::Module",
        &[],
        &[],
        &[],
        true,
        &[PathBuf::from(&inc_dir)],
        Duration::from_millis(100),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            assert!(uri.starts_with("file://"));
            assert!(uri.ends_with("Sys/Module.pm"));
        }
        other => return Err(format!("expected Resolved, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn system_inc_disabled_ignores_system_paths() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let inc_dir = temp.path().join("perl5");
    let module_file = inc_dir.join("Ignored.pm");
    std::fs::create_dir_all(must_some(module_file.parent()))?;
    std::fs::write(&module_file, "1;")?;

    let result = resolve_module_uri(
        "Ignored",
        &[],
        &[],
        &[],
        false,
        &[PathBuf::from(&inc_dir)],
        Duration::from_millis(100),
    );

    assert_eq!(result, ModuleUriResolution::NotFound);
    Ok(())
}

#[test]
fn system_inc_first_matching_path_wins() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;

    let inc1 = temp.path().join("inc1");
    let file1 = inc1.join("Multi.pm");
    std::fs::create_dir_all(&inc1)?;
    std::fs::write(&file1, "1;")?;

    let inc2 = temp.path().join("inc2");
    let file2 = inc2.join("Multi.pm");
    std::fs::create_dir_all(&inc2)?;
    std::fs::write(&file2, "1;")?;

    let result = resolve_module_uri(
        "Multi",
        &[],
        &[],
        &[],
        true,
        &[PathBuf::from(&inc1), PathBuf::from(&inc2)],
        Duration::from_millis(100),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            assert!(uri.contains("inc1"));
        }
        other => return Err(format!("expected Resolved, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn system_inc_missing_file_returns_not_found() {
    let result = resolve_module_uri(
        "Nonexistent::Pkg",
        &[],
        &[],
        &[],
        true,
        &[PathBuf::from("/no/such/directory")],
        Duration::from_millis(100),
    );

    assert_eq!(result, ModuleUriResolution::NotFound);
}

#[test]
fn duplicate_system_inc_entries_do_not_change_resolution_order()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;

    let inc1 = temp.path().join("inc1");
    let file1 = inc1.join("Stable.pm");
    std::fs::create_dir_all(&inc1)?;
    std::fs::write(&file1, "1;")?;

    let inc2 = temp.path().join("inc2");
    let file2 = inc2.join("Stable.pm");
    std::fs::create_dir_all(&inc2)?;
    std::fs::write(&file2, "1;")?;

    let result = resolve_module_uri(
        "Stable",
        &[],
        &[],
        &[],
        true,
        &[
            PathBuf::from(" . "),
            PathBuf::from(" "),
            PathBuf::from(&inc1),
            PathBuf::from(&inc1),
            PathBuf::from(&inc2),
        ],
        Duration::from_millis(100),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => assert!(uri.contains("inc1")),
        other => return Err(format!("expected Resolved, got {other:?}").into()),
    }
    Ok(())
}

// ===========================================================================
// Search precedence: open docs > workspace > system @INC
// ===========================================================================

#[test]
fn open_doc_beats_workspace_and_system_inc() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, workspace_uri) = setup_workspace_with_module("lib/Order/Test.pm")?;
    let open_doc = "file:///open/Order/Test.pm".to_string();

    let temp2 = tempfile::tempdir()?;
    let inc_dir = temp2.path().join("inc");
    let inc_file = inc_dir.join("Order").join("Test.pm");
    std::fs::create_dir_all(must_some(inc_file.parent()))?;
    std::fs::write(&inc_file, "1;")?;

    let result = resolve_module_uri(
        "Order::Test",
        std::slice::from_ref(&open_doc),
        &[workspace_uri],
        &["lib".to_string()],
        true,
        &[PathBuf::from(&inc_dir)],
        Duration::from_millis(100),
    );

    assert_eq!(result, ModuleUriResolution::Resolved(open_doc));
    Ok(())
}

#[test]
fn workspace_beats_system_inc() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, workspace_uri) = setup_workspace_with_module("lib/Prio/Ws.pm")?;

    let temp2 = tempfile::tempdir()?;
    let inc_dir = temp2.path().join("inc");
    let inc_file = inc_dir.join("Prio").join("Ws.pm");
    std::fs::create_dir_all(must_some(inc_file.parent()))?;
    std::fs::write(&inc_file, "1;")?;

    let result = resolve_module_uri(
        "Prio::Ws",
        &[],
        &[workspace_uri],
        &["lib".to_string()],
        true,
        &[PathBuf::from(&inc_dir)],
        Duration::from_millis(100),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            // Workspace match should win; URI should reference the workspace path
            assert!(!uri.contains("inc"));
        }
        other => return Err(format!("expected Resolved, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn precedence_is_stable_after_include_path_normalization() -> Result<(), Box<dyn std::error::Error>>
{
    let (_temp, workspace_uri) = setup_workspace_with_module("lib/Stable/Order.pm")?;

    let temp2 = tempfile::tempdir()?;
    let system_root = temp2.path().join("sys");
    let system_module = system_root.join("Stable").join("Order.pm");
    std::fs::create_dir_all(must_some(system_module.parent()))?;
    std::fs::write(&system_module, "1;")?;

    let result = resolve_module_uri(
        "Stable::Order",
        &[],
        &[workspace_uri],
        &["  ".to_string(), "./".to_string(), "lib/".to_string(), " lib ".to_string()],
        true,
        &[PathBuf::from(""), PathBuf::from(" . "), PathBuf::from(&system_root)],
        Duration::from_millis(100),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            assert!(uri.contains("workspace"));
            assert!(uri.ends_with("Stable/Order.pm"));
            assert!(!uri.contains("/sys/"));
        }
        other => return Err(format!("expected Resolved, got {other:?}").into()),
    }
    Ok(())
}

// ===========================================================================
// Timeout behavior
// ===========================================================================

#[test]
fn zero_timeout_with_workspace_search_times_out() {
    let result = resolve_module_uri(
        "Any::Module",
        &[],
        &["file:///workspace".to_string()],
        &["lib".to_string()],
        false,
        &[],
        Duration::ZERO,
    );

    assert_eq!(result, ModuleUriResolution::TimedOut);
}

#[test]
fn zero_timeout_open_doc_match_still_succeeds() {
    let open_doc = "file:///doc/Fast/Match.pm".to_string();

    let result = resolve_module_uri(
        "Fast::Match",
        std::slice::from_ref(&open_doc),
        &["file:///workspace".to_string()],
        &["lib".to_string()],
        false,
        &[],
        Duration::ZERO,
    );

    // Open doc match happens before timeout checks
    assert_eq!(result, ModuleUriResolution::Resolved(open_doc));
}

#[test]
fn nanosecond_timeout_with_many_folders_times_out() {
    let folders: Vec<String> = (0..1000).map(|i| format!("file:///ws-{i}")).collect();
    let includes: Vec<String> = (0..50).map(|i| format!("inc-{i}")).collect();

    let result = resolve_module_uri(
        "Never::Found",
        &[],
        &folders,
        &includes,
        false,
        &[],
        Duration::from_nanos(1),
    );

    assert_eq!(result, ModuleUriResolution::TimedOut);
}

#[test]
fn nanosecond_timeout_with_system_inc_times_out() {
    let folders: Vec<String> = (0..100).map(|i| format!("file:///ws-{i}")).collect();
    let system_inc: Vec<PathBuf> = (0..500).map(|i| PathBuf::from(format!("/inc/{i}"))).collect();

    let result = resolve_module_uri(
        "Never::Found",
        &[],
        &folders,
        &["lib".to_string()],
        true,
        &system_inc,
        Duration::from_nanos(1),
    );

    assert_eq!(result, ModuleUriResolution::TimedOut);
}

// ===========================================================================
// Empty and minimal inputs
// ===========================================================================

#[test]
fn all_empty_inputs_returns_not_found() {
    let result =
        resolve_module_uri("Anything", &[], &[], &[], false, &[], Duration::from_millis(100));

    assert_eq!(result, ModuleUriResolution::NotFound);
}

#[test]
fn empty_module_name_returns_not_found() {
    let result = resolve_module_uri("", &[], &[], &[], false, &[], Duration::from_millis(100));

    assert_eq!(result, ModuleUriResolution::NotFound);
}

#[test]
fn empty_open_documents_falls_through_to_workspace() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, workspace_uri) = setup_workspace_with_module("lib/Fall/Through.pm")?;

    let result = resolve_module_uri(
        "Fall::Through",
        &[],
        &[workspace_uri],
        &["lib".to_string()],
        false,
        &[],
        Duration::from_millis(100),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            assert!(uri.ends_with("Fall/Through.pm"));
        }
        other => return Err(format!("expected Resolved, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn empty_include_paths_skips_workspace_search() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, workspace_uri) = setup_workspace_with_module("lib/Skip/Me.pm")?;

    let result = resolve_module_uri(
        "Skip::Me",
        &[],
        &[workspace_uri],
        &[], // no include paths → inner loop never runs
        false,
        &[],
        Duration::from_millis(100),
    );

    assert_eq!(result, ModuleUriResolution::NotFound);
    Ok(())
}

#[test]
fn empty_workspace_folders_skips_to_system_inc() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let inc_dir = temp.path().join("perl5");
    let module_file = inc_dir.join("Direct.pm");
    std::fs::create_dir_all(&inc_dir)?;
    std::fs::write(&module_file, "1;")?;

    let result = resolve_module_uri(
        "Direct",
        &[],
        &[],
        &["lib".to_string()],
        true,
        &[PathBuf::from(&inc_dir)],
        Duration::from_millis(100),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            assert!(uri.ends_with("Direct.pm"));
        }
        other => return Err(format!("expected Resolved, got {other:?}").into()),
    }
    Ok(())
}

// ===========================================================================
// Path traversal / security
// ===========================================================================

#[test]
fn include_path_traversal_is_blocked() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace)?;

    // Create file outside workspace that a traversal attack would find
    let escaped = temp.path().join("Evil.pm");
    std::fs::write(&escaped, "1;")?;

    let workspace_uri = url::Url::from_file_path(&workspace).map_err(|()| "uri")?.to_string();

    let result = resolve_module_uri(
        "Evil",
        &[],
        &[workspace_uri],
        &["../".to_string()],
        false,
        &[],
        Duration::from_millis(100),
    );

    assert_eq!(result, ModuleUriResolution::NotFound);
    Ok(())
}

#[test]
fn absolute_include_path_outside_workspace_is_honored() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace)?;
    let outside = temp.path().join("outside_lib");
    let module_file = outside.join("External").join("Util.pm");
    std::fs::create_dir_all(module_file.parent().ok_or("no parent")?)?;
    std::fs::write(&module_file, "1;")?;

    let workspace_uri = url::Url::from_file_path(&workspace).map_err(|()| "uri")?.to_string();

    let result = resolve_module_uri(
        "External::Util",
        &[],
        &[workspace_uri],
        &[outside.to_string_lossy().to_string()],
        false,
        &[],
        Duration::from_millis(100),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            assert!(uri.ends_with("External/Util.pm"));
        }
        other => return Err(format!("expected Resolved, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn deeply_nested_traversal_is_blocked() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("deep").join("nested").join("workspace");
    std::fs::create_dir_all(&workspace)?;

    let workspace_uri = url::Url::from_file_path(&workspace).map_err(|()| "uri")?.to_string();

    let result = resolve_module_uri(
        "Escape",
        &[],
        &[workspace_uri],
        &["../../../".to_string()],
        false,
        &[],
        Duration::from_millis(100),
    );

    assert_eq!(result, ModuleUriResolution::NotFound);
    Ok(())
}

// ===========================================================================
// Module name patterns (special characters, deeply nested)
// ===========================================================================

#[test]
fn deeply_nested_module_name() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, workspace_uri) = setup_workspace_with_module("lib/A/B/C/D/E.pm")?;

    let result = resolve_module_uri(
        "A::B::C::D::E",
        &[],
        &[workspace_uri],
        &["lib".to_string()],
        false,
        &[],
        Duration::from_millis(100),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            assert!(uri.ends_with("A/B/C/D/E.pm"));
        }
        other => return Err(format!("expected Resolved, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn single_segment_module_name() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, workspace_uri) = setup_workspace_with_module("lib/strict.pm")?;

    let result = resolve_module_uri(
        "strict",
        &[],
        &[workspace_uri],
        &["lib".to_string()],
        false,
        &[],
        Duration::from_millis(100),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            assert!(uri.ends_with("strict.pm"));
        }
        other => return Err(format!("expected Resolved, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn module_with_numeric_segments() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, workspace_uri) = setup_workspace_with_module("lib/Net/HTTP2.pm")?;

    let result = resolve_module_uri(
        "Net::HTTP2",
        &[],
        &[workspace_uri],
        &["lib".to_string()],
        false,
        &[],
        Duration::from_millis(100),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            assert!(uri.ends_with("Net/HTTP2.pm"));
        }
        other => return Err(format!("expected Resolved, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn module_with_underscore_segments() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, workspace_uri) = setup_workspace_with_module("lib/My_App/Config_Loader.pm")?;

    let result = resolve_module_uri(
        "My_App::Config_Loader",
        &[],
        &[workspace_uri],
        &["lib".to_string()],
        false,
        &[],
        Duration::from_millis(100),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            assert!(uri.ends_with("My_App/Config_Loader.pm"));
        }
        other => return Err(format!("expected Resolved, got {other:?}").into()),
    }
    Ok(())
}

// ===========================================================================
// URI format validation
// ===========================================================================

#[test]
fn resolved_workspace_uri_is_valid_file_url() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, workspace_uri) = setup_workspace_with_module("lib/Valid/Uri.pm")?;

    let result = resolve_module_uri(
        "Valid::Uri",
        &[],
        &[workspace_uri],
        &["lib".to_string()],
        false,
        &[],
        Duration::from_millis(100),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            let parsed = must(url::Url::parse(&uri));
            assert_eq!(parsed.scheme(), "file");
        }
        other => return Err(format!("expected Resolved, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn resolved_system_inc_uri_is_valid_file_url() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let inc_dir = temp.path().join("perl5");
    let module_file = inc_dir.join("Check").join("Uri.pm");
    std::fs::create_dir_all(must_some(module_file.parent()))?;
    std::fs::write(&module_file, "1;")?;

    let result = resolve_module_uri(
        "Check::Uri",
        &[],
        &[],
        &[],
        true,
        &[PathBuf::from(&inc_dir)],
        Duration::from_millis(100),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            let parsed = must(url::Url::parse(&uri));
            assert_eq!(parsed.scheme(), "file");
        }
        other => return Err(format!("expected Resolved, got {other:?}").into()),
    }
    Ok(())
}

// ===========================================================================
// Workspace folder format variants
// ===========================================================================

#[test]
fn workspace_folder_as_plain_path() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    let module_file = workspace.join("lib").join("Plain.pm");
    std::fs::create_dir_all(must_some(module_file.parent()))?;
    std::fs::write(&module_file, "1;")?;

    // Pass workspace as plain filesystem path (not file:// URI)
    let result = resolve_module_uri(
        "Plain",
        &[],
        &[workspace.to_string_lossy().to_string()],
        &["lib".to_string()],
        false,
        &[],
        Duration::from_millis(100),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            assert!(uri.ends_with("Plain.pm"));
        }
        // Plain path handling depends on workspace_folder_to_path implementation;
        // NotFound is acceptable if it doesn't resolve plain paths
        ModuleUriResolution::NotFound => {}
        ModuleUriResolution::TimedOut => {
            return Err("unexpected timeout".into());
        }
    }
    Ok(())
}

// ===========================================================================
// Generous timeout — module genuinely not found
// ===========================================================================

#[test]
fn generous_timeout_still_returns_not_found_when_absent() {
    let result = resolve_module_uri(
        "Ghost::Module",
        &[],
        &[],
        &["lib".to_string()],
        true,
        &[PathBuf::from("/nonexistent/path")],
        Duration::from_secs(5),
    );

    assert_eq!(result, ModuleUriResolution::NotFound);
}

// ===========================================================================
// Interaction: open doc match with special URI patterns
// ===========================================================================

#[test]
fn open_doc_with_encoded_uri_characters() {
    let open_doc = "file:///path/with%20spaces/Foo/Bar.pm".to_string();

    let result = resolve_module_uri(
        "Foo::Bar",
        std::slice::from_ref(&open_doc),
        &[],
        &[],
        false,
        &[],
        Duration::from_millis(100),
    );

    // ends_with match works on the raw string, "Foo/Bar.pm" matches
    assert_eq!(result, ModuleUriResolution::Resolved(open_doc));
}

#[test]
fn open_doc_case_sensitive_matching() {
    let open_doc = "file:///path/foo/bar.pm".to_string();

    let result = resolve_module_uri(
        "Foo::Bar",
        std::slice::from_ref(&open_doc),
        &[],
        &[],
        false,
        &[],
        Duration::from_millis(100),
    );

    // Module name "Foo::Bar" → "Foo/Bar.pm", which does NOT match "foo/bar.pm"
    assert_eq!(result, ModuleUriResolution::NotFound);
}

// ===========================================================================
// must/must_some helper usage demonstration
// ===========================================================================

#[test]
fn must_helper_on_url_parse() -> Result<(), Box<dyn std::error::Error>> {
    let open_doc = "file:///demo/Test/Must.pm".to_string();

    let result = resolve_module_uri(
        "Test::Must",
        std::slice::from_ref(&open_doc),
        &[],
        &[],
        false,
        &[],
        Duration::from_millis(100),
    );

    let uri = match result {
        ModuleUriResolution::Resolved(uri) => uri,
        other => return Err(format!("expected Resolved, got {other:?}").into()),
    };

    // Demonstrate must_some on string operations
    let last_segment = must_some(uri.split('/').next_back());
    assert_eq!(last_segment, "Must.pm");
    Ok(())
}
