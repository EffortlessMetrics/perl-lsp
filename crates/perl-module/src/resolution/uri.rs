//! Deterministic Perl module URI resolution helpers.
//!
//! Extracts the URI-first, timeout-bounded resolution policy.

use crate::path::module_name_to_path;
use perl_parser_core::path_security::validate_workspace_path;
use perl_workspace::folder::workspace_folder_to_path;
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};
use url::Url;

/// Source/category of an effective include root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncRootKind {
    /// File-local lexical include roots (for example `use lib` overlays).
    FileLocalLexical,
    /// Workspace-relative include roots, resolved against each owning workspace.
    WorkspaceRelative,
    /// External absolute include roots.
    ExternalAbsolute,
    /// Paths sourced from the `PERL5LIB` environment variable.
    ///
    /// Treated like `ExternalAbsolute` for resolution (no workspace-boundary
    /// validation) but carries a distinct source label so diagnostics and
    /// tooling can tell environment-supplied roots apart from project-configured ones.
    Perl5LibEnv,
    /// Startup `@INC` entries from the selected Perl interpreter.
    InterpreterStartup,
    /// Runtime-derived include roots (reserved for future trusted runtime mode).
    RuntimeDerived,
}

/// A single ordered include root entry used to resolve modules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncRoot {
    /// Root kind/category.
    pub kind: IncRootKind,
    /// Path value for this root.
    pub path: PathBuf,
    /// Search precedence: lower values are searched first.
    pub precedence: usize,
    /// Human-readable source label.
    pub source: String,
}

/// Build ordered effective include roots from lexical, configured, environment,
/// and interpreter startup sources.
///
/// This centralizes the source-labeling and precedence model used by URI
/// resolution. Callers are still responsible for computing configured include
/// paths and deciding whether `PERL5LIB` / system `@INC` should participate.
#[must_use]
pub fn build_effective_inc_roots(
    include_paths: &[String],
    perl5lib_paths: &[String],
    use_perl5lib: bool,
    lexical_paths: &[String],
    system_paths: &[PathBuf],
) -> Vec<IncRoot> {
    let perl5lib_set: HashSet<String> =
        if use_perl5lib { perl5lib_paths.iter().cloned().collect() } else { HashSet::new() };

    let mut roots = Vec::new();
    let mut seen = HashSet::new();
    let mut precedence = 0usize;

    for path in lexical_paths {
        let path_buf = PathBuf::from(path);
        let kind = if path_buf.is_absolute() {
            IncRootKind::ExternalAbsolute
        } else {
            IncRootKind::FileLocalLexical
        };
        if !seen.insert(normalized_inc_key(&path_buf)) {
            continue;
        }
        roots.push(IncRoot {
            kind,
            path: path_buf,
            precedence,
            source: "use-lib-lexical".to_string(),
        });
        precedence += 1;
    }

    for path in include_paths {
        let path_buf = PathBuf::from(path);
        if !seen.insert(normalized_inc_key(&path_buf)) {
            continue;
        }
        let (kind, source) = if perl5lib_set.contains(path) {
            (IncRootKind::Perl5LibEnv, "perl5lib-env")
        } else if path_buf.is_absolute() {
            (IncRootKind::ExternalAbsolute, "workspace-include-paths")
        } else {
            (IncRootKind::WorkspaceRelative, "workspace-include-paths")
        };
        roots.push(IncRoot { kind, path: path_buf, precedence, source: source.to_string() });
        precedence += 1;
    }

    for path in system_paths {
        if !seen.insert(normalized_inc_key(path)) {
            continue;
        }
        roots.push(IncRoot {
            kind: IncRootKind::InterpreterStartup,
            path: path.clone(),
            precedence,
            source: "interpreter-startup-inc".to_string(),
        });
        precedence += 1;
    }

    roots
}

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
/// 1. Open document URIs (path-boundary match on relative module path)
/// 2. Workspace folders + `include_paths` (path-safe filesystem checks)
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
    let mut effective_inc_roots = Vec::new();
    let mut seen_include_paths = HashSet::new();

    for include_path in include_paths {
        let Some(path) = normalize_inc_path_string(include_path) else {
            continue;
        };
        if !seen_include_paths.insert(path.clone()) {
            continue;
        }

        let kind = if path.is_absolute() {
            IncRootKind::ExternalAbsolute
        } else {
            IncRootKind::WorkspaceRelative
        };
        effective_inc_roots.push(IncRoot {
            kind,
            path,
            precedence: effective_inc_roots.len(),
            source: "includePaths".to_string(),
        });
    }

    if use_system_inc {
        let mut seen_system_paths = HashSet::new();

        for path in system_inc {
            let Some(path) = normalize_system_inc_path(path) else {
                continue;
            };
            if !seen_system_paths.insert(path.clone()) {
                continue;
            }

            effective_inc_roots.push(IncRoot {
                kind: IncRootKind::InterpreterStartup,
                path,
                precedence: effective_inc_roots.len(),
                source: "interpreter-startup-inc".to_string(),
            });
        }
    }

    resolve_module_uri_with_effective_inc(
        module_name,
        open_document_uris,
        workspace_folders,
        &effective_inc_roots,
        timeout,
    )
}

/// Resolve a module name to a `file://` URI using an ordered effective `@INC` model.
#[must_use]
pub fn resolve_module_uri_with_effective_inc(
    module_name: &str,
    open_document_uris: &[String],
    workspace_folders: &[String],
    effective_inc_roots: &[IncRoot],
    timeout: Duration,
) -> ModuleUriResolution {
    let start_time = Instant::now();
    let relative_path = module_name_to_path(module_name);

    for uri in open_document_uris {
        if open_document_uri_matches_relative_path(uri, &relative_path) {
            return ModuleUriResolution::Resolved(uri.clone());
        }
    }

    let mut ordered_roots = effective_inc_roots.to_vec();
    ordered_roots.sort_by_key(|r| r.precedence);

    for workspace_folder in workspace_folders {
        if start_time.elapsed() > timeout {
            return ModuleUriResolution::TimedOut;
        }

        let workspace_path = workspace_folder_to_path(workspace_folder);

        for inc_root in &ordered_roots {
            if !matches!(
                inc_root.kind,
                IncRootKind::FileLocalLexical | IncRootKind::WorkspaceRelative
            ) {
                continue;
            }
            if start_time.elapsed() > timeout {
                return ModuleUriResolution::TimedOut;
            }

            let full_path = full_path_for_root(inc_root, &workspace_path, &relative_path);
            let Some(full_path) = full_path else { continue };

            if full_path.is_file()
                && let Ok(url) = Url::from_file_path(&full_path)
            {
                return ModuleUriResolution::Resolved(url.to_string());
            }
        }
    }

    for inc_root in &ordered_roots {
        if !matches!(
            inc_root.kind,
            IncRootKind::ExternalAbsolute
                | IncRootKind::Perl5LibEnv
                | IncRootKind::InterpreterStartup
                | IncRootKind::RuntimeDerived
        ) {
            continue;
        }
        if start_time.elapsed() > timeout {
            return ModuleUriResolution::TimedOut;
        }

        let full_path = inc_root.path.join(&relative_path);
        if full_path.is_file()
            && let Ok(url) = Url::from_file_path(&full_path)
        {
            return ModuleUriResolution::Resolved(url.to_string());
        }
    }

    ModuleUriResolution::NotFound
}

fn open_document_uri_matches_relative_path(uri: &str, relative_path: &str) -> bool {
    if relative_path.is_empty() {
        return false;
    }

    let normalized_uri = uri.replace('\\', "/");
    let normalized_relative_path = relative_path.replace('\\', "/");
    normalized_uri
        .strip_suffix(&normalized_relative_path)
        .is_some_and(|prefix| prefix.is_empty() || prefix.ends_with('/'))
}

fn normalize_inc_path_string(input: &str) -> Option<PathBuf> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(normalize_path_for_dedupe(Path::new(trimmed)))
}

fn normalize_system_inc_path(input: &Path) -> Option<PathBuf> {
    let trimmed = input.to_string_lossy().trim().to_string();
    if trimmed.is_empty() {
        return None;
    }

    let normalized = normalize_path_for_dedupe(Path::new(&trimmed));
    if normalized == Path::new(".") {
        return None;
    }

    Some(normalized)
}

fn normalize_path_for_dedupe(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        if component == Component::CurDir {
            continue;
        }
        normalized.push(component.as_os_str());
    }

    if normalized.as_os_str().is_empty() { PathBuf::from(".") } else { normalized }
}

fn normalized_inc_key(path: &Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    if normalized == "/" { normalized } else { normalized.trim_end_matches('/').to_string() }
}

fn full_path_for_root(
    inc_root: &IncRoot,
    workspace_path: &Path,
    relative_path: &str,
) -> Option<PathBuf> {
    match inc_root.kind {
        IncRootKind::FileLocalLexical | IncRootKind::WorkspaceRelative => {
            if inc_root.path == Path::new(".") {
                let full_path = workspace_path.join(relative_path);
                validate_workspace_path(&full_path, workspace_path).ok()
            } else if inc_root.path.is_absolute() {
                Some(inc_root.path.join(relative_path))
            } else {
                let full_path = workspace_path.join(&inc_root.path).join(relative_path);
                validate_workspace_path(&full_path, workspace_path).ok()
            }
        }
        IncRootKind::ExternalAbsolute
        | IncRootKind::Perl5LibEnv
        | IncRootKind::InterpreterStartup
        | IncRootKind::RuntimeDerived => Some(inc_root.path.join(relative_path)),
    }
}

#[cfg(test)]
mod tests {
    use super::{IncRootKind, build_effective_inc_roots, open_document_uri_matches_relative_path};
    use std::path::PathBuf;

    #[test]
    fn effective_inc_roots_dedupes_normalized_sources() {
        let include_paths = vec!["lib".to_string(), "lib/".to_string(), "other".to_string()];
        let lexical_paths = vec!["lib\\".to_string()];
        let system_paths = vec![PathBuf::from("other/"), PathBuf::from("syslib")];

        let roots =
            build_effective_inc_roots(&include_paths, &[], false, &lexical_paths, &system_paths);
        let root_paths: Vec<String> =
            roots.iter().map(|root| root.path.to_string_lossy().replace('\\', "/")).collect();

        assert_eq!(root_paths, vec!["lib/".to_string(), "other".to_string(), "syslib".to_string()]);
        assert_eq!(roots[0].source, "use-lib-lexical");
        assert_eq!(roots[1].source, "workspace-include-paths");
        assert_eq!(roots[2].source, "interpreter-startup-inc");
    }

    #[test]
    fn effective_inc_roots_preserves_first_source_precedence() {
        let include_paths = vec!["dup".to_string(), "late".to_string()];
        let lexical_paths = vec!["dup".to_string()];
        let system_paths = vec![PathBuf::from("late"), PathBuf::from("sys")];

        let roots =
            build_effective_inc_roots(&include_paths, &[], false, &lexical_paths, &system_paths);

        assert_eq!(roots.len(), 3);
        assert_eq!(roots[0].path, PathBuf::from("dup"));
        assert_eq!(roots[0].kind, IncRootKind::FileLocalLexical);
        assert_eq!(roots[1].path, PathBuf::from("late"));
        assert_eq!(roots[1].kind, IncRootKind::WorkspaceRelative);
        assert_eq!(roots[2].path, PathBuf::from("sys"));
        assert_eq!(roots[2].kind, IncRootKind::InterpreterStartup);
        assert_eq!(roots[0].precedence, 0);
        assert_eq!(roots[1].precedence, 1);
        assert_eq!(roots[2].precedence, 2);
    }

    #[test]
    fn effective_inc_roots_labels_perl5lib_only_when_enabled() {
        let perl5lib_path = "perl5lib".to_string();
        let include_paths = vec![perl5lib_path.clone(), "lib".to_string()];

        let enabled = build_effective_inc_roots(
            &include_paths,
            std::slice::from_ref(&perl5lib_path),
            true,
            &[],
            &[],
        );
        assert_eq!(enabled[0].kind, IncRootKind::Perl5LibEnv);
        assert_eq!(enabled[0].source, "perl5lib-env");
        assert_eq!(enabled[1].kind, IncRootKind::WorkspaceRelative);

        let disabled = build_effective_inc_roots(&include_paths, &[perl5lib_path], false, &[], &[]);
        assert_eq!(disabled[0].kind, IncRootKind::WorkspaceRelative);
        assert_eq!(disabled[0].source, "workspace-include-paths");
        assert_eq!(disabled[1].kind, IncRootKind::WorkspaceRelative);
    }

    #[test]
    fn open_document_uri_match_rejects_empty_relative_path() {
        assert!(
            !open_document_uri_matches_relative_path("file:///workspace/lib/Foo.pm", ""),
            "empty relative paths must never match an open document"
        );
    }

    #[test]
    fn open_document_uri_match_accepts_exact_relative_path() {
        let cases = [("Foo/Bar.pm", "Foo/Bar.pm", true), ("Other/Bar.pm", "Foo/Bar.pm", false)];

        for (normalized_uri, normalized_relative_path, expected) in cases {
            assert_eq!(
                open_document_uri_matches_relative_path(normalized_uri, normalized_relative_path),
                expected,
                "exact relative path equality should decide raw relative inputs"
            );
        }
    }

    #[test]
    fn open_document_uri_match_accepts_path_bounded_suffix() {
        assert!(
            open_document_uri_matches_relative_path(
                "file:///workspace/local/lib/Foo/Bar.pm",
                "Foo/Bar.pm"
            ),
            "open document URIs may contain editor or workspace prefixes before the module path"
        );
    }

    #[test]
    fn open_document_uri_match_rejects_unbounded_suffix() {
        assert!(
            !open_document_uri_matches_relative_path(
                "file:///workspace/local/lib/MyFoo/Bar.pm",
                "Foo/Bar.pm"
            ),
            "the preceding URI segment must end before the module path starts"
        );
    }

    #[test]
    fn open_document_uri_match_normalizes_windows_separators() {
        assert!(
            open_document_uri_matches_relative_path(
                "file:///workspace\\local\\lib\\Foo\\Bar.pm",
                "Foo\\Bar.pm"
            ),
            "path-boundary matching should not depend on slash direction"
        );
    }
}
