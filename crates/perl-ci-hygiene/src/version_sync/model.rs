use std::path::PathBuf;

/// A discovered reference to the workspace version somewhere on disk.
#[derive(Debug, Clone)]
pub struct VersionSite {
    /// Repo-relative path of the file.
    pub path: PathBuf,
    /// 1-based line number inside the file.
    pub line: usize,
    /// Human description of what this site is (for error messages).
    pub description: String,
    /// The version currently written at that site.
    pub found: String,
    /// When true, this site tracks the published/released channel (VS Code Marketplace,
    /// GitHub Releases) and is intentionally allowed to lag behind a pre-release workspace
    /// version. During a pre-release cycle (workspace version contains `-`), mismatches
    /// on channel-split sites are reported as warnings rather than hard failures.
    pub channel_split: bool,
}

impl VersionSite {
    /// Construct a standard (non-channel-split) site.
    pub(crate) fn new(path: PathBuf, line: usize, description: String, found: String) -> Self {
        Self { path, line, description, found, channel_split: false }
    }

    /// Construct a channel-split site that is allowed to lag during pre-release cycles.
    pub(crate) fn channel(path: PathBuf, line: usize, description: String, found: String) -> Self {
        Self { path, line, description, found, channel_split: true }
    }
}

/// Summary returned from [`bump`].
#[derive(Debug, Default)]
pub struct BumpReport {
    pub sites_total: usize,
    pub sites_updated: usize,
    pub sites_unchanged: usize,
    pub files_updated: usize,
    pub touched_files: Vec<PathBuf>,
}
