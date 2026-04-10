//! Deterministic Perl module URI resolution helpers.
//!
//! This microcrate extracts timeout-bounded URI resolution policy from the
//! broader `perl-module-resolution` crate.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use perl_module_path::module_name_to_path;
use perl_path_security::validate_workspace_path;
use perl_workspace_folder::workspace_folder_to_path;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use url::Url;

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

#[derive(Debug, Clone)]
struct IncEntry {
    base: PathBuf,
    workspace_root: Option<PathBuf>,
}

/// Resolve a module name to a `file://` URI using deterministic precedence.
///
/// Search order:
/// 1. Open document URIs (`ends_with` match on relative module path)
/// 2. Workspace folders + `include_paths` (relative entries scoped to workspace,
///    absolute entries honored literally)
/// 3. System `@INC` paths (when `use_system_inc` is true)
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

    let mut inc_entries = Vec::new();
    for workspace_folder in workspace_folders {
        if start_time.elapsed() > timeout {
            return ModuleUriResolution::TimedOut;
        }

        let workspace_path = workspace_folder_to_path(workspace_folder);
        for include_path in include_paths {
            let include = Path::new(include_path);
            if include.is_absolute() {
                inc_entries.push(IncEntry { base: include.to_path_buf(), workspace_root: None });
            } else {
                let base = if include_path == "." {
                    workspace_path.clone()
                } else {
                    workspace_path.join(include_path)
                };
                inc_entries.push(IncEntry { base, workspace_root: Some(workspace_path.clone()) });
            }
        }
    }

    if use_system_inc {
        for inc_path in system_inc {
            inc_entries.push(IncEntry { base: inc_path.clone(), workspace_root: None });
        }
    }

    for entry in inc_entries {
        if start_time.elapsed() > timeout {
            return ModuleUriResolution::TimedOut;
        }

        let full_path = entry.base.join(&relative_path);
        let full_path = if let Some(workspace_root) = entry.workspace_root {
            match validate_workspace_path(&full_path, &workspace_root) {
                Ok(path) => path,
                Err(_) => continue,
            }
        } else {
            full_path
        };

        if full_path.is_file()
            && let Ok(url) = Url::from_file_path(&full_path)
        {
            return ModuleUriResolution::Resolved(url.to_string());
        }
    }

    ModuleUriResolution::NotFound
}
