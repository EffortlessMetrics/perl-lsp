//! Workspace-aware Perl module path resolution.
//!
//! Convert a Perl module name into a canonical filesystem path candidate
//! under a workspace root.

use std::path::{Path, PathBuf};

use crate::path::{canonicalize_path_long_form, module_name_to_path};
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

        let safe_candidate = if base_path.is_absolute() {
            candidate
        } else {
            match validate_workspace_path(&candidate, root) {
                Ok(path) => path,
                Err(_) => continue,
            }
        };

        if safe_candidate.exists() {
            return Some(canonicalize_path_long_form(&safe_candidate));
        }
    }

    Some(canonicalize_path_long_form(&root.join("lib").join(relative_path)))
}
