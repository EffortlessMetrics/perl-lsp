//! Shared effective `@INC` context assembly.
//!
//! This module centralizes the ordered include-root view used by runtime
//! module-resolution consumers. It preserves source labels so diagnostics and
//! completion can later consume the same root set without rebuilding it.

use super::super::*;
use perl_lsp_rs_core::providers::missing_module::ModuleSearchPathDisplay;
use perl_module::resolution::use_lib::{
    no_lib_cancelled_paths_at_offset, resolve_use_lib_paths_from_source,
    resolve_use_lib_paths_from_source_at_offset,
};
use perl_module::resolution::{IncRoot, build_effective_inc_roots};
use std::path::PathBuf;

/// Effective include roots for a single document/resolution context.
// Staged fields are consumed by the next completion and PL701 migrations; this
// first slice wires only resolver use of the shared context.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct EffectiveIncContext {
    /// Workspace root used for relative include paths.
    pub(crate) root: PathBuf,
    /// Owning workspace folder URI, when the document maps to one.
    pub(crate) folder_uri: Option<String>,
    /// Document URI used to build this context.
    pub(crate) doc_uri: Option<String>,
    /// Ordered, labeled include roots used for module resolution.
    pub(crate) effective_roots: Vec<IncRoot>,
    /// Whether interpreter startup `@INC` participated.
    pub(crate) use_system_inc: bool,
    /// Whether `PERL5LIB` was eligible to participate.
    pub(crate) use_perl5lib: bool,
    /// Module-resolution timeout from the owning workspace config.
    pub(crate) resolution_timeout_ms: u64,
}

fn root_source_label(source: &str) -> &'static str {
    match source {
        "use-lib-lexical" => "use lib",
        "workspace-include-paths" => "workspace includePaths",
        "perl5lib-env" => "PERL5LIB",
        "interpreter-startup-inc" => "interpreter startup @INC",
        _ => "unknown @INC source",
    }
}

fn search_display_paths(roots: &[IncRoot]) -> Vec<ModuleSearchPathDisplay> {
    roots
        .iter()
        .map(|root| {
            ModuleSearchPathDisplay::new(
                root.path.to_string_lossy().into_owned(),
                root_source_label(&root.source),
            )
        })
        .collect()
}

impl EffectiveIncContext {
    /// Build labeled search paths suitable for PL701 display.
    ///
    /// This is intentionally lazy so completion can consume the same
    /// `EffectiveIncContext` without allocating diagnostic display strings on
    /// every keystroke.
    #[must_use]
    #[allow(dead_code)]
    pub(crate) fn search_display_paths(&self) -> Vec<ModuleSearchPathDisplay> {
        search_display_paths(&self.effective_roots)
    }
}

impl LspServer {
    /// Build the shared, labeled include-root context for a document.
    ///
    /// This is the central runtime path for assembling configured include
    /// roots, `PERL5LIB`, lexical `use lib`, and opt-in interpreter startup
    /// `@INC`. It does not mutate configured include paths.
    #[must_use]
    pub(crate) fn effective_inc_context_for_doc(
        &self,
        doc_uri: Option<&str>,
        doc_text: Option<&str>,
        doc_offset: Option<usize>,
    ) -> Option<EffectiveIncContext> {
        let (root, folder_uri, config) = {
            let folders = self.workspace_folders.lock();
            let best_folder =
                doc_uri.and_then(|uri| super::super::best_workspace_folder_for_doc(&folders, uri));
            if let Some(folder) = best_folder {
                let root = super::super::workspace_folder_path(folder)
                    .or_else(|| self.root_path.lock().clone())?;
                (root, Some(folder.uri.clone()), folder.effective_workspace_config.clone())
            } else {
                let fallback_root = folders
                    .first()
                    .and_then(super::super::workspace_folder_path)
                    .or_else(|| self.root_path.lock().clone())?;
                (fallback_root, None, self.workspace_config.lock().clone())
            }
        };

        let perl5lib_paths = std::env::var("PERL5LIB")
            .map(|value| perl_lsp_rs_core::config::WorkspaceConfig::parse_perl5lib(&value))
            .unwrap_or_default();
        let raw_include_paths = config.effective_include_paths(&perl5lib_paths);
        let mut lexical_paths = Vec::new();

        if let Some(text) = doc_text {
            let file_dir = doc_uri
                .and_then(super::super::source_path_from_uri)
                .and_then(|path| path.parent().map(|dir| dir.to_path_buf()));
            if file_dir.is_none() && doc_uri.is_some() {
                tracing::trace!("Effective @INC context failed to resolve doc_uri: {:?}", doc_uri);
            }
            lexical_paths = if let Some(offset) = doc_offset {
                resolve_use_lib_paths_from_source_at_offset(
                    text,
                    offset,
                    &root,
                    file_dir.as_deref(),
                )
            } else {
                resolve_use_lib_paths_from_source(text, &root, file_dir.as_deref())
            };
        }

        // When a position offset is provided, also compute the set of paths that
        // `no lib` has explicitly cancelled at that position. These cancellations
        // apply to configured include paths too — `no lib 'lib'` removes `lib` from
        // `@INC` regardless of whether it arrived via `use lib` or workspace config.
        let include_paths: Vec<String> = if let (Some(offset), Some(text)) = (doc_offset, doc_text)
        {
            let file_dir = doc_uri
                .and_then(super::super::source_path_from_uri)
                .and_then(|path| path.parent().map(|dir| dir.to_path_buf()));
            let cancelled =
                no_lib_cancelled_paths_at_offset(text, offset, &root, file_dir.as_deref());
            if cancelled.is_empty() {
                raw_include_paths
            } else {
                raw_include_paths.into_iter().filter(|p| !cancelled.contains(p)).collect()
            }
        } else {
            raw_include_paths
        };

        let system_paths = if config.use_system_inc {
            self.system_inc_for_context(folder_uri.as_deref())
        } else {
            Vec::new()
        };
        let effective_roots = build_effective_inc_roots(
            &include_paths,
            &perl5lib_paths,
            config.use_perl5lib,
            &lexical_paths,
            &system_paths,
        );

        Some(EffectiveIncContext {
            root,
            folder_uri,
            doc_uri: doc_uri.map(ToOwned::to_owned),
            effective_roots,
            use_system_inc: config.use_system_inc,
            use_perl5lib: config.use_perl5lib,
            resolution_timeout_ms: config.resolution_timeout_ms,
        })
    }

    fn system_inc_for_context(&self, folder_uri: Option<&str>) -> Vec<PathBuf> {
        if let Some(folder_uri) = folder_uri {
            let mut folders = self.workspace_folders.lock();
            if let Some(folder) = folders.iter_mut().find(|folder| folder.uri == folder_uri) {
                return folder.effective_workspace_config.get_system_inc().to_vec();
            }
        }

        self.workspace_config.lock().get_system_inc().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::workspace_folder::WorkspaceFolderState;
    use perl_module::resolution::IncRootKind;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn file_uri(path: &std::path::Path) -> Result<String, String> {
        url::Url::from_file_path(path)
            .map(|url| url.to_string())
            .map_err(|()| format!("failed to create URI for {}", path.display()))
    }

    #[test]
    fn effective_inc_context_labels_lexical_and_workspace_roots() -> TestResult {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");
        let script = workspace.join("script").join("run.pl");
        std::fs::create_dir_all(script.parent().ok_or("missing script parent")?)?;

        let workspace_uri = file_uri(&workspace)?;
        let doc_uri = file_uri(&script)?;
        let mut config = perl_lsp_rs_core::config::WorkspaceConfig::default();
        config.include_paths = vec!["lib".to_string()];
        config.use_system_inc = false;
        config.resolution_timeout_ms = 123;

        let server = LspServer::new();
        *server.workspace_folders.lock() = vec![
            WorkspaceFolderState::new(workspace_uri.clone())
                .with_path(workspace.clone())
                .with_effective_workspace_config(config),
        ];
        *server.root_path.lock() = Some(workspace.clone());

        let source = "use lib 't/lib';\nuse Demo::Worker;\n";
        let context = server
            .effective_inc_context_for_doc(Some(&doc_uri), Some(source), Some(source.len()))
            .ok_or("expected effective @INC context")?;

        assert_eq!(context.root, workspace);
        assert_eq!(context.folder_uri.as_deref(), Some(workspace_uri.as_str()));
        assert_eq!(context.doc_uri.as_deref(), Some(doc_uri.as_str()));
        assert!(!context.use_system_inc);
        assert!(context.use_perl5lib);
        assert_eq!(context.resolution_timeout_ms, 123);
        assert_eq!(context.effective_roots.len(), 2);
        assert_eq!(context.effective_roots[0].kind, IncRootKind::FileLocalLexical);
        assert_eq!(context.effective_roots[1].kind, IncRootKind::WorkspaceRelative);
        let search_display_paths = context.search_display_paths();
        assert_eq!(search_display_paths[0].source, "use lib");
        assert_eq!(search_display_paths[1].source, "workspace includePaths");
        Ok(())
    }

    #[test]
    fn effective_inc_context_returns_none_without_root() {
        let server = LspServer::new();
        assert!(server.effective_inc_context_for_doc(None, None, None).is_none());
    }
}
