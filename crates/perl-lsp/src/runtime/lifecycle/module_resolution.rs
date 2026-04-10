//! Module path resolution
//!
//! Handles resolution of Perl module names to file paths.

use super::super::*;
use perl_module_resolution::{
    ModuleUriResolution, resolve_module_path as resolve_workspace_module_path, resolve_module_uri,
    use_lib::resolve_use_lib_paths_from_source,
};
use std::path::PathBuf;
use std::sync::Once;
use std::time::Duration;

/// Fires a `tracing::warn!` the first time workspace root is found to be undetected.
///
/// Both `resolve_module_path` and `resolve_module_path_with_uri` share this sentinel
/// because both sites indicate the same underlying problem: no workspace root was
/// provided by the LSP client (single-file mode with no open folder).
static WARN_ONCE_ROOT_UNDETECTED: Once = Once::new();

/// Prepend `use lib` paths extracted from `doc_text` to `include_paths`.
///
/// The extra paths are scoped to this resolution pass only and are searched
/// ahead of the configured workspace paths.
/// Paths are scoped to this call only — `workspace_config.include_paths` is never mutated.
fn prepend_use_lib_paths(
    include_paths: &mut Vec<String>,
    doc_text: &str,
    workspace_root: &std::path::Path,
    file_dir: Option<&std::path::Path>,
) {
    let dynamic = resolve_use_lib_paths_from_source(doc_text, workspace_root, file_dir);
    for p in dynamic.into_iter().rev() {
        include_paths.retain(|existing| existing != &p);
        include_paths.insert(0, p);
    }
}

fn workspace_root_for_doc(workspace_folders: &[String], doc_uri: Option<&str>) -> Option<PathBuf> {
    let doc_path =
        doc_uri.and_then(|u| url::Url::parse(u).ok()).and_then(|u| u.to_file_path().ok());

    if let Some(doc_path) = doc_path {
        let mut best_match: Option<(PathBuf, usize)> = None;
        for folder in workspace_folders {
            let Some(candidate) = url::Url::parse(folder).ok().and_then(|u| u.to_file_path().ok())
            else {
                continue;
            };
            if doc_path.starts_with(&candidate) {
                let depth = candidate.components().count();
                match &best_match {
                    Some((_, best_depth)) if *best_depth >= depth => {}
                    _ => best_match = Some((candidate, depth)),
                }
            }
        }
        if let Some((best, _)) = best_match {
            return Some(best);
        }
    }

    workspace_folders
        .first()
        .and_then(|u| url::Url::parse(u).ok())
        .and_then(|u| u.to_file_path().ok())
}

impl LspServer {
    /// Enhanced module path resolver using workspace configuration and optional document text.
    ///
    /// When `doc_text` is provided, `use lib` paths extracted from it are prepended to the
    /// include path list for this call only (no global state mutation).
    ///
    /// Use `resolve_module_path_with_uri` when a document URI is available so that
    /// `FindBin`-relative paths are resolved against the document's directory.
    #[allow(dead_code)] // Used by tests and available for callers without a document URI
    pub(crate) fn resolve_module_path(
        &self,
        module: &str,
        doc_text: Option<&str>,
    ) -> Option<PathBuf> {
        let root = match self.root_path.lock().clone() {
            Some(r) => r,
            None => {
                WARN_ONCE_ROOT_UNDETECTED.call_once(|| {
                    tracing::warn!(
                        "perl-lsp: workspace root not detected — module resolution disabled. \
                         To enable: open the project folder in your editor (File > Open Folder) \
                         rather than individual files. This warning appears once per server session."
                    );
                });
                return None;
            }
        };

        let mut include_paths = {
            let config = self.workspace_config.lock();
            let perl5lib_paths = std::env::var("PERL5LIB")
                .map(|v| perl_lsp_config::WorkspaceConfig::parse_perl5lib(&v))
                .unwrap_or_default();
            config.effective_include_paths(&perl5lib_paths)
        };

        if let Some(text) = doc_text {
            prepend_use_lib_paths(&mut include_paths, text, &root, None);
        }

        resolve_workspace_module_path(&root, module, &include_paths)
    }

    /// Resolve module path with document URI for FindBin support.
    ///
    /// Like `resolve_module_path` but also accepts the document URI so that
    /// `$FindBin::Bin`-relative paths are resolved against the document's directory.
    pub(crate) fn resolve_module_path_with_uri(
        &self,
        module: &str,
        doc_text: Option<&str>,
        doc_uri: Option<&str>,
    ) -> Option<PathBuf> {
        let root = match self.root_path.lock().clone() {
            Some(r) => r,
            None => {
                WARN_ONCE_ROOT_UNDETECTED.call_once(|| {
                    tracing::warn!(
                        "perl-lsp: workspace root not detected — module resolution disabled. \
                         To enable: open the project folder in your editor (File > Open Folder) \
                         rather than individual files. This warning appears once per server session."
                    );
                });
                return None;
            }
        };

        let mut include_paths = {
            let config = self.workspace_config.lock();
            let perl5lib_paths = std::env::var("PERL5LIB")
                .map(|v| perl_lsp_config::WorkspaceConfig::parse_perl5lib(&v))
                .unwrap_or_default();
            config.effective_include_paths(&perl5lib_paths)
        };

        if let Some(text) = doc_text {
            let file_dir = doc_uri
                .and_then(|u| url::Url::parse(u).ok())
                .and_then(|u| u.to_file_path().ok())
                .and_then(|p| p.parent().map(|d| d.to_path_buf()));
            if file_dir.is_none() && doc_uri.is_some() {
                tracing::trace!("Module URI resolution failed for doc_uri: {:?}", doc_uri);
            }
            prepend_use_lib_paths(&mut include_paths, text, &root, file_dir.as_deref());
        }

        resolve_workspace_module_path(&root, module, &include_paths)
    }

    /// Resolve an XS bootstrap target to the most likely `.xs` source path.
    ///
    /// XS distributions commonly place native sources either next to the Perl
    /// module file (`lib/Foo/Bar.xs`) or at the dist root as a leaf file
    /// (`Bar.xs`). This helper covers those two high-signal layouts.
    pub(crate) fn resolve_xs_bootstrap_path_with_uri(
        &self,
        module: &str,
        doc_text: Option<&str>,
        doc_uri: Option<&str>,
    ) -> Option<PathBuf> {
        let normalized = normalize_package_separator(module);
        let leaf = normalized.rsplit("::").next()?;

        if let Some(pm_path) = self.resolve_module_path_with_uri(module, doc_text, doc_uri)
            && let Some(parent) = pm_path.parent()
        {
            let sibling = parent.join(format!("{leaf}.xs"));
            if sibling.is_file() {
                return Some(sibling);
            }
        }

        let root = self.root_path.lock().clone()?;
        let root_candidate = root.join(format!("{leaf}.xs"));
        if root_candidate.is_file() {
            return Some(root_candidate);
        }

        None
    }

    /// Resolve a module name to a file path URI
    ///
    /// ## Resolution Precedence Order (deterministic)
    ///
    /// The resolution follows a strict precedence order designed for optimal
    /// developer experience and predictable behavior:
    ///
    /// 1. **Open Documents** (fastest path)
    ///    - Already-opened documents are checked first
    ///    - This ensures edits in progress take precedence
    ///
    /// 2. **Workspace Folders** (in initialization order)
    ///    - Folders are searched in the order they were added
    ///    - For each folder, configured include_paths are searched
    ///    - This respects multi-root workspace priority
    ///
    /// 3. **Configured Include Paths** (user-specified)
    ///    - Custom paths from workspace configuration
    ///    - Relative paths are resolved against each workspace folder
    ///
    /// 4. **System @INC** (opt-in only)
    ///    - Disabled by default (network filesystem concern)
    ///    - Enable via `workspace.useSystemInc: true` in settings
    ///    - Filtered to exclude `.` (current directory) for security
    ///
    /// ## Performance Characteristics
    /// - Timeout: Configurable (default 50ms) to prevent blocking
    /// - Returns None on timeout, allowing graceful degradation
    pub(crate) fn resolve_module_to_path(&self, module_name: &str) -> Option<String> {
        self.resolve_module_to_path_with_doc(module_name, None, None)
    }

    /// Resolve a module name to a file path URI, with optional document context for `use lib` wiring.
    ///
    /// `doc_text` is scanned for `use lib` statements; matched paths are prepended to
    /// the include list for this call only. `doc_uri` enables FindBin resolution against
    /// the document's directory.
    pub(crate) fn resolve_module_to_path_with_doc(
        &self,
        module_name: &str,
        doc_text: Option<&str>,
        doc_uri: Option<&str>,
    ) -> Option<String> {
        let (mut include_paths, timeout_ms, use_system_inc) = {
            let config = self.workspace_config.lock();
            let perl5lib_paths = std::env::var("PERL5LIB")
                .map(|v| perl_lsp_config::WorkspaceConfig::parse_perl5lib(&v))
                .unwrap_or_default();
            (
                config.effective_include_paths(&perl5lib_paths),
                config.resolution_timeout_ms,
                config.use_system_inc,
            )
        };
        let timeout = Duration::from_millis(timeout_ms);

        let workspace_folders = self.workspace_folders.lock().clone();

        // Wire use lib paths scoped to this call
        if let Some(text) = doc_text {
            let root_opt = workspace_root_for_doc(&workspace_folders, doc_uri);
            if root_opt.is_none() && !workspace_folders.is_empty() {
                tracing::trace!(
                    "Module URI resolution failed for workspace folders: {:?}",
                    workspace_folders
                );
            }
            if let Some(root) = root_opt {
                let file_dir = doc_uri
                    .and_then(|u| url::Url::parse(u).ok())
                    .and_then(|u| u.to_file_path().ok())
                    .and_then(|p| p.parent().map(|d| d.to_path_buf()));
                if file_dir.is_none() && doc_uri.is_some() {
                    tracing::trace!("Module URI resolution failed for doc_uri: {:?}", doc_uri);
                }
                prepend_use_lib_paths(&mut include_paths, text, &root, file_dir.as_deref());
            }
        }

        let open_document_uris: Vec<String> = {
            let documents = self.documents.lock();
            documents.keys().cloned().collect()
        };

        let system_paths = if use_system_inc {
            let mut config = self.workspace_config.lock();
            // `WorkspaceConfig` now resolves the active interpreter with
            // perlbrew/plenv-aware fallback before probing startup `@INC`.
            config.get_system_inc().to_vec()
        } else {
            Vec::new()
        };

        match resolve_module_uri(
            module_name,
            &open_document_uris,
            &workspace_folders,
            &include_paths,
            use_system_inc,
            &system_paths,
            timeout,
        ) {
            ModuleUriResolution::Resolved(uri) => Some(uri),
            ModuleUriResolution::TimedOut => {
                tracing::warn!("Module resolution timeout for: {}", module_name);
                None
            }
            ModuleUriResolution::NotFound => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // --- workspace root detection warning tests ---

    /// When root_path is None, resolve_module_path must return None without panicking.
    ///
    /// NOTE: We do not capture tracing output here because tracing-test adds a
    /// non-trivial test dependency and the WARN_ONCE static is process-global —
    /// capturing reliably across parallel tests would require test isolation at the
    /// process level. The behavioral contract (None return, no panic) is verified
    /// instead. The once-per-session warning is exercised manually via the LSP server
    /// under normal operation.
    #[test]
    fn resolve_module_path_returns_none_when_root_path_unset() {
        let server = LspServer::new();
        // root_path is None by default — do not set it
        let result = server.resolve_module_path("Some::Module", None);
        assert!(
            result.is_none(),
            "resolve_module_path must return None when workspace root is not detected"
        );
    }

    #[test]
    fn resolve_module_path_with_uri_returns_none_when_root_path_unset() {
        let server = LspServer::new();
        // root_path is None by default — do not set it
        let result = server.resolve_module_path_with_uri("Some::Module", None, None);
        assert!(
            result.is_none(),
            "resolve_module_path_with_uri must return None when workspace root is not detected"
        );
    }

    /// Calling the same code path multiple times must not panic or cause issues.
    /// The WARN_ONCE guarantees the warning fires only once, but subsequent calls
    /// still return None (behavioral invariant).
    #[test]
    fn resolve_module_path_returns_none_repeatedly_when_root_path_unset() {
        let server = LspServer::new();
        for _ in 0..3 {
            let result = server.resolve_module_path("Repeat::Module", None);
            assert!(
                result.is_none(),
                "resolve_module_path must consistently return None when workspace root unset"
            );
        }
    }

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn resolve_module_path_blocks_traversal_include_paths() -> TestResult {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");
        let escaped_dir = temp.path().join("escaped");
        fs::create_dir_all(&workspace)?;
        fs::create_dir_all(&escaped_dir)?;

        let escaped_file = escaped_dir.join("Target.pm");
        fs::write(&escaped_file, "package escaped::Target; 1;")?;

        let server = LspServer::new();
        *server.root_path.lock() = Some(workspace.clone());
        {
            let mut config = server.workspace_config.lock();
            config.include_paths = vec!["..".to_string()];
        }

        let resolved = server
            .resolve_module_path("escaped::Target", None)
            .ok_or("expected resolve_module_path result")?;

        // Traversal include paths must not resolve to files outside workspace.
        assert!(resolved.starts_with(&workspace));
        assert_ne!(resolved, escaped_file);
        Ok(())
    }

    #[test]
    fn resolve_module_to_path_blocks_traversal_include_paths() -> TestResult {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");
        let escaped_dir = temp.path().join("escaped");
        fs::create_dir_all(&workspace)?;
        fs::create_dir_all(&escaped_dir)?;

        let escaped_file = escaped_dir.join("Target.pm");
        fs::write(&escaped_file, "package escaped::Target; 1;")?;

        let server = LspServer::new();
        let workspace_uri =
            url::Url::from_file_path(&workspace).map_err(|_| "failed to create workspace URI")?;
        *server.workspace_folders.lock() = vec![workspace_uri.to_string()];
        {
            let mut config = server.workspace_config.lock();
            config.include_paths = vec!["..".to_string()];
            config.use_system_inc = false;
        }

        let resolved = server.resolve_module_to_path("escaped::Target");
        assert!(
            resolved.is_none(),
            "module resolution should ignore traversal include path and not return outside URI"
        );
        Ok(())
    }

    #[test]
    fn resolve_module_to_path_finds_workspace_module() -> TestResult {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");
        let module_file = workspace.join("lib").join("Demo").join("Worker.pm");

        fs::create_dir_all(module_file.parent().ok_or("missing module parent")?)?;
        fs::write(&module_file, "package Demo::Worker; 1;")?;

        let server = LspServer::new();
        let workspace_uri =
            url::Url::from_file_path(&workspace).map_err(|_| "failed to create workspace URI")?;
        *server.workspace_folders.lock() = vec![workspace_uri.to_string()];
        {
            let mut config = server.workspace_config.lock();
            config.include_paths = vec!["lib".to_string()];
            config.use_system_inc = false;
        }

        let resolved = server.resolve_module_to_path("Demo::Worker");
        let resolved = resolved.ok_or("expected resolved module URI")?;

        assert!(resolved.starts_with("file://"));
        assert!(resolved.contains("Demo"));
        assert!(resolved.contains("Worker.pm"));
        Ok(())
    }

    #[test]
    fn workspace_root_for_doc_prefers_most_specific_workspace_folder() -> TestResult {
        let temp = tempfile::tempdir()?;
        let repo = temp.path().join("repo");
        let app = repo.join("app");
        let script = app.join("script").join("run.pl");
        fs::create_dir_all(script.parent().ok_or("missing script parent")?)?;
        fs::write(&script, "use strict;\n")?;

        let repo_uri = url::Url::from_file_path(&repo).map_err(|_| "failed repo URI")?;
        let app_uri = url::Url::from_file_path(&app).map_err(|_| "failed app URI")?;
        let doc_uri = url::Url::from_file_path(&script).map_err(|_| "failed doc URI")?;
        let workspace_folders = vec![repo_uri.to_string(), app_uri.to_string()];

        let matched = workspace_root_for_doc(&workspace_folders, Some(doc_uri.as_str()))
            .ok_or("expected a matching workspace root")?;
        assert_eq!(matched, app, "nested workspace root should prefer most specific folder");
        Ok(())
    }

    #[test]
    fn workspace_root_for_doc_falls_back_to_first_workspace_folder() -> TestResult {
        let temp = tempfile::tempdir()?;
        let repo = temp.path().join("repo");
        let app = repo.join("app");
        fs::create_dir_all(&repo)?;
        fs::create_dir_all(&app)?;

        let repo_uri = url::Url::from_file_path(&repo).map_err(|_| "failed repo URI")?;
        let app_uri = url::Url::from_file_path(&app).map_err(|_| "failed app URI")?;
        let workspace_folders = vec![repo_uri.to_string(), app_uri.to_string()];

        let matched = workspace_root_for_doc(&workspace_folders, None)
            .ok_or("expected fallback workspace root")?;
        assert_eq!(
            matched, repo,
            "fallback should keep first workspace folder when no document URI is provided"
        );
        Ok(())
    }

    #[test]
    fn resolve_xs_bootstrap_path_finds_sibling_xs_file() -> TestResult {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");
        let module_file = workspace.join("lib").join("My").join("Module.pm");
        let xs_file = workspace.join("lib").join("My").join("Module.xs");

        fs::create_dir_all(module_file.parent().ok_or("missing module parent")?)?;
        fs::write(&module_file, "package My::Module; 1;")?;
        fs::write(&xs_file, "EXTERN_C void boot_My__Module(pTHX_ CV* cv) {}")?;

        let server = LspServer::new();
        *server.root_path.lock() = Some(workspace.clone());
        let workspace_uri =
            url::Url::from_file_path(&workspace).map_err(|_| "failed to create workspace URI")?;
        *server.workspace_folders.lock() = vec![workspace_uri.to_string()];
        {
            let mut config = server.workspace_config.lock();
            config.include_paths = vec!["lib".to_string()];
            config.use_system_inc = false;
        }

        let resolved = server
            .resolve_xs_bootstrap_path_with_uri("My::Module", None, None)
            .ok_or("expected xs bootstrap path")?;
        assert_eq!(resolved, xs_file);
        Ok(())
    }

    #[test]
    fn resolve_xs_bootstrap_path_finds_root_leaf_xs_file() -> TestResult {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");
        let module_file = workspace.join("lib").join("My").join("Module.pm");
        let xs_file = workspace.join("Module.xs");

        fs::create_dir_all(module_file.parent().ok_or("missing module parent")?)?;
        fs::write(&module_file, "package My::Module; 1;")?;
        fs::write(&xs_file, "EXTERN_C void boot_My__Module(pTHX_ CV* cv) {}")?;

        let server = LspServer::new();
        *server.root_path.lock() = Some(workspace.clone());
        let workspace_uri =
            url::Url::from_file_path(&workspace).map_err(|_| "failed to create workspace URI")?;
        *server.workspace_folders.lock() = vec![workspace_uri.to_string()];
        {
            let mut config = server.workspace_config.lock();
            config.include_paths = vec!["lib".to_string()];
            config.use_system_inc = false;
        }

        let resolved = server
            .resolve_xs_bootstrap_path_with_uri("My::Module", None, None)
            .ok_or("expected xs bootstrap path")?;
        assert_eq!(resolved, xs_file);
        Ok(())
    }

    // --- use lib wiring tests ---

    #[test]
    fn test_resolve_module_path_use_lib_single_quoted() -> TestResult {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");
        let module_file = workspace.join("custom").join("Foo").join("Baz.pm");
        fs::create_dir_all(module_file.parent().ok_or("no parent")?)?;
        fs::write(&module_file, "package Foo::Baz; 1;")?;

        let server = LspServer::new();
        *server.root_path.lock() = Some(workspace.clone());
        // No static include_paths configured — relies entirely on use lib wiring
        {
            let mut config = server.workspace_config.lock();
            config.include_paths = vec![];
        }

        let doc_text = "use lib 'custom';\nuse Foo::Baz;\n";
        let resolved = server
            .resolve_module_path("Foo::Baz", Some(doc_text))
            .ok_or("expected resolve_module_path to find Foo::Baz via use lib")?;

        assert!(
            resolved.ends_with("custom/Foo/Baz.pm") || resolved.ends_with("custom\\Foo\\Baz.pm"),
            "unexpected path: {}",
            resolved.display()
        );
        Ok(())
    }

    #[test]
    fn test_resolve_module_path_use_lib_qw_multiple_paths() -> TestResult {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");
        let module_file = workspace.join("t").join("lib").join("Test").join("Helper.pm");
        fs::create_dir_all(module_file.parent().ok_or("no parent")?)?;
        fs::write(&module_file, "package Test::Helper; 1;")?;

        let server = LspServer::new();
        *server.root_path.lock() = Some(workspace.clone());
        {
            let mut config = server.workspace_config.lock();
            config.include_paths = vec![];
        }

        let doc_text = "use lib qw(custom t/lib);\n";
        let resolved = server
            .resolve_module_path("Test::Helper", Some(doc_text))
            .ok_or("expected resolve_module_path to find Test::Helper via use lib qw")?;

        assert!(
            resolved.ends_with("t/lib/Test/Helper.pm")
                || resolved.ends_with("t\\lib\\Test\\Helper.pm"),
            "unexpected path: {}",
            resolved.display()
        );
        Ok(())
    }

    #[test]
    fn test_resolve_module_path_no_lib_removes_overlay() -> TestResult {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");
        let custom_dir = workspace.join("custom");
        let module_file = custom_dir.join("Gone").join("Soon.pm");
        fs::create_dir_all(module_file.parent().ok_or("no parent")?)?;
        fs::write(&module_file, "package Gone::Soon; 1;")?;

        let server = LspServer::new();
        *server.root_path.lock() = Some(workspace.clone());
        {
            let mut config = server.workspace_config.lock();
            config.include_paths = vec![];
        }

        let doc_text = "use lib 'custom';\nno lib 'custom';\nuse Gone::Soon;\n";
        let resolved = server
            .resolve_module_path("Gone::Soon", Some(doc_text))
            .ok_or("expected candidate path")?;
        assert_ne!(
            resolved, module_file,
            "no lib should remove prior use lib path from lexical overlay"
        );
        Ok(())
    }

    #[test]
    fn test_resolve_module_path_repeated_use_lib_reorders_precedence() -> TestResult {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");

        let a_mod = workspace.join("a").join("Dup").join("Winner.pm");
        let b_mod = workspace.join("b").join("Dup").join("Winner.pm");
        fs::create_dir_all(a_mod.parent().ok_or("no parent")?)?;
        fs::create_dir_all(b_mod.parent().ok_or("no parent")?)?;
        fs::write(&a_mod, "package Dup::Winner; 1;")?;
        fs::write(&b_mod, "package Dup::Winner; 1;")?;

        let server = LspServer::new();
        *server.root_path.lock() = Some(workspace.clone());
        {
            let mut config = server.workspace_config.lock();
            config.include_paths = vec![];
        }

        let doc_text = "use lib 'a';\nuse lib 'b';\nuse lib 'a';\n";
        let resolved = server
            .resolve_module_path("Dup::Winner", Some(doc_text))
            .ok_or("expected resolve_module_path to find Dup::Winner via repeated use lib")?;

        assert_eq!(resolved, a_mod, "re-adding a path should move it to front");
        Ok(())
    }

    #[test]
    fn test_resolve_module_path_no_doc_text_unchanged() -> TestResult {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");
        let module_file = workspace.join("lib").join("Stable").join("Mod.pm");
        fs::create_dir_all(module_file.parent().ok_or("no parent")?)?;
        fs::write(&module_file, "package Stable::Mod; 1;")?;

        let server = LspServer::new();
        *server.root_path.lock() = Some(workspace.clone());
        {
            let mut config = server.workspace_config.lock();
            config.include_paths = vec!["lib".to_string()];
        }

        // None doc_text: should still find module via static include_paths
        let resolved = server
            .resolve_module_path("Stable::Mod", None)
            .ok_or("expected resolve_module_path to find Stable::Mod with None doc_text")?;

        assert!(
            resolved.ends_with("lib/Stable/Mod.pm") || resolved.ends_with("lib\\Stable\\Mod.pm"),
            "unexpected path: {}",
            resolved.display()
        );
        Ok(())
    }

    #[test]
    fn test_resolve_module_path_use_lib_no_global_pollution() -> TestResult {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");
        let module_file = workspace.join("custom").join("Transient").join("Mod.pm");
        fs::create_dir_all(module_file.parent().ok_or("no parent")?)?;
        fs::write(&module_file, "package Transient::Mod; 1;")?;

        let server = LspServer::new();
        *server.root_path.lock() = Some(workspace.clone());
        {
            let mut config = server.workspace_config.lock();
            config.include_paths = vec![];
        }

        // Doc A finds module via use lib (path contains "custom")
        let doc_a_text = "use lib 'custom';\n";
        let found = server
            .resolve_module_path("Transient::Mod", Some(doc_a_text))
            .ok_or("doc A should find Transient::Mod via use lib")?;
        assert!(
            found.starts_with(&workspace),
            "doc A result should be inside workspace: {found:?}"
        );
        let found_str = found.to_string_lossy();
        assert!(
            found_str.contains("custom"),
            "doc A result should use 'custom' path from use lib: {found_str}"
        );

        // Doc B (no use lib) must resolve to a different path — no global state pollution.
        // resolve_module_path always returns Some (a candidate), but must not use "custom".
        let doc_b_result = server
            .resolve_module_path("Transient::Mod", None)
            .ok_or("resolve_module_path with None doc_text returned None unexpectedly")?;
        let doc_b_str = doc_b_result.to_string_lossy();
        assert!(
            !doc_b_str.contains("custom"),
            "doc B (no use lib) must not include 'custom' path — global state pollution detected: {doc_b_str}"
        );
        Ok(())
    }

    #[test]
    fn test_resolve_module_path_use_lib_nonexistent_does_not_crash() -> TestResult {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");
        fs::create_dir_all(&workspace)?;

        let server = LspServer::new();
        *server.root_path.lock() = Some(workspace.clone());
        {
            let mut config = server.workspace_config.lock();
            config.include_paths = vec![];
        }

        let doc_text = "use lib '/totally/nonexistent/path';\n";
        // Should not panic/crash; returns None normally
        let _result = server.resolve_module_path("NoSuch::Module", Some(doc_text));
        Ok(())
    }

    #[test]
    fn test_resolve_module_path_use_lib_outside_workspace_honored() -> TestResult {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");
        fs::create_dir_all(&workspace)?;
        // Place a module OUTSIDE the workspace and verify absolute use lib can find it.
        let outside_dir = temp.path().join("outside");
        let outside_module = outside_dir.join("Evil").join("Hack.pm");
        fs::create_dir_all(outside_module.parent().ok_or("no parent")?)?;
        fs::write(&outside_module, "package Evil::Hack; 1;")?;

        let server = LspServer::new();
        *server.root_path.lock() = Some(workspace.clone());
        {
            let mut config = server.workspace_config.lock();
            config.include_paths = vec![];
        }

        // Absolute paths in use lib should be honored literally.
        let outside_dir_str = outside_dir.to_string_lossy().to_string();
        let doc_text = format!("use lib '{outside_dir_str}';\n");
        let result = server
            .resolve_module_path("Evil::Hack", Some(&doc_text))
            .ok_or("resolve_module_path returned None unexpectedly")?;
        assert_eq!(
            result, outside_module,
            "absolute path outside workspace should resolve directly: {result:?}"
        );
        Ok(())
    }

    #[test]
    fn test_resolve_module_path_findbin_resolves_against_file_dir() -> TestResult {
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");
        let scripts_dir = workspace.join("scripts");
        let lib_dir = scripts_dir.join("lib");
        let module_file = lib_dir.join("Local").join("Tool.pm");
        fs::create_dir_all(module_file.parent().ok_or("no parent")?)?;
        fs::write(&module_file, "package Local::Tool; 1;")?;

        let server = LspServer::new();
        *server.root_path.lock() = Some(workspace.clone());
        {
            let mut config = server.workspace_config.lock();
            config.include_paths = vec![];
        }

        let doc_text = "use FindBin;\nuse lib \"$FindBin::Bin/lib\";\n";
        // The doc_uri points to /workspace/scripts/main.pl
        let doc_uri = url::Url::from_file_path(scripts_dir.join("main.pl"))
            .map_err(|_| "failed to create doc URI")?
            .to_string();

        let resolved =
            server.resolve_module_path_with_uri("Local::Tool", Some(doc_text), Some(&doc_uri));
        let resolved = resolved.ok_or("expected resolve to find Local::Tool via FindBin")?;

        assert!(
            resolved.ends_with("scripts/lib/Local/Tool.pm")
                || resolved.ends_with("scripts\\lib\\Local\\Tool.pm"),
            "unexpected path: {}",
            resolved.display()
        );
        Ok(())
    }

    #[test]
    fn test_resolve_module_path_findbin_dotdot_traversal_blocked() -> TestResult {
        // A FindBin path like "$FindBin::Bin/../../../etc" must not escape the workspace.
        // Even if resolve_use_lib_paths emits an absolute path string for the out-of-workspace
        // resolved directory, validate_workspace_path in the resolution layer must reject it.
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");
        let scripts_dir = workspace.join("scripts");
        fs::create_dir_all(&scripts_dir)?;
        // Place a file outside the workspace that should never be reachable.
        let outside = temp.path().join("secret");
        fs::create_dir_all(outside.join("Evil"))?;
        fs::write(outside.join("Evil").join("Secrets.pm"), "package Evil::Secrets; 1;")?;

        let server = LspServer::new();
        *server.root_path.lock() = Some(workspace.clone());
        {
            let mut config = server.workspace_config.lock();
            config.include_paths = vec![];
        }

        // The doc URI is in scripts/; "$FindBin::Bin/../../secret" would escape the workspace.
        let doc_text = "use FindBin;\nuse lib \"$FindBin::Bin/../../secret\";\n";
        let doc_uri = url::Url::from_file_path(scripts_dir.join("main.pl"))
            .map_err(|_| "failed to create doc URI")?
            .to_string();

        let result =
            server.resolve_module_path_with_uri("Evil::Secrets", Some(doc_text), Some(&doc_uri));

        // Result must be None (file doesn't exist inside workspace) or a path inside workspace.
        if let Some(ref path) = result {
            assert!(
                path.starts_with(&workspace),
                "FindBin dotdot traversal must not resolve outside workspace: {path:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn test_resolve_module_path_malformed_use_lib_does_not_crash() -> TestResult {
        // Malformed use lib statements (unclosed quote, empty, bare word) must be
        // silently skipped — no panic, no crash, no spurious paths added.
        let temp = tempfile::tempdir()?;
        let workspace = temp.path().join("workspace");
        fs::create_dir_all(&workspace)?;

        let server = LspServer::new();
        *server.root_path.lock() = Some(workspace.clone());
        {
            let mut config = server.workspace_config.lock();
            config.include_paths = vec![];
        }

        let malformed_cases = [
            // Unclosed single quote
            "use lib 'unclosed;\n",
            // No argument at all
            "use lib;\n",
            // Bare word (no quotes)
            "use lib bareword;\n",
            // Empty qw
            "use lib qw();\n",
            // Mixed malformed + valid: valid path must still be picked up
            "use lib 'unclosed;\nuse lib 'good_path';\n",
        ];

        for doc_text in &malformed_cases {
            // Must not panic
            let _result = server.resolve_module_path("Any::Module", Some(doc_text));
        }
        Ok(())
    }
}
