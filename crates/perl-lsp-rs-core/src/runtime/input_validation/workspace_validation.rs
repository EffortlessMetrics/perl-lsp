use anyhow::{Result, anyhow};
use std::path::Path;

/// Validates workspace root to ensure it's safe.
pub fn validate_workspace_root(workspace_root: &Path) -> Result<()> {
    if !workspace_root.exists() {
        return Err(anyhow!("Workspace root does not exist: {}", workspace_root.display()));
    }

    if !workspace_root.is_dir() {
        return Err(anyhow!("Workspace root is not a directory: {}", workspace_root.display()));
    }

    let path_str = workspace_root.to_string_lossy();
    if path_str.contains("..") || path_str.contains('~') {
        return Err(anyhow!("Suspicious workspace root path: {}", workspace_root.display()));
    }

    Ok(())
}
