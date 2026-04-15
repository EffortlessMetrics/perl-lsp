//! Comprehensive unit tests for perl-module-resolution.
//!
//! Covers the full public API surface:
//! - `resolve_module_path`: filesystem path resolution with include path search
//! - `resolve_module_uri`: URI-based resolution with open docs, workspaces, @INC
//! - `ModuleUriResolution`: enum variant construction and trait behaviour

use perl_module::resolution::{ModuleUriResolution, resolve_module_path, resolve_module_uri};
use std::path::PathBuf;
use std::time::Duration;

// ============================================================================
// resolve_module_path
// ============================================================================

mod resolve_path {
    use super::*;

    #[test]
    fn resolves_simple_two_segment_module() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let target = root.join("lib").join("Foo").join("Bar.pm");

        std::fs::create_dir_all(target.parent().ok_or("no parent")?)?;
        std::fs::write(&target, "package Foo::Bar; 1;")?;

        let result = resolve_module_path(root, "Foo::Bar", &["lib".to_string()]);
        assert_eq!(result, Some(target));
        Ok(())
    }

    #[test]
    fn resolves_single_segment_module() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let target = root.join("lib").join("POSIX.pm");

        std::fs::create_dir_all(root.join("lib"))?;
        std::fs::write(&target, "package POSIX; 1;")?;

        let result = resolve_module_path(root, "POSIX", &["lib".to_string()]);
        assert_eq!(result, Some(target));
        Ok(())
    }

    #[test]
    fn falls_back_to_lib_when_include_paths_empty() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();

        let result = resolve_module_path(root, "Some::Module", &[]);
        // With no include paths, falls back to root/lib/<module>.pm
        assert_eq!(result, Some(root.join("lib").join("Some/Module.pm")));
        Ok(())
    }

    #[test]
    fn falls_back_to_lib_when_module_not_on_disk() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();

        std::fs::create_dir_all(root.join("lib"))?;

        let result = resolve_module_path(root, "Missing::Mod", &["lib".to_string()]);
        // Module doesn't exist on disk, so lib fallback is returned
        assert_eq!(result, Some(root.join("lib").join("Missing/Mod.pm")));
        Ok(())
    }

    #[test]
    fn skips_traversal_include_path_and_falls_back() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();

        let result = resolve_module_path(root, "Secret::Data", &["../outside".to_string()]);
        assert_eq!(result, Some(root.join("lib").join("Secret/Data.pm")));
        Ok(())
    }

    #[test]
    fn dot_include_path_searches_root_directly() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let target = root.join("App").join("Runner.pm");

        std::fs::create_dir_all(target.parent().ok_or("no parent")?)?;
        std::fs::write(&target, "package App::Runner; 1;")?;

        let result = resolve_module_path(root, "App::Runner", &[".".to_string()]);
        assert_eq!(result, Some(root.join("App/Runner.pm")));
        Ok(())
    }

    #[test]
    fn searches_include_paths_in_order() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();

        let first = root.join("vendor").join("Dup").join("Mod.pm");
        let second = root.join("lib").join("Dup").join("Mod.pm");

        std::fs::create_dir_all(first.parent().ok_or("no parent")?)?;
        std::fs::create_dir_all(second.parent().ok_or("no parent")?)?;
        std::fs::write(&first, "package Dup::Mod; 'vendor';")?;
        std::fs::write(&second, "package Dup::Mod; 'lib';")?;

        let result =
            resolve_module_path(root, "Dup::Mod", &["vendor".to_string(), "lib".to_string()]);
        assert_eq!(result, Some(first));
        Ok(())
    }

    #[test]
    fn resolves_four_segment_module() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let target = root.join("lib").join("A").join("B").join("C").join("D.pm");

        std::fs::create_dir_all(target.parent().ok_or("no parent")?)?;
        std::fs::write(&target, "package A::B::C::D; 1;")?;

        let result = resolve_module_path(root, "A::B::C::D", &["lib".to_string()]);
        assert_eq!(result, Some(target));
        Ok(())
    }

    #[test]
    fn module_with_underscores_and_digits() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let target = root.join("lib").join("My_App").join("V2_Worker.pm");

        std::fs::create_dir_all(target.parent().ok_or("no parent")?)?;
        std::fs::write(&target, "package My_App::V2_Worker; 1;")?;

        let result = resolve_module_path(root, "My_App::V2_Worker", &["lib".to_string()]);
        assert_eq!(result, Some(target));
        Ok(())
    }

    #[test]
    fn empty_module_name_produces_some_path() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();

        let result = resolve_module_path(root, "", &["lib".to_string()]);
        // Should not panic; returns a path (the fallback)
        assert!(result.is_some());
        Ok(())
    }

    #[test]
    fn multiple_traversal_paths_all_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();

        let result = resolve_module_path(
            root,
            "Foo::Bar",
            &["..".to_string(), "../../etc".to_string(), "../sibling".to_string()],
        );
        // All traversal paths rejected, falls back to lib
        assert_eq!(result, Some(root.join("lib").join("Foo/Bar.pm")));
        Ok(())
    }
}

// ============================================================================
// resolve_module_uri — open document precedence
// ============================================================================

mod uri_open_documents {
    use super::*;

    #[test]
    fn open_doc_matching_suffix_wins() -> Result<(), Box<dyn std::error::Error>> {
        let open_doc = "file:///editor/buffer/lib/Net/HTTP.pm".to_string();

        let result = resolve_module_uri(
            "Net::HTTP",
            std::slice::from_ref(&open_doc),
            &[],
            &["lib".to_string()],
            false,
            &[],
            Duration::from_millis(50),
        );

        assert_eq!(result, ModuleUriResolution::Resolved(open_doc));
        Ok(())
    }

    #[test]
    fn open_doc_non_matching_returns_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let open_doc = "file:///editor/Other/Module.pm".to_string();

        let result = resolve_module_uri(
            "Net::HTTP",
            std::slice::from_ref(&open_doc),
            &[],
            &["lib".to_string()],
            false,
            &[],
            Duration::from_millis(50),
        );

        assert_eq!(result, ModuleUriResolution::NotFound);
        Ok(())
    }

    #[test]
    fn first_matching_open_doc_wins() -> Result<(), Box<dyn std::error::Error>> {
        let doc1 = "file:///first/lib/Same/Mod.pm".to_string();
        let doc2 = "file:///second/lib/Same/Mod.pm".to_string();

        let result = resolve_module_uri(
            "Same::Mod",
            &[doc1.clone(), doc2],
            &[],
            &["lib".to_string()],
            false,
            &[],
            Duration::from_millis(50),
        );

        assert_eq!(result, ModuleUriResolution::Resolved(doc1));
        Ok(())
    }

    #[test]
    fn empty_open_docs_falls_through() -> Result<(), Box<dyn std::error::Error>> {
        let result = resolve_module_uri(
            "Foo::Bar",
            &[],
            &[],
            &["lib".to_string()],
            false,
            &[],
            Duration::from_millis(50),
        );

        assert_eq!(result, ModuleUriResolution::NotFound);
        Ok(())
    }

    #[test]
    fn open_doc_single_segment_module() -> Result<(), Box<dyn std::error::Error>> {
        let open_doc = "file:///workspace/lib/Carp.pm".to_string();

        let result = resolve_module_uri(
            "Carp",
            std::slice::from_ref(&open_doc),
            &[],
            &["lib".to_string()],
            false,
            &[],
            Duration::from_millis(50),
        );

        assert_eq!(result, ModuleUriResolution::Resolved(open_doc));
        Ok(())
    }
}

// ============================================================================
// resolve_module_uri — workspace folder resolution
// ============================================================================

mod uri_workspace {
    use super::*;

    #[test]
    fn resolves_from_workspace_lib() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let ws = temp.path().join("ws");
        let target = ws.join("lib").join("App").join("Config.pm");

        std::fs::create_dir_all(target.parent().ok_or("no parent")?)?;
        std::fs::write(&target, "package App::Config; 1;")?;

        let ws_uri = url::Url::from_file_path(&ws).map_err(|()| "bad URI")?.to_string();

        let result = resolve_module_uri(
            "App::Config",
            &[],
            &[ws_uri],
            &["lib".to_string()],
            false,
            &[],
            Duration::from_millis(200),
        );

        match result {
            ModuleUriResolution::Resolved(uri) => {
                assert!(uri.ends_with("App/Config.pm"), "unexpected URI: {uri}");
            }
            other => return Err(format!("expected Resolved, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn not_found_when_module_missing_from_workspace() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let ws = temp.path().join("ws");
        std::fs::create_dir_all(ws.join("lib"))?;

        let ws_uri = url::Url::from_file_path(&ws).map_err(|()| "bad URI")?.to_string();

        let result = resolve_module_uri(
            "Ghost::Module",
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
    fn second_workspace_searched_when_first_misses() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let ws1 = temp.path().join("ws1");
        let ws2 = temp.path().join("ws2");

        std::fs::create_dir_all(ws1.join("lib"))?;
        let target = ws2.join("lib").join("Only").join("Here.pm");
        std::fs::create_dir_all(target.parent().ok_or("no parent")?)?;
        std::fs::write(&target, "package Only::Here; 1;")?;

        let ws1_uri = url::Url::from_file_path(&ws1).map_err(|()| "bad URI")?.to_string();
        let ws2_uri = url::Url::from_file_path(&ws2).map_err(|()| "bad URI")?.to_string();

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
                assert!(uri.contains("ws2"), "should resolve from ws2, got: {uri}");
            }
            other => return Err(format!("expected Resolved, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn first_workspace_takes_priority_over_second() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let ws1 = temp.path().join("ws1");
        let ws2 = temp.path().join("ws2");

        let t1 = ws1.join("lib").join("Dup").join("Mod.pm");
        let t2 = ws2.join("lib").join("Dup").join("Mod.pm");
        std::fs::create_dir_all(t1.parent().ok_or("no parent")?)?;
        std::fs::create_dir_all(t2.parent().ok_or("no parent")?)?;
        std::fs::write(&t1, "package Dup::Mod; 'ws1';")?;
        std::fs::write(&t2, "package Dup::Mod; 'ws2';")?;

        let ws1_uri = url::Url::from_file_path(&ws1).map_err(|()| "bad URI")?.to_string();
        let ws2_uri = url::Url::from_file_path(&ws2).map_err(|()| "bad URI")?.to_string();

        let result = resolve_module_uri(
            "Dup::Mod",
            &[],
            &[ws1_uri, ws2_uri],
            &["lib".to_string()],
            false,
            &[],
            Duration::from_millis(200),
        );

        match result {
            ModuleUriResolution::Resolved(uri) => {
                assert!(uri.contains("ws1"), "first workspace should win, got: {uri}");
            }
            other => return Err(format!("expected Resolved, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn traversal_include_path_blocked_in_uri_resolution() -> Result<(), Box<dyn std::error::Error>>
    {
        let temp = tempfile::tempdir()?;
        let ws = temp.path().join("ws");
        let outside = temp.path().join("outside");

        std::fs::create_dir_all(&ws)?;
        let target = outside.join("Secret").join("Data.pm");
        std::fs::create_dir_all(target.parent().ok_or("no parent")?)?;
        std::fs::write(&target, "package Secret::Data; 1;")?;

        let ws_uri = url::Url::from_file_path(&ws).map_err(|()| "bad URI")?.to_string();

        let result = resolve_module_uri(
            "Secret::Data",
            &[],
            &[ws_uri],
            &["../outside".to_string()],
            false,
            &[],
            Duration::from_millis(200),
        );

        assert_eq!(result, ModuleUriResolution::NotFound);
        Ok(())
    }

    #[test]
    fn dot_include_path_resolves_from_workspace_root() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let ws = temp.path().join("ws");
        let target = ws.join("Local").join("Mod.pm");

        std::fs::create_dir_all(target.parent().ok_or("no parent")?)?;
        std::fs::write(&target, "package Local::Mod; 1;")?;

        let ws_uri = url::Url::from_file_path(&ws).map_err(|()| "bad URI")?.to_string();

        let result = resolve_module_uri(
            "Local::Mod",
            &[],
            &[ws_uri],
            &[".".to_string()],
            false,
            &[],
            Duration::from_millis(200),
        );

        match result {
            ModuleUriResolution::Resolved(uri) => {
                assert!(uri.ends_with("Local/Mod.pm"), "unexpected URI: {uri}");
            }
            other => return Err(format!("expected Resolved, got {other:?}").into()),
        }
        Ok(())
    }
}

// ============================================================================
// resolve_module_uri — system @INC
// ============================================================================

mod uri_system_inc {
    use super::*;

    #[test]
    fn enabled_system_inc_resolves_module() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let inc = temp.path().join("inc");
        let target = inc.join("Sys").join("Lib.pm");

        std::fs::create_dir_all(target.parent().ok_or("no parent")?)?;
        std::fs::write(&target, "package Sys::Lib; 1;")?;

        let result = resolve_module_uri(
            "Sys::Lib",
            &[],
            &[],
            &[],
            true,
            &[PathBuf::from(&inc)],
            Duration::from_millis(200),
        );

        match result {
            ModuleUriResolution::Resolved(uri) => {
                assert!(uri.contains("Sys/Lib.pm"), "unexpected URI: {uri}");
            }
            other => return Err(format!("expected Resolved, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn disabled_system_inc_skips_system_paths() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let inc = temp.path().join("inc");
        let target = inc.join("Hidden").join("Mod.pm");

        std::fs::create_dir_all(target.parent().ok_or("no parent")?)?;
        std::fs::write(&target, "package Hidden::Mod; 1;")?;

        let result = resolve_module_uri(
            "Hidden::Mod",
            &[],
            &[],
            &[],
            false,
            &[PathBuf::from(&inc)],
            Duration::from_millis(200),
        );

        assert_eq!(result, ModuleUriResolution::NotFound);
        Ok(())
    }

    #[test]
    fn system_inc_searched_after_workspaces() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let ws = temp.path().join("ws");
        let inc = temp.path().join("inc");

        // Module exists in both workspace and system @INC
        let ws_target = ws.join("lib").join("Dup").join("Mod.pm");
        let inc_target = inc.join("Dup").join("Mod.pm");

        std::fs::create_dir_all(ws_target.parent().ok_or("no parent")?)?;
        std::fs::create_dir_all(inc_target.parent().ok_or("no parent")?)?;
        std::fs::write(&ws_target, "package Dup::Mod; 'ws';")?;
        std::fs::write(&inc_target, "package Dup::Mod; 'inc';")?;

        let ws_uri = url::Url::from_file_path(&ws).map_err(|()| "bad URI")?.to_string();

        let result = resolve_module_uri(
            "Dup::Mod",
            &[],
            &[ws_uri],
            &["lib".to_string()],
            true,
            &[PathBuf::from(&inc)],
            Duration::from_millis(200),
        );

        match result {
            ModuleUriResolution::Resolved(uri) => {
                assert!(uri.contains("ws"), "workspace should win over @INC, got: {uri}");
            }
            other => return Err(format!("expected Resolved, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn multiple_system_inc_paths_first_wins() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let inc1 = temp.path().join("inc1");
        let inc2 = temp.path().join("inc2");

        let t1 = inc1.join("Multi").join("Inc.pm");
        let t2 = inc2.join("Multi").join("Inc.pm");

        std::fs::create_dir_all(t1.parent().ok_or("no parent")?)?;
        std::fs::create_dir_all(t2.parent().ok_or("no parent")?)?;
        std::fs::write(&t1, "package Multi::Inc; 'first';")?;
        std::fs::write(&t2, "package Multi::Inc; 'second';")?;

        let result = resolve_module_uri(
            "Multi::Inc",
            &[],
            &[],
            &[],
            true,
            &[PathBuf::from(&inc1), PathBuf::from(&inc2)],
            Duration::from_millis(200),
        );

        match result {
            ModuleUriResolution::Resolved(uri) => {
                assert!(uri.contains("inc1"), "first @INC should win, got: {uri}");
            }
            other => return Err(format!("expected Resolved, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn system_inc_with_nonexistent_dir_is_safe() -> Result<(), Box<dyn std::error::Error>> {
        let result = resolve_module_uri(
            "Ghost::Mod",
            &[],
            &[],
            &[],
            true,
            &[PathBuf::from("/nonexistent/perl/lib")],
            Duration::from_millis(50),
        );

        assert_eq!(result, ModuleUriResolution::NotFound);
        Ok(())
    }
}

// ============================================================================
// resolve_module_uri — timeout behaviour
// ============================================================================

mod uri_timeout {
    use super::*;

    #[test]
    fn zero_timeout_returns_not_found_or_timed_out_without_panic()
    -> Result<(), Box<dyn std::error::Error>> {
        let result = resolve_module_uri(
            "Any::Module",
            &[],
            &["file:///ws".to_string()],
            &["lib".to_string()],
            false,
            &[],
            Duration::ZERO,
        );

        // With zero timeout the function may return TimedOut or NotFound,
        // but must not panic
        assert!(
            result == ModuleUriResolution::TimedOut || result == ModuleUriResolution::NotFound,
            "unexpected result: {result:?}"
        );
        Ok(())
    }

    #[test]
    fn reasonable_timeout_resolves_normally() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let ws = temp.path().join("ws");
        let target = ws.join("lib").join("Fast").join("Mod.pm");

        std::fs::create_dir_all(target.parent().ok_or("no parent")?)?;
        std::fs::write(&target, "package Fast::Mod; 1;")?;

        let ws_uri = url::Url::from_file_path(&ws).map_err(|()| "bad URI")?.to_string();

        let result = resolve_module_uri(
            "Fast::Mod",
            &[],
            &[ws_uri],
            &["lib".to_string()],
            false,
            &[],
            Duration::from_secs(5),
        );

        match result {
            ModuleUriResolution::Resolved(uri) => {
                assert!(uri.ends_with("Fast/Mod.pm"));
            }
            other => return Err(format!("expected Resolved, got {other:?}").into()),
        }
        Ok(())
    }
}

// ============================================================================
// resolve_module_uri — edge cases and empty inputs
// ============================================================================

mod uri_edge_cases {
    use super::*;

    #[test]
    fn all_empty_inputs_returns_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let result = resolve_module_uri("", &[], &[], &[], false, &[], Duration::from_millis(50));

        assert_eq!(result, ModuleUriResolution::NotFound);
        Ok(())
    }

    #[test]
    fn empty_module_name_with_workspace_returns_not_found() -> Result<(), Box<dyn std::error::Error>>
    {
        let temp = tempfile::tempdir()?;
        let ws = temp.path().join("ws");
        std::fs::create_dir_all(ws.join("lib"))?;

        let ws_uri = url::Url::from_file_path(&ws).map_err(|()| "bad URI")?.to_string();

        let result = resolve_module_uri(
            "",
            &[],
            &[ws_uri],
            &["lib".to_string()],
            false,
            &[],
            Duration::from_millis(200),
        );

        // Empty module name won't match any file on disk
        assert_eq!(result, ModuleUriResolution::NotFound);
        Ok(())
    }

    #[test]
    fn single_segment_module_via_uri() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let ws = temp.path().join("ws");
        let target = ws.join("lib").join("Scalar.pm");

        std::fs::create_dir_all(ws.join("lib"))?;
        std::fs::write(&target, "package Scalar; 1;")?;

        let ws_uri = url::Url::from_file_path(&ws).map_err(|()| "bad URI")?.to_string();

        let result = resolve_module_uri(
            "Scalar",
            &[],
            &[ws_uri],
            &["lib".to_string()],
            false,
            &[],
            Duration::from_millis(200),
        );

        match result {
            ModuleUriResolution::Resolved(uri) => {
                assert!(uri.ends_with("Scalar.pm"), "unexpected URI: {uri}");
            }
            other => return Err(format!("expected Resolved, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn nonexistent_workspace_folder_returns_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let result = resolve_module_uri(
            "Foo::Bar",
            &[],
            &["file:///nonexistent/workspace".to_string()],
            &["lib".to_string()],
            false,
            &[],
            Duration::from_millis(50),
        );

        assert_eq!(result, ModuleUriResolution::NotFound);
        Ok(())
    }

    #[test]
    fn open_doc_wins_over_workspace_and_system_inc() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let ws = temp.path().join("ws");
        let inc = temp.path().join("inc");

        let ws_target = ws.join("lib").join("All").join("Three.pm");
        let inc_target = inc.join("All").join("Three.pm");

        std::fs::create_dir_all(ws_target.parent().ok_or("no parent")?)?;
        std::fs::create_dir_all(inc_target.parent().ok_or("no parent")?)?;
        std::fs::write(&ws_target, "package All::Three; 'ws';")?;
        std::fs::write(&inc_target, "package All::Three; 'inc';")?;

        let ws_uri = url::Url::from_file_path(&ws).map_err(|()| "bad URI")?.to_string();
        let open_doc = "file:///editor/lib/All/Three.pm".to_string();

        let result = resolve_module_uri(
            "All::Three",
            std::slice::from_ref(&open_doc),
            &[ws_uri],
            &["lib".to_string()],
            true,
            &[PathBuf::from(&inc)],
            Duration::from_millis(200),
        );

        assert_eq!(result, ModuleUriResolution::Resolved(open_doc));
        Ok(())
    }

    #[test]
    fn multiple_include_paths_with_custom_names() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let ws = temp.path().join("ws");

        let target = ws.join("local_lib").join("Custom").join("Path.pm");
        std::fs::create_dir_all(target.parent().ok_or("no parent")?)?;
        std::fs::write(&target, "package Custom::Path; 1;")?;

        let ws_uri = url::Url::from_file_path(&ws).map_err(|()| "bad URI")?.to_string();

        let result = resolve_module_uri(
            "Custom::Path",
            &[],
            &[ws_uri],
            &["lib".to_string(), "local_lib".to_string(), "vendor".to_string()],
            false,
            &[],
            Duration::from_millis(200),
        );

        match result {
            ModuleUriResolution::Resolved(uri) => {
                assert!(uri.contains("local_lib"), "should find in local_lib, got: {uri}");
            }
            other => return Err(format!("expected Resolved, got {other:?}").into()),
        }
        Ok(())
    }
}

// ============================================================================
// ModuleUriResolution enum traits
// ============================================================================

mod resolution_enum {
    use super::*;

    #[test]
    fn resolved_variant_stores_uri() -> Result<(), Box<dyn std::error::Error>> {
        let res = ModuleUriResolution::Resolved("file:///test.pm".to_string());
        match res {
            ModuleUriResolution::Resolved(uri) => assert_eq!(uri, "file:///test.pm"),
            other => return Err(format!("unexpected: {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn not_found_variant() -> Result<(), Box<dyn std::error::Error>> {
        let res = ModuleUriResolution::NotFound;
        assert_eq!(res, ModuleUriResolution::NotFound);
        Ok(())
    }

    #[test]
    fn timed_out_variant() -> Result<(), Box<dyn std::error::Error>> {
        let res = ModuleUriResolution::TimedOut;
        assert_eq!(res, ModuleUriResolution::TimedOut);
        Ok(())
    }

    #[test]
    fn clone_produces_equal_value() -> Result<(), Box<dyn std::error::Error>> {
        let original = ModuleUriResolution::Resolved("file:///foo.pm".to_string());
        let cloned = original.clone();
        assert_eq!(original, cloned);
        Ok(())
    }

    #[test]
    fn debug_format_is_readable() -> Result<(), Box<dyn std::error::Error>> {
        let res = ModuleUriResolution::Resolved("file:///x.pm".to_string());
        let debug = format!("{res:?}");
        assert!(debug.contains("Resolved"), "debug should contain variant name");
        assert!(debug.contains("x.pm"), "debug should contain URI");
        Ok(())
    }

    #[test]
    fn variants_are_not_equal_across_types() -> Result<(), Box<dyn std::error::Error>> {
        assert_ne!(ModuleUriResolution::NotFound, ModuleUriResolution::TimedOut);
        assert_ne!(
            ModuleUriResolution::Resolved("file:///a.pm".to_string()),
            ModuleUriResolution::NotFound
        );
        assert_ne!(
            ModuleUriResolution::Resolved("file:///a.pm".to_string()),
            ModuleUriResolution::TimedOut
        );
        Ok(())
    }

    #[test]
    fn different_uris_are_not_equal() -> Result<(), Box<dyn std::error::Error>> {
        let a = ModuleUriResolution::Resolved("file:///a.pm".to_string());
        let b = ModuleUriResolution::Resolved("file:///b.pm".to_string());
        assert_ne!(a, b);
        Ok(())
    }
}
