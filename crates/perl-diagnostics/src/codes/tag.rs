use std::fmt;

/// Diagnostic tags for additional classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DiagnosticTag {
    /// Code that can be safely removed (unused variables, imports).
    Unnecessary = 1,
    /// Code using deprecated features.
    Deprecated = 2,
}

impl DiagnosticTag {
    /// Get the LSP numeric value for this tag.
    pub fn to_lsp_value(self) -> u8 {
        self as u8
    }
}

impl fmt::Display for DiagnosticTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unnecessary => write!(f, "unnecessary"),
            Self::Deprecated => write!(f, "deprecated"),
        }
    }
}
