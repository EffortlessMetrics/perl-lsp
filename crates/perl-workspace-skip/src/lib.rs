//! Canonical workspace skip-directory rules.
//!
//! This microcrate provides one responsibility: identify directories and paths
//! that should be skipped during repository/workspace traversal.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use std::path::{Component, Path};

/// Canonical directory names to skip during workspace traversal.
pub const SKIPPED_DIRECTORY_NAMES: &[&str] =
    &[".git", ".hg", ".svn", "target", "node_modules", ".cache"];

/// Return whether a directory name should be skipped.
#[must_use]
pub fn is_skipped_dir_name(name: &str) -> bool {
    SKIPPED_DIRECTORY_NAMES.contains(&name)
}

/// Return whether any component in `path` should be skipped.
#[must_use]
pub fn path_contains_skipped_component(path: &Path) -> bool {
    path.components().any(|component| {
        if let Component::Normal(name) = component
            && let Some(value) = name.to_str()
        {
            return is_skipped_dir_name(value);
        }

        false
    })
}

#[cfg(test)]
mod tests {
    use super::{SKIPPED_DIRECTORY_NAMES, is_skipped_dir_name, path_contains_skipped_component};
    use std::path::Path;

    #[test]
    fn skipped_directory_names_are_stable() {
        assert_eq!(
            SKIPPED_DIRECTORY_NAMES,
            &[".git", ".hg", ".svn", "target", "node_modules", ".cache"]
        );
    }

    #[test]
    fn identifies_skipped_directory_names() {
        assert!(is_skipped_dir_name("target"));
        assert!(is_skipped_dir_name("node_modules"));
        assert!(!is_skipped_dir_name("src"));
    }

    #[test]
    fn detects_skipped_components_in_paths() {
        assert!(path_contains_skipped_component(Path::new("/repo/node_modules/pkg.pm")));
        assert!(path_contains_skipped_component(Path::new("a/target/build/out.pm")));
        assert!(!path_contains_skipped_component(Path::new("/repo/lib/My/Module.pm")));
    }
}
