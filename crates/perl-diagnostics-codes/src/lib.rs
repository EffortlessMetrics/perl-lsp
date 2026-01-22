//! Stable diagnostic codes and severity levels for Perl LSP.
//!
//! This crate provides the canonical definitions of diagnostic codes used
//! throughout the Perl LSP ecosystem. These codes are stable and can be
//! referenced in documentation and error messages.
//!
//! # Code Ranges
//!
//! | Range       | Category                  |
//! |-------------|---------------------------|
//! | PL001-PL099 | Parser diagnostics        |
//! | PL100-PL199 | Strict/warnings           |
//! | PL200-PL299 | Package/module            |
//! | PL300-PL399 | Subroutine                |
//! | PL400-PL499 | Best practices            |
//! | PC001-PC005 | Perl::Critic violations   |
//!
//! # Example
//!
//! ```
//! use perl_diagnostics_codes::{DiagnosticCode, DiagnosticSeverity};
//!
//! let code = DiagnosticCode::ParseError;
//! assert_eq!(code.as_str(), "PL001");
//! assert_eq!(code.severity(), DiagnosticSeverity::Error);
//! ```

use std::fmt;

/// Severity level of a diagnostic.
///
/// Maps to LSP DiagnosticSeverity values (1=Error, 2=Warning, 3=Info, 4=Hint).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum DiagnosticSeverity {
    /// Critical error that prevents parsing/execution.
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
            DiagnosticSeverity::Error => write!(f, "error"),
            DiagnosticSeverity::Warning => write!(f, "warning"),
            DiagnosticSeverity::Information => write!(f, "info"),
            DiagnosticSeverity::Hint => write!(f, "hint"),
        }
    }
}

/// Diagnostic tags for additional classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DiagnosticTag {
    /// Code that can be safely removed (unused variables, imports).
    Unnecessary,
    /// Code using deprecated features.
    Deprecated,
}

impl DiagnosticTag {
    /// Get the LSP numeric value for this tag.
    pub fn to_lsp_value(self) -> u8 {
        match self {
            DiagnosticTag::Unnecessary => 1,
            DiagnosticTag::Deprecated => 2,
        }
    }
}

/// Stable diagnostic codes for Perl LSP.
///
/// Each code has a fixed string representation and associated metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DiagnosticCode {
    // Parser diagnostics (PL001-PL099)
    /// General parse error
    ParseError,
    /// Syntax error
    SyntaxError,
    /// Unexpected end-of-file
    UnexpectedEof,

    // Strict/warnings (PL100-PL199)
    /// Missing 'use strict' pragma
    MissingStrict,
    /// Missing 'use warnings' pragma
    MissingWarnings,
    /// Unused variable
    UnusedVariable,
    /// Undefined variable
    UndefinedVariable,

    // Package/module (PL200-PL299)
    /// Missing package declaration
    MissingPackageDeclaration,
    /// Duplicate package declaration
    DuplicatePackage,

    // Subroutine (PL300-PL399)
    /// Duplicate subroutine definition
    DuplicateSubroutine,
    /// Missing explicit return statement
    MissingReturn,

    // Best practices (PL400-PL499)
    /// Bareword filehandle usage
    BarewordFilehandle,
    /// Two-argument open() call
    TwoArgOpen,
    /// Implicit return value
    ImplicitReturn,

    // Perl::Critic violations (PC001-PC005)
    /// Perl::Critic brutal (severity 1) violation
    CriticSeverity1,
    /// Perl::Critic cruel (severity 2) violation
    CriticSeverity2,
    /// Perl::Critic harsh (severity 3) violation
    CriticSeverity3,
    /// Perl::Critic stern (severity 4) violation
    CriticSeverity4,
    /// Perl::Critic gentle (severity 5) violation
    CriticSeverity5,
}

impl DiagnosticCode {
    /// Get the string representation of this code.
    pub fn as_str(&self) -> &'static str {
        match self {
            DiagnosticCode::ParseError => "PL001",
            DiagnosticCode::SyntaxError => "PL002",
            DiagnosticCode::UnexpectedEof => "PL003",
            DiagnosticCode::MissingStrict => "PL100",
            DiagnosticCode::MissingWarnings => "PL101",
            DiagnosticCode::UnusedVariable => "PL102",
            DiagnosticCode::UndefinedVariable => "PL103",
            DiagnosticCode::MissingPackageDeclaration => "PL200",
            DiagnosticCode::DuplicatePackage => "PL201",
            DiagnosticCode::DuplicateSubroutine => "PL300",
            DiagnosticCode::MissingReturn => "PL301",
            DiagnosticCode::BarewordFilehandle => "PL400",
            DiagnosticCode::TwoArgOpen => "PL401",
            DiagnosticCode::ImplicitReturn => "PL402",
            DiagnosticCode::CriticSeverity1 => "PC001",
            DiagnosticCode::CriticSeverity2 => "PC002",
            DiagnosticCode::CriticSeverity3 => "PC003",
            DiagnosticCode::CriticSeverity4 => "PC004",
            DiagnosticCode::CriticSeverity5 => "PC005",
        }
    }

    /// Get the documentation URL for this code, if available.
    pub fn documentation_url(&self) -> Option<&'static str> {
        match self {
            DiagnosticCode::ParseError => Some("https://docs.perl-lsp.org/errors/PL001"),
            DiagnosticCode::SyntaxError => Some("https://docs.perl-lsp.org/errors/PL002"),
            DiagnosticCode::UnexpectedEof => Some("https://docs.perl-lsp.org/errors/PL003"),
            DiagnosticCode::MissingStrict => Some("https://docs.perl-lsp.org/errors/PL100"),
            DiagnosticCode::MissingWarnings => Some("https://docs.perl-lsp.org/errors/PL101"),
            DiagnosticCode::UnusedVariable => Some("https://docs.perl-lsp.org/errors/PL102"),
            DiagnosticCode::UndefinedVariable => Some("https://docs.perl-lsp.org/errors/PL103"),
            DiagnosticCode::MissingPackageDeclaration => {
                Some("https://docs.perl-lsp.org/errors/PL200")
            }
            DiagnosticCode::DuplicatePackage => Some("https://docs.perl-lsp.org/errors/PL201"),
            DiagnosticCode::DuplicateSubroutine => Some("https://docs.perl-lsp.org/errors/PL300"),
            DiagnosticCode::MissingReturn => Some("https://docs.perl-lsp.org/errors/PL301"),
            DiagnosticCode::BarewordFilehandle => Some("https://docs.perl-lsp.org/errors/PL400"),
            DiagnosticCode::TwoArgOpen => Some("https://docs.perl-lsp.org/errors/PL401"),
            DiagnosticCode::ImplicitReturn => Some("https://docs.perl-lsp.org/errors/PL402"),
            // Perl::Critic codes don't have centralized documentation
            DiagnosticCode::CriticSeverity1
            | DiagnosticCode::CriticSeverity2
            | DiagnosticCode::CriticSeverity3
            | DiagnosticCode::CriticSeverity4
            | DiagnosticCode::CriticSeverity5 => None,
        }
    }

    /// Get the default severity for this diagnostic code.
    pub fn severity(&self) -> DiagnosticSeverity {
        match self {
            // Errors
            DiagnosticCode::ParseError
            | DiagnosticCode::SyntaxError
            | DiagnosticCode::UnexpectedEof
            | DiagnosticCode::UndefinedVariable => DiagnosticSeverity::Error,

            // Warnings
            DiagnosticCode::MissingStrict
            | DiagnosticCode::MissingWarnings
            | DiagnosticCode::UnusedVariable
            | DiagnosticCode::MissingPackageDeclaration
            | DiagnosticCode::DuplicatePackage
            | DiagnosticCode::DuplicateSubroutine
            | DiagnosticCode::MissingReturn
            | DiagnosticCode::BarewordFilehandle
            | DiagnosticCode::TwoArgOpen
            | DiagnosticCode::ImplicitReturn
            | DiagnosticCode::CriticSeverity1
            | DiagnosticCode::CriticSeverity2 => DiagnosticSeverity::Warning,

            // Information/Hints
            DiagnosticCode::CriticSeverity3
            | DiagnosticCode::CriticSeverity4
            | DiagnosticCode::CriticSeverity5 => DiagnosticSeverity::Hint,
        }
    }

    /// Get any diagnostic tags associated with this code.
    pub fn tags(&self) -> &'static [DiagnosticTag] {
        match self {
            DiagnosticCode::UnusedVariable => &[DiagnosticTag::Unnecessary],
            _ => &[],
        }
    }

    /// Try to infer a diagnostic code from a message.
    pub fn from_message(msg: &str) -> Option<DiagnosticCode> {
        let msg_lower = msg.to_lowercase();
        if msg_lower.contains("use strict") {
            Some(DiagnosticCode::MissingStrict)
        } else if msg_lower.contains("use warnings") {
            Some(DiagnosticCode::MissingWarnings)
        } else if msg_lower.contains("unused variable") || msg_lower.contains("never used") {
            Some(DiagnosticCode::UnusedVariable)
        } else if msg_lower.contains("undefined") || msg_lower.contains("not declared") {
            Some(DiagnosticCode::UndefinedVariable)
        } else if msg_lower.contains("bareword filehandle") {
            Some(DiagnosticCode::BarewordFilehandle)
        } else if msg_lower.contains("two-argument") || msg_lower.contains("2-arg") {
            Some(DiagnosticCode::TwoArgOpen)
        } else if msg_lower.contains("parse error") || msg_lower.contains("syntax error") {
            Some(DiagnosticCode::ParseError)
        } else {
            None
        }
    }

    /// Try to parse a code string into a DiagnosticCode.
    pub fn parse_code(code: &str) -> Option<DiagnosticCode> {
        match code {
            "PL001" => Some(DiagnosticCode::ParseError),
            "PL002" => Some(DiagnosticCode::SyntaxError),
            "PL003" => Some(DiagnosticCode::UnexpectedEof),
            "PL100" => Some(DiagnosticCode::MissingStrict),
            "PL101" => Some(DiagnosticCode::MissingWarnings),
            "PL102" => Some(DiagnosticCode::UnusedVariable),
            "PL103" => Some(DiagnosticCode::UndefinedVariable),
            "PL200" => Some(DiagnosticCode::MissingPackageDeclaration),
            "PL201" => Some(DiagnosticCode::DuplicatePackage),
            "PL300" => Some(DiagnosticCode::DuplicateSubroutine),
            "PL301" => Some(DiagnosticCode::MissingReturn),
            "PL400" => Some(DiagnosticCode::BarewordFilehandle),
            "PL401" => Some(DiagnosticCode::TwoArgOpen),
            "PL402" => Some(DiagnosticCode::ImplicitReturn),
            "PC001" => Some(DiagnosticCode::CriticSeverity1),
            "PC002" => Some(DiagnosticCode::CriticSeverity2),
            "PC003" => Some(DiagnosticCode::CriticSeverity3),
            "PC004" => Some(DiagnosticCode::CriticSeverity4),
            "PC005" => Some(DiagnosticCode::CriticSeverity5),
            _ => None,
        }
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Category of diagnostic codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DiagnosticCategory {
    /// Parser-related diagnostics (PL001-PL099)
    Parser,
    /// Strict/warnings pragmas (PL100-PL199)
    StrictWarnings,
    /// Package/module issues (PL200-PL299)
    PackageModule,
    /// Subroutine issues (PL300-PL399)
    Subroutine,
    /// Best practices (PL400-PL499)
    BestPractices,
    /// Perl::Critic violations (PC001-PC005)
    PerlCritic,
}

impl DiagnosticCode {
    /// Get the category of this diagnostic code.
    pub fn category(&self) -> DiagnosticCategory {
        match self {
            DiagnosticCode::ParseError
            | DiagnosticCode::SyntaxError
            | DiagnosticCode::UnexpectedEof => DiagnosticCategory::Parser,

            DiagnosticCode::MissingStrict
            | DiagnosticCode::MissingWarnings
            | DiagnosticCode::UnusedVariable
            | DiagnosticCode::UndefinedVariable => DiagnosticCategory::StrictWarnings,

            DiagnosticCode::MissingPackageDeclaration | DiagnosticCode::DuplicatePackage => {
                DiagnosticCategory::PackageModule
            }

            DiagnosticCode::DuplicateSubroutine | DiagnosticCode::MissingReturn => {
                DiagnosticCategory::Subroutine
            }

            DiagnosticCode::BarewordFilehandle
            | DiagnosticCode::TwoArgOpen
            | DiagnosticCode::ImplicitReturn => DiagnosticCategory::BestPractices,

            DiagnosticCode::CriticSeverity1
            | DiagnosticCode::CriticSeverity2
            | DiagnosticCode::CriticSeverity3
            | DiagnosticCode::CriticSeverity4
            | DiagnosticCode::CriticSeverity5 => DiagnosticCategory::PerlCritic,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_strings() {
        assert_eq!(DiagnosticCode::ParseError.as_str(), "PL001");
        assert_eq!(DiagnosticCode::MissingStrict.as_str(), "PL100");
        assert_eq!(DiagnosticCode::CriticSeverity1.as_str(), "PC001");
    }

    #[test]
    fn test_severity() {
        assert_eq!(DiagnosticCode::ParseError.severity(), DiagnosticSeverity::Error);
        assert_eq!(DiagnosticCode::UnusedVariable.severity(), DiagnosticSeverity::Warning);
        assert_eq!(DiagnosticCode::CriticSeverity5.severity(), DiagnosticSeverity::Hint);
    }

    #[test]
    fn test_from_message() {
        assert_eq!(
            DiagnosticCode::from_message("Missing 'use strict' pragma"),
            Some(DiagnosticCode::MissingStrict)
        );
        assert_eq!(
            DiagnosticCode::from_message("Unused variable $foo"),
            Some(DiagnosticCode::UnusedVariable)
        );
    }

    #[test]
    fn test_from_str() {
        assert_eq!(DiagnosticCode::parse_code("PL001"), Some(DiagnosticCode::ParseError));
        assert_eq!(DiagnosticCode::parse_code("INVALID"), None);
    }

    #[test]
    fn test_category() {
        assert_eq!(DiagnosticCode::ParseError.category(), DiagnosticCategory::Parser);
        assert_eq!(DiagnosticCode::MissingStrict.category(), DiagnosticCategory::StrictWarnings);
        assert_eq!(DiagnosticCode::CriticSeverity1.category(), DiagnosticCategory::PerlCritic);
    }

    #[test]
    fn test_tags() {
        assert!(DiagnosticCode::UnusedVariable.tags().contains(&DiagnosticTag::Unnecessary));
        assert!(DiagnosticCode::ParseError.tags().is_empty());
    }
}
