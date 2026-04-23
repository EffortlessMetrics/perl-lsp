//! Edge-case integration tests for perl-module-resolution.
//!
//! Covers scenarios missing from the existing test suite:
//! - Deeply nested module names
//! - Multiple workspace folders with precedence
//! - Multiple include paths with priority ordering
//! - System @INC enabled with successful resolution
//! - Modules with underscores and numbers in names
//! - Empty module name edge case

use perl_module::resolution::{ModuleUriResolution, resolve_module_path, resolve_module_uri};
use std::path::PathBuf;
use std::time::Duration;

// ---------------------------------------------------------------------------
// resolve_module_path: deeply nested module
// ---------------------------------------------------------------------------

#[test]
fn path_resolves_deeply_nested_module() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().to_path_buf();
    let deep = root.join("lib").join("A").join("B").join("C").join("D.pm");

    std::fs::create_dir_all(deep.parent().ok_or("no parent")?)?;
    std::fs::write(&deep, "package A::B::C::D; 1;")?;

    let resolved = resolve_module_path(&root, "A::B::C::D", &["lib".to_string()]);
    assert_eq!(resolved, Some(root.join("lib").join("A/B/C/D.pm")));
    Ok(())
}

// ---------------------------------------------------------------------------
// resolve_module_path: multiple include paths respect priority
// ---------------------------------------------------------------------------

#[test]
fn path_first_include_path_wins() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().to_path_buf();

    let first = root.join("first_lib").join("My").join("Mod.pm");
    let second = root.join("second_lib").join("My").join("Mod.pm");

    std::fs::create_dir_all(first.parent().ok_or("no parent")?)?;
    std::fs::create_dir_all(second.parent().ok_or("no parent")?)?;
    std::fs::write(&first, "package My::Mod; 'first';")?;
    std::fs::write(&second, "package My::Mod; 'second';")?;

    let resolved =
        resolve_module_path(&root, "My::Mod", &["first_lib".to_string(), "second_lib".to_string()]);

    assert_eq!(resolved, Some(first));
    Ok(())
}

// ---------------------------------------------------------------------------
// resolve_module_path: module with underscores and numbers
// ---------------------------------------------------------------------------

#[test]
fn path_resolves_module_with_underscores_and_numbers() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().to_path_buf();
    let module_file = root.join("lib").join("Foo_Bar").join("Baz123.pm");

    std::fs::create_dir_all(module_file.parent().ok_or("no parent")?)?;
    std::fs::write(&module_file, "package Foo_Bar::Baz123; 1;")?;

    let resolved = resolve_module_path(&root, "Foo_Bar::Baz123", &["lib".to_string()]);
    assert_eq!(resolved, Some(module_file));
    Ok(())
}

// ---------------------------------------------------------------------------
// resolve_module_path: empty module name returns lib fallback path
// ---------------------------------------------------------------------------

#[test]
fn path_empty_module_name_returns_fallback() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().to_path_buf();

    let resolved = resolve_module_path(&root, "", &[]);
    // Empty name should still produce a path (the lib fallback) without panicking
    assert!(resolved.is_some());
    Ok(())
}

// ---------------------------------------------------------------------------
// resolve_module_uri: multiple workspace folders
// ---------------------------------------------------------------------------

#[test]
fn uri_resolves_from_second_workspace_folder() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let ws1 = temp.path().join("workspace1");
    let ws2 = temp.path().join("workspace2");

    std::fs::create_dir_all(ws1.join("lib"))?;
    // Module only exists in workspace2
    let module_file = ws2.join("lib").join("Only").join("Here.pm");
    std::fs::create_dir_all(module_file.parent().ok_or("no parent")?)?;
    std::fs::write(&module_file, "package Only::Here; 1;")?;

    let ws1_uri = url::Url::from_file_path(&ws1).map_err(|()| "failed to create URI")?.to_string();
    let ws2_uri = url::Url::from_file_path(&ws2).map_err(|()| "failed to create URI")?.to_string();

    let result = resolve_module_uri(
        "Only::Here",
        &[],
        &[ws1_uri, ws2_uri],
        &["lib".to_string()],
        false,
        &[],
        Duration::from_millis(200),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            assert!(uri.contains("workspace2"), "should resolve from workspace2");
            assert!(uri.ends_with("Only/Here.pm"));
        }
        other => return Err(format!("expected Resolved, got {other:?}").into()),
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// resolve_module_uri: system @INC enabled with successful resolution
// ---------------------------------------------------------------------------

#[test]
fn uri_system_inc_enabled_resolves_module() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let system_dir = temp.path().join("perl5_lib");
    let module_file = system_dir.join("Sys").join("Module.pm");

    std::fs::create_dir_all(module_file.parent().ok_or("no parent")?)?;
    std::fs::write(&module_file, "package Sys::Module; 1;")?;

    let result = resolve_module_uri(
        "Sys::Module",
        &[],
        &[],
        &["lib".to_string()],
        true,
        &[PathBuf::from(&system_dir)],
        Duration::from_millis(200),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            assert!(uri.contains("Sys/Module.pm") || uri.contains("Sys%2FModule.pm"));
        }
        other => return Err(format!("expected Resolved via system INC, got {other:?}").into()),
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// resolve_module_uri: system @INC disabled does NOT search system paths
// ---------------------------------------------------------------------------

#[test]
fn uri_system_inc_disabled_skips_system_paths() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let system_dir = temp.path().join("perl5_lib");
    let module_file = system_dir.join("Hidden").join("Module.pm");

    std::fs::create_dir_all(module_file.parent().ok_or("no parent")?)?;
    std::fs::write(&module_file, "package Hidden::Module; 1;")?;

    let result = resolve_module_uri(
        "Hidden::Module",
        &[],
        &[],
        &["lib".to_string()],
        false,
        &[PathBuf::from(&system_dir)],
        Duration::from_millis(200),
    );

    assert_eq!(result, ModuleUriResolution::NotFound);
    Ok(())
}

// ---------------------------------------------------------------------------
// resolve_module_uri: deeply nested module across workspace + include
// ---------------------------------------------------------------------------

#[test]
fn uri_resolves_deeply_nested_module() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("ws");
    let deep = workspace.join("custom_lib").join("X").join("Y").join("Z").join("W.pm");

    std::fs::create_dir_all(deep.parent().ok_or("no parent")?)?;
    std::fs::write(&deep, "package X::Y::Z::W; 1;")?;

    let ws_uri =
        url::Url::from_file_path(&workspace).map_err(|()| "failed to create URI")?.to_string();

    let result = resolve_module_uri(
        "X::Y::Z::W",
        &[],
        &[ws_uri],
        &["custom_lib".to_string()],
        false,
        &[],
        Duration::from_millis(200),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            assert!(uri.contains("X/Y/Z/W.pm"));
        }
        other => return Err(format!("expected Resolved for deep module, got {other:?}").into()),
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// resolve_module_uri: multiple include paths with priority
// ---------------------------------------------------------------------------

#[test]
fn uri_first_include_path_wins() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("ws");

    let first = workspace.join("alpha_lib").join("Prio").join("Test.pm");
    let second = workspace.join("beta_lib").join("Prio").join("Test.pm");

    std::fs::create_dir_all(first.parent().ok_or("no parent")?)?;
    std::fs::create_dir_all(second.parent().ok_or("no parent")?)?;
    std::fs::write(&first, "package Prio::Test; 'alpha';")?;
    std::fs::write(&second, "package Prio::Test; 'beta';")?;

    let ws_uri =
        url::Url::from_file_path(&workspace).map_err(|()| "failed to create URI")?.to_string();

    let result = resolve_module_uri(
        "Prio::Test",
        &[],
        &[ws_uri],
        &["alpha_lib".to_string(), "beta_lib".to_string()],
        false,
        &[],
        Duration::from_millis(200),
    );

    match result {
        ModuleUriResolution::Resolved(uri) => {
            assert!(uri.contains("alpha_lib"), "first include path should win, got: {uri}");
        }
        other => return Err(format!("expected Resolved, got {other:?}").into()),
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// resolve_module_uri: open document takes precedence over filesystem
// ---------------------------------------------------------------------------

#[test]
fn uri_open_document_takes_precedence_over_filesystem() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("ws");
    let on_disk = workspace.join("lib").join("Dup").join("Mod.pm");

    std::fs::create_dir_all(on_disk.parent().ok_or("no parent")?)?;
    std::fs::write(&on_disk, "package Dup::Mod; 1;")?;

    let ws_uri =
        url::Url::from_file_path(&workspace).map_err(|()| "failed to create URI")?.to_string();
    let open_doc = "file:///tmp/editor-buffer/lib/Dup/Mod.pm".to_string();

    let result = resolve_module_uri(
        "Dup::Mod",
        std::slice::from_ref(&open_doc),
        &[ws_uri],
        &["lib".to_string()],
        false,
        &[],
        Duration::from_millis(200),
    );

    assert_eq!(result, ModuleUriResolution::Resolved(open_doc));
    Ok(())
}
