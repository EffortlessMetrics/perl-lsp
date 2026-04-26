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
        if uri.ends_with(&relative_path) {
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
        | IncRootKind::InterpreterStartup
        | IncRootKind::RuntimeDerived => Some(inc_root.path.join(relative_path)),
    }
}
