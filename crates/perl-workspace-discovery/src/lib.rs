//! Git-aware Perl workspace file discovery.
//!
//! This crate finds Perl source files in a workspace root with a two-step strategy:
//! 1. Try `git ls-files` for fast, `.gitignore`-aware enumeration.
//! 2. Fall back to filesystem walking with `WalkDir` when git is unavailable.
//!
//! The resulting behavior is intentionally conservative: common non-source directories
//! are skipped in both modes (`.git`, `.hg`, `.svn`, `target`, `node_modules`, `.cache`).

use perl_source_file::is_perl_source_path;
use perl_workspace_ignore::{
    is_ignored_workspace_dir_name, path_contains_ignored_workspace_component,
};
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
    let mut excluded_count: usize = 0;

    for entry in stdout.split('\0') {
        if entry.is_empty() {
            continue;
        }

        let relative_path = Path::new(entry);
        if path_contains_ignored_workspace_component(relative_path) {
            excluded_count += 1;
            continue;
        }

        let path = root.join(relative_path);
        if is_perl_source_path(&path) {
            files.push(path);
        } else {
            excluded_count += 1;
        }
    }

    (files, excluded_count)
}

fn walk_discovery(root: &Path, start: Instant) -> DiscoveryResult {
    let mut files = Vec::new();
    let mut excluded_count: usize = 0;

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| !should_skip_dir(entry))
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        if !entry.file_type().is_file() {
            continue;
        }

        if is_perl_source_path(entry.path()) {
            files.push(entry.path().to_path_buf());
        } else {
            excluded_count += 1;
        }
    }

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

    let name = entry.file_name().to_string_lossy();
    is_ignored_workspace_dir_name(name.as_ref())
}

fn log_discovery(result: &DiscoveryResult) {
    eprintln!(
        "[perl-workspace-discovery] {} files via {:?} in {:.1}ms (excluded: {})",
        result.files.len(),
        result.method,
        result.duration.as_secs_f64() * 1000.0,
        result.excluded_count
    );
}

#[cfg(test)]
mod tests {
    use super::{DiscoveryMethod, parse_git_ls_files_output, should_skip_dir, walk_discovery};
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
}
