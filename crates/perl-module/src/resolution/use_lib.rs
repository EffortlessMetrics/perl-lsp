//! Extract include paths from `use lib` and `FindBin` statements.
//!
//! Scans Perl source text for `use lib` pragmas and recognizes common
//! `FindBin` patterns to discover additional module include directories.

use std::path::{Component, Path, PathBuf};

/// A discovered include path from a `use lib` statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseLibPath {
    /// The resolved directory path (relative or absolute).
    pub path: String,
    /// Whether this path was derived from a `FindBin` variable.
    pub from_findbin: bool,
}

/// A `use lib` / `no lib` operation extracted from source in lexical order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UseLibAction {
    /// Add paths to the effective include stack.
    Add(Vec<UseLibPath>),
    /// Remove paths from the effective include stack.
    Remove(Vec<UseLibPath>),
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
/// - `use lib '$Bin/path'` and `"$RealBin/path"` (from `FindBin` exports)
///
/// Returns extracted paths in order of appearance.
pub fn extract_use_lib_paths(source: &str) -> Vec<UseLibPath> {
    let mut paths = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = strip_use_lib_prefix(trimmed) {
            extract_paths_from_args(rest, &mut paths);
        }
    }

    paths
}

/// Extract ordered `use lib` and `no lib` operations from source text.
#[must_use]
pub fn extract_use_lib_operations(source: &str) -> Vec<UseLibAction> {
    let mut ops = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = strip_use_lib_prefix(trimmed) {
            let mut paths = Vec::new();
            extract_paths_from_args(rest, &mut paths);
            if !paths.is_empty() {
                ops.push(UseLibAction::Add(paths));
            }
            continue;
        }

        if let Some(rest) = strip_no_lib_prefix(trimmed) {
            let mut paths = Vec::new();
            extract_paths_from_args(rest, &mut paths);
            if !paths.is_empty() {
                ops.push(UseLibAction::Remove(paths));
            }
        }
    }

    ops
}

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

/// Resolve effective include paths from lexical `use lib` / `no lib` operations.
#[must_use]
pub fn resolve_use_lib_paths_from_source(
    source: &str,
    workspace_root: &Path,
    file_dir: Option<&Path>,
) -> Vec<String> {
    resolve_use_lib_paths_from_source_at_offset(source, source.len(), workspace_root, file_dir)
}

/// Resolve effective include paths from lexical `use lib` / `no lib` operations,
/// considering only source text up to the provided byte offset.
#[must_use]
pub fn resolve_use_lib_paths_from_source_at_offset(
    source: &str,
    offset: usize,
    workspace_root: &Path,
    file_dir: Option<&Path>,
) -> Vec<String> {
    let mut resolved = Vec::new();
    let source_prefix = source.get(..offset).unwrap_or(source);
    for op in extract_use_lib_operations(source_prefix) {
        match op {
            UseLibAction::Add(paths) => {
                let added = resolve_use_lib_paths(&paths, workspace_root, file_dir);
                for path in added.into_iter().rev() {
                    resolved.retain(|existing| existing != &path);
                    resolved.insert(0, path);
                }
            }
            UseLibAction::Remove(paths) => {
                for path in resolve_use_lib_paths(&paths, workspace_root, file_dir) {
                    resolved.retain(|existing| existing != &path);
                }
            }
        }
    }
    resolved
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
    // Fully-qualified FindBin variables — no word-boundary ambiguity because `::` terminates
    // the name and braced forms are unambiguous.
    let qualified_vars =
        ["$FindBin::Bin", "$FindBin::RealBin", "${FindBin::Bin}", "${FindBin::RealBin}"];

    for var in &qualified_vars {
        if let Some(rest) = s.strip_prefix(var) {
            let path = rest.strip_prefix('/').unwrap_or(rest);
            if path.is_empty() {
                return (".".to_string(), true);
            }
            return (path.to_string(), true);
        }
    }

    // Short exported forms: `$Bin`, `$RealBin`, `${Bin}`, `${RealBin}`.
    // Braced forms (`${Bin}`) are always unambiguous.  Bare forms (`$Bin`,
    // `$RealBin`) must be followed by `/`, end-of-string, or a non-identifier
    // character to avoid false-positives on variables like `$BinDir` or
    // `$RealBinPath`.
    let bare_short = ["$Bin", "$RealBin"];
    let braced_short = ["${Bin}", "${RealBin}"];

    for var in &bare_short {
        if let Some(rest) = s.strip_prefix(var) {
            // Word-boundary check: the character after the variable name must
            // not be a Perl identifier character (letter, digit, or `_`).
            // This prevents `$BinDir` or `$RealBinPath` from matching `$Bin`/`$RealBin`.
            let next = rest.chars().next();
            if next.is_none() || next.is_some_and(|c| !c.is_alphanumeric() && c != '_') {
                let path = rest.strip_prefix('/').unwrap_or(rest);
                if path.is_empty() {
                    return (".".to_string(), true);
                }
                return (path.to_string(), true);
            }
        }
    }

    for var in &braced_short {
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
