//! Canonical workspace noise filtering rules.
//!
//! This crate centralizes the shared ignore directory policy used by workspace
//! discovery and runtime workspace operations.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use std::path::{Component, Path};

const SKIPPED_DIRS: [&str; 9] =
    [".git", ".hg", ".svn", "target", "node_modules", ".cache", "blib", "local", "vendor"];

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
    use super::{SKIPPED_DIRS, is_skipped_dir_name, path_contains_skipped_component};
    use std::path::Path;

    // ── is_skipped_dir_name: default patterns ──────────────────────────

    #[test]
    fn test_skipped_dir_name_git_matches() {
        assert!(is_skipped_dir_name(".git"));
    }

    #[test]
    fn test_skipped_dir_name_hg_matches() {
        assert!(is_skipped_dir_name(".hg"));
    }

    #[test]
    fn test_skipped_dir_name_svn_matches() {
        assert!(is_skipped_dir_name(".svn"));
    }

    #[test]
    fn test_skipped_dir_name_target_matches() {
        assert!(is_skipped_dir_name("target"));
    }

    #[test]
    fn test_skipped_dir_name_node_modules_matches() {
        assert!(is_skipped_dir_name("node_modules"));
    }

    #[test]
    fn test_skipped_dir_name_cache_matches() {
        assert!(is_skipped_dir_name(".cache"));
    }

    #[test]
    fn test_skipped_dir_name_blib_matches() {
        assert!(is_skipped_dir_name("blib"));
    }

    #[test]
    fn test_skipped_dir_name_local_matches() {
        assert!(is_skipped_dir_name("local"));
    }

    #[test]
    fn test_skipped_dir_name_vendor_matches() {
        assert!(is_skipped_dir_name("vendor"));
    }

    #[test]
    fn test_skipped_dir_name_all_canonical_patterns() {
        // Verify every entry in the constant is recognized
        for name in &SKIPPED_DIRS {
            assert!(is_skipped_dir_name(name), "expected '{name}' to be skipped");
        }
    }

    #[test]
    fn test_skipped_dir_name_constant_has_expected_count() {
        assert_eq!(SKIPPED_DIRS.len(), 9);
    }

    // ── is_skipped_dir_name: non-matching names ────────────────────────

    #[test]
    fn test_skipped_dir_name_rejects_common_directories() {
        for name in ["src", "lib", "tmp", "dist", "build"] {
            assert!(!is_skipped_dir_name(name), "'{name}' should not be skipped");
        }
    }

    #[test]
    fn test_skipped_dir_name_case_sensitive_git() {
        // Matching is case-sensitive; ".Git" and ".GIT" are not skipped
        assert!(!is_skipped_dir_name(".Git"));
        assert!(!is_skipped_dir_name(".GIT"));
        assert!(!is_skipped_dir_name(".GIT"));
    }

    #[test]
    fn test_skipped_dir_name_case_sensitive_target() {
        assert!(!is_skipped_dir_name("Target"));
        assert!(!is_skipped_dir_name("TARGET"));
    }

    #[test]
    fn test_skipped_dir_name_case_sensitive_node_modules() {
        assert!(!is_skipped_dir_name("Node_Modules"));
        assert!(!is_skipped_dir_name("NODE_MODULES"));
    }

    #[test]
    fn test_skipped_dir_name_prefix_suffix_not_matched() {
        // Substrings and extensions must not match
        assert!(!is_skipped_dir_name(".git2"));
        assert!(!is_skipped_dir_name("git"));
        assert!(!is_skipped_dir_name(".gitignore"));
        assert!(!is_skipped_dir_name("my_target"));
        assert!(!is_skipped_dir_name("targets"));
        assert!(!is_skipped_dir_name("node_modules_backup"));
    }

    #[test]
    fn test_skipped_dir_name_empty_string() {
        assert!(!is_skipped_dir_name(""));
    }

    #[test]
    fn test_skipped_dir_name_whitespace() {
        assert!(!is_skipped_dir_name(" "));
        assert!(!is_skipped_dir_name(" .git"));
        assert!(!is_skipped_dir_name(".git "));
    }

    #[test]
    fn test_skipped_dir_name_dot_only() {
        assert!(!is_skipped_dir_name("."));
        assert!(!is_skipped_dir_name(".."));
    }

    #[test]
    fn test_skipped_dir_name_slash_embedded() {
        // A name containing a slash is never a single directory name
        assert!(!is_skipped_dir_name(".git/objects"));
        assert!(!is_skipped_dir_name("node_modules/pkg"));
    }

    // ── path_contains_skipped_component: basic matching ────────────────

    #[test]
    fn test_path_skipped_component_git_at_root() {
        assert!(path_contains_skipped_component(Path::new(".git")));
    }

    #[test]
    fn test_path_skipped_component_git_nested() {
        assert!(path_contains_skipped_component(Path::new("repo/.git/objects/pack")));
    }

    #[test]
    fn test_path_skipped_component_node_modules_middle() {
        assert!(path_contains_skipped_component(Path::new(
            "workspace/node_modules/Some/Module.pm"
        )));
    }

    #[test]
    fn test_path_skipped_component_target_leaf() {
        assert!(path_contains_skipped_component(Path::new("project/target")));
    }

    #[test]
    fn test_path_skipped_component_each_pattern_in_path() {
        for name in &SKIPPED_DIRS {
            let p = format!("some/dir/{name}/file.txt");
            assert!(
                path_contains_skipped_component(Path::new(&p)),
                "path containing '{name}' should be flagged"
            );
        }
    }

    // ── path_contains_skipped_component: clean paths ───────────────────

    #[test]
    fn test_path_clean_lib_path_not_skipped() {
        assert!(!path_contains_skipped_component(Path::new("repo/lib/My/Module.pm")));
    }

    #[test]
    fn test_path_clean_src_path_not_skipped() {
        assert!(!path_contains_skipped_component(Path::new("crates/perl-parser/src/lib.rs")));
    }

    #[test]
    fn test_path_clean_deep_nesting_not_skipped() {
        assert!(!path_contains_skipped_component(Path::new("a/b/c/d/e/f/g/h/file.pm")));
    }

    // ── path_contains_skipped_component: nested / multiple skipped ─────

    #[test]
    fn test_path_multiple_skipped_components() {
        // Both .git and node_modules present
        assert!(path_contains_skipped_component(Path::new(".git/node_modules/something")));
    }

    #[test]
    fn test_path_skipped_inside_skipped() {
        assert!(path_contains_skipped_component(Path::new("target/.cache/build")));
    }

    #[test]
    fn test_path_deeply_nested_skipped() {
        assert!(path_contains_skipped_component(Path::new("a/b/c/d/e/.svn/f/g")));
    }

    // ── path_contains_skipped_component: edge cases ────────────────────

    #[test]
    fn test_path_empty_path_not_skipped() {
        assert!(!path_contains_skipped_component(Path::new("")));
    }

    #[test]
    fn test_path_dot_not_skipped() {
        assert!(!path_contains_skipped_component(Path::new(".")));
    }

    #[test]
    fn test_path_dotdot_not_skipped() {
        assert!(!path_contains_skipped_component(Path::new("..")));
    }

    #[test]
    fn test_path_root_only_not_skipped() {
        assert!(!path_contains_skipped_component(Path::new("/")));
    }

    #[test]
    fn test_path_absolute_with_skipped() {
        assert!(path_contains_skipped_component(Path::new("/home/user/project/.git/config")));
    }

    #[test]
    fn test_path_absolute_without_skipped() {
        assert!(!path_contains_skipped_component(Path::new("/home/user/project/lib/Module.pm")));
    }

    #[test]
    fn test_path_trailing_slash_with_skipped() {
        // Path::new normalizes trailing separators; the component is still detected
        assert!(path_contains_skipped_component(Path::new("repo/.git/")));
    }

    #[test]
    fn test_path_dotdot_before_skipped() {
        assert!(path_contains_skipped_component(Path::new("../other/node_modules/pkg")));
    }

    #[test]
    fn test_path_dot_before_skipped() {
        assert!(path_contains_skipped_component(Path::new("./target/debug/binary")));
    }

    #[test]
    fn test_path_skipped_name_as_file_extension_not_matched() {
        // "file.target" is a single component; not equal to "target"
        assert!(!path_contains_skipped_component(Path::new("dir/file.target")));
    }

    #[test]
    fn test_path_skipped_name_substring_in_component() {
        // "my_target_dir" contains "target" as a substring but is not equal
        assert!(!path_contains_skipped_component(Path::new("project/my_target_dir/file.rs")));
    }

    #[test]
    fn test_path_only_skipped_component() {
        // Path consists solely of one skipped name
        assert!(path_contains_skipped_component(Path::new("node_modules")));
        assert!(path_contains_skipped_component(Path::new(".hg")));
    }

    #[test]
    fn test_path_single_non_skipped_component() {
        assert!(!path_contains_skipped_component(Path::new("src")));
        assert!(!path_contains_skipped_component(Path::new("lib")));
    }

    #[test]
    fn test_path_unicode_component_not_skipped() {
        assert!(!path_contains_skipped_component(Path::new("repo/\u{00e9}t\u{00e9}/Module.pm")));
    }

    #[test]
    fn test_path_case_sensitive_git_in_path() {
        assert!(!path_contains_skipped_component(Path::new("repo/.Git/config")));
        assert!(!path_contains_skipped_component(Path::new("repo/.GIT/config")));
    }
}
