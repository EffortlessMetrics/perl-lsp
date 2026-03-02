//! Shared workspace ignore rules used by filesystem discovery/indexing crates.

use std::path::{Component, Path};

const IGNORED_WORKSPACE_DIRS: [&str; 6] =
    [".git", ".hg", ".svn", "target", "node_modules", ".cache"];

/// Returns `true` when `name` matches a conventional workspace noise directory.
#[must_use]
pub fn is_ignored_workspace_dir_name(name: &str) -> bool {
    IGNORED_WORKSPACE_DIRS.contains(&name)
}

/// Returns `true` if any normal path component is an ignored workspace directory.
#[must_use]
pub fn path_contains_ignored_workspace_component(path: &Path) -> bool {
    for component in path.components() {
        if let Component::Normal(name) = component
            && let Some(value) = name.to_str()
            && is_ignored_workspace_dir_name(value)
        {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::{is_ignored_workspace_dir_name, path_contains_ignored_workspace_component};
    use std::path::Path;

    #[test]
    fn known_noise_dirs_are_ignored() {
        assert!(is_ignored_workspace_dir_name(".git"));
        assert!(is_ignored_workspace_dir_name("target"));
        assert!(is_ignored_workspace_dir_name("node_modules"));
        assert!(!is_ignored_workspace_dir_name("src"));
    }

    #[test]
    fn path_component_detection_only_checks_normal_components() {
        assert!(path_contains_ignored_workspace_component(Path::new("/repo/node_modules/pkg.pm")));
        assert!(path_contains_ignored_workspace_component(Path::new(
            "/repo/target/build/generated.pm"
        )));
        assert!(!path_contains_ignored_workspace_component(Path::new("/repo/lib/My/Module.pm")));
    }
}
