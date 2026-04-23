//! Git-aware Perl workspace file discovery.
//!
//! Finds Perl source files in a workspace root with a two-step strategy:
//! 1. Try `git ls-files` for fast, `.gitignore`-aware enumeration.
//! 2. Fall back to filesystem walking with `WalkDir` when git is unavailable.
//!
//! The resulting behavior is intentionally conservative: common non-source directories
//! are skipped in both modes (`.git`, `.hg`, `.svn`, `target`, `node_modules`, `.cache`).

use crate::ignore::{is_skipped_dir_name, path_contains_skipped_component};
use perl_parser_core::source_file::is_perl_source_path;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use walkdir::{DirEntry, WalkDir};

const GIT_LS_FILES_ARGS: [&str; 5] =
    ["ls-files", "-z", "--cached", "--others", "--exclude-standard"];

/// How files were discovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryMethod {
    /// Files discovered via `git ls-files`.
    Git,
    /// Files discovered via `WalkDir` traversal.
    Walk,
}

/// File discovery result metadata.
#[derive(Debug, Clone)]
pub struct DiscoveryResult {
    /// Discovered Perl source files.
    pub files: Vec<PathBuf>,
    /// Discovery method used.
    pub method: DiscoveryMethod,
    /// Elapsed discovery duration.
    pub duration: Duration,
    /// Number of entries excluded by extension/skip rules.
    pub excluded_count: usize,
}

/// Discover Perl source files under `root`.
///
/// Strategy:
/// 1. Attempt `git ls-files -z --cached --others --exclude-standard`
/// 2. If git is unavailable or the root is not a repository, use `WalkDir`
#[must_use]
pub fn discover_perl_files(root: &Path) -> DiscoveryResult {
    let start = Instant::now();

    match try_git_discovery(root, start) {
        Ok(result) => result,
        Err(_) => walk_discovery(root, start),
    }
}

/// Returns `true` if `path` should be considered discoverable by workspace
/// indexing.
///
/// This intentionally includes XS implementation files, SWIG interface files,
/// and common Perl templating formats so editor discovery can surface them
/// even though they are not classified as Perl source files by the shared
/// source-file helper.
#[must_use]
pub fn is_perl_discovery_path(path: &Path) -> bool {
    is_perl_source_path(path)
        || path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| {
            ext.eq_ignore_ascii_case("i")
                || ext.eq_ignore_ascii_case("xs")
                || ext.eq_ignore_ascii_case("ep")
                || ext.eq_ignore_ascii_case("tt")
                || ext.eq_ignore_ascii_case("tt2")
        })
}

fn try_git_discovery(root: &Path, start: Instant) -> Result<DiscoveryResult, std::io::Error> {
    let output = std::process::Command::new("git")
        .args(GIT_LS_FILES_ARGS)
        .current_dir(root)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()?;

    if !output.status.success() {
        return Err(std::io::Error::other("git ls-files failed"));
    }

    let (files, excluded_count) = parse_git_ls_files_output(root, &output.stdout);
    let result = DiscoveryResult {
        files,
        method: DiscoveryMethod::Git,
        duration: start.elapsed(),
        excluded_count,
    };

    log_discovery(&result);
    Ok(result)
}

fn parse_git_ls_files_output(root: &Path, stdout: &[u8]) -> (Vec<PathBuf>, usize) {
    let stdout = String::from_utf8_lossy(stdout);
    let mut files = Vec::new();
    let mut seen = HashSet::new();
    let mut excluded_count: usize = 0;

    for entry in stdout.split('\0') {
        if entry.is_empty() {
            continue;
        }

        let relative_path = Path::new(entry);
        if path_contains_skipped_component(relative_path) {
            excluded_count += 1;
            continue;
        }

        let path = root.join(relative_path);
        if is_perl_discovery_path(&path) {
            if seen.insert(path.clone()) {
                files.push(path);
            } else {
                excluded_count += 1;
            }
        } else {
            excluded_count += 1;
        }
    }

    (files, excluded_count)
}

fn walk_discovery(root: &Path, start: Instant) -> DiscoveryResult {
    let mut files = Vec::new();
    let mut excluded_count: usize = 0;
    let mut skipped_dir_count: usize = 0;

    for entry in WalkDir::new(root).follow_links(false).into_iter().filter_entry(|entry| {
        if should_skip_dir(entry) {
            skipped_dir_count += 1;
            return false;
        }
        true
    }) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        if !entry.file_type().is_file() {
            continue;
        }

        if is_perl_discovery_path(entry.path()) {
            files.push(entry.path().to_path_buf());
        } else {
            excluded_count += 1;
        }
    }
    excluded_count += skipped_dir_count;

    let result = DiscoveryResult {
        files,
        method: DiscoveryMethod::Walk,
        duration: start.elapsed(),
        excluded_count,
    };

    log_discovery(&result);
    result
}

fn should_skip_dir(entry: &DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }

    is_skipped_dir_name(&entry.file_name().to_string_lossy())
}

fn log_discovery(result: &DiscoveryResult) {
    tracing::debug!(
        files = result.files.len(),
        method = ?result.method,
        duration_ms = result.duration.as_secs_f64() * 1000.0,
        excluded = result.excluded_count,
        "workspace discovery complete"
    );
}

#[cfg(test)]
mod tests {
    use super::{
        DiscoveryMethod, parse_git_ls_files_output, path_contains_skipped_component,
        should_skip_dir, walk_discovery,
    };
    use std::fs;
    use std::path::Path;
    use std::time::Instant;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn create_file(root: &Path, relative: &str) -> TestResult {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, "# synthetic\n")?;
        Ok(())
    }

    #[test]
    fn parses_git_output_and_filters_entries() {
        let root = Path::new("/tmp/workspace");
        let payload = b"lib/Foo.pm\0README.md\0node_modules/pkg.pm\0script.pl\0";

        let (files, excluded_count) = parse_git_ls_files_output(root, payload);

        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|path| path.ends_with("lib/Foo.pm")));
        assert!(files.iter().any(|path| path.ends_with("script.pl")));
        assert_eq!(excluded_count, 2);
    }

    #[test]
    fn skipped_component_detection_is_consistent() {
        assert!(path_contains_skipped_component(Path::new("/repo/node_modules/pkg.pm")));
        assert!(path_contains_skipped_component(Path::new("/repo/target/build/generated.pm")));
        assert!(!path_contains_skipped_component(Path::new("/repo/lib/My/Module.pm")));
    }

    #[test]
    fn parse_git_output_ignores_skipped_names_in_workspace_root_path() {
        let root = Path::new("/tmp/target/workspace");
        let payload = b"lib/Foo.pm\0";

        let (files, excluded_count) = parse_git_ls_files_output(root, payload);

        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("lib/Foo.pm"));
        assert_eq!(excluded_count, 0);
    }

    #[test]
    fn walk_discovery_ignores_skipped_directories() -> TestResult {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();

        create_file(root, "lib/Foo.pm")?;
        create_file(root, "node_modules/pkg.pm")?;
        create_file(root, "target/build/generated.pm")?;
        create_file(root, ".cache/precompiled.pm")?;

        let result = walk_discovery(root, Instant::now());
        assert_eq!(result.method, DiscoveryMethod::Walk);
        assert_eq!(result.files.len(), 1);
        assert!(result.files[0].ends_with("lib/Foo.pm"));

        Ok(())
    }

    #[test]
    fn walk_discovery_counts_skipped_directories_as_excluded() -> TestResult {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();

        create_file(root, "lib/Foo.pm")?;
        create_file(root, "node_modules/pkg.pm")?;
        create_file(root, "target/build/generated.pm")?;
        create_file(root, ".cache/precompiled.pm")?;

        let result = walk_discovery(root, Instant::now());
        assert_eq!(result.method, DiscoveryMethod::Walk);
        assert_eq!(result.files.len(), 1);
        assert!(result.files[0].ends_with("lib/Foo.pm"));
        assert_eq!(result.excluded_count, 3);

        Ok(())
    }

    #[test]
    fn should_skip_dir_matches_conventional_noise_directories() -> TestResult {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();

        fs::create_dir_all(root.join(".git"))?;
        fs::create_dir_all(root.join("node_modules"))?;
        fs::create_dir_all(root.join("src"))?;

        let mut seen_git = false;
        let mut seen_node_modules = false;
        let mut seen_src = false;

        for entry in walkdir::WalkDir::new(root).max_depth(1).into_iter().flatten() {
            if entry.path() == root {
                continue;
            }
            let name = entry.file_name().to_string_lossy();
            match name.as_ref() {
                ".git" => {
                    seen_git = true;
                    assert!(should_skip_dir(&entry));
                }
                "node_modules" => {
                    seen_node_modules = true;
                    assert!(should_skip_dir(&entry));
                }
                "src" => {
                    seen_src = true;
                    assert!(!should_skip_dir(&entry));
                }
                _ => {}
            }
        }

        assert!(seen_git);
        assert!(seen_node_modules);
        assert!(seen_src);

        Ok(())
    }

    // --- Additional coverage: parse_git_ls_files_output edge cases ---

    #[test]
    fn parse_git_output_empty_input_returns_nothing() {
        let root = Path::new("/tmp/workspace");
        let (files, excluded_count) = parse_git_ls_files_output(root, b"");
        assert_eq!(files.len(), 0);
        assert_eq!(excluded_count, 0);
    }

    #[test]
    fn parse_git_output_only_null_separators() {
        let root = Path::new("/tmp/workspace");
        let (files, excluded_count) = parse_git_ls_files_output(root, b"\0\0\0");
        assert_eq!(files.len(), 0);
        assert_eq!(excluded_count, 0);
    }

    #[test]
    fn parse_git_output_recognizes_all_perl_extensions() {
        let root = Path::new("/tmp/workspace");
        let payload =
            b"lib/Foo.pm\0scripts/run.pl\0t/basic.t\0app/main.psgi\0ext/native.xs\0templates/page.html.ep\0templates/page.tt\0templates/layout.tt2\0";
        let (files, excluded_count) = parse_git_ls_files_output(root, payload);

        assert_eq!(files.len(), 8);
        assert!(files.iter().any(|p| p.ends_with("Foo.pm")));
        assert!(files.iter().any(|p| p.ends_with("run.pl")));
        assert!(files.iter().any(|p| p.ends_with("basic.t")));
        assert!(files.iter().any(|p| p.ends_with("main.psgi")));
        assert!(files.iter().any(|p| p.ends_with("native.xs")));
        assert!(files.iter().any(|p| p.ends_with("page.html.ep")));
        assert!(files.iter().any(|p| p.ends_with("page.tt")));
        assert!(files.iter().any(|p| p.ends_with("layout.tt2")));
        assert_eq!(excluded_count, 0);
    }

    #[test]
    fn parse_git_output_counts_non_perl_as_excluded() {
        let root = Path::new("/tmp/workspace");
        let payload = b"README.md\0Makefile\0config.yaml\0";
        let (files, excluded_count) = parse_git_ls_files_output(root, payload);

        assert_eq!(files.len(), 0);
        assert_eq!(excluded_count, 3);
    }

    #[test]
    fn parse_git_output_excludes_all_skipped_directories() {
        let root = Path::new("/tmp/workspace");
        let payload = b".git/hooks/pre-commit.pl\0.hg/config.pm\0.svn/entries.pm\0target/out.pm\0node_modules/dep.pm\0.cache/fast.pm\0";
        let (files, excluded_count) = parse_git_ls_files_output(root, payload);

        assert_eq!(files.len(), 0);
        assert_eq!(excluded_count, 6);
    }

    #[test]
    fn parse_git_output_joins_root_to_relative_paths() {
        let root = Path::new("/home/user/project");
        let payload = b"lib/Module.pm\0";
        let (files, _) = parse_git_ls_files_output(root, payload);

        assert_eq!(files.len(), 1);
        assert_eq!(files[0], Path::new("/home/user/project/lib/Module.pm"));
    }

    #[test]
    fn parse_git_output_deduplicates_duplicate_entries() {
        let root = Path::new("/tmp/workspace");
        let payload = b"lib/Foo.pm\0lib/Foo.pm\0script.pl\0script.pl\0README.md\0";

        let (files, excluded_count) = parse_git_ls_files_output(root, payload);

        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|p| p.ends_with("lib/Foo.pm")));
        assert!(files.iter().any(|p| p.ends_with("script.pl")));
        // Two duplicate Perl paths + one non-Perl file.
        assert_eq!(excluded_count, 3);
    }

    // --- Additional coverage: path_contains_skipped_component ---

    #[test]
    fn skipped_component_detects_each_directory_individually() {
        let skipped = [".git", ".hg", ".svn", "target", "node_modules", ".cache"];
        for dir in skipped {
            let path_str = format!("lib/{dir}/nested.pm");
            assert!(
                path_contains_skipped_component(Path::new(&path_str)),
                "expected {dir} to be skipped"
            );
        }
    }

    #[test]
    fn skipped_component_allows_safe_directories() {
        let safe = ["lib", "src", "bin", "t", "scripts"];
        for dir in safe {
            let path_str = format!("{dir}/Module.pm");
            assert!(
                !path_contains_skipped_component(Path::new(&path_str)),
                "expected {dir} to be allowed"
            );
        }
    }

    #[test]
    fn skipped_component_rejects_blib_directory() {
        assert!(path_contains_skipped_component(Path::new("blib/Module.pm")));
    }

    #[test]
    fn skipped_component_empty_path_returns_false() {
        assert!(!path_contains_skipped_component(Path::new("")));
    }

    #[test]
    fn skipped_component_single_filename_returns_false() {
        assert!(!path_contains_skipped_component(Path::new("Module.pm")));
    }

    #[test]
    fn skipped_component_deeply_nested() {
        assert!(path_contains_skipped_component(Path::new("a/b/c/node_modules/d/e/f.pm")));
    }

    // --- Additional coverage: walk_discovery edge cases ---

    #[test]
    fn walk_discovery_empty_directory() -> TestResult {
        let tmp = tempfile::tempdir()?;
        let result = walk_discovery(tmp.path(), Instant::now());

        assert_eq!(result.method, DiscoveryMethod::Walk);
        assert_eq!(result.files.len(), 0);
        assert_eq!(result.excluded_count, 0);

        Ok(())
    }

    #[test]
    fn walk_discovery_only_non_perl_files() -> TestResult {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();

        create_file(root, "README.md")?;
        create_file(root, "Makefile")?;
        create_file(root, "config.yaml")?;

        let result = walk_discovery(root, Instant::now());
        assert_eq!(result.method, DiscoveryMethod::Walk);
        assert_eq!(result.files.len(), 0);
        assert_eq!(result.excluded_count, 3);

        Ok(())
    }

    #[test]
    fn walk_discovery_finds_all_perl_extensions() -> TestResult {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();

        create_file(root, "lib/Foo.pm")?;
        create_file(root, "bin/run.pl")?;
        create_file(root, "t/basic.t")?;
        create_file(root, "app/main.psgi")?;
        create_file(root, "xs/native.xs")?;
        create_file(root, "templates/page.html.ep")?;
        create_file(root, "templates/page.tt")?;
        create_file(root, "templates/layout.tt2")?;

        let result = walk_discovery(root, Instant::now());
        assert_eq!(result.files.len(), 8);
        assert!(result.files.iter().any(|p| p.ends_with("page.html.ep")));
        assert!(result.files.iter().any(|p| p.ends_with("page.tt")));
        assert!(result.files.iter().any(|p| p.ends_with("layout.tt2")));

        Ok(())
    }

    #[test]
    fn walk_discovery_deeply_nested_perl_files() -> TestResult {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();

        create_file(root, "a/b/c/d/e/Deep.pm")?;
        create_file(root, "x/y/z/script.pl")?;

        let result = walk_discovery(root, Instant::now());
        assert_eq!(result.files.len(), 2);
        assert!(result.files.iter().any(|p| p.ends_with("Deep.pm")));
        assert!(result.files.iter().any(|p| p.ends_with("script.pl")));

        Ok(())
    }

    #[test]
    fn walk_discovery_skips_all_six_noise_directories() -> TestResult {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();

        create_file(root, ".git/hooks/hook.pm")?;
        create_file(root, ".hg/config.pm")?;
        create_file(root, ".svn/entries.pm")?;
        create_file(root, "target/build/out.pm")?;
        create_file(root, "node_modules/dep.pm")?;
        create_file(root, ".cache/fast.pm")?;
        create_file(root, "lib/Visible.pm")?;

        let result = walk_discovery(root, Instant::now());
        assert_eq!(result.files.len(), 1);
        assert!(result.files[0].ends_with("lib/Visible.pm"));

        Ok(())
    }

    #[test]
    fn walk_discovery_records_duration() -> TestResult {
        let tmp = tempfile::tempdir()?;
        let result = walk_discovery(tmp.path(), Instant::now());
        // Duration should be non-zero (or at least not panic)
        let _ = result.duration.as_nanos();

        Ok(())
    }

    #[test]
    fn walk_discovery_ignores_subdirectories_themselves() -> TestResult {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();

        // Create a directory that looks like a .pm file (edge case)
        fs::create_dir_all(root.join("lib/Fake.pm/nested"))?;
        create_file(root, "lib/Real.pm")?;

        let result = walk_discovery(root, Instant::now());
        // Only the actual file should be found, not the directory
        assert_eq!(result.files.len(), 1);
        assert!(result.files[0].ends_with("lib/Real.pm"));

        Ok(())
    }

    // --- Additional coverage: should_skip_dir for non-directory entries ---

    #[test]
    fn should_skip_dir_returns_false_for_files() -> TestResult {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();

        // Create a file (not a directory)
        fs::write(root.join("target.txt"), "data")?;

        for entry in walkdir::WalkDir::new(root).max_depth(1).into_iter().flatten() {
            if entry.path() == root {
                continue;
            }
            if entry.file_type().is_file() {
                // Files should never be skipped by should_skip_dir
                assert!(!should_skip_dir(&entry));
            }
        }

        Ok(())
    }

    #[test]
    fn should_skip_dir_covers_all_six_directories() -> TestResult {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();

        let dirs = [".git", ".hg", ".svn", "target", "node_modules", ".cache"];
        for d in dirs {
            fs::create_dir_all(root.join(d))?;
        }

        let mut matched = 0usize;
        for entry in walkdir::WalkDir::new(root).max_depth(1).into_iter().flatten() {
            if entry.path() == root {
                continue;
            }
            if entry.file_type().is_dir() {
                let name = entry.file_name().to_string_lossy();
                if dirs.contains(&name.as_ref()) {
                    assert!(should_skip_dir(&entry), "expected {name} to be skipped");
                    matched += 1;
                }
            }
        }

        assert_eq!(matched, dirs.len());
        Ok(())
    }

    // --- Additional coverage: DiscoveryMethod traits ---

    #[test]
    fn discovery_method_debug_and_equality() {
        let git = DiscoveryMethod::Git;
        let walk = DiscoveryMethod::Walk;
        let git2 = DiscoveryMethod::Git;

        assert_eq!(git, git2);
        assert_ne!(git, walk);
        // Debug is derivable, just verify it doesn't panic
        let _ = format!("{git:?}");
        let _ = format!("{walk:?}");
    }

    #[test]
    fn discovery_method_clone_and_copy() {
        let original = DiscoveryMethod::Git;
        let cloned = original;
        let copied = original;

        assert_eq!(original, cloned);
        assert_eq!(original, copied);
    }

    // --- Additional coverage: DiscoveryResult ---

    #[test]
    fn discovery_result_clone_and_debug() -> TestResult {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();
        create_file(root, "lib/Foo.pm")?;

        let result = walk_discovery(root, Instant::now());
        let cloned = result.clone();

        assert_eq!(cloned.files.len(), result.files.len());
        assert_eq!(cloned.method, result.method);
        assert_eq!(cloned.excluded_count, result.excluded_count);
        // Debug format should not panic
        let _ = format!("{result:?}");

        Ok(())
    }

    // --- Additional coverage: mixed Perl and non-Perl content ---

    #[test]
    fn walk_discovery_mixed_content_accurate_counts() -> TestResult {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path();

        // 3 Perl files
        create_file(root, "lib/A.pm")?;
        create_file(root, "bin/b.pl")?;
        create_file(root, "t/c.t")?;
        // 2 non-Perl files
        create_file(root, "README.md")?;
        create_file(root, "Makefile")?;

        let result = walk_discovery(root, Instant::now());
        assert_eq!(result.files.len(), 3);
        assert_eq!(result.excluded_count, 2);

        Ok(())
    }

    #[test]
    fn parse_git_output_mixed_content_accurate_counts() {
        let root = Path::new("/tmp/workspace");
        let payload =
            b"lib/A.pm\0bin/b.pl\0t/c.t\0app/d.psgi\0README.md\0Makefile\0node_modules/e.pm\0";

        let (files, excluded_count) = parse_git_ls_files_output(root, payload);
        assert_eq!(files.len(), 4);
        // README.md + Makefile (non-perl) + node_modules/e.pm (skipped dir)
        assert_eq!(excluded_count, 3);
    }
}
