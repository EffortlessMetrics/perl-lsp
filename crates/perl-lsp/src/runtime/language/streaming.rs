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
        let (ai_enabled, streaming_enabled, max_output_tokens, timeout_ms, fallback) = {
            let guard = self.config.lock();
            let ai = &guard.ai_completion;
            (ai.enabled, ai.streaming.enabled, ai.max_output_tokens, ai.timeout_ms, ai.fallback)
        };
        if !ai_enabled || !streaming_enabled {
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
        let req =
            perl_lsp_inline_completion::BackendRequest { context, max_output_tokens, timeout_ms };

        let session_id = session.session_id.clone();
        let token_clone = token.clone();

        // Get the AI backend; fall back to one-shot if unavailable
        let backend = match self.ai_backend() {
            Some(b) => b,
            None => {
                if fallback {
                    return self.handle_inline_completion(Some(params));
                }
                // No backend and no fallback -- emit empty final and return
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
                    tracing::debug!(
                        "streaming inline completion: failed to send empty final: {}",
                        e
                    );
                }
                self.stream_sessions().cleanup();
                return Ok(Some(json!(null)));
            }
        };

        // Capture values needed inside the streaming closure.
        // No document locks are held at this point -- notify() only
        // touches the outbound channel, so it is safe to call during
        // the (potentially slow) network streaming call.
        let start_line = session.start_line;
        let start_character = session.start_character;

        // Track whether we sent any chunk so we know if a final is needed
        let mut sent_final = false;

        // Stream from the backend -- each chunk carries cumulative text
        let stream_result =
            backend.stream(&req, &mut |chunk: perl_lsp_inline_completion::StreamChunk| {
                // Check cancellation before emitting
                if session.is_cancelled() {
                    return perl_lsp_inline_completion::StreamControl::Stop;
                }

                // Update session cumulative text
                if let Ok(mut text) = session.current_text.lock() {
                    *text = chunk.text.clone();
                }

                let seq = session.next_sequence();
                let is_final = chunk.is_final;
                if is_final {
                    sent_final = true;
                }

                let progress = json!({
                    "token": token_clone,
                    "value": {
                        "kind": "perlInlineCompletionStream",
                        "sessionId": session_id,
                        "sequence": seq,
                        "isFinal": is_final,
                        "items": [{
                            "insertText": chunk.text,
                            "range": {
                                "start": { "line": start_line, "character": start_character },
                                "end": { "line": start_line, "character": start_character }
                            }
                        }]
                    }
                });

                if let Err(e) = self.notify("$/progress", progress) {
                    tracing::debug!("streaming inline completion: failed to send progress: {}", e);
                    return perl_lsp_inline_completion::StreamControl::Stop;
                }

                if is_final {
                    perl_lsp_inline_completion::StreamControl::Stop
                } else {
                    perl_lsp_inline_completion::StreamControl::Continue
                }
            });

        // If the stream ended without sending a final chunk, send one now
        if !sent_final && !session.is_cancelled() {
            let cumulative_text =
                session.current_text.lock().map(|t| t.clone()).unwrap_or_default();

            let items = if cumulative_text.is_empty() {
                json!([])
            } else {
                json!([{
                    "insertText": cumulative_text,
                    "range": {
                        "start": { "line": start_line, "character": start_character },
                        "end": { "line": start_line, "character": start_character }
                    }
                }])
            };

            let progress = json!({
                "token": token_clone,
                "value": {
                    "kind": "perlInlineCompletionStream",
                    "sessionId": session_id,
                    "sequence": session.next_sequence(),
                    "isFinal": true,
                    "items": items
                }
            });

            if let Err(e) = self.notify("$/progress", progress) {
                tracing::debug!(
                    "streaming inline completion: failed to send final progress: {}",
                    e
                );
            }
        }

        // Log backend errors but don't propagate -- the protocol contract
        // only needs the final isFinal:true notification to be sent.
        if let Err(e) = stream_result {
            tracing::debug!("streaming inline completion backend error: {}", e);
        }

        // Cleanup completed/cancelled sessions
        self.stream_sessions().cleanup();

        // Final response is null -- all data was sent via $/progress
        Ok(Some(json!(null)))
    }
}
