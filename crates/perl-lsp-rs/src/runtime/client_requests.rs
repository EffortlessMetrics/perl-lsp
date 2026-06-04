//! Server-to-client refresh requests (LSP 3.16+).
//!
//! Each method checks the relevant client capability before sending.

use super::*;
use crate::protocol::methods::WORKSPACE_APPLY_EDIT;

#[allow(dead_code)]
impl LspServer {
    /// Send a server-to-client request with no parameters (for refresh requests)
    pub(crate) fn send_request(&self, method: &str, params: Value) -> io::Result<ServerRequestId> {
        let id = self.next_server_request_id();
        self.outbound.send_request(id, method, params)?;
        Ok(id)
    }

    pub(crate) fn next_server_request_id(&self) -> ServerRequestId {
        loop {
            let current = self.next_request_id.load(Ordering::Relaxed);
            let next = if current == i32::MAX { 1 } else { current + 1 };
            if self
                .next_request_id
                .compare_exchange(current, next, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
            {
                if let Some(id) = ServerRequestId::new(current.max(1)) {
                    return id;
                }
            }
        }
    }

    pub(crate) fn request_apply_workspace_edit_with_metadata(
        &self,
        label: &str,
        edit: Value,
        is_refactoring: bool,
    ) -> io::Result<Option<ServerRequestId>> {
        let caps = self.client_capabilities.lock();
        if !caps.workspace_apply_edit_support || !caps.workspace_edit_metadata_support {
            return Ok(None);
        }
        drop(caps);

        let params = json!({
            "label": label,
            "edit": edit,
            "metadata": {
                "isRefactoring": is_refactoring,
            },
        });
        let id = self.send_request(WORKSPACE_APPLY_EDIT, params)?;
        tracing::debug!(%label, "Requested workspace/applyEdit with metadata");
        Ok(Some(id))
    }

    /// Request client to refresh code lenses (workspace/codeLens/refresh)
    pub fn request_code_lens_refresh(&self) -> io::Result<()> {
        if !self.client_capabilities.lock().code_lens_refresh_support {
            return Ok(());
        }
        self.send_request("workspace/codeLens/refresh", json!(null))?;
        tracing::debug!("Requested code lens refresh");
        Ok(())
    }

    /// Request client to refresh semantic tokens (workspace/semanticTokens/refresh)
    pub fn request_semantic_tokens_refresh(&self) -> io::Result<()> {
        if !self.client_capabilities.lock().semantic_tokens_refresh_support {
            return Ok(());
        }
        self.send_request("workspace/semanticTokens/refresh", json!(null))?;
        tracing::debug!("Requested semantic tokens refresh");
        Ok(())
    }

    /// Request client to refresh inlay hints (workspace/inlayHint/refresh)
    pub fn request_inlay_hint_refresh(&self) -> io::Result<()> {
        if !self.client_capabilities.lock().inlay_hint_refresh_support {
            return Ok(());
        }
        self.send_request("workspace/inlayHint/refresh", json!(null))?;
        tracing::debug!("Requested inlay hint refresh");
        Ok(())
    }

    /// Request client to refresh inline values (workspace/inlineValue/refresh)
    pub fn request_inline_value_refresh(&self) -> io::Result<()> {
        if !self.client_capabilities.lock().inline_value_refresh_support {
            return Ok(());
        }
        self.send_request("workspace/inlineValue/refresh", json!(null))?;
        tracing::debug!("Requested inline value refresh");
        Ok(())
    }

    /// Request client to refresh diagnostics (workspace/diagnostic/refresh)
    pub fn request_diagnostic_refresh(&self) -> io::Result<()> {
        if !self.client_capabilities.lock().diagnostic_refresh_support {
            return Ok(());
        }
        self.send_request("workspace/diagnostic/refresh", json!(null))?;
        tracing::debug!("Requested diagnostic refresh");
        Ok(())
    }

    /// Request client to refresh folding ranges (workspace/foldingRange/refresh)
    pub fn request_folding_range_refresh(&self) -> io::Result<()> {
        if !self.client_capabilities.lock().folding_range_refresh_support {
            return Ok(());
        }
        self.send_request("workspace/foldingRange/refresh", json!(null))?;
        tracing::debug!("Requested folding range refresh");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;
    use perl_lsp_rs_core::transport::framing::ContentLengthFramer;
    use std::io::Write;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

    #[derive(Clone, Default)]
    struct OutputCapture {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    impl OutputCapture {
        fn messages(&self) -> TestResult<Vec<Value>> {
            let bytes = self.buffer.lock().clone();
            let mut framer = ContentLengthFramer::new();
            framer.push(&bytes);

            let mut messages = Vec::new();
            while let Some(body) = framer.try_next()? {
                messages.push(serde_json::from_slice::<Value>(&body)?);
            }
            Ok(messages)
        }
    }

    impl Write for OutputCapture {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.buffer.lock().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn server_with_output_capture() -> (LspServer, OutputCapture) {
        let output = OutputCapture::default();
        let server = LspServer::with_output(Arc::new(Mutex::new(
            Box::new(output.clone()) as Box<dyn Write + Send>
        )));
        (server, output)
    }

    #[test]
    fn request_apply_workspace_edit_with_metadata_call_presence_observer() -> TestResult {
        let (server, output) = server_with_output_capture();
        {
            let mut caps = server.client_capabilities.lock();
            caps.workspace_apply_edit_support = true;
            caps.workspace_edit_metadata_support = true;
        }

        let request_id = server.request_apply_workspace_edit_with_metadata(
            "Safe delete reset",
            json!({"changes": {"file:///workspace/main.pl": []}}),
            true,
        )?;
        assert!(
            request_id.is_some(),
            "metadata-capable clients should receive workspace/applyEdit"
        );

        thread::sleep(Duration::from_millis(50));
        let messages = output.messages()?;
        let request = messages
            .iter()
            .find(|message| {
                message.get("method").and_then(Value::as_str) == Some(WORKSPACE_APPLY_EDIT)
            })
            .ok_or_else(|| format!("expected workspace/applyEdit request: {messages:?}"))?;
        assert_eq!(
            request.pointer("/params/label").and_then(Value::as_str),
            Some("Safe delete reset")
        );
        assert_eq!(
            request.pointer("/params/metadata/isRefactoring").and_then(Value::as_bool),
            Some(true)
        );
        assert!(
            request.pointer("/params/edit/metadata").is_none(),
            "metadata belongs on ApplyWorkspaceEditParams, not WorkspaceEdit: {request}"
        );
        Ok(())
    }

    #[test]
    fn request_apply_workspace_edit_with_metadata_boundary_discriminator() -> TestResult {
        let (server, output) = server_with_output_capture();
        server.client_capabilities.lock().workspace_apply_edit_support = true;
        let request_id = server.request_apply_workspace_edit_with_metadata(
            "Safe delete reset",
            json!({"changes": {"file:///workspace/main.pl": []}}),
            true,
        )?;
        assert!(
            request_id.is_none(),
            "workspace.applyEdit without metadataSupport must keep the old no-request path"
        );
        assert!(
            output.messages()?.is_empty(),
            "no workspace/applyEdit request should be emitted without metadataSupport"
        );

        let (server, output) = server_with_output_capture();
        server.client_capabilities.lock().workspace_edit_metadata_support = true;
        let request_id = server.request_apply_workspace_edit_with_metadata(
            "Safe delete reset",
            json!({"changes": {"file:///workspace/main.pl": []}}),
            true,
        )?;
        assert!(
            request_id.is_none(),
            "metadataSupport without workspace.applyEdit must keep the old no-request path"
        );
        assert!(
            output.messages()?.is_empty(),
            "no workspace/applyEdit request should be emitted without workspace.applyEdit"
        );
        Ok(())
    }

    #[test]
    fn request_apply_workspace_edit_with_metadata_return_value_discriminator() -> TestResult {
        let (server, _) = server_with_output_capture();
        let request_id = server.request_apply_workspace_edit_with_metadata(
            "Safe delete reset",
            json!({"changes": {"file:///workspace/main.pl": []}}),
            true,
        )?;
        assert!(request_id.is_none(), "the unsupported-client boundary returns Ok(None)");

        {
            let mut caps = server.client_capabilities.lock();
            caps.workspace_apply_edit_support = true;
            caps.workspace_edit_metadata_support = true;
        }
        let request_id = server.request_apply_workspace_edit_with_metadata(
            "Safe delete reset",
            json!({"changes": {"file:///workspace/main.pl": []}}),
            true,
        )?;
        assert!(
            request_id.is_some(),
            "the supported-client boundary returns Some server request id"
        );
        Ok(())
    }
}
