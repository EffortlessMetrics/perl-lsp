use serde::{Deserialize, Serialize};

/// Source of a section identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdSource {
    Explicit,
    Generated,
}

/// Optional expected output block after a section body separator (`---`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectedBlock {
    pub raw: String,
    pub format: ExpectedFormat,
}

/// Best-effort format classification for expected output blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpectedFormat {
    TreeSitterSexp,
    AstJson,
    DiagnosticsJson,
    PlainText,
    Unknown,
}

/// A test corpus section with metadata and source code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    /// Stable unique key, e.g., "regex.pos.001"
    pub id: String,
    /// Identifier source metadata.
    pub id_source: IdSource,
    /// Explicit identifier from `# @id:`.
    pub explicit_id: Option<String>,
    /// Generated fallback identifier when explicit ID is missing.
    pub generated_id: Option<String>,

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
    /// Optional expected output block parsed from text after `---`.
    pub expected: Option<ExpectedBlock>,

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
