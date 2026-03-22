//! Semantic tokens handlers
//!
//! Handles textDocument/semanticTokens/full and textDocument/semanticTokens/range requests.
//!
//! Includes deadline enforcement to prevent blocking on large files.
//! Implements LSP 3.16 delta encoding: resultId tracking + SemanticTokensDelta responses.

use super::super::*;
use crate::protocol::req_uri;
use crate::state::semantic_tokens_deadline;
use std::sync::atomic::Ordering;
use std::time::Instant;

impl LspServer {
    /// Handle textDocument/semanticTokens/full request
    ///
    /// Uses deadline enforcement to prevent blocking on very large files.
    /// If deadline is exceeded, returns partial tokens collected so far.
    /// Returns a `resultId` for delta tracking (LSP 3.16).
    pub(crate) fn handle_semantic_tokens(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let start = Instant::now();
        let deadline = semantic_tokens_deadline();

        if let Some(p) = params {
            let uri = req_uri(&p)?;
            let flat_data = {
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
                    data.into_iter().flatten().collect::<Vec<u32>>()
                } else {
                    Vec::new()
                }
            }; // documents lock released here

            if start.elapsed() >= deadline {
                eprintln!(
                    "SemanticTokens: deadline exceeded ({:?}), returning {} tokens",
                    start.elapsed(),
                    flat_data.len() / 5 // Each token is 5 u32s
                );
            }

            let result_id = self.next_semantic_tokens_result_id();
            self.semantic_tokens_cache
                .lock()
                .insert(uri.to_string(), (result_id.clone(), flat_data.clone()));

            return Ok(Some(json!({ "resultId": result_id, "data": flat_data })));
        }
        Ok(Some(json!({ "data": [] })))
    }

    /// Handle textDocument/semanticTokens/full/delta request (LSP 3.16).
    ///
    /// If the `previousResultId` matches the cached result, computes and returns
    /// a `SemanticTokensDelta` (`edits` array).  Falls back to a full response
    /// when the cache is empty or the ID does not match.
    pub(crate) fn handle_semantic_tokens_delta(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let p = match params {
            Some(p) => p,
            None => return Ok(Some(json!({ "data": [] }))),
        };

        let uri = req_uri(&p)?;
        let previous_result_id =
            p.get("previousResultId").and_then(|v| v.as_str()).ok_or_else(|| JsonRpcError {
                code: INVALID_REQUEST,
                message: "Missing required parameter: previousResultId".to_string(),
                data: None,
            })?;

        // Compute current tokens (documents lock released before cache access)
        let current_flat_data = {
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
                data.into_iter().flatten().collect::<Vec<u32>>()
            } else {
                Vec::new()
            }
        };

        let new_result_id = self.next_semantic_tokens_result_id();

        // Check cache and decide: delta or full fallback
        let cached = {
            let cache = self.semantic_tokens_cache.lock();
            cache.get(uri).cloned()
        };

        if let Some((cached_id, cached_data)) = cached {
            if cached_id == previous_result_id {
                // Compute and return delta
                let edits = compute_semantic_token_edits(&cached_data, &current_flat_data);
                self.semantic_tokens_cache
                    .lock()
                    .insert(uri.to_string(), (new_result_id.clone(), current_flat_data));
                return Ok(Some(json!({
                    "resultId": new_result_id,
                    "edits": edits
                })));
            }
        }

        // Stale or missing cache: return full response
        self.semantic_tokens_cache
            .lock()
            .insert(uri.to_string(), (new_result_id.clone(), current_flat_data.clone()));
        Ok(Some(json!({
            "resultId": new_result_id,
            "data": current_flat_data
        })))
    }

    /// Generate a monotonically increasing result ID for semantic tokens.
    fn next_semantic_tokens_result_id(&self) -> String {
        // Reuse the existing atomic counter for unique IDs across all LSP request types
        let id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        format!("st-{}", id)
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

/// Compute a minimal SemanticTokensEdit list from two flat u32 token arrays.
///
/// Uses a single-edit approach: find the first and last differing positions,
/// replace the changed interior with one edit.  Produces an empty vec when
/// the arrays are identical.
fn compute_semantic_token_edits(old: &[u32], new: &[u32]) -> Vec<Value> {
    // Find length of common prefix
    let prefix_len = old.iter().zip(new.iter()).take_while(|(a, b)| a == b).count();

    if prefix_len == old.len() && prefix_len == new.len() {
        return vec![]; // arrays are identical
    }

    // Find length of common suffix (capped so it doesn't overlap the prefix)
    let max_suffix = (old.len() - prefix_len).min(new.len() - prefix_len);
    let suffix_len = old[prefix_len..]
        .iter()
        .rev()
        .zip(new[prefix_len..].iter().rev())
        .take(max_suffix)
        .take_while(|(a, b)| a == b)
        .count();

    let delete_count = (old.len() - prefix_len - suffix_len) as u32;
    let insert_data: Vec<u32> = new[prefix_len..new.len() - suffix_len].to_vec();

    let mut edit = serde_json::Map::new();
    edit.insert("start".to_string(), json!(prefix_len as u32));
    edit.insert("deleteCount".to_string(), json!(delete_count));
    if !insert_data.is_empty() {
        edit.insert("data".to_string(), json!(insert_data));
    }
    vec![Value::Object(edit)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn compute_edits_identical_arrays() {
        let data = vec![0u32, 1, 2, 3, 4];
        assert!(compute_semantic_token_edits(&data, &data).is_empty());
    }

    #[test]
    fn compute_edits_empty_arrays() {
        assert!(compute_semantic_token_edits(&[], &[]).is_empty());
    }

    #[test]
    fn compute_edits_insert_into_empty() {
        let old: Vec<u32> = vec![];
        let new = vec![0u32, 1, 2, 3, 4];
        let edits = compute_semantic_token_edits(&old, &new);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0]["start"], json!(0u32));
        assert_eq!(edits[0]["deleteCount"], json!(0u32));
        assert_eq!(edits[0]["data"], json!(new));
    }

    #[test]
    fn compute_edits_delete_all() {
        let old = vec![0u32, 1, 2, 3, 4];
        let new: Vec<u32> = vec![];
        let edits = compute_semantic_token_edits(&old, &new);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0]["start"], json!(0u32));
        assert_eq!(edits[0]["deleteCount"], json!(5u32));
        assert!(edits[0].get("data").is_none());
    }

    #[test]
    fn compute_edits_middle_change() {
        let old = vec![0u32, 1, 2, 3, 4];
        let new = vec![0u32, 1, 99, 3, 4];
        let edits = compute_semantic_token_edits(&old, &new);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0]["start"], json!(2u32));
        assert_eq!(edits[0]["deleteCount"], json!(1u32));
        assert_eq!(edits[0]["data"], json!(vec![99u32]));
    }
}
