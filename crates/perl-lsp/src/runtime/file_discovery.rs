//! File discovery abstraction for workspace scanning.
//!
//! Provides a two-strategy approach: try `git ls-files` first for speed
//! and .gitignore awareness, then fall back to `WalkDir` enumeration.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use walkdir::WalkDir;

/// How files were discovered
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryMethod {
    /// Files discovered via `git ls-files` (fast, .gitignore-aware)
    Git,
    /// Files discovered via filesystem walk (slower, manual filtering)
    Walk,
}

/// Result of file discovery
#[derive(Debug)]
pub struct DiscoveryResult {
    /// Discovered Perl source files
    pub files: Vec<PathBuf>,
    /// Method used for discovery
    pub method: DiscoveryMethod,
    /// Time taken for discovery
    pub duration: Duration,
    /// Number of entries excluded during discovery
    pub excluded_count: usize,
}

/// Discover Perl source files in the given root directory.
///
/// Strategy:
/// 1. Try `git ls-files -z --cached --others --exclude-standard` for speed
///    and .gitignore awareness. Filter results to Perl extensions.
/// 2. If git fails (not a repo, git not installed, etc.), fall back to WalkDir
///    with the existing skip-directory logic.
pub fn discover_perl_files(root: &Path) -> DiscoveryResult {
    let start = Instant::now();

    // Try git strategy first
    match try_git_discovery(root, start) {
        Ok(result) => result,
        Err(_) => walk_discovery(root, start),
    }
}

/// Check if a path has a Perl source file extension.
///
/// Recognized extensions: `.pl`, `.pm`, `.t`, `.psgi` (case-insensitive).
fn is_perl_source_file(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
        return false;
    };
    matches!(ext.to_ascii_lowercase().as_str(), "pl" | "pm" | "t" | "psgi")
}

/// Check if a directory entry should be skipped during filesystem walk.
///
/// Skips: `.git`, `.hg`, `.svn`, `target`, `node_modules`, `.cache`.
fn should_skip_dir(entry: &walkdir::DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }
    let name = entry.file_name().to_string_lossy();
    matches!(name.as_ref(), ".git" | ".hg" | ".svn" | "target" | "node_modules" | ".cache")
}

/// Attempt file discovery using `git ls-files`.
///
/// Runs `git ls-files -z --cached --others --exclude-standard` in the given
/// root directory, parses NUL-delimited output, and filters to Perl source
/// files. Returns an error if git is not available or the directory is not
/// a git repository.
fn try_git_discovery(root: &Path, start: Instant) -> Result<DiscoveryResult, std::io::Error> {
    let output = std::process::Command::new("git")
        .args(["ls-files", "-z", "--cached", "--others", "--exclude-standard"])
        .current_dir(root)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()?;

    if !output.status.success() {
        return Err(std::io::Error::other("git ls-files failed"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut files = Vec::new();
    let mut excluded_count: usize = 0;

    for entry in stdout.split('\0') {
        if entry.is_empty() {
            continue;
        }
        let path = root.join(entry);
        if is_perl_source_file(&path) {
            files.push(path);
        } else {
            excluded_count += 1;
        }
    }

    let duration = start.elapsed();
    let result = DiscoveryResult { files, method: DiscoveryMethod::Git, duration, excluded_count };

    eprintln!(
        "[perl-lsp] File discovery: {} files via {:?} in {:.1}ms (excluded: {})",
        result.files.len(),
        result.method,
        result.duration.as_secs_f64() * 1000.0,
        result.excluded_count
    );

    Ok(result)
}

/// Discover files by walking the filesystem with WalkDir.
///
/// Uses `follow_links(false)` and skips directories matching the
/// `should_skip_dir` predicate. This is the fallback when git is not
/// available.
fn walk_discovery(root: &Path, start: Instant) -> DiscoveryResult {
    let mut files = Vec::new();
    let mut excluded_count: usize = 0;

    for entry in
        WalkDir::new(root).follow_links(false).into_iter().filter_entry(|e| !should_skip_dir(e))
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        if entry.file_type().is_file() {
            if is_perl_source_file(entry.path()) {
                files.push(entry.path().to_path_buf());
            } else {
                excluded_count += 1;
            }
        }
    }

    let duration = start.elapsed();
    let result = DiscoveryResult { files, method: DiscoveryMethod::Walk, duration, excluded_count };

    eprintln!(
        "[perl-lsp] File discovery: {} files via {:?} in {:.1}ms (excluded: {})",
        result.files.len(),
        result.method,
        result.duration.as_secs_f64() * 1000.0,
        result.excluded_count
    );

    result
}
