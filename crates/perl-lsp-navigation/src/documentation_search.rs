//! Documentation search provider for workspace-wide POD documentation search.
//!
//! Provides functionality to index POD (Plain Old Documentation) from Perl modules
//! and search across them with scope filtering.
//!
//! # Architecture
//!
//! This provider follows the same pattern as `WorkspaceSymbolsProvider`:
//! - `index_document()` to add/update documentation for a file
//! - `remove_document()` to remove a file's documentation
//! - `search()` to find documentation matching a query
//!
//! # Example
//!
//! ```rust,ignore
//! use perl_lsp_navigation::DocumentationSearchProvider;
//!
//! let mut provider = DocumentationSearchProvider::new();
//! provider.index_document("file:///lib/My/Module.pm", source_code);
//! let results = provider.search("My::Module", DocumentationSearchScope::Name);
//! ```

use std::collections::HashMap;

/// Search scope options for filtering documentation search results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DocumentationSearchScope {
    /// Search across all POD fields (name, synopsis, description, methods)
    #[default]
    All,
    /// Search module name only (from =head1 NAME)
    Name,
    /// Search synopsis only (from =head1 SYNOPSIS)
    Synopsis,
    /// Search description only (from =head1 DESCRIPTION)
    Description,
    /// Search method documentation only (from =head2 method_name)
    Methods,
}

/// A single documentation search result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentationSearchResult {
    /// URI of the document containing the match.
    pub uri: String,
    /// Module name from the matched document.
    pub module: String,
    /// Which POD section matched (e.g., "NAME", "SYNOPSIS", "DESCRIPTION", "method_name").
    pub section: Option<String>,
    /// The matching text excerpt.
    pub excerpt: String,
    /// Line number where the match starts (0-indexed).
    pub line: u32,
}

/// Internal document info stored in the index.
#[derive(Debug, Clone)]
struct DocInfo {
    /// Module name from the NAME section.
    module: String,
    /// URI of the document.
    uri: String,
    /// Synopsis section content.
    synopsis: Option<String>,
    /// Description section content.
    description: Option<String>,
    /// Method documentation sections.
    methods: HashMap<String, String>,
}

/// Provider for searching documentation across POD files in a workspace.
///
/// This follows the same provider pattern as `WorkspaceSymbolsProvider`:
/// - `index_document()` to add/update documentation for a file
/// - `remove_document()` to remove a file's documentation
/// - `search()` to find documentation matching a query
#[derive(Debug, Clone)]
pub struct DocumentationSearchProvider {
    /// Map of document URI to its extracted documentation.
    documents: HashMap<String, DocInfo>,
}

impl Default for DocumentationSearchProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentationSearchProvider {
    /// Creates a new empty documentation search provider.
    #[must_use]
    pub fn new() -> Self {
        Self { documents: HashMap::new() }
    }

    /// Indexes POD documentation from a document's source code.
    ///
    /// Extracts documentation using `perl_pod::extract_pod()` and stores it
    /// for later search queries. Replaces any previously indexed docs for the same URI.
    pub fn index_document(&mut self, uri: &str, source: &str) {
        let pod_doc = perl_pod::extract_pod(source);

        // Extract module name - use the name field or construct from URI
        let module = pod_doc.name.clone().unwrap_or_else(|| guess_module_name(uri));

        let doc_info = DocInfo {
            module,
            uri: uri.to_string(),
            synopsis: pod_doc.synopsis.clone(),
            description: pod_doc.description.clone(),
            methods: pod_doc.methods.clone(),
        };

        self.documents.insert(uri.to_string(), doc_info);
    }

    /// Removes a document and its documentation from the index.
    ///
    /// Called when a file is deleted or closed in the workspace.
    pub fn remove_document(&mut self, uri: &str) {
        self.documents.remove(uri);
    }

    /// Searches for documentation matching a query.
    ///
    /// Results are sorted by relevance: exact matches first, then prefix matches,
    /// then contains matches, then fuzzy/subsequence matches.
    ///
    /// # Arguments
    ///
    /// * `query` - The search query string
    /// * `scope` - Which POD fields to search (defaults to `All`)
    ///
    /// # Returns
    ///
    /// Vector of matching documentation results, sorted by relevance.
    #[must_use]
    pub fn search(
        &self,
        query: &str,
        scope: DocumentationSearchScope,
    ) -> Vec<DocumentationSearchResult> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for doc_info in self.documents.values() {
            match scope {
                DocumentationSearchScope::All => {
                    // Search all fields
                    if let Some(result) = self.search_name(doc_info, &query_lower) {
                        results.push(result);
                    }
                    if let Some(result) = self.search_synopsis(doc_info, &query_lower) {
                        results.push(result);
                    }
                    if let Some(result) = self.search_description(doc_info, &query_lower) {
                        results.push(result);
                    }
                    results.extend(self.search_methods(doc_info, &query_lower));
                }
                DocumentationSearchScope::Name => {
                    if let Some(result) = self.search_name(doc_info, &query_lower) {
                        results.push(result);
                    }
                }
                DocumentationSearchScope::Synopsis => {
                    if let Some(result) = self.search_synopsis(doc_info, &query_lower) {
                        results.push(result);
                    }
                }
                DocumentationSearchScope::Description => {
                    if let Some(result) = self.search_description(doc_info, &query_lower) {
                        results.push(result);
                    }
                }
                DocumentationSearchScope::Methods => {
                    results.extend(self.search_methods(doc_info, &query_lower));
                }
            }
        }

        // Sort by relevance: exact > prefix > contains > fuzzy
        results.sort_by(|a, b| {
            let a_score = relevance_score(&a.excerpt, &query_lower);
            let b_score = relevance_score(&b.excerpt, &query_lower);
            b_score.cmp(&a_score)
        });

        results
    }

    /// Search the NAME field.
    fn search_name(&self, doc_info: &DocInfo, query: &str) -> Option<DocumentationSearchResult> {
        let name_lower = doc_info.module.to_lowercase();
        if name_lower.contains(query) {
            Some(DocumentationSearchResult {
                uri: doc_info.uri.clone(),
                module: doc_info.module.clone(),
                section: Some("NAME".to_string()),
                excerpt: doc_info.module.clone(),
                line: 0,
            })
        } else {
            None
        }
    }

    /// Search the SYNOPSIS field.
    fn search_synopsis(
        &self,
        doc_info: &DocInfo,
        query: &str,
    ) -> Option<DocumentationSearchResult> {
        let synopsis = doc_info.synopsis.as_ref()?;
        let synopsis_lower = synopsis.to_lowercase();
        if synopsis_lower.contains(query) {
            Some(DocumentationSearchResult {
                uri: doc_info.uri.clone(),
                module: doc_info.module.clone(),
                section: Some("SYNOPSIS".to_string()),
                excerpt: synopsis.clone(),
                line: 0,
            })
        } else {
            None
        }
    }

    /// Search the DESCRIPTION field.
    fn search_description(
        &self,
        doc_info: &DocInfo,
        query: &str,
    ) -> Option<DocumentationSearchResult> {
        let description = doc_info.description.as_ref()?;
        let description_lower = description.to_lowercase();
        if description_lower.contains(query) {
            Some(DocumentationSearchResult {
                uri: doc_info.uri.clone(),
                module: doc_info.module.clone(),
                section: Some("DESCRIPTION".to_string()),
                excerpt: description.clone(),
                line: 0,
            })
        } else {
            None
        }
    }

    /// Search method documentation fields.
    fn search_methods(&self, doc_info: &DocInfo, query: &str) -> Vec<DocumentationSearchResult> {
        let mut results = Vec::new();
        for (method_name, method_doc) in &doc_info.methods {
            let method_lower = method_doc.to_lowercase();
            if method_lower.contains(query) || method_name.to_lowercase().contains(query) {
                results.push(DocumentationSearchResult {
                    uri: doc_info.uri.clone(),
                    module: doc_info.module.clone(),
                    section: Some(method_name.clone()),
                    excerpt: method_doc.clone(),
                    line: 0,
                });
            }
        }
        results
    }

    /// Returns the number of indexed documents.
    #[must_use]
    pub fn document_count(&self) -> usize {
        self.documents.len()
    }
}

/// Calculate a relevance score for sorting results.
/// Higher scores = more relevant.
fn relevance_score(excerpt: &str, query: &str) -> u32 {
    let excerpt_lower = excerpt.to_lowercase();

    // Exact match gets highest score
    if excerpt_lower == query {
        return 100;
    }

    // Exact substring match
    if excerpt_lower.contains(query) {
        // Prefix match gets higher score
        if excerpt_lower.starts_with(query) {
            return 80;
        }
        return 60;
    }

    // Fuzzy/subsequence match gets lower score
    if is_subsequence(query, &excerpt_lower) {
        return 40;
    }

    0
}

/// Check if query is a subsequence of text (fuzzy matching).
/// Case-insensitive comparison.
fn is_subsequence(query: &str, text: &str) -> bool {
    let query_lower: String = query.to_lowercase();
    let text_lower: String = text.to_lowercase();
    let mut query_chars = query_lower.chars().peekable();

    for ch in text_lower.chars() {
        if query_chars.peek() == Some(&ch) {
            query_chars.next();
        }
    }

    query_chars.next().is_none()
}

/// Guess a module name from a URI path.
/// e.g., "file:///lib/My/Module.pm" -> "lib::My::Module"
fn guess_module_name(uri: &str) -> String {
    // Remove file:// prefix if present
    let path = uri.strip_prefix("file://").or_else(|| uri.strip_prefix("file:")).unwrap_or(uri);

    // Remove .pm or .pod extension
    let path = path.trim_end_matches(".pm").trim_end_matches(".pod");

    // Convert / to ::
    let module = path.replace('/', "::");

    // Remove leading :: if any (from absolute paths like /lib/My/Module)
    module.strip_prefix("::").unwrap_or(&module).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guess_module_name() {
        assert_eq!(guess_module_name("file:///lib/My/Module.pm"), "lib::My::Module");
        assert_eq!(guess_module_name("/lib/My/Module.pm"), "lib::My::Module");
        assert_eq!(guess_module_name("file:///lib/My/Module.pod"), "lib::My::Module");
    }

    #[test]
    fn test_is_subsequence() {
        assert!(is_subsequence("abc", "axbyc"));
        assert!(is_subsequence("abc", "abcdef"));
        assert!(!is_subsequence("abc", "def"));
        assert!(is_subsequence("abc", "ABC"));
    }

    #[test]
    fn test_relevance_score() {
        assert_eq!(relevance_score("exact match", "exact match"), 100);
        assert_eq!(relevance_score("prefix match", "prefix"), 80);
        assert_eq!(relevance_score("contains match", "match"), 60);
        assert_eq!(relevance_score("fuzzy match", "fzy"), 40);
        assert_eq!(relevance_score("no match", "xyz"), 0);
    }

    #[test]
    fn test_pod_doc_extracts_name() {
        let source = r#"
package My::Module;

=head1 NAME

My::Module - A wonderful module

=cut
"#;

        let doc = perl_pod::extract_pod(source);
        assert_eq!(doc.name, Some("My::Module - A wonderful module".to_string()));
    }

    #[test]
    fn test_pod_doc_extracts_synopsis() {
        let source = r#"
=head1 SYNOPSIS

    use My::Module;

=cut
"#;

        let doc = perl_pod::extract_pod(source);
        assert!(doc.synopsis.is_some());
    }

    #[test]
    fn test_pod_doc_extracts_description() {
        let source = r#"
=head1 DESCRIPTION

This is a description.

=cut
"#;

        let doc = perl_pod::extract_pod(source);
        assert!(doc.description.is_some());
    }

    #[test]
    fn test_pod_doc_extracts_methods() {
        let source = r#"
=head2 process

The process method.

=cut
"#;

        let doc = perl_pod::extract_pod(source);
        assert!(doc.methods.contains_key("process"));
    }
}
