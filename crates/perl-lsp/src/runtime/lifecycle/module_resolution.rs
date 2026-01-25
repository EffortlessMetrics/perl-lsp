//! Module path resolution
//!
//! Handles resolution of Perl module names to file paths.

use super::super::*;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use url::Url;

impl LspServer {
    /// Enhanced module path resolver using workspace configuration
    ///
    /// Uses configurable include paths from `WorkspaceConfig` instead of
    /// hardcoded directories. Returns absolute filesystem path for a module.
    pub(crate) fn resolve_module_path(&self, module: &str) -> Option<PathBuf> {
        let root = self.root_path.lock().clone()?;
        let rel = module.replace("::", "/") + ".pm";

        // Use configured include paths
        let include_paths = {
            let config = self.workspace_config.lock();
            config.include_paths.clone()
        };

        for base in &include_paths {
            let p = if base == "." { root.join(&rel) } else { root.join(base).join(&rel) };
            if p.exists() {
                return Some(p);
            }
        }
        // Best-effort fallback for test workspaces
        Some(root.join("lib").join(rel))
    }

    /// Resolve a module name to a file path URI
    ///
    /// ## Resolution Precedence Order (deterministic)
    ///
    /// The resolution follows a strict precedence order designed for optimal
    /// developer experience and predictable behavior:
    ///
    /// 1. **Open Documents** (fastest path)
    ///    - Already-opened documents are checked first
    ///    - This ensures edits in progress take precedence
    ///
    /// 2. **Workspace Folders** (in initialization order)
    ///    - Folders are searched in the order they were added
    ///    - For each folder, configured include_paths are searched
    ///    - This respects multi-root workspace priority
    ///
    /// 3. **Configured Include Paths** (user-specified)
    ///    - Custom paths from workspace configuration
    ///    - Relative paths are resolved against each workspace folder
    ///
    /// 4. **System @INC** (opt-in only)
    ///    - Disabled by default (network filesystem concern)
    ///    - Enable via `workspace.useSystemInc: true` in settings
    ///    - Filtered to exclude `.` (current directory) for security
    ///
    /// ## Performance Characteristics
    /// - Timeout: Configurable (default 50ms) to prevent blocking
    /// - Returns None on timeout, allowing graceful degradation
    pub(crate) fn resolve_module_to_path(&self, module_name: &str) -> Option<String> {
        let start_time = Instant::now();
        let relative_path = format!("{}.pm", module_name.replace("::", "/"));

        // Get configuration upfront to minimize lock contention
        let (include_paths, timeout_ms, use_system_inc) = {
            let config = self.workspace_config.lock();
            (config.include_paths.clone(), config.resolution_timeout_ms, config.use_system_inc)
        };
        let timeout = Duration::from_millis(timeout_ms);

        // TIER 1: Open documents (fastest path - in-memory lookup)
        let documents = self.documents.lock();
        for (uri, _doc) in documents.iter() {
            if uri.ends_with(&relative_path) {
                return Some(uri.clone());
            }
        }
        drop(documents);

        // TIER 2 & 3: Workspace folders with configured include paths
        let workspace_folders = self.workspace_folders.lock().clone();

        for workspace_folder in workspace_folders.iter() {
            // Early timeout check
            if start_time.elapsed() > timeout {
                eprintln!(
                    "Module resolution timeout for: {} (elapsed: {:?})",
                    module_name,
                    start_time.elapsed()
                );
                return None;
            }

            // Parse the workspace folder URI to get the file path
            // Use PathBuf for cross-platform path handling (Windows + Unix)
            let workspace_path = if workspace_folder.starts_with("file://") {
                std::path::PathBuf::from(
                    workspace_folder.strip_prefix("file://").unwrap_or(workspace_folder),
                )
            } else {
                std::path::PathBuf::from(workspace_folder)
            };

            // Search configured include paths within this workspace folder
            for dir in &include_paths {
                if start_time.elapsed() > timeout {
                    return None;
                }

                // Use PathBuf::join for cross-platform path construction
                let full_path = if dir == "." {
                    workspace_path.join(&relative_path)
                } else {
                    workspace_path.join(dir).join(&relative_path)
                };

                match std::fs::metadata(&full_path) {
                    Ok(meta) if meta.is_file() => {
                        // Use Url::from_file_path for proper URI construction
                        if let Ok(url) = url::Url::from_file_path(&full_path) {
                            return Some(url.to_string());
                        }
                    }
                    _ => continue,
                }
            }
        }

        // TIER 4: System @INC (opt-in only)
        if use_system_inc {
            if start_time.elapsed() > timeout {
                return None;
            }

            // Get system @INC paths (lazily populated)
            let system_paths = {
                let mut config = self.workspace_config.lock();
                config.get_system_inc().to_vec()
            };

            for inc_path in system_paths {
                if start_time.elapsed() > timeout {
                    return None;
                }

                let full_path = inc_path.join(&relative_path);
                if full_path.is_file() {
                    // Use Url::from_file_path for proper URI construction (Windows-safe)
                    if let Ok(url) = url::Url::from_file_path(&full_path) {
                        return Some(url.to_string());
                    }
                }
            }
        }

        None
    }
}
