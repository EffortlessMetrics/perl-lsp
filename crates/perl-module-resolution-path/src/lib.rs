//! Workspace-aware Perl module path resolution.
//!
//! This microcrate has a narrow responsibility: convert a Perl module name into a
//! canonical filesystem path candidate under a workspace root.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use std::path::{Path, PathBuf};

use perl_module_path::module_name_to_path;
use perl_path_security::validate_workspace_path;

/// Resolve a Perl module name to a workspace-relative filesystem path candidate.
///
/// The search order is:
/// 1. Each `include_path` under `root`, rejecting path traversal.
/// 2. Fallback to `root/lib/<module>.pm`.
#[must_use]
pub fn resolve_module_path(
    root: &Path,
    module_name: &str,
    include_paths: &[String],
) -> Option<PathBuf> {
    let relative_path = module_name_to_path(module_name);

    for base in include_paths {
        let candidate = if base == "." {
            root.join(&relative_path)
        } else {
            root.join(base).join(&relative_path)
        };

        let safe_candidate = match validate_workspace_path(&candidate, root) {
            Ok(path) => path,
            Err(_) => continue,
        };

        if safe_candidate.exists() {
            return Some(safe_candidate);
        }
    }

    Some(root.join("lib").join(relative_path))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::tempdir;

    use super::resolve_module_path;

    #[test]
    fn returns_first_safe_candidate() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempdir()?;
        let root = temp.path().to_path_buf();
        let module_file = root.join("lib").join("Foo").join("Bar.pm");

        fs::create_dir_all(module_file.parent().ok_or("no parent")?)?;
        fs::write(&module_file, "package Foo::Bar; 1;")?;

        let resolved = resolve_module_path(&root, "Foo::Bar", &["lib".to_string()]);
        assert_eq!(resolved, Some(root.join("lib").join("Foo/Bar.pm")));
        Ok(())
    }

    #[test]
    fn rejects_traversal_candidate_and_falls_back_to_lib() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = tempfile::tempdir()?.path().to_path_buf();
        let resolved = resolve_module_path(&root, "Escaped::Target", &["..".to_string()]);

        assert_eq!(resolved, Some(root.join("lib").join("Escaped/Target.pm")));
        Ok(())
    }

    #[test]
    fn accepts_current_directory_as_include_path() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().to_path_buf();
        let module_file = root.join("Local").join("Only.pm");

        std::fs::create_dir_all(module_file.parent().ok_or("no parent")?)?;
        std::fs::write(&module_file, "package Local::Only; 1;")?;

        let resolved = resolve_module_path(&root, "Local::Only", &[".".to_string()]);
        assert_eq!(resolved, Some(root.join("Local/Only.pm")));
        Ok(())
    }

    #[test]
    fn accepts_absolute_include_path_inside_workspace() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().to_path_buf();
        let absolute_base = root.join("vendor").join("lib");
        let module_file = absolute_base.join("Vendor").join("Tool.pm");

        fs::create_dir_all(module_file.parent().ok_or("no parent")?)?;
        fs::write(&module_file, "package Vendor::Tool; 1;")?;

        let resolved = resolve_module_path(
            &root,
            "Vendor::Tool",
            &[absolute_base.to_string_lossy().to_string()],
        );
        assert_eq!(resolved, Some(module_file));
        Ok(())
    }

    #[test]
    fn falls_back_when_absolute_include_path_is_outside_workspace()
    -> Result<(), Box<dyn std::error::Error>> {
        let workspace = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        let root = workspace.path().to_path_buf();
        let outside_base = outside.path().join("lib");
        let fallback = root.join("lib").join("Outside").join("Mod.pm");

        fs::create_dir_all(fallback.parent().ok_or("no parent")?)?;
        fs::write(&fallback, "package Outside::Mod; 1;")?;

        let resolved = resolve_module_path(
            &root,
            "Outside::Mod",
            &[outside_base.to_string_lossy().to_string()],
        );
        assert_eq!(resolved, Some(fallback));
        Ok(())
    }
}
