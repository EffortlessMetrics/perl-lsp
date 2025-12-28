//! Semantic tokens handlers
//!
//! Handles textDocument/semanticTokens/full and textDocument/semanticTokens/range requests.

use super::super::*;

impl LspServer {
    /// Handle textDocument/semanticTokens/full request
    pub(crate) fn handle_semantic_tokens(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        if let Some(p) = params {
            let uri = p["textDocument"]["uri"].as_str().ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing textDocument.uri".into(),
                data: None,
            })?;
            let documents = self.documents.lock().unwrap_or_else(|e| e.into_inner());
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
                return Ok(Some(json!({ "data": data.into_iter().flatten().collect::<Vec<_>>() })));
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
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");

            eprintln!("Getting semantic tokens for: {}", uri);

            let documents = self.documents.lock().unwrap_or_else(|e| e.into_inner());
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
        if let Some(params) = params {
            let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
            let range = &params["range"];
            let start_line = range["start"]["line"].as_u64().unwrap_or(0) as u32;
            let end_line = range["end"]["line"].as_u64().unwrap_or(u32::MAX as u64) as u32;

            eprintln!(
                "Getting semantic tokens for range: {} (lines {}-{})",
                uri, start_line, end_line
            );

            let documents = self.documents.lock().unwrap_or_else(|e| e.into_inner());
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
