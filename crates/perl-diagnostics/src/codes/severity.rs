use std::fmt;

/// Severity level of a diagnostic.
///
/// Maps to LSP DiagnosticSeverity values (1=Error, 2=Warning, 3=Info, 4=Hint).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum DiagnosticSeverity {
    /// Critical error that prevents parsing/execution.
    #[default]
    Error = 1,
    /// Non-critical issue that should be addressed.
    Warning = 2,
    /// Informational message.
    Information = 3,
    /// Subtle suggestion or hint.
    Hint = 4,
}

impl DiagnosticSeverity {
    /// Get the LSP numeric value for this severity.
    pub fn to_lsp_value(self) -> u8 {
        self as u8
    }
}

impl fmt::Display for DiagnosticSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
            Self::Information => write!(f, "info"),
            Self::Hint => write!(f, "hint"),
        }
    }
}
