//! Workspace-aware Perl module path resolution.
//!
//! Convert a Perl module name into a canonical filesystem path candidate
//! under a workspace root.

use std::path::{Path, PathBuf};

use crate::path::module_name_to_path;
use perl_parser_core::path_security::validate_workspace_path;

/// Resolve a Perl module name to a workspace-relative filesystem path candidate.
///
/// The search order is:
/// 1. Each configured include path in order:
///    - Relative paths are resolved under `root` and validated against traversal.
///    - Absolute paths are treated as literal external roots.
/// 2. Fallback to `root/lib/<module>.pm`.
#[must_use]
pub fn resolve_module_path(
    root: &Path,
    module_name: &str,
    include_paths: &[String],
) -> Option<PathBuf> {
    let relative_path = module_name_to_path(module_name);

    for base in include_paths {
        let base_path = Path::new(base);
        let candidate = if base_path.is_absolute() {
            base_path.join(&relative_path)
        } else if base == "." {
            root.join(&relative_path)
        } else {
            root.join(base).join(&relative_path)
        };

        // For relative paths, validate safety (traversal prevention) but keep
        // the original candidate so the returned path stays relative to `root`
        // without canonicalization (canonicalize expands 8.3 short names on
        // Windows, making the result inconsistent with the caller-supplied root).
        if !base_path.is_absolute() && validate_workspace_path(&candidate, root).is_err() {
            continue;
        }

        if candidate.exists() {
            return Some(candidate);
        }
    }

    Some(root.join("lib").join(relative_path))
}
