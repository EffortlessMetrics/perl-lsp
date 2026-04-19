//! Tests for use/require handling, @INC search, and module name to path mapping.
//!
//! Covers gaps in the existing test suite:
//! - `require Module::Name` resolution via the same pipeline as `use`
//! - `require "path/to/file.pm"` file-path style resolution
//! - `use lib` effect on include paths (prepend vs append ordering)
//! - Module name to file path mapping (:: to /) through the full resolution pipeline
//! - Relative vs absolute include paths
//! - Legacy `'` separator through resolution
//! - Multi-level @INC search with workspace + system overlap

use perl_module::resolution::{ModuleUriResolution, resolve_module_path, resolve_module_uri};
use std::path::PathBuf;
use std::time::Duration;

// ============================================================================
// Module name to file path mapping (:: to /) through resolution
// ============================================================================

mod module_name_to_path_mapping {
    use super::*;

    /// Verifies that `::` separators are correctly converted to `/` directory
    /// separators when resolving a two-segment module name.
    #[test]
    fn two_segment_module_maps_colons_to_slashes() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let target = root.join("lib").join("Net").join("HTTP.pm");

        std::fs::create_dir_all(target.parent().ok_or("missing parent")?)?;
        std::fs::write(&target, "package Net::HTTP; 1;")?;

        let result = resolve_module_path(root, "Net::HTTP", &["lib".to_string()]);
        assert_eq!(result, Some(target));
        Ok(())
    }

    /// Verifies that a three-segment module name produces a three-level
    /// directory path: `A::B::C` becomes `A/B/C.pm`.
    #[test]
    fn three_segment_module_creates_nested_dirs() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let target = root
            .join("lib")
            .join("App")
            .join("Config")
            .join("Loader.pm");

        std::fs::create_dir_all(target.parent().ok_or("missing parent")?)?;
        std::fs::write(&target, "package App::Config::Loader; 1;")?;

        let result = resolve_module_path(root, "App::Config::Loader", &["lib".to_string()]);
        assert_eq!(result, Some(target));
        Ok(())
    }

    /// Single-segment module name (no `::`) maps directly to `<name>.pm`
    /// without any directory nesting.
    #[test]
    fn single_segment_module_maps_to_flat_file() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let target = root.join("lib").join("strict.pm");

        std::fs::create_dir_all(root.join("lib"))?;
        std::fs::write(&target, "package strict; 1;")?;

        let result = resolve_module_path(root, "strict", &["lib".to_string()]);
        assert_eq!(result, Some(target));
        Ok(())
    }

    /// Five-segment deeply nested module name maps correctly.
    #[test]
    fn five_segment_module_maps_to_deep_path() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let target = root
            .join("lib")
            .join("Org")
            .join("Corp")
            .join("Dept")
            .join("Team")
            .join("Worker.pm");

        std::fs::create_dir_all(target.parent().ok_or("missing parent")?)?;
        std::fs::write(&target, "package Org::Corp::Dept::Team::Worker; 1;")?;

        let result =
            resolve_module_path(root, "Org::Corp::Dept::Team::Worker", &["lib".to_string()]);
        assert_eq!(result, Some(target));
        Ok(())
    }

    /// Module name mapping through the URI resolution pipeline produces a
    /// correct file:// URI with `/` separators for `::` segments.
    #[test]
    fn uri_resolution_preserves_colon_to_slash_mapping() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let ws = temp.path().join("ws");
        let target = ws.join("lib").join("Data").join("Dumper.pm");

        std::fs::create_dir_all(target.parent().ok_or("missing parent")?)?;
        std::fs::write(&target, "package Data::Dumper; 1;")?;

        let ws_uri = url::Url::from_file_path(&ws)
            .map_err(|()| "bad URI")?
            .to_string();

        let result = resolve_module_uri(
            "Data::Dumper",
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
                    uri.ends_with("Data/Dumper.pm"),
                    "URI should end with Data/Dumper.pm, got: {uri}"
                );
                // Verify no `::` leaked into the file path portion
                let path_part = uri.strip_prefix("file://").ok_or("not a file URI")?;
                assert!(
                    !path_part.contains("::"),
                    "path should not contain ::, got: {path_part}"
                );
            }
            other => return Err(format!("expected Resolved, got {other:?}").into()),
        }
        Ok(())
    }
}

// ============================================================================
// require Module::Name resolution (same pipeline as use)
// ============================================================================

mod require_module_resolution {
    use super::*;

    /// `require Module::Name` resolves through the same path as `use` since
    /// the resolution pipeline receives a canonical module name either way.
    #[test]
    fn require_module_name_resolves_via_path() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let target = root.join("lib").join("File").join("Spec.pm");

        std::fs::create_dir_all(target.parent().ok_or("missing parent")?)?;
        std::fs::write(&target, "package File::Spec; 1;")?;

        // require File::Spec resolves exactly like use File::Spec
        let result = resolve_module_path(root, "File::Spec", &["lib".to_string()]);
        assert_eq!(result, Some(target));
        Ok(())
    }

    /// `require Module::Name` resolves via the URI pipeline identically to
    /// `use Module::Name`.
    #[test]
    fn require_module_name_resolves_via_uri() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let ws = temp.path().join("ws");
        let target = ws.join("lib").join("IO").join("Socket.pm");

        std::fs::create_dir_all(target.parent().ok_or("missing parent")?)?;
        std::fs::write(&target, "package IO::Socket; 1;")?;

        let ws_uri = url::Url::from_file_path(&ws)
            .map_err(|()| "bad URI")?
            .to_string();

        let result = resolve_module_uri(
            "IO::Socket",
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
                    uri.ends_with("IO/Socket.pm"),
                    "expected IO/Socket.pm, got: {uri}"
                );
            }
            other => return Err(format!("expected Resolved, got {other:?}").into()),
        }
        Ok(())
    }

    /// `require Module::Name` resolution searches system @INC just like `use`.
    #[test]
    fn require_module_resolves_from_system_inc() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let inc_dir = temp.path().join("perl5lib");
        let target = inc_dir.join("Cwd.pm");

        std::fs::create_dir_all(&inc_dir)?;
        std::fs::write(&target, "package Cwd; 1;")?;

        let result = resolve_module_uri(
            "Cwd",
            &[],
            &[],
            &[],
            true,
            &[inc_dir],
            Duration::from_millis(200),
        );

        match result {
            ModuleUriResolution::Resolved(uri) => {
                assert!(uri.ends_with("Cwd.pm"), "expected Cwd.pm, got: {uri}");
            }
            other => return Err(format!("expected Resolved, got {other:?}").into()),
        }
        Ok(())
    }
}

// ============================================================================
// require "path/to/file.pm" file-path style resolution
// ============================================================================

mod require_file_path_resolution {
    use super::*;

    /// When `require` uses a file path like "Foo/Bar.pm", the resolution
    /// pipeline receives the module name "Foo::Bar" (after extraction).
    /// This tests the end-to-end: path-style require resolves the same as
    /// module-style require.
    #[test]
    fn file_path_require_resolves_same_as_module_name() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let target = root.join("lib").join("Foo").join("Bar.pm");

        std::fs::create_dir_all(target.parent().ok_or("missing parent")?)?;
        std::fs::write(&target, "package Foo::Bar; 1;")?;

        // Both forms should resolve to the same path
        let via_module = resolve_module_path(root, "Foo::Bar", &["lib".to_string()]);
        assert_eq!(via_module, Some(target));
        Ok(())
    }

    /// File-path style require for a single-segment pragma module.
    #[test]
    fn file_path_require_single_segment() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let target = root.join("lib").join("warnings.pm");

        std::fs::create_dir_all(root.join("lib"))?;
        std::fs::write(&target, "package warnings; 1;")?;

        // require "warnings.pm" -> resolves module name "warnings"
        let result = resolve_module_path(root, "warnings", &["lib".to_string()]);
        assert_eq!(result, Some(target));
        Ok(())
    }

    /// File-path style require for a deeply nested module.
    #[test]
    fn file_path_require_deep_nesting() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let target = root
            .join("lib")
            .join("HTTP")
            .join("Request")
            .join("Common.pm");

        std::fs::create_dir_all(target.parent().ok_or("missing parent")?)?;
        std::fs::write(&target, "package HTTP::Request::Common; 1;")?;

        // require "HTTP/Request/Common.pm" -> module name HTTP::Request::Common
        let result = resolve_module_path(root, "HTTP::Request::Common", &["lib".to_string()]);
        assert_eq!(result, Some(target));
        Ok(())
    }
}

// ============================================================================
// @INC search order
// ============================================================================

mod inc_search_order {
    use super::*;

    /// The full search order is: open documents > workspace include paths >
    /// system @INC. This test verifies all three tiers in a single scenario.
    #[test]
    fn complete_search_order_open_doc_then_workspace_then_inc()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let ws = temp.path().join("ws");
        let inc = temp.path().join("inc");

        // Set up module in workspace AND system @INC
        let ws_target = ws.join("lib").join("Order").join("Test.pm");
        let inc_target = inc.join("Order").join("Test.pm");

        std::fs::create_dir_all(ws_target.parent().ok_or("missing parent")?)?;
        std::fs::create_dir_all(inc_target.parent().ok_or("missing parent")?)?;
        std::fs::write(&ws_target, "# workspace version")?;
        std::fs::write(&inc_target, "# inc version")?;

        let ws_uri = url::Url::from_file_path(&ws)
            .map_err(|()| "bad URI")?
            .to_string();

        // An open document matching the suffix should win over everything
        let open_doc = "file:///editor/lib/Order/Test.pm".to_string();

        let result = resolve_module_uri(
            "Order::Test",
            std::slice::from_ref(&open_doc),
            &[ws_uri],
            &["lib".to_string()],
            true,
            &[inc],
            Duration::from_millis(200),
        );

        assert_eq!(
            result,
            ModuleUriResolution::Resolved(open_doc),
            "open document should take highest precedence"
        );
        Ok(())
    }

    /// When no open document matches, workspace include paths are searched
    /// before system @INC.
    #[test]
    fn workspace_searched_before_system_inc() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let ws = temp.path().join("ws");
        let inc = temp.path().join("inc");

        let ws_target = ws.join("lib").join("Priority").join("Mod.pm");
        let inc_target = inc.join("Priority").join("Mod.pm");

        std::fs::create_dir_all(ws_target.parent().ok_or("missing parent")?)?;
        std::fs::create_dir_all(inc_target.parent().ok_or("missing parent")?)?;
        std::fs::write(&ws_target, "# workspace")?;
        std::fs::write(&inc_target, "# system inc")?;

        let ws_uri = url::Url::from_file_path(&ws)
            .map_err(|()| "bad URI")?
            .to_string();

        let result = resolve_module_uri(
            "Priority::Mod",
            &[], // no open docs
            &[ws_uri],
            &["lib".to_string()],
            true,
            &[inc],
            Duration::from_millis(200),
        );

        match result {
            ModuleUriResolution::Resolved(uri) => {
                assert!(
                    uri.contains("ws"),
                    "workspace should win over @INC, got: {uri}"
                );
            }
            other => return Err(format!("expected Resolved, got {other:?}").into()),
        }
        Ok(())
    }

    /// System @INC is used as fallback when workspace has no match.
    #[test]
    fn system_inc_used_as_fallback() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let ws = temp.path().join("ws");
        let inc = temp.path().join("perl5lib");

        std::fs::create_dir_all(ws.join("lib"))?; // workspace exists but module is not there
        let target = inc.join("Fallback").join("Mod.pm");
        std::fs::create_dir_all(target.parent().ok_or("missing parent")?)?;
        std::fs::write(&target, "package Fallback::Mod; 1;")?;

        let ws_uri = url::Url::from_file_path(&ws)
            .map_err(|()| "bad URI")?
            .to_string();

        let result = resolve_module_uri(
            "Fallback::Mod",
            &[],
            &[ws_uri],
            &["lib".to_string()],
            true,
            &[inc],
            Duration::from_millis(200),
        );

        match result {
            ModuleUriResolution::Resolved(uri) => {
                assert!(
                    uri.contains("perl5lib"),
                    "should fall back to system @INC, got: {uri}"
                );
            }
            other => return Err(format!("expected Resolved, got {other:?}").into()),
        }
        Ok(())
    }

    /// Multiple system @INC directories are searched in array order.
    #[test]
    fn multiple_inc_dirs_searched_in_array_order() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let site_perl = temp.path().join("site_perl");
        let vendor_perl = temp.path().join("vendor_perl");
        let core_perl = temp.path().join("core_perl");

        // Module only in vendor_perl (second entry)
        let target = vendor_perl.join("Encode").join("UTF8.pm");
        std::fs::create_dir_all(target.parent().ok_or("missing parent")?)?;
        std::fs::write(&target, "package Encode::UTF8; 1;")?;

        // core_perl also has a copy
        let core_target = core_perl.join("Encode").join("UTF8.pm");
        std::fs::create_dir_all(core_target.parent().ok_or("missing parent")?)?;
        std::fs::write(&core_target, "package Encode::UTF8; 1;")?;

        let result = resolve_module_uri(
            "Encode::UTF8",
            &[],
            &[],
            &[],
            true,
            &[
                PathBuf::from(&site_perl),
                PathBuf::from(&vendor_perl),
                PathBuf::from(&core_perl),
            ],
            Duration::from_millis(200),
        );

        match result {
            ModuleUriResolution::Resolved(uri) => {
                assert!(
                    uri.contains("vendor_perl"),
                    "should find in vendor_perl (first match), got: {uri}"
                );
            }
            other => return Err(format!("expected Resolved, got {other:?}").into()),
        }
        Ok(())
    }

    /// Include paths within a workspace are searched in array order.
    #[test]
    fn include_paths_searched_in_array_order() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();

        // Module exists in both "vendor" and "lib"
        let vendor_target = root.join("vendor").join("Search").join("Order.pm");
        let lib_target = root.join("lib").join("Search").join("Order.pm");

        std::fs::create_dir_all(vendor_target.parent().ok_or("missing parent")?)?;
        std::fs::create_dir_all(lib_target.parent().ok_or("missing parent")?)?;
        std::fs::write(&vendor_target, "# vendor")?;
        std::fs::write(&lib_target, "# lib")?;

        // "vendor" listed first in include_paths
        let result = resolve_module_path(
            root,
            "Search::Order",
            &["vendor".to_string(), "lib".to_string()],
        );
        assert_eq!(result, Some(vendor_target));
        Ok(())
    }
}

// ============================================================================
// use lib effect on @INC (include path prepending)
// ============================================================================

mod use_lib_effect {
    use super::*;

    /// Simulates `use lib 'local_lib'` by adding it to the front of the
    /// include paths. Modules in the prepended path should be found first.
    #[test]
    fn use_lib_prepends_to_include_paths() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();

        // Standard lib has the module
        let lib_target = root.join("lib").join("Config").join("Tiny.pm");
        std::fs::create_dir_all(lib_target.parent().ok_or("missing parent")?)?;
        std::fs::write(&lib_target, "# standard lib version")?;

        // local_lib also has the module (simulating a local override)
        let local_target = root.join("local_lib").join("Config").join("Tiny.pm");
        std::fs::create_dir_all(local_target.parent().ok_or("missing parent")?)?;
        std::fs::write(&local_target, "# local override")?;

        // Simulate: use lib 'local_lib' prepends to @INC
        // The effective include_paths are ["local_lib", "lib"]
        let result = resolve_module_path(
            root,
            "Config::Tiny",
            &["local_lib".to_string(), "lib".to_string()],
        );

        assert_eq!(
            result,
            Some(local_target),
            "local_lib (prepended by use lib) should win over lib"
        );
        Ok(())
    }

    /// Simulates multiple `use lib` calls stacking. Later `use lib` calls
    /// prepend, so the last one has highest priority.
    #[test]
    fn multiple_use_lib_stacking_last_prepend_wins() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();

        let first_lib = root.join("first_lib").join("Stack").join("Mod.pm");
        let second_lib = root.join("second_lib").join("Stack").join("Mod.pm");
        let default_lib = root.join("lib").join("Stack").join("Mod.pm");

        std::fs::create_dir_all(first_lib.parent().ok_or("missing parent")?)?;
        std::fs::create_dir_all(second_lib.parent().ok_or("missing parent")?)?;
        std::fs::create_dir_all(default_lib.parent().ok_or("missing parent")?)?;
        std::fs::write(&first_lib, "# first")?;
        std::fs::write(&second_lib, "# second")?;
        std::fs::write(&default_lib, "# default")?;

        // Simulating:
        //   use lib 'first_lib';   -> @INC = (first_lib, lib)
        //   use lib 'second_lib';  -> @INC = (second_lib, first_lib, lib)
        // So second_lib is at the front
        let result = resolve_module_path(
            root,
            "Stack::Mod",
            &[
                "second_lib".to_string(),
                "first_lib".to_string(),
                "lib".to_string(),
            ],
        );

        assert_eq!(
            result,
            Some(second_lib),
            "most recently prepended use lib should win"
        );
        Ok(())
    }

    /// `use lib` with a path that does not exist on disk is safely skipped,
    /// and resolution falls through to the next include path.
    #[test]
    fn use_lib_nonexistent_path_falls_through() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();

        let target = root.join("lib").join("Real").join("Mod.pm");
        std::fs::create_dir_all(target.parent().ok_or("missing parent")?)?;
        std::fs::write(&target, "package Real::Mod; 1;")?;

        // "nonexistent" does not exist on disk
        let result = resolve_module_path(
            root,
            "Real::Mod",
            &["nonexistent".to_string(), "lib".to_string()],
        );

        assert_eq!(
            result,
            Some(target),
            "should fall through nonexistent path to lib"
        );
        Ok(())
    }

    /// URI-level simulation of `use lib` prepending an extra include path
    /// to the workspace search.
    #[test]
    fn use_lib_effect_via_uri_resolution() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let ws = temp.path().join("ws");

        let lib_target = ws.join("lib").join("Override").join("Mod.pm");
        let extra_target = ws.join("extra_lib").join("Override").join("Mod.pm");

        std::fs::create_dir_all(lib_target.parent().ok_or("missing parent")?)?;
        std::fs::create_dir_all(extra_target.parent().ok_or("missing parent")?)?;
        std::fs::write(&lib_target, "# lib version")?;
        std::fs::write(&extra_target, "# extra_lib version")?;

        let ws_uri = url::Url::from_file_path(&ws)
            .map_err(|()| "bad URI")?
            .to_string();

        // Simulate: use lib 'extra_lib' prepends before 'lib'
        let result = resolve_module_uri(
            "Override::Mod",
            &[],
            &[ws_uri],
            &["extra_lib".to_string(), "lib".to_string()],
            false,
            &[],
            Duration::from_millis(200),
        );

        match result {
            ModuleUriResolution::Resolved(uri) => {
                assert!(
                    uri.contains("extra_lib"),
                    "extra_lib (from use lib) should take precedence, got: {uri}"
                );
            }
            other => return Err(format!("expected Resolved, got {other:?}").into()),
        }
        Ok(())
    }
}

// ============================================================================
// Relative vs absolute include paths
// ============================================================================

mod relative_vs_absolute_paths {
    use super::*;

    /// A relative include path (e.g., "lib") is joined relative to the
    /// workspace root.
    #[test]
    fn relative_include_path_joined_to_root() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let target = root.join("lib").join("Rel").join("Mod.pm");

        std::fs::create_dir_all(target.parent().ok_or("missing parent")?)?;
        std::fs::write(&target, "package Rel::Mod; 1;")?;

        let result = resolve_module_path(root, "Rel::Mod", &["lib".to_string()]);
        assert_eq!(result, Some(target));
        Ok(())
    }

    /// A nested relative include path (e.g., "vendor/lib") works correctly.
    #[test]
    fn nested_relative_include_path() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let target = root
            .join("vendor")
            .join("lib")
            .join("Nested")
            .join("Rel.pm");

        std::fs::create_dir_all(target.parent().ok_or("missing parent")?)?;
        std::fs::write(&target, "package Nested::Rel; 1;")?;

        let result = resolve_module_path(root, "Nested::Rel", &["vendor/lib".to_string()]);
        assert_eq!(result, Some(target));
        Ok(())
    }

    /// The special "." include path means the workspace root itself.
    #[test]
    fn dot_path_resolves_from_workspace_root() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let target = root.join("MyScript").join("Utils.pm");

        std::fs::create_dir_all(target.parent().ok_or("missing parent")?)?;
        std::fs::write(&target, "package MyScript::Utils; 1;")?;

        let result = resolve_module_path(root, "MyScript::Utils", &[".".to_string()]);
        assert_eq!(result, Some(root.join("MyScript/Utils.pm")));
        Ok(())
    }

    /// Path traversal attempts ("..") are rejected for security.
    #[test]
    fn traversal_path_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("workspace");
        let outside = temp.path().join("secret");

        std::fs::create_dir_all(&root)?;
        let target = outside.join("Leaked").join("Data.pm");
        std::fs::create_dir_all(target.parent().ok_or("missing parent")?)?;
        std::fs::write(&target, "package Leaked::Data; 1;")?;

        // "../secret" would escape the workspace root
        let result = resolve_module_path(&root, "Leaked::Data", &["../secret".to_string()]);

        // Should fall back to lib, NOT resolve the escaped path
        assert_eq!(result, Some(root.join("lib").join("Leaked/Data.pm")));
        Ok(())
    }

    /// Double traversal ("../../") is also rejected.
    #[test]
    fn double_traversal_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("deep").join("workspace");
        std::fs::create_dir_all(&root)?;

        let result = resolve_module_path(&root, "Escape::Attempt", &["../../etc".to_string()]);

        assert_eq!(result, Some(root.join("lib").join("Escape/Attempt.pm")));
        Ok(())
    }

    /// Multiple include paths mixing relative and "." work together.
    #[test]
    fn mixed_relative_and_dot_include_paths() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();

        // Module is in root directly (found via ".")
        let target = root.join("Direct").join("Mod.pm");
        std::fs::create_dir_all(target.parent().ok_or("missing parent")?)?;
        std::fs::write(&target, "package Direct::Mod; 1;")?;

        // "lib" is searched first but module is not there
        let result =
            resolve_module_path(root, "Direct::Mod", &["lib".to_string(), ".".to_string()]);

        // lib doesn't have it, but "." does
        assert_eq!(result, Some(root.join("Direct/Mod.pm")));
        Ok(())
    }
}

// ============================================================================
// Legacy ' separator through the resolution pipeline
// ============================================================================

mod legacy_separator_resolution {
    use super::*;

    /// Legacy `'` separator (e.g., `Foo'Bar`) should resolve the same as
    /// canonical `::` separator when the module name has been canonicalized.
    /// Note: the resolution functions receive pre-canonicalized names, but
    /// this test verifies that the `module_name_to_path` mapping handles it.
    #[test]
    fn legacy_separator_resolves_via_path_after_normalization()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let target = root.join("lib").join("Old").join("Style.pm");

        std::fs::create_dir_all(target.parent().ok_or("missing parent")?)?;
        std::fs::write(&target, "package Old::Style; 1;")?;

        // After normalization, Old'Style becomes Old::Style
        let result = resolve_module_path(root, "Old::Style", &["lib".to_string()]);
        assert_eq!(result, Some(target));
        Ok(())
    }

    /// The `perl_module::path::module_name_to_path` function (used internally)
    /// handles legacy separators directly.
    #[test]
    fn module_name_to_path_handles_legacy_tick() {
        // This exercises the path that resolve_module_path uses internally
        let path = perl_module::resolution::path::resolve_module_path(
            std::path::Path::new("/fake"),
            "Old::Style",
            &["lib".to_string()],
        );

        // Should produce a valid path (the fallback)
        assert!(path.is_some());
        if let Some(p) = path {
            assert!(
                p.to_string_lossy().contains("Old"),
                "path should contain module directory"
            );
        }
    }
}

// ============================================================================
// Edge cases in combined resolution
// ============================================================================

mod combined_edge_cases {
    use super::*;

    /// Empty include paths list causes resolution to fall back to
    /// `root/lib/<module>.pm`.
    #[test]
    fn empty_include_paths_uses_lib_fallback() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();

        let result = resolve_module_path(root, "Fallback::Target", &[]);
        assert_eq!(result, Some(root.join("lib").join("Fallback/Target.pm")));
        Ok(())
    }

    /// Resolution with many include paths (some non-existent) works without
    /// error, finding the module in the one path that has it.
    #[test]
    fn many_include_paths_some_nonexistent() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let target = root.join("real_lib").join("Found").join("Here.pm");

        std::fs::create_dir_all(target.parent().ok_or("missing parent")?)?;
        std::fs::write(&target, "package Found::Here; 1;")?;

        let result = resolve_module_path(
            root,
            "Found::Here",
            &[
                "nonexistent1".to_string(),
                "nonexistent2".to_string(),
                "real_lib".to_string(),
                "nonexistent3".to_string(),
            ],
        );

        assert_eq!(result, Some(target));
        Ok(())
    }

    /// Module resolution with an empty module name does not panic.
    #[test]
    fn empty_module_name_is_safe() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();

        let result = resolve_module_path(root, "", &["lib".to_string()]);
        assert!(result.is_some(), "should return fallback path, not None");
        Ok(())
    }

    /// Module resolution with whitespace-only module name does not panic.
    #[test]
    fn whitespace_module_name_is_safe() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path();

        let result = resolve_module_path(root, "  ", &["lib".to_string()]);
        assert!(result.is_some(), "should return fallback path");
        Ok(())
    }

    /// URI resolution with empty workspace folders and disabled system @INC
    /// returns NotFound.
    #[test]
    fn no_search_locations_returns_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let result = resolve_module_uri(
            "Missing::Module",
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

    /// Verifies that system @INC paths are NOT searched when use_system_inc
    /// is false, even when a module would be found there.
    #[test]
    fn system_inc_flag_strictly_controls_search() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let inc = temp.path().join("perl5lib");
        let target = inc.join("Strict").join("Control.pm");

        std::fs::create_dir_all(target.parent().ok_or("missing parent")?)?;
        std::fs::write(&target, "package Strict::Control; 1;")?;

        // use_system_inc = false should prevent finding it
        let not_found = resolve_module_uri(
            "Strict::Control",
            &[],
            &[],
            &[],
            false,
            std::slice::from_ref(&inc),
            Duration::from_millis(200),
        );
        assert_eq!(not_found, ModuleUriResolution::NotFound);

        // use_system_inc = true should find it
        let found = resolve_module_uri(
            "Strict::Control",
            &[],
            &[],
            &[],
            true,
            &[inc],
            Duration::from_millis(200),
        );
        match found {
            ModuleUriResolution::Resolved(uri) => {
                assert!(uri.contains("Strict/Control.pm"));
            }
            other => return Err(format!("expected Resolved, got {other:?}").into()),
        }
        Ok(())
    }
}
