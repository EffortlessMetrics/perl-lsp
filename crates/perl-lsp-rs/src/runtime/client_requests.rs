//! Server-to-client refresh requests (LSP 3.16+).
//!
//! Each method checks the relevant client capability before sending.

use super::*;

#[allow(dead_code)]
impl LspServer {
    /// Send a server-to-client request with no parameters (for refresh requests)
    pub(crate) fn send_request(&self, method: &str, params: Value) -> io::Result<()> {
        let request_id = self.next_request_id.fetch_add(1, Ordering::SeqCst);
        self.outbound.send_request(request_id, method, params)
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
