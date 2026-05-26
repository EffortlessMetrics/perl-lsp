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
    /// Number of discovered version sites considered during the bump.
    pub sites_total: usize,
    /// Sites whose version string was rewritten to the new value.
    pub sites_updated: usize,
    /// Sites that already matched the target version and required no change.
    pub sites_unchanged: usize,
    /// Distinct files that received at least one update.
    pub files_updated: usize,
    /// Repo-relative paths of every file that was modified, in walk order.
    pub touched_files: Vec<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // VersionSite::new — standard (non-channel-split) constructor
    // -----------------------------------------------------------------------

    #[test]
    fn version_site_new_sets_channel_split_false() {
        let site = VersionSite::new(
            PathBuf::from("Cargo.toml"),
            1,
            "workspace version".to_string(),
            "1.2.3".to_string(),
        );
        assert!(!site.channel_split, "VersionSite::new must set channel_split = false");
    }

    #[test]
    fn version_site_new_stores_path() {
        let path = PathBuf::from("crates/foo/Cargo.toml");
        let site = VersionSite::new(path.clone(), 5, "desc".to_string(), "0.1.0".to_string());
        assert_eq!(site.path, path);
    }

    #[test]
    fn version_site_new_stores_line() {
        let site =
            VersionSite::new(PathBuf::from("x"), 42, "desc".to_string(), "1.0.0".to_string());
        assert_eq!(site.line, 42);
    }

    #[test]
    fn version_site_new_stores_description() {
        let description = "[workspace.package] version".to_string();
        let site =
            VersionSite::new(PathBuf::from("x"), 1, description.clone(), "1.0.0".to_string());
        assert_eq!(site.description, description);
    }

    #[test]
    fn version_site_new_stores_found() {
        let found = "0.14.0-rc1".to_string();
        let site = VersionSite::new(PathBuf::from("x"), 1, "desc".to_string(), found.clone());
        assert_eq!(site.found, found);
    }

    // -----------------------------------------------------------------------
    // VersionSite::channel — channel-split constructor
    // -----------------------------------------------------------------------

    #[test]
    fn version_site_channel_sets_channel_split_true() {
        let site = VersionSite::channel(
            PathBuf::from("vscode-extension/package.json"),
            2,
            "vscode version".to_string(),
            "0.13.0".to_string(),
        );
        assert!(site.channel_split, "VersionSite::channel must set channel_split = true");
    }

    #[test]
    fn version_site_channel_stores_path() {
        let path = PathBuf::from("vscode-extension/package.json");
        let site = VersionSite::channel(path.clone(), 2, "desc".to_string(), "0.13.0".to_string());
        assert_eq!(site.path, path);
    }

    #[test]
    fn version_site_channel_stores_line() {
        let site =
            VersionSite::channel(PathBuf::from("x"), 7, "desc".to_string(), "0.1.0".to_string());
        assert_eq!(site.line, 7);
    }

    #[test]
    fn version_site_channel_stores_description() {
        let description = "vscode-extension package.json version".to_string();
        let site =
            VersionSite::channel(PathBuf::from("x"), 1, description.clone(), "0.1.0".to_string());
        assert_eq!(site.description, description);
    }

    #[test]
    fn version_site_channel_stores_found() {
        let found = "0.12.4".to_string();
        let site = VersionSite::channel(PathBuf::from("x"), 1, "desc".to_string(), found.clone());
        assert_eq!(site.found, found);
    }

    // -----------------------------------------------------------------------
    // BumpReport::default — zero/empty initial state
    // -----------------------------------------------------------------------

    #[test]
    fn bump_report_default_sites_total_is_zero() {
        let report = BumpReport::default();
        assert_eq!(report.sites_total, 0);
    }

    #[test]
    fn bump_report_default_sites_updated_is_zero() {
        let report = BumpReport::default();
        assert_eq!(report.sites_updated, 0);
    }

    #[test]
    fn bump_report_default_sites_unchanged_is_zero() {
        let report = BumpReport::default();
        assert_eq!(report.sites_unchanged, 0);
    }

    #[test]
    fn bump_report_default_files_updated_is_zero() {
        let report = BumpReport::default();
        assert_eq!(report.files_updated, 0);
    }

    #[test]
    fn bump_report_default_touched_files_is_empty() {
        let report = BumpReport::default();
        assert!(report.touched_files.is_empty());
    }
}
