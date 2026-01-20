//! Virtual document content support for LSP 3.18
//!
//! Provides support for workspace/textDocumentContent to serve virtual documents
//! like perldoc:// URIs for Perl documentation.

use super::super::*;

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

        // Parse URI to determine scheme
        if let Some(content) = fetch_virtual_content(uri) {
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
    }
}

/// Fetch content for a virtual URI
fn fetch_virtual_content(uri: &str) -> Option<String> {
    if let Some(module_name) = uri.strip_prefix("perldoc://") {
        fetch_perldoc(module_name)
    } else {
        None
    }
}

/// Fetch Perl documentation using perldoc
fn fetch_perldoc(module: &str) -> Option<String> {
    // Run perldoc -T Module::Name to get plain text documentation
    // Use -- to separate options from module name to prevent flag injection
    let output = std::process::Command::new("perldoc")
        .arg("-T")
        .arg("--")
        .arg(module)
        .output()
        .ok()?;

    if output.status.success() {
        String::from_utf8(output.stdout).ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_fetch_perldoc_injection_attempt() {
        // Try to inject a flag - should be treated as module name
        // "perldoc -T -- -v" (looks for module "-v")
        let result = fetch_perldoc("-v");
        // Should return None (module not found) or success if someone actually has a module named "-v"
        // But definitely should NOT execute verbose mode
        // Since we can't easily check what executed, we ensure it doesn't crash
        assert!(result.is_none() || result.is_some());
    }

    #[test]
    fn parser_fetch_perldoc_strict() {
        // Try to fetch documentation for the 'strict' module
        // This test will be skipped if perldoc is not available
        if let Some(content) = fetch_perldoc("strict") {
            assert!(content.contains("strict") || content.contains("STRICT"));
            assert!(content.len() > 100); // Should have some substantial content
        } else {
            eprintln!("Skipping test: perldoc not available or strict module not found");
        }
    }

    #[test]
    fn parser_fetch_perldoc_invalid() {
        // Try to fetch documentation for a non-existent module
        let result = fetch_perldoc("ThisModuleDefinitelyDoesNotExist12345");
        assert!(result.is_none());
    }

    #[test]
    fn parser_virtual_content_perldoc_uri() {
        let uri = "perldoc://strict";
        let content = fetch_virtual_content(uri);
        // May be None if perldoc is not available
        if let Some(content) = content {
            assert!(!content.is_empty());
        }
    }

    #[test]
    fn parser_virtual_content_invalid_scheme() {
        let uri = "invalid://some/path";
        let content = fetch_virtual_content(uri);
        assert!(content.is_none());
    }
}
