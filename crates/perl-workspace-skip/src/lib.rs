//! Shared skip-directory policy for workspace scanning.
//!
//! Consumers can use these helpers to apply the same conservative filtering
//! rules for non-source directories across git output parsing and filesystem walking.

use std::path::{Component, Path};

const SKIPPED_DIRECTORIES: [&str; 6] = [".git", ".hg", ".svn", "target", "node_modules", ".cache"];

/// Returns true when `name` is one of the conventional non-source directory names.
#[must_use]
pub fn should_skip_dir_name(name: &str) -> bool {
    SKIPPED_DIRECTORIES.contains(&name)
}

/// Returns true when any path component should be skipped.
#[must_use]
pub fn path_contains_skipped_component(path: &Path) -> bool {
    for component in path.components() {
        if let Component::Normal(name) = component
            && let Some(value) = name.to_str()
            && should_skip_dir_name(value)
        {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::{path_contains_skipped_component, should_skip_dir_name};
    use std::path::Path;

    #[test]
    fn skipped_names_are_recognized() {
        assert!(should_skip_dir_name(".git"));
        assert!(should_skip_dir_name("node_modules"));
        assert!(should_skip_dir_name("target"));
        assert!(!should_skip_dir_name("src"));
    }

    #[test]
    fn path_components_are_scanned() {
        assert!(path_contains_skipped_component(Path::new("/repo/target/build/Foo.pm")));
        assert!(path_contains_skipped_component(Path::new("/repo/node_modules/pkg.pm")));
        assert!(!path_contains_skipped_component(Path::new("/repo/lib/Foo.pm")));
    }
}
