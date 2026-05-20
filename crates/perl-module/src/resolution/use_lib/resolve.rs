//! Resolve extracted `use lib` paths against workspace and file locations.

use std::path::{Component, Path, PathBuf};

use super::UseLibPath;

/// Resolve `use lib` paths against a workspace root and optional file directory.
///
/// - Absolute paths are accepted only when they stay under `workspace_root`.
/// - `$FindBin::Bin`-relative paths are resolved against `file_dir` (or `workspace_root` if absent).
/// - Other relative paths are resolved against `workspace_root`.
pub fn resolve_use_lib_paths(
    use_lib_paths: &[UseLibPath],
    workspace_root: &Path,
    file_dir: Option<&Path>,
) -> Vec<String> {
    let mut result = Vec::new();

    for ulp in use_lib_paths {
        let path_str = &ulp.path;

        if ulp.from_findbin {
            let base = file_dir.unwrap_or(workspace_root);
            let Some(resolved) = normalize_findbin_path(base, path_str) else {
                continue;
            };
            if resolved.strip_prefix(workspace_root).is_err() {
                continue;
            }
            if let Some(s) = path_to_relative_string(&resolved, workspace_root)
                && !result.contains(&s)
            {
                result.push(s);
            }
        } else {
            let p = Path::new(path_str);
            if p.is_absolute() {
                if let Some(s) = path_to_relative_string(p, workspace_root)
                    && !result.contains(&s)
                {
                    result.push(s);
                }
            } else {
                let s = path_str.to_string();
                if !result.contains(&s) {
                    result.push(s);
                }
            }
        }
    }

    result
}

fn path_to_relative_string(path: &Path, workspace_root: &Path) -> Option<String> {
    if let Ok(rel) = path.strip_prefix(workspace_root) {
        // Guard against lexical strip_prefix matching an embedded `..` segment.
        // For example, `/workspace/../etc` strips the `/workspace` prefix lexically,
        // leaving `../etc` which would escape the workspace.  Reject any result
        // that contains a parent-directory component.
        if rel.components().any(|c| c == std::path::Component::ParentDir) {
            return None;
        }
        let s = normalize_relative_path_string(rel.to_string_lossy().as_ref());
        if s.is_empty() { Some(".".to_string()) } else { Some(s) }
    } else if path.is_absolute() {
        None
    } else {
        let s = normalize_relative_path_string(path.to_string_lossy().as_ref());
        Some(s)
    }
}

fn normalize_relative_path_string(path: &str) -> String {
    path.replace('\\', "/")
}

fn normalize_findbin_path(base: &Path, relative: &str) -> Option<PathBuf> {
    let mut normalized = PathBuf::from(base);
    for component in Path::new(relative).components() {
        match component {
            Component::CurDir => {}
            Component::Normal(segment) => normalized.push(segment),
            Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(normalized)
}
