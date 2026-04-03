//! Streaming inline completion handler.
//!
//! Implements the custom `textDocument/perlInlineCompletionStream` request.
//! This handler starts a streaming session that emits cumulative inline
//! completion candidates via `$/progress` notifications. The final JSON-RPC
//! response is `null` -- all data is delivered through progress tokens.

use super::super::*;
use crate::protocol::{invalid_params, req_position, req_uri};
use crate::runtime::stream_session::SessionKey;

impl LspServer {
    /// Handle `textDocument/perlInlineCompletionStream` custom request.
    ///
    /// Starts a streaming session that emits cumulative candidates via `$/progress`.
    /// The final JSON-RPC response is `null` (all data sent via progress).
    ///
    /// If the client does not supply a `partialResultToken`, falls back to the
    /// standard one-shot `textDocument/inlineCompletion` handler.
    pub(crate) fn handle_streaming_inline_completion(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let params = params.ok_or_else(|| invalid_params("missing params"))?;

        let uri = req_uri(&params)?;
        let (line, character) = req_position(&params)?;
        let partial_result_token =
            params.get("partialResultToken").and_then(|v| v.as_str()).map(|s| s.to_string());
        let document_version = params
            .get("textDocument")
            .and_then(|td| td.get("version"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        // Must have a partial result token for streaming
        let token = match partial_result_token {
            Some(t) => t,
            None => {
                // Fall back to one-shot inline completion
                return self.handle_inline_completion(Some(params));
            }
        };

        // Snapshot text
        let text = {
            let documents = self.documents_guard();
            match self.get_document(&documents, uri) {
                Some(doc) => doc.text.clone(),
                None => return Ok(Some(json!(null))),
            }
        };

        // Check AI config
        let ai_config = self.config.lock().ai_completion.clone();
        if !ai_config.enabled || !ai_config.streaming.enabled {
            // Fall back to one-shot
            return self.handle_inline_completion(Some(params));
        }

        // Start session (cancels any previous for same position)
        let session_key = SessionKey {
            uri: uri.to_string(),
            document_version,
            line: u64::from(line),
            character: u64::from(character),
        };
        let session = self.stream_sessions().start_session(session_key);

        // Prepare context
        let provider = perl_lsp_inline_completion::InlineCompletionProvider::new();
        let context = match provider.prepare_context(&text, line, character) {
            Some(ctx) => ctx,
            None => return Ok(Some(json!(null))),
        };

        // Build request
        let req = perl_lsp_inline_completion::BackendRequest {
            context,
            max_output_tokens: ai_config.max_output_tokens,
            timeout_ms: ai_config.timeout_ms,
        };

        let session_id = session.session_id.clone();
        let token_clone = token.clone();

        // NOTE: The actual AI backend is not yet wired into LspServer.
        // When a backend becomes available, this section would call
        // `backend.stream(&req, &mut |chunk| { ... })`.
        //
        // For now we emit a single final progress notification with
        // an empty candidate list so that the protocol contract is
        // exercised end-to-end.
        let _ = &req; // suppress unused warning

        let progress = json!({
            "token": token_clone,
            "value": {
                "kind": "perlInlineCompletionStream",
                "sessionId": session_id,
                "sequence": session.next_sequence(),
                "isFinal": true,
                "items": []
            }
        });

        if let Err(e) = self.notify("$/progress", progress) {
            tracing::debug!("streaming inline completion: failed to send progress: {}", e);
        }

        // Cleanup
        self.stream_sessions().cleanup();

        // Final response is null -- all data was sent via $/progress
        Ok(Some(json!(null)))
    }
}
