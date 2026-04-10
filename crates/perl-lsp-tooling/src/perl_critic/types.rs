use perl_parser_core::position::{Position, Range};
use serde::{Deserialize, Serialize};

#[cfg(feature = "lsp-compat")]
use lsp_types;

/// Severity levels for Perl::Critic violations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    /// Most severe issues (severity 5)
    Gentle = 5,
    /// High-severity issues (severity 4)
    Stern = 4,
    /// Important issues (severity 3)
    Harsh = 3,
    /// Lower-severity issues (severity 2)
    Cruel = 2,
    /// Least-severe issues (severity 1)
    Brutal = 1,
}

impl Severity {
    /// Converts a numeric severity (1-5) to a `Severity` variant.
    ///
    /// Values outside 1-5 default to `Harsh`.
    pub fn from_number(n: u8) -> Self {
        match n {
            1 => Self::Brutal,
            2 => Self::Cruel,
            3 => Self::Harsh,
            4 => Self::Stern,
            5 => Self::Gentle,
            _ => Self::Harsh,
        }
    }

    /// Converts this severity to a `DiagnosticSeverity` for LSP reporting.
    #[cfg(feature = "lsp-compat")]
    pub fn to_diagnostic_severity(self) -> lsp_types::DiagnosticSeverity {
        match self {
            Self::Gentle => lsp_types::DiagnosticSeverity::ERROR,
            Self::Stern | Self::Harsh => lsp_types::DiagnosticSeverity::WARNING,
            Self::Cruel => lsp_types::DiagnosticSeverity::INFORMATION,
            Self::Brutal => lsp_types::DiagnosticSeverity::HINT,
        }
    }

    /// Converts this severity to a numeric severity level (for non-LSP contexts).
    #[cfg(not(feature = "lsp-compat"))]
    pub fn to_severity_level(self) -> u8 {
        self as u8
    }
}

/// A Perl::Critic violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    /// The policy name that was violated (e.g., "TestingAndDebugging::RequireUseStrict")
    pub policy: String,
    /// A brief description of the violation
    pub description: String,
    /// A detailed explanation of why this policy exists
    pub explanation: String,
    /// The severity level of this violation
    pub severity: Severity,
    /// The source location where the violation occurred
    pub range: Range,
    /// The file path where the violation was found
    pub file: String,
}

/// Configuration for Perl::Critic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticConfig {
    /// Minimum severity level to report (1-5)
    pub severity: u8,
    /// Path to perlcriticrc file
    pub profile: Option<String>,
    /// Policies to explicitly include in analysis
    pub include: Vec<String>,
    /// Policies to explicitly exclude from analysis
    pub exclude: Vec<String>,
    /// Theme to use
    pub theme: Option<String>,
    /// Enable verbose output
    pub verbose: bool,
    /// Color output
    pub color: bool,
    /// Timeout in seconds for the perlcritic subprocess. Default: 30.
    pub timeout_secs: u64,
}

impl Default for CriticConfig {
    fn default() -> Self {
        Self {
            severity: 3,
            profile: None,
            include: Vec::new(),
            exclude: Vec::new(),
            theme: None,
            verbose: false,
            color: false,
            timeout_secs: 30,
        }
    }
}

#[cfg(not(feature = "lsp-compat"))]
/// Violation summary for non-LSP contexts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViolationSummary {
    /// Policy name
    pub policy: String,
    /// Description
    pub description: String,
    /// Severity level (1-5)
    pub severity: u8,
    /// Line number
    pub line: usize,
}

pub(crate) fn insertion_range() -> Range {
    Range {
        start: Position { byte: 0, line: 0, column: 0 },
        end: Position { byte: 0, line: 0, column: 0 },
    }
}
