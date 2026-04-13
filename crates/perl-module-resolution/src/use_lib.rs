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
///
/// Returns extracted paths in order of appearance.
///
/// # Examples
///
/// ```
/// use perl_module_resolution::use_lib::extract_use_lib_paths;
///
/// let paths = extract_use_lib_paths("use lib 'lib';");
/// assert_eq!(paths.len(), 1);
/// assert_eq!(paths[0].path, "lib");
/// ```
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
/// - Absolute paths are returned as-is (if they exist).
/// - `$FindBin::Bin`-relative paths are resolved against `file_dir` (or `workspace_root` if absent).
/// - Other relative paths are resolved against `workspace_root`.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use perl_module_resolution::use_lib::{extract_use_lib_paths, resolve_use_lib_paths};
///
/// let source = "use lib 'lib';";
/// let extracted = extract_use_lib_paths(source);
/// let resolved = resolve_use_lib_paths(&extracted, Path::new("/project"), None);
/// assert_eq!(resolved, vec![String::from("lib")]);
/// ```
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
            let resolved = base.join(path_str);
            if resolved.strip_prefix(workspace_root).is_err() {
                continue;
            }
            if let Some(s) = path_to_relative_string(&resolved, workspace_root) {
                if !result.contains(&s) {
                    result.push(s);
                }
            }
        } else {
            let p = Path::new(path_str);
            if p.is_absolute() {
                let s = normalize_relative_path_string(path_str);
                if !result.contains(&s) {
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
    // Front of this vector is highest-precedence (searched first), matching Perl's
    // `use lib` behavior where later statements prepend to `@INC`.
    let mut resolved = Vec::new();
    let source_prefix = source.get(..offset).unwrap_or(source);
    for op in extract_use_lib_operations(source_prefix) {
        match op {
            UseLibAction::Add(paths) => {
                let added = resolve_use_lib_paths(&paths, workspace_root, file_dir);
                for path in added.into_iter().rev() {
                    // Re-adding an existing path must move it back to highest precedence.
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

fn path_to_relative_string(path: &Path, workspace_root: &Path) -> Option<String> {
    if let Ok(rel) = path.strip_prefix(workspace_root) {
        let s = normalize_relative_path_string(rel.to_string_lossy().as_ref());
        if s.is_empty() { Some(".".to_string()) } else { Some(s) }
    } else {
        let s = normalize_relative_path_string(path.to_string_lossy().as_ref());
        Some(s)
    }
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
    fn single_quoted_with_subdir() {
        let paths = extract_use_lib_paths("use lib 'local/lib/perl5';");
        assert_eq!(paths, vec![UseLibPath { path: "local/lib/perl5".into(), from_findbin: false }]);
    }

    #[test]
    fn qw_parens_multiple_paths() {
        let paths = extract_use_lib_paths("use lib qw(lib t/lib);");
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].path, "lib");
        assert_eq!(paths[1].path, "t/lib");
    }

    #[test]
    fn qw_slash_delimiter() {
        let paths = extract_use_lib_paths("use lib qw/lib t-lib/;");
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].path, "lib");
        assert_eq!(paths[1].path, "t-lib");
    }

    #[test]
    fn qw_curly_delimiter() {
        let paths = extract_use_lib_paths("use lib qw{lib};");
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "lib");
    }

    #[test]
    fn qw_bracket_delimiter() {
        let paths = extract_use_lib_paths("use lib qw[lib t/lib];");
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].path, "lib");
        assert_eq!(paths[1].path, "t/lib");
    }

    #[test]
    fn paren_list_single() {
        let paths = extract_use_lib_paths("use lib ('lib');");
        assert_eq!(paths, vec![UseLibPath { path: "lib".into(), from_findbin: false }]);
    }

    #[test]
    fn paren_list_multiple() {
        let paths = extract_use_lib_paths("use lib ('lib', 't/lib');");
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].path, "lib");
        assert_eq!(paths[1].path, "t/lib");
    }

    #[test]
    fn findbin_bin_with_lib() {
        let paths = extract_use_lib_paths("use lib \"$FindBin::Bin/lib\";");
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "lib");
        assert!(paths[0].from_findbin);
    }

    #[test]
    fn findbin_bin_with_parent_lib() {
        let paths = extract_use_lib_paths("use lib \"$FindBin::Bin/../lib\";");
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "../lib");
        assert!(paths[0].from_findbin);
    }

    #[test]
    fn findbin_realbin() {
        let paths = extract_use_lib_paths("use lib \"$FindBin::RealBin/lib\";");
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "lib");
        assert!(paths[0].from_findbin);
    }

    #[test]
    fn findbin_braced_form() {
        let paths = extract_use_lib_paths("use lib \"${FindBin::Bin}/lib\";");
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "lib");
        assert!(paths[0].from_findbin);
    }

    #[test]
    fn findbin_bare_bin() {
        let paths = extract_use_lib_paths("use lib \"$FindBin::Bin\";");
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, ".");
        assert!(paths[0].from_findbin);
    }

    #[test]
    fn leading_whitespace() {
        let paths = extract_use_lib_paths("  use lib 'lib';");
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "lib");
    }

    #[test]
    fn multiple_use_lib_statements() {
        let source = "use lib 'lib';\nuse lib 't/lib';\n";
        let paths = extract_use_lib_paths(source);
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].path, "lib");
        assert_eq!(paths[1].path, "t/lib");
    }

    #[test]
    fn non_use_lib_lines_ignored() {
        let source = "use strict;\nuse warnings;\nuse lib 'lib';\nuse Foo::Bar;\n";
        let paths = extract_use_lib_paths(source);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].path, "lib");
    }

    #[test]
    fn use_library_not_confused_with_use_lib() {
        let paths = extract_use_lib_paths("use library 'foo';");
        assert!(paths.is_empty());
    }

    #[test]
    fn empty_source() {
        let paths = extract_use_lib_paths("");
        assert!(paths.is_empty());
    }

    #[test]
    fn no_use_lib() {
        let paths = extract_use_lib_paths("use strict;\nuse warnings;\n");
        assert!(paths.is_empty());
    }

    #[test]
    fn resolve_relative_path() {
        let paths = vec![UseLibPath { path: "lib".into(), from_findbin: false }];
        let resolved = resolve_use_lib_paths(&paths, Path::new("/project"), None);
        assert_eq!(resolved, vec!["lib"]);
    }

    #[test]
    fn resolve_findbin_path_with_file_dir() {
        let paths = vec![UseLibPath { path: "lib".into(), from_findbin: true }];
        let resolved =
            resolve_use_lib_paths(&paths, Path::new("/project"), Some(Path::new("/project/bin")));
        assert_eq!(resolved, vec!["bin/lib"]);
    }

    #[test]
    fn resolve_findbin_path_without_file_dir() {
        let paths = vec![UseLibPath { path: "lib".into(), from_findbin: true }];
        let resolved = resolve_use_lib_paths(&paths, Path::new("/project"), None);
        assert_eq!(resolved, vec!["lib"]);
    }

    #[test]
    fn resolve_absolute_path_inside_workspace() -> Result<(), Box<dyn std::error::Error>> {
        let workspace = tempfile::tempdir()?;
        let inside = workspace.path().join("lib");
        let paths =
            vec![UseLibPath { path: inside.to_string_lossy().to_string(), from_findbin: false }];
        let resolved = resolve_use_lib_paths(&paths, workspace.path(), None);
        let expected = inside.to_string_lossy().replace('\\', "/");
        assert_eq!(resolved, vec![expected]);
        Ok(())
    }

    #[test]
    fn resolve_absolute_path_outside_workspace_ignored() -> Result<(), Box<dyn std::error::Error>> {
        let workspace = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        let paths = vec![UseLibPath {
            path: outside.path().join("lib").to_string_lossy().to_string(),
            from_findbin: false,
        }];
        let resolved = resolve_use_lib_paths(&paths, workspace.path(), None);
        let expected = outside.path().join("lib").to_string_lossy().replace('\\', "/");
        assert_eq!(resolved, vec![expected]);
        Ok(())
    }

    #[test]
    fn resolve_deduplicates_paths() {
        let paths = vec![
            UseLibPath { path: "lib".into(), from_findbin: false },
            UseLibPath { path: "lib".into(), from_findbin: false },
        ];
        let resolved = resolve_use_lib_paths(&paths, Path::new("/project"), None);
        assert_eq!(resolved, vec!["lib"]);
    }

    #[test]
    fn extracts_no_lib_operations() {
        let ops = extract_use_lib_operations("use lib 'lib';\nno lib 'lib';\n");
        assert_eq!(ops.len(), 2);
        assert!(matches!(ops.first(), Some(UseLibAction::Add(_))));
        assert!(matches!(ops.get(1), Some(UseLibAction::Remove(_))));
    }

    #[test]
    fn resolves_use_and_no_lib_order() {
        let source = "use lib 'lib';\nuse lib 't/lib';\nno lib 'lib';\n";
        let resolved = resolve_use_lib_paths_from_source(source, Path::new("/project"), None);
        assert_eq!(resolved, vec!["t/lib"]);
    }

    #[test]
    fn resolve_use_lib_paths_respects_offset_before_no_lib() {
        let source = "use lib 'lib';
no lib 'lib';
use Target::Module;
";
        let cutoff = source.find("no lib").unwrap_or(source.len());
        let resolved = resolve_use_lib_paths_from_source_at_offset(
            source,
            cutoff,
            Path::new("/project"),
            None,
        );
        assert_eq!(resolved, vec!["lib"]);
    }

    #[test]
    fn resolve_use_lib_paths_respects_offset_after_no_lib() {
        let source = "use lib 'lib';
no lib 'lib';
use Target::Module;
";
        let cutoff = source.len();
        let resolved = resolve_use_lib_paths_from_source_at_offset(
            source,
            cutoff,
            Path::new("/project"),
            None,
        );
        assert!(resolved.is_empty());
    }

    #[test]
    fn repeated_use_lib_reinserts_with_highest_precedence() {
        let source = "use lib 'a';\nuse lib 'b';\nuse lib 'a';\n";
        let resolved = resolve_use_lib_paths_from_source(source, Path::new("/project"), None);
        assert_eq!(resolved, vec!["a", "b"]);
    }

    #[test]
    fn multiple_use_lib_statements_last_statement_wins() {
        let source = "use lib 'a';\nuse lib 'b';\n";
        let resolved = resolve_use_lib_paths_from_source(source, Path::new("/project"), None);
        assert_eq!(resolved, vec!["b", "a"]);
    }
}
