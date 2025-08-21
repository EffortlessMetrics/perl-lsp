use serde::{Deserialize, Serialize};

/// A test corpus section with metadata and source code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    /// Stable unique key, e.g., "regex.pos.001"
    pub id: String,
    
    /// Display title (line after "=====")
    pub title: String,
    
    /// File path (basename)
    pub file: String,
    
    /// Lowercased, space- or comma-separated tags
    pub tags: Vec<String>,
    
    /// Minimum perl version label (e.g., "5.10+")
    pub perl: Option<String>,
    
    /// Flags like "lexer-sensitive", "error-node-expected"
    pub flags: Vec<String>,
    
    /// Body text (source code of the section)
    pub body: String,
    
    /// Line number where section starts (for error reporting)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
}

impl Section {
    /// Check if section has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
    
    /// Check if section has a specific flag
    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.iter().any(|f| f == flag)
    }
}