//! Marker-based Perl dependency-manager include-path detection.

use std::path::Path;

const CARTON_INCLUDE_PATH: &str = "local/lib/perl5";
const CARMEL_INCLUDE_PATH: &str = "vendor/lib/perl5";

/// Detect include paths produced by Carton and Carmel in a workspace root.
///
/// Carton is identified by `cpanfile` plus either `carton.lock` or
/// `cpanfile.snapshot`. Carmel is identified by `cpanfile` plus an existing
/// `vendor/lib/perl5` directory. The returned paths are workspace-relative
/// and ordered with Carton before Carmel when both markers are present.
#[must_use]
pub fn detect_dependency_include_paths(workspace_root: &Path) -> Vec<String> {
    let has_cpanfile = workspace_root.join("cpanfile").is_file();
    if !has_cpanfile {
        return Vec::new();
    }

    let mut paths = Vec::new();
    let has_carton_lock = workspace_root.join("carton.lock").is_file();
    let has_cpanfile_snapshot = workspace_root.join("cpanfile.snapshot").is_file();
    if has_carton_lock || has_cpanfile_snapshot {
        paths.push(CARTON_INCLUDE_PATH.to_string());
    }

    if workspace_root.join(CARMEL_INCLUDE_PATH).is_dir() {
        paths.push(CARMEL_INCLUDE_PATH.to_string());
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::detect_dependency_include_paths;

    #[test]
    fn detects_carton_from_lockfile() -> Result<(), Box<dyn std::error::Error>> {
        let workspace = tempfile::tempdir()?;
        std::fs::write(workspace.path().join("cpanfile"), "requires 'JSON';\n")?;
        std::fs::write(workspace.path().join("carton.lock"), "snapshot\n")?;

        assert_eq!(detect_dependency_include_paths(workspace.path()), vec!["local/lib/perl5"]);
        Ok(())
    }

    #[test]
    fn detects_carton_from_cpanfile_snapshot() -> Result<(), Box<dyn std::error::Error>> {
        let workspace = tempfile::tempdir()?;
        std::fs::write(workspace.path().join("cpanfile"), "requires 'JSON';\n")?;
        std::fs::write(workspace.path().join("cpanfile.snapshot"), "snapshot\n")?;

        assert_eq!(detect_dependency_include_paths(workspace.path()), vec!["local/lib/perl5"]);
        Ok(())
    }

    #[test]
    fn detects_carmel_from_vendor_root() -> Result<(), Box<dyn std::error::Error>> {
        let workspace = tempfile::tempdir()?;
        std::fs::write(workspace.path().join("cpanfile"), "requires 'JSON';\n")?;
        std::fs::create_dir_all(workspace.path().join("vendor/lib/perl5"))?;

        assert_eq!(detect_dependency_include_paths(workspace.path()), vec!["vendor/lib/perl5"]);
        Ok(())
    }

    #[test]
    fn detects_both_managers_in_stable_order() -> Result<(), Box<dyn std::error::Error>> {
        let workspace = tempfile::tempdir()?;
        std::fs::write(workspace.path().join("cpanfile"), "requires 'JSON';\n")?;
        std::fs::write(workspace.path().join("carton.lock"), "snapshot\n")?;
        std::fs::create_dir_all(workspace.path().join("vendor/lib/perl5"))?;

        assert_eq!(
            detect_dependency_include_paths(workspace.path()),
            vec!["local/lib/perl5", "vendor/lib/perl5"]
        );
        Ok(())
    }

    #[test]
    fn requires_expected_markers() -> Result<(), Box<dyn std::error::Error>> {
        let workspace = tempfile::tempdir()?;
        std::fs::write(workspace.path().join("carton.lock"), "snapshot\n")?;
        assert!(detect_dependency_include_paths(workspace.path()).is_empty());

        std::fs::write(workspace.path().join("cpanfile"), "requires 'JSON';\n")?;
        assert_eq!(detect_dependency_include_paths(workspace.path()), vec!["local/lib/perl5"]);

        std::fs::remove_file(workspace.path().join("carton.lock"))?;
        assert!(detect_dependency_include_paths(workspace.path()).is_empty());

        std::fs::create_dir_all(workspace.path().join("vendor/lib/perl5"))?;
        assert_eq!(detect_dependency_include_paths(workspace.path()), vec!["vendor/lib/perl5"]);
        Ok(())
    }
}
