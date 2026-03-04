//! Shared workspace skip-directory policy.
//!
//! Centralizes conventional non-source directories excluded from workspace
//! traversal and discovery.

use std::path::{Component, Path};

/// Conventional directory names that should be skipped during workspace
/// traversal.
pub const SKIPPED_WORKSPACE_DIRS: [&str; 6] =
    [".git", ".hg", ".svn", "target", "node_modules", ".cache"];

/// Returns `true` if `name` is a conventional skip directory.
#[must_use]
pub fn is_skipped_workspace_dir_name(name: &str) -> bool {
    SKIPPED_WORKSPACE_DIRS.iter().any(|candidate| candidate == &name)
}

/// Returns `true` if `path` contains any conventional skip directory
/// component.
#[must_use]
pub fn path_contains_skipped_workspace_component(path: &Path) -> bool {
    path.components().any(|component| {
        if let Component::Normal(name) = component
            && let Some(value) = name.to_str()
        {
            return is_skipped_workspace_dir_name(value);
        }

        false
    })
}

#[cfg(test)]
mod tests {
    use super::{
        SKIPPED_WORKSPACE_DIRS, is_skipped_workspace_dir_name,
        path_contains_skipped_workspace_component,
    };
    use std::path::Path;

    #[test]
    fn exposes_expected_skip_dirs() {
        assert_eq!(
            SKIPPED_WORKSPACE_DIRS,
            [".git", ".hg", ".svn", "target", "node_modules", ".cache"]
        );
    }

    #[test]
    fn matches_skip_names_exactly() {
        for name in SKIPPED_WORKSPACE_DIRS {
            assert!(is_skipped_workspace_dir_name(name));
        }
        assert!(!is_skipped_workspace_dir_name("src"));
        assert!(!is_skipped_workspace_dir_name("Target"));
    }

    #[test]
    fn detects_skip_components_in_paths() {
        assert!(path_contains_skipped_workspace_component(Path::new("a/.git/config")));
        assert!(path_contains_skipped_workspace_component(Path::new("a/node_modules/pkg.pm")));
        assert!(path_contains_skipped_workspace_component(Path::new("target/debug/build")));
        assert!(!path_contains_skipped_workspace_component(Path::new("lib/My/Module.pm")));
    }
}
