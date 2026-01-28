//! Completion item types

use perl_parser_core::SourceLocation;

/// Type of completion item
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompletionItemKind {
    /// Variable (scalar, array, hash)
    Variable,
    /// Function or method
    Function,
    /// Perl keyword
    Keyword,
    /// Package or module
    Module,
    /// File path
    File,
    /// Snippet with placeholders
    Snippet,
    /// Constant value
    Constant,
    /// Property or hash key
    Property,
}

/// A single completion suggestion
#[derive(Debug, Clone)]
pub struct CompletionItem {
    /// The text to insert
    pub label: String,
    /// Kind of completion
    pub kind: CompletionItemKind,
    /// Optional detail text
    pub detail: Option<String>,
    /// Optional documentation
    pub documentation: Option<String>,
    /// Text to insert (if different from label)
    pub insert_text: Option<String>,
    /// Sort priority (lower is better)
    pub sort_text: Option<String>,
    /// Filter text for matching
    pub filter_text: Option<String>,
    /// Additional text edits to apply
    pub additional_edits: Vec<(SourceLocation, String)>,
    /// Range to replace in the document (for proper prefix handling)
    pub text_edit_range: Option<(usize, usize)>, // (start, end) offsets
}
