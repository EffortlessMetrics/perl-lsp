//! Deterministic and secure Perl module resolution helpers.
//!
//! This crate centralizes module resolution behavior shared by workspace-aware
//! tools such as LSP servers. It resolves module names in a strict precedence
//! order and applies path-safety checks for workspace-relative lookups.

use perl_module_path::module_name_to_path;
use perl_path_security::validate_workspace_path;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use url::Url;

#[cfg(not(target_arch = "wasm32"))]
use perl_uri::uri_to_fs_path;

/// Outcome of a module name to URI resolution attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleUriResolution {
    /// A matching module URI was found.
    Resolved(String),
    /// No matching module was found.
    NotFound,
    /// Resolution stopped because the timeout budget was exhausted.
    TimedOut,
}

/// Resolve a module name to a filesystem path under a workspace root.
///
/// Search order:
/// 1. `include_paths` entries (with workspace path validation)
/// 2. Fallback to `root/lib/<module>.pm`
#[must_use]
pub fn resolve_module_path(
    root: &Path,
    module_name: &str,
    include_paths: &[String],
) -> Option<PathBuf> {
    let relative_path = module_name_to_path(module_name);

    for base in include_paths {
        let candidate = if base == "." {
            root.join(&relative_path)
        } else {
            root.join(base).join(&relative_path)
        };

        let safe_candidate = match validate_workspace_path(&candidate, root) {
            Ok(path) => path,
            Err(_) => continue,
        };

        if safe_candidate.exists() {
            return Some(safe_candidate);
        }
    }

    Some(root.join("lib").join(relative_path))
}

/// Resolve a module name to a `file://` URI using deterministic precedence.
///
/// Search order:
/// 1. Open document URIs (`ends_with` match on module-relative path)
/// 2. Workspace folders + `include_paths` (path-safe filesystem checks)
/// 3. System `@INC` paths (only when `use_system_inc` is true)
#[must_use]
pub fn resolve_module_uri(
    module_name: &str,
    open_document_uris: &[String],
    workspace_folders: &[String],
    include_paths: &[String],
    use_system_inc: bool,
    system_inc: &[PathBuf],
    timeout: Duration,
) -> ModuleUriResolution {
    let start_time = Instant::now();
    let relative_path = module_name_to_path(module_name);

    for uri in open_document_uris {
        if uri.ends_with(&relative_path) {
            return ModuleUriResolution::Resolved(uri.clone());
        }
    }

    for workspace_folder in workspace_folders {
        if start_time.elapsed() > timeout {
            return ModuleUriResolution::TimedOut;
        }

        let workspace_path = workspace_folder_to_path(workspace_folder);

        for include_path in include_paths {
            if start_time.elapsed() > timeout {
                return ModuleUriResolution::TimedOut;
            }

            let full_path = if include_path == "." {
                workspace_path.join(&relative_path)
            } else {
                workspace_path.join(include_path).join(&relative_path)
            };

            let full_path = match validate_workspace_path(&full_path, &workspace_path) {
                Ok(path) => path,
                Err(_) => continue,
            };

            if full_path.is_file()
                && let Ok(url) = Url::from_file_path(&full_path)
            {
                return ModuleUriResolution::Resolved(url.to_string());
            }
        }
    }

    if use_system_inc {
        for inc_path in system_inc {
            if start_time.elapsed() > timeout {
                return ModuleUriResolution::TimedOut;
            }

            let full_path = inc_path.join(&relative_path);
            if full_path.is_file()
                && let Ok(url) = Url::from_file_path(&full_path)
            {
                return ModuleUriResolution::Resolved(url.to_string());
            }
        }
    }

    ModuleUriResolution::NotFound
}

fn workspace_folder_to_path(workspace_folder: &str) -> PathBuf {
    if workspace_folder.starts_with("file://") {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = uri_to_fs_path(workspace_folder) {
            return path;
        }

        return PathBuf::from(workspace_folder.trim_start_matches("file://"));
    }

    PathBuf::from(workspace_folder)
}

#[cfg(test)]
mod tests {
    use super::workspace_folder_to_path;

    #[test]
    fn parses_plain_workspace_folder_path() {
        let path = workspace_folder_to_path("/tmp/project");
        assert_eq!(path.to_string_lossy(), "/tmp/project");
    }

    #[test]
    fn parses_file_uri_workspace_folder() {
        let path = workspace_folder_to_path("file:///tmp/project");
        assert!(path.to_string_lossy().contains("tmp"));
        assert!(path.to_string_lossy().contains("project"));
    }
}
