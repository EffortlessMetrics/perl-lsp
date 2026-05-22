//! Virtual document content support for LSP 3.18
//!
//! Provides support for workspace/textDocumentContent to serve virtual documents
//! like perldoc:// URIs for Perl documentation.

use super::super::*;
#[cfg(not(target_arch = "wasm32"))]
use perl_lsp_rs_core::config::PerlOracleEnv;
use perl_lsp_rs_core::config::WorkspaceConfig;

impl LspServer {
    /// Handle workspace/textDocumentContent request
    pub(crate) fn handle_text_document_content(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError {
            code: crate::protocol::INVALID_PARAMS,
            message: "Missing params".to_string(),
            data: None,
        })?;

        let uri = params.get("uri").and_then(|u| u.as_str()).ok_or_else(|| JsonRpcError {
            code: crate::protocol::INVALID_PARAMS,
            message: "Missing or invalid URI".to_string(),
            data: None,
        })?;

        let workspace_config = self.workspace_config.lock().clone();
        if let Some(content) = fetch_virtual_content(uri, &workspace_config) {
            Ok(Some(json!({ "text": content })))
        } else {
            Err(JsonRpcError {
                code: -32600,
                message: format!("Unsupported URI scheme or content not found: {}", uri),
                data: None,
            })
        }
    }

    /// Request client to refresh virtual document content
    pub fn request_text_document_content_refresh(&self, uri: &str) -> io::Result<()> {
        self.send_request("workspace/textDocumentContent/refresh", json!({ "uri": uri }))
            .map(|_| ())
    }
}

/// Fetch content for a virtual URI
fn fetch_virtual_content(uri: &str, config: &WorkspaceConfig) -> Option<String> {
    if let Some(module_name) = uri.strip_prefix("perldoc://") {
        fetch_perldoc(module_name, config)
    } else {
        None
    }
}

/// Fetch Perl documentation using perldoc
#[cfg(not(target_arch = "wasm32"))]
fn fetch_perldoc(module: &str, config: &WorkspaceConfig) -> Option<String> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let oracle = PerlOracleEnv::for_perldoc(config, cwd);
    let timeout_secs = oracle.timeout.as_secs();

    // Run perldoc -T Module::Name to get plain text documentation
    // Use -- to prevent argument injection if module starts with -
    let mut cmd = oracle.into_command();
    cmd.arg("-T").arg("--").arg(module);
    let output = match crate::util::run_command_with_timeout(cmd, timeout_secs) {
        Ok(output) => output,
        Err(e) => {
            tracing::warn!(module, error = %e, "Failed to run perldoc");
            return None;
        }
    };

    if output.status.success() {
        String::from_utf8(output.stdout)
            .map_err(|e| tracing::warn!(module, error = %e, "Invalid UTF-8 in perldoc output"))
            .ok()
    } else {
        None
    }
}

/// Fetch Perl documentation using perldoc.
#[cfg(target_arch = "wasm32")]
fn fetch_perldoc(_module: &str, _config: &WorkspaceConfig) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_fetch_perldoc_strict() {
        // Try to fetch documentation for the 'strict' module
        // This test will be skipped if perldoc is not available
        let config = WorkspaceConfig::default();
        if let Some(content) = fetch_perldoc("strict", &config) {
            assert!(content.contains("strict") || content.contains("STRICT"));
            assert!(content.len() > 100); // Should have some substantial content
        } else {
            eprintln!("Skipping test: perldoc not available or strict module not found");
        }
    }

    #[test]
    fn parser_fetch_perldoc_invalid() {
        // Try to fetch documentation for a non-existent module
        let config = WorkspaceConfig::default();
        let result = fetch_perldoc("ThisModuleDefinitelyDoesNotExist12345", &config);
        assert!(result.is_none());
    }

    #[test]
    fn parser_virtual_content_perldoc_uri() {
        let uri = "perldoc://strict";
        let config = WorkspaceConfig::default();
        let content = fetch_virtual_content(uri, &config);
        // May be None if perldoc is not available
        if let Some(content) = content {
            assert!(!content.is_empty());
        }
    }

    #[test]
    fn parser_virtual_content_invalid_scheme() {
        let uri = "invalid://some/path";
        let config = WorkspaceConfig::default();
        let content = fetch_virtual_content(uri, &config);
        assert!(content.is_none());
    }

    #[test]
    fn parser_fetch_perldoc_argument_injection() {
        // Try to fetch documentation with a flag-like string
        // This should not crash or execute unexpected commands
        // perldoc -T -- -f should look for module named "-f" which likely doesn't exist
        let config = WorkspaceConfig::default();
        let result = fetch_perldoc("-f", &config);
        assert!(result.is_none());
    }
}
