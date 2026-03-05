//! Canonical workspace noise filtering rules.
//!
//! This crate centralizes the shared ignore directory policy used by workspace
//! discovery and runtime workspace operations.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use std::path::{Component, Path};

const SKIPPED_DIRS: [&str; 6] = [".git", ".hg", ".svn", "target", "node_modules", ".cache"];

/// Returns true when `name` matches one of the canonical workspace noise directories.
#[must_use]
pub fn is_skipped_dir_name(name: &str) -> bool {
    SKIPPED_DIRS.contains(&name)
}

/// Returns true when any path component belongs to the canonical skipped directory set.
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
    fn identifies_skipped_names() {
        for name in [".git", ".hg", ".svn", "target", "node_modules", ".cache"] {
            assert!(is_skipped_dir_name(name));
        }
    }

    #[test]
    fn rejects_non_skipped_names() {
        for name in ["src", "lib", "blib", "tmp"] {
            assert!(!is_skipped_dir_name(name));
        }
    }

    #[test]
    fn path_component_detection_works() {
        assert!(path_contains_skipped_component(Path::new("repo/node_modules/pkg.pm")));
        assert!(!path_contains_skipped_component(Path::new("repo/lib/My/Module.pm")));
    }
}
