//! Document store for managing in-memory text content
//!
//! Maintains the current state of all open documents, tracking
//! versions and content without relying on filesystem state.

use crate::line_index::LineIndex;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A document in the store
#[derive(Debug, Clone)]
pub struct Document {
    /// The document URI
    pub uri: String,
    /// LSP version number
    pub version: i32,
    /// The full text content
    pub text: String,
    /// Line index for position calculations
    pub line_index: LineIndex,
}

impl Document {
    /// Create a new document
    pub fn new(uri: String, version: i32, text: String) -> Self {
        let line_index = LineIndex::new(text.clone());
        Self { uri, version, text, line_index }
    }

    /// Update the document content
    pub fn update(&mut self, version: i32, text: String) {
        self.version = version;
        self.text = text.clone();
        self.line_index = LineIndex::new(text);
    }
}

/// Thread-safe document store
#[derive(Debug, Clone)]
pub struct DocumentStore {
    documents: Arc<RwLock<HashMap<String, Document>>>,
}

impl DocumentStore {
    /// Create a new empty store
    pub fn new() -> Self {
        Self { documents: Arc::new(RwLock::new(HashMap::new())) }
    }

    /// Normalize a URI to a consistent key
    /// This handles platform differences and ensures consistent lookups
    pub fn uri_key(uri: &str) -> String {
        perl_uri::uri_key(uri)
    }

    /// Open or update a document
    pub fn open(&self, uri: String, version: i32, text: String) {
        let key = Self::uri_key(&uri);
        let doc = Document::new(uri, version, text);

        if let Ok(mut docs) = self.documents.write() {
            docs.insert(key, doc);
        }
    }

    /// Update a document's content
    pub fn update(&self, uri: &str, version: i32, text: String) -> bool {
        let key = Self::uri_key(uri);

        let Ok(mut docs) = self.documents.write() else {
            return false;
        };
        if let Some(doc) = docs.get_mut(&key) {
            doc.update(version, text);
            true
        } else {
            false
        }
    }

    /// Close a document
    pub fn close(&self, uri: &str) -> bool {
        let key = Self::uri_key(uri);
        let Ok(mut docs) = self.documents.write() else {
            return false;
        };
        docs.remove(&key).is_some()
    }

    /// Get a document by URI
    pub fn get(&self, uri: &str) -> Option<Document> {
        let key = Self::uri_key(uri);
        let docs = self.documents.read().ok()?;
        docs.get(&key).cloned()
    }

    /// Get the text content of a document
    pub fn get_text(&self, uri: &str) -> Option<String> {
        self.get(uri).map(|doc| doc.text)
    }

    /// Get all open documents
    pub fn all_documents(&self) -> Vec<Document> {
        let Ok(docs) = self.documents.read() else {
            return Vec::new();
        };
        docs.values().cloned().collect()
    }

    /// Check if a document is open
    pub fn is_open(&self, uri: &str) -> bool {
        let key = Self::uri_key(uri);
        let Ok(docs) = self.documents.read() else {
            return false;
        };
        docs.contains_key(&key)
    }

    /// Get the count of open documents
    pub fn count(&self) -> usize {
        let Ok(docs) = self.documents.read() else {
            return 0;
        };
        docs.len()
    }
}

impl Default for DocumentStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::must_some;

    #[test]
    fn test_document_lifecycle() {
        let store = DocumentStore::new();
        let uri = "file:///test.pl".to_string();

        // Open document
        store.open(uri.clone(), 1, "print 'hello';".to_string());
        assert!(store.is_open(&uri));
        assert_eq!(store.count(), 1);

        // Get document
        let doc = must_some(store.get(&uri));
        assert_eq!(doc.version, 1);
        assert_eq!(doc.text, "print 'hello';");

        // Update document
        assert!(store.update(&uri, 2, "print 'world';".to_string()));
        let doc = must_some(store.get(&uri));
        assert_eq!(doc.version, 2);
        assert_eq!(doc.text, "print 'world';");

        // Close document
        assert!(store.close(&uri));
        assert!(!store.is_open(&uri));
        assert_eq!(store.count(), 0);
    }

    #[test]
    fn test_uri_drive_letter_normalization() {
        let uri1 = "file:///C:/test.pl";
        let uri2 = "file:///c:/test.pl";
        assert_eq!(DocumentStore::uri_key(uri1), DocumentStore::uri_key(uri2));
    }

    #[test]
    fn test_drive_letter_lookup() {
        let store = DocumentStore::new();
        let uri_upper = "file:///C:/test.pl".to_string();
        let uri_lower = "file:///c:/test.pl".to_string();

        store.open(uri_upper.clone(), 1, "# test".to_string());
        assert!(store.is_open(&uri_lower));
        assert_eq!(store.get_text(&uri_lower), Some("# test".to_string()));
        assert!(store.close(&uri_lower));
        assert_eq!(store.count(), 0);
    }

    #[test]
    fn test_multiple_documents() {
        let store = DocumentStore::new();

        let uri1 = "file:///a.pl".to_string();
        let uri2 = "file:///b.pl".to_string();

        store.open(uri1.clone(), 1, "# file a".to_string());
        store.open(uri2.clone(), 1, "# file b".to_string());

        assert_eq!(store.count(), 2);
        assert_eq!(store.get_text(&uri1), Some("# file a".to_string()));
        assert_eq!(store.get_text(&uri2), Some("# file b".to_string()));

        let all = store.all_documents();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_uri_with_spaces() {
        let store = DocumentStore::new();
        let uri = "file:///path%20with%20spaces/test.pl".to_string();

        store.open(uri.clone(), 1, "# test".to_string());
        assert!(store.is_open(&uri));

        let doc = must_some(store.get(&uri));
        assert_eq!(doc.text, "# test");
    }
}
