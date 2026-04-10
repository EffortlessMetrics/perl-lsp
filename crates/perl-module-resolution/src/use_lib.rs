//! Extract include paths from `use lib` and `FindBin` statements.
//!
//! Scans Perl source text for `use lib` pragmas and recognizes common
//! `FindBin` patterns to discover additional module include directories.

use std::path::Path;

/// A discovered include path from a `use lib` statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseLibPath {
    /// The resolved directory path (relative or absolute).
    pub path: String,
    /// Whether this path was derived from a `FindBin` variable.
    pub from_findbin: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UseLibAction {
    Add,
    Remove,
}

/// Extract include paths from `use lib` statements in Perl source text.
///
/// Handles the following patterns:
/// - `use lib 'path';`
/// - `use lib "path";`
/// - `use lib qw(path1 path2);`
/// - `use lib qw/path1 path2/;`
/// - `use lib ("path1", "path2");`
/// - `use lib '$FindBin::Bin/path'` and `"$FindBin::Bin/path"`
///
/// Returns extracted paths in order of appearance.
pub fn extract_use_lib_paths(source: &str) -> Vec<UseLibPath> {
    extract_use_lib_actions(source)
        .into_iter()
        .filter_map(|(action, path)| (action == UseLibAction::Add).then_some(path))
        .collect()
}

/// Apply ordered `use lib`/`no lib` statements to a base include path list.
///
/// This function preserves statement order from `source`: each `use lib` prepends
/// paths for subsequent resolution, and each `no lib` removes previously-added
/// or configured matching entries.
#[must_use]
pub fn apply_use_lib_overrides(
    source: &str,
    base_include_paths: &[String],
    workspace_root: &Path,
    file_dir: Option<&Path>,
) -> Vec<String> {
    let mut include_paths = base_include_paths.to_vec();
    for (action, path) in extract_use_lib_actions(source) {
        let resolved = resolve_use_lib_path(&path, workspace_root, file_dir);
        if resolved.is_empty() {
            continue;
        }

        match action {
            UseLibAction::Add => {
                include_paths.retain(|p| p != &resolved);
                include_paths.insert(0, resolved);
            }
            UseLibAction::Remove => include_paths.retain(|p| p != &resolved),
        }
    }
    include_paths
}

/// Resolve `use lib` paths against a workspace root and optional file directory.
///
/// - Absolute paths are returned as-is.
/// - `$FindBin::Bin`-relative paths are resolved against `file_dir` (or `workspace_root` if absent).
/// - Other relative paths are preserved as relative include entries.
pub fn resolve_use_lib_paths(
    use_lib_paths: &[UseLibPath],
    workspace_root: &Path,
    file_dir: Option<&Path>,
) -> Vec<String> {
    let mut result = Vec::new();

    for ulp in use_lib_paths {
        let resolved = resolve_use_lib_path(ulp, workspace_root, file_dir);
        if !resolved.is_empty() && !result.contains(&resolved) {
            result.push(resolved);
        }
    }

    result
}

fn strip_use_lib_prefix(trimmed: &str) -> Option<&str> {
    let rest = trimmed.strip_prefix("use")?;
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

fn strip_no_lib_prefix(trimmed: &str) -> Option<&str> {
    let rest = trimmed.strip_prefix("no")?;
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

fn extract_use_lib_actions(source: &str) -> Vec<(UseLibAction, UseLibPath)> {
    let mut entries = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = strip_use_lib_prefix(trimmed) {
            let mut out = Vec::new();
            extract_paths_from_args(rest, &mut out);
            entries.extend(out.into_iter().map(|p| (UseLibAction::Add, p)));
        } else if let Some(rest) = strip_no_lib_prefix(trimmed) {
            let mut out = Vec::new();
            extract_paths_from_args(rest, &mut out);
            entries.extend(out.into_iter().map(|p| (UseLibAction::Remove, p)));
        }
    }
    entries
}

fn extract_paths_from_args(args: &str, out: &mut Vec<UseLibPath>) {
    let args = args.trim_end_matches(';').trim();

    if let Some(rest) = args.strip_prefix("qw") {
        extract_qw_paths(rest.trim_start(), out);
        return;
    }

    if let Some(inner) = strip_parens(args) {
        extract_quoted_list(inner, out);
        return;
    }

    extract_quoted_list(args, out);
}

fn extract_qw_paths(rest: &str, out: &mut Vec<UseLibPath>) {
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
        out.push(UseLibPath { path: word.to_string(), from_findbin: false });
    }
}

fn strip_parens(s: &str) -> Option<&str> {
    let s = s.trim();
    let inner = s.strip_prefix('(')?;
    let inner = inner.trim_end().strip_suffix(')')?;
    Some(inner)
}

fn extract_quoted_list(s: &str, out: &mut Vec<UseLibPath>) {
    let mut remaining = s.trim();

    while !remaining.is_empty() {
        remaining = remaining.trim_start_matches(|c: char| c == ',' || c.is_whitespace());
        if remaining.is_empty() {
            break;
        }

        if let Some((path, from_findbin, rest)) = extract_one_quoted(remaining) {
            out.push(UseLibPath { path, from_findbin });
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

fn resolve_use_lib_path(
    path: &UseLibPath,
    workspace_root: &Path,
    file_dir: Option<&Path>,
) -> String {
    if path.from_findbin {
        let base = file_dir.unwrap_or(workspace_root);
        return normalize_relative_path_string(base.join(&path.path).to_string_lossy().as_ref());
    }

    normalize_relative_path_string(&path.path)
}

fn normalize_relative_path_string(path: &str) -> String {
    path.replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_quoted_lib() {
        let paths = extract_use_lib_paths("use lib 'lib';");
        assert_eq!(paths, vec![UseLibPath { path: "lib".into(), from_findbin: false }]);
    }

    #[test]
    fn double_quoted_lib() {
        let paths = extract_use_lib_paths("use lib \"lib\";");
        assert_eq!(paths, vec![UseLibPath { path: "lib".into(), from_findbin: false }]);
    }

    #[test]
    fn qw_parens_multiple_paths() {
        let paths = extract_use_lib_paths("use lib qw(lib t/lib);");
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].path, "lib");
        assert_eq!(paths[1].path, "t/lib");
    }

    #[test]
    fn findbin_bin_with_parent_lib() {
        let paths = extract_use_lib_paths("use lib \"$FindBin::Bin/../lib\";");
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "../lib");
        assert!(paths[0].from_findbin);
    }

    #[test]
    fn apply_use_lib_overrides_supports_no_lib() {
        let source = "use lib 'lib';\nno lib 'lib';";
        let resolved =
            apply_use_lib_overrides(source, &["base".to_string()], Path::new("/workspace"), None);
        assert_eq!(resolved, vec!["base"]);
    }

    #[test]
    fn resolve_use_lib_paths_keeps_absolute_paths() {
        let extracted = vec![UseLibPath { path: "/opt/perl/lib".into(), from_findbin: false }];
        let resolved = resolve_use_lib_paths(&extracted, Path::new("/workspace"), None);
        assert_eq!(resolved, vec!["/opt/perl/lib"]);
    }
}
