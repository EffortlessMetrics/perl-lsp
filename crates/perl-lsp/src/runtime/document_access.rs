//! Document access, position conversion, and URI helpers.
//!
//! Methods that look up documents, normalize URIs, convert between byte
//! offsets and LSP positions, and provide text-based fallback extractors.

use super::*;

#[allow(dead_code)]
impl LspServer {
    // === BEGIN_TEST_ONLY_POSITION_HELPERS ===
    /// Convert offset to line/column position (UTF-16 aware, CRLF safe)
    #[allow(deprecated)]
    pub fn offset_to_position(&self, content: &str, offset: usize) -> (u32, u32) {
        // Implementation moved to lsp/utils
        let p = offset_to_position(content, offset);
        (p.line, p.character)
    }

    /// Convert line/column position to offset (UTF-16 aware, CRLF safe)
    #[allow(deprecated)]
    pub fn position_to_offset(&self, content: &str, line: u32, character: u32) -> usize {
        // Implementation moved to lsp/utils
        position_to_offset(content, line, character).unwrap_or(content.len())
    }
    // === END_TEST_ONLY_POSITION_HELPERS ===

    /// Position conversion using cached line starts for O(log n) performance
    #[inline]
    pub(crate) fn pos16_to_offset(&self, doc: &DocumentState, line: u32, ch: u32) -> usize {
        // Uses the cached, CRLF/UTF-16 aware converter
        doc.line_starts.position_to_offset_rope(&doc.rope, line, ch)
    }

    /// Normalize URI key for consistent document lookup
    pub(crate) fn normalize_uri_key(&self, raw: &str) -> String {
        // Parse to Url to canonicalize, then stringify the way we store it.
        // If parsing fails, return the raw key so we at least try the given string.
        if let Ok(u) = url::Url::parse(raw) {
            // On Windows, lower-case the drive letter to match how many editors send it.
            #[cfg(windows)]
            {
                let s = u.as_str().to_string();
                if let Some(rest) = s.strip_prefix("file:///") {
                    if rest.len() > 1
                        && rest.as_bytes()[1] == b':'
                        && rest.as_bytes()[0].is_ascii_alphabetic()
                    {
                        return format!(
                            "file:///{}{}",
                            rest[0..1].to_ascii_lowercase(),
                            &rest[1..]
                        );
                    }
                }
                return s;
            }
            #[cfg(not(windows))]
            return u.as_str().to_string();
        }
        raw.to_string()
    }

    /// Get document by URI with normalization fallback
    pub(crate) fn get_document<'a>(
        &self,
        documents: &'a parking_lot::MutexGuard<'_, HashMap<String, DocumentState>>,
        uri: &str,
    ) -> Option<&'a DocumentState> {
        let normalized = self.normalize_uri_key(uri);
        documents.get(&normalized).or_else(|| documents.get(uri))
    }

    /// Get mutable document by URI with normalization fallback
    pub(crate) fn get_document_mut<'a>(
        &self,
        documents: &'a mut parking_lot::MutexGuard<'_, HashMap<String, DocumentState>>,
        uri: &str,
    ) -> Option<&'a mut DocumentState> {
        let normalized = self.normalize_uri_key(uri);
        if documents.contains_key(&normalized) {
            documents.get_mut(&normalized)
        } else {
            documents.get_mut(uri)
        }
    }

    /// Helper to create a ContentModified error response
    pub(crate) fn content_modified() -> JsonRpcError {
        JsonRpcError {
            code: CONTENT_MODIFIED,
            message: "Document changed before request executed".to_string(),
            data: None,
        }
    }

    /// Ensure the request version matches the current document version
    pub(crate) fn ensure_latest(
        &self,
        uri: &str,
        req_version: Option<i32>,
    ) -> Result<(), JsonRpcError> {
        if let Some(v) = req_version {
            let documents = self.documents.lock();
            if let Some(doc) = self.get_document(&documents, uri) {
                if v < doc.version {
                    return Err(Self::content_modified());
                }
            }
        }
        Ok(())
    }

    /// Offset to position conversion using cached line starts for O(log n) performance
    #[inline]
    pub(crate) fn offset_to_pos16(&self, doc: &DocumentState, offset: usize) -> (u32, u32) {
        doc.line_starts.offset_to_position_rope(&doc.rope, offset)
    }

    /// Extract code lenses from text when AST parsing fails
    pub(crate) fn extract_text_based_code_lenses(
        &self,
        text: &str,
        uri: &str,
    ) -> Vec<crate::code_lens_provider::CodeLens> {
        extract_text_based_code_lenses(text, uri)
    }

    /// Extract symbols from text when AST parsing fails
    #[cfg(feature = "workspace")]
    pub(crate) fn extract_text_based_symbols(
        &self,
        text: &str,
        uri: &str,
        query: &str,
    ) -> Vec<LspWorkspaceSymbol> {
        extract_text_based_symbols(text, uri, query)
    }

    /// Extract symbols stub when workspace feature is disabled
    #[cfg(not(feature = "workspace"))]
    pub(crate) fn extract_text_based_symbols(
        &self,
        _text: &str,
        _uri: &str,
        _query: &str,
    ) -> Vec<serde_json::Value> {
        Vec::new()
    }

    /// Get text around an offset position
    pub(crate) fn get_text_around_offset(
        &self,
        content: &str,
        offset: usize,
        radius: usize,
    ) -> String {
        get_text_around_offset(content, offset, radius)
    }

    /// Extract module reference from text (e.g., from "use Module::Name" or "require Module::Name")
    pub(crate) fn extract_module_reference(&self, text: &str, cursor_pos: usize) -> Option<String> {
        extract_module_reference(text, cursor_pos)
    }

    /// Extract module reference including `use parent`/`use base` argument modules.
    pub(crate) fn extract_module_reference_extended(
        &self,
        text: &str,
        cursor_pos: usize,
    ) -> Option<String> {
        extract_module_reference_extended(text, cursor_pos)
    }

    /// Get buffer text for a URI
    pub(crate) fn buffer_text(&self, uri: &str) -> Option<String> {
        let docs = self.documents.lock();
        docs.get(uri).map(|d| d.text.clone())
    }

    /// Iterate over all open buffers (for reference search)
    pub(crate) fn iter_open_buffers(&self) -> Vec<(String, String)> {
        let docs = self.documents.lock();
        docs.iter().map(|(uri, doc)| (uri.clone(), doc.text.clone())).collect()
    }

    /// Get or compute a `SemanticAnalyzer` for the given document text and AST.
    ///
    /// Cache key: `(normalized_uri, content_hash_of_text)`.
    /// The entry is valid as long as the content hash matches — no TTL needed.
    /// Cache is bounded to 50 entries; a simple clear is used when full.
    ///
    /// **Lock discipline**: acquires `semantic_analyzer_cache` in two short
    /// scopes (read, then write). May be called while `documents` is held;
    /// always acquire `documents` before `semantic_analyzer_cache` to maintain
    /// a consistent lock ordering and avoid deadlock.
    pub(crate) fn get_or_build_analyzer(
        &self,
        uri: &str,
        text: &str,
        ast: &perl_parser::ast::Node,
    ) -> Arc<crate::semantic::SemanticAnalyzer> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let content_hash = hasher.finish();

        let normalized = self.normalize_uri_key(uri);
        let key = (normalized, content_hash);

        // Read path: return clone of cached entry if present.
        {
            let cache = self.semantic_analyzer_cache.lock();
            if let Some(cached) = cache.get(&key) {
                return Arc::clone(cached);
            }
        }

        // Cache miss: build the analyzer outside the lock.
        let analyzer = Arc::new(crate::semantic::SemanticAnalyzer::analyze_with_source(ast, text));

        // Write path: insert, evicting all entries when the cache is full.
        {
            let mut cache = self.semantic_analyzer_cache.lock();
            if cache.len() >= 50 {
                cache.clear();
            }
            cache.insert(key, Arc::clone(&analyzer));
        }

        analyzer
    }
}
