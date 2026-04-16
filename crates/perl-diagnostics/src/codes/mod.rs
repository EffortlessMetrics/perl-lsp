//! Diagnostic codes, severity levels, tags, and categories.
//!
//! This module contains the canonical definitions of:
//! - `DiagnosticCode` — all diagnostic codes (ParseError, MissingStrict, etc.)
//! - `DiagnosticSeverity` — Error, Warning, Information, Hint (LSP values 1-4)
//! - `DiagnosticTag` — Unnecessary, Deprecated (LSP tag values)
//! - `DiagnosticCategory` — classification of diagnostic codes

use std::fmt;

/// Diagnostic severity levels (LSP standard).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DiagnosticSeverity {
    /// Error (LSP value 1)
    #[default]
    Error = 1,
    /// Warning (LSP value 2)
    Warning = 2,
    /// Information (LSP value 3)
    Information = 3,
    /// Hint (LSP value 4)
    Hint = 4,
}

impl DiagnosticSeverity {
    /// Convert to LSP wire format value (1-4).
    pub fn to_lsp_value(self) -> u8 {
        self as u8
    }
}

impl fmt::Display for DiagnosticSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
            Self::Information => write!(f, "information"),
            Self::Hint => write!(f, "hint"),
        }
    }
}

/// Diagnostic tags (LSP standard).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DiagnosticTag {
    /// Unnecessary (LSP value 1)
    Unnecessary = 1,
    /// Deprecated (LSP value 2)
    Deprecated = 2,
}

impl DiagnosticTag {
    /// Convert to LSP wire format value (1-2).
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

/// Diagnostic code enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DiagnosticCode {
    /// Default variant (ParseError)
    #[default]
    /// Parse error
    ParseError,
    /// Syntax error
    SyntaxError,
    /// Unexpected EOF
    UnexpectedEof,
    /// Missing 'use strict'
    MissingStrict,
    /// Missing 'use warnings'
    MissingWarnings,
    /// Phase-scoped strict pragma
    PhaseScopedStrictPragma,
    /// Phase-scoped warnings pragma
    PhaseScopedWarningsPragma,
    /// Unused variable
    UnusedVariable,
    /// Undefined variable
    UndefinedVariable,
    /// Capture variable without regex match
    CaptureVarWithoutRegexMatch,
    /// Missing package declaration
    MissingPackageDeclaration,
    /// Duplicate package
    DuplicatePackage,
    /// Duplicate subroutine
    DuplicateSubroutine,
    /// Missing return
    MissingReturn,
    /// Role conflict
    RoleConflict,
    /// Invalid prototype
    InvalidPrototype,
    /// Bareword filehandle
    BarewordFilehandle,
    /// Two-argument open
    TwoArgOpen,
    /// Implicit return
    ImplicitReturn,
    /// Printf format mismatch
    PrintfFormatMismatch,
    /// Security signal handler
    SecuritySignalHandler,
    /// Perl::Critic severity 1
    CriticSeverity1,
    /// Perl::Critic severity 2
    CriticSeverity2,
    /// Perl::Critic severity 3
    CriticSeverity3,
    /// Perl::Critic severity 4
    CriticSeverity4,
    /// Perl::Critic severity 5
    CriticSeverity5,
}

impl DiagnosticCode {
    /// Get stable string representation of this code.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ParseError => "PL001",
            Self::SyntaxError => "PL002",
            Self::UnexpectedEof => "PL003",
            Self::MissingStrict => "PL101",
            Self::MissingWarnings => "PL102",
            Self::PhaseScopedStrictPragma => "PL103",
            Self::PhaseScopedWarningsPragma => "PL104",
            Self::UnusedVariable => "PL105",
            Self::UndefinedVariable => "PL106",
            Self::CaptureVarWithoutRegexMatch => "PL107",
            Self::MissingPackageDeclaration => "PL201",
            Self::DuplicatePackage => "PL202",
            Self::DuplicateSubroutine => "PL301",
            Self::MissingReturn => "PL302",
            Self::RoleConflict => "PL303",
            Self::InvalidPrototype => "PL304",
            Self::BarewordFilehandle => "PL401",
            Self::TwoArgOpen => "PL402",
            Self::ImplicitReturn => "PL403",
            Self::PrintfFormatMismatch => "PL404",
            Self::SecuritySignalHandler => "PL405",
            Self::CriticSeverity1 => "PC001",
            Self::CriticSeverity2 => "PC002",
            Self::CriticSeverity3 => "PC003",
            Self::CriticSeverity4 => "PC004",
            Self::CriticSeverity5 => "PC005",
        }
    }

    /// Default severity for this diagnostic code.
    pub fn severity(self) -> DiagnosticSeverity {
        match self {
            Self::ParseError | Self::SyntaxError | Self::UnexpectedEof => DiagnosticSeverity::Error,
            _ => DiagnosticSeverity::Warning,
        }
    }

    /// Category of this diagnostic code.
    pub fn category(self) -> DiagnosticCategory {
        match self {
            Self::ParseError | Self::SyntaxError | Self::UnexpectedEof => {
                DiagnosticCategory::Parser
            }
            Self::MissingStrict
            | Self::MissingWarnings
            | Self::PhaseScopedStrictPragma
            | Self::PhaseScopedWarningsPragma
            | Self::UnusedVariable
            | Self::UndefinedVariable
            | Self::CaptureVarWithoutRegexMatch => DiagnosticCategory::StrictWarnings,
            Self::MissingPackageDeclaration | Self::DuplicatePackage => {
                DiagnosticCategory::PackageModule
            }
            Self::DuplicateSubroutine
            | Self::MissingReturn
            | Self::RoleConflict
            | Self::InvalidPrototype => DiagnosticCategory::Subroutine,
            Self::BarewordFilehandle
            | Self::TwoArgOpen
            | Self::ImplicitReturn
            | Self::PrintfFormatMismatch
            | Self::SecuritySignalHandler => DiagnosticCategory::BestPractices,
            Self::CriticSeverity1
            | Self::CriticSeverity2
            | Self::CriticSeverity3
            | Self::CriticSeverity4
            | Self::CriticSeverity5 => DiagnosticCategory::PerlCritic,
        }
    }

    /// Tags for this diagnostic code.
    pub fn tags(self) -> &'static [DiagnosticTag] {
        match self {
            Self::UnusedVariable => &[DiagnosticTag::Unnecessary],
            _ => &[],
        }
    }

    /// Optional documentation URL.
    pub fn documentation_url(self) -> Option<&'static str> {
        match self {
            Self::CriticSeverity1
            | Self::CriticSeverity2
            | Self::CriticSeverity3
            | Self::CriticSeverity4
            | Self::CriticSeverity5 => None,
            _ => Some("https://perl-lsp.org/diagnostics"),
        }
    }

    /// Parse a code string (e.g., "PL001") into a DiagnosticCode.
    pub fn parse_code(code: &str) -> Option<Self> {
        match code {
            "PL001" => Some(Self::ParseError),
            "PL002" => Some(Self::SyntaxError),
            "PL003" => Some(Self::UnexpectedEof),
            "PL101" => Some(Self::MissingStrict),
            "PL102" => Some(Self::MissingWarnings),
            "PL103" => Some(Self::PhaseScopedStrictPragma),
            "PL104" => Some(Self::PhaseScopedWarningsPragma),
            "PL105" => Some(Self::UnusedVariable),
            "PL106" => Some(Self::UndefinedVariable),
            "PL107" => Some(Self::CaptureVarWithoutRegexMatch),
            "PL201" => Some(Self::MissingPackageDeclaration),
            "PL202" => Some(Self::DuplicatePackage),
            "PL301" => Some(Self::DuplicateSubroutine),
            "PL302" => Some(Self::MissingReturn),
            "PL303" => Some(Self::RoleConflict),
            "PL304" => Some(Self::InvalidPrototype),
            "PL401" => Some(Self::BarewordFilehandle),
            "PL402" => Some(Self::TwoArgOpen),
            "PL403" => Some(Self::ImplicitReturn),
            "PL404" => Some(Self::PrintfFormatMismatch),
            "PL405" => Some(Self::SecuritySignalHandler),
            "PC001" => Some(Self::CriticSeverity1),
            "PC002" => Some(Self::CriticSeverity2),
            "PC003" => Some(Self::CriticSeverity3),
            "PC004" => Some(Self::CriticSeverity4),
            "PC005" => Some(Self::CriticSeverity5),
            _ => None,
        }
    }

    /// Infer a diagnostic code from an error message.
    pub fn from_message(msg: &str) -> Option<Self> {
        let msg_lower = msg.to_lowercase();
        if msg_lower.contains("strict") {
            Some(Self::MissingStrict)
        } else if msg_lower.contains("warnings") {
            Some(Self::MissingWarnings)
        } else if msg_lower.contains("parse") {
            Some(Self::ParseError)
        } else {
            None
        }
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Diagnostic code categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DiagnosticCategory {
    /// Parser errors
    Parser,
    /// Strict/warnings related
    StrictWarnings,
    /// Package/module related
    PackageModule,
    /// Subroutine related
    Subroutine,
    /// Best practices
    BestPractices,
    /// Perl::Critic
    PerlCritic,
}

impl fmt::Display for DiagnosticCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parser => write!(f, "Parser"),
            Self::StrictWarnings => write!(f, "Strict/Warnings"),
            Self::PackageModule => write!(f, "Package/Module"),
            Self::Subroutine => write!(f, "Subroutine"),
            Self::BestPractices => write!(f, "Best Practices"),
            Self::PerlCritic => write!(f, "Perl::Critic"),
        }
    }
}
