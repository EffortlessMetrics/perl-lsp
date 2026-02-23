//! Module path resolution
//!
//! Handles resolution of Perl module names to file paths.

use super::super::*;
use perl_module_resolution::{
    ModuleUriResolution, resolve_module_path as resolve_workspace_module_path, resolve_module_uri,
};
use std::path::PathBuf;
use std::time::Duration;

impl LspServer {
    /// Enhanced module path resolver using workspace configuration
    ///
    /// Uses configurable include paths from `WorkspaceConfig` instead of
    /// hardcoded directories. Returns absolute filesystem path for a module.
    pub(crate) fn resolve_module_path(&self, module: &str) -> Option<PathBuf> {
        let root = self.root_path.lock().clone()?;

        let include_paths = {
            let config = self.workspace_config.lock();
            config.include_paths.clone()
        };

        resolve_workspace_module_path(&root, module, &include_paths)
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
        let (include_paths, timeout_ms, use_system_inc) = {
            let config = self.workspace_config.lock();
            (config.include_paths.clone(), config.resolution_timeout_ms, config.use_system_inc)
        };
        let timeout = Duration::from_millis(timeout_ms);

        let open_document_uris: Vec<String> = {
            let documents = self.documents.lock();
            documents.keys().cloned().collect()
        };

        let workspace_folders = self.workspace_folders.lock().clone();

        let system_paths = if use_system_inc {
            let mut config = self.workspace_config.lock();
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
                eprintln!("Module resolution timeout for: {}", module_name);
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
            .resolve_module_path("escaped::Target")
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
}
