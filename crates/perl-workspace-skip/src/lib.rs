//! Canonical skip-directory rules for workspace traversal.

#![deny(unsafe_code)]

use std::path::{Component, Path};

/// Returns true when `name` is a directory that should be skipped during workspace scans.
#[must_use]
pub fn is_skipped_dir_name(name: &str) -> bool {
    matches!(name, ".git" | ".hg" | ".svn" | "target" | "node_modules" | ".cache")
}

/// Returns true if any component in `path` matches a skipped directory name.
#[must_use]
pub fn path_contains_skipped_component(path: &Path) -> bool {
    for component in path.components() {
        if let Component::Normal(name) = component
            && let Some(value) = name.to_str()
            && is_skipped_dir_name(value)
        {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::{is_skipped_dir_name, path_contains_skipped_component};
    use std::path::Path;

    #[test]
    fn skip_dir_name_matches_canonical_set() {
        for name in [".git", ".hg", ".svn", "target", "node_modules", ".cache"] {
            assert!(is_skipped_dir_name(name));
        }
        assert!(!is_skipped_dir_name("src"));
    }

    #[test]
    fn skip_component_detects_any_path_segment() {
        assert!(path_contains_skipped_component(Path::new("/repo/target/build/cache.pm")));
        assert!(path_contains_skipped_component(Path::new("/repo/node_modules/pkg/index.pm")));
        assert!(!path_contains_skipped_component(Path::new("/repo/lib/My/Module.pm")));
    }
}
