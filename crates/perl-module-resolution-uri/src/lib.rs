//! Deterministic Perl module URI resolution helpers.
//!
//! This microcrate extracts the URI-first, timeout-bounded resolution policy from
//! the broader `perl-module-resolution` crate so it can evolve independently.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use perl_module_path::module_name_to_path;
use perl_path_security::validate_workspace_path;
use perl_workspace_folder::workspace_folder_to_path;
use std::path::PathBuf;
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

/// Resolve a module name to a `file://` URI using deterministic precedence.
///
/// Search order:
/// 1. Open document URIs (`ends_with` match on relative module path)
/// 2. Workspace folders + `include_paths` (path-safe filesystem checks)
/// 3. System `@INC` paths (when `use_system_inc` is true)
///
/// The search observes `timeout` and returns [`ModuleUriResolution::TimedOut`] if
/// the budget is exhausted.
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
