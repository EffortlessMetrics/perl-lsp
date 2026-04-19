//! Integration tests for go-to-definition module resolution scenarios.
//!
//! These tests cover the end-to-end resolution pipeline used when the LSP
//! processes `use` statements for go-to-definition:
//!
//! 1. System-installed modules via @INC (`use File::Basename`)
//! 2. Workspace modules with custom include paths (`use lib 'lib'; use MyApp::Model`)
//! 3. Parent/base module resolution (`use parent 'Base::Class'`)
//! 4. Graceful not-found handling (no crash)

use perl_module::resolution::{ModuleUriResolution, resolve_module_path, resolve_module_uri};
use std::path::PathBuf;
use std::time::Duration;

// ============================================================================
// Scenario 1: System @INC resolution (e.g., `use File::Basename`)
// ============================================================================

mod system_inc_resolution {
    use super::*;

    #[test]
    fn resolves_module_from_simulated_system_inc() -> Result<(), Box<dyn std::error::Error>> {
        // Simulate a system @INC directory containing File/Basename.pm
        let temp = tempfile::tempdir()?;
        let inc_dir = temp.path().join("perl5lib");
        let module_file = inc_dir.join("File").join("Basename.pm");

        std::fs::create_dir_all(module_file.parent().ok_or("no parent")?)?;
        std::fs::write(&module_file, "package File::Basename;\nuse strict;\n1;")?;

        let result = resolve_module_uri(
            "File::Basename",
            &[],
            &[],
            &[],
            true,
            &[inc_dir],
            Duration::from_millis(200),
        );

        match result {
            ModuleUriResolution::Resolved(uri) => {
                assert!(
                    uri.ends_with("File/Basename.pm"),
                    "expected File/Basename.pm in URI, got: {uri}"
                );
            }
            other => return Err(format!("expected Resolved, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn system_inc_not_searched_when_disabled() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let inc_dir = temp.path().join("perl5lib");
        let module_file = inc_dir.join("File").join("Basename.pm");

        std::fs::create_dir_all(module_file.parent().ok_or("no parent")?)?;
        std::fs::write(&module_file, "package File::Basename; 1;")?;

        let result = resolve_module_uri(
            "File::Basename",
            &[],
            &[],
            &[],
            false, // system @INC disabled
            &[inc_dir],
            Duration::from_millis(200),
        );

        assert_eq!(result, ModuleUriResolution::NotFound);
        Ok(())
    }

    #[test]
    fn multiple_inc_dirs_searched_in_order() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let site_lib = temp.path().join("site_perl");
        let core_lib = temp.path().join("core_perl");

        // Module exists in both dirs -- site_perl should win
        let site_file = site_lib.join("File").join("Basename.pm");
        let core_file = core_lib.join("File").join("Basename.pm");

        std::fs::create_dir_all(site_file.parent().ok_or("no parent")?)?;
        std::fs::create_dir_all(core_file.parent().ok_or("no parent")?)?;
        std::fs::write(&site_file, "# site version")?;
        std::fs::write(&core_file, "# core version")?;

        let result = resolve_module_uri(
            "File::Basename",
            &[],
            &[],
            &[],
            true,
            &[site_lib.clone(), core_lib],
            Duration::from_millis(200),
        );

        match result {
            ModuleUriResolution::Resolved(uri) => {
                assert!(
                    uri.contains("site_perl"),
                    "first @INC entry should win, got: {uri}"
                );
            }
            other => return Err(format!("expected Resolved, got {other:?}").into()),
        }
        Ok(())
    }
}

// ============================================================================
// Scenario 2: Workspace modules with custom include paths
// (simulating `use lib 'lib'; use MyApp::Model`)
// ============================================================================

mod workspace_include_path_resolution {
    use super::*;

    #[test]
    fn resolves_module_under_lib_include_path() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("project");
        let module_file = workspace.join("lib").join("MyApp").join("Model.pm");

        std::fs::create_dir_all(module_file.parent().ok_or("no parent")?)?;
        std::fs::write(&module_file, "package MyApp::Model;\nuse strict;\n1;")?;

        // This simulates the effect of `use lib 'lib'` -- the 'lib' directory
        // is configured as an include path
        let ws_uri = url::Url::from_file_path(&workspace)
            .map_err(|()| "bad URI")?
            .to_string();

        let result = resolve_module_uri(
            "MyApp::Model",
            &[],
            &[ws_uri],
            &["lib".to_string()],
            false,
            &[],
            Duration::from_millis(200),
        );

        match result {
            ModuleUriResolution::Resolved(uri) => {
                assert!(
                    uri.ends_with("MyApp/Model.pm"),
                    "expected MyApp/Model.pm, got: {uri}"
                );
            }
            other => return Err(format!("expected Resolved, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn resolves_module_from_custom_include_path() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("project");
        // Module lives in a custom path, not the standard 'lib'
        let module_file = workspace
            .join("local_modules")
            .join("MyApp")
            .join("Model.pm");

        std::fs::create_dir_all(module_file.parent().ok_or("no parent")?)?;
        std::fs::write(&module_file, "package MyApp::Model; 1;")?;

        let ws_uri = url::Url::from_file_path(&workspace)
            .map_err(|()| "bad URI")?
            .to_string();

        let result = resolve_module_uri(
            "MyApp::Model",
            &[],
            &[ws_uri],
            &["local_modules".to_string()],
            false,
            &[],
            Duration::from_millis(200),
        );

        match result {
            ModuleUriResolution::Resolved(uri) => {
                assert!(
                    uri.contains("local_modules") && uri.ends_with("MyApp/Model.pm"),
                    "expected local_modules/MyApp/Model.pm, got: {uri}"
                );
            }
            other => return Err(format!("expected Resolved, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn multiple_include_paths_searched_in_priority_order() -> Result<(), Box<dyn std::error::Error>>
    {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("project");

        // Module in both 'vendor' and 'lib' -- 'vendor' is listed first
        let vendor_file = workspace.join("vendor").join("MyApp").join("Model.pm");
        let lib_file = workspace.join("lib").join("MyApp").join("Model.pm");

        std::fs::create_dir_all(vendor_file.parent().ok_or("no parent")?)?;
        std::fs::create_dir_all(lib_file.parent().ok_or("no parent")?)?;
        std::fs::write(&vendor_file, "# vendor version")?;
        std::fs::write(&lib_file, "# lib version")?;

        let ws_uri = url::Url::from_file_path(&workspace)
            .map_err(|()| "bad URI")?
            .to_string();

        let result = resolve_module_uri(
            "MyApp::Model",
            &[],
            &[ws_uri],
            &["vendor".to_string(), "lib".to_string()],
            false,
            &[],
            Duration::from_millis(200),
        );

        match result {
            ModuleUriResolution::Resolved(uri) => {
                assert!(
                    uri.contains("vendor"),
                    "first include path should win, got: {uri}"
                );
            }
            other => return Err(format!("expected Resolved, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn resolve_module_path_with_lib_include() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("project");
        let module_file = workspace.join("lib").join("MyApp").join("Model.pm");

        std::fs::create_dir_all(module_file.parent().ok_or("no parent")?)?;
        std::fs::write(&module_file, "package MyApp::Model; 1;")?;

        let result = resolve_module_path(&workspace, "MyApp::Model", &["lib".to_string()]);

        assert_eq!(result, Some(module_file));
        Ok(())
    }
}

// ============================================================================
// Scenario 3: use parent 'Base::Class' resolution
// (the module reference extraction happens in perl-module-reference,
//  but the resolution pipeline is tested here)
// ============================================================================

mod parent_base_module_resolution {
    use super::*;

    #[test]
    fn resolves_parent_module_from_workspace() -> Result<(), Box<dyn std::error::Error>> {
        // When the LSP extracts "Base::Class" from `use parent 'Base::Class'`,
        // it passes that module name to the resolution pipeline.
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("project");
        let module_file = workspace.join("lib").join("Base").join("Class.pm");

        std::fs::create_dir_all(module_file.parent().ok_or("no parent")?)?;
        std::fs::write(
            &module_file,
            "package Base::Class;\nsub new { bless {}, shift }\n1;",
        )?;

        let ws_uri = url::Url::from_file_path(&workspace)
            .map_err(|()| "bad URI")?
            .to_string();

        let result = resolve_module_uri(
            "Base::Class",
            &[],
            &[ws_uri],
            &["lib".to_string()],
            false,
            &[],
            Duration::from_millis(200),
        );

        match result {
            ModuleUriResolution::Resolved(uri) => {
                assert!(
                    uri.ends_with("Base/Class.pm"),
                    "expected Base/Class.pm, got: {uri}"
                );
            }
            other => return Err(format!("expected Resolved, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn resolves_parent_module_from_system_inc() -> Result<(), Box<dyn std::error::Error>> {
        // Parent module in system @INC (e.g., `use parent 'Exporter'`)
        let temp = tempfile::tempdir()?;
        let inc_dir = temp.path().join("perl5lib");
        let module_file = inc_dir.join("Exporter.pm");

        std::fs::create_dir_all(&inc_dir)?;
        std::fs::write(&module_file, "package Exporter; 1;")?;

        let result = resolve_module_uri(
            "Exporter",
            &[],
            &[],
            &[],
            true,
            &[inc_dir],
            Duration::from_millis(200),
        );

        match result {
            ModuleUriResolution::Resolved(uri) => {
                assert!(
                    uri.ends_with("Exporter.pm"),
                    "expected Exporter.pm, got: {uri}"
                );
            }
            other => return Err(format!("expected Resolved, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn resolve_module_path_for_parent_module() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("project");
        let module_file = workspace.join("lib").join("Base").join("Class.pm");

        std::fs::create_dir_all(module_file.parent().ok_or("no parent")?)?;
        std::fs::write(&module_file, "package Base::Class; 1;")?;

        let result = resolve_module_path(&workspace, "Base::Class", &["lib".to_string()]);
        assert_eq!(result, Some(module_file));
        Ok(())
    }
}

// ============================================================================
// Scenario 4: Graceful error handling (not found, no crash)
// ============================================================================

mod graceful_error_handling {
    use super::*;

    #[test]
    fn module_not_found_returns_not_found_not_panic() {
        let result = resolve_module_uri(
            "Nonexistent::Module::That::Does::Not::Exist",
            &[],
            &[],
            &[],
            false,
            &[],
            Duration::from_millis(100),
        );

        assert_eq!(result, ModuleUriResolution::NotFound);
    }

    #[test]
    fn module_not_found_with_workspace_returns_not_found() -> Result<(), Box<dyn std::error::Error>>
    {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("project");
        std::fs::create_dir_all(workspace.join("lib"))?;

        let ws_uri = url::Url::from_file_path(&workspace)
            .map_err(|()| "bad URI")?
            .to_string();

        let result = resolve_module_uri(
            "Missing::Module",
            &[],
            &[ws_uri],
            &["lib".to_string()],
            false,
            &[],
            Duration::from_millis(200),
        );

        assert_eq!(result, ModuleUriResolution::NotFound);
        Ok(())
    }

    #[test]
    fn module_not_found_with_system_inc_returns_not_found() -> Result<(), Box<dyn std::error::Error>>
    {
        let temp = tempfile::tempdir()?;
        let inc_dir = temp.path().join("perl5lib");
        std::fs::create_dir_all(&inc_dir)?;

        let result = resolve_module_uri(
            "Missing::Module",
            &[],
            &[],
            &[],
            true,
            &[inc_dir],
            Duration::from_millis(200),
        );

        assert_eq!(result, ModuleUriResolution::NotFound);
        Ok(())
    }

    #[test]
    fn empty_module_name_does_not_crash() {
        let result = resolve_module_uri(
            "",
            &[],
            &[],
            &[],
            true,
            &[PathBuf::from("/nonexistent")],
            Duration::from_millis(100),
        );

        // Should return gracefully, not crash
        assert!(
            result == ModuleUriResolution::NotFound || result == ModuleUriResolution::TimedOut,
            "unexpected result: {result:?}"
        );
    }

    #[test]
    fn nonexistent_workspace_folder_does_not_crash() {
        let result = resolve_module_uri(
            "Foo::Bar",
            &[],
            &["file:///does/not/exist/at/all".to_string()],
            &["lib".to_string()],
            false,
            &[],
            Duration::from_millis(100),
        );

        assert_eq!(result, ModuleUriResolution::NotFound);
    }

    #[test]
    fn nonexistent_system_inc_path_does_not_crash() {
        let result = resolve_module_uri(
            "Foo::Bar",
            &[],
            &[],
            &[],
            true,
            &[PathBuf::from("/absolutely/nonexistent/perl/lib")],
            Duration::from_millis(100),
        );

        assert_eq!(result, ModuleUriResolution::NotFound);
    }

    #[test]
    fn timeout_returns_timed_out_not_panic() {
        // Create a scenario that will definitely time out
        let workspace_folders: Vec<String> = (0..10_000)
            .map(|i| format!("file:///workspace-{i}"))
            .collect();

        let result = resolve_module_uri(
            "Never::Found",
            &[],
            &workspace_folders,
            &["lib".to_string()],
            false,
            &[],
            Duration::from_nanos(1),
        );

        assert_eq!(result, ModuleUriResolution::TimedOut);
    }

    #[test]
    fn resolve_module_path_with_nonexistent_root_does_not_crash() {
        let root = PathBuf::from("/nonexistent/workspace/root");
        let result = resolve_module_path(&root, "Some::Module", &["lib".to_string()]);

        // Should return Some (the fallback path), not panic
        assert!(result.is_some());
    }

    #[test]
    fn unicode_module_name_does_not_crash() {
        let result = resolve_module_uri(
            "\u{1F600}::Module",
            &[],
            &[],
            &[],
            false,
            &[],
            Duration::from_millis(100),
        );

        // Should handle gracefully
        assert_eq!(result, ModuleUriResolution::NotFound);
    }
}
