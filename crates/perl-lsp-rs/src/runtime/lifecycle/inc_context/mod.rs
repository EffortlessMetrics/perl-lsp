//! Shared effective `@INC` context assembly.
//!
//! This module centralizes the ordered include-root view used by runtime
//! module-resolution consumers. It preserves source labels so diagnostics and
//! completion can later consume the same root set without rebuilding it.

use super::super::*;
use perl_lsp_rs_core::providers::missing_module::ModuleSearchPathDisplay;
use perl_module::resolution::{IncRoot, build_effective_inc_roots};
use std::path::PathBuf;

mod assembly;
mod display;

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

impl EffectiveIncContext {
    /// Build labeled search paths suitable for PL701 display.
    ///
    /// This is intentionally lazy so completion can consume the same
    /// `EffectiveIncContext` without allocating diagnostic display strings on
    /// every keystroke.
    #[must_use]
    #[allow(dead_code)]
    pub(crate) fn search_display_paths(&self) -> Vec<ModuleSearchPathDisplay> {
        display::search_display_paths(&self.effective_roots)
    }

    /// Returns `true` if `symbol_uri` is directly reachable through one of the
    /// effective include roots in this context as a Perl module file.
    ///
    /// Used to filter workspace-symbol index hits against position-aware `@INC`
    /// state so that `no lib` cancellations are honoured for goto-definition and
    /// module completion (fixes #8537).
    ///
    /// # Module file convention
    ///
    /// A file at `<root>/Foo/Bar.pm` is reachable via @INC root `<root>` as
    /// module `Foo::Bar`. We verify that `symbol_uri` is a *direct child path*
    /// of one of the effective roots — i.e., the root is the *immediate parent
    /// prefix* that maps to a module name, not just any ancestor.
    ///
    /// # Non-file URIs
    ///
    /// Non-file-scheme URIs (e.g. `untitled:` or built-in virtual documents) are
    /// given the benefit of the doubt and are **not** filtered out.
    ///
    /// # Relative roots
    ///
    /// Roots that are relative (e.g. `FileLocalLexical` entries like `lib`) are
    /// resolved against `self.root` before the prefix check is applied.
    #[must_use]
    pub(crate) fn symbol_uri_reachable(&self, symbol_uri: &str) -> bool {
        let Some(symbol_path) = super::super::source_path_from_uri(symbol_uri) else {
            // Non-file URI — don't filter.
            return true;
        };

        // Normalise the symbol path to an absolute form for comparison.
        let symbol_abs =
            if symbol_path.is_absolute() { symbol_path } else { self.root.join(&symbol_path) };

        // Only count roots that are non-trivial (not equal to the workspace root
        // itself) for the file-as-module reachability check. The workspace root `.`
        // covers every file in the workspace, which would defeat the filter.
        // We rely on explicit include roots (use lib, includePaths, PERL5LIB) to
        // determine reachability.
        let root_is_workspace = |root_abs: &std::path::Path| root_abs == self.root.as_path();

        self.effective_roots.iter().any(|root| {
            let root_abs = if root.path.is_absolute() {
                root.path.clone()
            } else {
                self.root.join(&root.path)
            };
            // Skip the workspace root itself — it's a fallback include root that
            // covers the entire tree, not a specific module directory.
            if root_is_workspace(&root_abs) {
                return false;
            }
            symbol_abs.starts_with(&root_abs)
        })
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
        let lexical_paths = assembly::lexical_paths(doc_uri, doc_text, doc_offset, root.as_path());

        // When a position offset is provided, also compute the set of paths that
        // `no lib` has explicitly cancelled at that position. These cancellations
        // apply to configured include paths too — `no lib 'lib'` removes `lib` from
        // `@INC` regardless of whether it arrived via `use lib` or workspace config.
        let include_paths = assembly::include_paths_with_cancellations(
            doc_uri,
            doc_text,
            doc_offset,
            root.as_path(),
            raw_include_paths,
        );

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
        config.use_perl5lib = false;
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
        assert!(!context.use_perl5lib);
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

    #[test]
    fn symbol_uri_reachable_returns_true_for_symbol_under_inc_root() -> TestResult {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");
        let lib_dir = workspace.join("lib");
        std::fs::create_dir_all(&lib_dir)?;

        // Simulate a workspace symbol at lib/MyModule.pm.
        let module_path = lib_dir.join("MyModule.pm");
        std::fs::write(&module_path, "package MyModule;\n1;\n")?;
        let module_uri = file_uri(&module_path)?;

        // Build a context with lib as an include root.
        let workspace_uri = file_uri(&workspace)?;
        let script_path = workspace.join("script.pl");
        let script_uri = file_uri(&script_path)?;
        let mut config = perl_lsp_rs_core::config::WorkspaceConfig::default();
        config.include_paths = vec!["lib".to_string()];
        config.use_system_inc = false;

        let server = LspServer::new();
        *server.workspace_folders.lock() = vec![
            WorkspaceFolderState::new(workspace_uri.clone())
                .with_path(workspace.clone())
                .with_effective_workspace_config(config),
        ];
        *server.root_path.lock() = Some(workspace.clone());

        let source = "use MyModule;\n";
        let context = server
            .effective_inc_context_for_doc(Some(&script_uri), Some(source), Some(source.len()))
            .ok_or("expected effective @INC context")?;

        assert!(
            context.symbol_uri_reachable(&module_uri),
            "symbol under an include root must be reachable; root={:?} symbol={:?}",
            context.effective_roots,
            module_uri
        );
        Ok(())
    }

    #[test]
    fn symbol_uri_reachable_returns_false_after_no_lib_cancellation() -> TestResult {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");
        let lib_dir = workspace.join("lib");
        std::fs::create_dir_all(&lib_dir)?;

        let module_path = lib_dir.join("GoneModule.pm");
        std::fs::write(&module_path, "package GoneModule;\n1;\n")?;
        let module_uri = file_uri(&module_path)?;

        let workspace_uri = file_uri(&workspace)?;
        let script_path = workspace.join("script.pl");
        let script_uri = file_uri(&script_path)?;
        let config = perl_lsp_rs_core::config::WorkspaceConfig::default();

        let server = LspServer::new();
        *server.workspace_folders.lock() = vec![
            WorkspaceFolderState::new(workspace_uri.clone())
                .with_path(workspace.clone())
                .with_effective_workspace_config(config),
        ];
        *server.root_path.lock() = Some(workspace.clone());

        // `use lib 'lib'` then `no lib 'lib'` cancels the path at this offset.
        let source = "use lib 'lib';\nno lib 'lib';\nuse GoneModule;\n";
        let use_gone_offset = source.rfind("use GoneModule").ok_or("offset not found")?;
        let context = server
            .effective_inc_context_for_doc(Some(&script_uri), Some(source), Some(use_gone_offset))
            .ok_or("expected effective @INC context")?;

        assert!(
            !context.symbol_uri_reachable(&module_uri),
            "symbol under a no-lib-cancelled root must NOT be reachable; \
             roots={:?} symbol={:?}",
            context.effective_roots,
            module_uri
        );
        Ok(())
    }

    #[test]
    fn symbol_uri_reachable_returns_true_for_non_file_uri() -> TestResult {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");
        std::fs::create_dir_all(&workspace)?;

        let workspace_uri = file_uri(&workspace)?;
        let config = perl_lsp_rs_core::config::WorkspaceConfig::default();
        let server = LspServer::new();
        *server.workspace_folders.lock() = vec![
            WorkspaceFolderState::new(workspace_uri.clone())
                .with_path(workspace.clone())
                .with_effective_workspace_config(config),
        ];
        *server.root_path.lock() = Some(workspace.clone());

        let source = "use Foo;\n";
        let context = server
            .effective_inc_context_for_doc(Some(&workspace_uri), Some(source), Some(source.len()))
            .ok_or("expected effective @INC context")?;

        // Non-file URI should always be considered reachable (benefit of the doubt).
        assert!(context.symbol_uri_reachable("untitled:foo"), "non-file URI must not be filtered");
        Ok(())
    }
}
