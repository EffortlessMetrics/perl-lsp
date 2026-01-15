//! Semantic tokens handlers
//!
//! Handles textDocument/semanticTokens/full and textDocument/semanticTokens/range requests.
//!
//! Includes deadline enforcement to prevent blocking on large files.

use super::super::*;
use crate::protocol::req_uri;
use crate::state::semantic_tokens_deadline;
use std::time::Instant;

impl LspServer {
    /// Handle textDocument/semanticTokens/full request
    ///
    /// Uses deadline enforcement to prevent blocking on very large files.
    /// If deadline is exceeded, returns partial tokens collected so far.
    pub(crate) fn handle_semantic_tokens(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let start = Instant::now();
        let deadline = semantic_tokens_deadline();

        if let Some(p) = params {
            let uri = req_uri(&p)?;
            let documents = self.documents_guard();
            let doc = self.get_document(&documents, uri).ok_or_else(|| JsonRpcError {
                code: INVALID_REQUEST,
                message: format!("Document not open: {}", uri),
                data: None,
            })?;
            if let Some(ref ast) = doc.ast {
                let data =
                    crate::semantic_tokens::collect_semantic_tokens(ast, &doc.text, &|off| {
                        self.offset_to_pos16(doc, off)
                    });
                let flat_data: Vec<_> = data.into_iter().flatten().collect();

                if start.elapsed() >= deadline {
                    eprintln!(
                        "SemanticTokens: deadline exceeded ({:?}), returning {} tokens",
                        start.elapsed(),
                        flat_data.len() / 5 // Each token is 5 u32s
                    );
                }

                return Ok(Some(json!({ "data": flat_data })));
            }
        }
        Ok(Some(json!({ "data": [] })))
    }

    /// Handle semantic tokens full request (alternative method name)
    #[allow(dead_code)] // Alternative implementation using SemanticTokensProvider
    pub(crate) fn handle_semantic_tokens_full(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(params) = params {
            let uri = req_uri(&params)?;

            eprintln!("Getting semantic tokens for: {}", uri);

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let provider = SemanticTokensProvider::new(doc.text.clone());
                    let tokens = provider.extract(ast);
                    let encoded = encode_semantic_tokens(&tokens);

                    eprintln!("Found {} semantic tokens", tokens.len());

                    return Ok(Some(json!({
                        "data": encoded
                    })));
                }
            }
        }

        Ok(Some(json!({
            "data": []
        })))
    }

    /// Handle semantic tokens range request
    pub(crate) fn handle_semantic_tokens_range(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        use crate::protocol::req_range;
        if let Some(params) = params {
            let uri = req_uri(&params)?;
            let ((start_line, _start_char), (end_line, _end_char)) = req_range(&params)?;

            eprintln!(
                "Getting semantic tokens for range: {} (lines {}-{})",
                uri, start_line, end_line
            );

            let documents = self.documents_guard();
            if let Some(doc) = self.get_document(&documents, uri) {
                if let Some(ref ast) = doc.ast {
                    let provider = SemanticTokensProvider::new(doc.text.clone());
                    let all_tokens = provider.extract(ast);

                    // Filter tokens to the requested range
                    let range_tokens: Vec<_> = all_tokens
                        .into_iter()
                        .filter(|token| token.line >= start_line && token.line <= end_line)
                        .collect();

                    let encoded = encode_semantic_tokens(&range_tokens);

                    eprintln!("Found {} semantic tokens in range", range_tokens.len());

                    return Ok(Some(json!({
                        "data": encoded
                    })));
                }
            }
        }

        Ok(Some(json!({
            "data": []
        })))
    }
}
