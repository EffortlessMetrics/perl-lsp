//! Extract include path mutations from `use lib`/`no lib` and `FindBin` statements.

use std::path::{Path, PathBuf};

/// A discovered include path mutation from `use lib` or `no lib`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseLibDelta {
    /// Mutation type (add/remove include roots).
    pub kind: UseLibDeltaKind,
    /// Raw extracted path expression, normalized for FindBin patterns.
    pub path: String,
    /// Whether this path was derived from a `FindBin` variable.
    pub from_findbin: bool,
}

/// Type of include path mutation discovered in source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseLibDeltaKind {
    /// `use lib ...` adds paths at the beginning of the include list.
    Add,
    /// `no lib ...` removes previously-added/configured paths.
    Remove,
}

/// Backwards-compatible alias for callers still expecting only `use lib` additions.
pub type UseLibPath = UseLibDelta;

/// Extract include path mutations from Perl source text.
///
/// Handles:
/// - `use lib 'path';`
/// - `use lib qw(path1 path2);`
/// - `no lib 'path';`
/// - common `FindBin` forms (`$FindBin::Bin`, `$FindBin::RealBin`).
pub fn extract_use_lib_deltas(source: &str) -> Vec<UseLibDelta> {
    let mut deltas = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = strip_lib_prefix(trimmed, "use") {
            extract_paths_from_args(rest, UseLibDeltaKind::Add, &mut deltas);
            continue;
        }
        if let Some(rest) = strip_lib_prefix(trimmed, "no") {
            extract_paths_from_args(rest, UseLibDeltaKind::Remove, &mut deltas);
        }
    }

    deltas
}

/// Extract include paths from `use lib` statements (add-only compatibility API).
pub fn extract_use_lib_paths(source: &str) -> Vec<UseLibPath> {
    extract_use_lib_deltas(source).into_iter().filter(|d| d.kind == UseLibDeltaKind::Add).collect()
}

/// Apply extracted include path deltas against an include path list.
///
/// Paths are resolved as follows:
/// - `FindBin` paths are resolved relative to `file_dir` when available.
/// - Other relative paths stay relative (workspace-root scoped by caller policy).
/// - Absolute paths are preserved as absolute external roots.
pub fn apply_use_lib_deltas(
    include_paths: &mut Vec<String>,
    deltas: &[UseLibDelta],
    workspace_root: &Path,
    file_dir: Option<&Path>,
) {
    for delta in deltas {
        let Some(resolved) = resolve_delta_path(delta, workspace_root, file_dir) else {
            continue;
        };

        match delta.kind {
            UseLibDeltaKind::Add => {
                if !include_paths.contains(&resolved) {
                    include_paths.insert(0, resolved);
                }
            }
            UseLibDeltaKind::Remove => {
                include_paths.retain(|p| p != &resolved);
            }
        }
    }
}

/// Resolve include paths from `use lib` statements (legacy helper).
pub fn resolve_use_lib_paths(
    use_lib_paths: &[UseLibPath],
    workspace_root: &Path,
    file_dir: Option<&Path>,
) -> Vec<String> {
    let mut result = Vec::new();
    for path in use_lib_paths {
        let Some(resolved) = resolve_delta_path(path, workspace_root, file_dir) else {
            continue;
        };
        if !result.contains(&resolved) {
            result.push(resolved);
        }
    }
    result
}

fn resolve_delta_path(
    delta: &UseLibDelta,
    workspace_root: &Path,
    file_dir: Option<&Path>,
) -> Option<String> {
    if delta.from_findbin {
        let base = file_dir.unwrap_or(workspace_root);
        let resolved = base.join(&delta.path);
        // Keep FindBin-derived paths workspace-scoped for safety.
        if resolved.starts_with(workspace_root) {
            return Some(normalize_path_string(resolved));
        }
        return None;
    }

    let p = Path::new(&delta.path);
    if p.is_absolute() {
        Some(normalize_path_string(PathBuf::from(p)))
    } else {
        Some(delta.path.clone())
    }
}

fn strip_lib_prefix<'a>(trimmed: &'a str, keyword: &str) -> Option<&'a str> {
    let rest = trimmed.strip_prefix(keyword)?;
    if !rest.starts_with(|c: char| c.is_whitespace()) {
        return None;
    }
    let rest = rest.trim_start();
    let rest = rest.strip_prefix("lib")?;
    if !rest.starts_with(|c: char| c.is_whitespace() || c == '(' || c == ';') {
        return None;
    }
    Some(rest.trim_start())
}

fn extract_paths_from_args(args: &str, kind: UseLibDeltaKind, out: &mut Vec<UseLibDelta>) {
    let args = args.trim_end_matches(';').trim();

    if let Some(rest) = args.strip_prefix("qw") {
        extract_qw_paths(rest.trim_start(), kind, out);
        return;
    }

    if let Some(inner) = strip_parens(args) {
        extract_quoted_list(inner, kind, out);
        return;
    }

    extract_quoted_list(args, kind, out);
}

fn extract_qw_paths(rest: &str, kind: UseLibDeltaKind, out: &mut Vec<UseLibDelta>) {
    let (open, close) = match rest.chars().next() {
        Some('(') => ('(', ')'),
        Some('/') => ('/', '/'),
        Some('{') => ('{', '}'),
        Some('[') => ('[', ']'),
        Some('<') => ('<', '>'),
        Some('!') => ('!', '!'),
        _ => return,
    };

    let inner = &rest[open.len_utf8()..];
    let end = inner.find(close).unwrap_or(inner.len());
    let content = &inner[..end];

    for word in content.split_whitespace() {
        out.push(UseLibDelta { kind, path: word.to_string(), from_findbin: false });
    }
}

fn strip_parens(s: &str) -> Option<&str> {
    let s = s.trim();
    let inner = s.strip_prefix('(')?;
    let inner = inner.trim_end().strip_suffix(')')?;
    Some(inner)
}

fn extract_quoted_list(s: &str, kind: UseLibDeltaKind, out: &mut Vec<UseLibDelta>) {
    let mut remaining = s.trim();

    while !remaining.is_empty() {
        remaining = remaining.trim_start_matches(|c: char| c == ',' || c.is_whitespace());
        if remaining.is_empty() {
            break;
        }

        if let Some((path, from_findbin, rest)) = extract_one_quoted(remaining) {
            out.push(UseLibDelta { kind, path, from_findbin });
            remaining = rest.trim_start_matches(|c: char| c == ',' || c.is_whitespace());
        } else {
            break;
        }
    }
}

fn extract_one_quoted(s: &str) -> Option<(String, bool, &str)> {
    let s = s.trim();
    let quote = match s.chars().next()? {
        '\'' => '\'',
        '"' => '"',
        _ => return None,
    };

    let inner = &s[1..];
    let end = inner.find(quote)?;
    let content = &inner[..end];
    let rest = &inner[end + 1..];

    let (path, from_findbin) = resolve_findbin_in_string(content);
    Some((path, from_findbin, rest))
}

fn resolve_findbin_in_string(s: &str) -> (String, bool) {
    let findbin_vars =
        ["$FindBin::Bin", "$FindBin::RealBin", "${FindBin::Bin}", "${FindBin::RealBin}"];

    for var in &findbin_vars {
        if let Some(rest) = s.strip_prefix(var) {
            let path = rest.strip_prefix('/').unwrap_or(rest);
            if path.is_empty() {
                return (".".to_string(), true);
            }
            return (path.to_string(), true);
        }
    }

    (s.to_string(), false)
}

fn normalize_path_string(path: PathBuf) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_quoted_lib() {
        let paths = extract_use_lib_paths("use lib 'lib';");
        assert_eq!(
            paths,
            vec![UseLibDelta {
                kind: UseLibDeltaKind::Add,
                path: "lib".into(),
                from_findbin: false
            }]
        );
    }

    #[test]
    fn no_lib_extracts_remove_delta() {
        let paths = extract_use_lib_deltas("no lib 'lib';");
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].kind, UseLibDeltaKind::Remove);
        assert_eq!(paths[0].path, "lib");
    }

    #[test]
    fn findbin_bin_with_parent_lib() {
        let paths = extract_use_lib_paths("use lib \"$FindBin::Bin/../lib\";");
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "../lib");
        assert!(paths[0].from_findbin);
    }

    #[test]
    fn apply_use_lib_add_and_remove() {
        let mut include_paths = vec!["lib".to_string(), "vendor/lib".to_string()];
        let deltas = extract_use_lib_deltas("use lib 'tmp/lib';\nno lib 'vendor/lib';");
        apply_use_lib_deltas(&mut include_paths, &deltas, Path::new("/project"), None);

        assert_eq!(include_paths[0], "tmp/lib");
        assert!(!include_paths.contains(&"vendor/lib".to_string()));
    }

    #[test]
    fn resolve_findbin_path_with_file_dir() {
        let paths = vec![UseLibDelta {
            kind: UseLibDeltaKind::Add,
            path: "lib".into(),
            from_findbin: true,
        }];
        let resolved =
            resolve_use_lib_paths(&paths, Path::new("/project"), Some(Path::new("/project/bin")));
        assert_eq!(resolved, vec!["/project/bin/lib"]);
    }

    #[test]
    fn resolve_absolute_path_outside_workspace_is_kept() -> Result<(), Box<dyn std::error::Error>> {
        let workspace = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        let paths = vec![UseLibDelta {
            kind: UseLibDeltaKind::Add,
            path: outside.path().join("lib").to_string_lossy().to_string(),
            from_findbin: false,
        }];
        let resolved = resolve_use_lib_paths(&paths, workspace.path(), None);
        assert_eq!(resolved, vec![outside.path().join("lib").to_string_lossy().to_string()]);
        Ok(())
    }
}
